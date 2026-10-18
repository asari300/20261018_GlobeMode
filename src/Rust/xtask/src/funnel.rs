//! `cargo xtask msl-funnel`: MSL 4.1.0 の全例題に判定（S0〜S8 / OK / GM-U）を出し、報告を書く（S005、S7・S8 は S007）。
//!
//! - 例題の選び方は Rumoca の MSL ハーネスと同じ名前の規則（[`examples_in`]）。Rumoca の基準値と比べられるようにする。
//! - 検査器（[`Checker`]）は 1 つだけ作り、MSL を一度だけ読む。モデルごとに `catch_unwind` で囲み、panic したら数えて続ける
//!   （Mutex が毒されたら、`Checker` が次の検査でライブラリを読んだ直後の Session に戻す）。
//! - S5 まで通ったモデルは、`globe-mode-analysis` で S6（構造特異）も見て、Rumoca 自身の構造解析の数と照合する。
//! - S6 を通ったモデルは S7（Pantelides 法、[`analyze_index`]）を、そこまで Error が無ければ S8（初期化、
//!   [`analyze_init_system`]。要る初期条件の数は S7 の動的な状態の数で数え、S009 から初期化系もマッチングする）を回す。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use globe_mode_analysis::analyze_init_system;
use globe_mode_analysis::{IndexReport, InitReport, StructuralReport, analyze_index};
use globe_mode_core::{Diagnostic, Severity, Stage, codes};
use globe_mode_db::GlobeDb;
use globe_mode_frontend::{CheckReport, Checker, ReferenceStructure, unsupported};
use globe_mode_ir::{ClassIndex, ClassKind, DaeStructure, InitStructure, UnknownKind};

use crate::{MSL_SHA256, MSL_VERSION, RUMOCA_REV, RUMOCA_VERSION, Result};

/// Rumoca の MSL ハーネスの基準値（`crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json`、MSL 4.1.0）。
const RUMOCA_BASELINE: Funnel = Funnel {
    examples: 566,
    parse: 566,
    flatten: 494,
    dae: 305,
    balanced: 293,
};

/// 同じ基準値の `partial_models`（566 本のうち partial のもの）。
const RUMOCA_BASELINE_PARTIAL: usize = 13;

/// 例題の中の「補助のパッケージ」（Rumoca の `EXAMPLE_SUPPORT_SEGMENTS`）。この下は例題に数えない。
const SUPPORT_SEGMENTS: [&str; 4] = ["Utilities", "BaseClasses", "Internal", "Interfaces"];

/// 例題 1 本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Example {
    pub name: String,
    /// `partial` のクラス。Rumoca の 566 本にも 13 本含まれる（基準値の `partial_models`）ので、除かずに印を付ける。
    pub partial: bool,
}

/// MSL のディレクトリ（`data/msl`）から例題を選ぶ。`Modelica 4.1.0` をアウトラインで索引にして [`examples_in`] を当てる。
pub(crate) fn msl_examples(msl: &Path) -> Result<Vec<Example>> {
    let dir = msl.join(format!("Modelica {MSL_VERSION}"));
    let mut db = GlobeDb::new();
    db.load_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(examples_in(db.class_index()))
}

/// Rumoca の MSL ハーネスと同じ規則で例題を選ぶ（名前順）。
///
/// `crates/rumoca-test-msl/tests/balance_pipeline/balance_pipeline_example_targets.rs`
/// （`root_msl_example_model_names` / `is_root_msl_example_path`）と、候補の集合を作る
/// `crates/rumoca-compile/src/merge.rs`（`collect_models_from_classes`）の読みで再現した。
///
/// - クラスの種類が `model` / `block` / `class`（`Connections.branch` などの組み込みの演算子は除く）
/// - 一番外が `Modelica` で、途中に `Examples` がある（最初の `Examples` で切る）
/// - `Examples` と自分の間はすべて `package` で、`Utilities` / `BaseClasses` / `Internal` / `Interfaces` を含まない
///   （例題のモデルの中に入れ子にしたモデルは、親が package でないので外れる）
/// - `partial` は除かない（Rumoca も除かない。結果に印を付ける）
pub(crate) fn examples_in(index: &ClassIndex) -> Vec<Example> {
    index
        .names()
        .filter_map(|name| {
            let c = index.get(name)?;
            is_root_example(index, name, c.kind).then(|| Example {
                name: name.to_string(),
                partial: c.partial,
            })
        })
        .collect()
}

fn is_root_example(index: &ClassIndex, name: &str, kind: ClassKind) -> bool {
    if !matches!(kind, ClassKind::Model | ClassKind::Block | ClassKind::Class)
        || is_builtin_operator(name)
    {
        return false;
    }
    let segments: Vec<&str> = name.split('.').collect();
    if segments.first() != Some(&"Modelica") {
        return false;
    }
    let Some(examples) = segments.iter().position(|s| *s == "Examples") else {
        return false;
    };
    if examples + 1 >= segments.len() {
        return false;
    }
    let last = segments.len() - 1;
    if segments[examples + 1..last]
        .iter()
        .any(|s| SUPPORT_SEGMENTS.contains(s))
    {
        return false;
    }
    (examples + 1..last).all(|i| {
        index
            .get(&segments[..=i].join("."))
            .is_some_and(|c| c.kind == ClassKind::Package)
    })
}

/// Rumoca の `is_builtin_operator`（`crates/rumoca-compile/src/merge.rs`）。
fn is_builtin_operator(name: &str) -> bool {
    [
        "Connections.branch",
        "Connections.root",
        "Connections.potentialRoot",
        "Connections.isRoot",
        "Connections.rooted",
    ]
    .iter()
    .any(|op| name.contains(op))
}

// --------------------------------------------------------------------------- 検査器

/// 報告に書く検査器の説明。
const CHECKER_KIND: &str =
    "`globe_mode_frontend::Checker`（MSL を一度だけ読み、Rumoca の Session を使い回す）";

/// 検査器を作る（MSL を読む）。
fn make_checker(msl: &Path) -> Result<Checker> {
    Checker::new(&[msl]).map_err(|ds| {
        ds.iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// 1 本を検査する。S5 まで通って構造があれば、Rumoca 自身の構造解析の数も取る（S6 の照合用。
/// コンパイルの結果は Session が覚えているので、2 回目のコンパイルはほぼかからない）。
fn check_one(checker: &Checker, model: &str) -> (CheckReport, Option<ReferenceStructure>) {
    let report = checker.check(&[], model);
    let reference = (report.passed() && report.structure.is_some())
        .then(|| checker.reference_structure(&[], model))
        .flatten();
    (report, reference)
}

// --------------------------------------------------------------------------- 判定

/// 1 本の判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Verdict {
    /// S0〜S8 で Error が無い（S005 の報告では S0〜S6）
    Ok,
    /// モデルの誤り（GM-E）で止まった
    Ng,
    /// コンパイラが未対応（GM-U）で止まった
    Unsupported,
    /// 検査器が panic した
    Panic,
}

/// S6 の結果の要約。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S6 {
    pub equations: usize,
    pub unknowns: usize,
    /// der の形（`incidence` だけ）の最大マッチング。Rumoca の構造解析と照合する値
    pub matching: usize,
    /// `x` と `der(x)` を 1 つの列にまとめた形の最大マッチング。構造特異の判定に使う
    pub structural_matching: usize,
    /// 構造特異でない（まとめた形で完全マッチングがある）
    pub regular: bool,
    /// まとめた形では完全マッチングがあるが、der の形では無い（インデックス > 1 の見込み。S7 の結果と同値）
    pub index_reduction_needed: bool,
    /// der の形で未知数を 1 つも含まない方程式の数（状態・パラメータ・入力だけの拘束）。
    /// このような方程式は der の形ではどの未知数ともマッチしない
    pub der_no_unknown_equations: usize,
    /// Rumoca 自身の構造解析（`phase_structural::analyze`）の（方程式, 未知数, マッチング）
    pub rumoca: Option<(usize, usize, usize)>,
}

impl S6 {
    fn new(
        structure: &DaeStructure,
        r: &StructuralReport,
        reference: Option<ReferenceStructure>,
    ) -> Self {
        Self {
            equations: r.n_equations,
            unknowns: r.n_unknowns,
            matching: r.matching,
            structural_matching: r.structural_matching,
            regular: r.is_regular(),
            index_reduction_needed: r.index_reduction_needed,
            der_no_unknown_equations: structure.incidence.iter().filter(|r| r.is_empty()).count(),
            rumoca: reference.map(|x| (x.n_equations, x.n_unknowns, x.matching)),
        }
    }

    fn is_square(&self) -> bool {
        self.equations == self.unknowns
    }

    /// Rumoca の構造解析と（方程式, 未知数, der の形のマッチング）が同じか。照合していなければ None。
    pub(crate) fn agrees_with_rumoca(&self) -> Option<bool> {
        self.rumoca
            .map(|r| r == (self.equations, self.unknowns, self.matching))
    }
}

/// S7 の結果の要約（[`IndexReport`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S7 {
    /// 構造インデックスの見込み（打ち切ったら 0）
    pub index: usize,
    /// 1 回以上微分した方程式の数
    pub n_differentiated: usize,
    /// 微分の最大回数
    pub max_differentiations: u32,
    /// 状態選択（dummy derivative）の候補の状態の数
    pub dummy_candidates: usize,
    /// 状態の数（スカラー）と、動的な状態の数の見込み（自由度）
    pub states: usize,
    pub dynamic_states: usize,
    /// 打ち切った（`GM-E7001`）
    pub failed: bool,
}

impl S7 {
    fn new(structure: &DaeStructure, r: &IndexReport) -> Self {
        Self {
            index: r.index,
            n_differentiated: r.n_differentiated,
            max_differentiations: r.max_differentiations(),
            dummy_candidates: r.dummy_candidates.len(),
            states: structure
                .unknowns
                .iter()
                .filter(|u| u.kind == UnknownKind::Derivative)
                .count(),
            dynamic_states: r.dynamic_states,
            failed: r.failed,
        }
    }

    /// 動的な状態が状態の数より少ない（状態選択が要る。`GM-I7002`）。
    fn needs_state_selection(&self) -> bool {
        !self.failed && self.dynamic_states < self.states
    }

    /// 構造インデックスの分布の区分。
    fn bucket(&self) -> IndexBucket {
        match (self.failed, self.index) {
            (true, _) => IndexBucket::Failed,
            (false, 0 | 1) => IndexBucket::One,
            (false, 2) => IndexBucket::Two,
            (false, 3) => IndexBucket::Three,
            (false, _) => IndexBucket::FourOrMore,
        }
    }
}

/// 構造インデックスの分布の区分（未知数の無い構造のインデックス 0 は 1 に入れる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum IndexBucket {
    One,
    Two,
    Three,
    FourOrMore,
    /// 打ち切り（`GM-E7001`）
    Failed,
}

impl IndexBucket {
    const ALL: [IndexBucket; 5] = [
        IndexBucket::One,
        IndexBucket::Two,
        IndexBucket::Three,
        IndexBucket::FourOrMore,
        IndexBucket::Failed,
    ];

    fn label(self) -> &'static str {
        match self {
            IndexBucket::One => "1",
            IndexBucket::Two => "2",
            IndexBucket::Three => "3",
            IndexBucket::FourOrMore => "4 以上",
            IndexBucket::Failed => "打ち切り（GM-E7001）",
        }
    }

    fn key(self) -> &'static str {
        match self {
            IndexBucket::One => "1",
            IndexBucket::Two => "2",
            IndexBucket::Three => "3",
            IndexBucket::FourOrMore => "4+",
            IndexBucket::Failed => "failed",
        }
    }
}

/// S8 の結果の要約（[`InitReport`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S8 {
    /// 要る初期条件の数 / 与えられた初期条件の数
    pub required: usize,
    pub provided: usize,
    /// 初期化系をマッチングできた（S009。初期方程式の参照と S7 の結果があるとき）
    pub matched: bool,
    /// 過剰（冗長な初期条件）の数と不足（初期条件の無い未知数）の数。マッチングがあればその値、
    /// 無ければ与えた数と要る数の差（片方は 0）
    pub extra: usize,
    pub missing: usize,
    /// 値の無いパラメータの数
    pub parameters_without_value: usize,
    /// `GM-W8003`（値の無いパラメータ）が出た
    pub w8003: bool,
    /// 過剰だが確定できない（`GM-W8004`。離散変数を含む、拘束と重なる、またはインデックスの低減で状態が減る）
    pub uncertain: bool,
}

impl S8 {
    fn new(r: &InitReport, diagnostics: &[Diagnostic]) -> Self {
        Self {
            required: r.required,
            provided: r.provided,
            matched: r.matching.is_some(),
            extra: r.extra(),
            missing: r.missing(),
            parameters_without_value: r.parameters_without_value.len(),
            w8003: diagnostics
                .iter()
                .any(|d| d.code == codes::PARAMETER_WITHOUT_VALUE),
            uncertain: diagnostics
                .iter()
                .any(|d| d.code == codes::INIT_OVERDETERMINED_UNCERTAIN),
        }
    }

    /// 分布の区分。過剰（冗長）が 1 つでもあれば過剰（不足も同時にあるモデルは過剰に数える）、
    /// 無ければ不足があるかで分ける。
    fn balance(&self) -> S8Balance {
        if self.extra > 0 {
            S8Balance::Over
        } else if self.missing > 0 {
            S8Balance::Under
        } else {
            S8Balance::Balanced
        }
    }
}

/// S8 の分布の区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum S8Balance {
    /// 合う
    Balanced,
    /// 過剰（`GM-E8001`）
    Over,
    /// 不足（`GM-W8001`）
    Under,
}

impl S8Balance {
    fn label(self) -> &'static str {
        match self {
            S8Balance::Balanced => "合う",
            S8Balance::Over => "過剰",
            S8Balance::Under => "不足",
        }
    }
}

/// 成功したときの数（状態 / 代数 / 出力は配列を 1 個、方程式と未知数はスカラー）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub states: usize,
    pub algebraics: usize,
    pub outputs: usize,
    pub equations: usize,
    /// スカラーの未知数の総数（S5 の数え方。omc の `checkModel` の変数数と比べる、S008）
    pub unknowns: usize,
}

/// 1 本の結果。
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub model: String,
    pub partial: bool,
    pub verdict: Verdict,
    /// 止まった段階（OK と PANIC は None）
    pub stage: Option<Stage>,
    /// 止まった診断の GM コード
    pub gm_code: Option<&'static str>,
    /// Rumoca のコード（診断の `rumoca: XXX` の注記）
    pub rumoca_code: Option<String>,
    /// 止まった診断の本文、または panic の文言
    pub message: String,
    /// DAE まで変換できた
    pub has_dae: bool,
    pub counts: Option<Counts>,
    pub s6: Option<S6>,
    /// S6 を判定できなかった理由（DAE まで通ったのに構造が無い、など）
    pub s6_note: Option<String>,
    /// S7 の結果（S6 を通ったときだけ）
    pub s7: Option<S7>,
    /// S8 まで来た（S7 までに Error が無い）。S8 の入力が無くても true
    pub s8_reached: bool,
    /// S8 の結果（S8 まで来て、入力 `CheckReport::init` があるときだけ）
    pub s8: Option<S8>,
    pub seconds: f64,
}

impl Row {
    fn panic(example: &Example, message: String, seconds: f64) -> Self {
        Row {
            model: example.name.clone(),
            partial: example.partial,
            verdict: Verdict::Panic,
            stage: None,
            gm_code: None,
            rumoca_code: None,
            message,
            has_dae: false,
            counts: None,
            s6: None,
            s6_note: None,
            s7: None,
            s8_reached: false,
            s8: None,
            seconds,
        }
    }

    /// 段階 `stage` の Error で止めた印を付ける。
    fn stop(&mut self, stage: Stage, code: &'static str, message: String) {
        self.verdict = Verdict::Ng;
        self.stage = Some(stage);
        self.gm_code = Some(code);
        self.message = message;
    }

    /// 検査の結果から判定を出す。Error で止まっていなければ S6〜S8 を回す。
    fn from_report(
        example: &Example,
        report: &CheckReport,
        reference: Option<ReferenceStructure>,
        seconds: f64,
    ) -> Self {
        let mut row = Row {
            model: example.name.clone(),
            partial: example.partial,
            verdict: Verdict::Ok,
            stage: None,
            gm_code: None,
            rumoca_code: None,
            message: String::new(),
            has_dae: report.dae.is_some(),
            counts: report.dae.zip(report.balance).map(|(d, b)| Counts {
                states: d.states,
                algebraics: d.algebraics,
                outputs: d.outputs,
                equations: b.equations,
                unknowns: b.unknowns,
            }),
            s6: None,
            s6_note: None,
            s7: None,
            s8_reached: false,
            s8: None,
            seconds,
        };
        if let Some(d) = first_error(report) {
            row.verdict = if d.is_unsupported() {
                Verdict::Unsupported
            } else {
                Verdict::Ng
            };
            row.stage = Some(d.stage);
            row.gm_code = Some(d.code);
            row.rumoca_code = rumoca_code(d);
            row.message = d.message.clone();
            return row;
        }
        let mut index: Option<IndexReport> = None;
        match &report.structure {
            Some(structure) => {
                let s6 = S6::new(
                    structure,
                    &globe_mode_analysis::analyze(structure),
                    reference,
                );
                if !s6.is_square() {
                    // S5 の釣り合いは離散の変数も数えるので、連続の部分だけの構造は正方とは限らない。
                    // ここは構造特異とは言い切れないので、判定不能にする
                    row.s6_note = Some(format!(
                        "構造が正方でない（方程式 {} 本・未知数 {} 個）",
                        s6.equations, s6.unknowns
                    ));
                } else if !s6.regular {
                    row.stop(
                        Stage::S6,
                        codes::SINGULAR,
                        format!(
                            "構造特異: 方程式 {} 本・未知数 {} 個のうち、マッチングは {}（der の形 {}）",
                            s6.equations, s6.unknowns, s6.structural_matching, s6.matching
                        ),
                    );
                } else {
                    // S7: S6 を通った（正方で、まとめた形に完全マッチングがある）構造だけ
                    let ix = analyze_index(structure);
                    if let Some(d) = first_stage_error(&ix.diagnostics(structure)) {
                        row.stop(Stage::S7, d.code, d.message.clone());
                    }
                    row.s7 = Some(S7::new(structure, &ix));
                    index = Some(ix);
                }
                row.s6 = Some(s6);
            }
            None => {
                row.s6_note = Some(
                    report
                        .notes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "構造（DaeStructure）が無い".to_string()),
                );
            }
        }
        if row.verdict != Verdict::Ok {
            return row;
        }
        // S8: S7 までに Error が無ければ（S6 が判定不能でも、初期化は構造と別に見られる）
        row.s8_reached = true;
        if let Some(init) = &report.init {
            // 要る初期条件の数は、S7 の動的な状態の数で数える（S7 を回せなかったか打ち切ったら無し）。
            // S009 から、S7 の結果と DAE の構造があれば初期化系もマッチングする
            row.s8 = Some(Self::run_s8(
                &mut row,
                init,
                report.structure.as_ref(),
                index.as_ref(),
            ));
        }
        row
    }

    fn run_s8(
        row: &mut Row,
        init: &InitStructure,
        structure: Option<&DaeStructure>,
        index: Option<&IndexReport>,
    ) -> S8 {
        let r = analyze_init_system(init, structure, index);
        let diagnostics = r.diagnostics(init);
        if let Some(d) = first_stage_error(&diagnostics) {
            row.stop(Stage::S8, d.code, d.message.clone());
        }
        S8::new(&r, &diagnostics)
    }

    /// 段階を越えたか（Rumoca の基準値と比べるため）。OK は全段階を越えた。panic は数えない。
    fn passed(&self, stage: Stage) -> bool {
        match self.verdict {
            Verdict::Ok => true,
            Verdict::Panic => false,
            Verdict::Ng | Verdict::Unsupported => self.stage.is_some_and(|s| s > stage),
        }
    }

    /// 進み具合の 1 行（モデル名の前まで）。
    fn status(&self) -> String {
        let rumoca = self
            .rumoca_code
            .as_deref()
            .map_or(String::new(), |c| format!(" ({c})"));
        match self.verdict {
            Verdict::Ok => {
                let mut notes = Vec::new();
                if self.s6_note.is_some() {
                    notes.push("S6 判定不能".to_string());
                } else if self.s6.as_ref().and_then(S6::agrees_with_rumoca) == Some(false) {
                    notes.push("S6 が Rumoca と食い違う".to_string());
                }
                if let Some(x) = self.s7.as_ref().filter(|x| x.index >= 2) {
                    notes.push(format!("{}: インデックス {}", codes::HIGH_INDEX, x.index));
                }
                if let Some(x) = self
                    .s8
                    .as_ref()
                    .filter(|x| x.balance() != S8Balance::Balanced)
                {
                    notes.push(format!("S8 {}", x.balance().label()));
                }
                if notes.is_empty() {
                    "OK".to_string()
                } else {
                    format!("OK（{}）", notes.join("・"))
                }
            }
            Verdict::Ng => format!(
                "NG {} {}{rumoca}",
                self.stage.map_or("-".to_string(), |s| format!("{s:?}")),
                self.gm_code.unwrap_or("-")
            ),
            Verdict::Unsupported => format!("{}{rumoca}", self.gm_code.unwrap_or("GM-U")),
            Verdict::Panic => "PANIC".to_string(),
        }
    }
}

/// 1 つの段階の診断のうち、最初の Error。
fn first_stage_error(diagnostics: &[Diagnostic]) -> Option<&Diagnostic> {
    diagnostics.iter().find(|d| d.severity == Severity::Error)
}

/// 止まった段階の最初の Error。
fn first_error(report: &CheckReport) -> Option<&Diagnostic> {
    let stage = report.failed_stage()?;
    report
        .diagnostics
        .iter()
        .find(|d| d.is_error() && d.stage == stage)
}

fn rumoca_code(d: &Diagnostic) -> Option<String> {
    d.notes
        .iter()
        .find_map(|n| n.strip_prefix("rumoca: "))
        .map(|c| c.rsplit("::").next().unwrap_or(c).trim().to_string())
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "（文字列でない panic）".to_string())
}

// --------------------------------------------------------------------------- 集計

/// ファネルの本数（Rumoca の基準値と同じ切り方）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Funnel {
    pub examples: usize,
    /// 構文（S0）を越えた
    pub parse: usize,
    /// flatten（S1〜S4）を越えた
    pub flatten: usize,
    /// DAE まで変換できた
    pub dae: usize,
    /// 方程式数と変数数が合った（S5 を越えた）
    pub balanced: usize,
}

/// 結果の集計。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Summary {
    pub total: usize,
    pub ok: usize,
    /// OK のうち S6 を判定できなかった本数
    pub ok_without_s6: usize,
    /// S6 を Rumoca 自身の構造解析と照合した本数と、そのうち一致した本数
    pub s6_compared: usize,
    pub s6_agreed: usize,
    /// OK のうち S7 の情報（`GM-I7001`: インデックス > 1 の見込み）が付くもの（S007 から [`IndexReport`] で数える）
    pub s7_info: usize,
    /// S7 を回した本数と、構造インデックスの分布
    pub s7_ran: usize,
    pub s7_index: BTreeMap<IndexBucket, usize>,
    /// S7 を回したモデルのうち、状態選択が要るもの（動的な状態 < 状態の数。`GM-I7002`）
    pub s7_state_selection: usize,
    /// S8 まで来た本数と、その分布（合う / 過剰 / 不足 / 入力なし）
    pub s8_reached: usize,
    pub s8_balanced: usize,
    pub s8_over: usize,
    /// 過剰のうち確定できないもの（`GM-W8004`。OK のまま）
    pub s8_over_uncertain: usize,
    pub s8_under: usize,
    pub s8_no_input: usize,
    /// S8 で `GM-W8003`（値の無いパラメータ）が出たモデル
    pub s8_w8003: usize,
    /// S6 を見たモデルのうち、der の形で未知数を含まない方程式があるもの
    pub s6_with_der_no_unknown_equations: usize,
    /// 段階ごとの NG（GM-E）の本数
    pub ng_by_stage: BTreeMap<Stage, usize>,
    /// GM-U のコードごとの本数
    pub unsupported_by_code: BTreeMap<&'static str, usize>,
    pub panics: usize,
    pub partial: usize,
    pub partial_ok: usize,
    /// Rumoca のコードごとの本数（OK 以外）
    pub rumoca_codes: BTreeMap<String, usize>,
    pub funnel: Funnel,
    /// 検査にかかった時間の合計（MSL の読み込みを除く）
    pub seconds: f64,
}

impl Summary {
    pub(crate) fn from_rows(rows: &[Row]) -> Self {
        let mut s = Summary {
            total: rows.len(),
            ..Summary::default()
        };
        for r in rows {
            s.seconds += r.seconds;
            if r.partial {
                s.partial += 1;
                if r.verdict == Verdict::Ok {
                    s.partial_ok += 1;
                }
            }
            match r.verdict {
                Verdict::Ok => {
                    s.ok += 1;
                    if r.s6_note.is_some() {
                        s.ok_without_s6 += 1;
                    }
                }
                Verdict::Ng => {
                    if let Some(stage) = r.stage {
                        *s.ng_by_stage.entry(stage).or_default() += 1;
                    }
                }
                Verdict::Unsupported => {
                    *s.unsupported_by_code
                        .entry(r.gm_code.unwrap_or("GM-U"))
                        .or_default() += 1;
                }
                Verdict::Panic => s.panics += 1,
            }
            if let Some(c) = &r.rumoca_code {
                *s.rumoca_codes.entry(c.clone()).or_default() += 1;
            }
            if let Some(agree) = r.s6.as_ref().and_then(S6::agrees_with_rumoca) {
                s.s6_compared += 1;
                s.s6_agreed += usize::from(agree);
            }
            if r.s6
                .as_ref()
                .is_some_and(|x| x.der_no_unknown_equations > 0)
            {
                s.s6_with_der_no_unknown_equations += 1;
            }
            if let Some(x) = &r.s7 {
                s.s7_ran += 1;
                *s.s7_index.entry(x.bucket()).or_default() += 1;
                s.s7_state_selection += usize::from(x.needs_state_selection());
                if r.verdict == Verdict::Ok && x.index >= 2 {
                    s.s7_info += 1;
                }
            }
            if r.s8_reached {
                s.s8_reached += 1;
                match r.s8.as_ref().map(S8::balance) {
                    Some(S8Balance::Balanced) => s.s8_balanced += 1,
                    Some(S8Balance::Over) => {
                        s.s8_over += 1;
                        s.s8_over_uncertain +=
                            usize::from(r.s8.as_ref().is_some_and(|x| x.uncertain));
                    }
                    Some(S8Balance::Under) => s.s8_under += 1,
                    None => s.s8_no_input += 1,
                }
                s.s8_w8003 += usize::from(r.s8.as_ref().is_some_and(|x| x.w8003));
            }
        }
        s.funnel = Funnel {
            examples: rows.len(),
            parse: rows.iter().filter(|r| r.passed(Stage::S0)).count(),
            flatten: rows.iter().filter(|r| r.passed(Stage::S4)).count(),
            dae: rows.iter().filter(|r| r.has_dae).count(),
            balanced: rows.iter().filter(|r| r.passed(Stage::S5)).count(),
        };
        s
    }
}

/// 一覧（msl-known-unsupported.txt）の候補: コードだけでは決まらない Rumoca のコードで止まったもの。
/// partial のモデルは、釣り合わなくて当然なので別にする（一覧には入れない）。
pub(crate) fn list_candidates(rows: &[Row]) -> (Vec<String>, Vec<String>) {
    let mut candidates = Vec::new();
    let mut partial = Vec::new();
    for r in rows {
        let Some(code) = &r.rumoca_code else {
            continue;
        };
        if r.verdict != Verdict::Ng || unsupported::classify_code(code).is_some() {
            continue;
        }
        let line = format!("{} {code} # TODO: {}", r.model, one_line(&r.message, 100));
        if r.partial {
            partial.push(line);
        } else {
            candidates.push(line);
        }
    }
    candidates.sort();
    partial.sort();
    (candidates, partial)
}

/// 一覧にあるのに、今回の結果では当たらなかった行（モデルを検査したものだけ）。
fn stale_entries(rows: &[Row]) -> Vec<String> {
    let checked: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.model.as_str(), r)).collect();
    unsupported::known_list()
        .entries
        .iter()
        .filter_map(|e| {
            let r = checked.get(e.model.as_str())?;
            let hit = r.gm_code == Some(codes::KNOWN_LIMITATION)
                && r.rumoca_code.as_deref() == Some(e.code.as_str());
            (!hit).then(|| format!("{} {}（今回: {}）", e.model, e.code, r.status()))
        })
        .collect()
}

/// 改行を空白にし、`max` 文字で切る。
fn one_line(s: &str, max: usize) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > max {
        let cut: String = flat.chars().take(max).collect();
        format!("{cut}…")
    } else {
        flat
    }
}

// --------------------------------------------------------------------------- 実行

/// `msl-funnel` の引数。
#[derive(Debug)]
pub(crate) struct Options<'a> {
    pub msl: &'a Path,
    pub limit: Option<usize>,
    pub filter: Option<&'a str>,
    pub report: Option<&'a Path>,
    pub json: Option<&'a Path>,
    pub list: bool,
    /// N 本ごとに `Checker::reset` で Session を小さく戻す（0 なら戻さない）
    pub reset_every: usize,
}

pub(crate) fn msl_funnel(opts: &Options<'_>) -> Result<ExitCode> {
    if !opts.msl.join(".globe-mode-msl").exists() {
        return Err(format!(
            "{} に MSL が無い。先に `cargo xtask fetch-msl` を実行する",
            opts.msl.display()
        ));
    }
    let t = Instant::now();
    let all = msl_examples(opts.msl)?;
    let index_seconds = t.elapsed().as_secs_f64();
    let partial = all.iter().filter(|e| e.partial).count();
    let count_note = if all.len() == RUMOCA_BASELINE.examples {
        format!("Rumoca の基準値 {} 本と一致", RUMOCA_BASELINE.examples)
    } else {
        format!(
            "**Rumoca の基準値 {} 本と違う**（差 {:+}）",
            RUMOCA_BASELINE.examples,
            all.len() as i64 - RUMOCA_BASELINE.examples as i64
        )
    };
    let count_note = format!(
        "{count_note}。うち partial {partial} 本（Rumoca の基準値は {RUMOCA_BASELINE_PARTIAL} 本）"
    );
    let selected: Vec<Example> = all
        .iter()
        .filter(|e| opts.filter.is_none_or(|f| e.name.contains(f)))
        .take(opts.limit.unwrap_or(usize::MAX))
        .cloned()
        .collect();
    if opts.list {
        for e in &selected {
            println!("{}", e.name);
        }
        eprintln!(
            "例題 {} 本（{count_note}）。表示 {} 本",
            all.len(),
            selected.len()
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!(
        "例題 {} 本（{count_note}、索引 {index_seconds:.1} s）。検査するのは {} 本",
        all.len(),
        selected.len()
    );

    let t = Instant::now();
    let checker = make_checker(opts.msl)?;
    let setup_seconds = t.elapsed().as_secs_f64();
    println!("MSL を読み込んだ（{setup_seconds:.1} s）");
    let mut resets = 0usize;
    let mut reset_seconds = 0.0;
    let mut rows = Vec::with_capacity(selected.len());
    let width = selected.len().to_string().len();
    for (i, e) in selected.iter().enumerate() {
        let t = Instant::now();
        // panic で Mutex が毒されても、Checker は次の検査でライブラリを読んだ直後の Session に戻す
        let outcome = catch_unwind(AssertUnwindSafe(|| check_one(&checker, &e.name)));
        let seconds = t.elapsed().as_secs_f64();
        let row = match outcome {
            Ok((report, reference)) => Row::from_report(e, &report, reference, seconds),
            Err(payload) => Row::panic(e, panic_message(payload.as_ref()), seconds),
        };
        println!(
            "[{:>width$}/{}] {} {}{}  {seconds:.2} s",
            i + 1,
            selected.len(),
            row.status(),
            e.name,
            if e.partial { "（partial）" } else { "" },
        );
        if row.verdict == Verdict::Panic {
            println!("    panic: {}", one_line(&row.message, 300));
        }
        let _ = std::io::stdout().flush();
        rows.push(row);
        // Rumoca のキャッシュで Session が大きくなる（566 本で 7.8 GB）ので、ときどき読み込んだ直後に戻す
        if opts.reset_every > 0 && (i + 1) % opts.reset_every == 0 && i + 1 < selected.len() {
            let t = Instant::now();
            checker.reset();
            reset_seconds += t.elapsed().as_secs_f64();
            resets += 1;
        }
    }
    let peak_rss_kb = peak_rss_kb();

    let summary = Summary::from_rows(&rows);
    let run = RunInfo {
        enumerated: all.len(),
        count_note,
        opts,
        index_seconds,
        setup_seconds,
        resets,
        reset_seconds,
        peak_rss_kb,
    };
    println!("{}", summary_line(&summary));
    println!("{}", s7_s8_line(&summary));
    println!("時間: {}", time_line(&summary, &run));
    if let Some(kb) = peak_rss_kb {
        println!("最大の常駐メモリ（VmHWM）: {}", gigabytes(kb));
    }
    if let Some(path) = opts.report {
        fs::write(path, render_report(&rows, &summary, &run)).map_err(|e| e.to_string())?;
        println!("報告: {}", path.display());
    }
    if let Some(path) = opts.json {
        let text = serde_json::to_string_pretty(&render_json(&rows, &summary, &run))
            .map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())?;
        println!("JSON: {}", path.display());
    }
    Ok(if summary.panics == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// 報告に書く実行の情報。
struct RunInfo<'a> {
    enumerated: usize,
    count_note: String,
    opts: &'a Options<'a>,
    index_seconds: f64,
    /// 検査器の用意（MSL の読み込み）
    setup_seconds: f64,
    /// `Checker::reset` を呼んだ回数と、その時間
    resets: usize,
    reset_seconds: f64,
    /// 最大の常駐メモリ（Linux の `/proc/self/status` の VmHWM。読めなければ None）
    peak_rss_kb: Option<u64>,
}

/// 最大の常駐メモリ（kB）。Linux の `/proc/self/status` の `VmHWM` を読む。ほかの OS では None。
fn peak_rss_kb() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    parse_vm_hwm(&status)
}

fn parse_vm_hwm(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
}

fn gigabytes(kb: u64) -> String {
    format!("{:.1} GiB", kb as f64 / (1024.0 * 1024.0))
}

fn time_line(s: &Summary, run: &RunInfo<'_>) -> String {
    format!(
        "検査 {:.1} s（1 本あたり {:.2} s）・MSL の読み込み {:.1} s・Session を戻す {} 回 {:.1} s（{} 本ごと）・例題の索引 {:.1} s",
        s.seconds,
        s.seconds / s.total.max(1) as f64,
        run.setup_seconds,
        run.resets,
        run.reset_seconds,
        run.opts.reset_every,
        run.index_seconds
    )
}

fn summary_line(s: &Summary) -> String {
    let ng: usize = s.ng_by_stage.values().sum();
    let u: usize = s.unsupported_by_code.values().sum();
    format!(
        "{} 本: OK {}（うち S6 判定不能 {}・S7 の情報 {}）・NG {ng}・GM-U {u}・panic {}・S6 の Rumoca との照合 {}/{} 一致",
        s.total, s.ok, s.ok_without_s6, s.s7_info, s.panics, s.s6_agreed, s.s6_compared
    )
}

/// S7・S8 の 1 行の要約（S007 から）。
fn s7_s8_line(s: &Summary) -> String {
    let index = IndexBucket::ALL
        .iter()
        .map(|b| format!("{} {}", b.label(), s.s7_index.get(b).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("・");
    format!(
        "S7 {} 本（インデックス {index}、状態選択が要る {}）・S8 {} 本（合う {}・過剰 {}（うち見込み {}）・不足 {}・入力なし {}、{} あり {}）",
        s.s7_ran,
        s.s7_state_selection,
        s.s8_reached,
        s.s8_balanced,
        s.s8_over,
        s.s8_over_uncertain,
        s.s8_under,
        s.s8_no_input,
        codes::PARAMETER_WITHOUT_VALUE,
        s.s8_w8003
    )
}

const STAGES: [Stage; 9] = [
    Stage::S0,
    Stage::S1,
    Stage::S2,
    Stage::S3,
    Stage::S4,
    Stage::S5,
    Stage::S6,
    Stage::S7,
    Stage::S8,
];

fn render_report(rows: &[Row], s: &Summary, run: &RunInfo<'_>) -> String {
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# MSL {MSL_VERSION} の全例題のファネル（globe check）\n"
    );
    let _ = writeln!(
        md,
        "`cargo xtask msl-funnel` が生成した。手で編集しない。\n"
    );
    let _ = writeln!(md, "| | |\n| --- | --- |");
    let _ = writeln!(
        md,
        "| MSL | {MSL_VERSION}（sha256 `{}…`） |",
        &MSL_SHA256[..12]
    );
    let _ = writeln!(
        md,
        "| Rumoca | {RUMOCA_VERSION}（rev `{}`、ライブラリとして呼ぶ） |",
        &RUMOCA_REV[..8]
    );
    let _ = writeln!(md, "| 例題 | {} 本（{}） |", run.enumerated, run.count_note);
    let mut selection = Vec::new();
    if let Some(f) = run.opts.filter {
        selection.push(format!("`--filter {f}`"));
    }
    if let Some(n) = run.opts.limit {
        selection.push(format!("`--limit {n}`"));
    }
    let _ = writeln!(
        md,
        "| 検査した本数 | {} 本{} |",
        s.total,
        if selection.is_empty() {
            "（全部）".to_string()
        } else {
            format!("（{}）", selection.join("・"))
        }
    );
    let _ = writeln!(md, "| 検査器 | {CHECKER_KIND} |");
    let _ = writeln!(
        md,
        "| 段階 | S0〜S8。**S007 から S7（インデックス、Pantelides 法）・S8（初期化）を含む**（S005 の報告は S0〜S6。OK は S7・S8 の Error（{}・{}）が無いもの） |",
        codes::INDEX_REDUCTION_FAILED,
        codes::INIT_OVERDETERMINED
    );
    let _ = writeln!(md, "| 結果 | **{}** |", summary_line(s));
    let _ = writeln!(md, "| S7・S8 | {} |\n", s7_s8_line(s));

    let _ = writeln!(md, "## 集計\n");
    let _ = writeln!(md, "| 項目 | 本数 |\n| --- | ---: |");
    let _ = writeln!(md, "| 対象 | {} |", s.total);
    let _ = writeln!(
        md,
        "| OK（S0〜S8 で Error 無し。S005 の報告では S0〜S6） | {} |",
        s.ok
    );
    let _ = writeln!(
        md,
        "| うち S6 判定不能（構造が無い・正方でない） | {} |",
        s.ok_without_s6
    );
    let _ = writeln!(
        md,
        "| うち S7 の情報 {}（インデックス > 1 の見込み） | {} |",
        codes::HIGH_INDEX,
        s.s7_info
    );
    for stage in STAGES {
        let _ = writeln!(
            md,
            "| NG {stage:?}（{}） | {} |",
            stage.title(),
            s.ng_by_stage.get(&stage).copied().unwrap_or(0)
        );
    }
    for (code, title) in [
        (codes::UNSUPPORTED, "コンパイラが未対応"),
        (codes::COMPILER_BUG, "コンパイラの内部エラー"),
        (codes::KNOWN_LIMITATION, "既知の制約の一覧"),
    ] {
        let _ = writeln!(
            md,
            "| {code}（{title}） | {} |",
            s.unsupported_by_code.get(code).copied().unwrap_or(0)
        );
    }
    let _ = writeln!(md, "| panic | {} |", s.panics);
    let _ = writeln!(
        md,
        "| S6 の der の形のマッチングを Rumoca の構造解析（`phase_structural::analyze`）と照合（一致 / 照合した本数） | {} / {} |",
        s.s6_agreed, s.s6_compared
    );
    let _ = writeln!(
        md,
        "| S6 を見たモデルのうち、der の形で未知数を含まない方程式（状態・パラメータ・入力だけの拘束）があるもの | {} |",
        s.s6_with_der_no_unknown_equations
    );
    let _ = writeln!(
        md,
        "| S7（Pantelides 法）を回した本数（S6 を通ったもの） | {} |",
        s.s7_ran
    );
    for b in IndexBucket::ALL {
        let _ = writeln!(
            md,
            "| うち構造インデックス {} | {} |",
            b.label(),
            s.s7_index.get(&b).copied().unwrap_or(0)
        );
    }
    let _ = writeln!(
        md,
        "| うち状態選択（dummy derivative）が要るもの（{}: 動的な状態 < 状態の数） | {} |",
        codes::STATE_SELECTION,
        s.s7_state_selection
    );
    let _ = writeln!(
        md,
        "| S8（初期化）まで来た本数（S7 までに Error が無いもの） | {} |",
        s.s8_reached
    );
    for (label, n) in [
        ("うち初期条件の数が合う".to_string(), s.s8_balanced),
        (
            format!(
                "うち初期条件が過剰（{}。うち確定できず警告にとどめた {} が {} 本）",
                codes::INIT_OVERDETERMINED,
                codes::INIT_OVERDETERMINED_UNCERTAIN,
                s.s8_over_uncertain
            ),
            s.s8_over,
        ),
        (
            format!("うち初期条件が不足（{}）", codes::INIT_UNDERDETERMINED),
            s.s8_under,
        ),
        (
            "うち S8 の入力なし（初期化系の構造を作れなかった）".to_string(),
            s.s8_no_input,
        ),
    ] {
        let _ = writeln!(md, "| {label} | {n} |");
    }
    let _ = writeln!(
        md,
        "| うち {}（値の無いパラメータ）が出たもの | {} |",
        codes::PARAMETER_WITHOUT_VALUE,
        s.s8_w8003
    );
    let _ = writeln!(
        md,
        "| partial（参考。うち OK） | {}（{}） |",
        s.partial, s.partial_ok
    );
    let _ = writeln!(md, "| 時間 | {} |", time_line(s, run));
    let _ = writeln!(
        md,
        "| 最大の常駐メモリ（VmHWM） | {} |\n",
        run.peak_rss_kb.map_or("-".to_string(), gigabytes)
    );

    let _ = writeln!(md, "## Rumoca の基準値との比較\n");
    let _ = writeln!(
        md,
        "Rumoca の基準値は `crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json`（rev `{}` に同梱）。",
        &RUMOCA_REV[..8]
    );
    let _ = writeln!(
        md,
        "GlobeMode の列は、構文 = S0 を越えた、flatten = S4 を越えた（S5 以降で止まったか OK）、DAE = DAE まで変換できた、釣り合い = S5 を越えた、の本数。panic は数えない。\n"
    );
    let _ = writeln!(
        md,
        "| | 例題 | 構文 | flatten | DAE | 釣り合い |\n| --- | ---: | ---: | ---: | ---: | ---: |"
    );
    for (label, f) in [
        ("Rumoca の基準値", RUMOCA_BASELINE),
        ("GlobeMode（この実行）", s.funnel),
    ] {
        let _ = writeln!(
            md,
            "| {label} | {} | {} | {} | {} | {} |",
            f.examples, f.parse, f.flatten, f.dae, f.balanced
        );
    }
    let _ = writeln!(md);

    let _ = writeln!(md, "## Rumoca のエラーコードの度数\n");
    if s.rumoca_codes.is_empty() {
        let _ = writeln!(md, "（Rumoca のコード付きの失敗は無い）\n");
    } else {
        let _ = writeln!(md, "| コード | 本数 | 振り分け |\n| --- | ---: | --- |");
        let mut codes_by_count: Vec<(&String, &usize)> = s.rumoca_codes.iter().collect();
        codes_by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (code, n) in codes_by_count {
            let class = match unsupported::classify_code(code) {
                Some(u) => format!("{}（コードで決まる）", u.code),
                None => "コードだけでは決めない（一覧で決める）".to_string(),
            };
            let _ = writeln!(md, "| {code} | {n} | {class} |");
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## 既知の制約の一覧の原稿\n");
    let _ = writeln!(
        md,
        "コードだけでは決まらない Rumoca のコード（ED001 など）で止まった（モデル, コード）の組。理由を確かめて書き直してから `src/Modelica/msl-known-unsupported.txt` に移す（ADR-0007）。\n"
    );
    let (candidates, partial) = list_candidates(rows);
    if candidates.is_empty() && partial.is_empty() {
        let _ = writeln!(md, "（候補は無い）\n");
    } else {
        let _ = writeln!(md, "```text");
        for line in &candidates {
            let _ = writeln!(md, "{line}");
        }
        if !partial.is_empty() {
            let _ = writeln!(
                md,
                "# 以下は partial のモデル（釣り合わなくて当然なので、一覧には入れない）"
            );
            for line in &partial {
                let _ = writeln!(md, "# {line}");
            }
        }
        let _ = writeln!(md, "```\n");
    }
    let stale = stale_entries(rows);
    if !stale.is_empty() {
        let _ = writeln!(md, "一覧にあるのに、今回は当たらなかった行:\n");
        for line in stale {
            let _ = writeln!(md, "- `{line}`");
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## モデルごとの結果\n");
    let _ = writeln!(
        md,
        "- 数は、状態 / 代数 / 出力（配列は 1 個）と、方程式（スカラー。S5 の釣り合いで数える本数で、離散の方程式を含む）。\n- S6 は、マッチングの大きさ / 方程式の数（連続の部分だけ。未知数は `der(x)` と代数・出力のスカラー）。構造特異の判定は `x` と `der(x)` を 1 つの列にまとめた形で行う。der の形（`der(x)` だけを未知数に数える形）のマッチングが完全でないときは括弧に書き、S7 の情報 `GM-I7001`（インデックス > 1 の見込み）を付ける。der の形のマッチングが Rumoca の構造解析と食い違うときは、その数も括弧に書く。\n- S7（S007 から）は、Pantelides 法の構造インデックスの見込み（`index k`）と、微分した方程式の数・微分の最大回数・状態選択の候補の状態の数・動的な状態の見込み / 状態の数（動的な状態が少なければ `GM-I7002`）。打ち切りは `GM-E7001`。\n- S8（S007 から）は、与えられた初期条件の数 / 要る数と、合う・過剰（`GM-E8001`。離散変数を含むかインデックスの低減で状態が減るときは確定できないので、過剰の見込み `GM-W8004` の警告にとどめる）・不足（`GM-W8001`）。`GM-W8003` は値の無いパラメータがあること。入力なしは初期化系の構造を作れなかったもの。\n- 秒は、検査（と S6 の照合）にかかった時間。MSL の読み込みと S6〜S8 の解析は含まない。\n"
    );
    let _ = writeln!(
        md,
        "| # | モデル | 判定 | 段階 | GM コード | Rumoca | 状態 / 代数 / 出力・方程式 | S6 | S7 | S8 | 秒 |"
    );
    let _ = writeln!(
        md,
        "| ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: |"
    );
    for (i, r) in rows.iter().enumerate() {
        let short = r.model.strip_prefix("Modelica.").unwrap_or(&r.model);
        let verdict = match r.verdict {
            Verdict::Ok => "OK",
            Verdict::Ng => "NG",
            Verdict::Unsupported => "GM-U",
            Verdict::Panic => "**PANIC**",
        };
        let counts = r.counts.map_or("-".to_string(), |c| {
            format!(
                "{} / {} / {}・{}",
                c.states, c.algebraics, c.outputs, c.equations
            )
        });
        let s6 = match (&r.s6, &r.s6_note) {
            (Some(x), None) => {
                let head = if x.regular { "正則" } else { "**特異**" };
                let mut notes = Vec::new();
                if x.matching != x.equations {
                    notes.push(format!("der {}/{}", x.matching, x.equations));
                }
                if x.der_no_unknown_equations > 0 {
                    notes.push(format!(
                        "der の形で未知数を含まない方程式 {}",
                        x.der_no_unknown_equations
                    ));
                }
                if let (Some(false), Some((e, u, m))) = (x.agrees_with_rumoca(), x.rumoca) {
                    notes.push(format!("**Rumoca の der: {m}/{e}、未知数 {u}**"));
                }
                let notes = if notes.is_empty() {
                    String::new()
                } else {
                    format!("（{}）", notes.join("・"))
                };
                format!("{head} {}/{}{notes}", x.structural_matching, x.equations)
            }
            (Some(x), Some(_)) => format!("判定不能（{} × {}）", x.equations, x.unknowns),
            (None, Some(_)) => "判定不能".to_string(),
            (None, None) => "-".to_string(),
        };
        let s7 = r.s7.as_ref().map_or("-".to_string(), s7_cell);
        let s8 = match (&r.s8, r.s8_reached) {
            (Some(x), _) => s8_cell(x),
            (None, true) => "入力なし".to_string(),
            (None, false) => "-".to_string(),
        };
        let _ = writeln!(
            md,
            "| {} | `{short}`{} | {verdict} | {} | {} | {} | {counts} | {s6} | {s7} | {s8} | {:.2} |",
            i + 1,
            if r.partial { "（partial）" } else { "" },
            r.stage.map_or("-".to_string(), |s| format!("{s:?}")),
            match r.gm_code {
                Some(c) => c.to_string(),
                None if r.s7.as_ref().is_some_and(|x| x.index >= 2) => {
                    format!("{}（情報）", codes::HIGH_INDEX)
                }
                None => "-".to_string(),
            },
            r.rumoca_code.as_deref().unwrap_or("-"),
            r.seconds
        );
    }
    md
}

/// 報告の S7 の欄。
fn s7_cell(x: &S7) -> String {
    if x.failed {
        return format!("**打ち切り**（{}）", codes::INDEX_REDUCTION_FAILED);
    }
    let mut cell = format!("index {}", x.index);
    if x.n_differentiated > 0 {
        cell.push_str(&format!(
            "（微分 {} 本・最大 {} 回・候補 {}・動的 {}/{}）",
            x.n_differentiated,
            x.max_differentiations,
            x.dummy_candidates,
            x.dynamic_states,
            x.states
        ));
    }
    cell
}

/// 報告の S8 の欄。
fn s8_cell(x: &S8) -> String {
    let head = match x.balance() {
        S8Balance::Balanced => "合う".to_string(),
        S8Balance::Over if x.uncertain => {
            format!("過剰の見込み（{}）", codes::INIT_OVERDETERMINED_UNCERTAIN)
        }
        S8Balance::Over => format!("**過剰**（{}）", codes::INIT_OVERDETERMINED),
        S8Balance::Under => format!("不足（{}）", codes::INIT_UNDERDETERMINED),
    };
    let mut cell = format!("{head} {}/{}", x.provided, x.required);
    if x.matched && (x.extra > 0 || x.missing > 0) {
        // S009: マッチングで分かった冗長と不足（数の差は 不足 − 冗長）
        cell.push_str(&format!("（冗長 {}・決まらない {}）", x.extra, x.missing));
    }
    if x.w8003 {
        cell.push_str(&format!(
            "・{} {}",
            codes::PARAMETER_WITHOUT_VALUE,
            x.parameters_without_value
        ));
    }
    cell
}

fn render_json(rows: &[Row], s: &Summary, run: &RunInfo<'_>) -> serde_json::Value {
    use serde_json::json;
    let models: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "model": r.model,
                "partial": r.partial,
                "verdict": format!("{:?}", r.verdict),
                "stage": r.stage.map(|s| format!("{s:?}")),
                "gm_code": r.gm_code,
                "rumoca_code": r.rumoca_code,
                "message": r.message,
                "has_dae": r.has_dae,
                "counts": r.counts.map(|c| json!({
                    "states": c.states,
                    "algebraics": c.algebraics,
                    "outputs": c.outputs,
                    "equations": c.equations,
                    "unknowns": c.unknowns,
                })),
                "s6": r.s6.as_ref().map(|x| json!({
                    "equations": x.equations,
                    "unknowns": x.unknowns,
                    "matching": x.matching,
                    "structural_matching": x.structural_matching,
                    "regular": x.regular,
                    "index_reduction_needed": x.index_reduction_needed,
                    "der_no_unknown_equations": x.der_no_unknown_equations,
                    "rumoca": x.rumoca.map(|(e, u, m)| json!({
                        "equations": e,
                        "unknowns": u,
                        "matching": m,
                    })),
                })),
                "s6_note": r.s6_note,
                "s7": r.s7.as_ref().map(|x| json!({
                    "index": x.index,
                    "n_differentiated": x.n_differentiated,
                    "max_differentiations": x.max_differentiations,
                    "dummy_candidates": x.dummy_candidates,
                    "states": x.states,
                    "dynamic_states": x.dynamic_states,
                    "failed": x.failed,
                })),
                "s8_reached": r.s8_reached,
                "s8": r.s8.as_ref().map(|x| json!({
                    "required": x.required,
                    "provided": x.provided,
                    "matched": x.matched,
                    "extra": x.extra,
                    "missing": x.missing,
                    "balance": x.balance().label(),
                    "parameters_without_value": x.parameters_without_value,
                    "w8003": x.w8003,
                    "uncertain": x.uncertain,
                })),
                "seconds": r.seconds,
            })
        })
        .collect();
    let funnel = |f: Funnel| {
        json!({
            "examples": f.examples,
            "parse": f.parse,
            "flatten": f.flatten,
            "dae": f.dae,
            "balanced": f.balanced,
        })
    };
    json!({
        "msl_version": MSL_VERSION,
        "rumoca_rev": RUMOCA_REV,
        "enumerated": run.enumerated,
        "checker": CHECKER_KIND,
        "summary": {
            "total": s.total,
            "ok": s.ok,
            "ok_without_s6": s.ok_without_s6,
            "ng_by_stage": s.ng_by_stage.iter().map(|(k, v)| (format!("{k:?}"), *v)).collect::<BTreeMap<_, _>>(),
            "unsupported_by_code": s.unsupported_by_code,
            "panics": s.panics,
            "partial": s.partial,
            "partial_ok": s.partial_ok,
            "rumoca_codes": s.rumoca_codes,
            "funnel": funnel(s.funnel),
            "rumoca_baseline": funnel(RUMOCA_BASELINE),
            "seconds": s.seconds,
            "setup_seconds": run.setup_seconds,
            "reset_every": run.opts.reset_every,
            "resets": run.resets,
            "reset_seconds": run.reset_seconds,
            "peak_rss_kb": run.peak_rss_kb,
            "s7_info": s.s7_info,
            "s7_ran": s.s7_ran,
            "s7_index": IndexBucket::ALL
                .iter()
                .map(|b| (b.key(), s.s7_index.get(b).copied().unwrap_or(0)))
                .collect::<BTreeMap<_, _>>(),
            "s7_state_selection": s.s7_state_selection,
            "s8_reached": s.s8_reached,
            "s8_balanced": s.s8_balanced,
            "s8_over": s.s8_over,
            "s8_over_uncertain": s.s8_over_uncertain,
            "s8_under": s.s8_under,
            "s8_no_input": s.s8_no_input,
            "s8_w8003": s.s8_w8003,
            "stages": "S0-S8",
            "index_seconds": run.index_seconds,
            "s6_compared": s.s6_compared,
            "s6_agreed": s.s6_agreed,
            "s6_with_der_no_unknown_equations": s.s6_with_der_no_unknown_equations,
        },
        "models": models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_core::TextRange;
    use globe_mode_ir::{
        BalanceCount, Causality, ClassOutline, DaeCounts, DaeStructure, EquationOrigin,
        EquationRow, InitVariable, Outline, Unknown, UnknownKind,
    };

    fn class(name: &str, kind: ClassKind, classes: Vec<ClassOutline>) -> ClassOutline {
        ClassOutline {
            name: name.to_string(),
            kind,
            partial: false,
            causality: Causality::None,
            range: TextRange::default(),
            extends: Vec::new(),
            imports: Vec::new(),
            components: Vec::new(),
            connects: Vec::new(),
            classes,
        }
    }

    fn package(name: &str, classes: Vec<ClassOutline>) -> ClassOutline {
        class(name, ClassKind::Package, classes)
    }

    fn model(name: &str) -> ClassOutline {
        class(name, ClassKind::Model, Vec::new())
    }

    fn index() -> ClassIndex {
        let mut partial = model("PartialExample");
        partial.partial = true;
        let mut with_nested = model("WithNested");
        with_nested.classes.push(model("Nested"));
        let examples = package(
            "Examples",
            vec![
                model("First"),
                partial,
                with_nested,
                class("AsBlock", ClassKind::Block, Vec::new()),
                class("AsClass", ClassKind::Class, Vec::new()),
                class("ARecord", ClassKind::Record, Vec::new()),
                class("AFunction", ClassKind::Function, Vec::new()),
                class("AConnector", ClassKind::Connector, Vec::new()),
                package("Group", vec![model("Deep")]),
                package("Utilities", vec![model("Helper")]),
                package("BaseClasses", vec![model("Base")]),
                package("Internal", vec![model("Hidden")]),
                package("Interfaces", vec![model("Port")]),
                package("Group2", vec![package("Utilities", vec![model("X")])]),
                // パッケージでないクラスの中のモデルは数えない
                class("NotAPackage", ClassKind::Model, vec![model("Inside")]),
            ],
        );
        let blocks = package(
            "Blocks",
            vec![examples, package("Sources", vec![model("Sine")])],
        );
        // 別のファイル（within Modelica.Mechanics;）に分かれた例題
        let mechanics = Outline {
            within: Some("Modelica.Mechanics".to_string()),
            classes: vec![package("Examples", vec![model("Pendulum")])],
        };
        let modelica = Outline {
            within: None,
            classes: vec![package(
                "Modelica",
                vec![blocks, package("Mechanics", Vec::new())],
            )],
        };
        let other = Outline {
            within: None,
            classes: vec![package(
                "Other",
                vec![package("Examples", vec![model("Foreign")])],
            )],
        };
        ClassIndex::from_outlines([&modelica, &mechanics, &other])
    }

    #[test]
    fn example_rule_mirrors_rumoca() {
        let got = examples_in(&index());
        let names: Vec<&str> = got.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Modelica.Blocks.Examples.AsBlock",
                "Modelica.Blocks.Examples.AsClass",
                "Modelica.Blocks.Examples.First",
                "Modelica.Blocks.Examples.Group.Deep",
                "Modelica.Blocks.Examples.NotAPackage",
                "Modelica.Blocks.Examples.PartialExample",
                "Modelica.Blocks.Examples.WithNested",
                "Modelica.Mechanics.Examples.Pendulum",
            ]
        );
        // partial は除かずに印を付ける（Rumoca の 566 本にも含まれる）
        let partial: Vec<&str> = got
            .iter()
            .filter(|e| e.partial)
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(partial, ["Modelica.Blocks.Examples.PartialExample"]);
    }

    #[test]
    fn builtin_operators_are_not_examples() {
        assert!(is_builtin_operator(
            "Modelica.X.Examples.Connections.branch"
        ));
        assert!(!is_builtin_operator("Modelica.Blocks.Examples.Filter"));
    }

    fn row(model: &str, verdict: Verdict, stage: Option<Stage>, code: Option<&str>) -> Row {
        Row {
            model: model.to_string(),
            partial: false,
            verdict,
            stage,
            gm_code: match verdict {
                Verdict::Ok | Verdict::Panic => None,
                Verdict::Ng => Some(codes::UNBALANCED),
                Verdict::Unsupported => Some(codes::UNSUPPORTED),
            },
            rumoca_code: code.map(str::to_string),
            message: "unbalanced model:\n 3 equations".to_string(),
            has_dae: matches!(verdict, Verdict::Ok),
            counts: None,
            s6: matches!(verdict, Verdict::Ok).then_some(S6 {
                equations: 3,
                unknowns: 3,
                matching: 3,
                structural_matching: 3,
                regular: true,
                index_reduction_needed: false,
                der_no_unknown_equations: 0,
                rumoca: Some((3, 3, 3)),
            }),
            s6_note: None,
            s7: matches!(verdict, Verdict::Ok).then_some(S7 {
                index: 1,
                n_differentiated: 0,
                max_differentiations: 0,
                dummy_candidates: 0,
                states: 0,
                dynamic_states: 0,
                failed: false,
            }),
            s8_reached: matches!(verdict, Verdict::Ok),
            s8: None,
            seconds: 0.5,
        }
    }

    #[test]
    fn summary_counts_verdicts_and_funnel() {
        let mut no_s6 = row("M.Ok2", Verdict::Ok, None, None);
        no_s6.s6 = None;
        no_s6.s6_note = Some("構造が無い".into());
        no_s6.partial = true;
        no_s6.s7 = None;
        let mut high_index = row("M.Index", Verdict::Ok, None, None);
        if let Some(x) = high_index.s6.as_mut() {
            x.matching = 2;
            x.index_reduction_needed = true;
            x.der_no_unknown_equations = 1;
            x.rumoca = Some((3, 3, 2));
        }
        high_index.s7 = Some(S7 {
            index: 3,
            n_differentiated: 3,
            max_differentiations: 2,
            dummy_candidates: 4,
            states: 4,
            dynamic_states: 2,
            failed: false,
        });
        high_index.s8 = Some(S8 {
            required: 2,
            provided: 2,
            matched: false,
            extra: 0,
            missing: 0,
            parameters_without_value: 1,
            w8003: true,
            uncertain: false,
        });
        let mut under = row("M.Under", Verdict::Ok, None, None);
        under.s8 = Some(S8 {
            required: 3,
            provided: 1,
            matched: false,
            extra: 0,
            missing: 2,
            parameters_without_value: 0,
            w8003: false,
            uncertain: false,
        });
        // S8 で止まった（GM-E8001）: S7 は回した
        let mut over = row("M.Over", Verdict::Ok, None, None);
        over.verdict = Verdict::Ng;
        over.stage = Some(Stage::S8);
        over.gm_code = Some(codes::INIT_OVERDETERMINED);
        over.s8 = Some(S8 {
            required: 1,
            provided: 2,
            matched: false,
            extra: 1,
            missing: 0,
            parameters_without_value: 0,
            w8003: false,
            uncertain: false,
        });
        let rows = vec![
            row("M.Ok", Verdict::Ok, None, None),
            no_s6,
            high_index,
            under,
            over,
            row("M.S5", Verdict::Ng, Some(Stage::S5), Some("ED001")),
            row("M.S3", Verdict::Ng, Some(Stage::S3), Some("EI008")),
            row("M.U", Verdict::Unsupported, Some(Stage::S5), Some("ED019")),
            row("M.P", Verdict::Panic, None, None),
        ];
        let s = Summary::from_rows(&rows);
        assert_eq!(s.total, 9);
        assert_eq!(s.ok, 4);
        assert_eq!(s.ok_without_s6, 1);
        // S7 の情報は IndexReport のインデックスで数える（OK のものだけ）
        assert_eq!(s.s7_info, 1);
        assert_eq!(s.s7_ran, 4);
        assert_eq!(s.s7_index.get(&IndexBucket::One), Some(&3));
        assert_eq!(s.s7_index.get(&IndexBucket::Three), Some(&1));
        assert_eq!(s.s7_state_selection, 1);
        assert_eq!(s.ng_by_stage.get(&Stage::S8), Some(&1));
        // S8: 来たのは OK の 4 本と S8 で止まった 1 本。入力なしは S8 を持たない OK の 2 本
        assert_eq!(s.s8_reached, 5);
        assert_eq!(
            (
                s.s8_balanced,
                s.s8_over,
                s.s8_under,
                s.s8_no_input,
                s.s8_w8003
            ),
            (1, 1, 1, 2, 1)
        );
        let line = s7_s8_line(&s);
        assert!(line.starts_with("S7 4 本（インデックス 1 3・2 0・3 1・4 以上 0・打ち切り（GM-E7001） 0、状態選択が要る 1）"), "{line}");
        assert_eq!(s.s6_with_der_no_unknown_equations, 1);
        // Rumoca とは der の形のマッチングで照合する
        assert_eq!((s.s6_agreed, s.s6_compared), (4, 4));
        assert_eq!(s.ng_by_stage.get(&Stage::S5), Some(&1));
        assert_eq!(s.ng_by_stage.get(&Stage::S3), Some(&1));
        assert_eq!(s.unsupported_by_code.get(codes::UNSUPPORTED), Some(&1));
        assert_eq!(s.panics, 1);
        assert_eq!((s.partial, s.partial_ok), (1, 1));
        assert_eq!(s.rumoca_codes.get("ED001"), Some(&1));
        assert_eq!(s.rumoca_codes.len(), 3);
        assert!((s.seconds - 4.5).abs() < 1e-9);
        assert_eq!(
            s.funnel,
            Funnel {
                examples: 9,
                parse: 8,    // panic だけ数えない
                flatten: 7,  // S3 で止まったものと panic を除く
                dae: 5,      // 試験の行では OK（として作った行）だけが DAE を持つ
                balanced: 5, // S5 で止まったもの（GM-U を含む）を除く
            }
        );
    }

    #[test]
    fn list_candidates_only_take_ambiguous_codes() {
        let mut partial = row("M.Partial", Verdict::Ng, Some(Stage::S5), Some("ED001"));
        partial.partial = true;
        let rows = vec![
            row("M.B", Verdict::Ng, Some(Stage::S5), Some("ED001")),
            row("M.A", Verdict::Ng, Some(Stage::S4), Some("EF002")),
            // コードで決まるもの（ここに来るのは A の付け替えが無いときだけ）は候補にしない
            row("M.Code", Verdict::Ng, Some(Stage::S5), Some("ED008")),
            row("M.U", Verdict::Unsupported, Some(Stage::S5), Some("ED019")),
            row("M.NoCode", Verdict::Ng, Some(Stage::S1), None),
            row("M.Ok", Verdict::Ok, None, None),
            partial,
        ];
        let (candidates, partial) = list_candidates(&rows);
        assert_eq!(
            candidates,
            [
                "M.A EF002 # TODO: unbalanced model: 3 equations",
                "M.B ED001 # TODO: unbalanced model: 3 equations",
            ]
        );
        assert_eq!(partial.len(), 1);
        // 原稿の行は一覧の形として読める
        let list = unsupported::KnownList::parse(&candidates.join("\n"));
        assert_eq!(list.entries.len(), 2);
        assert!(list.malformed.is_empty());
    }

    #[test]
    fn one_line_flattens_and_cuts() {
        assert_eq!(one_line("a\n  b\tc", 10), "a b c");
        assert_eq!(one_line("あいうえお", 3), "あいう…");
    }

    fn example(name: &str) -> Example {
        Example {
            name: name.to_string(),
            partial: false,
        }
    }

    /// 方程式 `incidence.len()` 本・未知数 `n_unknowns` 個の構造（状態の参照なし）。
    fn structure(n_unknowns: usize, incidence: Vec<Vec<u32>>) -> DaeStructure {
        let n = incidence.len();
        with_states(n_unknowns, incidence, vec![Vec::new(); n])
    }

    /// `state_refs`（方程式ごとに、参照する状態 x の `der(x)` の未知数の添字）付きの構造。
    /// state_refs に現れる未知数は状態の微分（`der(x…)`、Derivative）、ほかは代数にする。
    fn with_states(
        n_unknowns: usize,
        incidence: Vec<Vec<u32>>,
        state_refs: Vec<Vec<u32>>,
    ) -> DaeStructure {
        let is_state = |i: usize| state_refs.iter().flatten().any(|&u| u as usize == i);
        DaeStructure {
            unknowns: (0..n_unknowns)
                .map(|i| Unknown {
                    name: if is_state(i) {
                        format!("der(x{i})")
                    } else {
                        format!("x{i}")
                    },
                    kind: if is_state(i) {
                        UnknownKind::Derivative
                    } else {
                        UnknownKind::Algebraic
                    },
                    file: None,
                    range: None,
                })
                .collect(),
            equations: (0..incidence.len())
                .map(|i| EquationRow {
                    label: format!("e{i}"),
                    origin: EquationOrigin::Source,
                    file: None,
                    range: None,
                })
                .collect(),
            incidence,
            state_refs,
        }
    }

    fn passed_report(structure: Option<DaeStructure>) -> CheckReport {
        let mut r = CheckReport::new("M.X");
        r.dae = Some(DaeCounts {
            algebraics: 2,
            continuous_equations: 2,
            ..DaeCounts::default()
        });
        r.balance = Some(BalanceCount {
            equations: 2,
            unknowns: 2,
            algebraics: 2,
            ..BalanceCount::default()
        });
        r.structure = structure;
        r
    }

    #[test]
    fn verdict_from_failed_report() {
        let mut r = CheckReport::new("M.X");
        r.diagnostics.push(
            Diagnostic::error(codes::UNSUPPORTED, Stage::S5, "unsupported")
                .note("rumoca: rumoca::todae::ED019")
                .note("Rumoca v0.10.2 が未対応の言語機能"),
        );
        // 後の段階の Error より、前の段階の Error を採る
        r.diagnostics
            .insert(0, Diagnostic::error(codes::UNBALANCED, Stage::S5, "later"));
        r.diagnostics
            .push(Diagnostic::error(codes::RESOLVE, Stage::S1, "first").note("rumoca: ER002"));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ng);
        assert_eq!(row.stage, Some(Stage::S1));
        assert_eq!(row.rumoca_code.as_deref(), Some("ER002"));
        assert_eq!(row.status(), "NG S1 GM-E1001 (ER002)");

        r.diagnostics.pop();
        r.diagnostics.remove(0);
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Unsupported);
        assert_eq!(row.rumoca_code.as_deref(), Some("ED019"));
        assert_eq!(row.status(), "GM-U0001 (ED019)");
    }

    #[test]
    fn s6_runs_only_after_s5_and_is_compared_with_rumoca() {
        let reference = |e, u, m| {
            Some(ReferenceStructure {
                n_equations: e,
                n_unknowns: u,
                matching: m,
            })
        };
        // 正則（Rumoca と一致）
        let r = passed_report(Some(structure(2, vec![vec![0], vec![0, 1]])));
        let row = Row::from_report(&example("M.X"), &r, reference(2, 2, 2), 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        let s6 = row.s6.as_ref().unwrap();
        assert!(s6.regular);
        assert_eq!(s6.agrees_with_rumoca(), Some(true));
        assert_eq!(row.status(), "OK");
        assert_eq!(
            row.counts,
            Some(Counts {
                states: 0,
                algebraics: 2,
                outputs: 0,
                equations: 2,
                unknowns: 2,
            })
        );

        // 特異: 2 本の方程式が同じ未知数だけを指す
        let r = passed_report(Some(structure(2, vec![vec![0], vec![0]])));
        let row = Row::from_report(&example("M.X"), &r, reference(2, 2, 2), 1.0);
        assert_eq!(row.verdict, Verdict::Ng);
        assert_eq!(row.stage, Some(Stage::S6));
        assert_eq!(row.gm_code, Some(codes::SINGULAR));
        assert_eq!(row.s6.as_ref().unwrap().matching, 1);
        assert_eq!(row.s6.as_ref().unwrap().structural_matching, 1);
        assert_eq!(row.s6.as_ref().unwrap().agrees_with_rumoca(), Some(false));
        assert_eq!(row.s6.as_ref().unwrap().der_no_unknown_equations, 0);

        // der の形で未知数を含まず、状態の参照も無い方程式は、まとめた形でも特異
        let r = passed_report(Some(structure(2, vec![vec![0, 1], vec![]])));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.stage, Some(Stage::S6));
        assert_eq!(row.s6.as_ref().unwrap().der_no_unknown_equations, 1);

        // インデックス > 1 の見込み: e1 は状態 x（列 0 = der(x)）だけの拘束。
        // der の形では e1 がマッチしないが、x と der(x) をまとめると完全マッチングがある → OK + GM-I7001
        let r = passed_report(Some(with_states(
            2,
            vec![vec![0, 1], vec![]],
            vec![vec![], vec![0]],
        )));
        let row = Row::from_report(&example("M.X"), &r, reference(2, 2, 1), 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        let s6 = row.s6.as_ref().unwrap();
        assert!(s6.regular && s6.index_reduction_needed);
        assert_eq!((s6.matching, s6.structural_matching), (1, 2));
        assert_eq!(s6.agrees_with_rumoca(), Some(true));
        // S7: e1 を 1 回微分してインデックス 2。状態 x は代数的に決まる（動的な状態 0）
        let s7 = row.s7.as_ref().unwrap();
        assert_eq!(
            (s7.index, s7.n_differentiated, s7.max_differentiations),
            (2, 1, 1)
        );
        assert_eq!(
            (s7.states, s7.dynamic_states, s7.dummy_candidates),
            (1, 0, 1)
        );
        assert!(!s7.failed && s7.needs_state_selection());
        assert_eq!(
            row.status(),
            format!("OK（{}: インデックス 2）", codes::HIGH_INDEX)
        );
        assert_eq!(
            s7_cell(s7),
            "index 2（微分 1 本・最大 1 回・候補 1・動的 0/1）"
        );
        let s = Summary::from_rows(std::slice::from_ref(&row));
        assert_eq!(
            (s.ok, s.s7_info, s.s6_with_der_no_unknown_equations),
            (1, 1, 1)
        );
        // S8 の入力が無いので、S8 は「入力なし」
        assert!(row.s8_reached && row.s8.is_none());
        assert_eq!((s.s8_reached, s.s8_no_input), (1, 1));

        // 正方でない: 判定不能（OK のまま）
        let r = passed_report(Some(structure(3, vec![vec![0], vec![1]])));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        assert!(row.s6_note.as_deref().unwrap().contains("正方でない"));
        assert_eq!(row.status(), "OK（S6 判定不能）");

        // 構造が無い: notes の理由を写す
        let mut r = passed_report(None);
        r.notes.push("射影できなかった".into());
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        assert_eq!(row.s6_note.as_deref(), Some("射影できなかった"));
        assert!(row.s6.is_none());
    }

    fn variable(name: &str, fixed: bool, has_start: bool) -> InitVariable {
        InitVariable {
            name: name.to_string(),
            fixed,
            has_start,
            has_binding: false,
            file: None,
            range: None,
        }
    }

    #[test]
    fn s8_runs_after_s7_and_stops_on_overdetermined_init() {
        let ode = || {
            let mut s = with_states(1, vec![vec![0]], vec![vec![0]]);
            s.equations[0].label = "der(x0) = -x0".into();
            s
        };
        let init = |fixed: bool, initial_equations: usize| InitStructure {
            states: vec![variable("x0", fixed, true)],
            parameters: Vec::new(),
            discretes: Vec::new(),
            algebraics: Vec::new(),
            initial_equations: (0..initial_equations)
                .map(|i| EquationRow {
                    label: format!("x0 = {i}"),
                    origin: EquationOrigin::Source,
                    file: None,
                    range: None,
                })
                .collect(),
            ..Default::default()
        };
        // 合う: 状態 1・fixed 1
        let mut r = passed_report(Some(ode()));
        r.init = Some(init(true, 0));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        assert_eq!(row.s7.as_ref().map(|x| x.index), Some(1));
        let s8 = row.s8.as_ref().unwrap();
        assert_eq!((s8.provided, s8.required), (1, 1));
        assert_eq!(s8.balance(), S8Balance::Balanced);
        assert_eq!(row.status(), "OK");
        assert_eq!(s8_cell(s8), "合う 1/1");

        // 不足: Warning なので OK のまま
        r.init = Some(init(false, 0));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ok);
        assert_eq!(row.s8.as_ref().unwrap().balance(), S8Balance::Under);
        assert_eq!(row.status(), "OK（S8 不足）");

        // 過剰: fixed = true と初期方程式の両方 → GM-E8001 で S8 の NG
        r.init = Some(init(true, 1));
        let row = Row::from_report(&example("M.X"), &r, None, 1.0);
        assert_eq!(row.verdict, Verdict::Ng);
        assert_eq!(row.stage, Some(Stage::S8));
        assert_eq!(row.gm_code, Some(codes::INIT_OVERDETERMINED));
        assert_eq!(row.s8.as_ref().unwrap().balance(), S8Balance::Over);
        assert_eq!(row.status(), "NG S8 GM-E8001");
        let s = Summary::from_rows(std::slice::from_ref(&row));
        assert_eq!((s.ok, s.ng_by_stage.get(&Stage::S8)), (0, Some(&1)));
        assert_eq!((s.s8_reached, s.s8_over), (1, 1));

        // S5 までに Error があれば S7・S8 は回さない
        let mut failed = passed_report(Some(ode()));
        failed.init = Some(init(true, 1));
        failed.diagnostics.push(Diagnostic::error(
            codes::UNBALANCED,
            Stage::S5,
            "unbalanced",
        ));
        let row = Row::from_report(&example("M.X"), &failed, None, 1.0);
        assert!(row.s7.is_none() && row.s8.is_none() && !row.s8_reached);
    }

    #[test]
    fn vm_hwm_is_read_from_proc_status() {
        let status = "Name:\txtask\nVmPeak:\t 9000 kB\nVmHWM:\t 7833012 kB\nVmRSS:\t 100 kB\n";
        assert_eq!(parse_vm_hwm(status), Some(7_833_012));
        assert_eq!(parse_vm_hwm("Name: x\n"), None);
        assert_eq!(gigabytes(7_833_012), "7.5 GiB");
    }

    #[test]
    fn panic_payloads_become_messages() {
        let payload = std::panic::catch_unwind(|| panic!("boom {}", 1)).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "boom 1");
        let payload = std::panic::catch_unwind(|| panic!("static")).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "static");
    }
}

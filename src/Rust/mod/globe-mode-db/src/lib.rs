//! salsa のクエリ DB（遅延評価・差分だけの再計算）。ADR-0005。
//!
//! ```text
//! SourceFile ─┬─▶ outline ──▶ outline_shape ─┬─▶ class_index ──▶ connect_graph
//!   (入力)    │   (位置あり)    (位置なし・Eq) │   (Workspace)      (GraphRequest)
//!             │                 ▲ 防火壁 1    └─▶ defined_classes ──▶ class_files ─┐
//!             │                                                                    ▼
//!             └─▶ shape ─┬─▶ source_shape ──┬─▶ identifiers ───────────────▶ deps_closure
//!                        │   (正規化・Eq)    │                                (CheckRequest)
//!                        │   ▲ 防火壁 0      └──────────────────────────────▶ check ──┬─▶ flat_summary
//!                        └─▶ shape_map                                       (S0〜S5) ├─▶ init_shape ──▶ init_report (S8)
//!                            (診断の位置を元の本文に戻す)                             ├─▶ dae_shape ──┬─▶ structural (S6)
//!                                                                                     │  ▲ 防火壁 2   ├─▶ index_report (S7)
//!                                                                                     │               └─▶ loop_report (S9)
//!                                                                                     └─▶ preflight_shape ──▶ preflight_report (S10)
//! ```
//!
//! - `outline` はファイルの本文が変わるたびに作り直す（エディタの位置の情報に使う）。
//! - `outline_shape` は位置を落とした写し。空白やコメントだけの編集では前と等しくなるので、
//!   salsa がそれ以降（`class_index`・`connect_graph`）の再計算を省く（backdate）。
//! - `source_shape` はコメントを除き空白を畳んだ本文（`globe_mode_core::normalize`）。`check` には
//!   これだけを渡すので、空白やコメントだけの編集では Rumoca のコンパイルが走らない。
//!   位置の対応表（`shape_map`）は別のクエリにして、`check` の依存に入れない。
//! - `check` は salsa の外の [`Checker`]（MSL などのライブラリを一度だけ読んだ Rumoca の Session）を使う。
//! - `check` に渡す文書は、モデルが使うファイルだけ（`deps_closure`）。モデルのファイルから始め、
//!   本文の識別子（`identifiers`）と同じ名前のクラスを定義するファイル（`defined_classes`）を固定点まで足す。
//!   使わないファイルの本文を変えても `check` は再実行されない。
//! - `check` は [`Checker::check_retaining`] を使い、前に渡した文書を Session に残す（文書の和集合。S010）。
//!   Rumoca は文書の組が変わるとライブラリごと解決し直すので、閉包の違うモデルへ切り替えるたびに
//!   組をそろえると約 1.8 s かかる。残した文書の本文が変わったか消えたら、[`GlobeDb::set_file`] と
//!   [`GlobeDb::remove_file`] が検査器に知らせて取り除かせる（salsa の依存は閉包のファイルのままなので、
//!   使わないファイルの編集で `check` が再実行されない性質は変わらない）。
//! - `init_shape`（S8 の入力）と `dae_shape` は位置を落とした写しなので、`init_report`（S8）と
//!   `index_report`（S7）は、位置だけが変わる編集では再計算されない。`init_report` は `init_shape` と
//!   `dae_shape` と `index_report`（動的な状態の数と微分の回数。S009 から初期化系のマッチングにも使う）を読む。
//! - `dae_shape` は位置を落とした DAE の構造。パラメータの値だけを変えた編集などでは前と等しくなり、
//!   `structural`（S6）は再計算されない。
//! - `loop_report`（S9、S010）は `dae_shape` と `index_report` と `init_shape`（反復変数の start 値の有無）を読む。
//!   `preflight_report`（S10、S010）は `preflight_shape`（位置を落としたプリフライトの入力）だけを読む。
//! - どのクエリが実行されたかは [`GlobeDb::take_executed`] で取り出せる（テストと計測に使う）。

mod deps;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// CLI が S10 の experiment の判定を表示するのに使う（S010）
pub use globe_mode_analysis::preflight::ExperimentVerdict;
use globe_mode_analysis::{IndexReport, InitReport, LoopReport, PreflightReport, StructuralReport};
use globe_mode_core::normalize::{self, SourceShape};
use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_frontend::{CheckReport, Checker, Frontend, RumocaFrontend, Source};
use globe_mode_graph::{ConnectGraph, GraphError};
use globe_mode_ir::{
    ClassIndex, ClassKind, DaeStructure, FlatSummary, InitStructure, Outline, PreflightStructure,
};
use salsa::Setter;

/// salsa の外で持つものに手が届く DB（`check` が使う）。
#[salsa::db]
pub trait Db: salsa::Database {
    /// ライブラリを読み込んだ検査器。
    fn checker(&self) -> &Checker;
}

/// 1 つのソースファイル（入力）。
#[salsa::input(debug)]
pub struct SourceFile {
    #[returns(ref)]
    pub path: String,
    #[returns(ref)]
    pub text: String,
}

/// 開いているファイルの集まり（入力）。
#[salsa::input(debug)]
pub struct Workspace {
    #[returns(ref)]
    pub files: Vec<SourceFile>,
}

/// connect グラフの求め（モデル名と深さ）。
#[salsa::interned(debug)]
pub struct GraphRequest<'db> {
    #[returns(ref)]
    pub model: String,
    #[returns(copy)]
    pub depth: u32,
}

/// モデルの検査の求め（モデルの完全修飾名）。
#[salsa::interned(debug)]
pub struct CheckRequest<'db> {
    #[returns(ref)]
    pub model: String,
}

/// ファイルのアウトライン（位置を含む）。構文の誤りは S0 の診断。
#[salsa::tracked(returns(ref))]
pub fn outline(db: &dyn salsa::Database, file: SourceFile) -> Result<Outline, Vec<Diagnostic>> {
    RumocaFrontend::new().outline(Source {
        name: file.path(db),
        text: file.text(db),
    })
}

/// 位置を落としたアウトライン（防火壁 1）。構文の誤りがあれば None。
#[salsa::tracked(returns(ref))]
pub fn outline_shape(db: &dyn salsa::Database, file: SourceFile) -> Option<Outline> {
    outline(db, file).as_ref().ok().map(Outline::without_ranges)
}

/// ワークスペースのクラスの索引。
#[salsa::tracked(returns(ref))]
pub fn class_index(db: &dyn salsa::Database, workspace: Workspace) -> ClassIndex {
    let shapes: Vec<&Outline> = workspace
        .files(db)
        .iter()
        .filter_map(|f| outline_shape(db, *f).as_ref())
        .collect();
    ClassIndex::from_outlines(shapes)
}

/// connect グラフ（flatten しない）。
#[salsa::tracked(returns(ref))]
pub fn connect_graph<'db>(
    db: &'db dyn salsa::Database,
    workspace: Workspace,
    request: GraphRequest<'db>,
) -> Result<ConnectGraph, GraphError> {
    globe_mode_graph::build(
        class_index(db, workspace),
        request.model(db),
        request.depth(db),
    )
}

/// 正規化した本文と位置の対応表（`source_shape` と `shape_map` の元）。
#[salsa::tracked(returns(ref))]
pub fn shape(db: &dyn salsa::Database, file: SourceFile) -> SourceShape {
    normalize::source_shape(file.text(db))
}

/// コメントを除き空白を畳んだ本文（防火壁 0）。空白やコメントだけの編集では前と等しい。
#[salsa::tracked(returns(ref))]
pub fn source_shape(db: &dyn salsa::Database, file: SourceFile) -> String {
    shape(db, file).text.clone()
}

/// 正規化後の位置 → 元の本文の位置の対応表（診断の位置を戻すのに使う。`check` の依存には入れない）。
#[salsa::tracked(returns(ref))]
pub fn shape_map(db: &dyn salsa::Database, file: SourceFile) -> Vec<(u32, u32)> {
    shape(db, file).map.clone()
}

/// 本文（`source_shape`）に現れる識別子（キーワードを含む。文字列の中は除く）。
///
/// `deps_closure` が、参照しているかもしれないクラスの名前として使う。コメントと空白だけの編集では
/// `source_shape` が前と等しいので、再計算されない。
#[salsa::tracked(returns(ref))]
pub fn identifiers(db: &dyn salsa::Database, file: SourceFile) -> BTreeSet<String> {
    deps::identifiers(source_shape(db, file))
}

/// ファイルが定義する**トップレベル**のクラスの単純名（`within A.B; model C` なら `C`）。構文の誤りがあれば空。
///
/// 防火壁 1（`outline_shape`）の下流なので、宣言の変わらない編集では値が変わらない。
///
/// 入れ子のクラスの名前は入れない。ほかのファイルから入れ子のクラス `T.N` に届く道
/// （修飾名 `T.N`、`within …T…;` の字句的なスコープ、`import …T…`、`T` を継承したクラスを経由）には、
/// どれもそのファイルのトップレベルのクラスの名前 `T` か、`T` を名指すファイルが現れるので、
/// `deps_closure` の固定点で `T` のファイルが入る。入れ子の名前まで入れると、`replaceable package Medium`
/// のようなありふれた名前で、使わないファイルまで入ってしまう。
#[salsa::tracked(returns(ref))]
pub fn defined_classes(db: &dyn salsa::Database, file: SourceFile) -> BTreeSet<String> {
    outline_shape(db, file)
        .as_ref()
        .map(|o| o.classes.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default()
}

/// ワークスペースの、クラスの単純名 → 定義するファイル（`deps_closure` の表）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassFiles {
    /// トップレベルのクラスの単純名 → そのクラスを定義するファイル（ワークスペースの順）
    pub by_name: BTreeMap<String, Vec<SourceFile>>,
    /// 構文の誤りでアウトラインを作れないファイル（何を定義しているか分からない）
    pub unparsed: Vec<SourceFile>,
}

/// [`ClassFiles`] を作る。どのモデルの `deps_closure` からも共有される。
#[salsa::tracked(returns(ref))]
pub fn class_files(db: &dyn salsa::Database, workspace: Workspace) -> ClassFiles {
    let mut table = ClassFiles::default();
    for &file in workspace.files(db) {
        if outline_shape(db, file).is_none() {
            table.unparsed.push(file);
            continue;
        }
        for name in defined_classes(db, file) {
            table.by_name.entry(name.clone()).or_default().push(file);
        }
    }
    table
}

/// `check` に渡すファイル（モデルが使うものだけ。パスの昇順）。
///
/// 1. モデルの完全修飾名を定義しているファイルを起点にする。ワークスペースに無いモデル
///    （MSL などライブラリの中のモデル、存在しないモデル）なら起点は無い。
/// 2. 構文の誤りのあるファイルは、いつも入れる（何を定義しているか分からないので。S0 で止まる）。
/// 3. 閉包の中のファイルの `identifiers` にある名前 `t` について、`t` をトップレベルに定義する
///    ワークスペースのファイルをすべて足す。これを固定点まで繰り返す。
///
/// 単純名の一致で拾うので広めに入る（`within` の連鎖、extends、部品の型、import、式の中の関数・定数・
/// redeclare の参照まで）。足りないと名前解決に失敗するので、狭くはしない。
#[salsa::tracked(returns(ref))]
pub fn deps_closure<'db>(
    db: &'db dyn salsa::Database,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Vec<SourceFile> {
    let model = request.model(db);
    let table = class_files(db, workspace);
    let mut queue: Vec<SourceFile> = Vec::new();
    if class_index(db, workspace).get(model).is_some() {
        queue.extend(workspace.files(db).iter().copied().filter(|f| {
            outline_shape(db, *f)
                .as_ref()
                .is_some_and(|o| o.defines(model))
        }));
    }
    queue.extend(table.unparsed.iter().copied());

    let mut closure: HashSet<SourceFile> = queue.iter().copied().collect();
    while let Some(file) = queue.pop() {
        for name in identifiers(db, file) {
            for &dep in table.by_name.get(name).into_iter().flatten() {
                if closure.insert(dep) {
                    queue.push(dep);
                }
            }
        }
    }
    let mut files: Vec<SourceFile> = closure.into_iter().collect();
    files.sort_by(|a, b| a.path(db).cmp(b.path(db)));
    files
}

/// モデルの検査（S0〜S5 と、S6〜S8 の入力）。`deps_closure` のファイルの `source_shape` だけを
/// 文書として Rumoca に渡す。位置は正規化した本文の中のもの。
///
/// 前の検査で渡した文書は Session に残す（[`Checker::check_retaining`]）。結果は閉包の文書だけで
/// 検査したときと同じ（残した文書は、モデルから参照されない名前しか定義しないため）。
#[salsa::tracked(returns(ref))]
pub fn check<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> CheckReport {
    let files = deps_closure(db, workspace, request);
    let texts: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.path(db).as_str(), source_shape(db, *f).as_str()))
        .collect();
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    let mut report = db.checker().check_retaining(&sources, request.model(db));
    if files.is_empty() {
        add_workspace_candidates(&mut report, class_index(db, workspace));
    }
    report
}

/// 文書を 1 つも渡さなかった（モデルがワークスペースに無い）ときの「見つからない」に、
/// ワークスペースのモデルを候補として足す（Rumoca は渡した文書の中からしか候補を出さないため）。
fn add_workspace_candidates(report: &mut CheckReport, index: &ClassIndex) {
    let Some(d) = report
        .diagnostics
        .iter_mut()
        .find(|d| d.code == codes::MODEL_NOT_FOUND)
    else {
        return;
    };
    if d.notes.iter().any(|n| n.starts_with("候補")) {
        return;
    }
    // Rumoca の候補（collect_model_names）と同じく、model / block / class
    let names: Vec<&str> = index
        .names()
        .filter(|n| {
            index.get(n).is_some_and(|c| {
                matches!(
                    c.kind,
                    ClassKind::Model | ClassKind::Block | ClassKind::Class
                )
            })
        })
        .collect();
    if !names.is_empty() {
        d.notes.push(format!("候補: {}", names.join(", ")));
    }
}

/// flat モデルの要約（部品ごとの方程式数など）。
#[salsa::tracked(returns(ref))]
pub fn flat_summary<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<FlatSummary> {
    check(db, workspace, request).flat.clone()
}

/// 位置を落とした DAE の構造（防火壁 2）。
#[salsa::tracked(returns(ref))]
pub fn dae_shape<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<DaeStructure> {
    check(db, workspace, request)
        .structure
        .as_ref()
        .map(DaeStructure::without_ranges)
}

/// S6（構造特異）の解析。DAE の構造が無ければ None。
#[salsa::tracked(returns(ref))]
pub fn structural<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<StructuralReport> {
    dae_shape(db, workspace, request)
        .as_ref()
        .map(globe_mode_analysis::analyze)
}

/// S7（インデックスの見積もりと状態選択の候補）。DAE の構造が無ければ None。
#[salsa::tracked(returns(ref))]
pub fn index_report<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<IndexReport> {
    dae_shape(db, workspace, request)
        .as_ref()
        .map(globe_mode_analysis::analyze_index)
}

/// 位置を落とした初期化系の構造（S8 の入力。防火壁）。取り出せなければ None（理由は `check` の notes）。
#[salsa::tracked(returns(ref))]
pub fn init_shape<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<InitStructure> {
    check(db, workspace, request)
        .init
        .as_ref()
        .map(InitStructure::without_ranges)
}

/// S8（初期化系の検査）。初期化系の構造が無ければ None。
///
/// 要る初期条件の数は「動的な状態の数（S7 の `dynamic_states`）+ fixed = false のパラメータ」。
/// インデックスが 2 以上のモデルでは、状態の数をそのまま使うと誤った不足の警告になるため。
/// S7 の結果が無い（DAE の構造が無い）か打ち切ったときは状態の数で数える。
/// S009 から、初期方程式の参照（`init_shape`）と `dae_shape`・`index_report` があれば初期化系を
/// マッチングして、冗長な初期条件と初期条件の無い未知数も出す（`InitReport::matching`）。
#[salsa::tracked(returns(ref))]
pub fn init_report<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<InitReport> {
    let structure = dae_shape(db, workspace, request).as_ref();
    let index = index_report(db, workspace, request).as_ref();
    init_shape(db, workspace, request)
        .as_ref()
        .map(|init| globe_mode_analysis::analyze_init_system(init, structure, index))
}

/// S9（代数ループ。S010）。DAE の構造と S7 の結果が無ければ None。
///
/// S7 で微分が要らなければ der の形の incidence を、要れば Pantelides 法の最高階の系を BLT に分ける。
/// 反復変数の start 値の有無に初期化系の構造（`init_shape`）を読む（無ければ判定しない）。
/// どれも位置を落とした写しなので、位置だけの編集では再計算されない。
#[salsa::tracked(returns(ref))]
pub fn loop_report<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<LoopReport> {
    let structure = dae_shape(db, workspace, request).as_ref()?;
    let index = index_report(db, workspace, request).as_ref()?;
    let init = init_shape(db, workspace, request).as_ref();
    Some(globe_mode_analysis::analyze_loops(structure, index, init))
}

/// 位置を落としたプリフライトの入力（S10 の入力。防火壁）。取り出せなければ None（理由は `check` の notes）。
#[salsa::tracked(returns(ref))]
pub fn preflight_shape<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<PreflightStructure> {
    check(db, workspace, request)
        .preflight
        .as_ref()
        .map(PreflightStructure::without_ranges)
}

/// S10（プリフライトの静的な部分: パラメータと start 値の評価、experiment annotation。S010）。
#[salsa::tracked(returns(ref))]
pub fn preflight_report<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> Option<PreflightReport> {
    preflight_shape(db, workspace, request)
        .as_ref()
        .map(globe_mode_analysis::analyze_preflight)
}

/// [`GlobeDb::check_model`] の結果: S0〜S5（`check`）、S6（`structural`）、S7（`index_report`）、
/// S8（`init_report`）、S9（`loop_report`）、S10（`preflight_report`）をまとめたもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullReport {
    pub model: String,
    /// `check` クエリの結果そのもの（位置は正規化した本文の中）。
    pub check: CheckReport,
    /// S6 の結果。S0〜S5 を通ったときだけ（段階ごとに止める約束）。
    pub structural: Option<StructuralReport>,
    /// S7 の結果。S6 が構造特異でない（regular）ときだけ。
    pub index: Option<IndexReport>,
    /// S8 の結果。S0〜S7 に Error が無く、初期化系の構造を取り出せたときだけ。
    pub init: Option<InitReport>,
    /// S9 の結果。S0〜S8 に Error が無く、S7 の結果があるときだけ。
    pub loops: Option<LoopReport>,
    /// S10 の結果。S0〜S9 に Error が無く、プリフライトの入力を取り出せたときだけ。
    pub preflight: Option<PreflightReport>,
    /// S0〜S10 の診断。位置は元の本文の中に戻してある。
    pub diagnostics: Vec<Diagnostic>,
}

impl FullReport {
    /// Error が 1 つも無ければ true。
    pub fn passed(&self) -> bool {
        !self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Error が出た最初の段階。
    pub fn failed_stage(&self) -> Option<Stage> {
        self.diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.stage)
            .min()
    }
}

/// GlobeMode の DB。ファイルの出し入れと、クエリの呼び出しをまとめる。
#[salsa::db]
#[derive(Clone)]
pub struct GlobeDb {
    storage: salsa::Storage<Self>,
    executed: Arc<Mutex<Vec<String>>>,
    files: BTreeMap<String, SourceFile>,
    workspace: Option<Workspace>,
    /// salsa の外の検査器（中の Rumoca の Session は共有される）
    checker: Checker,
}

#[salsa::db]
impl salsa::Database for GlobeDb {}

#[salsa::db]
impl Db for GlobeDb {
    fn checker(&self) -> &Checker {
        &self.checker
    }
}

impl Default for GlobeDb {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for GlobeDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobeDb")
            .field("files", &self.files.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl GlobeDb {
    /// ライブラリ無しの DB。
    pub fn new() -> Self {
        // ライブラリを読まない検査器は失敗しない
        Self::with_checker(Checker::new(&[]).expect("ライブラリ無しの検査器は作れる"))
    }

    /// ライブラリ（MSL など）を一度だけ読み込んだ DB。読めなければ S1 の診断（`GM-E1003`）。
    pub fn with_libraries(paths: &[PathBuf]) -> Result<GlobeDb, Vec<Diagnostic>> {
        let paths: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        Ok(Self::with_checker(Checker::new(&paths)?))
    }

    /// 作ってある検査器を使う DB（Session を別の DB や処理と共有するとき）。
    ///
    /// 検査器は持ち主を分けた取っ手（[`Checker::new_handle`]）にして持つ。`check` が Session に残す文書は
    /// この DB のファイルだけになり、同じ Session を共有するほかの DB の文書は残さない。
    pub fn with_checker(checker: Checker) -> Self {
        let checker = checker.new_handle();
        let executed: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = executed.clone();
        Self {
            storage: salsa::Storage::new(Some(Box::new(move |event| {
                if let salsa::EventKind::WillExecute { database_key } = event.kind {
                    // `outline(Id(…))` のような表記から、クエリの名前だけを残す
                    let name = format!("{database_key:?}");
                    let name = name.split('(').next().unwrap_or(&name).to_string();
                    if let Ok(mut v) = log.lock() {
                        v.push(name);
                    }
                }
            }))),
            executed,
            files: BTreeMap::new(),
            workspace: None,
            checker,
        }
    }

    /// ファイルを足すか、本文を差し替える。
    ///
    /// パスは区切りを `/` にそろえてから使う（Windows の `\\` と混ざっても同じファイルになるように）。
    pub fn set_file(&mut self, path: &str, text: impl Into<String>) {
        let text = text.into();
        let path = &normalize_path(path);
        if let Some(file) = self.files.get(path).copied() {
            // `check` が Session に残した文書なら、検査器に本文（渡すのと同じ正規化した形）を知らせる。
            // 形が変わっていれば取り除かれ、次にこのファイルを使うモデルの検査で足し直す
            if self.checker.retains(path) {
                self.checker
                    .document_changed(path, Some(&normalize::source_shape(&text).text));
            }
            file.set_text(self).to(text);
            return;
        }
        let file = SourceFile::new(self, path.to_string(), text);
        self.files.insert(path.to_string(), file);
        let files: Vec<SourceFile> = self.files.values().copied().collect();
        match self.workspace {
            Some(ws) => {
                ws.set_files(self).to(files);
            }
            None => self.workspace = Some(Workspace::new(self, files)),
        }
    }

    /// ファイルをワークスペースから除く。無ければ false。
    ///
    /// `check` が Session に残した文書なら、検査器からも取り除く。
    pub fn remove_file(&mut self, path: &str) -> bool {
        let path = &normalize_path(path);
        if self.files.remove(path).is_none() {
            return false;
        }
        self.checker.document_changed(path, None);
        let files: Vec<SourceFile> = self.files.values().copied().collect();
        if let Some(ws) = self.workspace {
            ws.set_files(self).to(files);
        }
        true
    }

    /// ディレクトリの下の `.mo` をすべて読む（Modelica のパッケージのディレクトリなど）。
    pub fn load_dir(&mut self, dir: &std::path::Path) -> std::io::Result<usize> {
        let mut paths = Vec::new();
        collect_mo(dir, &mut paths)?;
        paths.sort();
        for p in &paths {
            let text = std::fs::read_to_string(p)?;
            self.set_file(&p.display().to_string(), text);
        }
        Ok(paths.len())
    }

    fn workspace(&mut self) -> Workspace {
        match self.workspace {
            Some(ws) => ws,
            None => {
                let ws = Workspace::new(self, Vec::new());
                self.workspace = Some(ws);
                ws
            }
        }
    }

    pub fn outline(&self, path: &str) -> Option<&Result<Outline, Vec<Diagnostic>>> {
        self.files
            .get(&normalize_path(path))
            .map(|f| outline(self, *f))
    }

    pub fn class_index(&mut self) -> &ClassIndex {
        let ws = self.workspace();
        class_index(self, ws)
    }

    pub fn connect_graph(&mut self, model: &str, depth: u32) -> Result<ConnectGraph, GraphError> {
        let ws = self.workspace();
        let request = GraphRequest::new(self, model.to_string(), depth);
        connect_graph(self, ws, request).clone()
    }

    /// モデルの検査（S0〜S5）。位置は正規化した本文の中（元に戻すなら [`GlobeDb::check_model`]）。
    pub fn check(&mut self, model: &str) -> CheckReport {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        check(self, ws, request).clone()
    }

    /// flat モデルの要約。flatten まで通らなければ None。
    pub fn flat_summary(&mut self, model: &str) -> Option<FlatSummary> {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        flat_summary(self, ws, request).clone()
    }

    /// 位置を落とした DAE の構造。DAE まで変換できなければ None。
    pub fn dae_shape(&mut self, model: &str) -> Option<DaeStructure> {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        dae_shape(self, ws, request).clone()
    }

    /// `check` に渡すファイル（モデルが使うもの。パスの昇順）。モデルがワークスペースに無ければ空。
    pub fn deps_closure(&mut self, model: &str) -> Vec<String> {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        deps_closure(self, ws, request)
            .iter()
            .map(|f| f.path(self).clone())
            .collect()
    }

    /// 読み込んだファイルのパス（区切りは `/`、昇順）。
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// S0〜S10 をまとめて検査する。診断の位置は元の本文の中に戻す。
    ///
    /// Error の出た段階で止める約束:
    /// - S6 は S0〜S5 を通ったときだけ
    /// - S7 は S6 が構造特異でない（regular）ときだけ。S7 の Info（インデックス > 1 の見込み、状態選択の候補）では止めない
    /// - S8 は S0〜S7 に Error が無い（`GM-E7001` などが出ていない）ときだけ
    /// - S9 は S0〜S8 に Error が無く、S7 の結果があるときだけ（S9 は Warning と Info だけで止めない）
    /// - S10 は S0〜S9 に Error が無いときだけ
    pub fn check_model(&mut self, model: &str) -> FullReport {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        let report = check(self, ws, request).clone();
        let mut diagnostics: Vec<Diagnostic> = report
            .diagnostics
            .iter()
            .map(|d| self.to_original(d))
            .collect();

        // S6
        let structural = if report.passed() {
            structural(self, ws, request).clone()
        } else {
            None
        };
        if let (Some(s6), Some(structure)) = (&structural, &report.structure) {
            diagnostics.extend(
                s6.diagnostics(structure)
                    .iter()
                    .map(|d| self.to_original(d)),
            );
        }

        // S7: 診断は IndexReport::diagnostics からだけ取る
        let index = match (&structural, &report.structure) {
            (Some(s6), Some(structure)) if s6.is_regular() => {
                let s7 = index_report(self, ws, request).clone();
                if let Some(s7) = &s7 {
                    diagnostics.extend(
                        s7.diagnostics(structure)
                            .iter()
                            .map(|d| self.to_original(d)),
                    );
                }
                s7
            }
            _ => None,
        };

        // S8
        let stopped = !report.passed() || diagnostics.iter().any(Diagnostic::is_error);
        let init = if stopped {
            None
        } else {
            init_report(self, ws, request).clone()
        };
        if let (Some(s8), Some(structure)) = (&init, &report.init) {
            diagnostics.extend(
                s8.diagnostics(structure)
                    .iter()
                    .map(|d| self.to_original(d)),
            );
        }

        // S9
        let stopped = stopped || diagnostics.iter().any(Diagnostic::is_error);
        let loops = match (&index, &report.structure) {
            (Some(_), Some(structure)) if !stopped => {
                let s9 = loop_report(self, ws, request).clone();
                if let Some(s9) = &s9 {
                    diagnostics.extend(
                        s9.diagnostics(structure, report.init.as_ref())
                            .iter()
                            .map(|d| self.to_original(d)),
                    );
                }
                s9
            }
            _ => None,
        };

        // S10
        let stopped = stopped || diagnostics.iter().any(Diagnostic::is_error);
        let preflight = if stopped {
            None
        } else {
            preflight_report(self, ws, request).clone()
        };
        if let (Some(s10), Some(input)) = (&preflight, &report.preflight) {
            diagnostics.extend(s10.diagnostics(input).iter().map(|d| self.to_original(d)));
        }

        FullReport {
            model: model.to_string(),
            check: report,
            structural,
            index,
            init,
            loops,
            preflight,
            diagnostics,
        }
    }

    /// 診断の位置（正規化した本文の中）を、元の本文の中に戻す。ワークスペースに無いファイルはそのまま。
    fn to_original(&self, d: &Diagnostic) -> Diagnostic {
        let mut d = d.clone();
        if let (Some(path), Some(range)) = (&d.file, d.range) {
            d.range = Some(self.original_range(path, range));
        }
        d
    }

    /// 正規化した本文の中の位置（`check` の結果の位置）を、元の本文の中に戻す。
    /// ワークスペースに無いファイル（ライブラリなど）はそのまま。
    pub fn original_range(
        &self,
        path: &str,
        range: globe_mode_core::TextRange,
    ) -> globe_mode_core::TextRange {
        match self.files.get(&normalize_path(path)).copied() {
            Some(file) => {
                let map = shape_map(self, file);
                globe_mode_core::TextRange::new(
                    normalize::to_original(map, range.start),
                    normalize::to_original(map, range.end),
                )
            }
            None => range,
        }
    }

    /// 初期化系の構造（`check` の結果。位置は正規化した本文の中）の位置を、元の本文の中に戻した写し。
    pub fn init_in_original(&self, init: &InitStructure) -> InitStructure {
        let back = |file: &Option<String>, range: Option<globe_mode_core::TextRange>| match (
            file, range,
        ) {
            (Some(f), Some(r)) => Some(self.original_range(f, r)),
            _ => range,
        };
        let mut out = init.clone();
        for v in out
            .states
            .iter_mut()
            .chain(out.parameters.iter_mut())
            .chain(out.discretes.iter_mut())
            .chain(out.algebraics.iter_mut())
        {
            v.range = back(&v.file, v.range);
        }
        for e in &mut out.initial_equations {
            e.range = back(&e.file, e.range);
        }
        out
    }

    /// 検査器（中の Session は、この DB の `check` と共有される）。
    pub fn checker(&self) -> &Checker {
        &self.checker
    }

    /// これまでに実行されたクエリの名前（実行順）を取り出して、記録を空にする。
    pub fn take_executed(&self) -> Vec<String> {
        self.executed
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }
}

/// パスの区切りを `/` にそろえる（DB のファイルのキーと診断の `file` はこの形）。
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn collect_mo(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_mo(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "mo") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_and_slash_paths_are_the_same_file() {
        let mut db = GlobeDb::new();
        db.set_file("lib\\P\\A.mo", "within P;\nmodel A\nend A;\n");
        db.set_file("lib/P/A.mo", "within P;\nmodel B\nend B;\n");
        assert_eq!(db.files.len(), 1);
        assert!(db.class_index().get("P.B").is_some());
        assert!(db.class_index().get("P.A").is_none());
        assert!(db.outline("lib\\P\\A.mo").is_some());
    }
}

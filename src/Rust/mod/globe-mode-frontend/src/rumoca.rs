//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイルだけ。外へは中立 IR と GlobeMode の診断だけを出す。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{
    BalanceCount, Causality, ClassKind, ClassOutline, ComponentOutline, Connect, ConnectPrefix,
    DaeCounts, Experiment, FlatSummary, Outline,
};
use rumoca_compile::compile::{
    Dae, FailedPhase, FlatModel, PhaseResult, SourceRootKind, StrictCompileReport, VariableRole,
};
use rumoca_compile::parsing::{ParseError, ast};
use rumoca_compile::{Session, SessionConfig};

use crate::{CheckReport, Frontend, Source};

/// Phase A のフロントエンド。
#[derive(Debug, Clone, Default)]
pub struct RumocaFrontend {
    _private: (),
}

impl RumocaFrontend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Frontend for RumocaFrontend {
    fn outline(&self, source: Source<'_>) -> Result<Outline, Vec<Diagnostic>> {
        match rumoca_compile::parsing::parse_source_to_ast_with_errors(source.text, source.name) {
            Ok(def) => Ok(Outline {
                within: def.within.as_ref().map(ToString::to_string),
                classes: def.classes.values().map(class_outline).collect(),
            }),
            Err(errors) => Err(errors
                .iter()
                .map(|e| parse_diagnostic(source.name, e))
                .collect()),
        }
    }

    /// 1 回限りの検査。毎回ライブラリを読み直すので、繰り返すなら [`Checker`] を使う。
    fn check(&self, sources: &[Source<'_>], libraries: &[&Path], model: &str) -> CheckReport {
        // S0 はライブラリを読む前に見る（構文の誤りは、ライブラリの有無に関係なく先に出す）
        let mut report = CheckReport::new(model);
        report.diagnostics = syntax_diagnostics(sources);
        if !report.passed() {
            return report;
        }
        match Checker::new(libraries) {
            Ok(checker) => checker.check_parsed(sources, model),
            Err(diagnostics) => {
                report.diagnostics = diagnostics;
                report
            }
        }
    }
}

/// ライブラリ（MSL など）を一度だけ読み、モデルの検査を繰り返す検査器。
///
/// Rumoca の `Session` を `Arc<Mutex<_>>` で持つ（salsa の外の値。ADR-0005）。
/// - ライブラリは [`Checker::new`] で 1 回だけ読む（DurableExternal）。
/// - [`Checker::check`] は、渡された文書を Session に足し（本文が変わったものだけ差し替え）、
///   渡されなかった文書を取り除いてからコンパイルする。文書を変えなければ、Rumoca は解決済みの
///   状態とモデルごとの結果を使い回す。
/// - [`Checker::check_retaining`] は、渡されなかった文書も残す（文書の和集合を持つ。S010）。
///   Rumoca は文書を 1 つでも足すか除くと、ライブラリ（MSL）ごと解決し直す（約 1.8 s）ので、
///   閉包の違うモデルへ切り替えるたびに組をそろえると、そのたびに解決し直すことになる。
///   残した文書の本文が変わったら（消えたら）、持ち主が [`Checker::document_changed`] で知らせる。
/// - 検査の途中で Rumoca が panic して Mutex が毒されたら、次の検査でライブラリを読んだ直後の
///   状態に戻して続ける（`catch_unwind` で包んで使うとき）。
///
/// `Clone` は同じ Session と、同じ文書の持ち主を共有する（持ち主を分けるなら [`Checker::new_handle`]）。
#[derive(Clone)]
pub struct Checker {
    state: Arc<Mutex<CheckerState>>,
    libraries: Arc<[PathBuf]>,
    /// [`Checker::check_retaining`] で残す文書の持ち主
    owner: u64,
}

/// 文書の持ち主の番号を配る（検査器の取っ手ごとに別の番号）。
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

fn next_owner() -> u64 {
    NEXT_OWNER.fetch_add(1, Ordering::Relaxed)
}

struct CheckerState {
    session: Session,
    /// ライブラリだけを読んだ直後の写し（Mutex が毒されたときに戻す先）
    pristine: Session,
    /// Session に足してある文書（名前 → 記録）
    documents: HashMap<String, DocumentRecord>,
    /// ライブラリのトップレベルのクラスの完全修飾名（残した文書と名前がぶつかるかを見る）
    library_classes: HashSet<String>,
}

/// Session に足してある文書 1 つの記録。
struct DocumentRecord {
    text: String,
    /// [`Checker::check_retaining`] で足した持ち主。`check` で足したものは None（次に渡されなければ除く）
    owner: Option<u64>,
    /// トップレベルのクラスの完全修飾名（`within` を付けたもの）
    classes: Vec<String>,
}

impl std::fmt::Debug for Checker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let documents = self
            .state
            .lock()
            .map(|s| s.documents.len())
            .unwrap_or_default();
        f.debug_struct("Checker")
            .field("libraries", &self.libraries)
            .field("documents", &documents)
            .finish()
    }
}

/// DAE まで変換したモデル（S011）。実行系（`globe-mode-exec`）に渡す。
///
/// Rumoca の DAE を `Arc` で持つ（Session の結果と共有するので写さない）。中身に手が届くのは
/// [`CompiledModel::rumoca_dae`] だけで、使ってよいのは `globe-mode-exec` だけ（ADR-0002、ADR-0013）。
#[derive(Clone)]
pub struct CompiledModel {
    /// モデルの完全修飾名
    pub model: String,
    /// モデルの experiment annotation（根のクラスの本文を読み直した生の値。S010 の S10 と同じ）
    pub experiment: Option<Experiment>,
    dae: Arc<Dae>,
}

impl CompiledModel {
    /// Rumoca の DAE。`globe-mode-exec` だけが使う。
    #[doc(hidden)]
    pub fn rumoca_dae(&self) -> &Arc<Dae> {
        &self.dae
    }
}

impl std::fmt::Debug for CompiledModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledModel")
            .field("model", &self.model)
            .field("experiment", &self.experiment)
            .finish_non_exhaustive()
    }
}

/// Rumoca 自身の構造解析（`phase_structural::analyze`）の数。自前の S6 との照合に使う。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceStructure {
    pub n_equations: usize,
    pub n_unknowns: usize,
    /// 最大マッチングの大きさ
    pub matching: usize,
    /// 代数ループ（BLT の大きさ 2 以上のブロック）の大きさ（方程式の数、昇順）。自前の S9 との照合に使う（S010）
    pub loops: Vec<usize>,
}

impl Checker {
    /// ライブラリのディレクトリを読み込む。読めなければ S1 の診断（`GM-E1003`）。
    pub fn new(libraries: &[&Path]) -> Result<Checker, Vec<Diagnostic>> {
        let mut session = Session::new(SessionConfig::default());
        // ライブラリ（MSL など）。めったに変わらないので DurableExternal として読む。
        for lib in libraries {
            let load = session.load_source_root_tolerant(
                &format!("library:{}", lib.display()),
                SourceRootKind::DurableExternal,
                lib,
                None,
            );
            if !load.diagnostics.is_empty() || load.parsed_file_count == 0 {
                let mut d = Diagnostic::error(
                    codes::LIBRARY,
                    Stage::S1,
                    format!("ライブラリ {} を読み込めない", lib.display()),
                );
                for msg in load.diagnostics.iter().take(5) {
                    d = d.note(msg.clone());
                }
                return Err(vec![d]);
            }
        }
        let library_classes = session
            .document_uris()
            .into_iter()
            .filter_map(|uri| session.get_document(uri).and_then(|d| d.parsed()))
            .flat_map(top_level_classes)
            .collect();
        Ok(Checker {
            state: Arc::new(Mutex::new(CheckerState {
                pristine: session.clone(),
                session,
                documents: HashMap::new(),
                library_classes,
            })),
            libraries: libraries.iter().map(|p| p.to_path_buf()).collect(),
            owner: next_owner(),
        })
    }

    /// ライブラリを、ディレクトリの代わりにファイルの組（名前 → 本文）で読み込む（S012。wasm32 には
    /// ファイルシステムが無い）。[`Checker::new`] と同じく DurableExternal の組として Session に入れるので、
    /// ライブラリの文書は検査のたびに渡し直さない。構文の誤りのあるファイルがあれば S1 の診断（`GM-E1003`）。
    ///
    /// `name` は組の名前（診断に出す）。[`Checker::libraries`] は組の名前 `library:<name>` を 1 つだけ返す
    /// （ディレクトリではない。ライブラリがあることを示し、モデルの事前確認をライブラリのあるときと同じにする）。
    pub fn with_library_sources(
        name: &str,
        sources: &[Source<'_>],
    ) -> Result<Checker, Vec<Diagnostic>> {
        use rumoca_compile::compile::ParsedSourceDocument;
        let mut session = Session::new(SessionConfig::default());
        let mut documents = Vec::with_capacity(sources.len());
        let mut errors = Vec::new();
        for source in sources {
            match ParsedSourceDocument::parse(source.name, source.text) {
                Ok(doc) => documents.push(doc),
                Err(e) => errors.push(format!("{}: {e}", source.name)),
            }
        }
        if !errors.is_empty() || documents.is_empty() {
            let mut d = Diagnostic::error(
                codes::LIBRARY,
                Stage::S1,
                format!("ライブラリ {name} を読み込めない"),
            );
            if documents.is_empty() && errors.is_empty() {
                d = d.note("ファイルが 1 つも無い");
            }
            for msg in errors.iter().take(5) {
                d = d.note(msg.clone());
            }
            return Err(vec![d]);
        }
        let set_id = format!("library:{name}");
        session.replace_in_memory_source_set(
            &set_id,
            SourceRootKind::DurableExternal,
            documents,
            None,
        );
        let library_classes = session
            .document_uris()
            .into_iter()
            .filter_map(|uri| session.get_document(uri).and_then(|d| d.parsed()))
            .flat_map(top_level_classes)
            .collect();
        Ok(Checker {
            state: Arc::new(Mutex::new(CheckerState {
                pristine: session.clone(),
                session,
                documents: HashMap::new(),
                library_classes,
            })),
            libraries: Arc::from(vec![PathBuf::from(set_id)]),
            owner: next_owner(),
        })
    }

    /// 同じ Session を共有し、文書の持ち主だけを分けた取っ手。
    ///
    /// [`Checker::check_retaining`] は、自分の取っ手が足した文書だけを残す（ほかの取っ手が足した文書は、
    /// 渡されなければ取り除く）。ワークスペースの違う DB が 1 つの Session を共有しても、互いの文書を
    /// 見ないようにするため。
    pub fn new_handle(&self) -> Checker {
        Checker {
            owner: next_owner(),
            ..self.clone()
        }
    }

    /// 読み込んだライブラリのディレクトリ。
    pub fn libraries(&self) -> &[PathBuf] {
        &self.libraries
    }

    /// `sources` を文書として渡し、`model` を DAE まで変換して検査する（S0〜S5 と、S6 の入力）。
    ///
    /// 前の呼び出しで渡して今回渡さなかった文書は、Session から取り除く。
    pub fn check(&self, sources: &[Source<'_>], model: &str) -> CheckReport {
        // S0: 先にファイルごとに構文を見る。誤りがあればそこで止める（段階ごとに止める約束）。
        let syntax = syntax_diagnostics(sources);
        if !syntax.is_empty() {
            let mut report = CheckReport::new(model);
            report.diagnostics = syntax;
            return report;
        }
        self.check_parsed(sources, model)
    }

    /// [`Checker::check`] と同じ検査。ただし、前の呼び出しで渡して今回渡さなかった文書を残す（和集合）。
    ///
    /// 閉包の違うモデルへ切り替えても文書の組が変わらないので、Rumoca が解決し直さない。
    /// 結果は「`sources` だけを渡した `check`」と同じになるように、次のものは残さない:
    /// - ほかの取っ手（[`Checker::new_handle`]）か `check` が足した文書
    /// - 本文が変わったか消えたと [`Checker::document_changed`] で知らされた文書（知らされた時に除く）
    /// - トップレベルのクラスの完全修飾名が、ほかの文書かライブラリのクラスとぶつかる文書
    ///   （同じクラスを別のファイルへ移したときに、古い定義が残らないように）
    ///
    /// 前提: `sources` は、モデルが使うファイルをすべて含む（globe-mode-db の `deps_closure`）。
    /// 残した文書は、モデルから参照されない名前しか定義しないので、結果を変えない。
    pub fn check_retaining(&self, sources: &[Source<'_>], model: &str) -> CheckReport {
        let syntax = syntax_diagnostics(sources);
        if !syntax.is_empty() {
            let mut report = CheckReport::new(model);
            report.diagnostics = syntax;
            return report;
        }
        self.check_synced(sources, model, Some(self.owner))
    }

    /// `sources` を文書として渡し、`model` を DAE まで変換して返す（S011。実行系に渡す）。
    ///
    /// 文書の扱いは [`Checker::check_retaining`] と同じ（和集合を持つ）。同じ文書の組で検査した直後なら、
    /// Rumoca はモデルごとのキャッシュから返す。失敗したら [`Checker::check`] と同じ診断（S0〜S5）を返す。
    pub fn compile_retaining(
        &self,
        sources: &[Source<'_>],
        model: &str,
    ) -> Result<CompiledModel, Vec<Diagnostic>> {
        let syntax = syntax_diagnostics(sources);
        if !syntax.is_empty() {
            return Err(syntax);
        }
        let mut state = self.lock();
        sync_documents(&mut state, sources, Some(self.owner)).map_err(|d| vec![d])?;
        match state.session.compile_model_strict(model) {
            Ok(compilation) => {
                let result = compilation.result();
                Ok(CompiledModel {
                    model: model.to_string(),
                    experiment: crate::preflight::experiment_for_model(result, model),
                    dae: Arc::clone(&result.dae),
                })
            }
            Err(strict) => {
                let (d, rumoca_code) = strict_failure_diagnostic(&strict);
                let d = match rumoca_code
                    .as_deref()
                    .and_then(|c| crate::unsupported::classify(model, c))
                {
                    Some(u) => {
                        let mut d = d.note(u.reason);
                        d.code = u.code;
                        d
                    }
                    None => d,
                };
                Err(vec![d])
            }
        }
    }

    /// この取っ手が [`Checker::check_retaining`] で足し、いま Session に残してある文書なら true。
    pub fn retains(&self, name: &str) -> bool {
        self.lock()
            .documents
            .get(name)
            .is_some_and(|d| d.owner == Some(self.owner))
    }

    /// 残してある文書の本文が変わった（`Some`、渡すのと同じ形の本文）か、消えた（`None`）ことを知らせる。
    ///
    /// この取っ手の文書で本文が違えば、すぐに Session から取り除く（次に渡されたときに足し直す）。
    /// 取り除いても、解決し直すのは次のコンパイルのとき（Rumoca の解決は遅延）。
    pub fn document_changed(&self, name: &str, text: Option<&str>) {
        let mut state = self.lock();
        let stale = state
            .documents
            .get(name)
            .is_some_and(|d| d.owner == Some(self.owner) && text != Some(d.text.as_str()));
        if stale {
            state.session.remove_document(name);
            state.documents.remove(name);
        }
    }

    /// Session に足してある文書の名前（昇順。ライブラリの文書は含まない）。
    pub fn document_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.lock().documents.keys().cloned().collect();
        names.sort();
        names
    }

    /// Rumoca 自身の構造解析の数（照合用）。DAE まで変換できなければ None。
    pub fn reference_structure(
        &self,
        sources: &[Source<'_>],
        model: &str,
    ) -> Option<ReferenceStructure> {
        let mut state = self.lock();
        sync_documents(&mut state, sources, None).ok()?;
        let compilation = state.session.compile_model_strict(model).ok()?;
        let r = compilation
            .result()
            .dae
            .inspect(rumoca_compile::phase_structural::analyze);
        // v0.10.2 の `analyze` は `algebraic_loops` を埋めず、ループを ES002 の警告（"algebraic loop detected:
        // {n} equations must be solved simultaneously"）として diagnostics に積むだけなので、文言から n を読む
        let mut loops: Vec<usize> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("ES002"))
            .filter_map(|d| {
                d.message
                    .strip_prefix("algebraic loop detected: ")?
                    .split(' ')
                    .next()?
                    .parse()
                    .ok()
            })
            .collect();
        loops.sort_unstable();
        Some(ReferenceStructure {
            n_equations: r.n_equations,
            n_unknowns: r.n_unknowns,
            matching: r.matching_size,
            loops,
        })
    }

    /// Session を、ライブラリを読んだ直後の状態に戻す（文書と、モデルごとのキャッシュを捨てる）。
    ///
    /// 同じ Session で多くのモデルを続けて検査すると、Rumoca のキャッシュで大きくなる
    /// （MSL の例題 566 本で 7.8 GB）。ときどき呼んで小さく保つ。ライブラリは読み直さない。
    pub fn reset(&self) {
        let mut state = self.lock();
        state.session = state.pristine.clone();
        state.documents.clear();
    }

    fn lock(&self) -> MutexGuard<'_, CheckerState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // 前の検査が panic した。Session の中身は信用できないので、ライブラリを読んだ直後に戻す
                let mut guard = poisoned.into_inner();
                guard.session = guard.pristine.clone();
                guard.documents.clear();
                self.state.clear_poison();
                guard
            }
        }
    }

    /// S0 を通った後の検査。
    fn check_parsed(&self, sources: &[Source<'_>], model: &str) -> CheckReport {
        self.check_synced(sources, model, None)
    }

    /// 文書をそろえてから検査する。`owner` が Some なら、その持ち主の文書を残す（[`Checker::check_retaining`]）。
    fn check_synced(&self, sources: &[Source<'_>], model: &str, owner: Option<u64>) -> CheckReport {
        let mut report = CheckReport::new(model);
        let mut state = self.lock();
        if let Err(d) = sync_documents(&mut state, sources, owner) {
            report.diagnostics.push(d);
            return report;
        }
        let session = &mut state.session;

        // 候補を出すための事前確認。ユーザーのファイルだけのときに限る。
        // 名前は渡した文書の構文木から集める（S010。Session の model_names() は残した文書のモデルも
        // 数え、解決に失敗すると Err を返すので使わない）
        let names = if self.libraries.is_empty() {
            passed_model_names(session, sources)
        } else {
            Vec::new()
        };
        if self.libraries.is_empty() && !names.iter().any(|n| n == model) {
            let mut d = Diagnostic::error(
                codes::MODEL_NOT_FOUND,
                Stage::S1,
                format!("モデル `{model}` が見つからない"),
            );
            if !names.is_empty() {
                d = d.note(format!("候補: {}", names.join(", ")));
            }
            report.diagnostics.push(d);
            return report;
        }

        // Rumoca の CLI と同じ strict-reachable のコンパイル。MSL を解決したときの警告では失敗しない。
        match session.compile_model_strict(model) {
            Ok(compilation) => {
                let result = compilation.result();
                let b = &result.balance_detail;
                let balance = BalanceCount {
                    equations: b.equations(),
                    unknowns: b.unknowns(),
                    states: b.state_unknowns,
                    algebraics: b.algebraic_unknowns,
                    outputs: b.output_unknowns,
                    discrete: b.discrete_real_unknowns + b.discrete_value_unknowns,
                };
                let flat = flat_summary(&result.flat);
                if !balance.is_balanced() {
                    let d = Diagnostic::error(
                        codes::UNBALANCED,
                        Stage::S5,
                        format!(
                            "方程式が {} 個、変数が {} 個で、数が合わない",
                            balance.equations, balance.unknowns
                        ),
                    );
                    report.diagnostics.push(with_component_notes(d, &flat));
                }
                report.balance = Some(balance);
                report.dae = Some(dae_counts(&result.dae));
                report.flat = Some(flat);
                match crate::structure::extract(&result.dae) {
                    Ok(structure) => report.structure = Some(structure),
                    Err(reason) => report.notes.push(format!(
                        "S6 の入力（方程式 × 未知数）を作れなかった: {reason}"
                    )),
                }
                match crate::init::extract(&result.dae) {
                    Ok(init) => report.init = Some(init),
                    Err(reason) => report
                        .notes
                        .push(format!("S8 の入力（初期化系）を作れなかった: {reason}")),
                }
                match crate::preflight::extract_for_model(result, model) {
                    Ok(preflight) => report.preflight = Some(preflight),
                    Err(reason) => report.notes.push(format!(
                        "S10 の入力（プリフライト）を作れなかった: {reason}"
                    )),
                }
            }
            Err(strict) => {
                let (d, rumoca_code) = strict_failure_diagnostic(&strict);
                let mut d = match rumoca_code
                    .as_deref()
                    .and_then(|c| crate::unsupported::classify(model, c))
                {
                    // コンパイラが未対応（GM-U）。重さと段階はそのまま（実行は Error と同じく拒む）
                    Some(u) => {
                        let mut d = d.note(u.reason);
                        d.code = u.code;
                        d
                    }
                    None => d,
                };
                // DAE への変換（S5）で止まったときも、flat モデルだけは取っておく。
                // 数が合わない（ED001）ときは、部品ごとの方程式数を補足に出す
                if d.stage == Stage::S5
                    && let Ok(flat) =
                        session.compile_model_flat_strict_reachable_uncached_with_recovery(model)
                {
                    let flat = flat_summary(&flat);
                    if rumoca_code.as_deref() == Some("ED001") {
                        d = with_component_notes(d, &flat);
                    }
                    report.flat = Some(flat);
                }
                report.diagnostics.push(d);
            }
        }
        report
    }
}

/// S0: ファイルごとの構文の誤り。
fn syntax_diagnostics(sources: &[Source<'_>]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for s in sources {
        if let Err(errors) =
            rumoca_compile::parsing::parse_source_to_ast_with_errors(s.text, s.name)
        {
            out.extend(errors.iter().map(|e| parse_diagnostic(s.name, e)));
        }
    }
    out
}

/// Session の文書を `sources` にそろえる。本文が変わっていない文書は触らない
/// （差し替えると Rumoca が解決済みの状態を捨てるため）。
///
/// `owner` が None なら、渡されなかった文書をすべて取り除く（組を `sources` にそろえる）。
/// Some なら、その持ち主が足した文書は残す。ただし、トップレベルのクラスの完全修飾名がほかの文書か
/// ライブラリとぶつかるものは取り除く（[`Checker::check_retaining`]）。
fn sync_documents(
    state: &mut CheckerState,
    sources: &[Source<'_>],
    owner: Option<u64>,
) -> Result<(), Diagnostic> {
    let wanted: HashSet<&str> = sources.iter().map(|s| s.name).collect();
    let stale: Vec<String> = state
        .documents
        .iter()
        .filter(|(name, d)| {
            !wanted.contains(name.as_str()) && (owner.is_none() || d.owner != owner)
        })
        .map(|(name, _)| name.clone())
        .collect();
    for name in stale {
        state.session.remove_document(&name);
        state.documents.remove(&name);
    }
    for s in sources {
        if let Some(d) = state.documents.get_mut(s.name)
            && d.text == s.text
        {
            d.owner = owner;
            continue;
        }
        if let Err(e) = state.session.add_document(s.name, s.text) {
            // 古い本文を残さない（Session と記録を一致させる）
            state.session.remove_document(s.name);
            state.documents.remove(s.name);
            return Err(
                Diagnostic::error(codes::RESOLVE, Stage::S1, e.to_string()).at(s.name, None)
            );
        }
        let classes = state
            .session
            .get_document(s.name)
            .and_then(|d| d.parsed())
            .map(top_level_classes)
            .unwrap_or_default();
        state.documents.insert(
            s.name.to_string(),
            DocumentRecord {
                text: s.text.to_string(),
                owner,
                classes,
            },
        );
    }
    if owner.is_some() {
        remove_colliding_documents(state, &wanted);
    }
    Ok(())
}

/// 残した文書（渡されなかったもの）のうち、トップレベルのクラスの完全修飾名がほかの文書か
/// ライブラリのクラスとぶつかるものを取り除く。
///
/// 同じクラスを別のファイルへ移した（改名した）ときに古い定義が残ると、渡した文書だけのときと
/// 違う結果になる（Rumoca が 2 つの定義をまとめるか、まとめられずに解決が止まる）ため。
fn remove_colliding_documents(state: &mut CheckerState, wanted: &HashSet<&str>) {
    let mut defined: HashMap<&str, usize> = HashMap::new();
    for d in state.documents.values() {
        for c in &d.classes {
            *defined.entry(c.as_str()).or_default() += 1;
        }
    }
    let colliding: Vec<String> = state
        .documents
        .iter()
        .filter(|(name, d)| {
            !wanted.contains(name.as_str())
                && d.classes.iter().any(|c| {
                    defined.get(c.as_str()).is_some_and(|&n| n > 1)
                        || state.library_classes.contains(c)
                })
        })
        .map(|(name, _)| name.clone())
        .collect();
    for name in colliding {
        state.session.remove_document(&name);
        state.documents.remove(&name);
    }
}

/// 文書のトップレベルのクラスの完全修飾名（`within A.B; model C` なら `A.B.C`）。
fn top_level_classes(def: &ast::StoredDefinition) -> Vec<String> {
    def.classes
        .keys()
        .map(|c| rumoca_compile::parsing::qualify_stored_definition_class_name(def, c))
        .collect()
}

/// 渡した文書のモデル・ブロック・クラスの完全修飾名（渡した順、文書の中では宣言の順）。
///
/// Rumoca の `collect_model_names`（`Session::model_names` が解決した木から集めるのと同じ規則）を、
/// Session の文書ごとの構文木に使う。解決しないので、ほかの文書の名前の誤りに左右されず、
/// [`Checker::check_retaining`] で残した文書のモデルも入らない。
fn passed_model_names(session: &Session, sources: &[Source<'_>]) -> Vec<String> {
    sources
        .iter()
        .filter_map(|s| session.get_document(s.name).and_then(|d| d.parsed()))
        .flat_map(rumoca_compile::parsing::collect_model_names)
        .collect()
}

/// strict のコンパイルの失敗を診断にする。2 つ目は Rumoca のエラーコード（GM-U の振り分けに使う）。
fn strict_failure_diagnostic(strict: &StrictCompileReport) -> (Diagnostic, Option<String>) {
    let (d, code) = match &strict.requested_result {
        Some(PhaseResult::NeedsInner { missing_inners, .. }) => (
            Diagnostic::error(
                codes::MISSING_INNER,
                Stage::S3,
                format!(
                    "outer に対応する inner が無い: {}",
                    missing_inners.join(", ")
                ),
            ),
            None,
        ),
        Some(PhaseResult::Failed {
            phase,
            error,
            error_code,
            ..
        }) => (
            phase_diagnostic(Some(*phase), error_code.as_deref(), error.clone()),
            error_code.clone(),
        ),
        // 要求したモデル自体ではなく、依存先のクラスで失敗したとき
        _ => match strict.failures.first() {
            Some(f) => (
                phase_diagnostic(f.phase, f.error_code.as_deref(), f.error.clone())
                    .note(format!("失敗したクラス: {}", f.model_name)),
                f.error_code.clone(),
            ),
            None => (
                Diagnostic::error(codes::RESOLVE, Stage::S1, strict.failure_summary(5)),
                None,
            ),
        },
    };
    let more = strict.failures.len().saturating_sub(1);
    let mut d = if more > 0 {
        d.note(format!("ほかに {more} 件の失敗がある"))
    } else {
        d
    };
    // 位置: 要求したモデルの失敗の記録（無ければ最初の記録）の主なラベル
    if d.file.is_none()
        && let Some(map) = &strict.source_map
        && let Some(f) = strict
            .failures
            .iter()
            .find(|f| f.model_name == strict.requested_model)
            .or_else(|| strict.failures.first())
        && let Some(label) = &f.primary_label
        && let (Some(file), range) = crate::structure::span_location(map, label.span)
    {
        d = d.at(file, range);
    }
    (d, code)
}

/// flat モデルの要約。部品ごとの方程式数は、Rumoca が方程式の持ち主を決める規則
/// （`rumoca-phase-flatten` の `equation_origin_scope`）に合わせる。
fn flat_summary(flat: &FlatModel) -> FlatSummary {
    let mut by_component: BTreeMap<String, usize> = BTreeMap::new();
    for eq in &flat.equations {
        let scope = origin_scope(
            eq.origin.component_name(),
            eq.origin.binding_variable(),
            &eq.origin.to_string(),
        );
        *by_component.entry(scope).or_default() += eq.scalar_count;
    }
    FlatSummary {
        variables: flat.variables.len(),
        equations: flat.equations.iter().map(|e| e.scalar_count).sum(),
        initial_equations: flat.initial_equations.iter().map(|e| e.scalar_count).sum(),
        partial: flat.is_partial,
        top_level_connectors: flat.top_level_connectors.iter().cloned().collect(),
        equations_by_component: by_component.into_iter().collect(),
    }
}

/// 方程式の持ち主の部品（トップレベルは空文字列）。
///
/// rumoca の `EquationOrigin` の型は公開されていないので（型の名前を書けない）、
/// アクセサ（`component_name` / `binding_variable`）と `Display` の表記で見分ける。
fn origin_scope(component: Option<&str>, binding: Option<&str>, shown: &str) -> String {
    if let Some(component) = component {
        return component.to_string();
    }
    if let Some(variable) = binding {
        return parent_scope(variable);
    }
    if let Some(component) = shown.strip_prefix("algorithm from ") {
        return component.to_string();
    }
    for prefix in ["reinit equation for ", "when assignment for "] {
        if let Some(variable) = shown.strip_prefix(prefix) {
            return parent_scope(variable);
        }
    }
    if let Some(rest) = shown.strip_prefix("unconnected flow: ") {
        return parent_scope(rest.trim_end_matches(" = 0"));
    }
    // 接続の等式と flow の総和は、トップレベルに数える（Rumoca と同じ）
    String::new()
}

fn parent_scope(name: &str) -> String {
    rumoca_core::ComponentPath::from_flat_path(name)
        .parent()
        .unwrap_or_else(rumoca_core::ComponentPath::root)
        .to_flat_string()
}

/// S5 の診断に、方程式の多い部品を上から 5 つ補足として付ける。
fn with_component_notes(mut d: Diagnostic, flat: &FlatSummary) -> Diagnostic {
    let mut ranked: Vec<&(String, usize)> = flat.equations_by_component.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if ranked.is_empty() {
        return d;
    }
    let shown: Vec<String> = ranked
        .iter()
        .take(5)
        .map(|(c, n)| {
            let c = if c.is_empty() {
                "（トップレベル）"
            } else {
                c
            };
            format!("{c} {n}")
        })
        .collect();
    d = d.note(format!(
        "flat の方程式 {} 個（部品ごと、多い順）: {}",
        flat.equations,
        shown.join("、")
    ));
    d
}

/// DAE の変数を役割ごとに数える（Rumoca の CLI の `dae_counts` と同じ数え方）。
fn dae_counts(dae: &Dae) -> DaeCounts {
    dae.inspect(|view| {
        let mut c = DaeCounts {
            continuous_equations: view.continuous_equation_count(),
            ..DaeCounts::default()
        };
        for (_, v) in view.variables() {
            match v.role() {
                VariableRole::State => c.states += 1,
                VariableRole::Algebraic => c.algebraics += 1,
                VariableRole::Output => c.outputs += 1,
                VariableRole::Input => c.inputs += 1,
                VariableRole::Parameter => c.parameters += 1,
                VariableRole::Constant
                | VariableRole::DiscreteReal
                | VariableRole::DiscreteValue => {}
            }
        }
        c
    })
}

/// Rumoca の失敗段階とエラーコードを、GlobeMode の検査段階と GM コードに対応づける（ADR-0002）。
fn phase_diagnostic(
    phase: Option<FailedPhase>,
    error_code: Option<&str>,
    error: String,
) -> Diagnostic {
    let (code, stage) = match (phase, error_code) {
        // rumoca の ED001 は「方程式数と変数数が合わない」
        (Some(FailedPhase::ToDae), Some("ED001")) => (codes::UNBALANCED, Stage::S5),
        (Some(FailedPhase::Instantiate), _) => (codes::INSTANTIATE, Stage::S3),
        (Some(FailedPhase::Typecheck), _) => (codes::TYPECHECK, Stage::S2),
        (Some(FailedPhase::Flatten), _) => (codes::FLATTEN, Stage::S4),
        (Some(FailedPhase::ToDae), _) => (codes::TO_DAE, Stage::S5),
        (None, _) => (codes::RESOLVE, Stage::S1),
    };
    let d = Diagnostic::error(code, stage, error);
    match error_code {
        Some(c) => d.note(format!("rumoca: {c}")),
        None => d,
    }
}

fn parse_diagnostic(file: &str, e: &ParseError) -> Diagnostic {
    let span = e.span();
    let range = (!span.is_dummy()).then(|| TextRange::new(span.start.0 as u32, span.end.0 as u32));
    let message = match e {
        ParseError::SyntaxError { message, .. } => message.clone(),
        ParseError::NoAstProduced { .. } => "構文木を作れなかった".to_string(),
        ParseError::IoError { path, message, .. } => format!("{path}: {message}"),
    };
    Diagnostic::error(codes::SYNTAX, Stage::S0, message).at(file, range)
}

fn class_outline(c: &ast::ClassDef) -> ClassOutline {
    /// `import` 文 1 つを、IR の書式（[`ClassOutline::imports`]）の名前に直す。
    /// 先頭の `.`（`import .A.B;` のグローバルスコープ）は落とす（どちらも完全修飾名なので）。
    fn import_paths(i: &ast::Import) -> Vec<String> {
        match i {
            ast::Import::Qualified { path, .. } | ast::Import::Renamed { path, .. } => {
                vec![path.to_string()]
            }
            ast::Import::Unqualified { path, .. } => vec![format!("{path}.*")],
            ast::Import::Selective { path, names, .. } => {
                names.iter().map(|n| format!("{path}.{}", n.text)).collect()
            }
        }
    }

    let connects = c
        .equations
        .iter()
        .filter_map(|eq| match eq {
            ast::Equation::Connect { lhs, rhs } => Some(Connect {
                lhs: lhs.to_string(),
                rhs: rhs.to_string(),
                range: TextRange::new(lhs.span.start.0 as u32, rhs.span.end.0 as u32),
            }),
            _ => None,
        })
        .collect();

    ClassOutline {
        name: c.name.text.to_string(),
        kind: class_kind(c.class_type.as_str()),
        partial: c.partial,
        causality: causality(&c.causality),
        range: TextRange::new(c.location.start, c.location.end),
        extends: c.extends.iter().map(|e| e.base_name.to_string()).collect(),
        imports: c.imports.iter().flat_map(import_paths).collect(),
        components: c
            .components
            .values()
            .map(|comp| ComponentOutline {
                name: comp.name.clone(),
                type_name: comp.type_name.to_string(),
                prefix: match comp.connection {
                    ast::Connection::Flow(_) => ConnectPrefix::Flow,
                    ast::Connection::Stream(_) => ConnectPrefix::Stream,
                    ast::Connection::Empty => ConnectPrefix::Potential,
                },
                causality: causality(&comp.causality),
                range: TextRange::new(comp.name_token.location.start, comp.name_token.location.end),
            })
            .collect(),
        connects,
        classes: c.classes.values().map(class_outline).collect(),
    }
}

/// rumoca の Causality は公開されていないので、Debug の表記で見分ける（`Input(..)` / `Output(..)` / `Empty`）。
fn causality(c: &impl std::fmt::Debug) -> Causality {
    let s = format!("{c:?}");
    if s.starts_with("Input") {
        Causality::Input
    } else if s.starts_with("Output") {
        Causality::Output
    } else {
        Causality::None
    }
}

fn class_kind(keyword: &str) -> ClassKind {
    // rumoca の ClassType は公開されていないので、`as_str()` のキーワードで対応づける。
    match keyword {
        "class" => ClassKind::Class,
        "block" => ClassKind::Block,
        "connector" => ClassKind::Connector,
        "record" => ClassKind::Record,
        "type" => ClassKind::Type,
        "package" => ClassKind::Package,
        "function" => ClassKind::Function,
        "operator" => ClassKind::Operator,
        _ => ClassKind::Model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poisoned_checker_recovers_the_pristine_session() {
        let checker = Checker::new(&[]).unwrap();
        let model = "model A\n  Real x;\nequation\n  x = 1;\nend A;\n";
        let sources = [Source {
            name: "A.mo",
            text: model,
        }];
        assert!(checker.check(&sources, "A").passed());
        // 検査の途中（Mutex を持ったまま）で panic したことにする
        let c = checker.clone();
        let caught = std::panic::catch_unwind(move || {
            let _guard = c.state.lock().unwrap();
            panic!("Rumoca の panic の代わり");
        });
        assert!(caught.is_err());
        assert!(checker.state.is_poisoned());
        // 次の検査で、ライブラリを読んだ直後の Session に戻してから続ける
        let r = checker.check(&sources, "A");
        assert!(r.passed(), "{:?}", r.diagnostics);
        assert!(!checker.state.is_poisoned());
        assert_eq!(checker.lock().documents.len(), 1);
    }

    #[test]
    fn checks_still_work_after_reset() {
        let checker = Checker::new(&[]).unwrap();
        let sources = [Source {
            name: "A.mo",
            text: "model A\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend A;\n",
        }];
        let before = checker.check(&sources, "A");
        assert!(before.passed(), "{:?}", before.diagnostics);
        checker.reset();
        assert!(checker.lock().documents.is_empty());
        // 文書は次の検査で足し直す。結果は前と同じ
        let after = checker.check(&sources, "A");
        assert_eq!(after, before);
        assert_eq!(checker.lock().documents.len(), 1);
        // 足していない文書のモデルは、reset の後には見つからない
        checker.reset();
        let r = checker.check(&[], "A");
        assert_eq!(r.diagnostics[0].code, codes::MODEL_NOT_FOUND);
    }

    #[test]
    fn check_retaining_keeps_documents_until_check_or_a_change() {
        let checker = Checker::new(&[]).unwrap();
        let a = Source {
            name: "A.mo",
            text: "model A\n  Real x;\nequation\n  x = 1;\nend A;\n",
        };
        let b = Source {
            name: "B.mo",
            text: "model B\n  Real y;\nequation\n  y = 2;\nend B;\n",
        };
        assert!(checker.check_retaining(&[a], "A").passed());
        assert!(checker.check_retaining(&[b], "B").passed());
        // A.mo は渡していないが残る。候補には渡した文書のモデルだけを出す
        assert_eq!(checker.document_names(), ["A.mo", "B.mo"]);
        let r = checker.check_retaining(&[b], "A");
        assert_eq!(r.diagnostics[0].code, codes::MODEL_NOT_FOUND);
        assert_eq!(r.diagnostics[0].notes, ["候補: B"]);
        // 別の取っ手は、この取っ手の文書を残さない
        let other = checker.new_handle();
        assert!(!other.retains("B.mo"));
        assert!(other.check_retaining(&[a], "A").passed());
        assert_eq!(checker.document_names(), ["A.mo"]);
        // 本文が同じなら残し、違えば（消えれば）すぐに取り除く
        other.document_changed("A.mo", Some(a.text));
        assert!(other.retains("A.mo"));
        other.document_changed("A.mo", Some("model A\nend A;\n"));
        assert!(checker.document_names().is_empty());
        // check は組を渡した文書にそろえる（残さない）
        assert!(other.check_retaining(&[a], "A").passed());
        assert!(checker.check(&[b], "B").passed());
        assert_eq!(checker.document_names(), ["B.mo"]);
        assert!(!checker.retains("B.mo"));
    }

    #[test]
    fn origin_scope_follows_rumoca_rules() {
        assert_eq!(origin_scope(Some("r"), None, ""), "r");
        assert_eq!(origin_scope(None, Some("r.p.v"), ""), "r.p");
        assert_eq!(origin_scope(None, None, "algorithm from a.b"), "a.b");
        assert_eq!(
            origin_scope(None, None, "unconnected flow: c.n.i = 0"),
            "c.n"
        );
        assert_eq!(
            origin_scope(None, None, "connection equation: a.v = b.v"),
            ""
        );
        assert_eq!(
            origin_scope(None, None, "flow sum equation: a.i + b.i = 0"),
            ""
        );
    }
}

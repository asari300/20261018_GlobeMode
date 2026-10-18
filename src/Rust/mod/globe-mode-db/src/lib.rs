//! salsa のクエリ DB（遅延評価・差分だけの再計算）。ADR-0005。
//!
//! ```text
//! SourceFile ─┬─▶ outline ──▶ outline_shape ──▶ class_index ──▶ connect_graph
//!   (入力)    │   (位置あり)    (位置なし・Eq)     (Workspace)      (GraphRequest)
//!             │                 ▲ 防火壁 1
//!             └─▶ shape ─┬─▶ source_shape ─────▶ check ──┬─▶ flat_summary
//!                        │   (正規化した本文・Eq)  (CheckRequest)
//!                        │   ▲ 防火壁 0                   └─▶ dae_shape ──▶ structural
//!                        └─▶ shape_map                        (位置なし・Eq)   (S6)
//!                            (診断の位置を元の本文に戻す)       ▲ 防火壁 2
//! ```
//!
//! - `outline` はファイルの本文が変わるたびに作り直す（エディタの位置の情報に使う）。
//! - `outline_shape` は位置を落とした写し。空白やコメントだけの編集では前と等しくなるので、
//!   salsa がそれ以降（`class_index`・`connect_graph`）の再計算を省く（backdate）。
//! - `source_shape` はコメントを除き空白を畳んだ本文（`globe_mode_core::normalize`）。`check` には
//!   これだけを渡すので、空白やコメントだけの編集では Rumoca のコンパイルが走らない。
//!   位置の対応表（`shape_map`）は別のクエリにして、`check` の依存に入れない。
//! - `check` は salsa の外の [`Checker`]（MSL などのライブラリを一度だけ読んだ Rumoca の Session）を使う。
//! - `dae_shape` は位置を落とした DAE の構造。パラメータの値だけを変えた編集などでは前と等しくなり、
//!   `structural`（S6）は再計算されない。
//! - どのクエリが実行されたかは [`GlobeDb::take_executed`] で取り出せる（テストと計測に使う）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use globe_mode_analysis::StructuralReport;
use globe_mode_core::normalize::{self, SourceShape};
use globe_mode_core::{Diagnostic, Stage};
use globe_mode_frontend::{CheckReport, Checker, Frontend, RumocaFrontend, Source};
use globe_mode_graph::{ConnectGraph, GraphError};
use globe_mode_ir::{ClassIndex, DaeStructure, FlatSummary, Outline};
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

/// モデルの検査（S0〜S5 と、S6 の入力の DAE の構造）。ワークスペースの全ファイルの
/// `source_shape` を文書として Rumoca に渡す。位置は正規化した本文の中のもの。
#[salsa::tracked(returns(ref))]
pub fn check<'db>(
    db: &'db dyn Db,
    workspace: Workspace,
    request: CheckRequest<'db>,
) -> CheckReport {
    let files = workspace.files(db);
    let texts: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.path(db).as_str(), source_shape(db, *f).as_str()))
        .collect();
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    db.checker().check(&sources, request.model(db))
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

/// [`GlobeDb::check_model`] の結果: S0〜S5（`check`）と S6（`structural`）をまとめたもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullReport {
    pub model: String,
    /// `check` クエリの結果そのもの（位置は正規化した本文の中）。
    pub check: CheckReport,
    /// S6 の結果。S0〜S5 を通ったときだけ（段階ごとに止める約束）。
    pub structural: Option<StructuralReport>,
    /// S0〜S6 の診断。位置は元の本文の中に戻してある。
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
    pub fn with_checker(checker: Checker) -> Self {
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

    /// S0〜S6 をまとめて検査する。診断の位置は元の本文の中に戻す。
    ///
    /// S6 は S0〜S5 を通ったときだけ行う（Error の出た段階で止める約束）。
    pub fn check_model(&mut self, model: &str) -> FullReport {
        let ws = self.workspace();
        let request = CheckRequest::new(self, model.to_string());
        let report = check(self, ws, request).clone();
        let mut diagnostics: Vec<Diagnostic> = report
            .diagnostics
            .iter()
            .map(|d| self.to_original(d))
            .collect();
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
        FullReport {
            model: model.to_string(),
            check: report,
            structural,
            diagnostics,
        }
    }

    /// 診断の位置（正規化した本文の中）を、元の本文の中に戻す。ワークスペースに無いファイルはそのまま。
    fn to_original(&self, d: &Diagnostic) -> Diagnostic {
        let mut d = d.clone();
        if let (Some(path), Some(range)) = (&d.file, d.range)
            && let Some(file) = self.files.get(&normalize_path(path)).copied()
        {
            let map = shape_map(self, file);
            d.range = Some(globe_mode_core::TextRange::new(
                normalize::to_original(map, range.start),
                normalize::to_original(map, range.end),
            ));
        }
        d
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

//! salsa のクエリ DB（遅延評価・差分だけの再計算）。ADR-0005。
//!
//! ```text
//! SourceFile ──▶ outline ──▶ outline_shape ──▶ class_index ──▶ connect_graph
//!   (入力)        (位置あり)    (位置なし・Eq)     (Workspace)      (GraphRequest)
//!                               ▲ 防火壁 1
//! ```
//!
//! - `outline` はファイルの本文が変わるたびに作り直す（エディタの位置の情報に使う）。
//! - `outline_shape` は位置を落とした写し。空白やコメントだけの編集では前と等しくなるので、
//!   salsa がそれ以降（`class_index`・`connect_graph`）の再計算を省く（backdate）。
//! - どのクエリが実行されたかは [`GlobeDb::take_executed`] で取り出せる（テストと計測に使う）。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use globe_mode_core::Diagnostic;
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_graph::{ConnectGraph, GraphError};
use globe_mode_ir::{ClassIndex, Outline};
use salsa::Setter;

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

/// GlobeMode の DB。ファイルの出し入れと、クエリの呼び出しをまとめる。
#[salsa::db]
#[derive(Clone)]
pub struct GlobeDb {
    storage: salsa::Storage<Self>,
    executed: Arc<Mutex<Vec<String>>>,
    files: BTreeMap<String, SourceFile>,
    workspace: Option<Workspace>,
}

#[salsa::db]
impl salsa::Database for GlobeDb {}

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
    pub fn new() -> Self {
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
        }
    }

    /// ファイルを足すか、本文を差し替える。
    ///
    /// パスは区切りを `/` にそろえてから使う（Windows の `\\` と混ざっても同じファイルになるように）。
    pub fn set_file(&mut self, path: &str, text: impl Into<String>) {
        let text = text.into();
        let path = &normalize(path);
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
        self.files.get(&normalize(path)).map(|f| outline(self, *f))
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

    /// これまでに実行されたクエリの名前（実行順）を取り出して、記録を空にする。
    pub fn take_executed(&self) -> Vec<String> {
        self.executed
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }
}

/// パスの区切りを `/` にそろえる。
fn normalize(path: &str) -> String {
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

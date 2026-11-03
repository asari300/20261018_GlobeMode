//! GlobeMode の LSP サーバ（S038）。stdio の JSON-RPC を自前で回し、型だけ `lsp-types` を使う。
//!
//! 機能: 診断（`publishDiagnostics`、検査 S0〜S10 と lint）、`documentSymbol`（アウトライン）、
//! `hover`（クラスの種類）、`formatting`（S053、`globe-mode-fmt`）、`completion`（S053、[`completion`]）。
//! `definition`・`references`（S132、[`definition`]）。同期は Full で、要求は 1 本のスレッドで順に処理する
//! （S136: 入力の読みは別のスレッド。`$/cancelRequest` で `globe/simulate`・`globe/simulateColumns` を止め、
//! params に `workDoneToken` があれば `$/progress` で進捗を出す。ADR-0033）。
//!
//! 独自の要求（S049、ADR-0023。Desktop の UI が sidecar の `globe lsp` に同じ接続で頼む）:
//!
//! | 要求 | params | result |
//! | --- | --- | --- |
//! | `globe/check` | `{model}` | `CheckResponse` の JSON |
//! | `globe/simulate` | `{request}`（`SimulateRequest` の JSON） | `SimulateResponse` の JSON |
//! | `globe/simulateColumns` | `{request}` | `{response, layout, columns}`（S076。wasm の `simulate_columns` と同じ形。`response` に CSV は無く、`layout` は `ColumnsLayout`、`columns[i]` は列 `i` の数の配列で、有限でない値は null） |
//! | `globe/graph` | `{model, depth}` | graphology の JSON（`ConnectGraph::to_graphology`） |
//! | `globe/format` | `{text, indent?, insertOrigin?, blankAroundSections?, formatHtml?}` | 整形後の本文（S053。壊れていれば -32000） |
//! | `globe/lint` | `{text}` | `[{rule, line, column, message}]`（S053） |
//! | `globe/libraryTree` | `{prefix}`（空ならトップレベル） | `[{name, path, kind, description, file, hasChildren}]`（S127。直下の子だけ。wasm の `library_tree` と同じ形） |
//! | `globe/librarySource` | `{path}`（完全修飾名） | `{file, text}`（S127。そのクラスを定義するライブラリのファイル。無ければ -32000） |
//!
//! 設定ファイル（S130）: `initialize` の `rootUri`（無ければ `rootPath`）から上へたどって最初の `globe-mode.toml` を
//! 読み、lint の診断と `textDocument/formatting` に使う（`workspace/didChangeConfiguration` で読み直す）。
//! `globe/format`・`globe/lint` は wasm と同じく params だけで決める。
//!
//! params が読めなければ -32602、モデルが無いなど計算の失敗は -32000（`message` に理由）。
//! facade（`globe-mode`）の `Workspace` だけを呼び、新しい計算は入れない。

mod completion;
mod definition;

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;

use globe_mode::db::normalize_path;
use globe_mode::{
    CancelToken, ColumnsLayout, Diagnostic, Progress, SimulateHandle, SimulateRequest, Simulated,
    Workspace,
};
use globe_mode_core::Severity;
use globe_mode_ir::{ClassKind, ClassOutline};
use lsp_types::{
    DiagnosticSeverity, DocumentSymbol, Hover, HoverContents, MarkupContent, MarkupKind,
    NumberOrString, Position, Range, SymbolKind, Uri,
};
use serde_json::{Value, json};

/// 1 つのクライアントとの会話の状態。
#[derive(Debug)]
pub struct Server {
    ws: Workspace,
    /// 開いているファイルの本文（キーは URI の文字列）。位置の換算に使う
    texts: HashMap<String, String>,
    libraries: Vec<PathBuf>,
    /// ライブラリの木（S127。最初の `globe/libraryTree` で全ファイルの見出しを読んで作る）
    tree: Option<globe_mode_fmt::LibraryTree>,
    /// ワークスペースの根（`initialize` の `rootUri`）
    root: Option<PathBuf>,
    /// 根の `globe-mode.toml`（S130）。無ければ None
    config: Option<globe_mode_fmt::LintOptions>,
    /// 定義ジャンプで読んだライブラリのファイルのアウトラインだけを持つ（S132。検査には使わない）
    scratch: Workspace,
}

impl Server {
    /// `libraries` を一度だけ読む（読めなければ S1 の診断）。
    pub fn new(libraries: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        Ok(Server {
            ws: Workspace::with_libraries(libraries)?,
            texts: HashMap::new(),
            libraries: libraries.to_vec(),
            tree: None,
            root: None,
            config: None,
            scratch: Workspace::new(),
        })
    }

    fn tree(&mut self) -> &globe_mode_fmt::LibraryTree {
        self.tree.get_or_insert_with(|| {
            let mut files = Vec::new();
            for lib in &self.libraries {
                collect_mo(lib, &mut files);
            }
            // 名前は Workspace::with_libraries がライブラリのファイルに付けるのと同じ（display）。開くと同じ文書を差し替える
            let texts: Vec<(String, String)> = files
                .iter()
                .filter_map(|p| Some((p.display().to_string(), std::fs::read_to_string(p).ok()?)))
                .collect();
            globe_mode_fmt::LibraryTree::new(texts.iter().map(|(f, t)| (f.as_str(), t.as_str())))
        })
    }

    /// 根から上へたどって `globe-mode.toml` を読み直す（S130）。誤りは `window/showMessage` で知らせ、既定で続ける。
    fn load_config(&mut self) -> Vec<Value> {
        let found = self
            .root
            .as_deref()
            .map(globe_mode_fmt::LintOptions::find)
            .transpose()
            .map(Option::flatten);
        match found {
            Ok(c) => {
                self.config = c;
                vec![]
            }
            Err(e) => {
                self.config = None;
                vec![json!({"jsonrpc": "2.0", "method": "window/showMessage",
                    "params": {"type": 1, "message": format!("globe-mode.toml: {e}")}})]
            }
        }
    }

    /// 受けたメッセージ 1 つに対して、送り返すメッセージ（応答と通知）を返す。
    pub fn handle(&mut self, msg: Value) -> Vec<Value> {
        let mut notes = Vec::new();
        let replies = self.handle_with(msg, None, &mut |m| notes.push(m));
        notes.extend(replies);
        notes
    }

    /// [`Server::handle`] と同じだが、実行の要求は `cancel` で止められ、進捗（`$/progress`）は返り値を待たずに
    /// `notify` へ渡す（S136、ADR-0033）。
    pub fn handle_with(
        &mut self,
        msg: Value,
        cancel: Option<CancelToken>,
        notify: &mut dyn FnMut(Value),
    ) -> Vec<Value> {
        let id = msg.get("id").cloned();
        let params = &msg["params"];
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match msg["method"].as_str().unwrap_or("") {
            "initialize" => {
                self.root = params["rootUri"]
                    .as_str()
                    .map(uri_path)
                    .or_else(|| params["rootPath"].as_str().map(str::to_string))
                    .map(PathBuf::from);
                let mut out = reply(
                    id,
                    json!({
                        // 1 = Full
                        "capabilities": {"textDocumentSync": 1, "documentSymbolProvider": true, "hoverProvider": true,
                            "documentFormattingProvider": true, "completionProvider": {"triggerCharacters": ["."]},
                            "definitionProvider": true, "referencesProvider": true},
                        "serverInfo": {"name": "globe-mode", "version": env!("CARGO_PKG_VERSION")},
                    }),
                );
                out.extend(self.load_config());
                out
            }
            // S130: 設定ファイルを読み直し、開いている文書の診断を出し直す
            "workspace/didChangeConfiguration" => {
                let mut out = self.load_config();
                let open: Vec<_> = self.texts.clone().into_iter().collect();
                for (uri, text) in open {
                    out.extend(self.update(uri, text));
                }
                out
            }
            "shutdown" => reply(id, Value::Null),
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or("");
                self.update(uri, text.to_string())
            }
            "textDocument/didChange" => {
                // Full 同期: 最後の変更が本文全体
                let text = params["contentChanges"]
                    .as_array()
                    .and_then(|a| a.last())
                    .and_then(|c| c["text"].as_str())
                    .unwrap_or("");
                self.update(uri, text.to_string())
            }
            "textDocument/didClose" => {
                self.texts.remove(&uri);
                self.ws.remove_file(&uri_path(&uri));
                vec![publish(&uri, &[])]
            }
            "textDocument/documentSymbol" => {
                let symbols = match (self.ws.outline(&uri_path(&uri)), self.texts.get(&uri)) {
                    (Some(Ok(o)), Some(text)) => Some(
                        o.classes
                            .iter()
                            .map(|c| symbol(c, text))
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                };
                reply(id, json!(symbols))
            }
            "textDocument/hover" => {
                let pos = &params["position"];
                let pos = Position::new(
                    pos["line"].as_u64().unwrap_or(0) as u32,
                    pos["character"].as_u64().unwrap_or(0) as u32,
                );
                let hover = match (self.ws.outline(&uri_path(&uri)), self.texts.get(&uri)) {
                    (Some(Ok(o)), Some(text)) => {
                        innermost(&o.classes, offset(text, pos) as u32).map(hover)
                    }
                    _ => None,
                };
                reply(id, json!(hover))
            }
            "textDocument/formatting" => {
                // 文書全体を 1 つの TextEdit で置き換える。整形できない（文法が壊れている）ときは null
                // FormattingOptions の追加のキー: insertOrigin・blankAroundSections（S061。VS Code の拡張が設定から渡す）
                // S130: globe-mode.toml があればそれを土台にし、indent は tabSize より設定ファイルを採る（ruff と同じ）
                let opts = &params["options"];
                let base = self.config.clone().unwrap_or_default().format;
                let indent = match (&self.config, opts["tabSize"].as_u64()) {
                    (None, Some(n)) => n as usize,
                    _ => base.indent,
                };
                let options = globe_mode_fmt::Options {
                    indent,
                    insert_origin: opts["insertOrigin"].as_bool().unwrap_or(base.insert_origin),
                    blank_around_sections: opts["blankAroundSections"]
                        .as_bool()
                        .unwrap_or(base.blank_around_sections),
                    format_html: opts["formatHtml"].as_bool().unwrap_or(base.format_html),
                    ..base
                };
                let edit = self.texts.get(&uri).and_then(|text| {
                    let out = globe_mode_fmt::format(text, &options).ok()?;
                    (out != *text).then(|| {
                        json!([{"range": range(text, globe_mode_core::TextRange { start: 0, end: text.len() as u32 }), "newText": out}])
                    })
                });
                reply(id, edit.unwrap_or(Value::Null))
            }
            "textDocument/completion" => {
                let pos = &params["position"];
                let pos = Position::new(
                    pos["line"].as_u64().unwrap_or(0) as u32,
                    pos["character"].as_u64().unwrap_or(0) as u32,
                );
                let items = match self.texts.get(&uri) {
                    Some(text) => {
                        completion::complete(&self.ws, &uri_path(&uri), text, offset(text, pos))
                    }
                    None => vec![],
                };
                reply(id, json!(items))
            }
            "textDocument/definition" => {
                let Some(text) = self.texts.get(&uri).cloned() else {
                    return reply(id, Value::Null);
                };
                let at = offset(&text, lsp_position(&params["position"]));
                self.tree();
                let tree = self.tree.as_ref().expect("tree() で作った");
                let target = definition::Resolver {
                    ws: &self.ws,
                    tree,
                    scratch: &mut self.scratch,
                }
                .definition(&uri_path(&uri), &text, at);
                let location = target.and_then(|t| {
                    // 開いている文書ならその本文と URI、無ければディスクから読む
                    let open = self.texts.iter().find(|(u, _)| uri_path(u) == t.path);
                    let (u, body) = match open {
                        Some((u, body)) => (u.clone(), body.clone()),
                        None => (file_uri(&t.path), std::fs::read_to_string(&t.path).ok()?),
                    };
                    Some(json!({"uri": u, "range": range(&body, t.range)}))
                });
                reply(id, location.unwrap_or(Value::Null))
            }
            "textDocument/references" => {
                let refs: Vec<Value> = match self.texts.get(&uri) {
                    Some(text) => {
                        let at = offset(text, lsp_position(&params["position"]));
                        let o = self.ws.outline(&uri_path(&uri)).and_then(|r| r.as_ref().ok());
                        definition::references(o, text, at)
                            .into_iter()
                            .map(|r| json!({"uri": uri, "range": range(text, r)}))
                            .collect()
                    }
                    None => vec![],
                };
                reply(id, json!(refs))
            }
            "globe/check" => match params["model"].as_str() {
                Some(model) => reply(id, json!(self.ws.check(model))),
                None => error(id, -32602, "params.model が文字列でない"),
            },
            "globe/simulate" => {
                match serde_json::from_value::<SimulateRequest>(params["request"].clone()) {
                    Ok(request) => {
                        let s = self.simulate(&request, params.get("workDoneToken"), cancel, notify);
                        reply(id, json!(s.response))
                    }
                    Err(e) => error(id, -32602, &format!("params.request: {e}")),
                }
            }
            "globe/simulateColumns" => {
                match serde_json::from_value::<SimulateRequest>(params["request"].clone()) {
                    Ok(mut request) => {
                        request.no_csv = true;
                        let Simulated { response, run, .. } =
                            self.simulate(&request, params.get("workDoneToken"), cancel, notify);
                        let (names, columns) = match run {
                            Some(run) => {
                                let t = run.trajectory;
                                let names = std::iter::once("time".to_string())
                                    .chain(t.variables.iter().map(|v| v.name.clone()))
                                    .collect();
                                let columns: Vec<Vec<f64>> = std::iter::once(t.time)
                                    .chain(t.variables.into_iter().map(|v| v.values))
                                    .collect();
                                (names, columns)
                            }
                            None => (Vec::new(), Vec::new()),
                        };
                        let points = columns.first().map_or(0, Vec::len);
                        // serde_json は NaN / Inf を null にする（UI の欠損と同じ）
                        reply(
                            id,
                            json!({"response": response, "layout": ColumnsLayout { names, points }, "columns": columns}),
                        )
                    }
                    Err(e) => error(id, -32602, &format!("params.request: {e}")),
                }
            }
            "globe/graph" => match params["model"].as_str() {
                Some(model) => {
                    let depth = params["depth"].as_u64().unwrap_or(1) as u32;
                    match self.ws.graph(model, depth) {
                        Ok(g) => reply(id, g.to_graphology()),
                        Err(e) => error(id, -32000, &e.to_string()),
                    }
                }
                None => error(id, -32602, "params.model が文字列でない"),
            },
            // 文書に依らない整形と lint（S053。Desktop の UI が wasm と同じ口で頼む）
            "globe/format" => match params["text"].as_str() {
                Some(text) => {
                    let options = globe_mode_fmt::Options {
                        indent: params["indent"].as_u64().map_or(2, |n| n as usize),
                        insert_origin: params["insertOrigin"].as_bool().unwrap_or(true),
                        blank_around_sections: params["blankAroundSections"]
                            .as_bool()
                            .unwrap_or(true),
                        format_html: params["formatHtml"].as_bool().unwrap_or(true),
                        ..globe_mode_fmt::Options::default()
                    };
                    match globe_mode_fmt::format(text, &options) {
                        Ok(out) => reply(id, json!(out)),
                        Err(e) => error(id, -32000, &e.message),
                    }
                }
                None => error(id, -32602, "params.text が文字列でない"),
            },
            "globe/lint" => match params["text"].as_str() {
                Some(text) => reply(
                    id,
                    json!(globe_mode_fmt::lint(text, &globe_mode_fmt::LintOptions::default())
                        .iter()
                        .map(|m| json!({"rule": m.rule, "line": m.line, "column": m.column, "message": m.message}))
                        .collect::<Vec<_>>()),
                ),
                None => error(id, -32602, "params.text が文字列でない"),
            },
            "globe/libraryTree" => {
                let prefix = params["prefix"].as_str().unwrap_or("");
                let entries: Vec<Value> = self
                    .tree()
                    .children(prefix)
                    .iter()
                    .map(|e| json!({"name": e.name, "path": e.path, "kind": e.kind,
                        "description": e.description, "file": e.file, "hasChildren": e.has_children}))
                    .collect();
                reply(id, json!(entries))
            }
            "globe/librarySource" => match params["path"].as_str() {
                Some(path) => match self.tree().file(path).map(|f| (f.to_string(), std::fs::read_to_string(f))) {
                    Some((file, Ok(text))) => reply(id, json!({"file": file, "text": text})),
                    Some((file, Err(e))) => error(id, -32000, &format!("{file}: {e}")),
                    None => error(id, -32000, &format!("ライブラリに {path} が無い")),
                },
                None => error(id, -32602, "params.path が文字列でない"),
            },
            // 要求（id あり）は断る。知らない通知は無視する
            _ => match id {
                Some(id) => vec![
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}}),
                ],
                None => vec![],
            },
        }
    }

    /// 本文を差し替えて、診断を出し直す。
    /// 実行（ADR-0033）。`token`（params の `workDoneToken`）があれば `$/progress` の begin・report・end を出す。
    /// report の value には LSP の `message`（段）と `percentage`（t のあるときだけ）に、facade の `Progress` の
    /// `stage`・`time`・`start_time`・`stop_time` を添える（Desktop の UI がそのまま使う）。
    fn simulate(
        &mut self,
        request: &SimulateRequest,
        token: Option<&Value>,
        cancel: Option<CancelToken>,
        notify: &mut dyn FnMut(Value),
    ) -> Simulated {
        let mut progress = |value: Value| {
            if let Some(token) = token {
                notify(json!({"jsonrpc": "2.0", "method": "$/progress",
                    "params": {"token": token, "value": value}}));
            }
        };
        progress(
            json!({"kind": "begin", "title": format!("simulate {}", request.model), "cancellable": true}),
        );
        let mut on_progress = |p: &Progress| {
            let stage = format!("{:?}", p.stage).to_lowercase();
            let mut v = json!({"kind": "report", "message": stage, "stage": stage,
                "start_time": p.start_time, "stop_time": p.stop_time});
            if let Some(t) = p.time {
                let pct = (t - p.start_time) / (p.stop_time - p.start_time) * 100.0;
                v["time"] = json!(t);
                // 幅が 0 なら NaN で、`as` は 0 にする
                v["percentage"] = json!(pct.clamp(0.0, 100.0) as u32);
            }
            progress(v);
        };
        let handle = SimulateHandle {
            on_progress: Some(&mut on_progress),
            cancel,
        };
        let s = self
            .ws
            .simulate_observed(request, &globe_mode::now_ms, handle);
        progress(json!({"kind": "end"}));
        s
    }

    fn update(&mut self, uri: String, text: String) -> Vec<Value> {
        let path = uri_path(&uri);
        self.ws.set_file(&path, text.clone());
        let diags = self.diagnostics(&path, &text);
        self.texts.insert(uri.clone(), text);
        vec![publish(&uri, &diags)]
    }

    /// 構文の誤り（S0）はそのまま、通れば最上位の model / block / class を 1 つずつ検査する。
    /// shortcut: パッケージの中のモデルは検査しない。必要になったら再帰する。
    fn diagnostics(&mut self, path: &str, text: &str) -> Vec<lsp_types::Diagnostic> {
        let models: Vec<String> = match self.ws.outline(path) {
            Some(Err(diags)) => return diags.iter().map(|d| lsp_diagnostic(d, text)).collect(),
            Some(Ok(o)) => o
                .classes
                .iter()
                .filter(|c| {
                    matches!(
                        c.kind,
                        ClassKind::Model | ClassKind::Block | ClassKind::Class
                    )
                })
                .map(|c| match &o.within {
                    Some(w) => format!("{w}.{}", c.name),
                    None => c.name.clone(),
                })
                .collect(),
            None => return vec![],
        };
        let key = normalize_path(path);
        let lint = self.config.clone().unwrap_or_default();
        let mut out: Vec<lsp_types::Diagnostic> = lint_diagnostics(text, &lint);
        // S062: 型の食い違う connect は、ワークスペースの型で解決できたものだけ warning にする
        let mismatches = if lint.disabled.iter().any(|d| d == "connect-type-mismatch") {
            vec![]
        } else {
            self.ws.connect_mismatches(path)
        };
        for m in mismatches {
            out.push(lsp_types::Diagnostic {
                range: range(text, m.connect.range),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("GM-L0005".to_string())),
                source: Some("globe-mode".to_string()),
                message: format!(
                    "{} は {}、{} は {} で、型が合わない (connect-type-mismatch)",
                    m.connect.lhs, m.lhs_type, m.connect.rhs, m.rhs_type
                ),
                ..Default::default()
            });
        }
        for model in models {
            for d in self.ws.check(&model).diagnostics {
                if d.file.as_deref().is_none_or(|f| f == key) {
                    let d = lsp_diagnostic(&d, text);
                    if !out.contains(&d) {
                        out.push(d);
                    }
                }
            }
        }
        out
    }
}

/// lint（S053）を LSP の診断（warning、GM-L0001〜）にする。
fn lint_diagnostics(
    text: &str,
    options: &globe_mode_fmt::LintOptions,
) -> Vec<lsp_types::Diagnostic> {
    globe_mode_fmt::lint(text, options)
        .into_iter()
        .map(|m| {
            let (code, severity) = match m.rule {
                "line-too-long" => ("GM-L0001", DiagnosticSeverity::WARNING),
                "class-name-pascal-case" => ("GM-L0002", DiagnosticSeverity::WARNING),
                "missing-description" => ("GM-L0003", DiagnosticSeverity::WARNING),
                // S061: 整形で直る並びの指摘は通知の重さ
                _ => ("GM-L0004", DiagnosticSeverity::INFORMATION),
            };
            let start = Position::new(
                m.line.saturating_sub(1) as u32,
                m.column.saturating_sub(1) as u32,
            );
            lsp_types::Diagnostic {
                range: Range::new(start, start),
                severity: Some(severity),
                code: Some(NumberOrString::String(code.to_string())),
                source: Some("globe-mode".to_string()),
                message: format!("{} ({})", m.message, m.rule),
                ..Default::default()
            }
        })
        .collect()
}

/// 標準入出力で LSP を回す。`exit` か入力の終わりで返る。
pub fn run_stdio(libraries: &[PathBuf]) -> io::Result<()> {
    let server = Server::new(libraries).map_err(|ds| {
        let msgs: Vec<_> = ds.iter().map(|d| d.message.as_str()).collect();
        io::Error::other(msgs.join("; "))
    })?;
    serve(server, io::BufReader::new(io::stdin()), io::stdout().lock())
}

/// 実行の要求（`$/cancelRequest` で止められるもの。ADR-0033）。
fn cancellable(method: &str) -> bool {
    matches!(method, "globe/simulate" | "globe/simulateColumns")
}

/// 読みと計算を分けて回す（S136、ADR-0033）。読みのスレッドは枠を読んでキューに積み、実行の要求には
/// トークンを作って表に置く。`$/cancelRequest` はキューに積まず、表のトークンを立てる（待っている要求にも効く）。
/// 計算はこのスレッドで 1 度に 1 つ。書くのもこのスレッドだけ。
fn serve(
    mut server: Server,
    mut input: impl BufRead + Send + 'static,
    mut out: impl Write,
) -> io::Result<()> {
    let tokens: Arc<Mutex<HashMap<String, CancelToken>>> = Arc::default();
    let (tx, rx) = mpsc::channel::<io::Result<(Value, Option<CancelToken>)>>();
    let table = tokens.clone();
    let reader = thread::spawn(move || {
        loop {
            let msg = match read_message(&mut input) {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            };
            let method = msg["method"].as_str().unwrap_or("");
            if method == "exit" {
                break;
            }
            if method == "$/cancelRequest" {
                if let Some(t) = lock(&table).get(&msg["params"]["id"].to_string()) {
                    t.cancel();
                }
                continue;
            }
            let token = (cancellable(method) && msg.get("id").is_some()).then(|| {
                let t = CancelToken::new();
                lock(&table).insert(msg["id"].to_string(), t.clone());
                t
            });
            if tx.send(Ok((msg, token))).is_err() {
                break;
            }
        }
    });
    let mut write = |m: Value| -> io::Result<()> {
        let body = m.to_string();
        write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
        out.flush()
    };
    for got in rx {
        let (msg, token) = got?;
        let id = msg["id"].to_string();
        let mut failed = Ok(());
        let replies = server.handle_with(msg, token, &mut |m| {
            if failed.is_ok() {
                failed = write(m);
            }
        });
        lock(&tokens).remove(&id);
        failed?;
        for m in replies {
            write(m)?;
        }
    }
    // 読みのスレッドは入力の終わりか `exit` で抜けている（キューが閉じたので）
    let _ = reader.join();
    Ok(())
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// `Content-Length` の枠 1 つを読む。入力が終わっていれば None。
fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.eq_ignore_ascii_case("content-length")
        {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let len =
        len.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Content-Length が無い"))?;
    let mut buf = vec![0; len];
    input.read_exact(&mut buf)?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// 要求（id あり）への応答。通知なら何も返さない。
fn reply(id: Option<Value>, result: Value) -> Vec<Value> {
    id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": result}))
        .into_iter()
        .collect()
}

/// 要求（id あり）へのエラー応答。通知なら何も返さない。
fn error(id: Option<Value>, code: i32, message: &str) -> Vec<Value> {
    id.map(|id| json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
        .into_iter()
        .collect()
}

fn publish(uri: &str, diagnostics: &[lsp_types::Diagnostic]) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
           "params": {"uri": uri, "diagnostics": diagnostics}})
}

/// URI をワークスペースのパスにする。`file:` はパーセントを戻した path、それ以外は URI の文字列のまま。
/// `path` の下の `.mo`（`path` が `.mo` ならそれ自身）。読めないディレクトリは飛ばす。
fn collect_mo(path: &std::path::Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
            collect_mo(&entry.path(), out);
        }
    } else if path.extension().is_some_and(|e| e == "mo") {
        out.push(path.to_path_buf());
    }
}

/// LSP の `Position` の JSON（読めない欄は 0）。
fn lsp_position(p: &Value) -> Position {
    Position::new(
        p["line"].as_u64().unwrap_or(0) as u32,
        p["character"].as_u64().unwrap_or(0) as u32,
    )
}

/// パス → `file:` の URI（[`uri_path`] の逆。英数字と `/-._~:` 以外はパーセントにする）。
fn file_uri(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut out = String::from(if path.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn uri_path(uri: &str) -> String {
    match uri.parse::<Uri>() {
        Ok(u) if u.scheme().is_some_and(|s| s.as_str() == "file") => {
            let p = u.path().as_estr().decode().into_string_lossy().into_owned();
            // Windows: `/C:/x` → `C:/x`
            match p.as_bytes() {
                [b'/', d, b':', ..] if d.is_ascii_alphabetic() => p[1..].to_string(),
                _ => p,
            }
        }
        _ => uri.to_string(),
    }
}

/// バイト位置 → LSP の位置（行は 0 始まり、列は UTF-16 の単位）。範囲外は末尾、文字の途中は手前に寄せる。
fn position(text: &str, offset: u32) -> Position {
    let mut end = (offset as usize).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line = before.matches('\n').count() as u32;
    let tail = before.rsplit('\n').next().unwrap_or("");
    Position::new(line, tail.encode_utf16().count() as u32)
}

/// LSP の位置 → バイト位置。行の終わりを越える列は行末、行が足りなければ本文の末尾。
fn offset(text: &str, pos: Position) -> usize {
    let mut start = 0;
    for _ in 0..pos.line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let line = &text[start..];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    let mut units = 0;
    for (i, c) in line.char_indices() {
        if units >= pos.character {
            return start + i;
        }
        units += c.len_utf16() as u32;
    }
    start + line.len()
}

fn range(text: &str, r: globe_mode_core::TextRange) -> Range {
    Range::new(position(text, r.start), position(text, r.end))
}

fn lsp_diagnostic(d: &Diagnostic, text: &str) -> lsp_types::Diagnostic {
    let mut message = d.message.clone();
    for n in &d.notes {
        message.push('\n');
        message.push_str(n);
    }
    lsp_types::Diagnostic {
        range: d.range.map_or_else(Range::default, |r| range(text, r)),
        severity: Some(match d.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(d.code.to_string())),
        source: Some("globe-mode".to_string()),
        message,
        ..Default::default()
    }
}

fn keyword(kind: ClassKind) -> &'static str {
    match kind {
        ClassKind::Model => "model",
        ClassKind::Class => "class",
        ClassKind::Block => "block",
        ClassKind::Connector => "connector",
        ClassKind::Record => "record",
        ClassKind::Type => "type",
        ClassKind::Package => "package",
        ClassKind::Function => "function",
        ClassKind::Operator => "operator",
    }
}

/// クラス → シンボル。子は入れ子のクラスとコンポーネント。
#[allow(deprecated)] // DocumentSymbol::deprecated は構造体の初期化で必ず書く
fn symbol(c: &ClassOutline, text: &str) -> DocumentSymbol {
    let r = range(text, c.range);
    let kind = match c.kind {
        ClassKind::Package => SymbolKind::MODULE,
        ClassKind::Record => SymbolKind::STRUCT,
        ClassKind::Connector => SymbolKind::INTERFACE,
        ClassKind::Function | ClassKind::Operator => SymbolKind::FUNCTION,
        _ => SymbolKind::CLASS,
    };
    let mut children: Vec<_> = c.classes.iter().map(|k| symbol(k, text)).collect();
    children.extend(c.components.iter().map(|v| {
        let r = range(text, v.range);
        DocumentSymbol {
            name: v.name.clone(),
            detail: Some(v.type_name.clone()),
            kind: SymbolKind::FIELD,
            tags: None,
            deprecated: None,
            range: r,
            selection_range: r,
            children: None,
        }
    }));
    DocumentSymbol {
        name: c.name.clone(),
        detail: Some(keyword(c.kind).to_string()),
        kind,
        tags: None,
        deprecated: None,
        range: r,
        selection_range: r,
        children: Some(children),
    }
}

/// 位置（バイト）を含む一番内側のクラス。
fn innermost(classes: &[ClassOutline], at: u32) -> Option<&ClassOutline> {
    let c = classes
        .iter()
        .find(|c| c.range.start <= at && at < c.range.end)?;
    Some(innermost(&c.classes, at).unwrap_or(c))
}

fn hover(c: &ClassOutline) -> Hover {
    let partial = if c.partial { "partial " } else { "" };
    let mut value = format!("**{partial}{} {}**", keyword(c.kind), c.name);
    if !c.extends.is_empty() {
        value.push_str(&format!("\n\nextends {}", c.extends.join(", ")));
    }
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_order_is_information_and_blank_sections_can_be_turned_off() {
        let src = "model A \"d\"\n  Real x;\nequation\n  x = 1;\n  annotation(Documentation(info = \"i\"), experiment(StopTime = 1));\nend A;\n";
        let d = lint_diagnostics(src, &globe_mode_fmt::LintOptions::default());
        let order: Vec<_> = d
            .iter()
            .filter(|d| d.code == Some(NumberOrString::String("GM-L0004".into())))
            .collect();
        assert_eq!(order.len(), 1, "{d:?}");
        assert_eq!(order[0].severity, Some(DiagnosticSeverity::INFORMATION));

        let mut s = Server::new(&[]).unwrap();
        // S062: formatHtml の切り替え
        let html = "model H \"d\"\n  annotation(Documentation(info = \"<html><body><p>x</p></body></html>\"));\nend H;\n";
        for (on, want) in [(true, "info = \"\n"), (false, "info = \"<html><body>")] {
            let r = s.handle(json!({"id": 1, "method": "globe/format", "params": {"text": html, "formatHtml": on}}));
            assert!(r[0]["result"].as_str().unwrap().contains(want), "{}", r[0]);
        }
        let r = s.handle(json!({"id": 1, "method": "globe/format", "params": {"text": src, "blankAroundSections": false}}));
        let out = r[0]["result"].as_str().unwrap();
        assert!(out.contains("  Real x;\nequation\n  x = 1;"), "{out}");
        assert!(
            out.contains("annotation(experiment(StopTime = 1), Documentation(info = \"i\"));"),
            "{out}"
        );
    }

    #[test]
    fn position_counts_utf16_units() {
        // 「日本語」は UTF-8 で 3 バイト、UTF-16 で 1 単位。絵文字は UTF-16 で 2 単位
        let text = "// 日本語😀x\nmodel A end A;";
        let x = text.find('x').unwrap() as u32;
        assert_eq!(position(text, x), Position::new(0, 3 + 3 + 2));
        assert_eq!(
            position(text, text.find("model").unwrap() as u32),
            Position::new(1, 0)
        );
        // 文字の途中は手前に寄せ、範囲外は末尾
        assert_eq!(position(text, 4), Position::new(0, 3));
        assert_eq!(position(text, 9999).line, 1);
    }

    #[test]
    fn offset_is_the_inverse_of_position() {
        let text = "// 日本語😀x\nmodel A end A;";
        for (i, _) in text.char_indices() {
            assert_eq!(offset(text, position(text, i as u32)), i);
        }
        assert_eq!(
            offset(text, Position::new(0, 999)),
            text.find('\n').unwrap()
        );
        assert_eq!(offset(text, Position::new(9, 0)), text.len());
    }

    #[test]
    fn uri_to_path() {
        assert_eq!(uri_path("file:///home/a%20b/X.mo"), "/home/a b/X.mo");
        assert_eq!(uri_path("file:///C:/x/X.mo"), "C:/x/X.mo");
        assert_eq!(uri_path("untitled:Untitled-1"), "untitled:Untitled-1");
    }

    #[test]
    fn diagnostic_mapping_keeps_code_notes_and_range() {
        let text = "// 日本語\nmodel A end A;";
        let mut d = Diagnostic::error("GM-E6001", globe_mode::Stage::S6, "壊れている");
        d.range = Some(globe_mode_core::TextRange { start: 0, end: 2 });
        d.notes = vec!["補足".into()];
        let l = lsp_diagnostic(&d, text);
        assert_eq!(l.code, Some(NumberOrString::String("GM-E6001".into())));
        assert_eq!(l.message, "壊れている\n補足");
        assert_eq!(l.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(
            l.range,
            Range::new(Position::new(0, 0), Position::new(0, 2))
        );
        d.range = None;
        assert_eq!(lsp_diagnostic(&d, text).range, Range::default());
    }

    #[test]
    fn server_answers_open_symbol_hover_and_unknown() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/A.mo";
        let out = s.handle(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "modelica", "version": 1,
            "text": "model A\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend A;\n"}}}),
        );
        assert_eq!(out[0]["method"], "textDocument/publishDiagnostics");
        // 警告と情報は出るが、誤り（severity 1）は無い
        let diags = out[0]["params"]["diagnostics"].as_array().unwrap();
        assert!(!diags.is_empty() && diags.iter().all(|d| d["severity"] != 1));
        let sym = s.handle(json!({"id": 1, "method": "textDocument/documentSymbol", "params": {"textDocument": {"uri": uri}}}));
        assert_eq!(sym[0]["result"][0]["name"], "A");
        assert_eq!(sym[0]["result"][0]["children"][0]["name"], "x");
        let h = s.handle(json!({"id": 2, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": 0, "character": 7}}}));
        assert_eq!(h[0]["result"]["contents"]["value"], "**model A**");
        let h = s.handle(json!({"id": 3, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": 9, "character": 0}}}));
        assert_eq!(h[0]["result"], Value::Null);
        assert_eq!(
            s.handle(json!({"id": 4, "method": "foo/bar"}))[0]["error"]["code"],
            -32601
        );
        assert!(s.handle(json!({"method": "foo/bar"})).is_empty());
    }

    #[test]
    fn connect_with_different_types_is_a_warning() {
        let mut s = Server::new(&[]).unwrap();
        let src = "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n\
                   connector Port\n  Real T;\n  flow Real Q;\nend Port;\n\
                   model R\n  Pin p;\n  Port h;\nend R;\n\
                   model C\n  R a;\n  R b;\nequation\n  connect(a.p, b.p);\n  connect(a.p, b.h);\nend C;\n";
        let out = s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "A.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let d: Vec<_> = out[0]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == "GM-L0005")
            .collect();
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0]["severity"], 2);
        assert_eq!(d[0]["range"]["start"]["line"], 17); // 2 本目の connect
        assert!(d[0]["message"].as_str().unwrap().contains("Port"));
    }

    #[test]
    fn custom_requests_check_simulate_and_graph() {
        let mut s = Server::new(&[]).unwrap();
        let src = "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n\
                   model R\n  Pin p;\n  Pin n;\nequation\n  p.v - n.v = p.i;\n  p.i + n.i = 0;\nend R;\n\
                   model C\n  R a;\n  R b;\nequation\n  connect(a.n, b.p);\nend C;\n\
                   model D\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend D;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "A.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let r = s.handle(json!({"id": 1, "method": "globe/check", "params": {"model": "D"}}));
        assert_eq!(r[0]["result"]["model"], "D");
        assert_eq!(r[0]["result"]["passed"], true);
        let r = s.handle(
            json!({"id": 2, "method": "globe/simulate", "params": {"request": {
            "model": "D", "settings": {"stop_time": 1.0}, "no_csv": true}}}),
        );
        assert_eq!(r[0]["result"]["ok"], true, "{}", r[0]);
        let x = r[0]["result"]["final"]["x"].as_f64().unwrap();
        assert!((x - (-1f64).exp()).abs() < 1e-2, "x = {x}");
        let r = s.handle(
            json!({"id": 4, "method": "globe/simulateColumns", "params": {"request": {
            "model": "D", "settings": {"stop_time": 1.0}}}}),
        );
        let res = &r[0]["result"];
        assert_eq!(res["response"]["ok"], true, "{}", r[0]);
        assert!(res["response"]["csv"].is_null());
        assert_eq!(res["layout"]["names"], json!(["time", "x"]));
        let cols = res["columns"].as_array().unwrap();
        let n = res["layout"]["points"].as_u64().unwrap() as usize;
        assert!(n > 1 && cols.iter().all(|c| c.as_array().unwrap().len() == n));
        assert_eq!(cols[1][n - 1].as_f64().unwrap(), x);
        let r = s.handle(
            json!({"id": 3, "method": "globe/graph", "params": {"model": "C", "depth": 1}}),
        );
        assert_eq!(
            r[0]["result"]["edges"].as_array().unwrap().len(),
            1,
            "{}",
            r[0]
        );
        // 読めない params は -32602、モデルが無ければ -32000、通知（id 無し）には何も返さない
        for (method, params, code) in [
            ("globe/check", json!({}), -32602),
            ("globe/simulate", json!({"request": {"nope": 1}}), -32602),
            (
                "globe/simulateColumns",
                json!({"request": {"nope": 1}}),
                -32602,
            ),
            ("globe/graph", json!({"model": "Nope"}), -32000),
        ] {
            let r = s.handle(json!({"id": 9, "method": method, "params": params}));
            assert_eq!(r[0]["error"]["code"], code, "{method}: {}", r[0]);
        }
        assert!(
            s.handle(json!({"method": "globe/check", "params": {}}))
                .is_empty()
        );
    }

    #[test]
    fn library_tree_and_source() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let r = s.handle(json!({"id": 1, "method": "globe/libraryTree", "params": {"prefix": ""}}));
        assert_eq!(r[0]["result"][0]["path"], "GlobeModeValidation", "{}", r[0]);
        let r = s.handle(json!({"id": 2, "method": "globe/libraryTree", "params": {"prefix": "GlobeModeValidation.Simulation"}}));
        assert!(
            r[0]["result"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["name"] == "Decay" && e["kind"] == "model"),
            "{}",
            r[0]
        );
        let model = "GlobeModeValidation.Simulation.Decay";
        let r =
            s.handle(json!({"id": 3, "method": "globe/librarySource", "params": {"path": model}}));
        let (file, text) = (
            r[0]["result"]["file"].as_str().unwrap(),
            r[0]["result"]["text"].as_str().unwrap(),
        );
        assert!(
            file.ends_with("Decay.mo") && text.contains("model Decay"),
            "{}",
            r[0]
        );
        // 開くとライブラリの同じファイルを差し替える（検査が通る）
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": file, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let r = s.handle(json!({"id": 4, "method": "globe/check", "params": {"model": model}}));
        assert_eq!(r[0]["result"]["passed"], true, "{}", r[0]);
        let r =
            s.handle(json!({"id": 5, "method": "globe/librarySource", "params": {"path": "Nope"}}));
        assert_eq!(r[0]["error"]["code"], -32000);
    }

    #[test]
    fn config_file_at_the_root_drives_lint_and_formatting() {
        let dir = std::env::temp_dir().join(format!("globe-lsp-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let toml = dir.join("globe-mode.toml");
        std::fs::write(
            &toml,
            "[fmt]\nindent = 4\n[lint]\ndisable = [\"GM-L0003\"]\n",
        )
        .unwrap();
        let mut s = Server::new(&[]).unwrap();
        let init = s.handle(json!({"id": 0, "method": "initialize", "params": {"rootPath": dir}}));
        assert_eq!(init.len(), 1, "{init:?}");
        let uri = "file:///tmp/C.mo";
        let text = "model C\nReal x;\nend C;\n";
        let out = s.handle(json!({"method": "textDocument/didOpen", "params": {"textDocument": {"uri": uri, "text": text}}}));
        assert!(!out[0].to_string().contains("GM-L0003"), "{}", out[0]);
        // エディタの tabSize 2 より設定ファイルの indent 4
        let f = s.handle(json!({"id": 1, "method": "textDocument/formatting", "params": {"textDocument": {"uri": uri}, "options": {"tabSize": 2}}}));
        assert!(
            f[0]["result"][0]["newText"]
                .as_str()
                .unwrap()
                .contains("\n    Real x;"),
            "{}",
            f[0]
        );
        // 誤りを書いて読み直すと showMessage で知らせ、既定に戻る
        std::fs::write(&toml, "[fmt]\nindnt = 4\n").unwrap();
        let out = s.handle(
            json!({"method": "workspace/didChangeConfiguration", "params": {"settings": {}}}),
        );
        assert_eq!(out[0]["method"], "window/showMessage");
        assert!(out[1].to_string().contains("GM-L0003"), "{}", out[1]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 文書を開き、`needle` の `k` 文字目に置いたカーソルで `method` を頼む。
    fn ask_at(
        s: &mut Server,
        uri: &str,
        text: &str,
        method: &str,
        needle: &str,
        k: usize,
    ) -> Value {
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let pos = position(text, (text.find(needle).unwrap() + k) as u32);
        s.handle(
            json!({"id": 1, "method": method, "params": {"textDocument": {"uri": uri},
            "position": pos, "context": {"includeDeclaration": true}}}),
        )
        .remove(0)
    }

    #[test]
    fn definition_of_a_type_and_a_component_in_the_same_document() {
        let mut s = Server::new(&[]).unwrap();
        let init = s.handle(json!({"id": 0, "method": "initialize", "params": {}}));
        assert_eq!(
            init[0]["result"]["capabilities"]["definitionProvider"],
            true
        );
        assert_eq!(
            init[0]["result"]["capabilities"]["referencesProvider"],
            true
        );
        let uri = "file:///tmp/D.mo";
        let text = "package D\n  connector Pin\n    Real v;\n    flow Real i;\n  end Pin;\n  model M\n    Pin p;\n  equation\n    p.v = 0;\n  end M;\nend D;\n";
        // 型 Pin の上 → connector Pin の範囲
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "Pin p;", 1);
        assert_eq!(r["result"]["uri"], uri, "{r}");
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 1, "character": 12}),
            "{r}"
        );
        // p.v の v → Pin の部品 v
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "p.v =", 2);
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 2, "character": 9}),
            "{r}"
        );
        // 予約語や数は null
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "= 0", 2);
        assert_eq!(r["result"], Value::Null, "{r}");
    }

    #[test]
    fn definition_into_another_file_of_the_library() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let uri = "file:///tmp/T.mo";
        let text = "model T\n  import GlobeModeValidation.Components;\n  Components.Resistor r1;\nequation\n  r1.p.v = 0;\nend T;\n";
        let r = ask_at(
            &mut s,
            uri,
            text,
            "textDocument/definition",
            "Resistor r1",
            1,
        );
        let u = r["result"]["uri"].as_str().unwrap_or_default();
        assert!(
            u.starts_with("file://") && u.ends_with("/Components.mo"),
            "{r}"
        );
        assert_eq!(r["result"]["range"]["start"]["line"], 13, "{r}");
        // r1.p の p は extends TwoPin の先の部品
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "r1.p.v", 3);
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 3, "character": 19}),
            "{r}"
        );
    }

    #[test]
    fn references_in_the_same_class() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/R.mo";
        let text = "model R\n  Real x;\n  Real xx;\nequation\n  der(x) = -x + xx;\nend R;\n";
        let r = ask_at(&mut s, uri, text, "textDocument/references", "x;", 0);
        let lines: Vec<_> = r["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["range"]["start"]["line"].clone())
            .collect();
        assert_eq!(lines, [json!(1), json!(4), json!(4)], "{r}");
    }

    /// S136（ADR-0033）: 読みを別のスレッドにした `serve` を pipe で回す。長い実行の進捗が `$/progress` で届き、
    /// `$/cancelRequest` で止めると、普通の応答に GM-I11004 と途中までの列が載る。
    #[test]
    fn cancel_request_stops_a_long_run_with_partial_columns() {
        const WIGGLE: &str = "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;";
        fn send(w: &mut impl Write, m: Value) {
            let b = m.to_string();
            write!(w, "Content-Length: {}\r\n\r\n{b}", b.len()).unwrap();
        }
        let (in_r, mut in_w) = io::pipe().unwrap();
        let (out_r, out_w) = io::pipe().unwrap();
        let server = Server::new(&[]).unwrap();
        let t = thread::spawn(move || serve(server, io::BufReader::new(in_r), out_w));
        let mut out = io::BufReader::new(out_r);
        send(
            &mut in_w,
            json!({"jsonrpc": "2.0", "id": 7, "method": "globe/simulateColumns", "params": {
                "workDoneToken": "run", "request": {"model": "Wiggle", "files": {"Wiggle.mo": WIGGLE},
                "settings": {"solver": "esdirk34", "stop_time": 1000.0, "interval": 1.0}}}}),
        );
        let mut kinds = Vec::new();
        loop {
            let m = read_message(&mut out).unwrap().unwrap();
            assert_eq!(m["method"], "$/progress", "{m}");
            kinds.push(m["params"]["value"]["kind"].clone());
            if m["params"]["value"]["time"].is_number() {
                assert!(m["params"]["value"]["percentage"].is_number(), "{m}");
                break;
            }
        }
        assert_eq!(kinds[0], "begin");
        send(
            &mut in_w,
            json!({"jsonrpc": "2.0", "method": "$/cancelRequest", "params": {"id": 7}}),
        );
        let r = loop {
            let m = read_message(&mut out).unwrap().unwrap();
            if m["id"] == 7 {
                break m;
            }
            kinds.push(m["params"]["value"]["kind"].clone());
        };
        assert_eq!(kinds.last().unwrap(), "end");
        let res = &r["result"];
        assert_eq!(
            res["response"]["termination"]["kind"], "cancelled",
            "{}",
            res["response"]
        );
        let codes: Vec<_> = res["response"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["code"].clone())
            .collect();
        assert!(codes.contains(&json!("GM-I11004")), "{codes:?}");
        let time = res["columns"][0].as_array().unwrap();
        let last = time.last().unwrap().as_f64().unwrap();
        assert!(last > 0.0 && last < 1000.0, "{last}");
        // exit で読みのスレッドが抜け、serve が返る
        send(&mut in_w, json!({"jsonrpc": "2.0", "method": "exit"}));
        t.join().unwrap().unwrap();
    }

    #[test]
    fn frames_are_read_by_content_length() {
        let body = r#"{"id":1,"method":"x"}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut input = raw.as_bytes();
        assert_eq!(read_message(&mut input).unwrap().unwrap()["method"], "x");
        assert!(read_message(&mut input).unwrap().is_none());
    }
}

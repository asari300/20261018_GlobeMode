//! GlobeMode の LSP サーバ（S038）。stdio の JSON-RPC を自前で回し、型だけ `lsp-types` を使う。
//!
//! 機能は 3 つ（標準）と独自の要求 3 つ: 診断（`publishDiagnostics`、検査 S0〜S10）、`documentSymbol`（アウトライン）、
//! `hover`（クラスの種類）。補完・定義へ移動は無い。同期は Full で、1 本のスレッドで順に処理する。
//!
//! 独自の要求（S049、ADR-0023。Desktop の UI が sidecar の `globe lsp` に同じ接続で頼む）:
//!
//! | 要求 | params | result |
//! | --- | --- | --- |
//! | `globe/check` | `{model}` | `CheckResponse` の JSON |
//! | `globe/simulate` | `{request}`（`SimulateRequest` の JSON） | `SimulateResponse` の JSON |
//! | `globe/graph` | `{model, depth}` | graphology の JSON（`ConnectGraph::to_graphology`） |
//!
//! params が読めなければ -32602、モデルが無いなど計算の失敗は -32000（`message` に理由）。
//! facade（`globe-mode`）の `Workspace` だけを呼び、新しい計算は入れない。

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use globe_mode::db::normalize_path;
use globe_mode::{Diagnostic, SimulateRequest, Workspace};
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
}

impl Server {
    /// `libraries` を一度だけ読む（読めなければ S1 の診断）。
    pub fn new(libraries: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        Ok(Server {
            ws: Workspace::with_libraries(libraries)?,
            texts: HashMap::new(),
        })
    }

    /// 受けたメッセージ 1 つに対して、送り返すメッセージ（応答と通知）を返す。
    pub fn handle(&mut self, msg: Value) -> Vec<Value> {
        let id = msg.get("id").cloned();
        let params = &msg["params"];
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match msg["method"].as_str().unwrap_or("") {
            "initialize" => reply(
                id,
                json!({
                    // 1 = Full
                    "capabilities": {"textDocumentSync": 1, "documentSymbolProvider": true, "hoverProvider": true},
                    "serverInfo": {"name": "globe-mode", "version": env!("CARGO_PKG_VERSION")},
                }),
            ),
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
            "globe/check" => match params["model"].as_str() {
                Some(model) => reply(id, json!(self.ws.check(model))),
                None => error(id, -32602, "params.model が文字列でない"),
            },
            "globe/simulate" => {
                match serde_json::from_value::<SimulateRequest>(params["request"].clone()) {
                    Ok(request) => reply(id, json!(self.ws.simulate(&request))),
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
        let mut out = Vec::new();
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

/// 標準入出力で LSP を回す。`exit` か入力の終わりで返る。
pub fn run_stdio(libraries: &[PathBuf]) -> io::Result<()> {
    let mut server = Server::new(libraries).map_err(|ds| {
        let msgs: Vec<_> = ds.iter().map(|d| d.message.as_str()).collect();
        io::Error::other(msgs.join("; "))
    })?;
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    while let Some(msg) = read_message(&mut input)? {
        if msg["method"] == "exit" {
            break;
        }
        for m in server.handle(msg) {
            let body = m.to_string();
            write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
            out.flush()?;
        }
    }
    Ok(())
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
    fn frames_are_read_by_content_length() {
        let body = r#"{"id":1,"method":"x"}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut input = raw.as_bytes();
        assert_eq!(read_message(&mut input).unwrap().unwrap()["method"], "x");
        assert!(read_message(&mut input).unwrap().is_none());
    }
}

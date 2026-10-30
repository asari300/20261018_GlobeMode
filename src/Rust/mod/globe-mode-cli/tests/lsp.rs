//! `globe lsp` を別プロセスで回し、診断・documentSymbol・hover が返ることを確かめる（S038）。

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn send(stdin: &mut ChildStdin, msg: Value) {
    let body = msg.to_string();
    write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    stdin.flush().unwrap();
}

fn recv(out: &mut BufReader<ChildStdout>) -> Value {
    let mut len = 0;
    loop {
        let mut line = String::new();
        assert!(out.read_line(&mut line).unwrap() > 0, "サーバが先に閉じた");
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse().unwrap();
        }
    }
    let mut buf = vec![0; len];
    out.read_exact(&mut buf).unwrap();
    serde_json::from_slice(&buf).unwrap()
}

/// 指定の id の応答まで読む（通知は読み捨てる。無限に待たないよう 50 通まで）
fn response(out: &mut BufReader<ChildStdout>, id: u64) -> Value {
    for _ in 0..50 {
        let m = recv(out);
        if m["id"] == id {
            return m;
        }
    }
    panic!("id {id} の応答が来ない");
}

#[test]
fn lsp_serves_diagnostics_symbols_and_hover() {
    let file = root().join("src/Modelica/GlobeModeValidation/Invalid/StructurallySingular.mo");
    let text = std::fs::read_to_string(&file).unwrap();
    let uri = format!("file://{}", file.canonicalize().unwrap().display());
    let mut child = Command::new(env!("CARGO_BIN_EXE_globe"))
        .current_dir(root())
        .args(["lsp", "--library", "src/Modelica"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run globe lsp");
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
    );
    let init = response(&mut out, 1);
    assert_eq!(init["result"]["serverInfo"]["name"], "globe-mode");
    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
    );
    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
    );

    let mut diags = None;
    for _ in 0..50 {
        let m = recv(&mut out);
        if m["method"] == "textDocument/publishDiagnostics" {
            diags = Some(m["params"]["diagnostics"].clone());
            break;
        }
    }
    let diags = diags.expect("publishDiagnostics が来ない");
    assert!(
        diags
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "GM-E6001"),
        "GM-E6001 が無い: {diags}"
    );

    let doc = json!({"uri": uri});
    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol", "params": {"textDocument": doc}}),
    );
    let symbols = response(&mut out, 2);
    assert_eq!(symbols["result"][0]["name"], "StructurallySingular");

    // 2 行目が `model StructurallySingular ...`
    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover",
        "params": {"textDocument": doc, "position": {"line": 1, "character": 10}}}),
    );
    let hover = response(&mut out, 3);
    assert!(
        hover["result"]["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("model")
    );

    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "id": 4, "method": "shutdown"}),
    );
    response(&mut out, 4);
    send(&mut stdin, json!({"jsonrpc": "2.0", "method": "exit"}));
    assert!(child.wait().unwrap().success());
}

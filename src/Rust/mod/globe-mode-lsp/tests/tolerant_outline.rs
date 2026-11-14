//! S276: 構文の誤りがあるファイルでも `documentSymbol` にクラスと部品が出る。正常なファイルは Rumoca のアウトラインのまま。

use globe_mode_lsp::Server;
use serde_json::{Value, json};

fn symbols(text: &str) -> Value {
    let mut s = Server::new(&[]).unwrap();
    let uri = "file:///tmp/T.mo";
    s.handle(
        json!({"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
    );
    s.handle(json!({"id": 1, "method": "textDocument/documentSymbol", "params": {"textDocument": {"uri": uri}}}))[0]
        ["result"]
        .clone()
}

fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn broken_file_still_has_classes_and_components() {
    let r = symbols(include_str!(
        "../../globe-mode-frontend/tests/data/SyntaxError.mo"
    ));
    assert_eq!(names(&r), ["Broken"]);
    assert_eq!(r[0]["detail"], "model");
    assert_eq!(names(&r[0]["children"]), ["x"]);
    assert_eq!(r[0]["children"][0]["detail"], "Real");

    // 途中まで書いたクラス（入れ子・`end` 無し）
    let r = symbols(
        "package P\n  model A\n    Real a;\n    parameter Real k = \n  model B\n    Real b\n",
    );
    assert_eq!(names(&r), ["P"]);
    let kids = names(&r[0]["children"]);
    assert!(kids.contains(&"A".to_string()), "{kids:?}");
}

#[test]
fn valid_file_is_unchanged() {
    let r = symbols("model M\n  Real x;\nequation\n  x = 1;\nend M;\n");
    assert_eq!(names(&r), ["M"]);
    assert_eq!(names(&r[0]["children"]), ["x"]);
    assert_eq!(r[0]["range"]["end"], json!({"line": 4, "character": 5}));
}

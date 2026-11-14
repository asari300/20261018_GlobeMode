//! `textDocument/foldingRange`（S220）: 入れ子のクラス・節・annotation・コメント、CRLF、全角の文字。

use globe_mode_lsp::Server;
use serde_json::{Value, json};

fn ranges(s: &mut Server, uri: &str) -> Vec<(u64, u64, Option<String>)> {
    let r = s.handle(json!({"id": 1, "method": "textDocument/foldingRange", "params": {"textDocument": {"uri": uri}}}));
    r[0]["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["startLine"].as_u64().unwrap(),
                f["endLine"].as_u64().unwrap(),
                f["kind"].as_str().map(str::to_string),
            )
        })
        .collect()
}

#[test]
fn classes_sections_annotations_and_comments_fold_by_line() {
    let mut s = Server::new(&[]).unwrap();
    let uri = "file:///tmp/F.mo";
    let lines = [
        "package P",                           // 0
        "  // a",                              // 1
        "  // b",                              // 2
        "  model M",                           // 3
        "    parameter Real x = 1 \"é説明\";", // 4
        "  initial equation",                  // 5
        "    x = 1;",                          // 6
        "  equation",                          // 7
        "    der(x) = -x;",                    // 8
        "",                                    // 9
        "    annotation(",                     // 10
        "      experiment(StopTime = 1));",    // 11
        "  end M;",                            // 12
        "end P;",                              // 13
    ];
    let text = lines.join("\r\n") + "\r\n";
    s.handle(
        json!({"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
    );
    let got = ranges(&mut s, uri);
    for (a, b, kind) in [
        (0, 12, None),           // package（end P; の行は残す）
        (3, 11, None),           // model M
        (5, 6, None),            // initial equation（次の節の手前まで）
        (7, 11, None),           // equation（末尾が annotation の行）
        (10, 11, None),          // annotation
        (1, 2, Some("comment")), // 行コメントの連続
    ] {
        assert!(
            got.contains(&(a, b, kind.map(str::to_string))),
            "{a}-{b} not in {got:?}"
        );
    }

    // 構文が壊れてアウトラインが引けなくても、コメントは出る。1 行のものは出ない
    // （S276 から、クラスの範囲も CST から出る。`end` が無いので最後の字句の行の手前まで）
    s.handle(
        json!({"method": "textDocument/didChange", "params": {"textDocument": {"uri": uri},
        "contentChanges": [{"text": "model A\n// a\n// b\nx /* one */\n"}]}}),
    );
    let got = ranges(&mut s, uri);
    assert!(got.contains(&(1, 2, Some("comment".into()))), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}"); // コメントとクラス
    assert_eq!(s.handle(json!({"id": 3, "method": "textDocument/foldingRange", "params": {"textDocument": {"uri": "file:///nope.mo"}}}))[0]["result"], Value::Null);
}

/// S276: 構文の誤り（`Real x` の `;` 抜け）があっても、クラスと equation 節の折りたたみが出る
#[test]
fn broken_syntax_still_folds_classes_and_sections() {
    let mut s = Server::new(&[]).unwrap();
    let uri = "file:///tmp/Broken.mo";
    let text = include_str!("../../globe-mode-frontend/tests/data/SyntaxError.mo");
    s.handle(
        json!({"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
    );
    // 0: // コメント, 1: model Broken, 2: Real x, 3: equation, 4: x = 1;, 5: end Broken;
    let got = ranges(&mut s, uri);
    for (a, b) in [(1, 4), (3, 4)] {
        assert!(got.contains(&(a, b, None)), "{a}-{b} not in {got:?}");
    }
}

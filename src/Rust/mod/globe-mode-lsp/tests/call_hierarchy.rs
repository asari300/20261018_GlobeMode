//! call hierarchy（S354）: 関数だけ。呼び元は開いた文書とライブラリ（`-L`）の開いていないファイル、呼び先は組み込み・引けない名前を除く。

use globe_mode_lsp::Server;
use serde_json::{Value, json};

#[test]
fn prepare_incoming_and_outgoing_calls_of_functions() {
    let dir = std::env::temp_dir().join(format!("gm-lsp-callh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fns = "package Fns\n  function f\n    input Real a;\n    output Real y = a;\n  end f;\n\
  function g\n    input Real x;\n    output Real y = f(x) + f(2 * x) + sin(x) + nope(x);\n  protected\n    R r = R(x);\n  end g;\n  record R\n    Real a;\n  end R;\nend Fns;\n";
    std::fs::write(dir.join("Fns.mo"), fns).unwrap();
    std::fs::write(
        dir.join("LibUser.mo"),
        "model LibUser\n  Real x = Fns.g(1);\nend LibUser;\n",
    )
    .unwrap();
    let m = "model M\n  Real x = Fns.f(1) + Fns.g(2);\n  Real y = Fns.f(3);\nend M;\n";
    let mut s = Server::new(std::slice::from_ref(&dir)).unwrap();
    let init = s.handle(json!({"id": 0, "method": "initialize", "params": {}}));
    assert_eq!(
        init[0]["result"]["capabilities"]["callHierarchyProvider"],
        true
    );
    s.handle(
        json!({"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": "file:///tmp/M.mo", "languageId": "modelica", "version": 1, "text": m}}}),
    );
    let mut ask = |method: &str, params: Value| {
        s.handle(json!({"id": 1, "method": method, "params": params}))
            .remove(0)["result"]
            .clone()
    };
    let at = |needle: &str, k: usize| {
        let i = m.find(needle).unwrap() + k;
        let line = m[..i].matches('\n').count();
        json!({"line": line, "character": i - m[..i].rfind('\n').map_or(0, |n| n + 1)})
    };
    let prepare = |ask: &mut dyn FnMut(&str, Value) -> Value, needle: &str, k: usize| {
        ask(
            "textDocument/prepareCallHierarchy",
            json!({"textDocument": {"uri": "file:///tmp/M.mo"}, "position": at(needle, k)}),
        )
    };

    // prepare: 呼び出しの `f`（別のファイルの関数）。モデルの名前・組み込みの上は null
    let f = prepare(&mut ask, "Fns.f(1)", 5)[0].clone();
    assert_eq!(f["name"], "f");
    assert!(f["uri"].as_str().unwrap().ends_with("/Fns.mo"), "{f}");
    assert_eq!(
        f["selectionRange"]["start"],
        json!({"line": 1, "character": 11})
    );
    assert_eq!(prepare(&mut ask, "model M", 7), Value::Null);
    let g = prepare(&mut ask, "Fns.g(2)", 5)[0].clone();

    // incoming(f): g（本文の 2 回）と M（2 回）。宣言と `end f` は呼び出しでない。LibUser は g だけを呼ぶ
    let names = |v: &Value, key: &str| -> Vec<(String, usize)> {
        let mut o: Vec<_> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c[key]["name"].as_str().unwrap().to_string(),
                    c["fromRanges"].as_array().unwrap().len(),
                )
            })
            .collect();
        o.sort();
        o
    };
    let inc = ask("callHierarchy/incomingCalls", json!({"item": f}));
    assert_eq!(
        names(&inc, "from"),
        [("M".into(), 2), ("g".into(), 2)],
        "{inc}"
    );
    let g_call = inc
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["from"]["name"] == "g")
        .unwrap();
    assert_eq!(g_call["from"]["kind"], 12);
    assert_eq!(
        g_call["fromRanges"][0]["start"],
        json!({"line": 7, "character": 20})
    );
    // incoming(g): 開いた文書 M と、開いていない LibUser
    let inc = ask("callHierarchy/incomingCalls", json!({"item": g}));
    assert_eq!(
        names(&inc, "from"),
        [("LibUser".into(), 1), ("M".into(), 1)],
        "{inc}"
    );

    // outgoing(g): f（2 回）だけ。`sin`・引けない `nope`・レコードの構築子 `R(x)` は出ない。f は何も呼ばない
    let out = ask("callHierarchy/outgoingCalls", json!({"item": g}));
    assert_eq!(names(&out, "to"), [("f".into(), 2)], "{out}");
    assert_eq!(out[0]["to"]["selectionRange"], f["selectionRange"]);
    assert_eq!(
        ask("callHierarchy/outgoingCalls", json!({"item": f})),
        json!([])
    );
    // 関数でない項目（モデル M）は null
    let model = json!({"name": "M", "kind": 5, "uri": "file:///tmp/M.mo", "range": at("model M", 0),
        "selectionRange": {"start": at("model M", 6), "end": at("model M", 7)}});
    assert_eq!(
        ask("callHierarchy/incomingCalls", json!({"item": model})),
        Value::Null
    );
    assert_eq!(
        ask("callHierarchy/outgoingCalls", json!({"item": model})),
        Value::Null
    );
    let _ = std::fs::remove_dir_all(&dir);
}

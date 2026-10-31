//! API の検査。実際にサーバを立てて、整形と lint を HTTP で 1 往復ずつ確かめる（クライアントは std の TcpStream だけ）。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

use serde_json::{Value, json};

const BEFORE: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_before.mo");
const AFTER: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_after.mo");

/// HTTP/1.1 で 1 往復し、（状態, JSON の本文）を返す。`Connection: close` なので読み切れば終わる
fn call(addr: SocketAddr, method: &str, path: &str, body: &str) -> (u16, Value) {
    let mut s = TcpStream::connect(addr).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    let status = raw.split_whitespace().nth(1).unwrap().parse().unwrap();
    let (_, json) = raw.split_once("\r\n\r\n").unwrap();
    (status, serde_json::from_str(json).unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn format_and_lint_over_http() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, web_axum::app()).await.unwrap() });

    let (status, v) = call(addr, "GET", "/healthz", "");
    assert_eq!((status, v), (200, json!({"ok": true})));

    // 整形: 例題の Before が After になる。成功は ok = true、error = null
    let (status, v) = call(
        addr,
        "POST",
        "/api/format",
        &json!({"text": BEFORE}).to_string(),
    );
    assert_eq!(status, 200);
    assert_eq!(v["ok"], true);
    assert_eq!(v["text"], AFTER);
    assert!(v["error"].is_null());

    // 文法が壊れた入力と、範囲外のインデント: HTTP は 200 のまま、ok = false、text = null
    for body in [
        json!({"text": "model A\n"}),
        json!({"text": "model A\nend A;", "indent": 99}),
    ] {
        let (status, v) = call(addr, "POST", "/api/format", &body.to_string());
        assert_eq!(status, 200);
        assert_eq!(v["ok"], false);
        assert!(v["text"].is_null());
        assert!(v["error"].is_string());
    }

    // lint: Before には missing-description が 1 件。disable で止められる
    let (status, v) = call(
        addr,
        "POST",
        "/api/lint",
        &json!({"text": BEFORE}).to_string(),
    );
    assert_eq!(status, 200);
    let messages = v["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["rule"], "missing-description");
    assert_eq!(messages[0]["line"], 2);
    let (_, v) = call(
        addr,
        "POST",
        "/api/lint",
        &json!({"text": BEFORE, "disable": ["missing-description"]}).to_string(),
    );
    assert_eq!(v["messages"], json!([]));
}

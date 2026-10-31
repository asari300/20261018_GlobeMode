//! API の検査。`rocket::local::blocking::Client` で、サーバを立てずに経路を 1 本ずつ呼ぶ。

use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::serde::json::{Value, json};

const BEFORE: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_before.mo");
const AFTER: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_after.mo");

fn client() -> Client {
    Client::tracked(web_rocket::app()).expect("valid rocket instance")
}

fn post_json(client: &Client, path: &str, body: Value) -> (Status, Value) {
    let res = client
        .post(path)
        .header(ContentType::JSON)
        .body(body.to_string())
        .dispatch();
    let status = res.status();
    (status, res.into_json::<Value>().expect("JSON body"))
}

#[test]
fn index_serves_the_page() {
    let client = client();
    let res = client.get("/").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert!(res.into_string().unwrap().contains("整形"));
}

#[test]
fn healthz_reports_ok() {
    let client = client();
    let res = client.get("/healthz").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_json::<Value>().unwrap(), json!({"ok": true}));
}

#[test]
fn format_example_succeeds() {
    let (status, v) = post_json(&client(), "/api/format", json!({"text": BEFORE}));
    assert_eq!(status, Status::Ok);
    assert_eq!(v["ok"], true);
    assert!(v["error"].is_null());
    let text = v["text"].as_str().unwrap();
    assert!(text.contains("origin = {0, 0}"));
    assert_eq!(text, AFTER);
}

#[test]
fn format_broken_input_is_ok_false_with_http_200() {
    let (status, v) = post_json(&client(), "/api/format", json!({"text": "model A\n"}));
    assert_eq!(status, Status::Ok);
    assert_eq!(v["ok"], false);
    assert!(v["text"].is_null());
    assert!(v["error"].is_string());
}

#[test]
fn format_rejects_out_of_range_indent() {
    let (status, v) = post_json(
        &client(),
        "/api/format",
        json!({"text": "model A\nend A;", "indent": 99}),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(v["ok"], false);
    assert!(v["text"].is_null());
}

#[test]
fn lint_lists_messages() {
    let (status, v) = post_json(&client(), "/api/lint", json!({"text": "model a\nend a;\n"}));
    assert_eq!(status, Status::Ok);
    let messages = v["messages"].as_array().unwrap();
    assert!(
        messages
            .iter()
            .any(|m| m["rule"] == "class-name-pascal-case" && m["line"] == 1),
        "{messages:?}"
    );
    for m in messages {
        assert!(m["column"].is_u64() && m["message"].is_string());
    }
}

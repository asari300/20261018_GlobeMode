//! API の検査。`actix_web::test` で実際のルーティングを通し、整形・lint・healthz を確かめる。

use actix_web::{App, http::StatusCode, test};
use serde_json::{Value, json};
use web_actix::configure;

const BEFORE: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_before.mo");

#[actix_web::test]
async fn healthz_responds_ok() {
    let app = test::init_service(App::new().configure(configure)).await;
    let res = test::call_service(&app, test::TestRequest::get().uri("/healthz").to_request()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = test::read_body_json(res).await;
    assert_eq!(v, json!({"ok": true}));
}

#[actix_web::test]
async fn format_accepts_example_and_rejects_broken_input() {
    let app = test::init_service(App::new().configure(configure)).await;

    // 例題は整形でき、Rectangle に origin = {0, 0} が足される
    let req = test::TestRequest::post()
        .uri("/api/format")
        .set_json(json!({"text": BEFORE}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = test::read_body_json(res).await;
    assert_eq!(v["ok"], true);
    assert!(v["text"].as_str().unwrap().contains("origin = {0, 0}"));
    assert!(v["error"].is_null());

    // 文法が壊れた入力は HTTP 200 のまま、ok = false、text = null
    let req = test::TestRequest::post()
        .uri("/api/format")
        .set_json(json!({"text": "model A\n"}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = test::read_body_json(res).await;
    assert_eq!(v["ok"], false);
    assert!(v["text"].is_null());
    assert!(v["error"].is_string());
}

#[actix_web::test]
async fn lint_reports_and_disables_rules() {
    let app = test::init_service(App::new().configure(configure)).await;

    // Before には missing-description が 1 件
    let req = test::TestRequest::post()
        .uri("/api/lint")
        .set_json(json!({"text": BEFORE}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = test::read_body_json(res).await;
    let messages = v["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["rule"], "missing-description");
    assert_eq!(messages[0]["line"], 2);

    // disable で止められる
    let req = test::TestRequest::post()
        .uri("/api/lint")
        .set_json(json!({"text": BEFORE, "disable": ["missing-description"]}))
        .to_request();
    let res = test::call_service(&app, req).await;
    let v: Value = test::read_body_json(res).await;
    assert_eq!(v["messages"], json!([]));
}

//! API を HTTP で 1 往復ずつ確かめる（loco_rs::testing::request。DB は使わない）。

use loco_rs::testing::request::request;
use serde_json::{Value, json};
use web_loco::App;

const SAMPLE: &str = include_str!("../../../mod/globe-mode-fmt/tests/data/objects_before.mo");

#[tokio::test]
async fn healthz_returns_ok() {
    request::<App, _, _>(|server, _ctx| async move {
        let res = server.get("/healthz").await;
        res.assert_status_ok();
        assert_eq!(res.json::<Value>(), json!({"ok": true}));
    })
    .await;
}

#[tokio::test]
async fn format_sample_inserts_origin() {
    request::<App, _, _>(|server, _ctx| async move {
        let res = server
            .post("/api/format")
            .json(&json!({"text": SAMPLE}))
            .await;
        res.assert_status_ok();
        let body = res.json::<Value>();
        assert_eq!(body["ok"], json!(true));
        assert!(body["error"].is_null());
        assert!(body["text"].as_str().unwrap().contains("origin = {0, 0}"));
    })
    .await;
}

#[tokio::test]
async fn format_broken_source_reports_error() {
    request::<App, _, _>(|server, _ctx| async move {
        let res = server
            .post("/api/format")
            .json(&json!({"text": "model A\n"}))
            .await;
        res.assert_status_ok();
        let body = res.json::<Value>();
        assert_eq!(body["ok"], json!(false));
        assert!(body["text"].is_null());
        assert!(body["error"].as_str().is_some());
    })
    .await;
}

#[tokio::test]
async fn lint_reports_and_honours_disable() {
    // 小文字の class 名は class-name-pascal-case に当たる（整形後は 80 桁に収まるので line-too-long は出ない）
    const BAD: &str = "model badName\nend badName;\n";
    request::<App, _, _>(|server, _ctx| async move {
        let res = server.post("/api/lint").json(&json!({"text": BAD})).await;
        res.assert_status_ok();
        let body = res.json::<Value>();
        let messages = body["messages"].as_array().unwrap();
        let hit = messages
            .iter()
            .find(|m| m["rule"] == "class-name-pascal-case")
            .expect("class-name-pascal-case が出る");
        assert!(hit["line"].as_u64().unwrap() >= 1);
        assert!(hit["column"].as_u64().unwrap() >= 1);
        assert!(hit["message"].as_str().is_some());

        let res = server
            .post("/api/lint")
            .json(&json!({"text": BAD, "disable": ["class-name-pascal-case"]}))
            .await;
        let body = res.json::<Value>();
        assert!(
            body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .all(|m| m["rule"] != "class-name-pascal-case")
        );
    })
    .await;
}

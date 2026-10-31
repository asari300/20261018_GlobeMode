//! GlobeMode の Modelica 整形と lint の Web アプリ（axum）。画面と API の形は `doc/web-apps.md` の契約に従う。
//! 計算はすべて `globe-mode-fmt` にある。ここは JSON と HTML を運ぶだけ。

use axum::{
    Json, Router,
    response::Html,
    routing::{get, post},
};
use globe_mode_fmt::{LintOptions, Options};
use serde::{Deserialize, Serialize};

const PAGE: &str = include_str!("index.html");
/// インデントの上限。大きな値は出力を膨らませるだけなので、外からの入力は 16 までにする
const MAX_INDENT: usize = 16;

/// `POST /api/format` の要求。`text` 以外は省略できる（省略時は `Options::default()`）
#[derive(Deserialize)]
pub struct FormatRequest {
    text: String,
    indent: Option<usize>,
    width: Option<usize>,
    insert_origin: Option<bool>,
}

/// `POST /api/lint` の要求。`disable` は省略できる
#[derive(Deserialize)]
pub struct LintRequest {
    text: String,
    #[serde(default)]
    disable: Vec<String>,
}

#[derive(Serialize)]
struct FormatResponse {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct LintMessage {
    rule: String,
    line: usize,
    column: usize,
    message: String,
}

#[derive(Serialize)]
struct LintResponse {
    messages: Vec<LintMessage>,
}

/// アプリ全体の経路。テストからもこれを呼ぶ。
pub fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/api/format", post(format))
        .route("/api/lint", post(lint))
}

async fn index() -> Html<&'static str> {
    Html(PAGE)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

// shortcut: 整形と lint は要求の task の上で同期に行う（小さい入力向け。入力が大きくて応答が遅れたら spawn_blocking に移す）
async fn format(Json(req): Json<FormatRequest>) -> Json<FormatResponse> {
    let defaults = Options::default();
    let indent = req.indent.unwrap_or(defaults.indent);
    let res = if indent > MAX_INDENT {
        Err(format!("indent は 0〜{MAX_INDENT} の整数で指定する"))
    } else {
        let options = Options {
            indent,
            width: req.width.unwrap_or(defaults.width),
            insert_origin: req.insert_origin.unwrap_or(defaults.insert_origin),
        };
        globe_mode_fmt::format(&req.text, &options).map_err(|e| e.to_string())
    };
    Json(match res {
        Ok(text) => FormatResponse {
            ok: true,
            text: Some(text),
            error: None,
        },
        Err(error) => FormatResponse {
            ok: false,
            text: None,
            error: Some(error),
        },
    })
}

async fn lint(Json(req): Json<LintRequest>) -> Json<LintResponse> {
    let options = LintOptions {
        disabled: req.disable,
        ..LintOptions::default()
    };
    let messages = globe_mode_fmt::lint(&req.text, &options)
        .into_iter()
        .map(|m| LintMessage {
            rule: m.rule.to_string(),
            line: m.line,
            column: m.column,
            message: m.message,
        })
        .collect();
    Json(LintResponse { messages })
}

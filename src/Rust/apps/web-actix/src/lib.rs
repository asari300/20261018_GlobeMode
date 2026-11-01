//! GlobeMode の Modelica 整形と lint の Web アプリ（actix-web）。画面と API の形は `doc/web-apps.md` の契約に従う。
//! 計算はすべて `globe-mode-fmt` にある。ここは JSON と HTML を運ぶだけ。

use actix_web::{HttpResponse, web};
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

/// アプリの経路。テストと本番の両方から `App::new().configure(configure)` で使う。
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(index))
        .route("/healthz", web::get().to(healthz))
        .route("/api/format", web::post().to(format_api))
        .route("/api/lint", web::post().to(lint_api));
}

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(PAGE)
}

async fn healthz() -> web::Json<serde_json::Value> {
    web::Json(serde_json::json!({"ok": true}))
}

// shortcut: 整形と lint は要求の task の上で同期に行う（小さい入力向け。入力が大きくて応答が遅れたら web::block に移す）
async fn format_api(req: web::Json<FormatRequest>) -> web::Json<FormatResponse> {
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
    web::Json(match res {
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

async fn lint_api(req: web::Json<LintRequest>) -> web::Json<LintResponse> {
    let options = LintOptions {
        disabled: req.disable.clone(),
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
    web::Json(LintResponse { messages })
}

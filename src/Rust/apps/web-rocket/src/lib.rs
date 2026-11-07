//! GlobeMode の Modelica の整形と lint の Web アプリ（Rocket）。画面と API の形は `doc/web-apps.md` の契約に従う。
//! 計算はすべて `globe-mode-fmt` にある。ここは JSON と HTML を運ぶだけ。

use globe_mode_fmt::{LintOptions, Options};
use rocket::config::Config;
use rocket::response::content::RawHtml;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, get, post, routes};
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
pub struct FormatResponse {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct LintMessage {
    rule: String,
    line: usize,
    column: usize,
    message: String,
}

#[derive(Serialize)]
pub struct LintResponse {
    messages: Vec<LintMessage>,
}

#[derive(Serialize)]
pub struct Health {
    ok: bool,
}

/// アプリ本体。テストからもこれを呼ぶ。
/// `address` は既定で 127.0.0.1（外から見えないように）。`ROCKET_ADDRESS` で上書きできる。
/// 待ち受けの port は Rocket の `ROCKET_PORT`（既定 8000）。
pub fn app() -> Rocket<Build> {
    rocket::custom(Config::figment().join(("address", "127.0.0.1")))
        .mount("/", routes![index, healthz, api_format, api_lint])
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(PAGE)
}

#[get("/healthz")]
fn healthz() -> Json<Health> {
    Json(Health { ok: true })
}

#[post("/api/format", data = "<req>")]
fn api_format(req: Json<FormatRequest>) -> Json<FormatResponse> {
    let req = req.into_inner();
    let defaults = Options::default();
    let indent = req.indent.unwrap_or(defaults.indent);
    let res = if indent > MAX_INDENT {
        Err(format!("indent は 0〜{MAX_INDENT} の整数で指定する"))
    } else {
        let options = Options {
            indent,
            width: req.width.unwrap_or(defaults.width),
            insert_origin: req.insert_origin.unwrap_or(defaults.insert_origin),
            ..defaults
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

#[post("/api/lint", data = "<req>")]
fn api_lint(req: Json<LintRequest>) -> Json<LintResponse> {
    let req = req.into_inner();
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

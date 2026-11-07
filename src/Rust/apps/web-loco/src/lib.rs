//! GlobeMode の Modelica 整形と lint を HTTP で使えるようにした loco のアプリ（S053）。
//! 計算は crate `globe-mode-fmt` にあり、ここは契約（doc/web-apps.md）どおりに JSON を組むだけ。

use async_trait::async_trait;
use axum::{
    Json,
    response::Html,
    routing::{get, post},
};
use globe_mode_fmt::{LintOptions, Options};
use loco_rs::{
    Result,
    app::{AppContext, Hooks},
    bgworker::Queue,
    boot::{BootResult, StartMode},
    config::Config,
    controller::{AppRoutes, Routes},
    environment::Environment,
    task::Tasks,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const INDEX_HTML: &str = include_str!("index.html");

/// インデントの上限。大きすぎる値で長い空白を作らせない（入力の信頼境界）。
const MAX_INDENT: usize = 16;

pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        loco_rs::boot::create_app::<Self>(mode, environment, config).await
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::empty().add_route(
            Routes::new()
                .add("/", get(index))
                .add("/healthz", get(healthz))
                .add("/api/format", post(format))
                .add("/api/lint", post(lint)),
        )
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn healthz() -> Json<Value> {
    Json(json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct FormatRequest {
    text: String,
    indent: Option<usize>,
    width: Option<usize>,
    insert_origin: Option<bool>,
}

#[derive(Serialize)]
pub struct FormatReply {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
}

/// 整形の失敗（文法が壊れている等）は HTTP 200 で `ok: false` を返す。
async fn format(Json(req): Json<FormatRequest>) -> Json<FormatReply> {
    let defaults = Options::default();
    let indent = req.indent.unwrap_or(defaults.indent);
    if indent > MAX_INDENT {
        return Json(FormatReply {
            ok: false,
            text: None,
            error: Some(format!("indent は 0〜{MAX_INDENT} の整数にする")),
        });
    }
    let options = Options {
        indent,
        width: req.width.unwrap_or(defaults.width),
        insert_origin: req.insert_origin.unwrap_or(defaults.insert_origin),
        ..defaults
    };
    Json(match globe_mode_fmt::format(&req.text, &options) {
        Ok(text) => FormatReply {
            ok: true,
            text: Some(text),
            error: None,
        },
        Err(e) => FormatReply {
            ok: false,
            text: None,
            error: Some(e.to_string()),
        },
    })
}

#[derive(Deserialize)]
pub struct LintRequest {
    text: String,
    #[serde(default)]
    disable: Vec<String>,
}

#[derive(Serialize)]
pub struct LintItem {
    rule: &'static str,
    line: usize,
    column: usize,
    message: String,
}

#[derive(Serialize)]
pub struct LintReply {
    messages: Vec<LintItem>,
}

async fn lint(Json(req): Json<LintRequest>) -> Json<LintReply> {
    let options = LintOptions {
        disabled: req.disable,
        ..LintOptions::default()
    };
    let messages = globe_mode_fmt::lint(&req.text, &options)
        .into_iter()
        .map(|m| LintItem {
            rule: m.rule,
            line: m.line,
            column: m.column,
            message: m.message,
        })
        .collect();
    Json(LintReply { messages })
}

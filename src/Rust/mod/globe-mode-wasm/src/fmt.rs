//! 整形と lint（S053、`globe-mode-fmt`）。Workspace を要らないので自由関数。

use wasm_bindgen::prelude::*;

/// 本文を整形する。文法が壊れていれば `Err`（文字列）で、本文は変えない。
#[wasm_bindgen]
pub fn format(text: &str, indent: usize, insert_origin: bool) -> Result<String, JsValue> {
    let options = globe_mode_fmt::Options {
        indent,
        insert_origin,
        ..globe_mode_fmt::Options::default()
    };
    globe_mode_fmt::format(text, &options).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// lint の結果（`[{rule, line, column, message}]` の JSON）。
#[wasm_bindgen]
pub fn lint(text: &str) -> String {
    let msgs: Vec<serde_json::Value> = globe_mode_fmt::lint(text, &globe_mode_fmt::LintOptions::default())
        .into_iter()
        .map(|m| serde_json::json!({"rule": m.rule, "line": m.line, "column": m.column, "message": m.message}))
        .collect();
    serde_json::to_string(&msgs).unwrap_or_else(|_| "[]".to_string())
}

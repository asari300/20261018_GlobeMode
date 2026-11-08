//! UI 向けの説明パネル（S232。wasm の `documentation` と `globe/documentation` が使う）。クラスの
//! `annotation(Documentation(info, revisions))` の生の HTML を返す。HTML は洗わない（表示する側が信用せずに洗う）。

use lsp_types::Position;
use serde_json::{Value, json};

/// `class`（完全修飾名）の Documentation を `{class, info, revisions}` で返す。`class` が空なら `at = (本文, 行, 列)`
/// （列は UTF-16）にあるいちばん内側のクラス。まず `at` の本文から、無ければ `library(class)`（そのクラスを定義するライブラリの
/// ファイルの本文）から引く。クラスが引けなければ null。
pub fn documentation(
    class: &str,
    at: Option<(&str, u32, u32)>,
    library: impl Fn(&str) -> Option<String>,
) -> Value {
    let class = match (class, at) {
        ("", Some((text, line, character))) => {
            globe_mode_fmt::class_at(text, crate::offset(text, Position::new(line, character)))
        }
        ("", None) => None,
        (c, _) => Some(c.to_string()),
    };
    let Some(class) = class else {
        return Value::Null;
    };
    let found = at
        .and_then(|(text, ..)| globe_mode_fmt::documentation(text, &class))
        .or_else(|| library(&class).and_then(|s| globe_mode_fmt::documentation(&s, &class)));
    match found {
        Some(d) => json!({"class": class, "info": d.info, "revisions": d.revisions}),
        None => Value::Null,
    }
}

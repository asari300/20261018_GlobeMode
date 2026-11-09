//! UI 向けのアイコン（S244。wasm の `icon` と `globe/icon` が使う）。クラスの `annotation(Icon(…))` を形のデータ（JSON）で返す。
//! 描く（SVG の要素にする）のは UI。`documentation` と同じ探し方（文書のクラスを先に、無ければライブラリのクラス）。

use serde_json::{Value, json};

/// `class` のアイコンを `{class, extent, shapes}` で返す（`class` が空なら `at` のカーソルのクラス）。引けなければ null。
/// `library(名前)` はそのクラスを定義するライブラリのファイルの本文（`extends` 先も引く）。
pub fn icon(
    class: &str,
    at: Option<(&str, u32, u32)>,
    library: &dyn Fn(&str) -> Option<String>,
) -> Value {
    let Some(class) = crate::documentation::target(class, at) else {
        return Value::Null;
    };
    let found = at
        .and_then(|(text, ..)| globe_mode_fmt::icon(text, &class, library))
        .or_else(|| library(&class).and_then(|s| globe_mode_fmt::icon(&s, &class, library)));
    match found {
        Some(i) => json!({
            "class": i.class,
            "extent": i.extent,
            "shapes": i.shapes.iter().map(shape).collect::<Vec<_>>(),
        }),
        None => Value::Null,
    }
}

pub(crate) fn shape(s: &globe_mode_fmt::Shape) -> Value {
    json!({
        "kind": s.kind, "origin": s.origin, "rotation": s.rotation, "extent": s.extent, "points": s.points,
        "line": s.line, "fill": s.fill, "thickness": s.thickness, "dash": s.dash, "radius": s.radius,
        "text": s.text, "fontSize": s.font_size, "align": s.align, "arrows": s.arrows, "arrowSize": s.arrow_size,
    })
}

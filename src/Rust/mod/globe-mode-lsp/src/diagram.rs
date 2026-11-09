//! UI 向けの図（S245。wasm の `diagram` と `globe/diagram` が使う）。クラスの部品の配置と `connect` を形のデータ（JSON）で返す。
//! 描く（SVG の要素にする）のは UI。`icon` と同じ探し方（文書のクラスを先に、無ければライブラリのクラス）。

use serde_json::{Value, json};

/// `class` の図を `{class, extent, parts, connections, shapes}` で返す（`class` が空なら `at` のカーソルのクラス）。引けなければ null。
pub fn diagram(
    class: &str,
    at: Option<(&str, u32, u32)>,
    library: &dyn Fn(&str) -> Option<String>,
) -> Value {
    let Some(class) = crate::documentation::target(class, at) else {
        return Value::Null;
    };
    let found = at
        .and_then(|(text, ..)| globe_mode_fmt::diagram(text, &class, library))
        .or_else(|| library(&class).and_then(|s| globe_mode_fmt::diagram(&s, &class, library)));
    let shapes = |l: &[globe_mode_fmt::Shape]| l.iter().map(crate::icon::shape).collect::<Vec<_>>();
    match found {
        Some(d) => json!({
            "class": d.class,
            "extent": d.extent,
            "parts": d.parts.iter().map(|p| json!({
                "name": p.name, "class": p.class, "line": p.line, "transform": p.transform, "bbox": p.bbox, "shapes": shapes(&p.shapes),
            })).collect::<Vec<_>>(),
            "connections": d.connections.iter().map(|c| json!({"from": c.from, "to": c.to, "line": crate::icon::shape(&c.line)})).collect::<Vec<_>>(),
            "shapes": shapes(&d.shapes),
        }),
        None => Value::Null,
    }
}

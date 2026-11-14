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
                "connectors": p.connectors.iter().map(|(n, pos)| json!({"name": n, "pos": pos})).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "connections": d.connections.iter().map(|c| json!({"from": c.from, "to": c.to, "line": crate::icon::shape(&c.line)})).collect::<Vec<_>>(),
            "shapes": shapes(&d.shapes),
        }),
        None => Value::Null,
    }
}

/// S250: `class` の部品 `part` を図の上で (dx, dy) だけ動かす編集（LSP の TextEdit の配列 `[{range, newText}]`）。
/// 動かせない（クラス・部品・Placement が無い・`origin` が数でない・`at` の本文が無い）ときは null。ファイルは書き換えない。
pub fn move_part(class: &str, at: Option<(&str, u32, u32)>, part: &str, dx: f64, dy: f64) -> Value {
    let Some(class) = crate::documentation::target(class, at) else {
        return Value::Null;
    };
    let Some((text, ..)) = at else {
        return Value::Null;
    };
    match globe_mode_fmt::move_part(text, &class, part, dx, dy) {
        Some((a, b, new_text)) => {
            let r = |o| crate::position(text, o as u32);
            json!([{"range": {"start": r(a), "end": r(b)}, "newText": new_text}])
        }
        None => Value::Null,
    }
}

/// S261: `class` に `connect(from, to)`（`points` の線つき）を足す編集（`move_part` と同じ TextEdit の配列）。足せなければ（クラスが無い・端が不正）null。ファイルは書き換えない。
pub fn add_connect(
    class: &str,
    at: Option<(&str, u32, u32)>,
    from: &str,
    to: &str,
    points: &[[f64; 2]],
) -> Value {
    let Some(class) = crate::documentation::target(class, at) else {
        return Value::Null;
    };
    let Some((text, ..)) = at else {
        return Value::Null;
    };
    match globe_mode_fmt::add_connect(text, &class, from, to, points) {
        Some((a, b, new_text)) => {
            let r = |o| crate::position(text, o as u32);
            json!([{"range": {"start": r(a), "end": r(b)}, "newText": new_text}])
        }
        None => Value::Null,
    }
}

/// S265: `class` に部品 `ty` の宣言（`origin` は図の座標を 2 単位に吸着）を足す編集（`move_part` と同じ TextEdit の配列）。
/// `ty` が package・function・record・type と分かる・型が名前でない・クラスが無ければ null。ファイルは書き換えない。
pub fn add_part(
    class: &str,
    at: Option<(&str, u32, u32)>,
    ty: &str,
    origin: [f64; 2],
    library: &dyn Fn(&str) -> Option<String>,
) -> Value {
    let Some(class) = crate::documentation::target(class, at) else {
        return Value::Null;
    };
    let Some((text, ..)) = at else {
        return Value::Null;
    };
    match globe_mode_fmt::add_part(text, &class, ty, origin, library) {
        Some((a, b, new_text)) => {
            let r = |o| crate::position(text, o as u32);
            json!([{"range": {"start": r(a), "end": r(b)}, "newText": new_text}])
        }
        None => Value::Null,
    }
}

/// `[[x, y], …]` の JSON を点の列にする（読めない要素は捨てる）。
pub fn points(v: &Value) -> Vec<[f64; 2]> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
                .collect()
        })
        .unwrap_or_default()
}

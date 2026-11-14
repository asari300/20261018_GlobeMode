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

/// S294: `class` の部品の修飾を書く編集（TextEdit の配列。1 つ）。`modifiers` は `[{part, name, value}]`（`value` が空なら外す）。
/// 順に `globe_mode_fmt::set_modifier` で本文に当てていき、最初と最後の本文の差（共通の頭と尻を除いた範囲）を 1 つの編集にする（同じ部品への複数でも壊れない）。
/// 断る理由（クラス・部品が無い・入れ子の名前・`each`/`final`・壊れた式）は Err。何も変わらなければ空の配列。ファイルは書き換えない。
pub fn set_modifiers(
    class: &str,
    at: Option<(&str, u32, u32)>,
    modifiers: &Value,
) -> Result<Value, String> {
    let (Some(class), Some((text, ..))) = (crate::documentation::target(class, at), at) else {
        return Err("クラスか本文が無い".into());
    };
    let list = modifiers
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("modifiers が空か配列でない")?;
    let mut cur = text.to_string();
    for m in list {
        let f = |k: &str| {
            m[k].as_str()
                .ok_or(format!("modifiers の {k} が文字列でない"))
        };
        let (a, b, t) =
            globe_mode_fmt::set_modifier(&cur, &class, f("part")?, f("name")?, f("value")?)?;
        cur.replace_range(a..b, &t);
    }
    let head = text
        .char_indices()
        .zip(cur.chars())
        .find(|((_, a), b)| a != b)
        .map_or(text.len().min(cur.len()), |((i, _), _)| i);
    let (mut a, mut b) = (text.len(), cur.len());
    while a > head && b > head && text.as_bytes()[a - 1] == cur.as_bytes()[b - 1] {
        a -= 1;
        b -= 1;
    }
    while !(text.is_char_boundary(a) && cur.is_char_boundary(b)) {
        a += 1;
        b += 1;
    }
    if text == cur {
        return Ok(json!([]));
    }
    let r = |o| crate::position(text, o as u32);
    Ok(json!([{"range": {"start": r(head), "end": r(a)}, "newText": &cur[head..b]}]))
}

/// S281: `class` の部品 `part` の宣言とその部品につながる `connect` を消す編集（TextEdit の配列。複数。重ならない）。
/// 消せない（クラス・部品・Placement が無い・宣言が複数の名前を持つ）ときは null。ファイルは書き換えない。
pub fn delete_part(class: &str, at: Option<(&str, u32, u32)>, part: &str) -> Value {
    erase(class, at, |t, c| globe_mode_fmt::delete_part(t, c, part))
}

/// S281: `class` の `connect(a, b)`（逆の並びも同じ）を消す編集。無ければ null。
pub fn delete_connect(class: &str, at: Option<(&str, u32, u32)>, a: &str, b: &str) -> Value {
    erase(class, at, |t, c| globe_mode_fmt::delete_connect(t, c, a, b))
}

fn erase(
    class: &str,
    at: Option<(&str, u32, u32)>,
    f: impl Fn(&str, &str) -> Option<Vec<(usize, usize, String)>>,
) -> Value {
    let (Some(class), Some((text, ..))) = (crate::documentation::target(class, at), at) else {
        return Value::Null;
    };
    let r = |o| crate::position(text, o as u32);
    match f(text, &class) {
        Some(e) => Value::Array(
            e.into_iter()
                .map(|(a, b, t)| json!({"range": {"start": r(a), "end": r(b)}, "newText": t}))
                .collect(),
        ),
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

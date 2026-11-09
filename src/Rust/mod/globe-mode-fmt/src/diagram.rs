//! クラスの Diagram（部品の配置と `connect`）を形のデータにする（S245。読み取り専用）。
//! `icon.rs` と同じく字句解析（[`lex`]）・文の切り出し（[`split`]）・クラスの範囲（[`class_spans`]）だけで読み、評価も SVG の文字列もしない（描くのは UI）。
//! 部品は `annotation(Placement(transformation(…)))` を持つ宣言だけ。部品のクラスの Icon を引いて（`extends` は 1 段）変換の行列を付ける。
//! 読めないもの（型が引けない・Placement が壊れている・端が部品でない connect）は飛ばして続ける。`extends` で継ぐ部品・`import {A, B}` は読まない。

use crate::documentation::{join, within};
use crate::icon::{Shape, Val, annotation_call, get, icon_named, num, nums, rect, shape};
use crate::lexer::{Kind, Token, lex};
use crate::structure::{class_spans, dotted, skip_group, split};

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    /// 宣言に書いてある型の名前
    pub class: String,
    /// 宣言の行（0 始まり）
    pub line: usize,
    /// 部品のアイコンの座標から Diagram の座標（どちらも Modelica のまま、y は上向き）への変換 `[a, b, c, d, e, f]`
    /// （`x' = a x + c y + e`, `y' = b x + d y + f`。SVG の `matrix(a b c d e f)` と同じ並び）
    pub transform: [f64; 6],
    /// 変換したアイコンの枠（Diagram の座標）
    pub bbox: [f64; 4],
    /// アイコンの図形（引けなければ空）
    pub shapes: Vec<Shape>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    /// `a.p` のように書いてある端
    pub from: String,
    pub to: String,
    /// `Line(points = …)` があればその線、無ければ 2 つの部品の中心を結ぶ直線
    pub line: Shape,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub class: String,
    /// `coordinateSystem(extent)`。無ければ部品・線が収まる枠（何も無ければ `[-100, -100, 100, 100]`）
    pub extent: [f64; 4],
    pub parts: Vec<Part>,
    pub connections: Vec<Connection>,
    /// `Diagram(graphics = {…})`
    pub shapes: Vec<Shape>,
}

const PREFIX: &[&str] = &[
    "flow",
    "stream",
    "input",
    "output",
    "parameter",
    "constant",
    "discrete",
    "inner",
    "outer",
    "final",
    "replaceable",
    "redeclare",
];

struct Raw {
    name: String,
    ty: String,
    line: usize,
    origin: [f64; 2],
    extent: Option<[f64; 4]>,
    rotation: f64,
}

/// `src` の `qualified` のクラスの Diagram。クラスが無ければ None（あって部品が無ければ空）。
/// `library(名前)` はそのクラスを定義するファイルの本文（部品の型を引く）。
pub fn diagram(
    src: &str,
    qualified: &str,
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<Diagram> {
    let body = src.strip_prefix('\u{feff}').unwrap_or(src);
    let tokens = lex(body).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let own = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    let local = |c: &str| spans.iter().any(|s| join(&within, &s.path) == c);

    let (mut raws, mut conns) = (Vec::new(), Vec::new());
    let (mut aliases, mut wild) = (Vec::<(String, String)>::new(), Vec::<String>::new());
    for e in split(&tokens, false).ok()? {
        let t = e.tokens;
        let st = t[0].start;
        if t[0].is_comment() || spans.iter().any(|s| s.start == st) {
            continue; // コメント・クラスの頭
        }
        let Some(owner) = spans
            .iter()
            .filter(|s| s.start <= st && st < s.end)
            .max_by_key(|s| s.start)
        else {
            continue;
        };
        let mine = owner.path == own.path;
        if !(mine || own.path.starts_with(&format!("{}.", owner.path))) {
            continue;
        }
        if t[0].is("import") {
            if t.get(2).is_some_and(|s| s.is("=")) {
                aliases.push((t[1].text.to_string(), dotted(t, 3)));
            } else {
                let d = dotted(t, 1);
                match d.strip_suffix('.') {
                    Some(w) => wild.push(w.to_string()),
                    None => aliases.push((d.rsplit('.').next().unwrap_or(&d).to_string(), d)),
                }
            }
        } else if !mine {
        } else if t[0].is("connect") {
            conns.push(t);
        } else if let Some(r) = declaration(t, body) {
            raws.push(r);
        }
    }

    let name_of = qualified.rsplit('.').next().unwrap_or(qualified);
    let mut parts = Vec::new();
    for r in raws {
        let icon = resolve(
            &r.ty, &r.name, qualified, &aliases, &wild, &local, src, library,
        );
        let (ie, shapes) = icon.map_or(([-100.0, -100.0, 100.0, 100.0], Vec::new()), |i| {
            (i.extent, i.shapes)
        });
        let transform = matrix(ie, r.extent.unwrap_or(ie), r.origin, r.rotation);
        let corners = [
            (ie[0], ie[1]),
            (ie[0], ie[3]),
            (ie[2], ie[1]),
            (ie[2], ie[3]),
        ]
        .map(|(x, y)| apply(&transform, [x, y]));
        parts.push(Part {
            name: r.name,
            class: r.ty,
            line: r.line,
            bbox: [
                corners.iter().fold(f64::MAX, |m, c| m.min(c[0])),
                corners.iter().fold(f64::MAX, |m, c| m.min(c[1])),
                corners.iter().fold(f64::MIN, |m, c| m.max(c[0])),
                corners.iter().fold(f64::MIN, |m, c| m.max(c[1])),
            ],
            transform,
            shapes,
        });
    }

    let centre = |part: &str| {
        let p = parts.iter().find(|p| p.name == part)?;
        Some([(p.bbox[0] + p.bbox[2]) / 2.0, (p.bbox[1] + p.bbox[3]) / 2.0])
    };
    let mut connections = Vec::new();
    for t in conns {
        let Some((from, to)) = connect_ends(t) else {
            continue;
        };
        let drawn = annotation_in(t, "Line").and_then(|v| shape(&v, "", ""));
        let line = drawn.or_else(|| {
            let (a, b) = (centre(first(&from))?, centre(first(&to))?);
            Some(Shape {
                kind: "line",
                points: vec![a, b],
                line: Some([0, 0, 0]),
                thickness: 0.25,
                dash: "solid",
                align: "center",
                arrows: ["none"; 2],
                arrow_size: 3.0,
                ..Shape::default()
            })
        });
        if let Some(line) = line {
            connections.push(Connection { from, to, line });
        }
    }

    let (mut extent, mut shapes) = (None, Vec::new());
    if let Some(Val::Call(_, a)) = own.annotation.and_then(|a| annotation_call(a, "Diagram")) {
        let cs = call(&a, "coordinateSystem");
        extent = cs.and_then(|cs| get(cs, "extent")).and_then(rect);
        if let Some(Val::List(items)) = get(&a, "graphics") {
            shapes = items
                .iter()
                .filter_map(|v| shape(v, name_of, qualified))
                .collect();
        }
    }
    let extent = extent.unwrap_or_else(|| {
        let mut pts: Vec<[f64; 2]> = parts
            .iter()
            .flat_map(|p| [[p.bbox[0], p.bbox[1]], [p.bbox[2], p.bbox[3]]])
            .collect();
        pts.extend(
            connections
                .iter()
                .flat_map(|c| c.line.points.iter().copied()),
        );
        if pts.is_empty() {
            return [-100.0, -100.0, 100.0, 100.0];
        }
        let f = |g: fn(f64, f64) -> f64, i, init| pts.iter().fold(init, |m, p| g(m, p[i]));
        let (x1, y1) = (f(f64::min, 0, f64::MAX), f(f64::min, 1, f64::MAX));
        let (x2, y2) = (f(f64::max, 0, f64::MIN), f(f64::max, 1, f64::MIN));
        let pad = ((x2 - x1).max(y2 - y1) * 0.1).max(1.0);
        [x1 - pad, y1 - pad, x2 + pad, y2 + pad]
    });
    Some(Diagram {
        class: qualified.to_string(),
        extent,
        parts,
        connections,
        shapes,
    })
}

/// 名前のない位置引数の `name(…)`（`Placement(transformation(…))` の `transformation(…)`）の引数。
fn call<'a>(args: &'a [(String, Val)], name: &str) -> Option<&'a Vec<(String, Val)>> {
    args.iter().find_map(|(_, v)| match v {
        Val::Call(n, a) if n == name => Some(a),
        _ => None,
    })
}

fn first(end: &str) -> &str {
    end.split(['.', '[']).next().unwrap_or(end)
}

/// 要素の `annotation(…)` の直下の `name(…)`。
fn annotation_in(t: &[Token<'_>], name: &str) -> Option<Val> {
    let p = t.iter().position(|s| s.is("annotation"))?;
    annotation_call(&t[p..], name)
}

/// `connect(a.p, b[1].n)` の 2 つの端（`a.p`・`b.n` ではなく書いてあるままの字句をつなぐ）。
fn connect_ends(t: &[Token<'_>]) -> Option<(String, String)> {
    if !t.get(1)?.is("(") {
        return None;
    }
    let mut ends = [String::new(), String::new()];
    let (mut i, mut k) = (2, 0);
    while let Some(s) = t.get(i) {
        if s.is(",") && k == 0 {
            k = 1;
        } else if s.is(")") {
            break;
        } else if s.is("[") {
            i = skip_group(t, i).ok()?;
            continue;
        } else if s.kind == Kind::Ident || s.is(".") {
            ends[k].push_str(s.text);
        } else {
            return None; // 式（`if` など）は読まない
        }
        i += 1;
    }
    (!ends[0].is_empty() && !ends[1].is_empty()).then(|| {
        let [a, b] = ends;
        (a, b)
    })
}

/// 部品の宣言（`型 名前 … annotation(Placement(transformation(…)))`）。Placement が無い・隠す・壊れていれば None。
fn declaration(t: &[Token<'_>], body: &str) -> Option<Raw> {
    let mut i = 0;
    while t
        .get(i)
        .is_some_and(|s| s.kind == Kind::Ident && PREFIX.contains(&s.text))
    {
        i += 1;
    }
    let head = t
        .get(i)
        .filter(|s| s.kind == Kind::Ident && !s.is_keyword())?;
    let mut ty = head.text.to_string();
    i += 1;
    while t.get(i).is_some_and(|d| d.is("."))
        && let Some(n) = t.get(i + 1).filter(|n| n.kind == Kind::Ident)
    {
        ty = format!("{ty}.{}", n.text);
        i += 2;
    }
    if t.get(i).is_some_and(|s| s.is("[")) {
        i = skip_group(t, i).ok()?;
    }
    let name = t
        .get(i)
        .filter(|n| n.kind == Kind::Ident && !n.is_keyword())?;
    let Val::Call(_, p) = annotation_in(t, "Placement")? else {
        return None;
    };
    if get(&p, "visible").is_some_and(|v| matches!(v, Val::Id(s) if s == "false")) {
        return None;
    }
    let tr = call(&p, "transformation")?;
    let pair = |k: &str| get(tr, k).and_then(nums).filter(|n| n.len() == 2);
    Some(Raw {
        name: name.text.to_string(),
        ty,
        line: body[..name.start].bytes().filter(|&b| b == b'\n').count(),
        origin: pair("origin").map_or([0.0; 2], |n| [n[0], n[1]]),
        // 無ければアイコンの枠のまま。あって読めなければ壊れた Placement
        extent: match get(tr, "extent") {
            None => None,
            Some(v) => Some(rect(v)?),
        },
        rotation: get(tr, "rotation").and_then(num).unwrap_or(0.0),
    })
}

/// 部品の型を完全修飾名にして Icon を引く。`import` の別名・`.*`・内側から外側へのスコープの順。
#[allow(clippy::too_many_arguments)]
fn resolve(
    ty: &str,
    name: &str,
    class: &str,
    aliases: &[(String, String)],
    wild: &[String],
    local: &dyn Fn(&str) -> bool,
    src: &str,
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<crate::icon::Icon> {
    let (head, rest) = ty.split_once('.').map_or((ty, ""), |(h, r)| (h, r));
    let tail = |base: &str| {
        if rest.is_empty() {
            base.to_string()
        } else {
            format!("{base}.{rest}")
        }
    };
    let mut cands: Vec<String> = aliases
        .iter()
        .filter(|(a, _)| a == head)
        .map(|(_, t)| tail(t))
        .collect();
    cands.extend(wild.iter().map(|w| format!("{w}.{ty}")));
    let mut scope = class;
    loop {
        cands.push(if scope.is_empty() {
            ty.to_string()
        } else {
            format!("{scope}.{ty}")
        });
        if scope.is_empty() {
            break;
        }
        scope = scope.rsplit_once('.').map_or("", |(p, _)| p);
    }
    cands.into_iter().find_map(|c| {
        (if local(&c) {
            icon_named(src, &c, library, Some(name))
        } else {
            None
        })
        .or_else(|| library(&c).and_then(|t| icon_named(&t, &c, library, Some(name))))
    })
}

/// アイコンの枠 `ie` を Placement の枠 `pe` に写し（`pe` の向きが逆なら反転）、`origin` を中心に `rotation`（度、反時計回り）回す。
fn matrix(ie: [f64; 4], pe: [f64; 4], origin: [f64; 2], rotation: f64) -> [f64; 6] {
    let ratio = |p: f64, q: f64, i: f64, j: f64| if j == i { 1.0 } else { (q - p) / (j - i) };
    let (sx, sy) = (
        ratio(pe[0], pe[2], ie[0], ie[2]),
        ratio(pe[1], pe[3], ie[1], ie[3]),
    );
    // アイコンの中心 → Placement の中心
    let ex = (pe[0] + pe[2]) / 2.0 - sx * (ie[0] + ie[2]) / 2.0;
    let ey = (pe[1] + pe[3]) / 2.0 - sy * (ie[1] + ie[3]) / 2.0;
    let (s, c) = rotation.to_radians().sin_cos();
    [
        c * sx,
        s * sx,
        -s * sy,
        c * sy,
        origin[0] + c * ex - s * ey,
        origin[1] + s * ex + c * ey,
    ]
    .map(|v| (v * 1e9).round() / 1e9 + 0.0) // cos 90° = 6e-17 などを落とす
}

fn apply(m: &[f64; 6], p: [f64; 2]) -> [f64; 2] {
    [
        m[0] * p[0] + m[2] * p[1] + m[4],
        m[1] * p[0] + m[3] * p[1] + m[5],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB: &str = r#"within P;
package Basic
  model Res
    extends Interfaces.Two;
    annotation(Icon(coordinateSystem(extent = {{-100, -100}, {100, 100}}), graphics = {Rectangle(extent = {{-70, -30}, {70, 30}}, fillColor = {255, 255, 255}, fillPattern = FillPattern.Solid), Text(extent = {{-150, 40}, {150, 80}}, textString = "%name")}));
  end Res;
end Basic;
"#;
    const MODEL: &str = r#"within P;
model Circ
  import P.Basic;
  parameter Real k = 2;
  Basic.Res r1(R = 1) annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}}, origin = {20, 0}, rotation = 90)));
  P.Basic.Res r2[3] annotation(Placement(transformation(extent = {{10, -10}, {-10, 10}})));
  Nope.Gone g annotation(Placement(transformation(extent = {{0, 0}, {10, 10}})));
  Basic.Res hidden annotation(Placement(visible = false, transformation(extent = {{0, 0}, {10, 10}})));
  Basic.Res bad annotation(Placement(transformation(extent = {{0, 0}, 5}, rotation = 1)));
  Basic.Res plain;
equation
  connect(r1.n, r2[1].p) annotation(Line(points = {{20, 10}, {20, 30}, {-5, 30}}, color = {0, 0, 255}));
  connect(r1.p, g.p);
  connect(r1.p, ghost.p);
  annotation(Diagram(coordinateSystem(extent = {{-50, -50}, {50, 50}}), graphics = {Text(extent = {{-40, 40}, {40, 50}}, textString = "hi")}));
end Circ;
"#;

    fn lib(c: &str) -> Option<String> {
        (c == "P.Basic.Res").then(|| LIB.to_string())
    }

    #[test]
    fn parts_connections_and_graphics_are_read_and_the_broken_ones_skipped() {
        let d = diagram(MODEL, "P.Circ", &lib).unwrap();
        let names: Vec<_> = d.parts.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["r1", "r2", "g"],
            "hidden・壊れた Placement・Placement 無しは飛ばす"
        );
        let r1 = &d.parts[0];
        assert_eq!(r1.shapes.len(), 2, "import の別名で型が引ける");
        assert_eq!(r1.shapes[1].text, "r1", "%name は部品の名前");
        assert_eq!(r1.line, 4);
        // 枠 200 → 20（0.1 倍）、90 度回して原点 {20, 0} へ。アイコンの点 (100, 0) は (0, 10) に回り (20, 10)
        let p = apply(&r1.transform, [100.0, 0.0]);
        assert!(
            (p[0] - 20.0).abs() < 1e-9 && (p[1] - 10.0).abs() < 1e-9,
            "{p:?}"
        );
        assert!((r1.bbox[3] - 10.0).abs() < 1e-9);
        let r2 = &d.parts[1];
        assert_eq!(r2.transform[0], -0.1, "枠の向きが逆なら反転、配列は 1 つ");
        assert!(
            d.parts[2].shapes.is_empty(),
            "型が引けない部品は空のアイコン"
        );
        assert_eq!(d.connections.len(), 2, "端が部品でない connect は飛ばす");
        let c = &d.connections[0];
        assert_eq!((c.from.as_str(), c.to.as_str()), ("r1.n", "r2.p"));
        assert_eq!(c.line.points.len(), 3);
        assert_eq!(c.line.line, Some([0, 0, 255]));
        assert_eq!(
            d.connections[1].line.points.len(),
            2,
            "Line が無ければ中心どうし"
        );
        assert_eq!(d.extent, [-50.0, -50.0, 50.0, 50.0]);
        assert_eq!(d.shapes[0].text, "hi");
    }

    #[test]
    fn without_a_coordinate_system_the_extent_fits_the_parts() {
        let d = diagram("model M\n  Real x;\n  Foo a annotation(Placement(transformation(extent = {{0, 0}, {100, 50}})));\nend M;", "M", &|_| None).unwrap();
        assert_eq!(d.parts.len(), 1);
        assert!(d.extent[0] < 0.0 && d.extent[2] > 100.0, "{:?}", d.extent);
        let empty = diagram("model M end M;", "M", &|_| None).unwrap();
        assert_eq!(empty.extent, [-100.0, -100.0, 100.0, 100.0]);
        assert!(diagram("model M end M;", "N", &|_| None).is_none());
        assert!(
            diagram(
                "model M annotation(Diagram(graphics = {{{{ ;\nend M;",
                "M",
                &|_| None
            )
            .is_none()
        );
    }
}

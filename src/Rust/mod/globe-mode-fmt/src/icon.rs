//! クラスの `annotation(Icon(coordinateSystem(extent = …), graphics = {…}))` を形のデータにする（S244）。
//! 字句解析（[`lex`]）と文の切り出し（[`class_spans`]）だけで読み、Rumoca の構文解析も SVG の文字列も作らない（描くのは UI）。
//! 読めない要素・属性は飛ばして続ける（壊れた annotation でも空のアイコン）。評価はしない（`DynamicSelect` は最初の引数、式は読めない値）。

use crate::documentation::{join, unescape, within};
use crate::lexer::{Kind, Token, lex};
use crate::structure::{class_spans, skip_group};

/// 1 つの図形。座標は Modelica のまま（y は上向き）。使わない項目は 0・空。
/// `kind` は rect・ellipse・line・polygon・text・bitmap。text の色は `line`、bitmap の `text` はファイル名。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    pub kind: &'static str,
    pub origin: [f64; 2],
    pub rotation: f64,
    pub extent: [f64; 4],
    pub points: Vec<[f64; 2]>,
    /// 線（text は文字）の色。線が無ければ None
    pub line: Option<[u8; 3]>,
    /// 塗りの色。塗りが無ければ None（`fillPattern` が Solid 以外でも None 以外は Solid 扱い）
    pub fill: Option<[u8; 3]>,
    pub thickness: f64,
    /// solid・dash・dot・dashdot・dashdotdot
    pub dash: &'static str,
    pub radius: f64,
    pub text: String,
    /// 0 は枠に合わせる
    pub font_size: f64,
    /// left・center・right
    pub align: &'static str,
    /// line の両端: none・open・filled・half
    pub arrows: [&'static str; 2],
    pub arrow_size: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    pub class: String,
    /// `coordinateSystem(extent)` の `[x1, y1, x2, y2]`（無ければ `[-100, -100, 100, 100]`）
    pub extent: [f64; 4],
    /// 描く順（基底クラスのあとに自分）
    pub shapes: Vec<Shape>,
}

const DEFAULT_EXTENT: [f64; 4] = [-100.0, -100.0, 100.0, 100.0];
/// `extends` をたどる段数（基底クラスの Icon を重ねる）。
const EXTENDS_DEPTH: usize = 1;
const MAX_NEST: usize = 64;

/// `src` の `qualified`（`within` を含む完全修飾名）のクラスのアイコン。クラスが無ければ None（あってアイコンが無ければ空）。
/// `library(名前)` はそのクラスを定義するファイルの本文（`extends` 先を引く）。
pub fn icon(src: &str, qualified: &str, library: &dyn Fn(&str) -> Option<String>) -> Option<Icon> {
    icon_named(src, qualified, library, None)
}

/// `icon` の `%name` を `instance`（部品の名前。S245: Diagram の部品）にしたもの。None ならクラスの名前。
pub(crate) fn icon_named(
    src: &str,
    qualified: &str,
    library: &dyn Fn(&str) -> Option<String>,
    instance: Option<&str>,
) -> Option<Icon> {
    let (extent, shapes) = collect(src, qualified, library, EXTENDS_DEPTH, instance)?;
    Some(Icon {
        class: qualified.to_string(),
        extent: extent.unwrap_or(DEFAULT_EXTENT),
        shapes,
    })
}

type Parts = (Option<[f64; 4]>, Vec<Shape>);

fn collect(
    src: &str,
    qualified: &str,
    library: &dyn Fn(&str) -> Option<String>,
    depth: usize,
    instance: Option<&str>,
) -> Option<Parts> {
    let tokens = lex(src.strip_prefix('\u{feff}').unwrap_or(src)).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let span = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    let (mut extent, mut shapes) = (None, Vec::new());
    if depth > 0 {
        for base in &span.extends {
            // 相対名は外側のクラスから順に試す（同じファイルの兄弟→ライブラリ）
            let mut scope = qualified;
            loop {
                scope = scope.rsplit_once('.').map_or("", |(p, _)| p);
                let cand = if scope.is_empty() {
                    base.clone()
                } else {
                    format!("{scope}.{base}")
                };
                let found = spans
                    .iter()
                    .find(|s| join(&within, &s.path) == cand)
                    .and_then(|_| collect(src, &cand, library, depth - 1, instance))
                    .or_else(|| {
                        library(&cand)
                            .and_then(|t| collect(&t, &cand, library, depth - 1, instance))
                    });
                if let Some((e, s)) = found {
                    extent = extent.or(e);
                    shapes.extend(s);
                    break;
                }
                if scope.is_empty() {
                    break;
                }
            }
        }
    }
    let name = instance.unwrap_or_else(|| qualified.rsplit('.').next().unwrap_or(qualified));
    if let Some(Val::Call(_, args)) = span.annotation.and_then(|a| annotation_call(a, "Icon")) {
        let cs = args.iter().find_map(|(_, v)| match v {
            Val::Call(n, cs) if n == "coordinateSystem" => Some(cs),
            _ => None,
        });
        if let Some(cs) = cs {
            extent = get(cs, "extent").and_then(rect).or(extent);
        }
        if let Some(Val::List(items)) = get(&args, "graphics") {
            shapes.extend(items.iter().filter_map(|v| shape(v, name, qualified)));
        }
    }
    Some((extent, shapes))
}

#[derive(Debug, Clone)]
pub(crate) enum Val {
    Num(f64),
    Str(String),
    Id(String),
    List(Vec<Val>),
    Call(String, Vec<(String, Val)>),
    Other,
}

/// `annotation(…)` の直下の `Icon(…)`・`Diagram(…)`・`Placement(…)`（`name`）。
pub(crate) fn annotation_call(annotation: &[Token<'_>], name: &str) -> Option<Val> {
    let mut i = 1;
    if !annotation.get(i)?.is("(") {
        return None;
    }
    i += 1;
    args(annotation, &mut i, 0)
        .into_iter()
        .find_map(|(_, v)| matches!(&v, Val::Call(n, _) if n == name).then_some(v))
}

/// `(` の次から `)` までの引数（`名前 = 値` か位置引数＝名前は空）。`*i` は `)` の次に進む。
pub(crate) fn args(t: &[Token<'_>], i: &mut usize, nest: usize) -> Vec<(String, Val)> {
    let mut out = Vec::new();
    while let Some(tok) = t.get(*i) {
        if tok.is(")") {
            *i += 1;
            break;
        }
        let before = *i;
        let name = match (tok.kind == Kind::Ident, t.get(*i + 1)) {
            (true, Some(eq)) if eq.is("=") => {
                *i += 2;
                tok.text.to_string()
            }
            _ => String::new(),
        };
        out.push((name, value(t, i, nest + 1)));
        if t.get(*i).is_some_and(|c| c.is(",")) || *i == before {
            *i += 1; // 進まないときの保険
        }
    }
    out
}

fn value(t: &[Token<'_>], i: &mut usize, nest: usize) -> Val {
    let Some(tok) = t.get(*i) else {
        return Val::Other;
    };
    if nest > MAX_NEST {
        *i = t.len();
        return Val::Other;
    }
    let v = match tok.kind {
        Kind::Number => {
            *i += 1;
            tok.text.parse().map_or(Val::Other, Val::Num)
        }
        Kind::Str | Kind::Html => {
            let mut s = unescape(tok.text);
            *i += 1;
            while t.get(*i).is_some_and(|p| p.is("+"))
                && t.get(*i + 1)
                    .is_some_and(|n| matches!(n.kind, Kind::Str | Kind::Html))
            {
                s.push_str(&unescape(t[*i + 1].text));
                *i += 2;
            }
            Val::Str(s)
        }
        Kind::Punct if tok.text == "-" || tok.text == "+" => {
            *i += 1;
            match value(t, i, nest + 1) {
                Val::Num(n) if tok.text == "-" => Val::Num(-n),
                v @ Val::Num(_) => v,
                _ => Val::Other,
            }
        }
        Kind::Punct if tok.text == "{" => {
            *i += 1;
            let mut items = Vec::new();
            while let Some(c) = t.get(*i) {
                if c.is("}") {
                    *i += 1;
                    break;
                }
                let before = *i;
                items.push(value(t, i, nest + 1));
                if t.get(*i).is_some_and(|c| c.is(",")) || *i == before {
                    *i += 1; // 進まないときの保険
                }
            }
            Val::List(items)
        }
        Kind::Ident => {
            let mut name = tok.text.to_string();
            *i += 1;
            while t.get(*i).is_some_and(|d| d.is("."))
                && t.get(*i + 1).is_some_and(|n| n.kind == Kind::Ident)
            {
                name.push('.');
                name.push_str(t[*i + 1].text);
                *i += 2;
            }
            if t.get(*i).is_some_and(|p| p.is("(")) {
                *i += 1;
                Val::Call(name, args(t, i, nest + 1))
            } else {
                Val::Id(name)
            }
        }
        _ => {
            if !(tok.is(",") || tok.is(")") || tok.is("}")) {
                *i += 1;
            }
            Val::Other
        }
    };
    // 値のあとに式が続く（`a * b`）なら読めない値
    let mut rest = false;
    while let Some(n) = t.get(*i).filter(|n| !(n.is(",") || n.is(")") || n.is("}"))) {
        *i = if n.is("(") || n.is("{") || n.is("[") {
            skip_group(t, *i).unwrap_or(t.len())
        } else {
            *i + 1
        };
        rest = true;
    }
    if rest { Val::Other } else { v }
}

pub(crate) fn get<'a>(args: &'a [(String, Val)], key: &str) -> Option<&'a Val> {
    args.iter().find(|(k, _)| k == key).map(|(_, v)| plain(v))
}

/// `DynamicSelect(静的な値, 動的な式)` は静的な値。
fn plain(v: &Val) -> &Val {
    match v {
        Val::Call(n, a) if n == "DynamicSelect" => a.first().map_or(v, |(_, f)| plain(f)),
        _ => v,
    }
}

pub(crate) fn num(v: &Val) -> Option<f64> {
    match plain(v) {
        Val::Num(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

pub(crate) fn nums(v: &Val) -> Option<Vec<f64>> {
    match plain(v) {
        Val::List(l) => l.iter().map(num).collect(),
        _ => None,
    }
}

/// `{{x1, y1}, {x2, y2}}`
pub(crate) fn rect(v: &Val) -> Option<[f64; 4]> {
    let p = points(v)?;
    (p.len() == 2).then(|| [p[0][0], p[0][1], p[1][0], p[1][1]])
}

pub(crate) fn points(v: &Val) -> Option<Vec<[f64; 2]>> {
    match plain(v) {
        Val::List(l) => l
            .iter()
            .map(|p| nums(p).filter(|n| n.len() == 2).map(|n| [n[0], n[1]]))
            .collect(),
        _ => None,
    }
}

pub(crate) fn color(v: &Val) -> Option<[u8; 3]> {
    let n = nums(v).filter(|n| n.len() == 3)?;
    Some([
        n[0].clamp(0.0, 255.0) as u8,
        n[1].clamp(0.0, 255.0) as u8,
        n[2].clamp(0.0, 255.0) as u8,
    ])
}

/// `LinePattern.Dash` などの列挙値の最後の名前（小文字）。
pub(crate) fn variant(v: Option<&Val>) -> String {
    match v.map(plain) {
        Some(Val::Id(s)) => s.rsplit('.').next().unwrap_or(s).to_ascii_lowercase(),
        _ => String::new(),
    }
}

pub(crate) fn shape(v: &Val, name: &str, class: &str) -> Option<Shape> {
    let Val::Call(kind, a) = v else { return None };
    if variant(get(a, "visible")) == "false" {
        return None;
    }
    let arg = |k: &str| get(a, k);
    let pair = |k: &str| arg(k).and_then(nums).filter(|n| n.len() == 2);
    let origin = pair("origin").map_or([0.0; 2], |n| [n[0], n[1]]);
    let kind = match kind.as_str() {
        "Rectangle" => "rect",
        "Ellipse" => "ellipse",
        "Line" => "line",
        "Polygon" => "polygon",
        "Text" => "text",
        "Bitmap" => "bitmap",
        _ => return None,
    };
    let black = [0, 0, 0];
    let mut s = Shape {
        kind,
        origin,
        rotation: arg("rotation").and_then(num).unwrap_or(0.0),
        thickness: arg(if kind == "line" {
            "thickness"
        } else {
            "lineThickness"
        })
        .and_then(num)
        .unwrap_or(0.25),
        dash: match variant(arg("pattern")).as_str() {
            "dash" => "dash",
            "dot" => "dot",
            "dashdot" => "dashdot",
            "dashdotdot" => "dashdotdot",
            _ => "solid",
        },
        align: "center",
        arrow_size: arg("arrowSize").and_then(num).unwrap_or(3.0),
        ..Shape::default()
    };
    let stroke = arg(if kind == "line" { "color" } else { "lineColor" })
        .and_then(color)
        .unwrap_or(black);
    let no_line = variant(arg("pattern")) == "none";
    match kind {
        "rect" | "ellipse" | "bitmap" => s.extent = arg("extent").and_then(rect)?,
        "text" => s.extent = arg("extent").and_then(rect)?,
        _ => {
            s.points = arg("points").and_then(points)?;
            if s.points.len() < 2 {
                return None;
            }
        }
    }
    match kind {
        "text" => {
            let raw = match arg("textString") {
                Some(Val::Str(t)) => t,
                _ => return None,
            };
            s.text = raw.replace("%name", name).replace("%class", class);
            if s.text.contains('%') {
                return None; // 部品の値の置き換え（`%k`）は評価しない
            }
            s.line = Some(arg("textColor").and_then(color).unwrap_or(stroke));
            s.font_size = arg("fontSize").and_then(num).unwrap_or(0.0);
            s.align = match variant(arg("horizontalAlignment")).as_str() {
                "left" => "left",
                "right" => "right",
                _ => "center",
            };
        }
        "bitmap" => {
            s.line = Some([128, 128, 128]);
            s.text = match arg("fileName") {
                Some(Val::Str(f)) => f.rsplit(['/', '\\']).next().unwrap_or(f).to_string(),
                _ => "bitmap".into(),
            };
        }
        _ => {
            s.line = (!no_line).then_some(stroke);
            if kind != "line" && !matches!(variant(arg("fillPattern")).as_str(), "" | "none") {
                s.fill = Some(arg("fillColor").and_then(color).unwrap_or(black));
            }
            if kind == "rect" {
                s.radius = arg("radius").and_then(num).unwrap_or(0.0);
            }
            if kind == "line" {
                let end = |v: &str| match v {
                    "open" => "open",
                    "filled" => "filled",
                    "half" => "half",
                    _ => "none",
                };
                if let Some(Val::List(l)) = arg("arrow") {
                    s.arrows = [end(&variant(l.first())), end(&variant(l.get(1)))];
                } else {
                    s.arrows = ["none"; 2];
                }
            }
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"within P;
package Interfaces
  partial block Base
    annotation(Icon(coordinateSystem(extent = {{-50, -40}, {50, 40}}), graphics = {Rectangle(extent = {{-50, -40}, {50, 40}}, lineColor = {0, 0, 127}, fillColor = {255, 255, 255}, fillPattern = FillPattern.Solid), Text(extent = {{-150, 60}, {150, 100}}, textString = "%name", textColor = {0, 0, 255})}));
  end Base;
end Interfaces;
"#;
    const MODEL: &str = r#"within P;
model Gain "g"
  extends Interfaces.Base;
  parameter Real k = 1;
  annotation(Documentation(info = "<b>x</b>"), Icon(graphics = {
    Line(points = {{-80, 0}, {80, 0}}, color = {0, 0, 0}, arrow = {Arrow.None, Arrow.Filled}, pattern = LinePattern.Dash, thickness = 0.5),
    Polygon(points = {{-10, -10}, {-10, 10}, {10, 0}}, fillColor = {1, 2, 3}, fillPattern = FillPattern.HorizontalCylinder, smooth = Smooth.Bezier),
    Ellipse(extent = {{-5, -5}, {5, 5}}, origin = {10, 20}, rotation = 30, visible = DynamicSelect(true, k > 1)),
    Ellipse(extent = {{-5, -5}, {5, 5}}, visible = false),
    Text(extent = {{-100, -60}, {100, -40}}, textString = "k=%k"),
    Text(extent = {{-100, -60}, {100, -40}}, textString = "%class: " + "%name", fontSize = 12),
    Bitmap(extent = {{-1, -1}, {1, 1}}, fileName = "modelica://P/Resources/a.png"),
    Rectangle(extent = {{-1, -1}}),
    Foo(a = 1),
    Rectangle(extent = {{-9, -9}, {9, 9}}, radius = 2 * 3)
  }));
end Gain;
"#;

    fn lib(c: &str) -> Option<String> {
        (c == "P.Interfaces.Base").then(|| SRC.to_string())
    }

    #[test]
    fn shapes_are_read_and_the_base_icon_is_layered_once() {
        let i = icon(MODEL, "P.Gain", &lib).unwrap();
        assert_eq!(i.extent, [-50.0, -40.0, 50.0, 40.0], "extent は基底から");
        let kinds: Vec<_> = i.shapes.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [
                "rect", "text", "line", "polygon", "ellipse", "text", "bitmap", "rect"
            ]
        );
        assert_eq!(i.shapes[0].fill, Some([255, 255, 255]));
        assert_eq!(i.shapes[0].line, Some([0, 0, 127]));
        assert_eq!(
            i.shapes[1].text, "Base",
            "%name はクラスの名前（基底のもの）"
        );
        let line = &i.shapes[2];
        assert_eq!(
            (line.dash, line.arrows, line.thickness, line.fill),
            ("dash", ["none", "filled"], 0.5, None)
        );
        assert_eq!(
            i.shapes[3].fill,
            Some([1, 2, 3]),
            "Solid 以外の塗りは Solid 扱い"
        );
        assert_eq!(
            (i.shapes[4].origin, i.shapes[4].rotation),
            ([10.0, 20.0], 30.0)
        );
        let t = &i.shapes[5];
        assert_eq!((t.text.as_str(), t.font_size), ("P.Gain: Gain", 12.0));
        assert_eq!(i.shapes[6].text, "a.png");
        assert_eq!(i.shapes[7].radius, 0.0, "式は読めない値");
    }

    #[test]
    fn broken_or_missing_annotations_give_an_empty_icon() {
        let empty =
            |src: &str, c: &str| icon(src, c, &|_| None).map(|i| (i.extent, i.shapes.len()));
        assert_eq!(empty("model M end M;", "M"), Some((DEFAULT_EXTENT, 0)));
        assert_eq!(empty("model M\n annotation(Icon(graphics = {Rectangle(extent = {{1, 2}, {3, 4}}, ), })); \nend M;", "M").map(|e| e.1), Some(1));
        assert_eq!(
            empty("model M\n annotation(Icon(graphics = {{{{{ ;\nend M;", "M"),
            None,
            "括弧が閉じない文書は文の切り出しで落ちる（ほかの機能と同じ）"
        );
        assert_eq!(
            empty(
                "model M\n annotation(Icon(graphics = 3, coordinateSystem(extent = 1)));\nend M;",
                "M"
            ),
            Some((DEFAULT_EXTENT, 0))
        );
        assert_eq!(empty("model M", "M"), None);
        assert_eq!(empty("model M end M;", "N"), None);
        let deep = format!(
            "model M\n annotation(Icon(graphics = {}{}));\nend M;",
            "{".repeat(100_000),
            "}".repeat(100_000)
        );
        assert_eq!(empty(&deep, "M").map(|e| e.1), Some(0));
    }
}

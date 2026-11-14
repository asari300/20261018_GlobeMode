//! クラスの Diagram（部品の配置と `connect`）を形のデータにする（S245。読み取り専用）。
//! `icon.rs` と同じく字句解析（[`lex`]）・文の切り出し（[`split`]）・クラスの範囲（[`class_spans`]）だけで読み、評価も SVG の文字列もしない（描くのは UI）。
//! 部品は `annotation(Placement(transformation(…)))` を持つ宣言だけ。部品のクラスの Icon を引いて（`extends` は 1 段）変換の行列を付ける。
//! 読めないもの（型が引けない・Placement が壊れている・端が部品でない connect）は飛ばして続ける。`extends` で継ぐ部品・`import {A, B}` は読まない。

use crate::documentation::{join, within};
use crate::icon::{
    Shape, Val, annotation_call, find_base, get, icon_named, num, nums, rect, shape,
};
use crate::lexer::{Kind, Token, lex};
use crate::structure::{class_spans, dotted, skip_group, split};

/// 名前と位置の並び（コネクタ）。
pub type Located = Vec<(String, [f64; 2])>;

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
    /// 部品のクラス（と `extends` の 2 段まで）のコネクタの位置（名前と Diagram の座標。S261: 結線を引くヒット領域。`Placement` を持ち、型が `connector` でないと分かるものは除く）
    pub connectors: Located,
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
            read_import(t, &mut aliases, &mut wild);
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
        let (icon, conns) = resolve(
            &r.ty, &r.name, qualified, &aliases, &wild, &local, src, library,
        )
        .unzip();
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
            connectors: conns
                .unwrap_or_default()
                .into_iter()
                .map(|(n, p)| (n, apply(&transform, p)))
                .collect(),
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

/// 部品 `part` を図の上で (dx, dy) だけ動かす編集（S250）。`Placement(transformation(origin = {x, y}))` の `{x, y}` を書き換える
/// （`origin` が無ければ `transformation(` の直後に `origin = {dx, dy}, ` を足す）。返すのは `src` のバイト範囲と、そこへ入れる文字列。
/// 動かせない（クラス・部品が無い・Placement が無い・`origin` が数の並びでない）ときは None。ソースの他の所には触らない。
pub fn move_part(
    src: &str,
    qualified: &str,
    part: &str,
    dx: f64,
    dy: f64,
) -> Option<(usize, usize, String)> {
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let own = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    for e in split(&tokens, false).ok()? {
        let t = e.tokens;
        let st = t[0].start;
        if t[0].is_comment() || spans.iter().any(|s| s.start == st) {
            continue;
        }
        let owner = spans
            .iter()
            .filter(|s| s.start <= st && st < s.end)
            .max_by_key(|s| s.start);
        if owner.is_none_or(|o| o.path != own.path)
            || declaration(t, body).is_none_or(|r| r.name != part)
        {
            continue;
        }
        let a = t.iter().position(|s| s.is("annotation"))?;
        let Some(Val::Call(_, p)) = annotation_in(t, "Placement") else {
            return None;
        };
        let tr = call(&p, "transformation")?;
        let at = |i: usize| t.get(i).map(|s| s.start + bom);
        let fmt = |v: f64| {
            let v = (v * 1000.0).round() / 1000.0 + 0.0; // -0 を 0 に
            v.to_string()
        };
        return match get(tr, "origin") {
            Some(v) => {
                let n = nums(v).filter(|n| n.len() == 2)?;
                // 値の `{ … }` は、annotation 以降の `origin` `=` のあとの波括弧
                let o = (a..t.len())
                    .find(|&i| t[i].is("origin") && t.get(i + 1).is_some_and(|s| s.is("=")))?;
                let close = skip_group(t, o + 2).ok()?;
                Some((
                    at(o + 2)?,
                    at(close - 1)? + 1,
                    format!("{{{}, {}}}", fmt(n[0] + dx), fmt(n[1] + dy)),
                ))
            }
            None => {
                let o = (a..t.len()).find(|&i| {
                    t[i].is("transformation") && t.get(i + 1).is_some_and(|s| s.is("("))
                })?;
                let pos = at(o + 1)? + 1;
                Some((pos, pos, format!("origin = {{{}, {}}}, ", fmt(dx), fmt(dy))))
            }
        };
    }
    None
}

/// クラス `qualified` に `connect(a, b) annotation(Line(points = {…}, color = {0, 0, 255}));` を足す編集（S261）。
/// 返すのは `src` のバイト範囲（挿入だけなので始めと終わりは同じ）と、そこへ入れる文字列。
/// 最後の `equation` 節の末尾（クラスの `annotation` の前）に、その節の字下げで足す。`equation` が無ければ、クラスの `annotation`（無ければ `end`）の行の前に
/// `equation` 節ごと足す（`equation` は `end` の字下げ、文は最初の宣言の字下げ）。`a`・`b` は `part.connector`（英数字・`_`・`.`・`[]` だけ）。
/// `points` は図の座標の線（空なら `annotation` を付けない）。クラスが無い・端が不正・挿入位置の行に他の文がある（1 行に詰めた）ときは None。
pub fn add_connect(
    src: &str,
    qualified: &str,
    a: &str,
    b: &str,
    points: &[[f64; 2]],
) -> Option<(usize, usize, String)> {
    let end_ok = |e: &str| {
        !e.is_empty()
            && e.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.[]".contains(c))
    };
    if !end_ok(a) || !end_ok(b) {
        return None;
    }
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let own = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    let name_of = qualified.rsplit('.').next().unwrap_or(qualified);
    // 自分のクラス直下の要素（入れ子のクラスの中は除く）
    let mut mine = Vec::new();
    for e in split(&tokens, false).ok()? {
        let st = e.tokens[0].start;
        let inner = spans
            .iter()
            .filter(|s| s.start <= st && st < s.end)
            .max_by_key(|s| s.start);
        if inner.is_some_and(|o| o.path == own.path) && !spans.iter().any(|s| s.start == st) {
            mine.push(e.tokens);
        }
    }
    let fmt = |v: f64| ((v * 1000.0).round() / 1000.0 + 0.0).to_string();
    let line = if points.is_empty() {
        String::new()
    } else {
        let pts: Vec<_> = points
            .iter()
            .map(|p| format!("{{{}, {}}}", fmt(p[0]), fmt(p[1])))
            .collect();
        format!(
            " annotation(Line(points = {{{}}}, color = {{0, 0, 255}}))",
            pts.join(", ")
        )
    };
    let stmt = format!("connect({a}, {b}){line};");
    let indent_of = |pos: usize| {
        let ls = body[..pos].rfind('\n').map_or(0, |i| i + 1);
        body[ls..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>()
    };
    let last_eq = mine.iter().rposition(|t| t[0].is("equation"));
    if let Some(i) = last_eq {
        // 節は次の節の見出しの手前まで。クラスの annotation は節の外
        let section = mine[i + 1..].iter().take_while(|t| {
            !(t[0].is("equation")
                || t[0].is("algorithm")
                || t[0].is("initial")
                || t[0].is("public")
                || t[0].is("protected")
                || t[0].is("annotation")
                || (t[0].is("end") && t.get(1).is_some_and(|n| n.is(name_of))))
        });
        let last = section.last().copied();
        let indent = match last {
            Some(t) => indent_of(t[0].start),
            None => decl_indent(&mine, body, name_of),
        };
        let t = last.unwrap_or(mine[i]);
        let end = t.last()?;
        let pos = end.start + end.text.len() + bom;
        return Some((pos, pos, format!("\n{indent}{stmt}")));
    }
    let target = mine.iter().find(|t| t[0].is("annotation")).or_else(|| {
        mine.iter()
            .find(|t| t[0].is("end") && t.get(1).is_some_and(|n| n.is(name_of)))
    })?;
    let end_ind = indent_of(
        mine.iter()
            .find(|t| t[0].is("end") && t.get(1).is_some_and(|n| n.is(name_of)))?[0]
            .start,
    );
    let ls = body[..target[0].start].rfind('\n').map_or(0, |i| i + 1);
    if !body[ls..target[0].start].trim().is_empty() {
        return None; // 同じ行に他の文がある
    }
    let decl = decl_indent(&mine, body, name_of);
    Some((
        ls + bom,
        ls + bom,
        format!("{end_ind}equation\n{decl}{stmt}\n"),
    ))
}

/// クラスの最初の宣言の字下げ（無ければ `end` の字下げ + 2 桁）。
fn decl_indent(mine: &[&[Token<'_>]], body: &str, name_of: &str) -> String {
    let ind = |pos: usize| {
        let ls = body[..pos].rfind('\n').map_or(0, |i| i + 1);
        body[ls..pos].to_string()
    };
    mine.iter()
        .find(|t| {
            !t[0].is_comment() && !t[0].is("end") && !t[0].is("annotation") && !t[0].is("equation")
        })
        .map(|t| ind(t[0].start))
        .or_else(|| {
            let e = mine
                .iter()
                .find(|t| t[0].is("end") && t.get(1).is_some_and(|n| n.is(name_of)))?;
            Some(format!("{}  ", ind(e[0].start)))
        })
        .unwrap_or_else(|| "  ".to_string())
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

/// 宣言の頭（`接頭辞 型 [配列] 名前`）の型の名前と名前のトークン。
fn head<'t, 'a>(t: &'t [Token<'a>]) -> Option<(String, &'t Token<'a>)> {
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
    Some((ty, name))
}

/// 部品の宣言（`型 名前 … annotation(Placement(transformation(…)))`）。Placement が無い・隠す・壊れていれば None。
fn declaration(t: &[Token<'_>], body: &str) -> Option<Raw> {
    let (ty, name) = head(t)?;
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

/// `import` 1 文を別名（`A = B.C` と `import B.C` の `C`）かワイルドカード（`B.*` の `B`）に積む。
fn read_import(t: &[Token<'_>], aliases: &mut Vec<(String, String)>, wild: &mut Vec<String>) {
    if t.get(2).is_some_and(|s| s.is("=")) {
        aliases.push((t[1].text.to_string(), dotted(t, 3)));
    } else {
        let d = dotted(t, 1);
        match d.strip_suffix('.') {
            Some(w) => wild.push(w.to_string()),
            None => aliases.push((d.rsplit('.').next().unwrap_or(&d).to_string(), d)),
        }
    }
}

/// 型の名前 `ty` の完全修飾名の候補（`import` の別名・`.*`・内側から外側へのスコープの順）。
fn candidates(ty: &str, class: &str, aliases: &[(String, String)], wild: &[String]) -> Vec<String> {
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
    cands
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
) -> Option<(crate::icon::Icon, Located)> {
    let cands = candidates(ty, class, aliases, wild);
    let both = |t: &str, c: &str| {
        let icon = icon_named(t, c, library, Some(name))?;
        Some((icon, connectors(t, c, library, CONNECTOR_DEPTH)?))
    };
    cands.into_iter().find_map(|c| {
        (if local(&c) { both(src, &c) } else { None })
            .or_else(|| library(&c).and_then(|t| both(&t, &c)))
    })
}

/// `extends` をたどる段数（コネクタは `Resistor` → `OnePort` → `TwoPin` のように 2 段）。
const CONNECTOR_DEPTH: usize = 2;

/// クラス `qualified` の、`Placement` を持つ宣言（`parameter`・`constant` を除く）の名前と位置（部品のアイコンの座標）。クラスが無ければ None。
/// `iconTransformation` があればそれ、無ければ `transformation` の `extent` の中心を `origin` のまわりに `rotation` 回したもの。
/// 基底クラスのものを先に、自分のものを後に（同じ名前は自分が勝つ）。`iconTransformation` が無い宣言は、型が引けてコネクタでなければ除く
/// （`LimPID` の中の `Gain` など。引けなければ残す）。
fn connectors(
    src: &str,
    qualified: &str,
    library: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> Option<Located> {
    let body = src.strip_prefix('\u{feff}').unwrap_or(src);
    let tokens = lex(body).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let own = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    let mut out: Located = Vec::new();
    let mut put = |n: String, p: [f64; 2]| {
        out.retain(|(m, _)| *m != n);
        out.push((n, p));
    };
    if depth > 0 {
        for base in &own.extends {
            let f = |t: &str, c: &str| connectors(t, c, library, depth - 1);
            for (n, p) in
                find_base(src, &spans, &within, qualified, base, library, &f).unwrap_or_default()
            {
                put(n, p);
            }
        }
    }
    let (mut aliases, mut wild) = (Vec::<(String, String)>::new(), Vec::<String>::new());
    for e in split(&tokens, false).ok()? {
        let t = e.tokens;
        let st = t[0].start;
        let Some(owner) = spans
            .iter()
            .filter(|s| s.start <= st && st < s.end)
            .max_by_key(|s| s.start)
        else {
            continue;
        };
        if t[0].is("import")
            && (owner.path == own.path || own.path.starts_with(&format!("{}.", owner.path)))
        {
            read_import(t, &mut aliases, &mut wild);
            continue;
        }
        if t[0].is_comment()
            || t[0].is("parameter")
            || t[0].is("constant")
            || owner.path != own.path
            || spans.iter().any(|s| s.start == st)
        {
            continue;
        }
        if let Some((ty, name)) = head(t)
            && let Some(Val::Call(_, p)) = annotation_in(t, "Placement")
            && let Some(tr) = call(&p, "iconTransformation").or_else(|| call(&p, "transformation"))
            && let Some(x) = get(tr, "extent").and_then(rect)
            && (call(&p, "iconTransformation").is_some()
                || is_connector(&ty, qualified, &aliases, &wild, src, library) != Some(false))
        {
            let o = get(tr, "origin")
                .and_then(nums)
                .filter(|n| n.len() == 2)
                .map_or([0.0; 2], |n| [n[0], n[1]]);
            let (s, c) = get(tr, "rotation")
                .and_then(num)
                .unwrap_or(0.0)
                .to_radians()
                .sin_cos();
            let (u, v) = ((x[0] + x[2]) / 2.0, (x[1] + x[3]) / 2.0);
            put(
                name.text.to_string(),
                [o[0] + c * u - s * v, o[1] + s * u + c * v],
            );
        }
    }
    Some(out)
}

/// 型 `ty` のクラスが `connector` か。引けなければ None。ファイルを字句解析せず、`<種類> <名前>` の字面を探すだけ（大きなライブラリの本文を何度も読むので）。
fn is_connector(
    ty: &str,
    class: &str,
    aliases: &[(String, String)],
    wild: &[String],
    src: &str,
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<bool> {
    let last = ty.rsplit('.').next().unwrap_or(ty);
    candidates(ty, class, aliases, wild)
        .into_iter()
        .find_map(|c| {
            let text = library(&c).or_else(|| (c == ty).then(|| src.to_string()))?;
            text.match_indices(last).find_map(|(i, _)| {
                let after = text[i + last.len()..].chars().next();
                if after.is_some_and(|ch| ch.is_alphanumeric() || ch == '_') {
                    return None;
                }
                let kind = text[..i].split_whitespace().next_back()?;
                match kind {
                    "connector" => Some(true),
                    "model" | "block" | "record" | "class" | "type" | "package" | "function" => {
                        Some(false)
                    }
                    _ => None,
                }
            })
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

    #[test]
    fn move_part_rewrites_or_adds_the_origin_and_leaves_the_rest_alone() {
        let apply_edit = |src: &str, part: &str, dx: f64, dy: f64| {
            let (a, b, new) = move_part(src, "M", part, dx, dy)?;
            Some(format!("{}{new}{}", &src[..a], &src[b..]))
        };
        let m = "model M\n  A r1(R = 1) annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}}, origin = {20, 0.5}, rotation = 90)));\n  A r2 annotation(Placement(transformation(extent = {{0, 0}, {10, 10}})));\n  A r3 annotation(Placement(transformation(origin = {1, 2})));\n  A plain;\n  A odd annotation(Placement(transformation(origin = {x, 0})));\nend M;\n";
        let r1 = apply_edit(m, "r1", 4.0, -0.25).unwrap();
        assert!(r1.contains("origin = {24, 0.25}, rotation = 90"), "{r1}");
        assert!(r1.contains("A r2 annotation") && r1.ends_with("end M;\n"));
        let r2 = apply_edit(m, "r2", 2.0, 0.0).unwrap();
        assert!(
            r2.contains("transformation(origin = {2, 0}, extent = {{0, 0}, {10, 10}})"),
            "origin が無ければ足す: {r2}"
        );
        assert!(
            apply_edit(m, "r3", -1.0, -2.0)
                .unwrap()
                .contains("origin = {0, 0})"),
            "-0 は 0"
        );
        assert!(
            apply_edit(m, "plain", 1.0, 1.0).is_none(),
            "Placement が無い宣言は動かさない"
        );
        assert!(
            apply_edit(m, "odd", 1.0, 1.0).is_none(),
            "origin が数でなければ動かさない"
        );
        assert!(apply_edit(m, "nope", 1.0, 1.0).is_none());
        assert!(move_part(m, "N", "r1", 1.0, 1.0).is_none());
        // 動かした結果を diagram が読み直すと、部品の中心がその分だけ動く
        let before = diagram(m, "M", &|_| None).unwrap().parts[0].bbox;
        let after = diagram(&r1, "M", &|_| None).unwrap().parts[0].bbox;
        assert!(
            (after[0] - before[0] - 4.0).abs() < 1e-9 && (after[1] - before[1] + 0.25).abs() < 1e-9
        );
        // BOM があっても範囲は元の文字列のまま
        let bom = format!("\u{feff}{m}");
        let (a, _, _) = move_part(&bom, "M", "r1", 1.0, 0.0).unwrap();
        assert!(bom[a..].starts_with('{'));
    }

    #[test]
    fn connectors_are_read_through_extends_and_placed_in_the_diagram() {
        const BASE: &str = "within P;\npackage Interfaces\n  partial model Two\n    Pin p annotation(Placement(transformation(extent = {{-110, -10}, {-90, 10}})));\n    Pin n annotation(Placement(transformation(extent = {{110, -10}, {90, 10}}), iconTransformation(extent = {{90, -10}, {110, 10}})));\n    parameter Real k = 1 annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\n  end Two;\nend Interfaces;\n";
        let lib = |c: &str| match c {
            "P.Basic.Res" => Some(LIB.to_string()),
            "P.Interfaces.Two" => Some(BASE.to_string()),
            _ => None,
        };
        let d = diagram(MODEL, "P.Circ", &lib).unwrap();
        // LIB の Res は `extends Interfaces.Two`（P.Basic.Interfaces.Two → P.Interfaces.Two の順に探す）
        let r1 = &d.parts[0];
        let names: Vec<_> = r1.connectors.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(names, ["p", "n"], "parameter は除く: {:?}", r1.connectors);
        // r1: 枠 200→20（0.1 倍）、90 度回して origin {20, 0}。p は (-100, 0) → (0, -10) → (20, -10)、n は iconTransformation の (100, 0) → (20, 10)
        let near =
            |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
        assert!(
            near(r1.connectors[0].1, [20.0, -10.0]),
            "{:?}",
            r1.connectors
        );
        assert!(
            near(r1.connectors[1].1, [20.0, 10.0]),
            "{:?}",
            r1.connectors
        );
    }

    #[test]
    fn a_declaration_without_icon_transformation_is_a_connector_only_if_its_type_is_not_known_to_be_something_else()
     {
        let lib = |c: &str| match c {
            "P.Pin" => Some("within P;\nconnector Pin\nend Pin;\n".to_string()),
            "P.Gain" => Some("within P;\nblock Gain\nend Gain;\n".to_string()),
            _ => None,
        };
        let src = "within P;\nmodel M\n  Pin p annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));\n  Gain g annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));\n  Unknown u annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));\n  Gain h annotation(Placement(transformation(extent = {{0, 0}, {2, 2}}), iconTransformation(extent = {{0, 0}, {2, 2}})));\nend M;\n";
        let got = connectors(src, "P.M", &lib, 0).unwrap();
        let names: Vec<_> = got.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(names, ["p", "u", "h"], "{got:?}");
        assert!(connectors(src, "P.Nope", &lib, 0).is_none());
    }

    #[test]
    fn add_connect_goes_to_the_end_of_the_last_equation_section() {
        let pts = [[0.0, 1.0], [10.0, 1.5]];
        let apply = |src: &str, class: &str| {
            let (a, b, t) = add_connect(src, class, "a.p", "b.n", &pts).unwrap();
            format!("{}{}{}", &src[..a], t, &src[b..])
        };
        let stmt = "connect(a.p, b.n) annotation(Line(points = {{0, 1}, {10, 1.5}}, color = {0, 0, 255}));";
        // equation 節がある: 最後の文の次の行へ、その字下げで。クラスの annotation の前
        let with = "model M\n  Box a;\nequation\n  x = 1;\n    y = 2; // 末尾\nannotation(Diagram());\nend M;\n";
        let out = apply(with, "M");
        assert_eq!(
            out,
            format!(
                "model M\n  Box a;\nequation\n  x = 1;\n    y = 2; // 末尾\n    {stmt}\nannotation(Diagram());\nend M;\n"
            )
        );
        // 空の equation 節: 最初の宣言の字下げ
        let empty = "model M\n  Box a;\nequation\nend M;\n";
        assert_eq!(
            apply(empty, "M"),
            format!("model M\n  Box a;\nequation\n  {stmt}\nend M;\n")
        );
        // equation が無い: annotation の前（無ければ end の前）に節ごと。入れ子のクラスは自分の end
        let none = "package P\n  model M\n    Box a;\n    annotation(Icon());\n  end M;\n  model N\n    Box b;\n  end N;\nend P;\n";
        assert_eq!(
            apply(none, "P.M"),
            format!(
                "package P\n  model M\n    Box a;\n  equation\n    {stmt}\n    annotation(Icon());\n  end M;\n  model N\n    Box b;\n  end N;\nend P;\n"
            )
        );
        assert_eq!(
            apply(none, "P.N"),
            format!(
                "package P\n  model M\n    Box a;\n    annotation(Icon());\n  end M;\n  model N\n    Box b;\n  equation\n    {stmt}\n  end N;\nend P;\n"
            )
        );
        // equation の後ろに algorithm / initial equation があっても、最後の equation 節の末尾
        let multi = "model M\n  Real x;\nequation\n  x = 1;\ninitial equation\n  x = 0;\nend M;\n";
        assert!(apply(multi, "M").contains("  x = 1;\n  connect(a.p, b.n)"));
        // 不正な端・クラス無し・線無し
        assert!(add_connect(with, "M", "a.p; x", "b.n", &pts).is_none());
        assert!(add_connect(with, "Nope", "a.p", "b.n", &pts).is_none());
        let (_, _, t) = add_connect(with, "M", "a.p", "b[2].n", &[]).unwrap();
        assert_eq!(t, "\n    connect(a.p, b[2].n);");
    }
}

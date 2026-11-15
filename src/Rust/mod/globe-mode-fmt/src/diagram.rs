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
        } else {
            raws.extend(declarations(t, body).into_iter().map(|(r, _)| r));
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
        if owner.is_none_or(|o| o.path != own.path) {
            continue;
        }
        // 複数の名前（`A a, b annotation(…)`）は名前ごとの区切りだけを見る（S288）
        let Some((_, t)) = declarations(t, body)
            .into_iter()
            .find(|(r, _)| r.name == part)
        else {
            continue;
        };
        let Some(Val::Call(_, p)) = annotation_in(t, "Placement") else {
            return None;
        };
        let tr = call(&p, "transformation")?;
        let at = |i: usize| t.get(i).map(|s| s.start + bom);
        // `Placement(` の中で、`transformation(` を先に探してから `origin` を探す（`iconTransformation(origin = …)` が先でも誤らない。S288）
        let a = t.iter().position(|s| s.is("annotation"))?;
        let o = (a..t.len())
            .find(|&i| t[i].is("transformation") && t.get(i + 1).is_some_and(|s| s.is("(")))?;
        return match get(tr, "origin") {
            Some(v) => {
                let n = nums(v).filter(|n| n.len() == 2)?;
                let open = o + 1;
                let end = skip_group(t, open).ok()?;
                let k = (open..end)
                    .find(|&i| t[i].is("origin") && t.get(i + 1).is_some_and(|s| s.is("=")))?;
                let close = skip_group(t, k + 2).ok()?;
                Some((
                    at(k + 2)?,
                    at(close - 1)? + 1,
                    format!("{{{}, {}}}", coord(n[0] + dx)?, coord(n[1] + dy)?),
                ))
            }
            None => {
                let pos = at(o + 1)? + 1;
                Some((
                    pos,
                    pos,
                    format!("origin = {{{}, {}}}, ", coord(dx)?, coord(dy)?),
                ))
            }
        };
    }
    None
}

/// クラス `qualified` に `connect(a, b) annotation(Line(points = {…}, color = {0, 0, 255}));` を足す編集（S261）。
/// 返すのは `src` のバイト範囲（挿入だけなので始めと終わりは同じ）と、そこへ入れる文字列。
/// 最後の `equation` 節の末尾（クラスの `annotation` の前）に、その節の字下げで足す。`equation` が無ければ、クラスの `annotation`（無ければ `end`）の行の前に
/// `equation` 節ごと足す（`equation` は `end` の字下げ、文は最初の宣言の字下げ）。`a`・`b` は `part.connector`（`名前` か `名前[数]` を `.` でつないだ形。座標は有限で絶対値が 1e6 以下、でなければ None。S288）。
/// `points` は図の座標の線（空なら `annotation` を付けない）。クラスが無い・端が不正・挿入位置の行に他の文がある（1 行に詰めた）ときは None。
pub fn add_connect(
    src: &str,
    qualified: &str,
    a: &str,
    b: &str,
    points: &[[f64; 2]],
) -> Option<(usize, usize, String)> {
    if !end_ok(a) || !end_ok(b) {
        return None;
    }
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let name_of = qualified.rsplit('.').next().unwrap_or(qualified);
    let mine = own_elements(&tokens, qualified)?;
    let line = if points.is_empty() {
        String::new()
    } else {
        let mut pts = Vec::new();
        for p in points {
            pts.push(format!("{{{}, {}}}", coord(p[0])?, coord(p[1])?));
        }
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

/// 部品 `part` の宣言と、その部品につながる `connect(...)` の方程式をまとめて消す編集（S281）。返すのは `src` のバイト範囲と置き換える文字列（空）の組で、
/// 重ならず、先頭から並ぶ。要素（文と同じ行の後ろのコメント）だけが行にあれば、字下げと行末の改行まで消し、他の文が同じ行にあれば要素だけを消す。
/// 宣言が `A a, b;` のように複数の名前を持つとき・Placement の無い宣言（図に出ない）・クラスや部品が無いときは None。`if` の中の `connect` は数えない。
pub fn delete_part(src: &str, qualified: &str, part: &str) -> Option<Vec<(usize, usize, String)>> {
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let mine = own_elements(&tokens, qualified)?;
    let decl = mine.iter().find(|t| {
        !t[0].is_comment() && declarations(t, body).iter().any(|(r, _)| r.name == part)
    })?;
    let mut i = 0;
    while i < decl.len() {
        if ["(", "[", "{"].iter().any(|b| decl[i].is(b)) {
            i = skip_group(decl, i).ok()?;
            continue;
        }
        if decl[i].is(",") {
            return None; // 複数の名前（`A a, b;`）
        }
        i += 1;
    }
    let hit = |t: &&&[Token<'_>]| {
        std::ptr::eq(**t, *decl)
            || (t[0].is("connect")
                && connect_ends(t).is_some_and(|(a, b)| first(&a) == part || first(&b) == part))
    };
    Some(erase(body, bom, mine.iter().filter(hit).copied()))
}

/// `connect(a, b)` の方程式（`annotation` と同じ行の後ろのコメントを含む文全体）を消す編集（S281）。端は `Connection` の `from`・`to` と同じ字句で、
/// 逆の並びも同じ結線とみなす（添字を含めて比べる。`b[2].p` と `b[ 2 ].p` は同じ、`b[1].p` は別。S283）。同じ端の `connect` が複数あれば全部消す。無ければ None。範囲の決め方は [`delete_part`] と同じ。
pub fn delete_connect(
    src: &str,
    qualified: &str,
    a: &str,
    b: &str,
) -> Option<Vec<(usize, usize, String)>> {
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let mine = own_elements(&tokens, qualified)?;
    let hit: Vec<_> = mine
        .iter()
        .filter(|t| {
            t[0].is("connect")
                && connect_ends(t).is_some_and(|(x, y)| (x == a && y == b) || (x == b && y == a))
        })
        .copied()
        .collect();
    (!hit.is_empty()).then(|| erase(body, bom, hit.into_iter()))
}

/// `connect(a, b)`（逆の並びも同じ）の `Line(points = {…})` の途中の点 `index`（1 以上、最後の点の手前。端は部品のコネクタに付くので動かさない）を (dx, dy) だけ動かす編集（S299）。
/// 返すのは `src` のバイト範囲（その点の `{x, y}`）と入れる文字列。結線が無い・`points` が無い・`index` が端か範囲外・座標が不正（[`coord`]）なら None。ソースの他の所には触らない。
pub fn move_waypoint(
    src: &str,
    qualified: &str,
    a: &str,
    b: &str,
    index: usize,
    dx: f64,
    dy: f64,
) -> Option<(usize, usize, String)> {
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let tokens = lex(&src[bom..]).ok()?;
    let mine = own_elements(&tokens, qualified)?;
    let (ranges, pts) = waypoints(connect_of(&mine, a, b)?)?;
    if index == 0 || index + 1 >= pts.len() {
        return None;
    }
    let p = pts[index];
    Some((
        ranges[index].0 + bom,
        ranges[index].1 + bom,
        format!("{{{}, {}}}", coord(p[0] + dx)?, coord(p[1] + dy)?),
    ))
}

/// `connect(a, b)` の線に途中の点 `at` を足す編集（S299）。`Line(points)` があれば `index`（1 以上、点の数以下）番目の点の前に挿す（`index` は線の線分の番号 + 1）。
/// `points` が無い（中心どうしの直線として描かれている）結線には、`ends`（始めと終わり。UI はコネクタの位置を渡す）と `at` の 3 点の `Line(points = …)` を足す
/// （`Line(` があればその直後、`annotation(` だけならその直後、無ければ末尾に `annotation(Line(points = …, color = {0, 0, 255}))` を足す。`index` は見ない。`ends` が None なら断る）。結線が無い・座標が不正・`points` が読めない（`index` が範囲外を含む）なら None。
pub fn add_waypoint(
    src: &str,
    qualified: &str,
    a: &str,
    b: &str,
    index: usize,
    at: [f64; 2],
    ends: Option<[[f64; 2]; 2]>,
) -> Option<(usize, usize, String)> {
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let tokens = lex(&src[bom..]).ok()?;
    let mine = own_elements(&tokens, qualified)?;
    let t = connect_of(&mine, a, b)?;
    let pt = |p: [f64; 2]| Some(format!("{{{}, {}}}", coord(p[0])?, coord(p[1])?));
    let mid = pt(at)?;
    let line = line_at(t);
    if line.is_some_and(|(o, c)| (o..c).any(|i| t[i].is("points"))) {
        let (ranges, pts) = waypoints(t)?;
        if index == 0 || index > pts.len().saturating_sub(1) {
            return None;
        }
        let s = ranges[index].0 + bom;
        return Some((s, s, format!("{mid}, ")));
    }
    let ends = ends?; // points が無いのに端が渡されなければ断る
    let list = format!("{{{}, {mid}, {}}}", pt(ends[0])?, pt(ends[1])?);
    let after_open = |o: usize, text: String| {
        let comma = if t.get(o + 1).is_some_and(|s| s.is(")")) {
            ""
        } else {
            ", "
        };
        let pos = t[o].start + 1 + bom;
        Some((pos, pos, format!("{text}{comma}")))
    };
    if let Some((o, _)) = line {
        return after_open(o, format!("points = {list}"));
    }
    let ann = t.iter().position(|s| s.is("annotation"));
    if let Some(i) = ann.filter(|&i| t.get(i + 1).is_some_and(|s| s.is("("))) {
        return after_open(i + 1, format!("Line(points = {list})"));
    }
    if ann.is_some() {
        return None;
    }
    // 文末の `;` の前（要素に `;` が入っていれば）
    let last = t.last()?;
    let end = if last.is(";") {
        t.len() - 2
    } else {
        t.len() - 1
    };
    let e = t.get(end)?;
    let pos = e.start + e.text.len() + bom;
    Some((
        pos,
        pos,
        format!(" annotation(Line(points = {list}, color = {{0, 0, 255}}))"),
    ))
}

/// 要素の中の `connect(a, b)`（逆の並びも同じ。添字を含めて比べる。最初の 1 本）。
fn connect_of<'t, 'a>(mine: &[&'t [Token<'a>]], a: &str, b: &str) -> Option<&'t [Token<'a>]> {
    mine.iter().copied().find(|t| {
        t[0].is("connect")
            && connect_ends(t).is_some_and(|(x, y)| (x == a && y == b) || (x == b && y == a))
    })
}

/// `annotation(…Line(…)…)` の `Line` の `(` と、閉じの次のトークンの位置。
fn line_at(t: &[Token<'_>]) -> Option<(usize, usize)> {
    let p = t.iter().position(|s| s.is("annotation"))?;
    let l = (p..t.len()).find(|&i| t[i].is("Line") && t.get(i + 1).is_some_and(|s| s.is("(")))?;
    Some((l + 1, skip_group(t, l + 1).ok()?))
}

/// 各点のバイト範囲と座標
type Waypoints = (Vec<(usize, usize)>, Vec<[f64; 2]>);

/// `Line(points = {{x, y}, …})` の各点の `{x, y}` のバイト範囲と座標。`points` が無い・読めないときは None。
fn waypoints(t: &[Token<'_>]) -> Option<Waypoints> {
    let (o, c) = line_at(t)?;
    let k = (o..c).find(|&i| t[i].is("points") && t.get(i + 1).is_some_and(|s| s.is("=")))?;
    let end = skip_group(t, k + 2).ok()?;
    let (mut ranges, mut i) = (Vec::new(), k + 3);
    while i + 1 < end {
        if t[i].is("{") {
            let j = skip_group(t, i).ok()?;
            ranges.push((t[i].start, t[j - 1].start + 1));
            i = j;
        } else {
            i += 1;
        }
    }
    let Some(Val::Call(_, args)) = annotation_in(t, "Line") else {
        return None;
    };
    let Some(Val::List(items)) = get(&args, "points") else {
        return None;
    };
    let pts: Vec<[f64; 2]> = items
        .iter()
        .map(|v| nums(v).filter(|n| n.len() == 2).map(|n| [n[0], n[1]]))
        .collect::<Option<_>>()?;
    (pts.len() == ranges.len()).then_some((ranges, pts))
}

/// 要素（トークンの並び）を消すバイト範囲。行に他に何も無ければ行ごと（行末の改行まで）、あれば要素と後ろの空白だけ。
fn erase<'t, 'a: 't>(
    body: &str,
    bom: usize,
    elems: impl Iterator<Item = &'t [Token<'a>]>,
) -> Vec<(usize, usize, String)> {
    elems
        .map(|t| {
            let (s, e) = (t[0].start, t.last().map_or(0, |l| l.start + l.text.len()));
            let ls = body[..s].rfind('\n').map_or(0, |i| i + 1);
            let le = body[e..].find('\n').map_or(body.len(), |i| e + i);
            let alone = body[ls..s].trim().is_empty() && body[e..le].trim().is_empty();
            let (from, to) = if alone {
                (ls, (le + 1).min(body.len()))
            } else {
                let sp = body[e..].len() - body[e..].trim_start_matches([' ', '\t']).len();
                (s, e + sp)
            };
            (from + bom, to + bom, String::new())
        })
        .collect()
}

/// クラス直下の要素（入れ子のクラスの中は除く）。クラスが無ければ None。
fn own_elements<'t, 'a>(tokens: &'t [Token<'a>], qualified: &str) -> Option<Vec<&'t [Token<'a>]>> {
    let within = within(tokens);
    let spans = class_spans(tokens).ok()?;
    let own = spans.iter().find(|s| join(&within, &s.path) == qualified)?;
    let mut mine = Vec::new();
    for e in split(tokens, false).ok()? {
        let st = e.tokens[0].start;
        let inner = spans
            .iter()
            .filter(|s| s.start <= st && st < s.end)
            .max_by_key(|s| s.start);
        if inner.is_some_and(|o| o.path == own.path) && !spans.iter().any(|s| s.start == st) {
            mine.push(e.tokens);
        }
    }
    Some(mine)
}

/// クラス `qualified` に部品 `<ty> <名前> annotation(Placement(transformation(origin = {x, y}, extent = {{-10, -10}, {10, 10}})));` の宣言を足す編集（S265）。
/// 返すのは `src` のバイト範囲（挿入だけ）と、そこへ入れる文字列。最後の public の宣言の後（`protected`・`equation` などの節の前）に、その字下げで足す。
/// 宣言が無ければ、最初の節かクラスの `annotation`（無ければ `end`）の行の前に足す。名前は型の最後の部分の頭を小文字にしたもので、
/// クラスの中の字句と重なる（キーワードを含む）ときは `1`、`2`… を付ける。`origin` は 2 単位に吸着。
/// 型が `package`・`function`・`record`・`type` と分かれば None（`library` で引けない・種類が読めなければ受ける）。型が名前でない・クラスが無い・挿入位置の行に他の文がある（1 行に詰めた）ときも None。
pub fn add_part(
    src: &str,
    qualified: &str,
    ty: &str,
    origin: [f64; 2],
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<(usize, usize, String)> {
    let ident = |w: &str| {
        w.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !ty.split('.').all(ident) {
        return None;
    }
    if matches!(
        kind_of(ty, "", &[], &[], src, library),
        Some("package" | "function" | "record" | "type")
    ) {
        return None;
    }
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).ok()?;
    let name_of = qualified.rsplit('.').next().unwrap_or(qualified);
    let mine = own_elements(&tokens, qualified)?;
    let last = ty.rsplit('.').next()?;
    let base = format!("{}{}", last[..1].to_lowercase(), &last[1..]);
    let taken = |n: &str| {
        n == last
            || crate::KEYWORDS.contains(&n)
            || mine
                .iter()
                .flat_map(|t| t.iter())
                .any(|t| t.kind == Kind::Ident && t.text == n)
    };
    let name = if taken(&base) {
        (1..).map(|i| format!("{base}{i}")).find(|n| !taken(n))?
    } else {
        base
    };
    let snap = |v: f64| coord((v / 2.0).round() * 2.0 + 0.0);
    let decl = format!(
        "{ty} {name} annotation(Placement(transformation(origin = {{{}, {}}}, extent = {{{{-10, -10}}, {{10, 10}}}})));",
        snap(origin[0])?,
        snap(origin[1])?
    );
    let indent_of = |pos: usize| {
        let ls = body[..pos].rfind('\n').map_or(0, |i| i + 1);
        body[ls..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>()
    };
    let is_end = |t: &[Token<'_>]| t[0].is("end") && t.get(1).is_some_and(|n| n.is(name_of));
    let stop = |t: &[Token<'_>]| {
        [
            "protected",
            "equation",
            "algorithm",
            "initial",
            "annotation",
        ]
        .iter()
        .any(|k| t[0].is(k))
            || is_end(t)
    };
    let decls: Vec<_> = mine.iter().take_while(|t| !stop(t)).collect();
    if let Some(l) = decls.last() {
        let indent = decls
            .iter()
            .rev()
            .find(|t| !t[0].is_comment() && !t[0].is("public"))
            .map_or_else(
                || decl_indent(&mine, body, name_of),
                |t| indent_of(t[0].start),
            );
        let end = l.last()?;
        let pos = end.start + end.text.len() + bom;
        return Some((pos, pos, format!("\n{indent}{decl}")));
    }
    let target = mine.get(decls.len())?;
    let ls = body[..target[0].start].rfind('\n').map_or(0, |i| i + 1);
    if !body[ls..target[0].start].trim().is_empty() {
        return None; // 同じ行に他の文がある
    }
    let end = mine.iter().find(|t| is_end(t))?;
    let indent = format!("{}  ", indent_of(end[0].start));
    Some((ls + bom, ls + bom, format!("{indent}{decl}\n")))
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

/// 部品 `part` の宣言の修飾 `(name = value)` を足す・書き換える・外す編集（S294）。返すのは `src` のバイト範囲と、そこへ入れる文字列。
/// 修飾が無ければ名前（と配列の添字）の直後に `(name = value)` を、`(…)` があれば末尾に `, name = value` を足し、`name` の引数があれば `=` の右だけ置き換える。
/// `value` が空なら `name` の束縛を外す（引数が他に何も持たなければ引数ごと、`(…)` が空になれば括弧ごと）。何も変わらないなら範囲が空の編集を返す。
/// 断る理由（Err）: 入れ子の名前（`a.b`）、`each`・`final` つきの引数、`name.x = …` のような入れ子の引数がある、式が壊れている、クラス・部品が無い。
/// 式の検査は字句と括弧の対応だけ（型や名前は見ない。fmt は依存 0 なので構文の木は持たない）。複数の名前（`A a, b;`）は名前ごとの区切りだけを見る。
pub fn set_modifier(
    src: &str,
    qualified: &str,
    part: &str,
    name: &str,
    value: &str,
) -> Result<(usize, usize, String), String> {
    let ident = |s: &str| {
        lex(s).is_ok_and(|t| t.len() == 1 && t[0].kind == Kind::Ident && !t[0].is_keyword())
    };
    if name.contains('.') {
        return Err(format!(
            "入れ子の名前 `{name}` の修飾は書けない（部品の直下の 1 段だけ）"
        ));
    }
    if !ident(part) || !ident(name) {
        return Err(format!("`{part}`・`{name}` は識別子でない"));
    }
    let value = value.trim();
    if !value.is_empty() {
        expr_ok(value)?;
    }
    let bom = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &src[bom..];
    let tokens = lex(body).map_err(|e| e.to_string())?;
    let mine = own_elements(&tokens, qualified)
        .ok_or(format!("クラス `{qualified}` がこのファイルに無い"))?;
    let (t, n) = mine
        .iter()
        .filter(|t| !t[0].is_comment())
        .find_map(|t| site(t, part))
        .ok_or(format!("部品 `{part}` の宣言が `{qualified}` に無い"))?;
    let at = |i: usize| t[i].start + bom;
    let end = |i: usize| t[i].start + t[i].text.len() + bom;
    let group = |i: usize| skip_group(t, i).map_err(|e| e.to_string());
    let opener = |i: usize| ["(", "[", "{"].iter().any(|b| t[i].is(b));
    let mut j = n + 1;
    if t.get(j).is_some_and(|s| s.is("[")) {
        j = group(j)?;
    }
    if !t.get(j).is_some_and(|s| s.is("(")) {
        let text = if value.is_empty() {
            String::new()
        } else {
            format!("({name} = {value})")
        };
        return Ok((end(j - 1), end(j - 1), text));
    }
    let close = group(j)? - 1;
    // 引数ごとの [from, to)（コメントは両端から外す）
    let (mut args, mut from, mut i) = (Vec::new(), j + 1, j + 1);
    while i < close {
        if opener(i) {
            i = group(i)?;
            continue;
        }
        if t[i].is(",") {
            args.push((from, i));
            from = i + 1;
        }
        i += 1;
    }
    args.push((from, close));
    let args: Vec<(usize, usize)> = args
        .into_iter()
        .filter_map(|(a, b)| {
            let c: Vec<usize> = (a..b).filter(|&k| !t[k].is_comment()).collect();
            Some((*c.first()?, c.last()? + 1))
        })
        .collect();
    let mut found = None;
    for (k, &(a, b)) in args.iter().enumerate() {
        let mut i = a;
        while i < b && (t[i].is("each") || t[i].is("final")) {
            i += 1;
        }
        if i < b && t[i].is(name) {
            if t.get(i + 1).is_some_and(|s| s.is(".")) {
                return Err(format!(
                    "`{name}` の入れ子の引数（`{name}.x = …`）があるので書けない"
                ));
            }
            if i > a {
                return Err(format!("`each`・`final` つきの `{name}` は書き換えない"));
            }
            found = Some((k, i));
        }
    }
    let Some((k, i)) = found else {
        return Ok(match (value.is_empty(), args.last()) {
            (true, _) => (end(close), end(close), String::new()),
            (_, None) => (end(j), end(j), format!("{name} = {value}")),
            (_, Some(&(_, b))) => (end(b - 1), end(b - 1), format!(", {name} = {value}")),
        });
    };
    let (a, b) = args[k];
    let mut eq = i + 1;
    while eq < b && !t[eq].is("=") {
        eq = if opener(eq) { group(eq)? } else { eq + 1 };
    }
    if eq >= b {
        let text = if value.is_empty() {
            String::new()
        } else {
            format!(" = {value}")
        };
        return Ok((end(b - 1), end(b - 1), text));
    }
    // 式の終わり: 後ろに説明の文字列（`k = 1 "説明"`）があれば手前まで
    let desc = t[b - 1].kind == Kind::Str
        && b - 1 > eq + 1
        && (t[b - 2].kind != Kind::Punct || [")", "]", "}"].iter().any(|c| t[b - 2].is(c)));
    let stop = end(if desc { b - 2 } else { b - 1 });
    if !value.is_empty() {
        return Ok((at(eq + 1), stop, value.to_string()));
    }
    if eq > i + 1 {
        return Ok((end(eq - 1), stop, String::new())); // `k(min = 0) = 1` は ` = 1` だけ外す
    }
    Ok(match (k, args.len()) {
        (_, 1) => (at(j), end(close), String::new()),
        (0, _) => (at(a), at(args[1].0), String::new()),
        _ => (end(args[k - 1].1 - 1), end(b - 1), String::new()),
    })
}

/// 要素の `part` の名前の区切りと、その中の名前のトークンの位置。
fn site<'t, 'a>(t: &'t [Token<'a>], part: &str) -> Option<(&'t [Token<'a>], usize)> {
    let (_, first) = head(t)?;
    comma_split(t).into_iter().enumerate().find_map(|(k, p)| {
        let i = if k == 0 {
            p.iter().position(|s| s.start == first.start)?
        } else {
            0
        };
        (p.get(i)?.kind == Kind::Ident && p[i].text == part).then_some((p, i))
    })
}

/// 修飾に入れる式の簡単な検査: 字句に分けられ、コメント・`;`・`=`・深さ 0 の `,` が無く、括弧が対応する。
fn expr_ok(v: &str) -> Result<(), String> {
    let bad = || format!("式 `{v}` が壊れている");
    let t = lex(v).map_err(|_| bad())?;
    let mut i = 0;
    while i < t.len() {
        if ["(", "[", "{"].iter().any(|b| t[i].is(b)) {
            i = skip_group(&t, i).map_err(|_| bad())?;
            continue;
        }
        if t[i].is_comment() || [")", "]", "}", ",", ";", "="].iter().any(|c| t[i].is(c)) {
            return Err(bad());
        }
        i += 1;
    }
    Ok(())
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

/// `connect(a.p, b[1].n)` の 2 つの端（`a.p`・`b[1].n`。添字を含め、書いてあるままの字句を空白なしでつなぐ。S283）。
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
            let j = skip_group(t, i).ok()?;
            t[i..j].iter().for_each(|s| ends[k].push_str(s.text)); // 添字も端の一部（空白は落ちる）
            i = j;
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

/// 座標を Modelica のソースに入れる字にする（小数 3 桁）。有限でなく、絶対値が 1e6 を超えるものは None（`inf` を書かない。S288）。
/// LSP のどの編集（`movePart`・`addConnect`・`addPart`）もここを通るので、クライアントの検査に頼らない。
fn coord(v: f64) -> Option<String> {
    (v.is_finite() && v.abs() <= 1e6).then(|| ((v * 1000.0).round() / 1000.0 + 0.0).to_string())
}

/// `connect` の端の形: `名前` か `名前[数]` を `.` でつないだもの（`a.p`・`b[2].n`）。字の種類でなく形を見る（S288）。
fn end_ok(e: &str) -> bool {
    let name = |w: &str| {
        let (id, idx) = w.split_once('[').map_or((w, None), |(a, b)| (a, Some(b)));
        let id_ok = id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        id_ok
            && idx.is_none_or(|i| {
                i.strip_suffix(']').is_some_and(|n| {
                    !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == ',')
                })
            })
    };
    e.split('.').all(name)
}

/// 宣言の要素を、トップレベルの `,` で名前ごとの区切りに割る（`A a annotation(…), b annotation(…)`。括弧の中は割らない）。
fn comma_split<'t, 'a>(t: &'t [Token<'a>]) -> Vec<&'t [Token<'a>]> {
    let (mut out, mut from, mut i) = (Vec::new(), 0, 0);
    while i < t.len() {
        if ["(", "[", "{"].iter().any(|b| t[i].is(b)) {
            let Ok(j) = skip_group(t, i) else { break };
            i = j;
            continue;
        }
        if t[i].is(",") {
            out.push(&t[from..i]);
            from = i + 1;
        }
        i += 1;
    }
    out.push(&t[from..]);
    out
}

/// 要素の部品の宣言（`型 名前 … annotation(Placement(transformation(…)))`）。名前が複数あれば、それぞれの区切りの `annotation` を読む（S288）。
/// Placement が無い・隠す・壊れているものは含めない。区切りのトークン列も返す。
fn declarations<'t, 'a>(t: &'t [Token<'a>], body: &str) -> Vec<(Raw, &'t [Token<'a>])> {
    let Some((ty, first_name)) = head(t) else {
        return Vec::new();
    };
    comma_split(t)
        .into_iter()
        .enumerate()
        .filter_map(|(k, piece)| {
            let name = if k == 0 {
                first_name
            } else {
                piece
                    .first()
                    .filter(|n| n.kind == Kind::Ident && !n.is_keyword())?
            };
            Some((placement(piece, &ty, name, body)?, piece))
        })
        .collect()
}

fn placement(t: &[Token<'_>], ty: &str, name: &Token<'_>, body: &str) -> Option<Raw> {
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
        ty: ty.to_string(),
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

/// 型 `ty` のクラスが `connector` か。引けなければ None。
fn is_connector(
    ty: &str,
    class: &str,
    aliases: &[(String, String)],
    wild: &[String],
    src: &str,
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<bool> {
    kind_of(ty, class, aliases, wild, src, library).map(|k| k == "connector")
}

/// 型 `ty` のクラスの種類（`connector`・`model`・`block`・`record`・`class`・`type`・`package`・`function`）。引けなければ None。
/// ファイルを字句解析せず、`<種類> <名前>` の字面を探すだけ（大きなライブラリの本文を何度も読むので）。
fn kind_of(
    ty: &str,
    class: &str,
    aliases: &[(String, String)],
    wild: &[String],
    src: &str,
    library: &dyn Fn(&str) -> Option<String>,
) -> Option<&'static str> {
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
                [
                    "connector",
                    "model",
                    "block",
                    "record",
                    "class",
                    "type",
                    "package",
                    "function",
                ]
                .into_iter()
                .find(|k| *k == kind)
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
        assert_eq!((c.from.as_str(), c.to.as_str()), ("r1.n", "r2[1].p"));
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
    fn move_part_edits_the_transformation_origin_even_when_icon_transformation_comes_first() {
        // S288 (#98): iconTransformation が先でも、書き換えるのは transformation の origin
        let src = "model M\n  Foo f annotation(Placement(iconTransformation(origin = {0, 0}, extent = {{-10, -10}, {10, 10}}), transformation(origin = {10, 20}, extent = {{-10, -10}, {10, 10}})));\nend M;\n";
        let (a, b, t) = move_part(src, "M", "f", 5.0, 5.0).unwrap();
        assert_eq!((&src[a..b], t.as_str()), ("{10, 20}", "{15, 25}"));
    }

    #[test]
    fn several_names_in_one_declaration_each_read_their_own_annotation() {
        // S288 (#99): `Foo a annotation(…), b annotation(…)` は a・b の 2 部品。annotation が最後の名前だけにあれば a には Placement が無い
        let pl = |x: i32| {
            format!(
                "annotation(Placement(transformation(origin = {{{x}, 0}}, extent = {{{{-10, -10}}, {{10, 10}}}})))"
            )
        };
        let src = format!("model M\n  Foo a {}, b {};\nend M;\n", pl(0), pl(40));
        let d = diagram(&src, "M", &|_| None).unwrap();
        let names: Vec<_> = d
            .parts
            .iter()
            .map(|p| (p.name.as_str(), p.bbox[0]))
            .collect();
        assert_eq!(names, [("a", -10.0), ("b", 30.0)]);
        let (a, b, t) = move_part(&src, "M", "b", 2.0, 0.0).unwrap();
        assert_eq!((&src[a..b], t.as_str()), ("{40, 0}", "{42, 0}"));
        let last = format!("model M\n  Foo a, b {};\nend M;\n", pl(40));
        let names: Vec<_> = diagram(&last, "M", &|_| None)
            .unwrap()
            .parts
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["b"]);
    }

    #[test]
    fn edits_refuse_malformed_ends_and_non_finite_or_huge_coordinates() {
        // S288 (#100): 端は成分参照の形、座標は有限で 1e6 以下（`inf` や構文エラーのソースを作る編集を返さない）
        let src = "model M\n  Foo a;\n  Foo b;\nequation\n  connect(a.p, b.n);\nend M;\n";
        assert!(add_connect(src, "M", "a[", "]b..", &[]).is_none());
        assert!(add_connect(src, "M", "a.", "b", &[]).is_none());
        assert!(add_connect(src, "M", "a[1].p", "b.n", &[]).is_some());
        assert!(add_connect(src, "M", "a.p", "b.n", &[[1e306, 0.0]]).is_none());
        let s = "model M\n  Foo a(k = 1) if cond annotation(Placement(transformation(origin={1,2})));\nend M;\n";
        assert!(move_part(s, "M", "a", 1e306, 0.0).is_none());
        assert!(move_part(s, "M", "a", 1.0, 0.0).is_some());
        assert!(add_part(src, "M", "Foo", [f64::INFINITY, 0.0], &|_| None).is_none());
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

    #[test]
    fn add_part_goes_after_the_last_public_declaration() {
        let lib = |c: &str| match c {
            "L.Res" => Some("within L;\nmodel Res\nend Res;\n".to_string()),
            "L.Fn" => Some("within L;\nfunction Fn\nend Fn;\n".to_string()),
            _ => None,
        };
        let apply = |src: &str, class: &str, ty: &str| {
            let (a, b, t) = add_part(src, class, ty, [13.0, -7.0], &lib)?;
            Some(format!("{}{}{}", &src[..a], t, &src[b..]))
        };
        let decl = "L.Res res annotation(Placement(transformation(origin = {14, -8}, extent = {{-10, -10}, {10, 10}})));";
        // 宣言の後（protected の前）、最後の宣言の字下げで。末尾のコメントの後ろ
        let with = "model M\n    Real x; // 値\n    Real y;\nprotected\n  Real z;\nequation\n  x = 1;\nend M;\n";
        assert_eq!(
            apply(with, "M", "L.Res").unwrap(),
            format!(
                "model M\n    Real x; // 値\n    Real y;\n    {decl}\nprotected\n  Real z;\nequation\n  x = 1;\nend M;\n"
            )
        );
        // 宣言が無い: 最初の節の前（end の字下げ + 2）。入れ子のクラスは自分の end
        let none = "package P\n  model M\n  equation\n    x = 1;\n  end M;\n  model N\n    Real a;\n  end N;\nend P;\n";
        assert_eq!(
            apply(none, "P.M", "L.Res").unwrap(),
            format!(
                "package P\n  model M\n    {decl}\n  equation\n    x = 1;\n  end M;\n  model N\n    Real a;\n  end N;\nend P;\n"
            )
        );
        // 名前の重複: res（宣言とキーワードでない字句）→ res1 → res2
        let dup = "model M\n  L.Res res;\n  L.Res res1;\nend M;\n";
        assert!(
            apply(dup, "M", "L.Res")
                .unwrap()
                .contains("\n  L.Res res2 annotation")
        );
        // 種類: function は断る、引けなければ受ける
        assert!(apply(with, "M", "L.Fn").is_none());
        assert!(
            apply(with, "M", "Other.Thing")
                .unwrap()
                .contains("Other.Thing thing annotation")
        );
        // 不正な型・クラス無し
        assert!(apply(with, "M", "L.Res; x").is_none());
        assert!(apply(with, "Nope", "L.Res").is_none());
    }

    #[test]
    fn delete_part_removes_the_declaration_and_its_connects() {
        let apply = |src: &str, edits: Vec<(usize, usize, String)>| {
            let mut out = src.to_string();
            for (a, b, t) in edits.into_iter().rev() {
                out.replace_range(a..b, &t);
            }
            out
        };
        let src = "model M\n  Box a annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\n  Box b[2] annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\n  Box c annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\nequation\n  connect(a.p, b[1].n) annotation(Line(points = {{0, 0}, {1, 1}})); // a-b\n  connect(b[2].p, c.n);\n    connect(c.p, a.n);\n  x = 1;\nend M;\n";
        let got = apply(src, delete_part(src, "M", "a").unwrap());
        assert_eq!(
            got,
            "model M\n  Box b[2] annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\n  Box c annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\nequation\n  connect(b[2].p, c.n);\n  x = 1;\nend M;\n"
        );
        // 配列の部品（b[1]）は b の名前で数える
        let got = apply(src, delete_part(src, "M", "b").unwrap());
        assert!(
            !got.contains("b[") && got.contains("connect(c.p, a.n);"),
            "{got}"
        );
        // 結線だけ: 逆の並びでも消え、字下げは他の行を壊さない
        let got = apply(src, delete_connect(src, "M", "c.n", "b[2].p").unwrap());
        assert_eq!(got, src.replace("  connect(b[2].p, c.n);\n", ""));
        assert!(delete_connect(src, "M", "a.p", "c.p").is_none());
        assert!(
            delete_connect(src, "M", "c.n", "b.p").is_none(),
            "添字の無い端は別物"
        );
        // 同じ行に他の文があれば、その要素だけ
        let packed = "model M\n  Box a annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\nequation\n  x = 1; connect(a.p, a.n);  y = 2;\nend M;\n";
        let got = apply(packed, delete_connect(packed, "M", "a.p", "a.n").unwrap());
        assert_eq!(
            got,
            "model M\n  Box a annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\nequation\n  x = 1; y = 2;\nend M;\n"
        );
        // 複数の名前・Placement 無し・無い部品・BOM
        let multi = "model M\n  Box a, b annotation(Placement(transformation(extent = {{0, 0}, {1, 1}})));\nend M;\n";
        assert!(delete_part(multi, "M", "a").is_none());
        assert!(delete_part(src, "M", "zzz").is_none());
        assert!(delete_part(src, "N", "a").is_none());
        let bom = format!("\u{feff}{src}");
        let got = apply(&bom, delete_connect(&bom, "M", "b[2].p", "c.n").unwrap());
        assert_eq!(got, bom.replace("  connect(b[2].p, c.n);\n", ""));
    }

    #[test]
    fn delete_connect_tells_array_indices_apart() {
        let src = "model M\n  Box b[3];\n  Box c;\nequation\n  connect(b[1].p, c.n);\n  connect(b[ 2 ].p, c.n);\n  connect(c.n, b[i + 1].p);\nend M;\n";
        let del = |a: &str, b: &str| {
            let mut out = src.to_string();
            for (s, e, t) in delete_connect(src, "M", a, b).unwrap().into_iter().rev() {
                out.replace_range(s..e, &t);
            }
            out
        };
        // 1 本だけ消える（空白の違いは同じ、逆の並びも同じ）
        assert_eq!(
            del("b[2].p", "c.n"),
            src.replace("  connect(b[ 2 ].p, c.n);\n", "")
        );
        assert_eq!(
            del("c.n", "b[1].p"),
            src.replace("  connect(b[1].p, c.n);\n", "")
        );
        assert_eq!(
            del("b[i+1].p", "c.n"),
            src.replace("  connect(c.n, b[i + 1].p);\n", "")
        );
        assert!(delete_connect(src, "M", "b[3].p", "c.n").is_none());
    }
    #[test]
    fn set_modifier_adds_replaces_and_removes() {
        let go = |decl: &str, name: &str, v: &str| -> Result<String, String> {
            let src = format!("model M\n  {decl}\n  Real y;\nend M;\n");
            let (a, b, t) = set_modifier(&src, "M", "r", name, v)?;
            Ok(format!("{}{}{}", &src[..a], t, &src[b..])[8..]
                .lines()
                .next()
                .unwrap()
                .trim()
                .to_string())
        };
        let ann = "annotation(Placement(transformation(origin = {0, 0})));";
        // 修飾なし: 名前の直後へ（配列の添字・複数の名前・annotation の前）
        assert_eq!(
            go(&format!("Res r {ann}"), "R", "2").unwrap(),
            format!("Res r(R = 2) {ann}")
        );
        assert_eq!(go("Res r[2];", "R", " 2 ").unwrap(), "Res r[2](R = 2);");
        assert_eq!(go("Res a, r;", "R", "2").unwrap(), "Res a, r(R = 2);");
        // 別の修飾の末尾へ・空の括弧へ・式の中の , や ( は 1 つの値
        assert_eq!(
            go("Res r(C = 1);", "R", "max({1, 2})").unwrap(),
            "Res r(C = 1, R = max({1, 2}));"
        );
        assert_eq!(go("Res r();", "R", "2").unwrap(), "Res r(R = 2);");
        // 書き換え: 前後の引数・自分の修飾・説明の文字列は残す
        assert_eq!(
            go("Res r(R = 1, C = 3);", "R", "2").unwrap(),
            "Res r(R = 2, C = 3);"
        );
        assert_eq!(
            go("Res r(C = 3, R(min = 0) = f(1, 2) \"d\");", "R", "2").unwrap(),
            "Res r(C = 3, R(min = 0) = 2 \"d\");"
        );
        // 外す: 唯一なら括弧ごと、先頭・末尾は区切りごと、自分の修飾があれば束縛だけ
        assert_eq!(go("Res r(R = 1);", "R", "").unwrap(), "Res r;");
        assert_eq!(
            go("Res r(R = 1, C = 3);", "R", "").unwrap(),
            "Res r(C = 3);"
        );
        assert_eq!(
            go("Res r(C = 3, R = 1);", "R", "").unwrap(),
            "Res r(C = 3);"
        );
        assert_eq!(
            go("Res r(R(min = 0) = 1);", "R", "").unwrap(),
            "Res r(R(min = 0));"
        );
        assert_eq!(go("Res r(C = 3);", "R", "").unwrap(), "Res r(C = 3);"); // 無ければ何も変えない
        assert_eq!(go("Res r;", "R", "").unwrap(), "Res r;");
        // 断る（理由つき）
        for (d, n, v, why) in [
            ("Res r;", "a.b", "1", "入れ子の名前"),
            ("Res r;", "R", "1 +)", "壊れている"),
            ("Res r;", "R", "1, 2", "壊れている"),
            ("Res r;", "R", "a = 1", "壊れている"),
            ("Res r;", "R", "1 // c", "壊れている"),
            ("Res r(each R = 1);", "R", "2", "each"),
            ("Res r(final R = 1);", "R", "2", "final"),
            ("Res r(R.start = 1);", "R", "2", "入れ子の引数"),
            ("Res q;", "R", "2", "部品 `r`"),
        ] {
            let e = go(d, n, v).unwrap_err();
            assert!(e.contains(why), "{d} {n} {v}: {e}");
        }
        assert!(
            set_modifier("model M end M;", "N", "r", "R", "1")
                .unwrap_err()
                .contains("クラス")
        );
        // BOM があっても範囲は元のバイト位置
        let src = "\u{feff}model M\n  Res r;\nend M;\n";
        let (a, b, t) = set_modifier(src, "M", "r", "R", "2").unwrap();
        assert_eq!(
            format!("{}{}{}", &src[..a], t, &src[b..]),
            "\u{feff}model M\n  Res r(R = 2);\nend M;\n"
        );
    }

    #[test]
    fn waypoints_move_add_and_refuse_the_ends() {
        let src = "model M\n  Box a;\n  Box b;\nequation\n  connect(a.p, b.n) annotation(Line(points = {{0, 0}, {10, 0}, {10, 5}, {20, 5}}, color = {0, 0, 255}));\n  connect(a.q, b.q);\n  connect(a.r, b.r) annotation(Line(color = {1, 2, 3}));\n  connect(a.s, b.s) annotation(Icon());\nend M;\n";
        let apply = |e: Option<(usize, usize, String)>| {
            let (a, b, t) = e?;
            Some(format!("{}{t}{}", &src[..a], &src[b..]))
        };
        // 途中の点は動き、逆の並びでも同じ結線
        let m = apply(move_waypoint(src, "M", "b.n", "a.p", 2, 1.0, -0.5)).unwrap();
        assert!(m.contains("{{0, 0}, {10, 0}, {11, 4.5}, {20, 5}}"), "{m}");
        assert!(m.contains("color = {0, 0, 255}));\n  connect(a.q"), "{m}");
        // 端・範囲外・points の無い結線・無い結線・座標が不正は断る
        for (i, a) in [(0, "a.p"), (3, "a.p"), (9, "a.p"), (1, "a.q"), (1, "x.y")] {
            assert!(
                move_waypoint(src, "M", a, "b.n", i, 1.0, 1.0).is_none(),
                "{i} {a}"
            );
        }
        assert!(move_waypoint(src, "M", "a.p", "b.n", 1, f64::NAN, 0.0).is_none());
        assert!(move_waypoint(src, "M", "a.p", "b.n", 1, 2e6, 0.0).is_none());
        // points がある結線への追加: index の前に挿す（端の前後は不可）
        let e = ([0.0, 0.0], [20.0, 5.0]);
        let add = |a: &str, b: &str, i| {
            apply(add_waypoint(
                src,
                "M",
                a,
                b,
                i,
                [5.0, 7.5],
                Some([e.0, e.1]),
            ))
        };
        assert!(
            add("a.p", "b.n", 1)
                .unwrap()
                .contains("{{0, 0}, {5, 7.5}, {10, 0}, {10, 5}, {20, 5}}")
        );
        assert!(
            add("a.p", "b.n", 3)
                .unwrap()
                .contains("{10, 5}, {5, 7.5}, {20, 5}}")
        );
        assert!(add("a.p", "b.n", 0).is_none() && add("a.p", "b.n", 4).is_none());
        // points が無い結線: annotation が無い・Line が無い・Line に points が無い
        let n = add("a.q", "b.q", 1).unwrap();
        assert!(n.contains("connect(a.q, b.q) annotation(Line(points = {{0, 0}, {5, 7.5}, {20, 5}}, color = {0, 0, 255}));"), "{n}");
        let l = add("a.r", "b.r", 1).unwrap();
        assert!(
            l.contains(
                "annotation(Line(points = {{0, 0}, {5, 7.5}, {20, 5}}, color = {1, 2, 3}));"
            ),
            "{l}"
        );
        let i = add("a.s", "b.s", 1).unwrap();
        assert!(
            i.contains("annotation(Line(points = {{0, 0}, {5, 7.5}, {20, 5}}), Icon());"),
            "{i}"
        );
        assert!(add("a.p", "x.y", 1).is_none());
        assert!(add_waypoint(src, "M", "a.q", "b.q", 1, [5.0, 7.5], None).is_none());
        // 足した結果を diagram が読み直すと、途中の点が 1 つできる
        let d = diagram(&n, "M", &|_| None).unwrap();
        assert_eq!(
            d.connections[1].line.points,
            [[0.0, 0.0], [5.0, 7.5], [20.0, 5.0]]
        );
    }
}

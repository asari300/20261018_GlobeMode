//! 図形プリミティブ（Rectangle, Ellipse, Polygon, Line, Text, Bitmap）の引数を
//! 「位置・形状 → points → fill 系 → line 系 → その他」の順に並べ替え、`origin` が無ければ足す。
//! `annotation(…)` と `Documentation(…)` の引数も決まった順に並べる（S061、[`ANNOTATION_ORDER`]）。

use crate::layout::{Arg, Node};
use crate::lexer::{Kind, Token};

const PRIMITIVES: &[&str] = &["Rectangle", "Ellipse", "Polygon", "Line", "Text", "Bitmap"];

/// グループごとの引数名。グループの番号が rank の上位、表の位置が下位になる。
const GROUPS: &[&[&str]] = &[
    &[
        "visible",
        "origin",
        "rotation",
        "extent",
        "radius",
        "smooth",
        "startAngle",
        "endAngle",
        "closure",
    ],
    &["points", "arrow", "arrowSize"],
    &["fillColor", "fillPattern"],
    &["lineColor", "pattern", "lineThickness", "borderPattern"],
];
const OTHER: u16 = (GROUPS.len() as u16 + 1) * 100;

/// `annotation(…)` の引数の順（S061。ユーザーの指定は experiment → Diagram → Documentation。Icon は Diagram の隣に置いた）。
/// 表に無いもの（uses、version など）は後ろに、元の順で。
pub const ANNOTATION_ORDER: &[&str] = &["experiment", "Diagram", "Icon", "Documentation"];
/// `Documentation(…)` の引数の順。header は OpenModelica の `__OpenModelica_infoHeader`。
pub const DOCUMENTATION_ORDER: &[&str] = &["__OpenModelica_infoHeader", "info", "revisions"];

/// 直前の名前がこれなら、続く `(…)` の引数をその表の順に並べる。
fn order_table(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "annotation" => Some(ANNOTATION_ORDER),
        "Documentation" => Some(DOCUMENTATION_ORDER),
        _ => None,
    }
}

/// 引数の先頭の名前（`info = …` の info、`experiment(…)` の experiment）の、表での位置。無ければ表の長さ（後ろ）。
fn position_in(table: &[&str], a: &Arg<'_>) -> usize {
    match a.nodes.first() {
        Some(Node::Leaf(t)) if t.kind == Kind::Ident => table
            .iter()
            .position(|n| *n == t.text)
            .unwrap_or(table.len()),
        _ => table.len(),
    }
}

/// 並べ替えが要る `annotation(…)` / `Documentation(…)` を探す（lint の `annotation-order`）。
/// 見つけたら (順を外れた最初の引数の先頭のバイト位置, 表の名前) を足す。
pub fn misordered(nodes: &[Node<'_>], in_annotation: bool, out: &mut Vec<(usize, &'static str)>) {
    let mut prev: Option<&str> = None;
    for n in nodes {
        match n {
            Node::Leaf(t) => prev = (t.kind == Kind::Ident).then_some(t.text),
            Node::Group { open, args, .. } => {
                if let (Some(name), true) = (prev, open.is("("))
                    && (in_annotation || name == "annotation")
                    && let Some(table) = order_table(name)
                {
                    let pos: Vec<usize> = args.iter().map(|a| position_in(table, a)).collect();
                    if let Some(k) = (1..pos.len()).find(|&k| pos[k] < pos[k - 1])
                        && let Some(Node::Leaf(t)) = args[k].nodes.first()
                    {
                        let table = if name == "annotation" {
                            "annotation"
                        } else {
                            "Documentation"
                        };
                        out.push((t.start, table));
                    }
                }
                let inside = in_annotation || (prev == Some("annotation") && open.is("("));
                prev = None;
                for a in args {
                    misordered(&a.nodes, inside, out);
                }
            }
        }
    }
}

/// `in_annotation` は `annotation(…)` の中か。外の同じ名前（`extends Pkg.Line(R = 1)` や関数呼び出し）は触らない（S211）。
pub fn apply(nodes: &mut [Node<'_>], insert_origin: bool, format_html: bool, in_annotation: bool) {
    let mut prev: Option<&str> = None;
    for n in nodes.iter_mut() {
        match n {
            Node::Leaf(t) => prev = (t.kind == Kind::Ident).then_some(t.text),
            Node::Group { open, args, .. } => {
                let inside = in_annotation || (prev == Some("annotation") && open.is("("));
                if let (Some(name), true, true) = (prev, open.is("("), inside) {
                    if PRIMITIVES.contains(&name) {
                        sort_args(args, insert_origin);
                    } else if let Some(table) = order_table(name) {
                        // 安定な並べ替え。rank は 0 のまま（グループの境目の改行は図形プリミティブだけ）
                        args.sort_by_key(|a| position_in(table, a));
                        let n = args.len();
                        for (k, a) in args.iter_mut().enumerate() {
                            a.sep = (k + 1 < n).then_some(",");
                        }
                        if format_html && name == "Documentation" {
                            mark_html(args);
                        }
                    }
                }
                prev = None;
                for a in args.iter_mut() {
                    apply(&mut a.nodes, insert_origin, format_html, inside);
                }
            }
        }
    }
}

/// `name = "…"` の文字列が HTML として整えられるなら、字句の種類を `Html` にする（S062）。
fn mark_html(args: &mut [Arg<'_>]) {
    for a in args {
        if let [Node::Leaf(n), Node::Leaf(eq), Node::Leaf(s)] = a.nodes.as_mut_slice()
            && n.kind == Kind::Ident
            && eq.is("=")
            && s.kind == Kind::Str
            && crate::html::pretty(s.text).is_some()
        {
            s.kind = Kind::Html;
        }
    }
}

fn arg_name<'a>(a: &Arg<'a>) -> Option<&'a str> {
    match (a.nodes.first(), a.nodes.get(1)) {
        (Some(Node::Leaf(n)), Some(Node::Leaf(eq))) if n.kind == Kind::Ident && eq.is("=") => {
            Some(n.text)
        }
        _ => None,
    }
}

fn rank(name: Option<&str>) -> u16 {
    let Some(name) = name else { return OTHER };
    for (g, names) in GROUPS.iter().enumerate() {
        if let Some(p) = names.iter().position(|n| *n == name) {
            return (g as u16 + 1) * 100 + p as u16;
        }
    }
    OTHER
}

fn sort_args(args: &mut Vec<Arg<'_>>, insert_origin: bool) {
    // 名前のない引数（位置引数）があれば並べ替えない
    if args.iter().any(|a| arg_name(a).is_none()) {
        return;
    }
    for a in args.iter_mut() {
        a.rank = rank(arg_name(a));
    }
    if insert_origin && !args.iter().any(|a| arg_name(a) == Some("origin")) {
        let syn = |k, s| Node::Leaf(Token::synthetic(k, s));
        let nodes = vec![
            syn(Kind::Ident, "origin"),
            syn(Kind::Punct, "="),
            Node::Group {
                open: Token::synthetic(Kind::Punct, "{"),
                args: vec![
                    Arg {
                        nodes: vec![syn(Kind::Number, "0")],
                        sep: Some(","),
                        rank: 0,
                    },
                    Arg {
                        nodes: vec![syn(Kind::Number, "0")],
                        sep: None,
                        rank: 0,
                    },
                ],
                close: Token::synthetic(Kind::Punct, "}"),
            },
        ];
        args.push(Arg {
            nodes,
            sep: None,
            rank: rank(Some("origin")),
        });
    }
    args.sort_by_key(|a| a.rank);
    let n = args.len();
    for (k, a) in args.iter_mut().enumerate() {
        a.sep = (k + 1 < n).then_some(",");
        // 境目の判定はグループ単位
        a.rank /= 100;
    }
}

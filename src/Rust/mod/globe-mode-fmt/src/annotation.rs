//! 図形プリミティブ（Rectangle, Ellipse, Polygon, Line, Text, Bitmap）の引数を
//! 「位置・形状 → points → fill 系 → line 系 → その他」の順に並べ替え、`origin` が無ければ足す。

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

pub fn apply(nodes: &mut [Node<'_>], insert_origin: bool) {
    let mut prev_primitive = false;
    for n in nodes.iter_mut() {
        match n {
            Node::Leaf(t) => prev_primitive = t.kind == Kind::Ident && PRIMITIVES.contains(&t.text),
            Node::Group { open, args, .. } => {
                if prev_primitive && open.is("(") {
                    sort_args(args, insert_origin);
                }
                prev_primitive = false;
                for a in args.iter_mut() {
                    apply(&mut a.nodes, insert_origin);
                }
            }
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

//! ファイルの中の使わないクラスを落とす（S243。FMU の `resources/lib` を小さくする）。
//!
//! 字句解析（[`lex`]）と文の切り出し（[`class_spans`]）だけで読み、Rumoca の構文解析も名前解決もしない。
//! 規則は「名前が一度も出てこないクラスは使われない」という字句だけの判断で、広めに残す側に倒してある。
//! - 各ファイルのトップレベルのクラスは、いつも残す。
//! - 入れ子のクラス `X` は、親が残り、かつ `X` という識別子が（利用者の本文、または残るクラスの本文に）出てくるときだけ残す。
//!   残すクラスの本文からまた名前が増えるので、増えなくなるまで繰り返す（`extends` の鎖もこれでたどれる）。
//! - `operator` の付いたクラスの子は、演算子（`'+'` など）が名前なしで使われるので、全部残す。
//! - 文を切り出せない・字句に分けられないファイルは、そのまま残す（その識別子は使われたことにする）。
//!
//! shortcut: 名前の一致だけを見る（同じ名前の別のクラスや変数があっても残す）。スコープまでたどるのは、足りなくなる危険のほうが大きい。

use std::collections::HashSet;

use crate::lexer::{Kind, lex};
use crate::structure::class_spans;

/// 1 つのクラス。`own` はクラスの範囲のうち、入れ子のクラスの範囲を除いた部分にある識別子。
struct Node<'a> {
    name: String,
    parent: Option<usize>,
    start: usize,
    end: usize,
    opaque: bool,
    own: Vec<&'a str>,
    kept: bool,
}

/// 字句に分けて切り出せたファイル。
struct Parsed<'a> {
    body: &'a str,
    nodes: Vec<Node<'a>>,
}

fn parse(text: &str) -> Option<Parsed<'_>> {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let tokens = lex(body).ok()?;
    let spans = class_spans(&tokens).ok()?;
    let mut nodes: Vec<Node> = spans
        .iter()
        .map(|s| {
            let parent = s
                .path
                .rsplit_once('.')
                .and_then(|(p, _)| spans.iter().position(|q| q.path == p));
            let first = tokens.iter().position(|t| t.start == s.start).unwrap_or(0);
            Node {
                name: s.path.rsplit('.').next().unwrap_or(&s.path).to_string(),
                parent,
                start: s.start,
                end: s.end,
                opaque: tokens[first..].iter().take(4).any(|t| t.is("operator")),
                own: Vec::new(),
                kept: parent.is_none(),
            }
        })
        .collect();
    // 入れ子のクラスの範囲に入らないトークンが、そのクラスの識別子（spans は出現の順）
    let mut open: Vec<usize> = Vec::new();
    let mut next = 0;
    for t in tokens.iter().filter(|t| t.kind == Kind::Ident) {
        while next < nodes.len() && nodes[next].start <= t.start {
            open.push(next);
            next += 1;
        }
        while open.last().is_some_and(|&o| nodes[o].end <= t.start) {
            open.pop();
        }
        if let Some(&o) = open.last() {
            nodes[o].own.push(t.text);
        }
    }
    Some(Parsed { body, nodes })
}

/// `files`（本文）の使わない入れ子のクラスを落とした本文を返す。`seed` は最初から使われている識別子
/// （利用者の本文の識別子と、モデルの名前の各部分）。
pub fn prune_classes(files: &[&str], seed: &HashSet<String>) -> Vec<String> {
    let mut parsed: Vec<Option<Parsed>> = files.iter().map(|t| parse(t)).collect();
    let mut ids: HashSet<&str> = seed.iter().map(String::as_str).collect();
    for (text, p) in files.iter().zip(&parsed) {
        match p {
            Some(p) => ids.extend(
                p.nodes
                    .iter()
                    .filter(|n| n.parent.is_none())
                    .flat_map(|n| &n.own)
                    .copied(),
            ),
            None => ids.extend(
                lex(text)
                    .iter()
                    .flatten()
                    .filter(|t| t.kind == Kind::Ident)
                    .map(|t| t.text),
            ),
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for p in parsed.iter_mut().flatten() {
            for i in 0..p.nodes.len() {
                let n = &p.nodes[i];
                let Some(parent) = n.parent else { continue };
                if !n.kept
                    && p.nodes[parent].kept
                    && (p.nodes[parent].opaque || ids.contains(n.name.as_str()))
                {
                    ids.extend(n.own.iter().copied());
                    p.nodes[i].kept = true;
                    changed = true;
                }
            }
        }
    }
    files
        .iter()
        .zip(&parsed)
        .map(|(text, p)| match p {
            Some(p) => cut(p),
            None => (*text).to_string(),
        })
        .collect()
}

/// 残らないクラスの範囲（と、その後ろの改行）を削る。
fn cut(p: &Parsed<'_>) -> String {
    let mut out = String::with_capacity(p.body.len());
    let mut at = 0;
    for n in &p.nodes {
        // 親が落ちていれば、親の範囲ごと削れている（親は子より先に来る）ので `n.start < at`
        if n.kept || n.start < at {
            continue;
        }
        out.push_str(&p.body[at..n.start]);
        at = n.end + usize::from(p.body.as_bytes().get(n.end) == Some(&b'\n'));
    }
    out.push_str(&p.body[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn drops_unused_siblings_and_keeps_within_nesting_and_extends_targets() {
        let a = "within Lib;\npackage P\n  model Base\n    Real x;\n  end Base;\n\n  model Used\n    extends Base;\n  end Used;\n\n  model Unused\n    Real y;\n  end Unused;\n\n  package Q\n    model Deep\n    end Deep;\n    model Dead\n    end Dead;\n  end Q;\nend P;\n";
        let user = "model M\n  Lib.P.Used u;\n  Lib.P.Q.Deep d;\nend M;\n";
        let user_ids: HashSet<String> = lex(user)
            .unwrap()
            .iter()
            .filter(|t| t.kind == Kind::Ident)
            .map(|t| t.text.to_string())
            .collect();
        let out = prune_classes(&[a], &user_ids);
        let o = &out[0];
        assert!(o.starts_with("within Lib;\npackage P\n"), "{o}");
        for kept in [
            "model Base",
            "model Used",
            "package Q",
            "model Deep",
            "end P;",
        ] {
            assert!(o.contains(kept), "{kept} が消えた:\n{o}");
        }
        for gone in ["Unused", "Dead"] {
            assert!(!o.contains(gone), "{gone} が残った:\n{o}");
        }
        // 何も使われなければトップレベルのクラスだけ
        let out = prune_classes(&[a], &seed(&[]));
        assert!(!out[0].contains("model"), "{}", out[0]);
        assert!(out[0].contains("package P") && out[0].contains("end P;"));
    }

    #[test]
    fn operator_children_and_unparsable_files_stay() {
        let op = "within L;\noperator record C\n  Real re;\n  encapsulated operator function '+'\n  end '+';\nend C;\n";
        assert_eq!(prune_classes(&[op], &seed(&[]))[0], op);
        let bad = "package P model A end"; // end の後ろに名前が無く、文を切り出せない
        assert_eq!(prune_classes(&[bad], &seed(&[]))[0], bad);
    }
}

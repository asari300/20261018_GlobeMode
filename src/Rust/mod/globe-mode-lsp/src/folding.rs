//! `textDocument/foldingRange`（S220）。字句は `globe-mode-fmt::lex`、クラスの範囲は文書のアウトラインを使い、
//! 新しいパーサは持たない。行だけで決まるので桁（UTF-16）は出さない。
//!
//! 範囲: クラス（`end` の行は残す）、`equation`・`algorithm`（`initial` を含む）・`public`・`protected` の節
//! （次の節かクラスの `end` の手前まで。末尾の空行は含めない）、複数行の `annotation(…)`（閉じる `)` だけの行は残す）、
//! 連続する行コメントと複数行のブロックコメント（kind は `comment`）。
//! shortcut: 構文が壊れてアウトラインが引けない間は、クラスと節の範囲が出ない（コメントと annotation は出る）。

use globe_mode_fmt::{Kind, Token, lex};
use globe_mode_ir::ClassOutline;
use serde_json::{Value, json};

pub fn folding_ranges(text: &str, classes: Option<&[ClassOutline]>) -> Vec<Value> {
    let Ok(tokens) = lex(text) else {
        return Vec::new();
    };
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line = |off: usize| starts.partition_point(|&s| s <= off) - 1;
    let blank = |l: usize| {
        let end = starts.get(l + 1).copied().unwrap_or(text.len());
        text[starts[l]..end].trim().is_empty()
    };
    let mut out = Vec::new();
    // 開始の行から終了の行（含む）まで。1 行に収まるものは折りたたまない
    let mut add = |from: usize, to: usize, kind: Option<&str>| {
        if from < to {
            out.push(json!({"startLine": from, "endLine": to, "kind": kind}));
        }
    };

    for c in classes.into_iter().flatten() {
        class_ranges(c, &tokens, &line, &blank, &mut add);
    }
    // annotation(…): 対応する `)` まで
    for (i, t) in tokens.iter().enumerate() {
        if t.is("annotation") && tokens.get(i + 1).is_some_and(|n| n.is("(")) {
            let mut depth = 0;
            for (j, u) in tokens.iter().enumerate().skip(i + 1) {
                depth += u.is("(") as i32 - u.is(")") as i32;
                if depth == 0 {
                    let closer_alone =
                        line(tokens[j - 1].start + tokens[j - 1].text.len()) < line(u.start);
                    add(line(t.start), line(u.start) - closer_alone as usize, None);
                    break;
                }
            }
        }
    }
    // コメント: 行コメントは続く行ごと、ブロックコメントは複数行のもの
    let mut run: Option<(usize, usize)> = None;
    for t in &tokens {
        let (a, b) = (
            line(t.start),
            line(t.start + t.text.len().saturating_sub(1)),
        );
        match (t.kind, run) {
            (Kind::LineComment, Some((from, to))) if a == to + 1 => run = Some((from, a)),
            _ => {
                if let Some((from, to)) = run.take() {
                    add(from, to, Some("comment"));
                }
                match t.kind {
                    Kind::LineComment => run = Some((a, a)),
                    Kind::BlockComment => add(a, b, Some("comment")),
                    _ => {}
                }
            }
        }
    }
    if let Some((from, to)) = run {
        add(from, to, Some("comment"));
    }
    out
}

fn class_ranges(
    c: &ClassOutline,
    tokens: &[Token<'_>],
    line: &dyn Fn(usize) -> usize,
    blank: &dyn Fn(usize) -> bool,
    add: &mut dyn FnMut(usize, usize, Option<&str>),
) {
    let (first, last) = (line(c.range.start as usize), line(c.range.end as usize));
    add(first, last.saturating_sub(1), None);
    // この class 直下の節（入れ子のクラスの中は、そのクラスが数える）
    let mut sections: Vec<usize> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        let at = t.start;
        if at < c.range.start as usize
            || at >= c.range.end as usize
            || c.classes
                .iter()
                .any(|k| (k.range.start as usize..k.range.end as usize).contains(&at))
            || !["equation", "algorithm", "public", "protected"]
                .iter()
                .any(|k| t.is(k))
        {
            continue;
        }
        let initial = i > 0 && tokens[i - 1].is("initial") && !t.is("public") && !t.is("protected");
        sections.push(line(if initial { tokens[i - 1].start } else { at }));
    }
    for (k, &from) in sections.iter().enumerate() {
        let mut to = sections.get(k + 1).copied().unwrap_or(last);
        to = to.saturating_sub(1);
        while to > from && blank(to) {
            to -= 1;
        }
        add(from, to, None);
    }
    for k in &c.classes {
        class_ranges(k, tokens, line, blank, add);
    }
}

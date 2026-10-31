//! lint。規則は 3 つ: `line-too-long`（整形しても行幅を超える行）、
//! `class-name-pascal-case`（クラス名が大文字で始まらない）、`missing-description`（長いクラスに説明文字列が無い）。

use crate::lexer::{Kind, lex};
use crate::{Options, format};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintMessage {
    pub rule: &'static str,
    /// 1 始まり
    pub line: usize,
    /// 1 始まり、文字単位
    pub column: usize,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LintOptions {
    pub format: Options,
    /// 止める規則の名前
    pub disabled: Vec<String>,
}

const CLASS_KEYWORD: &[&str] = &[
    "model",
    "package",
    "function",
    "record",
    "block",
    "connector",
    "type",
    "class",
];

pub fn lint(src: &str, options: &LintOptions) -> Vec<LintMessage> {
    let on = |rule: &str| !options.disabled.iter().any(|d| d == rule);
    let mut out = Vec::new();
    let Ok(tokens) = lex(src) else { return out };
    let line_col = |start: usize| {
        let before = &src[..start];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, column)
    };
    for (i, t) in tokens.iter().enumerate() {
        if t.kind != Kind::Ident || !CLASS_KEYWORD.contains(&t.text) {
            continue;
        }
        // `type X = …;` と `model extends X` の名前も見る
        let name_i = if tokens.get(i + 1).is_some_and(|s| s.is("extends")) {
            i + 2
        } else {
            i + 1
        };
        let Some(name) = tokens
            .get(name_i)
            .filter(|s| s.kind == Kind::Ident && !s.is_keyword())
        else {
            continue;
        };
        let (line, column) = line_col(name.start);
        if on("class-name-pascal-case")
            && !name.text.starts_with(|c: char| c.is_ascii_uppercase())
            && !name.text.starts_with('\'')
        {
            out.push(LintMessage {
                rule: "class-name-pascal-case",
                line,
                column,
                message: format!("クラス名 {} は大文字で始める", name.text),
            });
        }
        let short = tokens.get(name_i + 1).is_some_and(|s| s.is("="));
        if on("missing-description")
            && !short
            && !tokens.get(name_i + 1).is_some_and(|s| s.kind == Kind::Str)
        {
            out.push(LintMessage {
                rule: "missing-description",
                line,
                column,
                message: format!("{} {} に説明の文字列が無い", t.text, name.text),
            });
        }
    }
    if on("line-too-long") {
        let width = options.format.width;
        let formatted = format(src, &options.format).unwrap_or_else(|_| src.to_string());
        let src_lines: Vec<&str> = src.lines().collect();
        for long in formatted.lines().filter(|l| l.chars().count() > width) {
            let trimmed = long.trim();
            let line = src_lines
                .iter()
                .position(|l| l.trim() == trimmed)
                .map_or(1, |p| p + 1);
            out.push(LintMessage {
                rule: "line-too-long",
                line,
                column: width + 1,
                message: format!(
                    "整形しても {width} 桁を超える（{} 文字）",
                    long.chars().count()
                ),
            });
        }
    }
    out.sort_by_key(|m| (m.line, m.column));
    out
}

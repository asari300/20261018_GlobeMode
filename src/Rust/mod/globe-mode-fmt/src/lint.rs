//! lint。規則は 4 つ: `line-too-long`（整形しても行幅を超える行）、
//! `class-name-pascal-case`（クラス名が大文字で始まらない）、`missing-description`（長いクラスに説明文字列が無い）、
//! `annotation-order`（S061。annotation と Documentation の引数の順。整形で直る。LSP では通知の重さ）。

use std::collections::HashMap;

use crate::lexer::{Kind, lex};
use crate::{Options, annotation, format, layout, structure};

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

/// 規則のコード（LSP の診断の code）と名前。`connect-type-mismatch` はワークスペースが要るので CLI と LSP が見る
pub const RULES: &[(&str, &str)] = &[
    ("GM-L0001", "line-too-long"),
    ("GM-L0002", "class-name-pascal-case"),
    ("GM-L0003", "missing-description"),
    ("GM-L0004", "annotation-order"),
    ("GM-L0005", "connect-type-mismatch"),
];

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
    // S211: BOM は字句ではない。行・桁は BOM を除いた本文で数える（エディタの見え方）
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let Ok(tokens) = lex(src) else { return out };
    // S207: 診断ごとに先頭から数え直すと本文の長さの 2 乗になる。改行の位置の表を 1 度だけ作り、二分探索で引く
    let newlines: Vec<usize> = src.match_indices('\n').map(|(i, _)| i).collect();
    let line_col = |start: usize| {
        let line = newlines.partition_point(|&p| p < start);
        let line_start = if line == 0 { 0 } else { newlines[line - 1] + 1 };
        (line + 1, src[line_start..start].chars().count() + 1)
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
    if on("annotation-order")
        && let Ok(elements) = structure::split(&tokens, false)
    {
        let mut found = Vec::new();
        for e in &elements {
            if let Ok(tree) = layout::build(e.tokens) {
                annotation::misordered(&tree, false, &mut found);
            }
        }
        for (start, table) in found {
            let (line, column) = line_col(start);
            let message = if table == "annotation" {
                "annotation の引数は experiment → Diagram → Icon → Documentation の順（ほかは後ろ）。整形で直る"
            } else {
                "Documentation の引数は header（__OpenModelica_infoHeader）→ info → revisions の順。整形で直る"
            };
            out.push(LintMessage {
                rule: "annotation-order",
                line,
                column,
                message: message.to_string(),
            });
        }
    }
    if on("line-too-long") {
        let width = options.format.width;
        let formatted = format(src, &options.format).unwrap_or_else(|_| src.to_string());
        // 最初に現れる行（trim して比べる）。長い行ごとに全行を探すと 2 乗になる
        let mut first_line: HashMap<&str, usize> = HashMap::new();
        for (i, l) in src.lines().enumerate() {
            first_line.entry(l.trim()).or_insert(i + 1);
        }
        for long in formatted.lines().filter(|l| l.chars().count() > width) {
            let line = first_line.get(long.trim()).copied().unwrap_or(1);
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

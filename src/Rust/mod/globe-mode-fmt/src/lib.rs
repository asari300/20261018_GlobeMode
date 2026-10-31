//! GlobeMode の Modelica 整形器と lint（S053）。依存は無く、wasm でも CLI でもそのまま使える。
//!
//! 規則は `doc/format-style.md`。流れは 字句解析（[`lexer`]）→ 文の切り出しと段付け（[`structure`]）→
//! 80 桁への配置（[`layout`]）。図形プリミティブの引数の並べ替えと `origin` の補完は [`annotation`]。
//! 整形の出力は再び字句解析して、空白以外のトークン列が意図どおりであることを確かめる（意味を変えない保証）。

mod annotation;
mod layout;
mod lexer;
mod lint;
mod structure;

pub use lint::{LintMessage, LintOptions, lint};

/// Modelica の予約語と組み込み型。補完の候補に使う。
pub const KEYWORDS: &[&str] = &[
    "algorithm",
    "and",
    "annotation",
    "assert",
    "block",
    "break",
    "class",
    "connect",
    "connector",
    "constant",
    "constrainedby",
    "der",
    "discrete",
    "each",
    "else",
    "elseif",
    "elsewhen",
    "encapsulated",
    "end",
    "enumeration",
    "equation",
    "expandable",
    "extends",
    "external",
    "false",
    "final",
    "flow",
    "for",
    "function",
    "if",
    "import",
    "impure",
    "in",
    "initial",
    "inner",
    "input",
    "loop",
    "model",
    "not",
    "operator",
    "or",
    "outer",
    "output",
    "package",
    "parameter",
    "partial",
    "protected",
    "public",
    "pure",
    "record",
    "redeclare",
    "replaceable",
    "return",
    "stream",
    "then",
    "true",
    "type",
    "when",
    "while",
    "within",
    "Real",
    "Integer",
    "Boolean",
    "String",
];

/// 整形の設定。既定はインデント 2、80 桁、`origin` の補完あり。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// 1 段のインデントの空白の数
    pub indent: usize,
    /// 行幅。収まらない呼び出しと配列を括弧で折り返す
    pub width: usize,
    /// 図形プリミティブに `origin` が無ければ `origin = {0, 0}` を足す
    pub insert_origin: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            indent: 2,
            width: 80,
            insert_origin: true,
        }
    }
}

/// 整形できなかった理由。`offset` は入力のバイト位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatError {
    pub message: String,
    pub offset: usize,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (byte {})", self.message, self.offset)
    }
}

impl std::error::Error for FormatError {}

fn err(message: impl Into<String>, offset: usize) -> FormatError {
    FormatError {
        message: message.into(),
        offset,
    }
}

/// 本文を整形する。文法が壊れている（括弧や `end` が合わない）ときは `Err` を返し、本文には触らない。
pub fn format(src: &str, options: &Options) -> Result<String, FormatError> {
    let tokens = lexer::lex(src)?;
    let elements = structure::split(&tokens)?;
    let newline = if src.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(src.len() + src.len() / 8);
    let mut expected: Vec<(lexer::Kind, &str)> = Vec::with_capacity(tokens.len());
    let mut first = true;
    for e in &elements {
        let mut tree = layout::build(e.tokens)?;
        annotation::apply(&mut tree, options.insert_origin);
        layout::flatten(&tree, &mut expected);
        if e.blank_before && !first {
            out.push_str(newline);
        }
        for line in layout::render(&tree, e.indent, options) {
            out.push_str(&line);
            out.push_str(newline);
        }
        first = false;
    }
    let actual: Vec<(lexer::Kind, &str)> = lexer::lex(&out)
        .map_err(|e| err(format!("整形の出力を読み直せない: {}", e.message), 0))?
        .iter()
        .map(|t| (t.kind, t.text))
        .collect();
    if actual != expected {
        let at = actual
            .iter()
            .zip(&expected)
            .position(|(a, b)| a != b)
            .unwrap_or(expected.len().min(actual.len()));
        return Err(err(
            format!("整形でトークンが変わった（{at} 番目）。入力をそのまま残す"),
            tokens.get(at).map_or(0, |t| t.start),
        ));
    }
    Ok(out)
}

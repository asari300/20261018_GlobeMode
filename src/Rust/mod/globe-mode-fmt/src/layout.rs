//! 1 文のトークン列を括弧の木にして、行幅に収まるように配置する。
//!
//! 収まる呼び出し・配列は 1 行に書く。収まらなければ開き括弧で改行し、中身を 1 段下げ、
//! 閉じ括弧を元の段に単独で置く。中身の引数は行幅まで詰め、「展開した引数の後」と
//! 「並べ替えのグループの境目」では必ず改行する。コメントと文字列の中身は変えない。

use crate::Options;
use crate::lexer::{Kind, Token};
use crate::{FormatError, err};

#[derive(Debug)]
pub enum Node<'a> {
    Leaf(Token<'a>),
    Group {
        open: Token<'a>,
        args: Vec<Arg<'a>>,
        close: Token<'a>,
    },
}

#[derive(Debug)]
pub struct Arg<'a> {
    pub nodes: Vec<Node<'a>>,
    /// 後ろに続く区切り（`,` か行列の `;`）
    pub sep: Option<&'static str>,
    /// 並べ替えのグループ。違うグループの間では改行する（0 は並べ替えなし）
    pub rank: u16,
}

/// 括弧の入れ子の上限（S211）。木をたどる `flatten`・`render`・`annotation::apply` と木の drop は入れ子の深さだけ再帰するので、
/// 外から来る本文が 200 KB の `(` で 2 MB のスタックを溢れさせ、プロセスごと落とすのを防ぐ。実際のモデルは 20 段にも届かない。
const MAX_DEPTH: usize = 256;

pub fn build<'a>(tokens: &[Token<'a>]) -> Result<Vec<Node<'a>>, FormatError> {
    let mut i = 0;
    let nodes = seq(tokens, &mut i, None, 0)?;
    if i < tokens.len() {
        return Err(err("対応しない閉じ括弧", tokens[i].start));
    }
    Ok(nodes)
}

fn seq<'a>(
    tokens: &[Token<'a>],
    i: &mut usize,
    close: Option<&str>,
    depth: usize,
) -> Result<Vec<Node<'a>>, FormatError> {
    let mut out = Vec::new();
    while *i < tokens.len() {
        let t = tokens[*i];
        if t.is(")") || t.is("]") || t.is("}") {
            if close == Some(t.text) {
                return Ok(out);
            }
            return Err(err(format!("括弧 {} が合わない", t.text), t.start));
        }
        if close.is_some() && (t.is(",") || (t.is(";") && close == Some("]"))) {
            return Ok(out);
        }
        *i += 1;
        if t.is("(") || t.is("[") || t.is("{") {
            let want = match t.text {
                "(" => ")",
                "[" => "]",
                _ => "}",
            };
            if depth >= MAX_DEPTH {
                return Err(err(
                    format!("括弧の入れ子が深すぎる（{MAX_DEPTH} 段まで）"),
                    t.start,
                ));
            }
            let mut args = Vec::new();
            loop {
                let nodes = seq(tokens, i, Some(want), depth + 1)?;
                let Some(next) = tokens.get(*i) else {
                    return Err(err("括弧が閉じていない", t.start));
                };
                if next.is(want) {
                    *i += 1;
                    if !nodes.is_empty() || !args.is_empty() {
                        args.push(Arg {
                            nodes,
                            sep: None,
                            rank: 0,
                        });
                    }
                    out.push(Node::Group {
                        open: t,
                        args,
                        close: *next,
                    });
                    break;
                }
                *i += 1;
                args.push(Arg {
                    nodes,
                    sep: Some(if next.is(",") { "," } else { ";" }),
                    rank: 0,
                });
            }
        } else {
            out.push(Node::Leaf(t));
        }
    }
    match close {
        Some(_) => Err(err(
            "括弧が閉じていない",
            tokens.last().map_or(0, |t| t.start),
        )),
        None => Ok(out),
    }
}

/// 木をトークン列（種類と字面）に戻す。整形の出力と突き合わせるために使う。
pub fn flatten<'a>(nodes: &[Node<'a>], out: &mut Vec<(Kind, &'a str)>) {
    for n in nodes {
        match n {
            Node::Leaf(t) => out.push((t.kind, t.text)),
            Node::Group { open, args, close } => {
                out.push((open.kind, open.text));
                for a in args {
                    flatten(&a.nodes, out);
                    if let Some(s) = a.sep {
                        out.push((Kind::Punct, s));
                    }
                }
                out.push((close.kind, close.text));
            }
        }
    }
}

pub fn render(nodes: &[Node<'_>], indent: usize, options: &Options) -> Vec<String> {
    let mut r = Renderer {
        lines: Vec::new(),
        cur: " ".repeat(indent * options.indent),
        prev: None,
        prev_unary: false,
        after_line_comment: false,
        opt: options,
    };
    r.emit(nodes, indent);
    r.flush();
    r.lines
}

struct Renderer<'o> {
    lines: Vec<String>,
    cur: String,
    prev: Option<(Kind, String)>,
    prev_unary: bool,
    after_line_comment: bool,
    opt: &'o Options,
}

/// この予約語の後ろの `(` には空白を置く（`if (a)`）。`der(` や `initial(` は関数呼び出しなので詰める。
const SPACE_BEFORE_PAREN: &[&str] = &[
    "if",
    "elseif",
    "when",
    "elsewhen",
    "while",
    "for",
    "and",
    "or",
    "not",
    "then",
    "else",
    "in",
    "return",
    "loop",
    "import",
    "within",
    "extends",
    "constrainedby",
    "each",
    "final",
    "flow",
    "stream",
];

/// 直前のトークンの後ろに空白を置くか。`prev_unary` は直前が単項の `+` `-` だったか。
fn space_between(prev: &(Kind, String), prev_unary: bool, next: &Token<'_>) -> bool {
    let (pk, pt) = (prev.0, prev.1.as_str());
    if next.is_comment() {
        return true;
    }
    if prev_unary {
        return false;
    }
    if next.kind == Kind::Punct {
        match next.text {
            ")" | "]" | "}" | "," | ";" | "." | "^" | ":" => return false,
            "(" => {
                return !(pk == Kind::Ident && !SPACE_BEFORE_PAREN.contains(&pt))
                    && !matches!(pt, ")" | "]");
            }
            "[" => return !(pk == Kind::Ident || matches!(pt, ")" | "]" | "}")),
            _ => {}
        }
    }
    !(pk == Kind::Punct && matches!(pt, "(" | "[" | "{" | "." | "^" | ":"))
}

/// `-` `+` が単項か（直前が演算子・開き括弧・区切り・予約語なら単項）。
fn is_unary(prev: Option<&(Kind, String)>, t: &Token<'_>) -> bool {
    if !(t.is("-") || t.is("+")) {
        return false;
    }
    match prev {
        None => true,
        Some((Kind::Punct, p)) => !matches!(p.as_str(), ")" | "]" | "}"),
        Some((Kind::Ident, p)) => {
            crate::KEYWORDS.contains(&p.as_str())
                && !matches!(
                    p.as_str(),
                    "true" | "false" | "Real" | "Integer" | "Boolean" | "String" | "end"
                )
        }
        _ => false,
    }
}

impl Renderer<'_> {
    fn width(&self) -> usize {
        self.cur.chars().count()
    }

    fn newline(&mut self, indent: usize) {
        self.lines.push(std::mem::take(&mut self.cur));
        self.cur = " ".repeat(indent * self.opt.indent);
        self.after_line_comment = false;
    }

    fn flush(&mut self) {
        if !self.cur.trim().is_empty() {
            self.lines.push(std::mem::take(&mut self.cur));
        }
    }

    fn push_token(&mut self, t: &Token<'_>, indent: usize) {
        if self.after_line_comment {
            self.newline(indent + 1);
        }
        if let Some(p) = &self.prev
            && !self.cur.trim().is_empty()
            && space_between(p, self.prev_unary, t)
        {
            self.cur.push(' ');
        }
        self.prev_unary = is_unary(self.prev.as_ref(), t);
        if t.kind == Kind::Html {
            // 開きの `"` の後で改行し、中身を 1 段下げ、閉じの `"` を元の段に置く
            let pad = |d: usize| " ".repeat(d * self.opt.indent);
            self.cur.push('"');
            for (d, line) in crate::html::pretty(t.text).unwrap_or_default() {
                self.cur.push('\n');
                self.cur.push_str(&pad(indent + 1 + d));
                self.cur.push_str(&line);
            }
            self.cur.push('\n');
            self.cur.push_str(&pad(indent));
            self.cur.push('"');
        } else {
            self.cur.push_str(t.text);
        }
        if t.kind == Kind::LineComment {
            self.after_line_comment = true;
        }
        self.prev = Some((t.kind, t.text.to_string()));
    }

    /// 1 行に書けるならその文字列。コメントや改行を含む字句があれば `None`。
    fn flat(&self, nodes: &[Node<'_>]) -> Option<String> {
        let mut r = Renderer {
            lines: Vec::new(),
            cur: String::new(),
            prev: self.prev.clone(),
            prev_unary: self.prev_unary,
            after_line_comment: false,
            opt: self.opt,
        };
        r.flat_into(nodes)?;
        Some(r.cur)
    }

    fn flat_into(&mut self, nodes: &[Node<'_>]) -> Option<()> {
        for n in nodes {
            match n {
                Node::Leaf(t) => {
                    if matches!(t.kind, Kind::LineComment | Kind::Html) || t.text.contains('\n') {
                        return None;
                    }
                    self.push_token(t, 0);
                }
                Node::Group { open, args, close } => {
                    self.push_token(open, 0);
                    for a in args {
                        self.flat_into(&a.nodes)?;
                        if let Some(s) = a.sep {
                            self.push_token(&Token::synthetic(Kind::Punct, s), 0);
                        }
                    }
                    self.push_token(close, 0);
                }
            }
        }
        Some(())
    }

    fn emit(&mut self, nodes: &[Node<'_>], indent: usize) {
        for n in nodes {
            match n {
                Node::Leaf(t) => self.push_token(t, indent),
                Node::Group { open, args, close } => {
                    let flat = self.flat(std::slice::from_ref(n));
                    match flat {
                        Some(f) if self.width() + f.chars().count() <= self.opt.width => {
                            self.push_flat(&f, open, close);
                        }
                        _ => self.expand(open, args, close, indent),
                    }
                }
            }
        }
    }

    /// 1 行に収めた文字列を置く。`f` は直前のトークンとの空白を含めて作ってある。
    fn push_flat(&mut self, f: &str, first: &Token<'_>, last: &Token<'_>) {
        if self.after_line_comment {
            self.newline(0);
        }
        if self.cur.trim().is_empty() {
            self.cur.push_str(f.trim_start());
        } else {
            if let Some(p) = &self.prev
                && space_between(p, self.prev_unary, first)
            {
                self.cur.push(' ');
            }
            self.cur.push_str(f);
        }
        self.prev_unary = false;
        self.prev = Some((last.kind, last.text.to_string()));
    }

    fn expand(&mut self, open: &Token<'_>, args: &[Arg<'_>], close: &Token<'_>, indent: usize) {
        self.push_token(open, indent);
        let inner = indent + 1;
        self.newline(inner);
        let mut prev_expanded = false;
        let mut prev_rank = None;
        for a in args {
            let sep_len = usize::from(a.sep.is_some());
            let flat = self.flat(&a.nodes);
            let at_start = self.cur.trim().is_empty();
            let fits_here = flat
                .as_ref()
                .is_some_and(|f| self.width() + f.chars().count() + sep_len <= self.opt.width);
            let rank_break = prev_rank.is_some_and(|r| r != a.rank);
            if !at_start && (prev_expanded || rank_break || !fits_here) {
                self.newline(inner);
            }
            let before = self.lines.len();
            match flat {
                Some(f)
                    if !a.nodes.is_empty()
                        && self.width() + f.trim_start().chars().count() + sep_len
                            <= self.opt.width =>
                {
                    self.push_flat(&f, first_token(&a.nodes), last_token(&a.nodes));
                }
                _ => self.emit(&a.nodes, inner),
            }
            prev_expanded = self.lines.len() > before;
            if let Some(s) = a.sep {
                self.push_token(&Token::synthetic(Kind::Punct, s), inner);
            }
            prev_rank = Some(a.rank);
        }
        self.newline(indent);
        self.push_token(close, indent);
    }
}

fn first_token<'n, 'a>(nodes: &'n [Node<'a>]) -> &'n Token<'a> {
    match nodes.first().expect("引数は空でない") {
        Node::Leaf(t) => t,
        Node::Group { open, .. } => open,
    }
}

fn last_token<'n, 'a>(nodes: &'n [Node<'a>]) -> &'n Token<'a> {
    match nodes.last().expect("引数は空でない") {
        Node::Leaf(t) => t,
        Node::Group { close, .. } => close,
    }
}

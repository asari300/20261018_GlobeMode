//! 寛容なパーサ。誤りがあっても必ず木を作り、読めなかったところは `Error` 節点に入れて診断を残す。
//! 全トークン（空白・コメント含む）が木に入るので、木の文字列は元の文字列と一致する。
//!
//! 最初の一切れの範囲: クラスの定義・`extends`・`import`・部品の宣言・`equation` / `algorithm` 節。
//! 式・修飾・注釈の中身・文の中身は「括弧の対応と `;` までだけ見る」不透明な節点（式の文法は次回）。

use crate::kind::{SyntaxKind, SyntaxKind::*};
use crate::lexer::lex;
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

/// 入れ子（クラス・if/for/when）の上限。超えたら読み飛ばす（スタックを溢れさせない）。
const MAX_DEPTH: u32 = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub range: TextRange,
    pub message: String,
}

#[derive(Debug)]
pub struct Parse {
    green: GreenNode,
    pub errors: Vec<ParseError>,
}

impl Parse {
    pub fn syntax(&self) -> crate::SyntaxNode {
        crate::SyntaxNode::new_root(self.green.clone())
    }
}

pub fn parse(text: &str) -> Parse {
    let mut p = Parser::new(text);
    p.stored_definition();
    p.finish()
}

struct Tok {
    kind: SyntaxKind,
    start: usize,
    end: usize,
}

struct Parser<'a> {
    text: &'a str,
    toks: Vec<Tok>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
    depth: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Scan {
    /// 宣言の束縛・条件。`,` `annotation` と、被演算子の後ろの `if`・文字列で止まる
    Expr,
    /// 方程式・文・short class。`;` だけで止まる
    Stmt,
    /// `(` `[` `{` から対応する閉じまで
    Group,
    /// `if` / `when` / `for` / `while` の条件。`then`（入れ子の if 式の分を除く）か `loop` で止まる
    Header,
}

fn is_class_kind(k: SyntaxKind) -> bool {
    matches!(
        k,
        KwClass
            | KwModel
            | KwBlock
            | KwConnector
            | KwType
            | KwPackage
            | KwRecord
            | KwFunction
            | KwOperator
    )
}

fn is_class_prefix(k: SyntaxKind) -> bool {
    is_class_kind(k)
        || matches!(
            k,
            KwEncapsulated | KwPartial | KwExpandable | KwPure | KwImpure
        )
}

/// 部品にも入れ子のクラスにもつく前置き
fn is_element_prefix(k: SyntaxKind) -> bool {
    matches!(k, KwRedeclare | KwFinal | KwInner | KwOuter | KwReplaceable)
}

fn is_component_prefix(k: SyntaxKind) -> bool {
    is_element_prefix(k)
        || matches!(
            k,
            KwFlow | KwStream | KwDiscrete | KwParameter | KwConstant | KwInput | KwOutput
        )
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        let mut toks = Vec::new();
        let mut at = 0;
        for t in lex(text) {
            toks.push(Tok {
                kind: t.kind,
                start: at,
                end: at + t.len,
            });
            at += t.len;
        }
        let mut errors = Vec::new();
        // 字句の誤りは、字句を読んだ時点で診断にする
        for t in &toks {
            let s = &text[t.start..t.end];
            let msg = match t.kind {
                ErrorToken => Some("unexpected character"),
                Str if s.len() < 2 || !s.ends_with('"') => Some("unterminated string"),
                BlockComment if !s.ends_with("*/") => Some("unterminated block comment"),
                Ident if s.starts_with('\'') && (s.len() < 2 || !s.ends_with('\'')) => {
                    Some("unterminated quoted identifier")
                }
                _ => None,
            };
            if let Some(m) = msg {
                errors.push(ParseError {
                    range: range(t.start, t.end),
                    message: m.into(),
                });
            }
        }
        Parser {
            text,
            toks,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors,
            depth: 0,
        }
    }

    fn finish(mut self) -> Parse {
        // 末尾の空白・コメント
        while self.pos < self.toks.len() {
            self.emit();
        }
        self.builder.finish_node();
        Parse {
            green: self.builder.finish(),
            errors: self.errors,
        }
    }

    // ---- トークンの読み取り
    fn nth_idx(&self, n: usize) -> Option<usize> {
        (self.pos..self.toks.len())
            .filter(|&i| !self.toks[i].kind.is_trivia())
            .nth(n)
    }

    /// n 個先の（trivia を除いた）トークン。無ければ None（EOF）
    fn nth(&self, n: usize) -> Option<SyntaxKind> {
        self.nth_idx(n).map(|i| self.toks[i].kind)
    }

    fn cur(&self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    fn at(&self, k: SyntaxKind) -> bool {
        self.cur() == Some(k)
    }

    fn eof(&self) -> bool {
        self.cur().is_none()
    }

    fn emit(&mut self) {
        let t = &self.toks[self.pos];
        self.builder
            .token(rowan::SyntaxKind(t.kind as u16), &self.text[t.start..t.end]);
        self.pos += 1;
    }

    fn eat_trivia(&mut self) {
        while self.pos < self.toks.len() && self.toks[self.pos].kind.is_trivia() {
            self.emit();
        }
    }

    fn bump(&mut self) {
        self.eat_trivia();
        if self.pos < self.toks.len() {
            self.emit();
        }
    }

    fn eat(&mut self, k: SyntaxKind) -> bool {
        let ok = self.at(k);
        if ok {
            self.bump();
        }
        ok
    }

    fn start(&mut self, k: SyntaxKind) {
        self.eat_trivia();
        self.builder.start_node(rowan::SyntaxKind(k as u16));
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn error(&mut self, msg: &str) {
        let r = match self.nth_idx(0) {
            Some(i) => range(self.toks[i].start, self.toks[i].end),
            None => range(self.text.len(), self.text.len()),
        };
        self.errors.push(ParseError {
            range: r,
            message: msg.into(),
        });
    }

    fn expect(&mut self, k: SyntaxKind, what: &str) -> bool {
        if self.eat(k) {
            return true;
        }
        self.error(&format!("expected {what}"));
        false
    }

    /// 次の 1 トークンを `Error` 節点に入れて食べる
    fn bump_error(&mut self, msg: &str) {
        self.error(msg);
        self.start(Error);
        self.bump();
        self.finish_node();
    }

    // ---- 不透明な走査
    /// 構造を壊してまで先へ進まない場所（節の見出し・クラスの `end Name`）
    fn is_boundary(&self) -> bool {
        match self.cur() {
            Some(KwEquation | KwAlgorithm | KwPublic | KwProtected | KwExternal) => true,
            Some(KwInitial) => matches!(self.nth(1), Some(KwEquation | KwAlgorithm)),
            Some(KwEnd) => self.nth(1) == Some(Ident),
            _ => false,
        }
    }

    fn scan(&mut self, mode: Scan) {
        let mut stack: Vec<SyntaxKind> = Vec::new();
        let mut pending_then = 0u32;
        let mut prev_operand = false;
        while let Some(k) = self.cur() {
            if self.is_boundary() {
                break;
            }
            let top_level = stack.is_empty();
            match k {
                Semi if stack.last() != Some(&LBracket) => break,
                Comma | KwAnnotation if mode == Scan::Expr && top_level => break,
                Str | KwIf if mode == Scan::Expr && top_level && prev_operand => break,
                RParen | RBracket | RBrace if mode == Scan::Expr && top_level => break,
                KwLoop if mode == Scan::Header => break,
                KwThen if mode == Scan::Header => {
                    if pending_then == 0 {
                        break;
                    }
                    pending_then -= 1;
                }
                KwIf | KwElseif if mode == Scan::Header => pending_then += 1,
                _ => {}
            }
            match k {
                LParen | LBracket | LBrace => stack.push(k),
                RParen | RBracket | RBrace => {
                    stack.pop();
                }
                _ => {}
            }
            prev_operand = matches!(
                k,
                Ident | UInt | UReal | Str | RParen | RBracket | RBrace | KwTrue | KwFalse | KwEnd
            );
            self.bump();
            if mode == Scan::Group && stack.is_empty() {
                return;
            }
        }
        if !stack.is_empty() {
            self.error("unclosed bracket");
        }
    }

    // ---- 文法
    fn stored_definition(&mut self) {
        self.builder
            .start_node(rowan::SyntaxKind(StoredDefinition as u16));
        if self.at(KwWithin) {
            self.start(WithinClause);
            self.bump();
            if matches!(self.cur(), Some(Ident | Dot)) {
                self.name();
            }
            self.expect(Semi, "`;`");
            self.finish_node();
        }
        while !self.eof() {
            let before = self.pos;
            let mut n = 0;
            while self.nth(n) == Some(KwFinal) {
                n += 1;
            }
            if self.nth(n).is_some_and(is_class_prefix) {
                self.class_def();
            } else {
                self.garbage();
            }
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    /// 読めない並びを `;` まで（`;` を含めて）`Error` 節点に入れる
    fn garbage(&mut self) {
        self.error("unexpected token");
        self.start(Error);
        self.scan(Scan::Stmt);
        self.eat(Semi);
        self.finish_node();
    }

    fn name(&mut self) {
        self.start(Name);
        self.eat(Dot);
        if !self.eat(Ident) {
            self.error("expected name");
        }
        while self.at(Dot) && self.nth(1) == Some(Ident) {
            self.bump();
            self.bump();
        }
        self.finish_node();
    }

    fn group_node(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.scan(Scan::Group);
        self.finish_node();
    }

    /// 説明文字列（`"a" + "b"`）
    fn comment(&mut self) {
        if !self.at(Str) {
            return;
        }
        self.start(Comment);
        self.bump();
        while self.at(Plus) && self.nth(1) == Some(Str) {
            self.bump();
            self.bump();
        }
        self.finish_node();
    }

    fn annotation(&mut self) {
        if !self.at(KwAnnotation) {
            return;
        }
        self.start(Annotation);
        self.bump();
        if self.at(LParen) {
            self.scan(Scan::Group);
        } else {
            self.error("expected `(`");
        }
        self.finish_node();
    }

    fn class_def(&mut self) {
        if self.depth >= MAX_DEPTH {
            return self.garbage();
        }
        self.depth += 1;
        self.start(ClassDef);
        while self
            .cur()
            .is_some_and(|k| is_element_prefix(k) || is_class_prefix(k))
        {
            self.bump();
        }
        if self.at(KwExtends) {
            // class extends X(...) … end X;
            self.bump();
            self.eat(Ident);
            if self.at(LParen) {
                self.group_node(Modification);
            }
        } else if !self.eat(Ident) {
            self.error("expected class name");
        }
        if self.at(Eq) {
            self.start(ShortBody);
            self.bump();
            self.scan(Scan::Stmt);
            self.finish_node();
            self.expect(Semi, "`;`");
        } else {
            self.comment();
            self.composition();
            if self.eat(KwEnd) {
                if !self.eat(Ident) {
                    self.error("expected class name after `end`");
                }
            } else {
                self.error("expected `end`");
            }
            self.expect(Semi, "`;`");
        }
        self.finish_node();
        self.depth -= 1;
    }

    fn composition(&mut self) {
        while !self.eof() && !self.at(KwEnd) {
            let before = self.pos;
            match self.cur() {
                Some(KwPublic | KwProtected) => {
                    self.start(Visibility);
                    self.bump();
                    self.finish_node();
                }
                Some(KwEquation) => self.section(EquationSection, true),
                Some(KwAlgorithm) => self.section(AlgorithmSection, false),
                Some(KwInitial) if matches!(self.nth(1), Some(KwEquation | KwAlgorithm)) => {
                    let eq = self.nth(1) == Some(KwEquation);
                    self.section(
                        if eq {
                            EquationSection
                        } else {
                            AlgorithmSection
                        },
                        eq,
                    );
                }
                Some(KwExternal) => {
                    self.start(ExternalClause);
                    self.bump();
                    self.scan(Scan::Stmt);
                    self.expect(Semi, "`;`");
                    self.finish_node();
                }
                Some(KwAnnotation) => {
                    self.annotation();
                    self.expect(Semi, "`;`");
                }
                Some(KwExtends) => self.extends_clause(),
                Some(KwImport) => self.import_clause(),
                Some(Semi) => self.bump_error("stray `;`"),
                _ => {
                    let mut n = 0;
                    while self.nth(n).is_some_and(is_element_prefix) {
                        n += 1;
                    }
                    if self.nth(n).is_some_and(is_class_prefix) {
                        self.class_def();
                    } else {
                        self.component_clause();
                    }
                }
            }
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    fn extends_clause(&mut self) {
        self.start(ExtendsClause);
        self.bump();
        if matches!(self.cur(), Some(Ident | Dot)) {
            self.name();
        } else {
            self.error("expected name");
        }
        if self.at(LParen) {
            self.group_node(Modification);
        }
        self.annotation();
        self.expect(Semi, "`;`");
        self.finish_node();
    }

    fn import_clause(&mut self) {
        self.start(ImportClause);
        self.bump();
        if self.at(Ident) && self.nth(1) == Some(Eq) {
            self.bump();
            self.bump();
        }
        if matches!(self.cur(), Some(Ident | Dot)) {
            self.name();
        } else {
            self.error("expected name");
        }
        // `.*` は 1 つの字句（DotStar）
        if !self.eat(DotStar) && self.eat(Dot) {
            if self.at(LBrace) {
                self.scan(Scan::Group);
            } else if !self.eat(Star) {
                self.error("expected `*` or `{`");
            }
        }
        self.comment();
        self.annotation();
        self.expect(Semi, "`;`");
        self.finish_node();
    }

    fn component_clause(&mut self) {
        self.start(ComponentClause);
        while self.cur().is_some_and(is_component_prefix) {
            self.bump();
        }
        if !matches!(self.cur(), Some(Ident | Dot)) {
            self.error("expected type name");
            self.scan(Scan::Stmt);
            self.eat(Semi);
            self.finish_node();
            return;
        }
        self.name();
        if self.at(LBracket) {
            self.group_node(Subscripts);
        }
        loop {
            self.declaration();
            if !self.eat(Comma) {
                break;
            }
        }
        if self.at(KwConstrainedby) {
            self.bump();
            self.scan(Scan::Expr);
            self.comment();
            self.annotation();
        }
        self.expect(Semi, "`;`");
        self.finish_node();
    }

    fn declaration(&mut self) {
        self.start(Declaration);
        if !self.eat(Ident) {
            self.error("expected component name");
        }
        if self.at(LBracket) {
            self.group_node(Subscripts);
        }
        if matches!(self.cur(), Some(LParen | Eq | Assign)) {
            self.start(Modification);
            if self.at(LParen) {
                self.scan(Scan::Group);
            }
            if self.eat(Eq) || self.eat(Assign) {
                self.scan(Scan::Expr);
            }
            self.finish_node();
        }
        if self.at(KwIf) {
            self.start(Condition);
            self.bump();
            self.scan(Scan::Expr);
            self.finish_node();
        }
        self.comment();
        self.annotation();
        self.finish_node();
    }

    fn section(&mut self, kind: SyntaxKind, eq: bool) {
        self.start(kind);
        self.eat(KwInitial);
        self.bump();
        self.statements(eq, false);
        self.finish_node();
    }

    /// 文を `;` ごとに読む。節の終わり・`else` 系・`end` で止まる（食べない）。
    fn statements(&mut self, eq: bool, in_compound: bool) {
        while !self.eof() {
            let before = self.pos;
            match self.cur() {
                Some(KwElse | KwElseif | KwElsewhen) if in_compound => break,
                Some(
                    KwEquation | KwAlgorithm | KwPublic | KwProtected | KwExternal | KwAnnotation,
                ) => break,
                Some(KwInitial) if matches!(self.nth(1), Some(KwEquation | KwAlgorithm)) => break,
                Some(KwEnd)
                    if in_compound
                        || !matches!(self.nth(1), Some(KwIf | KwFor | KwWhen | KwWhile)) =>
                {
                    break;
                }
                Some(KwElse | KwElseif | KwElsewhen) => self.bump_error("`else` without `if`"),
                Some(KwEnd) => {
                    // 対応する if / for / when が無い `end if;`
                    self.error("`end` without a block");
                    self.start(Error);
                    self.scan(Scan::Stmt);
                    self.eat(Semi);
                    self.finish_node();
                }
                Some(KwIf | KwFor | KwWhen | KwWhile) if self.depth < MAX_DEPTH => {
                    self.compound(eq)
                }
                Some(Semi) => self.bump_error("empty statement"),
                _ => {
                    self.start(if eq { Equation } else { Statement });
                    self.scan(Scan::Stmt);
                    self.expect(Semi, "`;`");
                    self.finish_node();
                }
            }
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    /// `if … then … elseif … else … end if;`、`for … loop … end for;`、`when … elsewhen … end when;`
    fn compound(&mut self, eq: bool) {
        self.depth += 1;
        let kw = self.cur().unwrap_or(KwIf);
        self.start(if eq { Equation } else { Statement });
        self.bump();
        loop {
            self.start(Expr);
            self.scan(Scan::Header);
            self.finish_node();
            if !(self.eat(KwThen) || self.eat(KwLoop)) {
                self.error("expected `then` or `loop`");
            }
            let mut more = false;
            loop {
                self.statements(eq, true);
                if matches!(self.cur(), Some(KwElseif | KwElsewhen)) {
                    self.bump();
                    more = true;
                    break;
                }
                if !self.eat(KwElse) {
                    break;
                }
            }
            if !more {
                break;
            }
        }
        if self.at(KwEnd) && self.nth(1) == Some(kw) {
            self.bump();
            self.bump();
        } else {
            self.error("expected `end` of the block");
        }
        self.expect(Semi, "`;`");
        self.finish_node();
        self.depth -= 1;
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32))
}

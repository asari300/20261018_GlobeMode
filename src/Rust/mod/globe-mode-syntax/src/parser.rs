//! 寛容なパーサ。誤りがあっても必ず木を作り、読めなかったところは `Error` 節点に入れて診断を残す。
//! 全トークン（空白・コメント含む）が木に入るので、木の文字列は元の文字列と一致する。
//!
//! 範囲: クラスの定義・`extends`・`import`・部品の宣言・`equation` / `algorithm` 節（S268）、
//! 式（Pratt パーサ）・修飾・注釈・方程式と文の中身（S269）。
//! 不透明なまま残すもの: `redeclare` / `replaceable` の引数、`import A.{…}` の中、`enumeration(…)`、
//! 誤りから戻るときに読み飛ばす範囲（`Error` 節点）。

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
    /// 誤りから戻る・不透明な文。`;` と、ブロックの終わり（`end` / `else` 系）で止まる
    Stmt,
    /// `(` `[` `{` から対応する閉じまで
    Group,
    /// `redeclare` の引数。`,` か `)` で止まる
    Arg,
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
        while let Some(k) = self.cur() {
            if self.is_boundary() {
                break;
            }
            let top_level = stack.is_empty();
            match k {
                Semi if stack.last() != Some(&LBracket) => break,
                KwEnd | KwElse | KwElseif | KwElsewhen if mode == Scan::Stmt && top_level => break,
                Comma | RParen if mode == Scan::Arg && top_level => break,
                _ => {}
            }
            match k {
                LParen | LBracket | LBrace => stack.push(k),
                RParen | RBracket | RBrace => {
                    stack.pop();
                }
                _ => {}
            }
            self.bump();
            if mode == Scan::Group && stack.is_empty() {
                return;
            }
        }
        if !stack.is_empty() {
            self.error("unclosed bracket");
        }
    }

    /// 文・宣言の終わりの `;`。無ければ誤りにして、`;` まで（ブロックの終わりの手前まで）を `Error` に入れる
    fn semi(&mut self) {
        if self.eat(Semi) {
            return;
        }
        self.error("expected `;`");
        if self.eof()
            || self.is_boundary()
            || matches!(self.cur(), Some(KwEnd | KwElse | KwElseif | KwElsewhen))
        {
            return;
        }
        self.start(Error);
        self.scan(Scan::Stmt);
        self.eat(Semi);
        self.finish_node();
    }

    // ---- 式（Pratt）
    /// 式を始められないが、読み飛ばしてはいけないトークン（構造を作る語・閉じ・区切り）
    fn is_stop(k: SyntaxKind) -> bool {
        matches!(
            k,
            Semi | Comma | RParen | RBracket | RBrace | Eq | Assign | Colon
        ) || ((KwAlgorithm..=KwWithin).contains(&k)
            && !matches!(
                k,
                KwNot
                    | KwDer
                    | KwInitial
                    | KwIf
                    | KwFunction
                    | KwTrue
                    | KwFalse
                    | KwEnd
                    | KwConnect
            ))
    }

    /// 1 つの式（if 式・範囲を含む）
    fn expr(&mut self) {
        self.eat_trivia();
        let cp = self.builder.checkpoint();
        self.expr_bp(1);
        if self.at(Colon) && self.depth < MAX_DEPTH {
            self.builder
                .start_node_at(cp, rowan::SyntaxKind(RangeExpr as u16));
            for _ in 0..2 {
                if !self.eat(Colon) {
                    break;
                }
                self.expr_bp(1);
            }
            self.finish_node();
        }
    }

    /// 優先順位（左・右の束縛力）。`^` は左結合の連鎖として平らに持つ（仕様では連鎖は不可）
    fn infix(k: SyntaxKind) -> Option<(u8, u8)> {
        Some(match k {
            KwOr => (1, 2),
            KwAnd => (3, 4),
            Lt | Le | Gt | Ge | EqEq | Ne => (5, 6),
            Plus | Minus | DotPlus | DotMinus => (7, 8),
            Star | Slash | DotStar | DotSlash => (9, 10),
            Caret | DotCaret => (11, 12),
            _ => return None,
        })
    }

    fn expr_bp(&mut self, min: u8) {
        if self.depth >= MAX_DEPTH {
            self.error("expression is nested too deeply");
            // 打ち切り: 閉じている範囲（括弧の組）か 1 トークンを読み飛ばす
            if matches!(self.cur(), Some(LParen | LBracket | LBrace)) {
                self.start(Error);
                self.scan(Scan::Group);
                self.finish_node();
            } else if self.cur().is_some_and(|k| !Self::is_stop(k)) {
                self.bump_error("expression is nested too deeply");
            }
            return;
        }
        self.depth += 1;
        self.eat_trivia();
        let cp = self.builder.checkpoint();
        self.unary_or_atom(min);
        let mut open: Option<u8> = None;
        while let Some((l, r)) = self.cur().and_then(Self::infix) {
            if l < min {
                break;
            }
            if open != Some(l) {
                if open.is_some() {
                    self.finish_node();
                }
                self.builder
                    .start_node_at(cp, rowan::SyntaxKind(BinaryExpr as u16));
                open = Some(l);
            }
            self.bump();
            self.expr_bp(r);
        }
        if open.is_some() {
            self.finish_node();
        }
        self.depth -= 1;
    }

    fn unary_or_atom(&mut self, min: u8) {
        match self.cur() {
            Some(Plus | Minus | DotPlus | DotMinus) => {
                self.start(UnaryExpr);
                self.bump();
                self.expr_bp(min.max(9));
                self.finish_node();
            }
            Some(KwNot) => {
                self.start(UnaryExpr);
                self.bump();
                self.expr_bp(min.max(5));
                self.finish_node();
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) {
        match self.cur() {
            Some(UInt | UReal | Str | KwTrue | KwFalse) => {
                self.start(Literal);
                self.bump();
                self.finish_node();
            }
            Some(KwEnd) => {
                self.start(EndExpr);
                self.bump();
                self.finish_node();
            }
            Some(Ident | Dot) => {
                self.eat_trivia();
                let cp = self.builder.checkpoint();
                self.component_ref();
                if self.at(LParen) {
                    self.builder
                        .start_node_at(cp, rowan::SyntaxKind(CallExpr as u16));
                    self.function_args();
                    self.finish_node();
                }
            }
            Some(KwDer | KwInitial | KwConnect) => {
                self.start(CallExpr);
                self.bump();
                if self.at(LParen) {
                    self.function_args();
                } else {
                    self.error("expected `(`");
                }
                self.finish_node();
            }
            Some(KwFunction) => {
                self.start(PartialApp);
                self.bump();
                if matches!(self.cur(), Some(Ident | Dot)) {
                    self.component_ref();
                } else {
                    self.error("expected function name");
                }
                if self.at(LParen) {
                    self.function_args();
                }
                self.finish_node();
            }
            Some(KwIf) => self.if_expr(),
            Some(LParen) => {
                self.start(ParenExpr);
                self.bump();
                while !self.at(RParen) {
                    let before = self.pos;
                    if !self.at(Comma) {
                        self.expr();
                    }
                    if !self.eat(Comma) || self.pos == before {
                        break;
                    }
                }
                self.close(RParen, "`)`");
                self.finish_node();
            }
            Some(LBracket) => {
                self.start(MatrixExpr);
                self.bump();
                while !self.at(RBracket) {
                    let before = self.pos;
                    self.expr();
                    if !(self.eat(Comma) || self.eat(Semi)) || self.pos == before {
                        break;
                    }
                }
                self.close(RBracket, "`]`");
                self.finish_node();
            }
            Some(LBrace) => {
                self.start(ArrayExpr);
                self.bump();
                self.arg_list(RBrace);
                self.close(RBrace, "`}`");
                self.finish_node();
            }
            Some(k) if !Self::is_stop(k) => self.bump_error("expected an expression"),
            _ => self.error("expected an expression"),
        }
    }

    fn close(&mut self, k: SyntaxKind, what: &str) {
        if !self.eat(k) {
            self.error(&format!("expected {what}"));
        }
    }

    fn component_ref(&mut self) {
        self.start(ComponentRef);
        self.eat(Dot);
        loop {
            if !self.eat(Ident) {
                self.error("expected name");
                break;
            }
            if self.at(LBracket) {
                self.subscripts();
            }
            if !(self.at(Dot) && self.nth(1) == Some(Ident)) {
                break;
            }
            self.bump();
        }
        self.finish_node();
    }

    /// `[` 添字 `,` … `]`。添字は `:` か式
    fn subscripts(&mut self) {
        self.start(Subscripts);
        self.bump();
        while !self.at(RBracket) {
            let before = self.pos;
            if !self.eat(Colon) {
                self.expr();
            }
            if !self.eat(Comma) || self.pos == before {
                break;
            }
        }
        self.close(RBracket, "`]`");
        self.finish_node();
    }

    fn if_expr(&mut self) {
        self.start(IfExpr);
        self.bump();
        loop {
            self.expr();
            self.close(KwThen, "`then`");
            self.expr();
            if !self.eat(KwElseif) {
                break;
            }
        }
        if self.eat(KwElse) {
            self.expr();
        } else {
            self.error("expected `else`");
        }
        self.finish_node();
    }

    /// `(` 引数 `,` … `)`。名前つき引数（`k = e`）と内包（`e for i in r`）を含む
    fn function_args(&mut self) {
        self.start(FunctionArgs);
        self.bump();
        self.arg_list(RParen);
        self.close(RParen, "`)`");
        self.finish_node();
    }

    fn arg_list(&mut self, close: SyntaxKind) {
        while !self.at(close) {
            let before = self.pos;
            if self.at(Ident) && self.nth(1) == Some(Eq) {
                self.start(NamedArg);
                self.bump();
                self.bump();
                self.expr();
                self.finish_node();
            } else {
                self.eat_trivia();
                let cp = self.builder.checkpoint();
                self.expr();
                if self.at(KwFor) {
                    self.builder
                        .start_node_at(cp, rowan::SyntaxKind(Comprehension as u16));
                    self.bump();
                    self.for_indices();
                    self.finish_node();
                }
            }
            if !self.eat(Comma) || self.pos == before {
                break;
            }
            if self.at(close) {
                self.error("expected an argument");
            }
        }
    }

    /// `i in 1:n, j in v`（`in` が無い添字も許す）
    fn for_indices(&mut self) {
        self.start(ForIndices);
        loop {
            self.start(ForIndex);
            if !self.eat(Ident) {
                self.error("expected index name");
            }
            if self.eat(KwIn) {
                self.expr();
            }
            self.finish_node();
            if !self.eat(Comma) {
                break;
            }
        }
        self.finish_node();
    }

    // ---- 修飾・注釈
    /// `( 要素の修飾 , … )`
    fn class_modification(&mut self) {
        if self.depth >= MAX_DEPTH {
            self.error("modification is nested too deeply");
            self.start(Error);
            self.scan(Scan::Group);
            self.finish_node();
            return;
        }
        self.depth += 1;
        self.start(ClassModification);
        self.bump();
        while !self.at(RParen) {
            let before = self.pos;
            let mut n = 0;
            while matches!(self.nth(n), Some(KwEach | KwFinal)) {
                n += 1;
            }
            if self.at(KwBreak) {
                // Modelica 3.6: `extends B(break x, break connect(a, b))`
                self.start(BreakModification);
                self.bump();
                if self.at(KwConnect) {
                    self.expr();
                } else if self.at(Ident) {
                    self.name();
                } else {
                    self.error("expected name");
                }
                self.finish_node();
            } else if matches!(self.nth(n), Some(KwRedeclare | KwReplaceable)) {
                self.start(Redeclaration);
                self.scan(Scan::Arg);
                self.finish_node();
            } else {
                self.start(ElementModification);
                while matches!(self.cur(), Some(KwEach | KwFinal)) {
                    self.bump();
                }
                if self.at(Ident) {
                    self.name();
                } else {
                    self.error("expected name");
                }
                if matches!(self.cur(), Some(LParen | Eq | Assign)) {
                    self.modification();
                }
                self.comment();
                self.finish_node();
            }
            if !self.eat(Comma) || self.pos == before {
                break;
            }
        }
        self.close(RParen, "`)`");
        self.finish_node();
        self.depth -= 1;
    }

    /// `(…)` と `= 式` / `:= 式`
    fn modification(&mut self) {
        self.start(Modification);
        if self.at(LParen) {
            self.class_modification();
        }
        if self.eat(Eq) || self.eat(Assign) {
            self.expr();
        }
        self.finish_node();
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
            self.class_modification();
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
                self.class_modification();
            }
        } else if !self.eat(Ident) {
            self.error("expected class name");
        }
        if self.at(Eq) {
            self.short_body();
            self.semi();
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

    /// `= [前置き] 型名 [添字] [(修飾)] [constrainedby …] [説明] [annotation]`、`= enumeration(…)`、`= der(…)`
    fn short_body(&mut self) {
        self.start(ShortBody);
        self.bump();
        while self.cur().is_some_and(is_component_prefix) {
            self.bump();
        }
        match self.cur() {
            Some(KwEnumeration | KwDer) => {
                // shortcut: 列挙の中身と `der(F, x)` の引数は不透明、使う層ができたら構造にする
                self.bump();
                if self.at(LParen) {
                    self.scan(Scan::Group);
                }
            }
            Some(Ident | Dot) => {
                self.name();
                if self.at(LBracket) {
                    self.subscripts();
                }
                if self.at(LParen) {
                    self.class_modification();
                }
            }
            _ => self.error("expected type name"),
        }
        self.constrainedby();
        self.comment();
        self.annotation();
        self.finish_node();
    }

    fn constrainedby(&mut self) {
        if self.at(KwConstrainedby) {
            self.bump();
            if matches!(self.cur(), Some(Ident | Dot)) {
                self.name();
            } else {
                self.error("expected name");
            }
            if self.at(LParen) {
                self.class_modification();
            }
        }
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
                    self.eat(Str);
                    if matches!(self.cur(), Some(Ident | Dot)) {
                        self.expr();
                        if self.eat(Eq) {
                            self.expr();
                        }
                    }
                    self.annotation();
                    self.semi();
                    self.finish_node();
                }
                Some(KwAnnotation) => {
                    self.annotation();
                    self.semi();
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
            self.class_modification();
        }
        self.annotation();
        self.semi();
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
        self.semi();
        self.finish_node();
    }

    fn component_clause(&mut self) {
        self.start(ComponentClause);
        while self.cur().is_some_and(is_component_prefix) {
            self.bump();
        }
        if !matches!(self.cur(), Some(Ident | Dot)) {
            self.error("expected type name");
            self.start(Error);
            self.scan(Scan::Stmt);
            self.eat(Semi);
            self.finish_node();
            self.finish_node();
            return;
        }
        self.name();
        if self.at(LBracket) {
            self.subscripts();
        }
        loop {
            self.declaration();
            if !self.eat(Comma) {
                break;
            }
        }
        if self.at(KwConstrainedby) {
            self.constrainedby();
            self.comment();
            self.annotation();
        }
        self.semi();
        self.finish_node();
    }

    fn declaration(&mut self) {
        self.start(Declaration);
        if !self.eat(Ident) {
            self.error("expected component name");
        }
        if self.at(LBracket) {
            self.subscripts();
        }
        if matches!(self.cur(), Some(LParen | Eq | Assign)) {
            self.modification();
        }
        if self.at(KwIf) {
            self.start(Condition);
            self.bump();
            self.expr();
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
                    self.bump();
                    self.scan(Scan::Stmt);
                    self.eat(Semi);
                    self.finish_node();
                }
                Some(KwIf | KwFor | KwWhen | KwWhile) if self.depth < MAX_DEPTH => {
                    self.compound(eq)
                }
                Some(Semi) => self.bump_error("empty statement"),
                _ => self.simple_item(eq),
            }
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    /// `a = b;`（方程式）、`a := b;`・`f(x);`・`(a, b) := f(x);`・`return;`・`break;`（文）。`connect(a, b);` と
    /// `f(x);` の呼び出しは式の `CallExpr` になる。説明文字列と annotation も付けられる
    fn simple_item(&mut self, eq: bool) {
        self.start(if eq { Equation } else { Statement });
        if !eq && matches!(self.cur(), Some(KwReturn | KwBreak)) {
            self.bump();
        } else {
            self.expr();
            if self.eat(if eq { Eq } else { Assign }) {
                self.expr();
            }
        }
        self.comment();
        self.annotation();
        self.semi();
        self.finish_node();
    }

    /// `if … then … elseif … else … end if;`、`for … loop … end for;`、`when … elsewhen … end when;`
    fn compound(&mut self, eq: bool) {
        self.depth += 1;
        let kw = self.cur().unwrap_or(KwIf);
        self.start(if eq { Equation } else { Statement });
        self.bump();
        loop {
            if kw == KwFor {
                self.for_indices();
            } else {
                self.expr();
            }
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

//! rowan の CST の上の薄い型（式の節点だけ。S269）。節点を `cast` で包み、子を取り出す。
//! 構造は CST のままで、複製はしない。文・宣言・クラスの型は B2（resolve）を書くときに足す。

use crate::{SyntaxKind, SyntaxNode, SyntaxToken};

macro_rules! ast_node {
    ($($name:ident = $kind:ident),* $(,)?) => {$(
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(SyntaxNode);
        impl $name {
            pub fn cast(n: SyntaxNode) -> Option<Self> {
                (n.kind() == SyntaxKind::$kind).then(|| Self(n))
            }
            pub fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    )*};
}

ast_node! {
    Literal = Literal, ComponentRef = ComponentRef, EndExpr = EndExpr, ParenExpr = ParenExpr,
    ArrayExpr = ArrayExpr, MatrixExpr = MatrixExpr, UnaryExpr = UnaryExpr, BinaryExpr = BinaryExpr,
    RangeExpr = RangeExpr, IfExpr = IfExpr, CallExpr = CallExpr, PartialApp = PartialApp,
    Comprehension = Comprehension,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    ComponentRef(ComponentRef),
    End(EndExpr),
    Paren(ParenExpr),
    Array(ArrayExpr),
    Matrix(MatrixExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Range(RangeExpr),
    If(IfExpr),
    Call(CallExpr),
    Partial(PartialApp),
    Comprehension(Comprehension),
}

impl Expr {
    pub fn cast(n: SyntaxNode) -> Option<Self> {
        Some(match n.kind() {
            SyntaxKind::Literal => Self::Literal(Literal(n)),
            SyntaxKind::ComponentRef => Self::ComponentRef(ComponentRef(n)),
            SyntaxKind::EndExpr => Self::End(EndExpr(n)),
            SyntaxKind::ParenExpr => Self::Paren(ParenExpr(n)),
            SyntaxKind::ArrayExpr => Self::Array(ArrayExpr(n)),
            SyntaxKind::MatrixExpr => Self::Matrix(MatrixExpr(n)),
            SyntaxKind::UnaryExpr => Self::Unary(UnaryExpr(n)),
            SyntaxKind::BinaryExpr => Self::Binary(BinaryExpr(n)),
            SyntaxKind::RangeExpr => Self::Range(RangeExpr(n)),
            SyntaxKind::IfExpr => Self::If(IfExpr(n)),
            SyntaxKind::CallExpr => Self::Call(CallExpr(n)),
            SyntaxKind::PartialApp => Self::Partial(PartialApp(n)),
            SyntaxKind::Comprehension => Self::Comprehension(Comprehension(n)),
            _ => return None,
        })
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Literal(x) => &x.0,
            Self::ComponentRef(x) => &x.0,
            Self::End(x) => &x.0,
            Self::Paren(x) => &x.0,
            Self::Array(x) => &x.0,
            Self::Matrix(x) => &x.0,
            Self::Unary(x) => &x.0,
            Self::Binary(x) => &x.0,
            Self::Range(x) => &x.0,
            Self::If(x) => &x.0,
            Self::Call(x) => &x.0,
            Self::Partial(x) => &x.0,
            Self::Comprehension(x) => &x.0,
        }
    }
}

/// 子の式（節点）を順に。`FunctionArgs` などの中間の節点は降りない
fn exprs(n: &SyntaxNode) -> impl Iterator<Item = Expr> {
    n.children().filter_map(Expr::cast)
}

/// 子の字句（空白・コメントを除く）
fn tokens(n: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    n.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
}

impl Literal {
    pub fn token(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }
}

impl ComponentRef {
    /// `a.b[i].c` の `["a", "b", "c"]`（添字は含めない。先頭の `.` は `is_global`）
    pub fn parts(&self) -> Vec<String> {
        tokens(&self.0)
            .filter(|t| t.kind() == SyntaxKind::Ident)
            .map(|t| t.text().to_string())
            .collect()
    }
    pub fn is_global(&self) -> bool {
        tokens(&self.0)
            .next()
            .is_some_and(|t| t.kind() == SyntaxKind::Dot)
    }
}

impl BinaryExpr {
    /// 同じ優先順位の連鎖（`a - b + c`）は被演算子 3 つ・演算子 2 つ。左から結合する
    pub fn operands(&self) -> Vec<Expr> {
        exprs(&self.0).collect()
    }
    pub fn ops(&self) -> Vec<SyntaxToken> {
        tokens(&self.0).collect()
    }
}

impl UnaryExpr {
    pub fn op(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }
    pub fn operand(&self) -> Option<Expr> {
        exprs(&self.0).next()
    }
}

impl RangeExpr {
    /// `a:b` は 2 つ、`a:b:c` は 3 つ（`start:step:stop`）
    pub fn parts(&self) -> Vec<Expr> {
        exprs(&self.0).collect()
    }
}

impl IfExpr {
    /// `(条件, 値)` の組（`if` と `elseif`）
    pub fn branches(&self) -> Vec<(Expr, Expr)> {
        let v: Vec<_> = exprs(&self.0).collect();
        v.as_chunks::<2>()
            .0
            .iter()
            .map(|[c, v]| (c.clone(), v.clone()))
            .collect()
    }
    pub fn else_branch(&self) -> Option<Expr> {
        let v: Vec<_> = exprs(&self.0).collect();
        (v.len() % 2 == 1).then(|| v[v.len() - 1].clone())
    }
}

impl CallExpr {
    /// 呼び出す名前。`der` / `initial` / `connect` は予約語のまま
    pub fn callee(&self) -> Option<String> {
        self.0.children_with_tokens().find_map(|e| match e {
            rowan::NodeOrToken::Node(n) if n.kind() == SyntaxKind::ComponentRef => {
                ComponentRef::cast(n).map(|c| c.parts().join("."))
            }
            rowan::NodeOrToken::Token(t) if !t.kind().is_trivia() => Some(t.text().to_string()),
            _ => None,
        })
    }
    fn args_node(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|c| c.kind() == SyntaxKind::FunctionArgs)
    }
    /// 位置引数（`for` つきの内包を含む）
    pub fn args(&self) -> Vec<Expr> {
        self.args_node()
            .map(|n| exprs(&n).collect())
            .unwrap_or_default()
    }
    /// 名前つき引数 `k = e`
    pub fn named_args(&self) -> Vec<(String, Expr)> {
        let Some(n) = self.args_node() else {
            return Vec::new();
        };
        n.children()
            .filter(|c| c.kind() == SyntaxKind::NamedArg)
            .filter_map(|c| {
                let name = tokens(&c)
                    .find(|t| t.kind() == SyntaxKind::Ident)?
                    .text()
                    .to_string();
                Some((name, exprs(&c).next()?))
            })
            .collect()
    }
}

impl ParenExpr {
    /// `(a, b)` の要素（`(a, , b)` の空の位置は含まない）
    pub fn items(&self) -> Vec<Expr> {
        exprs(&self.0).collect()
    }
}

impl ArrayExpr {
    pub fn items(&self) -> Vec<Expr> {
        self.0.children().filter_map(Expr::cast).collect()
    }
}

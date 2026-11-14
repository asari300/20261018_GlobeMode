//! rowan の CST の上の薄い型（式の節点だけ。S269）。節点を `cast` で包み、子を取り出す。
//! 構造は CST のままで、複製はしない。S272 でクラス・宣言・`extends`・`import` の型を足した
//! （文の型はまだ無い。`Equation` / `Statement` は節点のまま使う）。

use crate::{SyntaxKind, SyntaxNode, SyntaxToken};
use rowan::TextRange;

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
    Comprehension = Comprehension, StoredDefinition = StoredDefinition, ClassDef = ClassDef,
    ShortBody = ShortBody, ExtendsClause = ExtendsClause, ImportClause = ImportClause,
    ComponentClause = ComponentClause, Declaration = Declaration, Name = Name,
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

// ---- クラス・宣言・extends・import（S272）

fn name_parts(n: &SyntaxNode) -> Vec<String> {
    tokens(n)
        .filter(|t| t.kind() == SyntaxKind::Ident)
        .map(|t| t.text().to_string())
        .collect()
}

impl Name {
    /// `A.B.C` の `["A", "B", "C"]`（先頭の `.` は `is_global`）
    pub fn parts(&self) -> Vec<String> {
        name_parts(&self.0)
    }
    pub fn is_global(&self) -> bool {
        tokens(&self.0)
            .next()
            .is_some_and(|t| t.kind() == SyntaxKind::Dot)
    }
    pub fn range(&self) -> TextRange {
        self.0.text_range()
    }
}

impl ComponentRef {
    pub fn range(&self) -> TextRange {
        self.0.text_range()
    }
}

impl StoredDefinition {
    /// `within A.B;` は `Some(["A", "B"])`、`within;` は `Some([])`、無ければ `None`
    pub fn within(&self) -> Option<Vec<String>> {
        let w = self
            .0
            .children()
            .find(|c| c.kind() == SyntaxKind::WithinClause)?;
        Some(
            w.children()
                .find_map(Name::cast)
                .map(|n| n.parts())
                .unwrap_or_default(),
        )
    }
    pub fn classes(&self) -> Vec<ClassDef> {
        self.0.children().filter_map(ClassDef::cast).collect()
    }
}

/// クラスの中の要素（宣言の順）。`Visibility` は以降の要素の公開・非公開を切り替える
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Class(ClassDef),
    Extends(ExtendsClause),
    Import(ImportClause),
    Component(ComponentClause),
    /// `public`（true）/ `protected`（false）
    Visibility(bool),
}

impl ClassDef {
    fn direct_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        tokens(&self.0)
    }
    fn name_token(&self) -> Option<SyntaxToken> {
        self.direct_tokens().find(|t| t.kind() == SyntaxKind::Ident)
    }
    /// `class extends X` では `X`
    pub fn name(&self) -> Option<String> {
        self.name_token().map(|t| t.text().to_string())
    }
    pub fn name_range(&self) -> Option<TextRange> {
        self.name_token().map(|t| t.text_range())
    }
    fn has_prefix(&self, k: SyntaxKind) -> bool {
        // 名前より前の字句だけ（`end X` の後ろは見ない）
        self.direct_tokens()
            .take_while(|t| t.kind() != SyntaxKind::Ident)
            .any(|t| t.kind() == k)
    }
    pub fn is_encapsulated(&self) -> bool {
        self.has_prefix(SyntaxKind::KwEncapsulated)
    }
    pub fn is_replaceable(&self) -> bool {
        self.has_prefix(SyntaxKind::KwReplaceable)
    }
    pub fn is_class_extends(&self) -> bool {
        self.has_prefix(SyntaxKind::KwExtends)
    }
    /// `class` / `model` / `package` など（`record` や `function` の前置き `operator` は別の字句）
    pub fn kind_token(&self) -> Option<SyntaxToken> {
        self.direct_tokens()
            .take_while(|t| t.kind() != SyntaxKind::Ident)
            .filter(|t| {
                use SyntaxKind::*;
                matches!(
                    t.kind(),
                    KwClass
                        | KwModel
                        | KwBlock
                        | KwConnector
                        | KwType
                        | KwPackage
                        | KwRecord
                        | KwFunction
                )
            })
            .last()
    }
    pub fn short_body(&self) -> Option<ShortBody> {
        self.0.children().find_map(ShortBody::cast)
    }
    pub fn elements(&self) -> Vec<Element> {
        let mut v = Vec::new();
        for c in self.0.children() {
            match c.kind() {
                SyntaxKind::Visibility => v.push(Element::Visibility(
                    tokens(&c).any(|t| t.kind() == SyntaxKind::KwPublic),
                )),
                SyntaxKind::ClassDef => v.extend(ClassDef::cast(c).map(Element::Class)),
                SyntaxKind::ExtendsClause => v.extend(ExtendsClause::cast(c).map(Element::Extends)),
                SyntaxKind::ImportClause => v.extend(ImportClause::cast(c).map(Element::Import)),
                SyntaxKind::ComponentClause => {
                    v.extend(ComponentClause::cast(c).map(Element::Component))
                }
                _ => {}
            }
        }
        v
    }
}

impl ShortBody {
    /// `= Base(...)` の `Base`（`enumeration(...)` と `der(...)` には無い）
    pub fn base(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }
    /// `= enumeration(A, B "c", ...)` の `["A", "B"]`と、その字句の範囲。`enumeration(:)` は空
    pub fn enum_literals(&self) -> Vec<(String, TextRange)> {
        let mut depth = 0;
        let mut prev = None;
        let mut seen = false;
        let mut out = Vec::new();
        for t in tokens(&self.0) {
            match t.kind() {
                SyntaxKind::KwEnumeration => seen = true,
                SyntaxKind::LParen if seen => depth += 1,
                SyntaxKind::RParen if seen => depth -= 1,
                SyntaxKind::Ident
                    if seen
                        && depth == 1
                        && matches!(prev, Some(SyntaxKind::LParen | SyntaxKind::Comma)) =>
                {
                    out.push((t.text().to_string(), t.text_range()))
                }
                _ => {}
            }
            prev = Some(t.kind());
        }
        out
    }
}

/// `extends` の直後の `(…)` に `redeclare` / `replaceable` があるか（型が差し替わりうる印）
fn has_redeclare(n: &SyntaxNode) -> bool {
    n.children()
        .filter(|c| c.kind() == SyntaxKind::ClassModification)
        .any(|m| {
            m.descendants()
                .any(|d| d.kind() == SyntaxKind::Redeclaration)
        })
}

impl ShortBody {
    pub fn has_redeclare(&self) -> bool {
        has_redeclare(&self.0)
    }
    pub fn redeclared_classes(&self) -> Vec<RedeclaredClass> {
        redeclared_classes(&self.0)
    }
}

impl ExtendsClause {
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }
    pub fn has_redeclare(&self) -> bool {
        has_redeclare(&self.0)
    }
    pub fn redeclared_classes(&self) -> Vec<RedeclaredClass> {
        redeclared_classes(&self.0)
    }
}

/// 修飾の中の `redeclare [replaceable] package M = X [constrainedby Y]`（短いクラスの形だけ。S275）。
/// 部品の再宣言（`redeclare Real x`）と長い形（`redeclare model M … end M`）は含まない。パスは `(部分, 先頭の . の有無)`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedeclaredClass {
    pub name: String,
    pub base: (Vec<String>, bool),
    pub constraint: Option<(Vec<String>, bool)>,
}

/// `n` の直下の `(…)` と、宣言の `Modification` の中の `(…)` にある `Redeclaration`（入れ子の修飾の中は見ない）
fn redeclared_classes(n: &SyntaxNode) -> Vec<RedeclaredClass> {
    let mods = n.children().flat_map(|c| match c.kind() {
        SyntaxKind::Modification => c.children().collect::<Vec<_>>(),
        _ => vec![c],
    });
    mods.filter(|m| m.kind() == SyntaxKind::ClassModification)
        .flat_map(|m| m.children().collect::<Vec<_>>())
        .filter(|r| r.kind() == SyntaxKind::Redeclaration)
        .filter_map(|r| parse_redeclared_class(&r))
        .collect()
}

fn parse_redeclared_class(r: &SyntaxNode) -> Option<RedeclaredClass> {
    use SyntaxKind::*;
    let t: Vec<_> = tokens(r).collect();
    let mut i = 0;
    while t
        .get(i)
        .is_some_and(|t| matches!(t.kind(), KwRedeclare | KwReplaceable | KwEach | KwFinal))
    {
        i += 1;
    }
    if !matches!(
        t.get(i)?.kind(),
        KwClass | KwModel | KwBlock | KwConnector | KwType | KwPackage | KwRecord | KwFunction
    ) {
        return None;
    }
    let name = t
        .get(i + 1)
        .filter(|t| t.kind() == Ident)?
        .text()
        .to_string();
    if t.get(i + 2)?.kind() != Eq {
        return None;
    }
    i += 3;
    let path = |i: &mut usize| {
        let global = t.get(*i).is_some_and(|t| t.kind() == Dot);
        *i += usize::from(global);
        let mut v = Vec::new();
        while let Some(id) = t.get(*i).filter(|t| t.kind() == Ident) {
            v.push(id.text().to_string());
            *i += 1;
            if t.get(*i).is_some_and(|t| t.kind() == Dot) {
                *i += 1;
            } else {
                break;
            }
        }
        (!v.is_empty()).then_some((v, global))
    };
    let base = path(&mut i)?;
    // `Base(…)` の修飾は読み飛ばして `constrainedby` を探す
    let mut depth = 0;
    let mut constraint = None;
    while let Some(tok) = t.get(i) {
        match tok.kind() {
            LParen => depth += 1,
            RParen => depth -= 1,
            KwConstrainedby if depth == 0 => {
                i += 1;
                constraint = path(&mut i);
                break;
            }
            _ => {}
        }
        i += 1;
    }
    Some(RedeclaredClass {
        name,
        base,
        constraint,
    })
}

/// `import` の形（S272）。パスは完全修飾（`Modelica.Units.SI`）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportKind {
    /// `import A.B.C;`（`C` が入る）
    Qualified(Vec<String>),
    /// `import X = A.B;`
    Alias(String, Vec<String>),
    /// `import A.B.*;`
    Unqualified(Vec<String>),
    /// `import A.B.{C, D};`
    Multi(Vec<String>, Vec<String>),
}

impl ImportClause {
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }
    pub fn kind(&self) -> Option<ImportKind> {
        let path = self.name()?.parts();
        let toks: Vec<_> = tokens(&self.0).collect();
        // toks[0] は `import`
        if toks.get(2).is_some_and(|t| t.kind() == SyntaxKind::Eq) {
            return Some(ImportKind::Alias(toks[1].text().to_string(), path));
        }
        let name_end = self.name()?.0.text_range().end();
        let rest: Vec<_> = toks
            .iter()
            .filter(|t| t.text_range().start() >= name_end)
            .collect();
        Some(match rest.first().map(|t| t.kind()) {
            Some(SyntaxKind::DotStar) => ImportKind::Unqualified(path),
            Some(SyntaxKind::Dot) if rest.get(1).is_some_and(|t| t.kind() == SyntaxKind::Star) => {
                ImportKind::Unqualified(path)
            }
            Some(SyntaxKind::Dot) => {
                let mut prev = SyntaxKind::Dot;
                let mut names = Vec::new();
                for t in &rest[1..] {
                    if t.kind() == SyntaxKind::Ident
                        && matches!(prev, SyntaxKind::LBrace | SyntaxKind::Comma)
                    {
                        names.push(t.text().to_string());
                    }
                    prev = t.kind();
                }
                ImportKind::Multi(path, names)
            }
            _ => ImportKind::Qualified(path),
        })
    }
}

impl ComponentClause {
    /// 型名（最初の `Name`。`constrainedby` の `Name` は `constraint`）
    pub fn type_name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }
    pub fn constraint(&self) -> Option<Name> {
        self.0.children().filter_map(Name::cast).nth(1)
    }
    pub fn declarations(&self) -> Vec<Declaration> {
        self.0.children().filter_map(Declaration::cast).collect()
    }
}

impl Declaration {
    pub fn redeclared_classes(&self) -> Vec<RedeclaredClass> {
        redeclared_classes(&self.0)
    }
    pub fn name(&self) -> Option<(String, TextRange)> {
        let t = tokens(&self.0).find(|t| t.kind() == SyntaxKind::Ident)?;
        Some((t.text().to_string(), t.text_range()))
    }
}

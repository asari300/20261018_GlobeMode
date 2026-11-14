//! 字句と節点の種類。トークンは前、節点は後ろ。

macro_rules! kinds {
    ($($name:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum SyntaxKind { $($name),* }
        impl SyntaxKind {
            const ALL: &'static [SyntaxKind] = &[$(SyntaxKind::$name),*];
        }
    };
}

kinds! {
    // ---- 空白・コメント（trivia。木には残る）
    Whitespace, LineComment, BlockComment,
    // ---- 字句
    Ident, UInt, UReal, Str, ErrorToken,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Dot, Comma, Semi, Colon, Eq, EqEq, Ne, Lt, Le, Gt, Ge, Assign,
    Plus, Minus, Star, Slash, Caret, DotPlus, DotMinus, DotStar, DotSlash, DotCaret,
    // ---- 予約語（Modelica 3.6 の 2.3.3）
    KwAlgorithm, KwAnd, KwAnnotation, KwBlock, KwBreak, KwClass, KwConnect, KwConnector,
    KwConstant, KwConstrainedby, KwDer, KwDiscrete, KwEach, KwElse, KwElseif, KwElsewhen,
    KwEncapsulated, KwEnd, KwEnumeration, KwEquation, KwExpandable, KwExtends, KwExternal,
    KwFalse, KwFinal, KwFlow, KwFor, KwFunction, KwIf, KwImport, KwImpure, KwIn, KwInitial,
    KwInner, KwInput, KwLoop, KwModel, KwNot, KwOperator, KwOr, KwOuter, KwOutput, KwPackage,
    KwParameter, KwPartial, KwProtected, KwPublic, KwPure, KwRecord, KwRedeclare,
    KwReplaceable, KwReturn, KwStream, KwThen, KwTrue, KwType, KwWhen, KwWhile, KwWithin,
    // ---- 節点
    StoredDefinition, WithinClause, ClassDef, ShortBody, Visibility, ExtendsClause, ImportClause,
    ComponentClause, Name, Subscripts, Declaration, Modification, Condition, Comment, Annotation,
    EquationSection, AlgorithmSection, ExternalClause, Equation, Statement, Expr,
    // 読めなかったところ（診断と対）
    Error,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::LineComment | Self::BlockComment
        )
    }
}

pub(crate) const KEYWORDS: &[(&str, SyntaxKind)] = {
    use SyntaxKind::*;
    &[
        ("algorithm", KwAlgorithm),
        ("and", KwAnd),
        ("annotation", KwAnnotation),
        ("block", KwBlock),
        ("break", KwBreak),
        ("class", KwClass),
        ("connect", KwConnect),
        ("connector", KwConnector),
        ("constant", KwConstant),
        ("constrainedby", KwConstrainedby),
        ("der", KwDer),
        ("discrete", KwDiscrete),
        ("each", KwEach),
        ("else", KwElse),
        ("elseif", KwElseif),
        ("elsewhen", KwElsewhen),
        ("encapsulated", KwEncapsulated),
        ("end", KwEnd),
        ("enumeration", KwEnumeration),
        ("equation", KwEquation),
        ("expandable", KwExpandable),
        ("extends", KwExtends),
        ("external", KwExternal),
        ("false", KwFalse),
        ("final", KwFinal),
        ("flow", KwFlow),
        ("for", KwFor),
        ("function", KwFunction),
        ("if", KwIf),
        ("import", KwImport),
        ("impure", KwImpure),
        ("in", KwIn),
        ("initial", KwInitial),
        ("inner", KwInner),
        ("input", KwInput),
        ("loop", KwLoop),
        ("model", KwModel),
        ("not", KwNot),
        ("operator", KwOperator),
        ("or", KwOr),
        ("outer", KwOuter),
        ("output", KwOutput),
        ("package", KwPackage),
        ("parameter", KwParameter),
        ("partial", KwPartial),
        ("protected", KwProtected),
        ("public", KwPublic),
        ("pure", KwPure),
        ("record", KwRecord),
        ("redeclare", KwRedeclare),
        ("replaceable", KwReplaceable),
        ("return", KwReturn),
        ("stream", KwStream),
        ("then", KwThen),
        ("true", KwTrue),
        ("type", KwType),
        ("when", KwWhen),
        ("while", KwWhile),
        ("within", KwWithin),
    ]
};

/// rowan の `Language`。`SyntaxNode` / `SyntaxToken` はこの型で使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modelica {}

impl rowan::Language for Modelica {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::ALL[raw.0 as usize]
    }
    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Modelica>;
pub type SyntaxToken = rowan::SyntaxToken<Modelica>;

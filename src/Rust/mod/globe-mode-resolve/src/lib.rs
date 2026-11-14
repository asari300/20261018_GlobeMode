//! Track B の B2 の最初の一切れ（S272）: クラスと部品の名前解決。
//!
//! - `Library`: ライブラリ全体のクラスの木。`within` でパッケージの入れ子を組むので、`package.mo` のディレクトリ構造も
//!   単一ファイルも、`Modelica 4.1.0/` のように版つきのディレクトリも同じに扱う（`package.order` は順序だけなので読まない）。
//! - 解決する名前: 型名（部品・short class・`extends` の基底・`constrainedby`・`import`）と、式の中の成分参照の先頭（`a.b.c` の `a`）。
//!   先頭がクラスなら続きもクラスの要素としてたどる。部品なら先頭で止める（続きは部品の型の要素で、型の解決を要する）。
//! - 探索は Modelica 3.6 の 5.3: 内側から外へ（宣言 → 受け継いだ要素 → `import`）、`encapsulated` で止めて最上位へ、組み込み。
//! - 未対応: `redeclare` / `replaceable` で型が変わる所（失敗に関わったら `Reason::Redeclare`）、`annotation` の中、
//!   `redeclare` の引数の中、`protected` の要素への修飾つきアクセスの検査。
//!
//! 既存のフロントエンド（Rumoca）・LSP にはつなげない（doc/adr/0039-native-cst-parser.md の付記）。

mod build;
mod lookup;

use rowan::TextRange;
use std::cell::Cell;
use std::collections::HashMap;

pub use build::Library;

pub type ClassId = usize;

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub parent: Option<ClassId>,
    pub file: u32,
    pub name_range: TextRange,
    pub encapsulated: bool,
    pub replaceable: bool,
    /// 入れ子のクラス（直接の子）
    pub classes: HashMap<String, ClassId>,
    /// 部品と、列挙の要素
    pub components: Vec<Component>,
    pub(crate) comp_index: HashMap<String, usize>,
    pub(crate) bases: Vec<PathRef>,
    pub(crate) imports: Vec<Import>,
}

#[derive(Debug)]
pub struct Component {
    pub name: String,
    pub range: TextRange,
    pub public: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum St {
    Todo,
    Busy,
    Done(ClassId),
    Fail(Reason),
}

/// 継承元（`extends`・short class の基底）と、`import A.B.*` の `A.B`。クラスへの解決を覚えておく
#[derive(Debug)]
pub(crate) struct PathRef {
    pub path: Vec<String>,
    pub global: bool,
    pub redeclare: bool,
    /// `class extends X`（`path` は `[X]`）: 受け継いだ `X`（外側のクラスの継承元にある同名のクラス）を継ぐ
    pub class_ext: bool,
    pub state: Cell<St>,
}

#[derive(Debug)]
pub(crate) enum Import {
    Alias(String, Vec<String>),
    Qualified(Vec<String>),
    Unqualified(PathRef),
    Multi(PathRef, Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    /// 型名（部品・short class・`constrainedby`）
    Type,
    /// `extends` と short class の基底。自分の受け継ぎを見ずに外へ探す
    Extends,
    /// `import` のパス（最上位から）
    Import,
    /// 式の成分参照（`a.b.c`）
    Expr,
}

/// 名前の使われ方 1 件
#[derive(Debug)]
pub struct Ref {
    pub file: u32,
    pub range: TextRange,
    pub scope: ClassId,
    pub kind: RefKind,
    pub parts: Vec<String>,
    pub global: bool,
    /// `for` の添字（解決済み）
    pub local: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    NotFound,
    /// 継承の輪（`A extends B`・`B extends A`）
    Cycle,
    /// `redeclare` つきの `extends`・`replaceable` なクラスが関わった失敗（型が差し替わりうるので未対応）
    Redeclare,
    /// 継承元が解決できなかったため
    Inherited,
    /// 型名の途中か最後が部品だった
    NotAClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Class(ClassId),
    /// クラス（のインデックス）の部品。式の成分参照の先頭がこれなら、続きは未解決のまま
    Component(ClassId, usize),
    /// `for` の添字
    Local,
    /// 組み込み（`Real`・`time`・`sin`・`der`…）
    Builtin,
    Unresolved(Reason),
}

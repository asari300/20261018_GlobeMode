//! Track B の B1（S268）: Modelica 3.6 の字句解析と、rowan の CST を作る寛容なパーサ。
//!
//! - `lex`: 全バイトをトークンに分ける（空白・コメント・誤りも）。
//! - `parse`: 誤りがあっても木を作る。`Parse::syntax().to_string()` は元の文字列と一致する。
//!
//! 既存のフロントエンド（Rumoca）にはつながない（doc/adr/0039-native-cst-parser.md）。

mod kind;
mod lexer;
mod parser;

pub use kind::{Modelica, SyntaxKind, SyntaxNode, SyntaxToken};
pub use lexer::{Token, lex};
pub use parser::{Parse, ParseError, parse};

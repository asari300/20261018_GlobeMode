//! S276: 構文に誤りがあって Rumoca のアウトラインが引けないとき、`globe-mode-syntax`（寛容な CST）から
//! アウトラインを作る（Track B を初めて製品につないだ所）。`documentSymbol` と `foldingRange` が使う。
//!
//! 作るのはクラス（名前・種類・`partial`・範囲）と部品（名前・型名・範囲）だけ。`extends`・`import`・connect などは空
//! （表示に要らない）。範囲は最初の字句から最後の字句まで（末尾の空白・コメントは含めない）。
//! shortcut: `operator` だけのクラスは `class` として出す。正常なファイルでは使わない（Rumoca のアウトラインが先）。

use globe_mode_core::TextRange;
use globe_mode_ir::{Causality, ClassKind, ClassOutline, ComponentOutline, ConnectPrefix};
use globe_mode_syntax::ast::{ClassDef, Element, StoredDefinition};
use globe_mode_syntax::{SyntaxKind, SyntaxNode, parse};

pub fn outline(text: &str) -> Vec<ClassOutline> {
    let p = parse(text);
    StoredDefinition::cast(p.syntax())
        .map(|s| s.classes().iter().filter_map(class).collect())
        .unwrap_or_default()
}

/// 空白・コメントを除いた範囲
pub(crate) fn span(n: &SyntaxNode) -> Option<TextRange> {
    let mut t = n
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia());
    let first = t.next()?;
    let last = t.last().unwrap_or_else(|| first.clone());
    Some(TextRange {
        start: first.text_range().start().into(),
        end: last.text_range().end().into(),
    })
}

fn class(c: &ClassDef) -> Option<ClassOutline> {
    use SyntaxKind::*;
    let name = c.name()?;
    let kind = match c.kind_token().map(|t| t.kind()) {
        Some(KwModel) => ClassKind::Model,
        Some(KwBlock) => ClassKind::Block,
        Some(KwConnector) => ClassKind::Connector,
        Some(KwRecord) => ClassKind::Record,
        Some(KwType) => ClassKind::Type,
        Some(KwPackage) => ClassKind::Package,
        Some(KwFunction) => ClassKind::Function,
        _ => ClassKind::Class,
    };
    let partial = c
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .take_while(|t| t.kind() != Ident)
        .any(|t| t.kind() == KwPartial);
    let (mut classes, mut components) = (Vec::new(), Vec::new());
    for e in c.elements() {
        match e {
            Element::Class(k) => classes.extend(class(&k)),
            Element::Component(cc) => {
                let type_name = cc
                    .type_name()
                    .map(|n| n.parts().join("."))
                    .unwrap_or_default();
                for d in cc.declarations() {
                    if let (Some((name, _)), Some(range)) = (d.name(), span(d.syntax())) {
                        components.push(ComponentOutline {
                            name,
                            type_name: type_name.clone(),
                            prefix: ConnectPrefix::default(),
                            causality: Causality::default(),
                            range,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Some(ClassOutline {
        name,
        kind,
        partial,
        causality: Causality::default(),
        range: span(c.syntax())?,
        extends: Vec::new(),
        imports: Vec::new(),
        components,
        connects: Vec::new(),
        classes,
    })
}

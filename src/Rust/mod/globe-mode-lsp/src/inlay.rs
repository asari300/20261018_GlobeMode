//! `textDocument/inlayHint`（S352）。関数の呼び出しの位置引数の前に、引数の名前を出す（`f(x, y)` → `f(a: x, b: y)`）。
//! 自前の構文の木（`globe-mode-syntax`）と名前解決（`globe-mode-resolve`）の道なので、書きかけの文書でも出る（S279 と同じ）。
//! 名前は呼び先の関数の `input` の並び（`extends` の基底が先）から取る。呼び先が引けない（組み込み・部品・解決できない名前・
//! 基底が引けない関数）、引数が `input` より多いときは出さない（推測しない）。
//! shortcut: 呼び先はライブラリ（`-L`）と同じ文書のものだけ（別の開いた文書の関数は引けない）。レコードの構築子・名前つき引数・内包の引数は対象外。

use crate::definition::Resolver;
use globe_mode_resolve::{Ref, Resolution};
use globe_mode_syntax::ast::{CallExpr, Expr};
use globe_mode_syntax::{SyntaxKind, parse};
use serde_json::{Value, json};
use std::collections::HashMap;

/// 引数の頭が `from..=to`（バイト）にある位置引数のヒント（LSP の `InlayHint` の JSON。kind 2 = Parameter）。
pub fn inlay_hints(r: &mut Resolver, path: &str, text: &str, from: usize, to: usize) -> Vec<Value> {
    let Some(lib) = crate::completion::tolerant_lib(r) else {
        return vec![];
    };
    let root = parse(text).syntax();
    lib.with_source(path, text, |l, file| {
        // 呼び先の名前の範囲 → 名前の使われ方（式の成分参照。`der` など予約語の呼び先は ComponentRef でないので無い）
        let refs: HashMap<(u32, u32), &Ref> = l
            .refs
            .iter()
            .filter(|x| x.file == file)
            .map(|x| ((x.range.start().into(), x.range.end().into()), x))
            .collect();
        let mut out = Vec::new();
        for call in root.descendants().filter_map(CallExpr::cast) {
            let Some(callee) = call
                .syntax()
                .children()
                .find(|c| c.kind() == SyntaxKind::ComponentRef)
            else {
                continue;
            };
            let key = (
                callee.text_range().start().into(),
                callee.text_range().end().into(),
            );
            let names = match refs.get(&key).map(|r| l.resolve(r)) {
                Some(Resolution::Class(c)) => l.function_inputs(c),
                _ => None,
            };
            for (arg, name) in call.args().iter().zip(names.iter().flatten()) {
                let start = crate::tolerant::span(arg.syntax())
                    .filter(|_| !matches!(arg, Expr::Comprehension(_)))
                    .map(|s| s.start as usize);
                if let Some(at) = start.filter(|a| (from..=to).contains(a)) {
                    out.push(json!({"position": crate::position(text, at as u32),
                        "label": format!("{name}:"), "kind": 2, "paddingRight": true}));
                }
            }
        }
        out
    })
}

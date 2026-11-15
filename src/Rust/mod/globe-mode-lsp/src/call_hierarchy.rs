//! `textDocument/prepareCallHierarchy`・`callHierarchy/incomingCalls`・`callHierarchy/outgoingCalls`（S354）。
//! 対象は Modelica の `function` だけ（モデル・部品の上では空）。新しい規則は作らず、既にある道をつなぐ。
//! - 関数の特定と呼び元の候補: `textDocument/references`（S166・S230）と同じ `class_name_at`・`class_hits`・`disk_class_hits`
//!   （開いた文書・根の下の開いていない `.mo`・ライブラリ）。候補のうち、自前の構文（`globe-mode-syntax`）の `f(…)` の呼び出しの名前の
//!   終わりに当たるものだけが呼び出し（宣言・`end f`・`extends f(…)` は除く）。呼び元は、呼び出しを囲む一番内側のクラス（関数とは限らない）。
//! - 呼び先: inlay hint（S352）と同じ、関数の本体の呼び出しを自前の名前解決（`globe-mode-resolve`）で引き、関数のクラスだけを出す。引けない名前・組み込み・レコードの構築子は出さない。
//!
//! shortcut: 呼び元の候補はアウトラインの道で引くので、構文の誤りで読めないファイルの関数は対象にできない（呼び先は書きかけの文書でも出る）。
//! 呼ばれる関数の名前（字面）が部分文字列で入るファイルを全部字句に分ける（references と同じ費用）。要るなら索引を持つ。

use crate::definition::{Resolver, Target, class_hits, class_name_at, enclosing};
use globe_mode_core::TextRange;
use globe_mode_fmt::{Kind, lex};
use globe_mode_ir::ClassKind;
use globe_mode_resolve::{Ref, Resolution};
use globe_mode_syntax::ast::{CallExpr, ClassDef};
use globe_mode_syntax::{SyntaxKind, SyntaxNode, parse};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// LSP の `SymbolKind`
const FUNCTION: u32 = 12;
const CLASS: u32 = 5;

fn item(uri: &str, body: &str, name: &str, kind: u32, range: TextRange, sel: TextRange) -> Value {
    json!({"name": name, "kind": kind, "uri": uri,
        "range": crate::range(body, range), "selectionRange": crate::range(body, sel)})
}

/// 関数の定義のファイル `target` の URI（開いている文書 `path` と同じなら `uri` のまま）
fn item_uri(uri: &str, path: &str, target: &str) -> String {
    if target == path {
        uri.to_string()
    } else {
        crate::file_uri(target)
    }
}

/// 呼び出しの名前（`f`・`Lib.f`）の節
fn callee(call: &CallExpr) -> Option<SyntaxNode> {
    call.syntax()
        .children()
        .find(|c| c.kind() == SyntaxKind::ComponentRef)
}

/// カーソル `at` の名前が指す関数（字面と定義の場所）。関数でなければ None。
fn function_at(r: &mut Resolver, path: &str, text: &str, at: usize) -> Option<(String, Target)> {
    let (name, t) = class_name_at(r, path, text, at)?;
    let o = r.outline(&t.path)?;
    let c = enclosing(&o.classes, t.range.start).last().copied()?;
    (c.name == name && c.kind == ClassKind::Function).then_some((name, t))
}

/// `textDocument/prepareCallHierarchy`: カーソルの関数（宣言の名前でも呼び出しでも）の `CallHierarchyItem`。
pub fn prepare(r: &mut Resolver, uri: &str, path: &str, text: &str, at: usize) -> Option<Value> {
    let (name, t) = function_at(r, path, text, at)?;
    let body = if t.path == path {
        text.to_string()
    } else {
        (r.read)(&t.path)?
    };
    // 宣言の名前: クラスの範囲の中の最初の同じ字面（見出しの語は名前と同じ字面にならない）
    let k = lex(&body)
        .ok()?
        .into_iter()
        .find(|k| k.kind == Kind::Ident && k.text == name && k.start >= t.range.start as usize)?;
    let sel = TextRange {
        start: k.start as u32,
        end: (k.start + k.text.len()) as u32,
    };
    Some(item(
        &item_uri(uri, path, &t.path),
        &body,
        &name,
        FUNCTION,
        t.range,
        sel,
    ))
}

/// `callHierarchy/incomingCalls`: `at`（関数の名前）を呼ぶクラスと、その中の呼び出しの範囲。`docs` は開いている文書（uri と本文）。
pub fn incoming(
    r: &mut Resolver,
    root: Option<&Path>,
    libs: &[PathBuf],
    docs: &[(String, String)],
    path: &str,
    text: &str,
    at: usize,
) -> Option<Value> {
    let (name, t) = function_at(r, path, text, at)?;
    let mut files: Vec<(String, String, Vec<TextRange>)> = docs
        .iter()
        .map(|(u, b)| {
            let hits = class_hits(r, &crate::uri_path(u), b, &name, &t);
            (u.clone(), b.clone(), hits)
        })
        .collect();
    let open: Vec<String> = docs
        .iter()
        .map(|(u, _)| crate::normalize_path(&crate::uri_path(u)))
        .collect();
    for h in crate::disk_class_hits(r, root, libs, &open, &name, &t).0 {
        files.push((crate::file_uri(&h.path), h.text, h.ranges));
    }
    // 呼び元（ファイルとクラスの頭）ごとの (item, 呼び出しの範囲)
    let mut out: Vec<(String, u32, Value, Vec<Value>)> = Vec::new();
    for (uri, body, hits) in &files {
        for call in parse(body)
            .syntax()
            .descendants()
            .filter_map(CallExpr::cast)
        {
            let Some(span) = callee(&call).and_then(|c| crate::tolerant::span(&c)) else {
                continue;
            };
            if !hits.iter().any(|h| h.end == span.end) {
                continue;
            }
            let Some(caller) = call.syntax().ancestors().find_map(ClassDef::cast) else {
                continue;
            };
            let (Some(range), Some(nr)) =
                (crate::tolerant::span(caller.syntax()), caller.name_range())
            else {
                continue;
            };
            let from = json!(crate::range(body, span));
            match out.iter_mut().find(|o| o.0 == *uri && o.1 == range.start) {
                Some(o) => o.3.push(from),
                None => {
                    let sel = TextRange {
                        start: nr.start().into(),
                        end: nr.end().into(),
                    };
                    let func = caller
                        .kind_token()
                        .is_some_and(|k| k.kind() == SyntaxKind::KwFunction);
                    let name = caller.name().unwrap_or_default();
                    let it = item(
                        uri,
                        body,
                        &name,
                        if func { FUNCTION } else { CLASS },
                        range,
                        sel,
                    );
                    out.push((uri.clone(), range.start, it, vec![from]));
                }
            }
        }
    }
    Some(json!(
        out.into_iter()
            .map(|(_, _, from, ranges)| json!({"from": from, "fromRanges": ranges}))
            .collect::<Vec<_>>()
    ))
}

/// `callHierarchy/outgoingCalls`: `at`（関数の名前）の関数の本体が呼ぶ関数と、呼び出しの範囲（`text` の中）。
pub fn outgoing(r: &mut Resolver, uri: &str, path: &str, text: &str, at: usize) -> Option<Value> {
    let read = r.read;
    let lib = crate::completion::tolerant_lib(r)?;
    let root = parse(text).syntax();
    // 呼び先ごとの (ファイル, 名前, 範囲, 名前の範囲, 呼び出しの範囲)
    type Callee = (String, String, TextRange, TextRange, Vec<TextRange>);
    let found: Vec<Callee> = lib.with_source(path, text, |l, file| {
        let f = l
            .class_at(file, at as u32)
            .filter(|&c| l.classes[c].is_function)?;
        let body = l.classes[f].range;
        let refs: HashMap<(u32, u32), &Ref> = l
            .refs
            .iter()
            .filter(|x| x.file == file)
            .map(|x| ((x.range.start().into(), x.range.end().into()), x))
            .collect();
        let mut out: Vec<(usize, Callee)> = Vec::new();
        for call in root.descendants().filter_map(CallExpr::cast) {
            let Some(node) = callee(&call).filter(|n| body.contains_range(n.text_range())) else {
                continue;
            };
            let key = (
                node.text_range().start().into(),
                node.text_range().end().into(),
            );
            let Some(Resolution::Class(c)) = refs.get(&key).map(|x| l.resolve(x)) else {
                continue;
            };
            let (Some(span), true) = (crate::tolerant::span(&node), l.classes[c].is_function)
            else {
                continue;
            };
            if let Some(o) = out.iter_mut().find(|o| o.0 == c) {
                o.1.4.push(span);
                continue;
            }
            let cl = &l.classes[c];
            out.push((
                c,
                (
                    l.files[cl.file as usize].clone(),
                    cl.name.clone(),
                    TextRange {
                        start: cl.range.start().into(),
                        end: cl.range.end().into(),
                    },
                    TextRange {
                        start: cl.name_range.start().into(),
                        end: cl.name_range.end().into(),
                    },
                    vec![span],
                ),
            ));
        }
        Some(out.into_iter().map(|o| o.1).collect())
    })?;
    Some(json!(
        found
            .into_iter()
            .filter_map(|(p, name, range, sel, from)| {
                let body = if p == path { text.to_string() } else { read(&p)? };
                Some(json!({"to": item(&item_uri(uri, path, &p), &body, &name, FUNCTION, range, sel),
                    "fromRanges": from.into_iter().map(|x| crate::range(text, x)).collect::<Vec<_>>()}))
            })
            .collect::<Vec<_>>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode::Workspace;
    use globe_mode_fmt::LibraryTree;

    /// S360: Windows の CI で `incoming` が空だった。ライブラリの木のパス（`C:\lib\Fns.mo`、`display()` のまま）と、
    /// 項目の URI から戻したパス（`C:/lib/Fns.mo`）は別の綴りでも同じファイル。宣言の名前は後者、`Fns.f` の解決は前者で出る。
    #[test]
    fn incoming_finds_callers_when_tree_path_spells_backslashes() {
        let fns = "package Fns\n  function f\n    input Real a;\n    output Real y = a;\n  end f;\nend Fns;\n";
        let m = "model M\n  Real x = Fns.f(1) + Fns.f(2);\nend M;\n";
        let tree = LibraryTree::new([("C:\\lib\\Fns.mo", fns)]);
        let (mut ws, mut scratch) = (Workspace::new(), Workspace::new());
        ws.set_file("/tmp/M.mo", m.to_string());
        let read = |p: &str| (p.replace('\\', "/") == "C:/lib/Fns.mo").then(|| fns.to_string());
        let mut r = Resolver {
            ws: &ws,
            tree: &tree,
            scratch: &mut scratch,
            overlay: None,
            lib: None,
            read: &read,
        };
        let docs = [("file:///tmp/M.mo".to_string(), m.to_string())];
        let at = fns.find("function f").unwrap() + "function ".len();
        let inc = incoming(&mut r, None, &[], &docs, "C:/lib/Fns.mo", fns, at).unwrap();
        let from: Vec<(&str, usize)> = inc
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["from"]["name"].as_str().unwrap(),
                    c["fromRanges"].as_array().unwrap().len(),
                )
            })
            .collect();
        assert_eq!(from, [("M", 2)], "{inc}");
    }
}

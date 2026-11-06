//! 補完（S053、S171）。予約語、同じ文書の `Outline` のクラス名・部品名（継承した部品も）、ライブラリの最上位クラスを返す。
//! 点つきの名前の途中（`a.b.`）は、部品なら型の部品名（継承した分も）、クラスならライブラリの木とその文書の入れ子のクラスの直下の子。
//! 文字列・コメントの中は出さない。最大 200 件で、打ちかけの語に字が順に当たるものだけを返す。新しい解析は入れない。

use crate::definition::{Resolver, scopes_at};
use globe_mode::{Outline, Workspace};
use globe_mode_ir::{ClassKind, ClassOutline, ComponentOutline};
use lsp_types::{CompletionItem, CompletionItemKind, Documentation};
use serde_json::{Value, json};

/// 返す件数の上限
const LIMIT: usize = 200;

/// `text[..at]` の末尾の単語（`a.b.c` なら `a.b.` と `c`）を返す。
fn word_before(text: &str, at: usize) -> (&str, &str) {
    let head = &text[..at.min(text.len())];
    let start = head
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .map_or(0, |i| i + 1);
    let word = &head[start..];
    match word.rfind('.') {
        Some(d) => (&word[..=d], &word[d + 1..]),
        None => ("", word),
    }
}

/// `at` が文字列・コメントの中か。shortcut: 引用符つきの識別子（`'a"b'`）は見ない
fn inert(text: &str, at: usize) -> bool {
    let b = text.as_bytes();
    let (mut i, mut state) = (0, 0u8); // 0 通常、1 文字列、2 行コメント、3 ブロックコメント
    while i < at.min(b.len()) {
        match state {
            0 => match (b[i], b.get(i + 1)) {
                (b'"', _) => state = 1,
                (b'/', Some(b'/')) => (state, i) = (2, i + 1),
                (b'/', Some(b'*')) => (state, i) = (3, i + 1),
                _ => {}
            },
            1 => match b[i] {
                b'\\' => i += 1,
                b'"' => state = 0,
                _ => {}
            },
            2 => {
                if b[i] == b'\n' {
                    state = 0;
                }
            }
            _ => {
                if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    (state, i) = (0, i + 1);
                }
            }
        }
        i += 1;
    }
    state != 0
}

/// 入力の途中で文書が壊れているとき、カーソルの行を空白にした本文のアウトライン（バイトの位置は変わらない）。
/// shortcut: 行をまたぐ文の途中や、カーソルの行以外にも誤りがあるときは読めない（補完は候補なしになる）
fn repair(path: &str, text: &str, at: usize) -> Option<Outline> {
    let from = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let to = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let blank: String = text[from..to]
        .chars()
        .map(|c| " ".repeat(c.len_utf8()))
        .collect();
    let mut ws = Workspace::new();
    ws.set_file(path, format!("{}{blank}{}", &text[..from], &text[to..]));
    ws.outline(path)?.as_ref().ok().cloned()
}

fn item(label: &str, kind: CompletionItemKind, detail: Option<String>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        ..Default::default()
    }
}

/// ライブラリの木の項目（種類と説明つき）
fn tree_item(e: &globe_mode_fmt::TreeEntry) -> CompletionItem {
    let mut i = item(
        e.name,
        match e.kind {
            "package" => CompletionItemKind::MODULE,
            "function" => CompletionItemKind::FUNCTION,
            _ => CompletionItemKind::CLASS,
        },
        Some(e.kind.to_string()),
    );
    if !e.description.is_empty() {
        i.documentation = Some(Documentation::String(e.description.to_string()));
    }
    i
}

fn class_items(c: &ClassOutline, out: &mut Vec<CompletionItem>) {
    out.push(item(
        &c.name,
        CompletionItemKind::CLASS,
        Some(format!("{:?}", c.kind).to_lowercase()),
    ));
    for comp in &c.components {
        out.push(item(
            &comp.name,
            CompletionItemKind::VARIABLE,
            Some(comp.type_name.clone()),
        ));
    }
    for n in &c.classes {
        class_items(n, out);
    }
}

/// クラス `q` の部品。継承した分も（深さ 8 まで。同名は先のもの）。
fn all_components(
    r: &mut Resolver,
    q: &str,
    class: &ClassOutline,
    depth: u32,
) -> Vec<ComponentOutline> {
    let mut v = class.components.clone();
    if depth <= 8 {
        for e in &class.extends {
            if let Some((eq, _, ec)) = r.resolve(q, e) {
                for c in all_components(r, &eq, &ec, depth + 1) {
                    if !v.iter().any(|x| x.name == c.name) {
                        v.push(c);
                    }
                }
            }
        }
    }
    v
}

/// `a.b` が部品の名前なら、その型の部品（継承した分も）。部品でなければ None。
fn members(
    r: &mut Resolver,
    o: &Outline,
    path: &str,
    at: usize,
    name: &str,
) -> Option<Vec<ComponentOutline>> {
    let (chain, scopes, _) = scopes_at(o, at);
    let mut segs = name.split('.');
    let head = segs.next()?;
    for (c, q) in chain.iter().zip(&scopes).rev() {
        let Some((mut dq, _, mut comp)) = r.component(q, path, c, head, 0) else {
            continue;
        };
        for s in segs {
            let (tq, tf, tc) = r.resolve(&dq, &comp.type_name)?;
            (dq, _, comp) = r.component(&tq, &tf, &tc, s, 0)?;
        }
        let (tq, _, tc) = r.resolve(&dq, &comp.type_name)?;
        return Some(all_components(r, &tq, &tc, 0));
    }
    None
}

/// クラス `name`（`Modelica.Blocks`、または `scope` から見た入れ子・import の別名）の直下の子のクラス。
fn class_children(r: &mut Resolver, scope: &str, name: &str, out: &mut Vec<CompletionItem>) {
    let mut q = name.to_string();
    if r.tree.children(name).is_empty() {
        let Some((rq, _, c)) = r.resolve(scope, name) else {
            return;
        };
        for n in &c.classes {
            out.push(item(
                &n.name,
                if n.kind == ClassKind::Package {
                    CompletionItemKind::MODULE
                } else {
                    CompletionItemKind::CLASS
                },
                Some(format!("{:?}", n.kind).to_lowercase()),
            ));
        }
        q = rq;
    }
    out.extend(r.tree.children(&q).iter().map(tree_item));
}

/// `word` の字が `label` に順に（大文字小文字を区別せず）当たるか。CodeMirror の絞り込みと同じ向き。
fn matches(label: &str, word: &str) -> bool {
    let mut l = label.chars().flat_map(char::to_lowercase);
    word.chars()
        .flat_map(char::to_lowercase)
        .all(|w| l.any(|c| c == w))
}

pub fn complete(r: &mut Resolver, path: &str, text: &str, at: usize) -> Vec<CompletionItem> {
    let at = at.min(text.len());
    if inert(text, at) {
        return vec![];
    }
    if let Some(Ok(o)) = r.ws.outline(path) {
        return candidates(r, path, text, at, Some(o));
    }
    // 打ちかけで文書が壊れている間は、カーソルの行を除いて読んだアウトラインで引く
    let fixed = repair(path, text, at);
    let mut r = Resolver {
        ws: r.ws,
        tree: r.tree,
        scratch: &mut *r.scratch,
        read: r.read,
        overlay: fixed.as_ref().map(|o| (path, o)),
    };
    candidates(&mut r, path, text, at, fixed.as_ref())
}

fn candidates(
    r: &mut Resolver,
    path: &str,
    text: &str,
    at: usize,
    outline: Option<&Outline>,
) -> Vec<CompletionItem> {
    let (prefix, word) = word_before(text, at);
    let ws = r.ws;
    let mut out = Vec::new();
    if !prefix.is_empty() {
        let name = prefix.trim_end_matches('.');
        let found = outline.and_then(|o| members(r, o, path, at, name));
        if let Some(ms) = found {
            for comp in ms {
                out.push(item(
                    &comp.name,
                    CompletionItemKind::FIELD,
                    Some(comp.type_name),
                ));
            }
        } else {
            let scope = outline
                .map(|o| scopes_at(o, at))
                .map_or_else(String::new, |(_, s, w)| s.last().cloned().unwrap_or(w));
            class_children(r, &scope, name, &mut out);
        }
    } else {
        for k in globe_mode_fmt::KEYWORDS {
            out.push(item(k, CompletionItemKind::KEYWORD, None));
        }
        if let Some(o) = outline {
            for c in &o.classes {
                class_items(c, &mut out);
            }
            let (chain, scopes, _) = scopes_at(o, at);
            for (c, q) in chain.iter().zip(&scopes) {
                for comp in all_components(r, q, c, 0) {
                    out.push(item(
                        &comp.name,
                        CompletionItemKind::VARIABLE,
                        Some(comp.type_name),
                    ));
                }
            }
        }
        out.extend(r.tree.children("").iter().map(tree_item));
        for f in ws.files().collect::<Vec<_>>() {
            if f == path {
                continue;
            }
            if let Some(Ok(o)) = ws.outline(f) {
                for c in &o.classes {
                    let label = match &o.within {
                        Some(w) => format!("{w}.{}", c.name),
                        None => c.name.clone(),
                    };
                    // ライブラリのファイルは木から出す
                    if r.tree.file(&label) == Some(f) {
                        continue;
                    }
                    out.push(item(
                        &label,
                        CompletionItemKind::MODULE,
                        Some(f.to_string()),
                    ));
                }
            }
        }
    }
    out.retain(|i| matches(&i.label, word));
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out.dedup_by(|a, b| a.label == b.label);
    out.truncate(LIMIT);
    out
}

/// LSP の `CompletionList`（S198）: 上限 `LIMIT` に達したときは `isIncomplete`（打ち足すと別の候補が出うるので、クライアントは打つたびに問い直す）。
pub fn complete_list(r: &mut Resolver, path: &str, text: &str, at: usize) -> Value {
    let items = complete(r, path, text, at);
    json!({"isIncomplete": items.len() >= LIMIT, "items": items})
}

/// UI 向け（S171。wasm が使う）。行・列は LSP と同じ UTF-16。LSP の `CompletionItem` の**配列**（`isIncomplete` は付けない。LSP の `globe/completion` は `complete_list`）。
pub fn complete_at(r: &mut Resolver, path: &str, text: &str, line: u32, character: u32) -> Value {
    let at = crate::offset(text, lsp_types::Position::new(line, character));
    json!(complete(r, path, text, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_fmt::LibraryTree;

    /// `docs` を足したワークスペースと、`lib`（ファイル名と本文）から作った木で `at` の補完のラベルを返す。
    fn labels(docs: &[(&str, &str)], lib: &[(&str, &str)], path: &str, at: usize) -> Vec<String> {
        let tree = LibraryTree::new(lib.iter().copied());
        let (mut ws, mut scratch) = (Workspace::new(), Workspace::new());
        for (n, t) in docs {
            ws.set_file(n, t.to_string());
        }
        let text = docs.iter().find(|(n, _)| *n == path).unwrap().1;
        complete(
            &mut Resolver {
                ws: &ws,
                tree: &tree,
                scratch: &mut scratch,
                overlay: None,
                read: &|f| {
                    lib.iter()
                        .find(|(n, _)| *n == f)
                        .map(|(_, t)| t.to_string())
                },
            },
            path,
            text,
            at,
        )
        .into_iter()
        .map(|i| i.label)
        .collect()
    }

    #[test]
    fn keywords_names_and_dot_members() {
        let pin = (
            "Pin.mo",
            "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n",
        );
        let text = "model M\n  Pin p;\n  Real x;\nequation\n  p.v = x;\nend M;\n";
        let docs = [pin, ("M.mo", text)];
        let all = labels(&docs, &[], "M.mo", 0);
        for want in ["model", "M", "x", "Pin"] {
            assert!(all.iter().any(|l| l == want), "{want} in {all:?}");
        }
        let at = text.find("p.").unwrap() + 2;
        assert_eq!(labels(&docs, &[], "M.mo", at), ["i", "v"]);
        assert_eq!(word_before("a.b.c", 5), ("a.b.", "c"));
    }

    #[test]
    fn library_children_after_a_dotted_name_in_a_type_position() {
        let lib = [
            ("Lib/package.mo", "package Lib \"the lib\"\nend Lib;\n"),
            ("Lib/Sub/package.mo", "within Lib;\npackage Sub\nend Sub;\n"),
            (
                "Lib/Sub/Sine.mo",
                "within Lib.Sub;\nblock Sine \"a sine\"\nend Sine;\n",
            ),
            (
                "Lib/Sub/Step.mo",
                "within Lib.Sub;\nblock Step\nend Step;\n",
            ),
        ];
        let text = "model M\n  Lib.Sub.\n  Real x;\nend M;\n"; // 打ちかけで文書は壊れている
        let docs = [("M.mo", text)];
        let at = text.find("Sub.").unwrap() + 4;
        assert_eq!(labels(&docs, &lib, "M.mo", at), ["Sine", "Step"]);
        let at = text.find("Lib.").unwrap() + 4;
        assert_eq!(labels(&docs, &lib, "M.mo", at), ["Sub"]);
        // 打ちかけの語で絞る。何も打っていない位置には最上位のクラスも出る
        let typed = "model M\n  Lib.Sub.Si\nend M;\n";
        let at = typed.find("Si").unwrap() + 2;
        assert_eq!(labels(&[("M.mo", typed)], &lib, "M.mo", at), ["Sine"]);
        assert!(labels(&docs, &lib, "M.mo", 0).contains(&"Lib".to_string()));
        // import の別名の後ろも、定義ジャンプと同じ解決で引ける
        let aliased = "model M\n  import S = Lib.Sub;\n  S.\nend M;\n";
        let at = aliased.find("S.\n").unwrap() + 2;
        assert_eq!(
            labels(&[("M.mo", aliased)], &lib, "M.mo", at),
            ["Sine", "Step"]
        );
    }

    #[test]
    fn typed_word_narrows_before_the_limit_cuts() {
        // S198: `Lib.SI.` の直下が 300 件あっても、`Lib.SI.Vol` では Voltage が出る（絞ってから 200 件に切る）
        let mut lib = vec![(
            "Lib/package.mo".to_string(),
            "package Lib\nend Lib;\n".to_string(),
        )];
        for n in (0..300)
            .map(|i| format!("T{i:03}"))
            .chain(["Voltage".to_string()])
        {
            lib.push((
                format!("Lib/{n}.mo"),
                format!("within Lib;\ntype {n} = Real;\n"),
            ));
        }
        let lib: Vec<(&str, &str)> = lib.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let at_end = |t: &str| t.find("\nend").unwrap();
        let all = "model M\n  Lib.\nend M;\n";
        assert_eq!(
            labels(&[("M.mo", all)], &lib, "M.mo", at_end(all)).len(),
            LIMIT
        );
        let typed = "model M\n  Lib.Vol\nend M;\n";
        assert_eq!(
            labels(&[("M.mo", typed)], &lib, "M.mo", at_end(typed)),
            ["Voltage"]
        );
    }

    #[test]
    fn inherited_members_and_broken_document() {
        let text = "model A\n  Real a;\nend A;\nmodel B\n  extends A;\n  Real b;\nend B;\nmodel M\n  B y;\nequation\n  y.\nend M;\n";
        let at = text.find("  y.\n").unwrap() + 4; // 壊れた文書でも、カーソルの行を除いて読む
        assert_eq!(labels(&[("M.mo", text)], &[], "M.mo", at), ["a", "b"]);
    }

    #[test]
    fn nothing_in_strings_and_comments() {
        let text = "model M // Re\n  Real x \"Re\"; /* Re */\nend M;\n";
        for p in ["Re\n", "Re\"", "Re *"] {
            let at = text.find(p).unwrap() + 2;
            assert!(labels(&[("M.mo", text)], &[], "M.mo", at).is_empty(), "{p}");
        }
        let at = text.find("end").unwrap() + 1;
        assert!(!labels(&[("M.mo", text)], &[], "M.mo", at).is_empty());
    }
}

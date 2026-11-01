//! 補完（S053）。予約語と、同じファイルの `Outline` のクラス名・部品名、ライブラリの最上位クラス名を返す。
//! `.` の後ろは、手前の部品の型のアウトラインがあればその部品名だけ。新しい解析は入れない。

use globe_mode::{Outline, Workspace};
use globe_mode_ir::ClassOutline;
use lsp_types::{CompletionItem, CompletionItemKind};

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

fn item(label: &str, kind: CompletionItemKind, detail: Option<String>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        ..Default::default()
    }
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

/// 名前（`A.B` のように `within` 付きも可）のクラスを、開いているファイルとライブラリから探す。
fn find_class<'w>(ws: &'w Workspace, name: &str) -> Option<&'w ClassOutline> {
    let find_in = |o: &'w Outline| -> Option<&'w ClassOutline> {
        let rel = match &o.within {
            Some(w) => name
                .strip_prefix(w.as_str())
                .and_then(|r| r.strip_prefix('.'))
                .unwrap_or(name),
            None => name,
        };
        let mut parts = rel.split('.');
        let mut cur = o
            .classes
            .iter()
            .find(|c| c.name == parts.next().unwrap_or(""))?;
        for p in parts {
            cur = cur.classes.iter().find(|c| c.name == p)?;
        }
        Some(cur)
    };
    ws.files()
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|f| ws.outline(f))
        .filter_map(|r| r.as_ref().ok())
        .find_map(find_in)
}

pub fn complete(ws: &Workspace, path: &str, text: &str, at: usize) -> Vec<CompletionItem> {
    let (prefix, _) = word_before(text, at);
    let outline = ws.outline(path).and_then(|r| r.as_ref().ok());
    let mut out = Vec::new();
    if !prefix.is_empty() {
        // `comp.` → comp の型の部品名。見つからなければ空
        let comp_name = prefix.trim_end_matches('.');
        let type_name =
            outline.and_then(|o| o.classes.iter().find_map(|c| component_type(c, comp_name)));
        if let Some(class) = type_name.and_then(|t| find_class(ws, &t)) {
            for comp in &class.components {
                out.push(item(
                    &comp.name,
                    CompletionItemKind::FIELD,
                    Some(comp.type_name.clone()),
                ));
            }
        }
        return out;
    }
    for k in globe_mode_fmt::KEYWORDS {
        out.push(item(k, CompletionItemKind::KEYWORD, None));
    }
    if let Some(o) = outline {
        for c in &o.classes {
            class_items(c, &mut out);
        }
    }
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
                out.push(item(
                    &label,
                    CompletionItemKind::MODULE,
                    Some(f.to_string()),
                ));
            }
        }
    }
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out.dedup_by(|a, b| a.label == b.label);
    out
}

fn component_type(c: &ClassOutline, comp: &str) -> Option<String> {
    c.components
        .iter()
        .find(|x| x.name == comp)
        .map(|x| x.type_name.clone())
        .or_else(|| c.classes.iter().find_map(|n| component_type(n, comp)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_names_and_dot_members() {
        let mut ws = Workspace::new();
        ws.set_file(
            "Pin.mo",
            "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n",
        );
        // shortcut: 文法が壊れた文書は outline が Err で補完が予約語だけになる。入力途中の `p.` を拾うなら前回の outline を保つ
        let text = "model M\n  Pin p;\n  Real x;\nequation\n  p.v = x;\nend M;\n";
        ws.set_file("M.mo", text);
        let all = complete(&ws, "M.mo", text, 0);
        let labels: Vec<_> = all.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"model")
                && labels.contains(&"M")
                && labels.contains(&"x")
                && labels.contains(&"Pin")
        );
        let at = text.find("p.").unwrap() + 2;
        let members: Vec<_> = complete(&ws, "M.mo", text, at)
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert_eq!(members, ["v", "i"]);
        assert_eq!(word_before("a.b.c", 5), ("a.b.", "c"));
    }
}

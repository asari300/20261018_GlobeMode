//! ライブラリの木（S127）。ファイルの見出し（`within` と最初のクラスの種類・名前・説明文字列）だけを読み、
//! 完全修飾名 → (種類, 説明, ファイル) の索引を作る。UI のライブラリツリーが展開のたびに直下の子を引く。
//!
//! 単位はファイル: 1 ファイルに入れ子のクラスを持つパッケージ（`Sources.mo` など）は葉で、開くとそのファイル全体。
//! shortcut: 並びは名前の順（`package.order` は見ない。ブラウザの MSL.json は `.mo` だけを持つ）。順が要るなら order も渡す。

use std::collections::BTreeMap;

/// 木の 1 項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry<'a> {
    /// 最後の名前（`ChuaCircuit`）
    pub name: &'a str,
    /// 完全修飾名（`Modelica.Electrical.Analog.Examples.ChuaCircuit`）
    pub path: &'a str,
    /// `model`・`package`・`operator record` など（`partial`・`encapsulated` は除く）
    pub kind: &'a str,
    pub description: &'a str,
    /// このクラスを定義するファイル
    pub file: &'a str,
    /// 木の下にファイル単位の子があるか
    pub has_children: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryTree {
    classes: BTreeMap<String, (String, String, String)>,
}

impl LibraryTree {
    /// (ファイル名, 本文) の組から作る。見出しの読めないファイルは飛ばす。
    pub fn new<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut classes = BTreeMap::new();
        for (file, text) in files {
            if let Some(h) = header(text) {
                let path = match h.within {
                    Some(w) if !w.is_empty() => format!("{w}.{}", h.name),
                    _ => h.name.to_string(),
                };
                classes.insert(path, (h.kind, h.description.to_string(), file.to_string()));
            }
        }
        LibraryTree { classes }
    }

    /// `prefix`（空ならトップレベル）の直下の子。
    pub fn children(&self, prefix: &str) -> Vec<TreeEntry<'_>> {
        let start = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}.")
        };
        self.under(start.clone())
            .filter(|(k, _)| !k[start.len()..].contains('.'))
            .map(|(k, (kind, description, file))| TreeEntry {
                name: &k[start.len()..],
                path: k,
                kind,
                description,
                file,
                has_children: self.under(format!("{k}.")).next().is_some(),
            })
            .collect()
    }

    /// `path` を定義するファイル。
    pub fn file(&self, path: &str) -> Option<&str> {
        self.classes.get(path).map(|(_, _, f)| f.as_str())
    }

    fn under(&self, start: String) -> impl Iterator<Item = (&String, &(String, String, String))> {
        self.classes
            .range(start.clone()..)
            .take_while(move |(k, _)| k.starts_with(&start))
    }
}

struct Header<'a> {
    within: Option<&'a str>,
    kind: String,
    name: &'a str,
    description: &'a str,
}

enum Tok<'a> {
    Word(&'a str),
    Str(&'a str),
    Punct,
}

/// 空白とコメントを飛ばして次の字句を返す（見出しを読むのに要る分だけ）。
fn next<'a>(s: &'a str, i: &mut usize) -> Option<Tok<'a>> {
    let b = s.as_bytes();
    loop {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if s[*i..].starts_with("//") {
            *i = s[*i..].find('\n').map_or(b.len(), |n| *i + n);
        } else if s[*i..].starts_with("/*") {
            *i = s[*i + 2..].find("*/").map_or(b.len(), |n| *i + n + 4);
        } else {
            break;
        }
    }
    let start = *i;
    let c = *b.get(start)?;
    if c == b'"' {
        *i += 1;
        while *i < b.len() && b[*i] != b'"' {
            *i += if b[*i] == b'\\' { 2 } else { 1 };
        }
        *i = (*i + 1).min(b.len());
        return Some(Tok::Str(&s[start + 1..(*i - 1).max(start + 1)]));
    }
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'.';
    if word(c) {
        while *i < b.len() && word(b[*i]) {
            *i += 1;
        }
        return Some(Tok::Word(&s[start..*i]));
    }
    *i += 1;
    Some(Tok::Punct)
}

const KINDS: &[&str] = &[
    "model",
    "package",
    "block",
    "connector",
    "record",
    "type",
    "function",
    "class",
    "operator",
    "expandable",
    "pure",
    "impure",
];

fn header(src: &str) -> Option<Header<'_>> {
    let mut i = 0;
    let mut within = None;
    let mut t = next(src, &mut i)?;
    if matches!(t, Tok::Word("within")) {
        t = next(src, &mut i)?;
        if let Tok::Word(w) = t {
            within = Some(w);
            t = next(src, &mut i)?;
        }
        if !matches!(t, Tok::Punct) {
            return None;
        }
        t = next(src, &mut i)?;
    }
    let mut kind = Vec::new();
    let name = loop {
        match t {
            Tok::Word("encapsulated" | "partial" | "final") => {}
            Tok::Word(w) if KINDS.contains(&w) => kind.push(w),
            Tok::Word(w) if !kind.is_empty() => break w,
            _ => return None,
        }
        t = next(src, &mut i)?;
    };
    let description = match next(src, &mut i) {
        Some(Tok::Str(d)) => d,
        _ => "",
    };
    Some(Header {
        within,
        kind: kind.join(" "),
        name,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_are_read_from_file_headers() {
        let tree = LibraryTree::new([
            ("M/package.mo", "within;\npackage M \"Lib\"\nend M;\n"),
            (
                "M/A/package.mo",
                "// c\nwithin M;\n/* x */ encapsulated package A \"Sub \\\"q\\\"\"\nend A;\n",
            ),
            ("M/A/B.mo", "within M.A;\npartial model B\nend B;\n"),
            ("M/C.mo", "within M;\noperator record C \"Rec\"\nend C;\n"),
            ("Bad.mo", "not modelica"),
        ]);
        let top = tree.children("");
        assert_eq!(top.len(), 1);
        assert_eq!(
            (
                top[0].name,
                top[0].kind,
                top[0].description,
                top[0].has_children
            ),
            ("M", "package", "Lib", true)
        );
        let m: Vec<_> = tree
            .children("M")
            .iter()
            .map(|e| (e.path, e.kind, e.description, e.has_children))
            .collect();
        assert_eq!(
            m,
            [
                ("M.A", "package", "Sub \\\"q\\\"", true),
                ("M.C", "operator record", "Rec", false)
            ]
        );
        assert_eq!(tree.children("M.A")[0].kind, "model");
        assert_eq!(tree.file("M.A.B"), Some("M/A/B.mo"));
        assert_eq!(tree.file("M.Z"), None);
    }
}

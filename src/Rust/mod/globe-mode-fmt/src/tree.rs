//! ライブラリの木（S127）。ファイルの見出し（`within` と最初のクラスの種類・名前・説明文字列）だけを読み、
//! 完全修飾名 → (種類, 説明, ファイル) の索引を作る。UI のライブラリツリーが展開のたびに直下の子を引く。
//!
//! S167: ファイルの中の入れ子のクラス（`Sources.mo` の `Sine` など）も木の項目にする（字句解析と文の切り出しだけ。
//! Rumoca の構文解析はしない）。入れ子の項目の `file` は、それを定義するファイル（開くとそのファイル全体）。
//! 構文の誤りで文を切り出せないファイルは、見出しの 1 クラスだけ。
//! shortcut: 並びは名前の順（`package.order` は見ない。ブラウザの MSL.json は `.mo` だけを持つ）。順が要るなら order も渡す。

use std::collections::BTreeMap;

use crate::lexer::lex;
use crate::structure::class_defs;

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
    /// 木の下に子（別のファイルのクラス、またはファイルの中の入れ子のクラス）があるか
    pub has_children: bool,
}

/// (種類, 説明, ファイル)
type Class = (String, String, String);

#[derive(Debug, Clone, Default)]
pub struct LibraryTree {
    classes: BTreeMap<String, Class>,
}

impl LibraryTree {
    /// (ファイル名, 本文) の組から作る。見出しの読めないファイルは飛ばす。
    pub fn new<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut classes = BTreeMap::new();
        for (file, text) in files {
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            let h = header(text);
            let within = h.as_ref().and_then(|h| h.within).filter(|w| !w.is_empty());
            let join = |name: &str| within.map_or(name.to_string(), |w| format!("{w}.{name}"));
            if let Some(h) = &h {
                classes.insert(
                    join(h.name),
                    (h.kind.clone(), h.description.to_string(), file.to_string()),
                );
            }
            for (rel, kind, description) in
                lex(text).and_then(|t| class_defs(&t)).unwrap_or_default()
            {
                classes
                    .entry(join(&rel))
                    .or_insert_with(|| (kind, description.to_string(), file.to_string()));
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
            .map(|(k, v)| self.entry(k, v, start.len()))
            .collect()
    }

    /// 名前の部分一致（大文字小文字を区別しない。ASCII だけ）で最大 `limit` 件。`.` を含む検索語は完全修飾名に当てる。
    /// 並びは、名前が完全一致、前方一致、部分一致の順で、同じ中では短い順（浅い順）、同じ長さなら名前の順。
    pub fn search(&self, query: &str, limit: usize) -> Vec<TreeEntry<'_>> {
        let q = query.trim().as_bytes();
        let has = |s: &str| {
            s.len() >= q.len()
                && s.as_bytes()
                    .windows(q.len())
                    .any(|w| w.eq_ignore_ascii_case(q))
        };
        if q.is_empty() {
            return Vec::new();
        }
        let by_path = q.contains(&b'.');
        let mut hit: Vec<(u8, &String, &Class, usize)> = self
            .classes
            .iter()
            .filter_map(|(k, v)| {
                let at = k.rfind('.').map_or(0, |i| i + 1);
                let name = &k[at..];
                if !has(if by_path { k } else { name }) {
                    return None;
                }
                let rank = if name.as_bytes().eq_ignore_ascii_case(q) {
                    0
                } else if name.len() >= q.len()
                    && name.as_bytes()[..q.len()].eq_ignore_ascii_case(q)
                {
                    1
                } else {
                    2
                };
                Some((rank, k, v, at))
            })
            .collect();
        hit.sort_by_key(|(rank, k, ..)| (*rank, k.len(), *k));
        hit.truncate(limit);
        hit.into_iter()
            .map(|(_, k, v, at)| self.entry(k, v, at))
            .collect()
    }

    fn entry<'s>(&'s self, k: &'s String, v: &'s Class, name_at: usize) -> TreeEntry<'s> {
        TreeEntry {
            name: &k[name_at..],
            path: k,
            kind: &v.0,
            description: &v.1,
            file: &v.2,
            has_children: self.under(format!("{k}.")).next().is_some(),
        }
    }

    /// すべてのクラスの (完全修飾名, 種類)（名前の順）。
    pub fn classes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.classes.iter().map(|(k, v)| (k.as_str(), v.0.as_str()))
    }

    /// `path` を定義するファイル。
    pub fn file(&self, path: &str) -> Option<&str> {
        self.classes.get(path).map(|(_, _, f)| f.as_str())
    }

    fn under(&self, start: String) -> impl Iterator<Item = (&String, &Class)> {
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
        // 閉じない文字列が `\` と多バイト文字で終わると、*i - 1 が文字の途中になる
        return Some(Tok::Str(
            s.get(start + 1..(*i - 1).max(start + 1)).unwrap_or(""),
        ));
    }
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'.';
    if word(c) {
        while *i < b.len() && word(b[*i]) {
            *i += 1;
        }
        return Some(Tok::Word(&s[start..*i]));
    }
    *i += s[start..].chars().next().map_or(1, char::len_utf8);
    Some(Tok::Punct)
}

pub(crate) const KINDS: &[&str] = &[
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

    /// #59: BOM 付きのファイルでも `within` を読み、閉じない文字列が `\\` と多バイト文字で終わっても panic しない。
    #[test]
    fn a_bom_and_a_broken_string_do_not_break_the_tree() {
        let tree = LibraryTree::new([("P/M.mo", "\u{feff}within P;\nmodel M\nend M;\n")]);
        assert_eq!(tree.file("P.M"), Some("P/M.mo"));
        assert_eq!(tree.file("M"), None);
        LibraryTree::new([("x.mo", "model M \"abc\\\u{3042}")]);
    }

    const SOURCES: &str = "within M;\npackage Sources \"Src\"\n  \
        block Sine \"Sine wave\"\n    parameter Real f = 1;\n    function g\n    end g;\n  equation\n    if f > 0 then\n    end if;\n  end Sine;\n  \
        type Level = Real;\n  model Step\n  end Step;\nend Sources;\n";

    #[test]
    fn classes_nested_in_a_file_are_children() {
        let tree = LibraryTree::new([
            ("M/Sources.mo", SOURCES),
            ("M/package.mo", "package M\nend M;\n"),
        ]);
        let kids: Vec<_> = tree
            .children("M.Sources")
            .iter()
            .map(|e| (e.name, e.kind, e.description, e.file, e.has_children))
            .collect();
        // 短いクラス `type Level = Real;` も出す。`Sine` の中の function は 1 段下
        assert_eq!(
            kids,
            [
                ("Level", "type", "", "M/Sources.mo", false),
                ("Sine", "block", "Sine wave", "M/Sources.mo", true),
                ("Step", "model", "", "M/Sources.mo", false)
            ]
        );
        assert_eq!(tree.children("M.Sources.Sine")[0].path, "M.Sources.Sine.g");
        assert_eq!(tree.file("M.Sources.Sine"), Some("M/Sources.mo"));
    }

    #[test]
    fn search_is_case_insensitive_and_ranked() {
        let tree = LibraryTree::new([
            ("M/Sources.mo", SOURCES),
            ("M/package.mo", "package M\nend M;\n"),
        ]);
        let names = |q| {
            tree.search(q, 10)
                .iter()
                .map(|e| e.path)
                .collect::<Vec<_>>()
        };
        assert_eq!(names("sine"), ["M.Sources.Sine"]);
        assert_eq!(
            names("S"),
            ["M.Sources", "M.Sources.Sine", "M.Sources.Step"]
        );
        assert_eq!(
            names("m.sources.s"),
            ["M.Sources.Sine", "M.Sources.Step", "M.Sources.Sine.g"]
        );
        assert!(names("").is_empty() && names("zzz").is_empty());
        assert_eq!(tree.search("s", 2).len(), 2);
    }

    #[test]
    fn extends_class_with_nested_parentheses_is_read() {
        let src = "package P\n  redeclare model extends B(p(a = 1), q(each x = if true then 1 else 2)) \"d\"\n  end B;\n  type E = enumeration(A \"a\") \"enum\";\nend P;\n";
        let tree = LibraryTree::new([("P.mo", src)]);
        let kids: Vec<_> = tree
            .children("P")
            .iter()
            .map(|e| (e.name, e.kind, e.description))
            .collect();
        assert_eq!(kids, [("B", "model", "d"), ("E", "type", "enum")]);
    }
}

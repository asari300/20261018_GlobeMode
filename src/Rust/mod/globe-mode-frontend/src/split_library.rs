//! ライブラリの分割読み込み（S037）の字句の走査。構文木は作らない。
//!
//! MSL 全体（2556 ファイル）を Rumoca の Session に読むと、wasm の線形メモリが 262 MiB、`ChuaCircuit` を
//! 実行した後で 513 MiB になる（S012）。モデルが使うファイルだけを読めば足りるので、ライブラリのファイルを
//! 本文のまま持ち、モデルと作業中の文書から名前をたどって、使うファイルの集合（閉包）だけを返す。
//!
//! ファイルは `within A.B;` と先頭のクラス `C` から、修飾名 `A.B.C` で引く（ディレクトリ構造に頼らない）。
//! 本文の中の名前の連なり `a.b.c` は、次の道で修飾名にたどる。たどれた先のファイルを足し、その先でも繰り返す。
//! 1. 絶対: `a` をトップレベルの名前として
//! 2. 字句的なスコープ: ファイル自身とその外側のパッケージの子として（`Interfaces.OnePort`）
//! 3. `import X = a.b;`（別名）と `import a.b.*;`（ワイルドカード）、`import a.b.{c, d};`
//!
//! 外側のパッケージのファイル（`package.mo`）は、必ず足す。広めに拾うのは構わない（余計に読むだけ）が、
//! 足りないと Rumoca の名前解決が失敗するので、狭くはしない。
//!
//! shortcut: 1 ファイルに 1 つのトップレベルのクラス（Modelica の構造化ディレクトリの規約）と仮定する。
//! 2 つ目以降のクラスは索引に載らない。そういうライブラリは分割しない読み込み（`Checker::with_library_sources`）で読む。

use std::collections::{BTreeSet, HashMap, HashSet};

/// 字句。名前の連なりと `import` の読み取りに要るものだけ。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tok<'a> {
    Id(&'a str),
    Dot,
    Star,
    Eq,
    Semi,
    LBrace,
    RBrace,
    Other,
}

/// コメント・文字列・数・引用識別子を読み飛ばして字句に分ける。`max` 個で止める（先頭だけ見るとき）。
fn tokens(text: &str, max: usize) -> Vec<Tok<'_>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() && out.len() < max {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    i += 1;
                }
                i += 2;
            }
            c if c.is_ascii_whitespace() => i += 1,
            q @ (b'"' | b'\'') => {
                i += 1;
                while i < b.len() && b[i] != q {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                out.push(Tok::Other);
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Id(&text[start..i]));
            }
            // 数（`1e5` の `e5` を識別子にしない）
            c if c.is_ascii_digit() => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.')
                {
                    i += 1;
                }
                out.push(Tok::Other);
            }
            c => {
                out.push(match c {
                    b'.' => Tok::Dot,
                    b'*' => Tok::Star,
                    b'=' => Tok::Eq,
                    b';' => Tok::Semi,
                    b'{' => Tok::LBrace,
                    b'}' => Tok::RBrace,
                    _ => Tok::Other,
                });
                i += 1;
            }
        }
    }
    out
}

/// `Id (Dot Id)*` を読む。返すのは名前の連なりと、次の字句の位置。
fn chain<'a>(toks: &[Tok<'a>], mut i: usize) -> (Vec<&'a str>, usize) {
    let mut out = Vec::new();
    while let Some(Tok::Id(id)) = toks.get(i) {
        out.push(*id);
        if toks.get(i + 1) == Some(&Tok::Dot) && matches!(toks.get(i + 2), Some(Tok::Id(_))) {
            i += 2;
        } else {
            i += 1;
            break;
        }
    }
    (out, i)
}

/// 修飾名を読むのに見る字句の数（`within` と接頭辞と、クラスの種別と名前が入れば足りる）
const HEADER_TOKENS: usize = 64;

const CLASS_KEYWORDS: [&str; 8] = [
    "class",
    "model",
    "record",
    "block",
    "connector",
    "type",
    "package",
    "function",
];

/// ファイルの先頭の `within` と、先頭のクラスの名前から作る修飾名（`within A.B; model C` なら `A.B.C`）。
/// クラスが見つからなければ None。
fn qualified_name(toks: &[Tok<'_>]) -> Option<String> {
    let mut within = String::new();
    let mut i = 0;
    if toks.first() == Some(&Tok::Id("within")) {
        let (path, next) = chain(toks, 1);
        within = path.join(".");
        i = next;
    }
    let name = toks[i..].windows(2).find_map(|w| match w {
        [Tok::Id(kw), Tok::Id(name)] if CLASS_KEYWORDS.contains(kw) => Some(*name),
        _ => None,
    })?;
    Some(if within.is_empty() {
        name.to_string()
    } else {
        format!("{within}.{name}")
    })
}

/// 修飾名 `A.B.C` の `A`、`A.B`、`A.B.C`（字句的なスコープ）。
fn scopes(qualified: &str) -> Vec<&str> {
    qualified
        .match_indices('.')
        .map(|(i, _)| &qualified[..i])
        .chain([qualified])
        .collect()
}

/// ライブラリのファイル 1 つ。
struct LibFile {
    name: String,
    text: String,
}

/// 分割して読むライブラリ。ファイルの本文と、修飾名の索引と、ファイルごとの依存先（たどった結果の記憶）。
pub(crate) struct SplitLibrary {
    files: Vec<LibFile>,
    /// 修飾名 → ファイルの番号
    tree: HashMap<String, Vec<usize>>,
    /// 先頭のクラスが見つからないファイル（索引に載らないので、いつも読む）
    unindexed: Vec<usize>,
    /// ファイルの番号 → 修飾名
    qualified: Vec<Option<String>>,
    deps: Vec<Option<Vec<usize>>>,
}

impl SplitLibrary {
    pub(crate) fn new(sources: impl IntoIterator<Item = (String, String)>) -> Self {
        let files: Vec<LibFile> = sources
            .into_iter()
            .map(|(name, text)| LibFile { name, text })
            .collect();
        let mut tree: HashMap<String, Vec<usize>> = HashMap::new();
        let mut unindexed = Vec::new();
        let mut qualified = Vec::with_capacity(files.len());
        for (n, f) in files.iter().enumerate() {
            let q = qualified_name(&tokens(&f.text, HEADER_TOKENS));
            match &q {
                Some(q) => tree.entry(q.clone()).or_default().push(n),
                None => unindexed.push(n),
            }
            qualified.push(q);
        }
        let deps = vec![None; files.len()];
        SplitLibrary {
            files,
            tree,
            unindexed,
            qualified,
            deps,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.files.len()
    }

    pub(crate) fn file(&self, n: usize) -> (&str, &str) {
        (&self.files[n].name, &self.files[n].text)
    }

    /// 索引にある修飾名（トップレベルのクラスの完全修飾名。残した文書とのぶつかりを見るのに使う）。
    pub(crate) fn class_names(&self) -> impl Iterator<Item = &str> {
        self.qualified.iter().flatten().map(String::as_str)
    }

    /// 先頭のクラスが見つからないファイル。
    pub(crate) fn unindexed(&self) -> &[usize] {
        &self.unindexed
    }

    /// `model` と `texts`（作業中の文書の本文）が使うファイルの集合。外側のパッケージのファイルを含む。
    pub(crate) fn closure<'t>(
        &mut self,
        model: &str,
        texts: impl IntoIterator<Item = &'t str>,
    ) -> BTreeSet<usize> {
        let mut seeds = HashSet::new();
        self.follow("", &model.split('.').collect::<Vec<_>>(), &mut seeds);
        for text in texts {
            let toks = tokens(text, usize::MAX);
            let own = qualified_name(&toks);
            let scopes = own.as_deref().map(scopes).unwrap_or_default();
            self.references(&toks, &scopes, &mut seeds);
            // 作業中の文書の外側のパッケージ（`within A.B;` の `A`、`A.B`）
            for s in &scopes {
                seeds.extend(self.tree.get(*s).into_iter().flatten());
            }
        }
        let mut closure: BTreeSet<usize> = seeds.into_iter().collect();
        let mut queue: Vec<usize> = closure.iter().copied().collect();
        while let Some(n) = queue.pop() {
            for d in self.deps_of(n) {
                if closure.insert(d) {
                    queue.push(d);
                }
            }
        }
        closure
    }

    fn deps_of(&mut self, n: usize) -> Vec<usize> {
        if let Some(d) = &self.deps[n] {
            return d.clone();
        }
        let toks = tokens(&self.files[n].text, usize::MAX);
        let own = self.qualified[n].clone();
        let scopes = own.as_deref().map(scopes).unwrap_or_default();
        let mut out = HashSet::new();
        self.references(&toks, &scopes, &mut out);
        for s in &scopes {
            out.extend(self.tree.get(*s).into_iter().flatten());
        }
        let mut d: Vec<usize> = out.into_iter().collect();
        d.sort_unstable();
        self.deps[n] = Some(d.clone());
        d
    }

    /// `base` の下で `chain` を、ファイルのあるところまでたどり、通ったファイルを `out` に足す。
    fn follow(&self, base: &str, chain: &[&str], out: &mut HashSet<usize>) {
        let mut cur = base.to_string();
        for seg in chain {
            if !cur.is_empty() {
                cur.push('.');
            }
            cur.push_str(seg);
            match self.tree.get(&cur) {
                Some(files) => out.extend(files),
                None => break,
            }
        }
    }

    /// 字句の列にある名前の連なりをたどる（本文の説明を参照）。
    fn references(&self, toks: &[Tok<'_>], scopes: &[&str], out: &mut HashSet<usize>) {
        let mut chains: HashSet<Vec<&str>> = HashSet::new();
        let mut alias: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut wild: Vec<Vec<&str>> = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            match toks[i] {
                Tok::Id("import") => {
                    let mut j = i + 1;
                    let name = match (toks.get(j), toks.get(j + 1)) {
                        (Some(Tok::Id(a)), Some(Tok::Eq)) => {
                            j += 2;
                            Some(*a)
                        }
                        _ => None,
                    };
                    let (path, next) = chain(toks, j);
                    j = next;
                    if path.is_empty() {
                        i = j.max(i + 1);
                        continue;
                    }
                    if let Some(a) = name {
                        alias.insert(a, path.clone());
                    } else if toks.get(j) == Some(&Tok::Dot) {
                        match toks.get(j + 1) {
                            Some(Tok::Star) => wild.push(path.clone()),
                            Some(Tok::LBrace) => {
                                j += 2;
                                while let Some(t) = toks.get(j) {
                                    j += 1;
                                    match t {
                                        Tok::Id(n) => {
                                            let mut p = path.clone();
                                            p.push(n);
                                            alias.insert(n, p);
                                        }
                                        Tok::RBrace | Tok::Semi => break,
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else if let Some(last) = path.last() {
                        // `import A.B.C;` は `C` を `A.B.C` の別名にする
                        alias.insert(last, path.clone());
                    }
                    chains.insert(path);
                    i = j.max(i + 1);
                }
                Tok::Id(_) => {
                    let (c, next) = chain(toks, i);
                    chains.insert(c);
                    i = next;
                }
                _ => i += 1,
            }
        }
        for ch in &chains {
            self.follow("", ch, out);
            for s in scopes {
                self.follow(s, ch, out);
            }
            if let Some(path) = alias.get(ch[0]) {
                let full: Vec<&str> = path.iter().chain(&ch[1..]).copied().collect();
                self.follow("", &full, out);
            }
            for w in &wild {
                self.follow(&w.join("."), ch, out);
            }
        }
        for path in alias.values() {
            self.follow("", path, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(files: &[(&str, &str)]) -> SplitLibrary {
        SplitLibrary::new(files.iter().map(|(n, t)| (n.to_string(), t.to_string())))
    }

    fn names(l: &SplitLibrary, c: &BTreeSet<usize>) -> Vec<String> {
        c.iter().map(|&n| l.file(n).0.to_string()).collect()
    }

    #[test]
    fn qualified_names_come_from_within_and_the_first_class() {
        let t = |s| qualified_name(&tokens(s, HEADER_TOKENS));
        assert_eq!(
            t("within A.B;\nencapsulated partial model C end C;"),
            Some("A.B.C".into())
        );
        assert_eq!(
            t("// c\n/* within X; */ package P end P;"),
            Some("P".into())
        );
        assert_eq!(t("within ;\nmodel M end M;"), Some("M".into()));
        assert_eq!(t("within A;\n"), None);
        assert_eq!(scopes("A.B.C"), ["A", "A.B", "A.B.C"]);
    }

    #[test]
    fn closure_follows_names_scopes_and_imports() {
        let mut l = lib(&[
            ("L/package.mo", "package L end L;"),
            (
                "L/Top.mo",
                "within L; model Top\n  extends Sub.Base;\n  Util.Fn f;\n  import U = L.Units;\n  import L.Deep.Types;\n  Types.T t;\nend Top;",
            ),
            ("L/Sub/package.mo", "within L; package Sub end Sub;"),
            (
                "L/Sub/Base.mo",
                "within L.Sub; model Base Leaf x; end Base;",
            ),
            ("L/Sub/Leaf.mo", "within L.Sub; model Leaf end Leaf;"),
            ("L/Sub/Other.mo", "within L.Sub; model Other end Other;"),
            (
                "L/Util.mo",
                "within L; package Util function Fn end Fn; end Util;",
            ),
            ("L/Units.mo", "within L; package Units end Units;"),
            ("L/Unused.mo", "within L; model Unused end Unused;"),
            ("L/Deep/package.mo", "within L; package Deep end Deep;"),
            (
                "L/Deep/Types/package.mo",
                "within L.Deep; package Types end Types;",
            ),
            ("L/Deep/Types/T.mo", "within L.Deep.Types; type T = Real;"),
            ("W/package.mo", "package W end W;"),
            ("W/A.mo", "within W; model A end A;"),
            (
                "W/Wild.mo",
                "within W; model Wild import W.*; A a; end Wild;",
            ),
        ]);
        let c = l.closure("L.Top", []);
        assert_eq!(
            names(&l, &c),
            [
                "L/package.mo",
                "L/Top.mo",
                "L/Sub/package.mo",
                "L/Sub/Base.mo",
                "L/Sub/Leaf.mo",
                "L/Util.mo",
                "L/Units.mo",
                "L/Deep/package.mo",
                "L/Deep/Types/package.mo",
                "L/Deep/Types/T.mo"
            ]
        );
        // ワイルドカードの import と、作業中の文書（within と参照）からも始められる
        let c = l.closure(
            "Nothing",
            ["within W; model Mine import W.*; A a; end Mine;"],
        );
        assert_eq!(names(&l, &c), ["W/package.mo", "W/A.mo"]);
        // モデルが無ければ何も足さない
        assert!(l.closure("X.Y", []).is_empty());
    }

    #[test]
    fn files_without_a_class_are_listed_as_unindexed() {
        let l = lib(&[("a.mo", "model A end A;"), ("b.mo", "// nothing")]);
        assert_eq!(l.unindexed(), [1]);
        assert_eq!(l.class_names().collect::<Vec<_>>(), ["A"]);
    }
}

//! 定義ジャンプと参照（S132）。`Outline` だけで名前を引く（新しい解析は入れない）。
//!
//! 定義: カーソルの点つきの名前（頭からカーソルのある区切りまで）を、内側のクラスから外へ
//! 部品名として探し、無ければ型名として `ClassIndex::resolve` と同じ順（入れ子 → import → 外側）で引く。
//! クラスは開いている文書（ワークスペース）を先に、無ければライブラリの木（S127）でファイルを決めて読む。
//! shortcut: 受け継いだ入れ子のクラス・コメントや文字列の中の名前・for の添字は区別しない。要るなら字句の位置を持つ解析にする。

use globe_mode::{Outline, Workspace};
use globe_mode_core::TextRange;
use globe_mode_fmt::LibraryTree;
use globe_mode_ir::{ClassOutline, ComponentOutline};

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// カーソルの名前（`R1.p` の `p` の上なら `R1.p`、`R1` の上なら `R1`）と、その頭のバイト位置。
pub fn name_at(text: &str, at: usize) -> Option<(usize, &str)> {
    let at = at.min(text.len());
    let start = text[..at]
        .char_indices()
        .rev()
        .find(|&(_, c)| !(is_ident(c) || c == '.'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    let end = at + text[at..].find(|c| !is_ident(c)).unwrap_or(text.len() - at);
    let start = start + text[start..end].len() - text[start..end].trim_start_matches('.').len();
    let name = text[start..end].trim_end_matches('.');
    (!name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit())).then_some((start, name))
}

/// 位置を含むクラスの列（外から内へ）。
fn enclosing(classes: &[ClassOutline], at: u32) -> Vec<&ClassOutline> {
    let mut out = Vec::new();
    let mut level = classes;
    while let Some(c) = level
        .iter()
        .find(|c| c.range.start <= at && at < c.range.end)
    {
        out.push(c);
        level = &c.classes;
    }
    out
}

/// `within` の下の `rest`（`A.B`）を、アウトラインの入れ子のクラスからたどる。
fn walk<'o>(o: &'o Outline, qualified: &str) -> Option<&'o ClassOutline> {
    let rest = match o.within.as_deref().filter(|w| !w.is_empty()) {
        Some(w) => qualified.strip_prefix(w)?.strip_prefix('.')?,
        None => qualified,
    };
    let mut parts = rest.split('.');
    let first = parts.next()?;
    let mut cur = o.classes.iter().find(|c| c.name == first)?;
    for p in parts {
        cur = cur.classes.iter().find(|c| c.name == p)?;
    }
    Some(cur)
}

/// 定義の場所（ワークスペースのパスとバイトの範囲）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: String,
    pub range: TextRange,
}

/// 名前を引く道具。`scratch` はライブラリのファイルを読んでアウトラインにするだけの別のワークスペース。
pub struct Resolver<'a> {
    pub ws: &'a Workspace,
    pub tree: &'a LibraryTree,
    pub scratch: &'a mut Workspace,
}

impl Resolver<'_> {
    /// 完全修飾名のクラスを探す（ファイルのパスと、クラスの写し）。
    fn locate(&mut self, qualified: &str) -> Option<(String, ClassOutline)> {
        for f in self.ws.files() {
            if let Some(Ok(o)) = self.ws.outline(f)
                && let Some(c) = walk(o, qualified)
            {
                return Some((f.to_string(), c.clone()));
            }
        }
        // ライブラリ: 名前の頭から一番長く木にあるもののファイルを読む
        let mut head = qualified;
        let file = loop {
            if let Some(f) = self.tree.file(head) {
                break f.to_string();
            }
            head = &head[..head.rfind('.')?];
        };
        if self.scratch.outline(&file).is_none() {
            let text = std::fs::read_to_string(&file).ok()?;
            self.scratch.set_file(&file, text);
        }
        let Some(Ok(o)) = self.scratch.outline(&file) else {
            return None;
        };
        walk(o, qualified).map(|c| (file, c.clone()))
    }

    /// `scope`（完全修飾名）の中から見た型名 `name` を完全修飾名にして探す。
    fn resolve(&mut self, scope: &str, name: &str) -> Option<(String, String, ClassOutline)> {
        let first = name.split('.').next().unwrap_or(name);
        let rest = &name[first.len()..];
        let mut parts: Vec<&str> = scope.split('.').filter(|p| !p.is_empty()).collect();
        loop {
            let joined = parts.join(".");
            let candidate = if joined.is_empty() {
                name.to_string()
            } else {
                format!("{joined}.{name}")
            };
            if let Some((f, c)) = self.locate(&candidate) {
                return Some((candidate, f, c));
            }
            let imports = if joined.is_empty() {
                vec![]
            } else {
                self.locate(&joined)
                    .map(|(_, c)| c.imports)
                    .unwrap_or_default()
            };
            for import in imports {
                let target = if let Some((alias, path)) = import.split_once('=') {
                    (alias == first).then(|| path.to_string())
                } else if let Some(package) = import.strip_suffix(".*") {
                    Some(format!("{package}.{first}"))
                } else {
                    (import.rsplit('.').next() == Some(first)).then(|| import.clone())
                };
                if let Some(t) = target.map(|t| format!("{t}{rest}"))
                    && let Some((f, c)) = self.locate(&t)
                {
                    return Some((t, f, c));
                }
            }
            parts.pop()?;
        }
    }

    /// クラス `q`（ファイル `file`）の部品 `name`。無ければ extends の先を探す（深さ 8 まで）。
    /// 返すのは (部品を宣言したクラスの完全修飾名, ファイル, 部品)。
    fn component(
        &mut self,
        q: &str,
        file: &str,
        class: &ClassOutline,
        name: &str,
        depth: u32,
    ) -> Option<(String, String, ComponentOutline)> {
        if let Some(c) = class.components.iter().find(|c| c.name == name) {
            return Some((q.to_string(), file.to_string(), c.clone()));
        }
        if depth > 8 {
            return None;
        }
        for e in &class.extends {
            if let Some((eq, ef, ec)) = self.resolve(q, e)
                && let Some(found) = self.component(&eq, &ef, &ec, name, depth + 1)
            {
                return Some(found);
            }
        }
        None
    }

    /// 文書 `path`（本文 `text`）の `at` にある名前の定義。
    pub fn definition(&mut self, path: &str, text: &str, at: usize) -> Option<Target> {
        let (_, name) = name_at(text, at)?;
        let Some(Ok(o)) = self.ws.outline(path) else {
            return None;
        };
        let chain: Vec<ClassOutline> = enclosing(&o.classes, at as u32)
            .into_iter()
            .cloned()
            .collect();
        let within = o.within.clone().unwrap_or_default();
        // 内側のクラスの完全修飾名（外から順に）
        let mut scopes = Vec::new();
        let mut q = within.clone();
        for c in &chain {
            q = if q.is_empty() {
                c.name.clone()
            } else {
                format!("{q}.{}", c.name)
            };
            scopes.push(q.clone());
        }
        let mut segs = name.split('.');
        let head = segs.next()?;
        // 部品名（内側のクラスから外へ、extends の先も）
        for (c, q) in chain.iter().zip(&scopes).rev() {
            let Some((mut decl_q, mut file, mut comp)) = self.component(q, path, c, head, 0) else {
                continue;
            };
            for s in segs {
                // 部品の型は、宣言したクラスの中から引く
                let (tq, tf, tc) = self.resolve(&decl_q, &comp.type_name)?;
                (decl_q, file, comp) = self.component(&tq, &tf, &tc, s, 0)?;
            }
            return Some(Target {
                path: file,
                range: comp.range,
            });
        }
        // 型名
        let scope = scopes.last().cloned().unwrap_or(within);
        let (_, file, c) = self.resolve(&scope, name)?;
        Some(Target {
            path: file,
            range: c.range,
        })
    }
}

/// 同じ文書で、カーソルの名前と字句が同じ位置（カーソルを含む一番内側のクラスの中、無ければ文書全体）。
pub fn references(outline: Option<&Outline>, text: &str, at: usize) -> Vec<TextRange> {
    let Some((_, name)) = name_at(text, at) else {
        return vec![];
    };
    let (lo, hi) = outline
        .and_then(|o| enclosing(&o.classes, at as u32).last().map(|c| c.range))
        .map_or((0, text.len()), |r| (r.start as usize, r.end as usize));
    let scope = &text[lo..hi.min(text.len())];
    scope
        .match_indices(name)
        .filter(|&(i, _)| {
            let before = scope[..i].chars().next_back();
            let after = scope[i + name.len()..].chars().next();
            !before.is_some_and(|c| is_ident(c) || c == '.') && !after.is_some_and(is_ident)
        })
        .map(|(i, _)| TextRange {
            start: (lo + i) as u32,
            end: (lo + i + name.len()) as u32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_at_cuts_at_the_segment_under_the_cursor() {
        let t = "  R1.p.v = 0;";
        assert_eq!(name_at(t, 3), Some((2, "R1")));
        assert_eq!(name_at(t, 5), Some((2, "R1.p")));
        assert_eq!(name_at(t, 7), Some((2, "R1.p.v")));
        assert_eq!(name_at(t, 10), None);
        assert_eq!(name_at("x = 12;", 5), None);
    }
}

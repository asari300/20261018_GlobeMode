//! 定義ジャンプと参照（S132）。`Outline` だけで名前を引く（新しい解析は入れない）。
//!
//! 定義: カーソルの点つきの名前（頭からカーソルのある区切りまで）を、内側のクラスから外へ
//! 部品名として探し、無ければ型名として `ClassIndex::resolve` と同じ順（入れ子 → import → 外側）で引く。
//! クラスは開いている文書（ワークスペース）を先に、無ければライブラリの木（S127）でファイルを決めて読む。
//! shortcut: 受け継いだ入れ子のクラス・コメントや文字列の中の名前・for の添字は区別しない。要るなら字句の位置を持つ解析にする。

use globe_mode::{Outline, Workspace};
use globe_mode_core::TextRange;
use globe_mode_fmt::{Kind, LibraryTree, lex};
use globe_mode_ir::{ClassOutline, ComponentOutline};
use lsp_types::Position;
use serde_json::{Value, json};

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
    /// クラスの定義か（false なら部品の宣言。範囲は名前だけ）
    pub class: bool,
}

/// 名前を引く道具。`scratch` はライブラリのファイルを読んでアウトラインにするだけの別のワークスペース。
#[allow(missing_debug_implementations)] // dyn Fn を持つ
pub struct Resolver<'a> {
    pub ws: &'a Workspace,
    pub tree: &'a LibraryTree,
    pub scratch: &'a mut Workspace,
    /// ライブラリのファイル名 → 本文（LSP はディスク、wasm は読み込んだ組。S138）
    pub read: &'a dyn Fn(&str) -> Option<String>,
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
            let text = (self.read)(&file)?;
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
                class: false,
            });
        }
        // 型名
        let scope = scopes.last().cloned().unwrap_or(within);
        let (_, file, c) = self.resolve(&scope, name)?;
        Some(Target {
            path: file,
            range: c.range,
            class: true,
        })
    }
}

/// UI 向けの定義ジャンプ（S138。wasm と `globe/definition` が使う）。文書 `path`（本文 `text`）の行・列（LSP と同じ UTF-16）にある
/// 名前の定義を `{file, text, range}`（`text` は定義のあるファイルの本文）で返す。
pub fn definition_at(
    r: &mut Resolver,
    path: &str,
    text: &str,
    line: u32,
    character: u32,
) -> Option<Value> {
    let at = crate::offset(text, Position::new(line, character));
    let t = r.definition(path, text, at)?;
    let body = if t.path == path {
        text.to_string()
    } else {
        (r.read)(&t.path)?
    };
    Some(json!({"file": t.path, "range": crate::range(&body, t.range), "text": body}))
}

/// UI 向けのホバー（S149）。`definition_at` と同じ解決で、定義の宣言を 1 つの文字列 `contents` にして
/// `{contents, range}`（`range` はカーソルの名前、LSP の Range）で返す。クラスは見出しの 1 行（`model Resistor "説明"`）、
/// 部品は宣言の行頭から `;` まで（`Real x(start = 1)`。説明文字列があれば入る）で、空白は 1 つにまとめる。
/// shortcut: 宣言が行の途中から始まるときは行頭から取る・文字列の中の `;` で切れる。要るなら宣言の範囲を持つ解析にする。
pub fn hover_at(
    r: &mut Resolver,
    path: &str,
    text: &str,
    line: u32,
    character: u32,
) -> Option<Value> {
    let at = crate::offset(text, Position::new(line, character));
    let (start, name) = name_at(text, at)?;
    let t = r.definition(path, text, at)?;
    let body = if t.path == path {
        text.to_string()
    } else {
        (r.read)(&t.path)?
    };
    let from = body[..t.range.start as usize]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let decl = body[from..]
        .split(if t.class { '\n' } else { ';' })
        .next()?;
    let contents = decl.split_whitespace().collect::<Vec<_>>().join(" ");
    let name_range = TextRange {
        start: start as u32,
        end: (start + name.len()) as u32,
    };
    Some(json!({"contents": contents, "range": crate::range(text, name_range)}))
}

/// 同じ文書で、カーソルの名前と同じ識別子のトークンの位置（カーソルを含む一番内側のクラスの中、無ければ文書全体）。
/// 文字列・コメントの中は対象にしない（`globe-mode-fmt` の字句解析。S161）。点つきの名前（`R1.p`）は `R1 . p` と並ぶトークン列、
/// 前が `.` のもの（別の部品のメンバ）は除く。字句に分けられない本文（閉じていない文字列など）は空。カーソルが識別子の上でなければ空。
pub fn references(outline: Option<&Outline>, text: &str, at: usize) -> Vec<TextRange> {
    let Some((_, name)) = name_at(text, at) else {
        return vec![];
    };
    let Ok(tokens) = lex(text) else {
        return vec![];
    };
    let on_ident = tokens
        .iter()
        .any(|t| t.kind == Kind::Ident && t.start <= at && at <= t.start + t.text.len());
    if !on_ident {
        return vec![];
    }
    let (lo, hi) = outline
        .and_then(|o| enclosing(&o.classes, at as u32).last().map(|c| c.range))
        .map_or((0, text.len()), |r| (r.start as usize, r.end as usize));
    let segs: Vec<&str> = name.split('.').collect();
    let width = segs.len() * 2 - 1; // 識別子と `.` が交互に並ぶ
    (0..tokens.len().saturating_sub(width - 1))
        .filter(|&i| {
            let run = &tokens[i..i + width];
            run[0].start >= lo
                && run[width - 1].start + run[width - 1].text.len() <= hi
                && !(i > 0 && tokens[i - 1].text == "." && tokens[i - 1].kind == Kind::Punct)
                && run.iter().enumerate().all(|(k, t)| {
                    if k % 2 == 0 {
                        t.kind == Kind::Ident && t.text == segs[k / 2]
                    } else {
                        t.kind == Kind::Punct && t.text == "."
                    }
                })
        })
        .map(|i| TextRange {
            start: tokens[i].start as u32,
            end: (tokens[i + width - 1].start + tokens[i + width - 1].text.len()) as u32,
        })
        .collect()
}

/// UI 向けの参照（S161。wasm と `globe/references` が使う）。`references` の範囲を LSP の Range の配列にする。
pub fn references_at(outline: Option<&Outline>, text: &str, line: u32, character: u32) -> Value {
    let at = crate::offset(text, Position::new(line, character));
    json!(
        references(outline, text, at)
            .into_iter()
            .map(|r| crate::range(text, r))
            .collect::<Vec<_>>()
    )
}

/// リネームで置き換える範囲（S161）。カーソルの名前が、いちばん内側のクラスで宣言した部品のときだけ受け、`references` と同じ出現を返す。
/// 断る理由は文字列で返す: 部品でない（型・クラス・継承した部品・`a.b` の `b`・予約語の上）、新しい名前が識別子でない・予約語・同じクラスに既にある。
/// shortcut: 同名の入れ子のクラスの中は区別しない。要るなら宣言の位置を持つ解析にする。
pub fn rename(
    outline: Option<&Outline>,
    text: &str,
    at: usize,
    new_name: &str,
) -> Result<Vec<TextRange>, String> {
    let (_, old) = name_at(text, at).ok_or("名前の上でない")?;
    let class = outline
        .and_then(|o| enclosing(&o.classes, at as u32).last().copied())
        .filter(|c| c.components.iter().any(|m| m.name == old))
        .ok_or_else(|| {
            format!(
                "{old} はこのクラスで宣言した部品でない（型・クラス・継承した部品・別の部品のメンバのリネームは未対応）"
            )
        })?;
    let mut chars = new_name.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "{new_name:?} は Modelica の識別子でない（英字か _ で始まる英数字と _）"
        ));
    }
    if globe_mode_fmt::KEYWORDS.contains(&new_name) {
        return Err(format!("{new_name} は予約語"));
    }
    if new_name != old
        && (class.components.iter().any(|m| m.name == new_name)
            || class.classes.iter().any(|c| c.name == new_name))
    {
        return Err(format!("{new_name} は同じクラスに既にある"));
    }
    let hits = references(outline, text, at);
    if hits.is_empty() {
        return Err(
            "カーソルが識別子の上にない（コメント・文字列の中、または字句に分けられない本文）"
                .into(),
        );
    }
    Ok(hits)
}

/// UI 向けのリネーム（S161。wasm と `globe/rename` が使う）。LSP の TextEdit（`{range, newText}`）の配列。
pub fn rename_at(
    outline: Option<&Outline>,
    text: &str,
    line: u32,
    character: u32,
    new_name: &str,
) -> Result<Value, String> {
    let at = crate::offset(text, Position::new(line, character));
    let edits = rename(outline, text, at, new_name)?;
    Ok(json!(
        edits
            .into_iter()
            .map(|r| json!({"range": crate::range(text, r), "newText": new_name}))
            .collect::<Vec<_>>()
    ))
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

    #[test]
    fn hover_shows_the_declaration_and_description() {
        let text = "model A \"the model\"\n  Real x(start = 1)\n    \"state\";\n  A y;\nequation\n  der(x) = -x;\nend A;\n";
        let (tree, mut scratch) = (LibraryTree::new(std::iter::empty()), Workspace::new());
        let mut ws = Workspace::new();
        ws.set_file("A.mo", text.to_string());
        let mut r = Resolver {
            ws: &ws,
            tree: &tree,
            scratch: &mut scratch,
            read: &|_| None,
        };
        let h = hover_at(&mut r, "A.mo", text, 5, 12).unwrap(); // -x の x
        assert_eq!(h["contents"], "Real x(start = 1) \"state\"", "{h}");
        assert_eq!(h["range"]["start"]["character"], 12, "{h}");
        let h = hover_at(&mut r, "A.mo", text, 3, 2).unwrap(); // `A y;` の型 A
        assert_eq!(h["contents"], "model A \"the model\"", "{h}");
        assert_eq!(hover_at(&mut r, "A.mo", text, 0, 0), None); // 予約語
    }

    #[test]
    fn rename_replaces_every_occurrence_and_refuses_three_ways() {
        // 説明文字列・`//`・`/* */` の x は対象外
        let text = "model A\n  Real x \"x position\"; // x\n  Real xx;\n  A.B y;\nequation\n  /* x */ der(x) = -x + xx + y.x;\nend A;\n";
        let mut ws = Workspace::new();
        ws.set_file("A.mo", text.to_string());
        let o = ws.outline("A.mo").and_then(|r| r.as_ref().ok());
        let at = text.find("x \"").unwrap();
        let hits = |new: &str| rename(o, text, at, new);
        // `xx` と `y.x` は含めない。宣言と der(x) と -x の 3 箇所（文字列・コメントの x は含めない）
        assert_eq!(hits("z").unwrap().len(), 3);
        let comment = text.find("// x").unwrap() + 3; // コメントの中のカーソルは断る
        assert!(
            rename(o, text, comment, "z")
                .unwrap_err()
                .contains("識別子の上にない")
        );
        for r in hits("z").unwrap() {
            assert_eq!(&text[r.start as usize..r.end as usize], "x");
            assert!(
                !text[..r.start as usize].ends_with("\"")
                    && !text[..r.start as usize].ends_with("// ")
            );
        }
        assert!(hits("1z").unwrap_err().contains("識別子"));
        assert!(hits("x y").unwrap_err().contains("識別子"));
        assert!(hits("model").unwrap_err().contains("予約語"));
        assert!(hits("xx").unwrap_err().contains("既に"));
        // 部品でない名前（型 Real の上）は未対応
        assert!(rename(o, text, text.find("Real").unwrap() + 1, "z").is_err());
        let r = rename_at(o, text, 1, 7, "z").unwrap();
        assert_eq!(r[0]["newText"], "z");
        assert_eq!(r[0]["range"]["start"]["line"], 1);
    }
}

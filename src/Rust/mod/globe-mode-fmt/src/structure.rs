//! 文の切り出しと段付け。クラス・節・ブロック（if / for / while / when）を見分けて、
//! 各文（要素）に段（インデントの深さ）と「前に空行を置くか」を付ける。

use crate::lexer::Token;
use crate::{FormatError, err};

#[derive(Debug)]
pub struct Element<'t, 'a> {
    pub tokens: &'t [Token<'a>],
    /// 段（0 がファイルの最上位）
    pub indent: usize,
    pub blank_before: bool,
}

#[derive(Debug)]
enum Ctx<'a> {
    Class(&'a str),
    /// 開いた if / for / while / when（`end <種類>` との対応を確かめる）
    Block(&'a str),
}

const CLASS_PREFIX: &[&str] = &[
    "partial",
    "encapsulated",
    "expandable",
    "pure",
    "impure",
    "replaceable",
    "redeclare",
    "final",
    "inner",
    "outer",
];
const CLASS_KEYWORD: &[&str] = &[
    "model",
    "package",
    "function",
    "record",
    "block",
    "connector",
    "type",
    "class",
    "operator",
];
const BLOCK_END: &[&str] = &["if", "for", "while", "when"];

/// `blank_sections`: `equation` / `algorithm` の前後に空行を置く（S061、`Options::blank_around_sections`）。
pub fn split<'t, 'a>(
    tokens: &'t [Token<'a>],
    blank_sections: bool,
) -> Result<Vec<Element<'t, 'a>>, FormatError> {
    let mut out: Vec<Element<'t, 'a>> = Vec::new();
    let mut stack: Vec<Ctx<'a>> = Vec::new();
    let mut force_blank = false;
    let mut i = 0;
    let n = tokens.len();
    let at = |i: usize| tokens.get(i).map_or(usize::MAX, |t| t.start);
    while i < n {
        let t = &tokens[i];
        let mut blank = force_blank || (t.newlines_before >= 2 && !out.is_empty());
        force_blank = false;
        if t.is_comment() {
            // 文の後ろに続くコメントは、その文の要素が下で取り込むので、ここに来るのは行頭のコメントだけ
            let indent = stack.len();
            out.push(Element {
                tokens: &tokens[i..=i],
                indent,
                blank_before: blank,
            });
            i += 1;
            continue;
        }
        let start = i;
        let mut indent = stack.len();
        if t.is("end") {
            let name = tokens
                .get(i + 1)
                .ok_or_else(|| err("end の後ろに名前が無い", t.start))?;
            if !tokens.get(i + 2).is_some_and(|s| s.is(";")) {
                return Err(err("end の後ろに ; が無い", at(i + 2)));
            }
            let is_block = BLOCK_END.contains(&name.text) && name.is_keyword();
            match (stack.pop(), is_block) {
                (Some(Ctx::Block(k)), true) if k == name.text => {}
                (Some(Ctx::Block(k)), true) => {
                    return Err(err(
                        format!("end {} が {} と合わない", name.text, k),
                        name.start,
                    ));
                }
                (Some(Ctx::Class(c)), false) if c == name.text => blank = true,
                (Some(Ctx::Class(c)), false) => {
                    return Err(err(
                        format!("end {} が {} と合わない", name.text, c),
                        name.start,
                    ));
                }
                _ => return Err(err(format!("対応しない end {}", name.text), t.start)),
            }
            indent = stack.len();
            i += 3;
        } else if is_section(tokens, i) {
            if blank_sections && !(t.is("public") || t.is("protected")) {
                blank = true;
                force_blank = true;
            }
            i += if t.is("initial") { 2 } else { 1 };
            indent = class_level(&stack);
        } else if t.is("if") || t.is("for") || t.is("while") || t.is("when") {
            i = scan_to(tokens, i, |s| s.is("then") || s.is("loop"))?;
            stack.push(Ctx::Block(t.text));
        } else if t.is("else") || t.is("elseif") || t.is("elsewhen") {
            if !matches!(stack.last(), Some(Ctx::Block(_))) {
                return Err(err("ブロックの外の else", t.start));
            }
            indent = stack.len() - 1;
            i = if t.is("else") {
                i + 1
            } else {
                scan_to(tokens, i, |s| s.is("then"))?
            };
        } else if let Some((kw, name)) = class_header(tokens, i) {
            if tokens.get(name + 1).is_some_and(|s| s.is("=")) {
                // 短いクラス（type X = Real;）は文として扱う
                i = scan_to(tokens, i, |s| s.is(";"))?;
            } else if tokens.get(kw + 1).is_some_and(|s| s.is("extends")) {
                // `model extends X(mods) "desc"` … `end X;`
                i = name + 1;
                while tokens
                    .get(i)
                    .is_some_and(|s| s.is("(") || s.kind == crate::lexer::Kind::Str || s.is("+"))
                {
                    i = if tokens[i].is("(") {
                        skip_group(tokens, i)?
                    } else {
                        i + 1
                    };
                }
                stack.push(Ctx::Class(tokens[name].text));
                force_blank = true;
            } else {
                i = name + 1;
                while tokens
                    .get(i)
                    .is_some_and(|s| s.kind == crate::lexer::Kind::Str || s.is("+"))
                {
                    i += 1;
                }
                stack.push(Ctx::Class(tokens[name].text));
                force_blank = true;
            }
        } else {
            i = scan_to(tokens, i, |s| s.is(";"))?;
            if t.is("within") {
                force_blank = true;
            }
            if t.is("annotation") {
                indent = class_level(&stack);
            }
        }
        // 同じ行に続くコメント
        while tokens
            .get(i)
            .is_some_and(|s| s.is_comment() && s.newlines_before == 0)
        {
            i += 1;
        }
        out.push(Element {
            tokens: &tokens[start..i],
            indent,
            blank_before: blank,
        });
    }
    if let Some(ctx) = stack.last() {
        let what = match ctx {
            Ctx::Class(c) => format!("end {c}; が無い"),
            Ctx::Block(_) => "end if / for / while / when が無い".to_string(),
        };
        return Err(err(what, tokens.last().map_or(0, |t| t.start)));
    }
    Ok(out)
}

/// ファイルの中の全クラス定義（入れ子を含む。短いクラス `type X = Real;` も）。S167: ライブラリの木が入れ子を出すのに使う。
/// `path` はファイルの中の入れ子の名前（`Sources.Sine`）、`kind` は `model`・`operator record` など、`description` は説明文字列（無ければ空）。
pub fn class_defs<'a>(tokens: &[Token<'a>]) -> Result<Vec<(String, String, &'a str)>, FormatError> {
    let mut stack: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for e in split(tokens, false)? {
        let t = e.tokens;
        if t[0].is("end") {
            // `end if;` などのブロックは split が対応を確かめている。クラスの end だけ閉じる
            if !t
                .get(1)
                .is_some_and(|n| BLOCK_END.contains(&n.text) && n.is_keyword())
            {
                stack.pop();
            }
        } else if let Some((_, name)) = class_header(t, 0) {
            let short = t.get(name + 1).is_some_and(|s| s.is("="));
            let kind = t[..name]
                .iter()
                .map(|k| k.text)
                .filter(|k| crate::tree::KINDS.contains(k))
                .collect::<Vec<_>>()
                .join(" ");
            // 括弧の外で最初の文字列（`enumeration(A "a")` の中は説明でない）。annotation より前
            let mut depth = 0;
            let description = t[name + 1..]
                .iter()
                .take_while(|s| !s.is("annotation"))
                .find(|s| {
                    depth += i32::from(s.is("(")) - i32::from(s.is(")"));
                    depth == 0 && s.kind == crate::lexer::Kind::Str
                })
                .map_or("", |s| {
                    s.text
                        .strip_prefix('"')
                        .and_then(|d| d.strip_suffix('"'))
                        .unwrap_or(s.text)
                });
            stack.push(t[name].text);
            out.push((stack.join("."), kind, description));
            if short {
                stack.pop();
            }
        }
    }
    Ok(out)
}

/// クラスの範囲（S232）。`path` はファイルの中の入れ子の名前（`Sources.Sine`）、`start`・`end` は宣言の先頭と `end X;` の `;` の次のバイト位置、
/// `annotation` はクラス直下の `annotation(…);`（短いクラスは宣言の中の `annotation(…)`）のトークン列。
pub struct ClassSpan<'t, 'a> {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub annotation: Option<&'t [Token<'a>]>,
    /// `extends A.B(…)` の名前（出現の順。短いクラス `type X = A.B` の `A.B` も。S244: アイコンを 1 段重ねる）
    pub extends: Vec<String>,
}

/// `tokens[i..]` の先頭の `A.B.C`（点でつないだ識別子）。
pub(crate) fn dotted(tokens: &[Token<'_>], mut i: usize) -> String {
    let mut out = String::new();
    while let Some(t) = tokens
        .get(i)
        .filter(|t| t.kind == crate::lexer::Kind::Ident || t.is("."))
    {
        out.push_str(t.text);
        i += 1;
    }
    out
}

/// ファイルの中の全クラスの範囲（入れ子を含む。出現の順）。文を切り出せなければ `Err`。
pub fn class_spans<'t, 'a>(tokens: &'t [Token<'a>]) -> Result<Vec<ClassSpan<'t, 'a>>, FormatError> {
    let mut stack: Vec<usize> = Vec::new(); // out の添字（開いているクラス）
    let mut out: Vec<ClassSpan<'t, 'a>> = Vec::new();
    for e in split(tokens, false)? {
        let t = e.tokens;
        if t[0].is("end") {
            if !t
                .get(1)
                .is_some_and(|n| BLOCK_END.contains(&n.text) && n.is_keyword())
                && let Some(i) = stack.pop()
            {
                out[i].end = t.get(2).map_or(t[0].start, |s| s.start + 1);
            }
        } else if let Some((_, name)) = class_header(t, 0) {
            let path = match stack.last() {
                Some(&p) => format!("{}.{}", out[p].path, t[name].text),
                None => t[name].text.to_string(),
            };
            let mut span = ClassSpan {
                path,
                start: t[0].start,
                end: 0,
                annotation: None,
                extends: Vec::new(),
            };
            if t.get(name + 1).is_some_and(|s| s.is("=")) {
                span.extends.push(dotted(t, name + 2));
                span.end = t.last().map_or(0, |s| s.start + s.text.len());
                span.annotation = t.iter().position(|s| s.is("annotation")).map(|p| &t[p..]);
            } else {
                stack.push(out.len());
            }
            out.push(span);
        } else if t[0].is("annotation")
            && let Some(&i) = stack.last()
        {
            out[i].annotation = Some(t);
        } else if t[0].is("extends")
            && let Some(&i) = stack.last()
        {
            out[i].extends.push(dotted(t, 1));
        }
    }
    Ok(out)
}

fn class_level(stack: &[Ctx<'_>]) -> usize {
    match stack.last() {
        Some(Ctx::Class(_)) => stack.len() - 1,
        _ => stack.len(),
    }
}

fn is_section(tokens: &[Token<'_>], i: usize) -> bool {
    let t = &tokens[i];
    (t.is("equation") || t.is("algorithm") || t.is("public") || t.is("protected"))
        || (t.is("initial")
            && tokens
                .get(i + 1)
                .is_some_and(|s| s.is("equation") || s.is("algorithm")))
}

/// 接頭辞を飛ばしてクラスの頭なら (キーワードの位置, 名前の位置) を返す。
fn class_header(tokens: &[Token<'_>], mut i: usize) -> Option<(usize, usize)> {
    while tokens
        .get(i)
        .is_some_and(|t| t.kind == crate::lexer::Kind::Ident && CLASS_PREFIX.contains(&t.text))
    {
        i += 1;
    }
    let kw = tokens.get(i)?;
    if kw.kind != crate::lexer::Kind::Ident || !CLASS_KEYWORD.contains(&kw.text) {
        return None;
    }
    // `operator record` / `operator function`
    let kw_i = if kw.is("operator")
        && tokens
            .get(i + 1)
            .is_some_and(|t| t.is("record") || t.is("function"))
    {
        i + 1
    } else {
        i
    };
    let name_i = if tokens.get(kw_i + 1).is_some_and(|t| t.is("extends")) {
        kw_i + 2
    } else {
        kw_i + 1
    };
    let name = tokens.get(name_i)?;
    (name.kind == crate::lexer::Kind::Ident && !name.is_keyword()).then_some((kw_i, name_i))
}

/// `tokens[open]`（`(`）に対応する閉じ括弧の次の位置（S167: `redeclare model extends X(p(a = 1))` の修飾を読み飛ばす）。
pub(crate) fn skip_group(tokens: &[Token<'_>], open: usize) -> Result<usize, FormatError> {
    let mut depth = 0i32;
    for (i, t) in tokens.iter().enumerate().skip(open) {
        depth += i32::from(t.is("(") || t.is("[") || t.is("{"));
        depth -= i32::from(t.is(")") || t.is("]") || t.is("}"));
        if depth == 0 {
            return Ok(i + 1);
        }
    }
    Err(err("括弧が閉じない", tokens[open].start))
}

/// 括弧の深さ 0 で `stop` を満たすトークンまで進み、その次の位置を返す。
fn scan_to(
    tokens: &[Token<'_>],
    from: usize,
    stop: impl Fn(&Token<'_>) -> bool,
) -> Result<usize, FormatError> {
    let mut depth = 0i32;
    let mut i = from;
    while i < tokens.len() {
        let t = &tokens[i];
        if t.is("(") || t.is("[") || t.is("{") {
            depth += 1;
        } else if t.is(")") || t.is("]") || t.is("}") {
            depth -= 1;
            if depth < 0 {
                return Err(err("閉じ括弧が多い", t.start));
            }
        } else if depth == 0 && stop(t) {
            return Ok(i + 1);
        } else if depth == 0
            && i > from
            && matches!(
                t.text,
                "equation" | "algorithm" | "public" | "protected" | "end"
            )
            && t.is_keyword()
        {
            return Err(err(
                format!("文の途中に {}（; が抜けている）", t.text),
                t.start,
            ));
        }
        i += 1;
    }
    Err(err("文が終わらない（; が無い）", tokens[from].start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn shape(src: &str) -> Vec<(usize, bool, String)> {
        let t = lex(src).unwrap();
        split(&t, false)
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e.indent,
                    e.blank_before,
                    e.tokens
                        .iter()
                        .map(|t| t.text)
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            })
            .collect()
    }

    #[test]
    fn classes_sections_and_blocks() {
        let s = shape(
            "within A;\nmodel M \"d\"\n  Real x; // c\nequation\n  if x > 0 then\n    x = 1;\n  else\n    x = 2;\n  end if;\n  annotation(a = 1);\nend M;",
        );
        assert_eq!(
            s,
            vec![
                (0, false, "within A ;".into()),
                (0, true, "model M \"d\"".into()),
                (1, true, "Real x ; // c".into()),
                (0, false, "equation".into()),
                (1, false, "if x > 0 then".into()),
                (2, false, "x = 1 ;".into()),
                (1, false, "else".into()),
                (2, false, "x = 2 ;".into()),
                (1, false, "end if ;".into()),
                (0, false, "annotation ( a = 1 ) ;".into()),
                (0, true, "end M ;".into()),
            ]
        );
    }

    #[test]
    fn short_class_and_nested() {
        let s = shape("package P\n  type T = Real;\n  partial model Q\n  end Q;\nend P;");
        assert_eq!(s[1], (1, true, "type T = Real ;".into()));
        assert_eq!(s[2], (1, false, "partial model Q".into()));
        assert_eq!(s[3], (1, true, "end Q ;".into()));
        assert_eq!(s[4], (0, true, "end P ;".into()));
    }

    #[test]
    fn errors() {
        for bad in [
            "model M\n  Real x\nend M;",
            "model M\nend N;",
            "model M\n  x = (1;\nend M;",
            "model M\n  Real x;",
            // S192: 開いた if と end for の不一致、種類の違う end
            "model M\n  Real x;\nequation\n  if true then\n    x = 1;\n  end for;\nend M;",
            "model M\n  Real x;\nequation\n  for i in 1:2 loop\n    x = 1;\n  end if;\nend M;",
        ] {
            assert!(split(&lex(bad).unwrap(), true).is_err(), "{bad}");
        }
    }
}

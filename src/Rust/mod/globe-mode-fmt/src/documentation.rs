//! クラスの `annotation(Documentation(info = "…", revisions = "…"))` の生の HTML を取り出す（S232）。
//! 字句解析（[`lex`]）と文の切り出し（[`class_spans`]）だけで読み、Rumoca の構文解析はしない。HTML は整えも検証もしない
//! （表示する側が信用せずに洗う）。文字列は Modelica のエスケープ（`\"`・`\\`・`\n` など）を戻し、`"a" + "b"` はつなぐ。

use crate::lexer::{Kind, Token, lex};
use crate::structure::{class_spans, skip_group};

/// クラスの Documentation。無い項目は空。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documentation {
    pub info: String,
    pub revisions: String,
}

/// `src` で `qualified`（`within` を含む完全修飾名）のクラスの Documentation。クラスが無ければ None（あって Documentation が無ければ空）。
pub fn documentation(src: &str, qualified: &str) -> Option<Documentation> {
    let tokens = lex(src.strip_prefix('\u{feff}').unwrap_or(src)).ok()?;
    let within = within(&tokens);
    let span = class_spans(&tokens)
        .ok()?
        .into_iter()
        .find(|s| join(&within, &s.path) == qualified)?;
    let doc = span.annotation.and_then(documentation_args);
    let arg = |name| doc.map_or(String::new(), |d| string_arg(d, name));
    Some(Documentation {
        info: arg("info"),
        revisions: arg("revisions"),
    })
}

/// `offset`（バイト位置）を含む、いちばん内側のクラスの完全修飾名。
pub fn class_at(src: &str, offset: usize) -> Option<String> {
    let (body, offset) = match src.strip_prefix('\u{feff}') {
        Some(b) => (b, offset.saturating_sub('\u{feff}'.len_utf8())),
        None => (src, offset),
    };
    let tokens = lex(body).ok()?;
    let within = within(&tokens);
    let spans = class_spans(&tokens).ok()?;
    let s = spans
        .iter()
        .filter(|s| s.start <= offset && offset < s.end)
        .max_by_key(|s| s.start)?;
    Some(join(&within, &s.path))
}

pub(crate) fn join(within: &str, path: &str) -> String {
    if within.is_empty() {
        path.to_string()
    } else {
        format!("{within}.{path}")
    }
}

/// 先頭の `within A.B;`（無ければ空）。
pub(crate) fn within(tokens: &[Token<'_>]) -> String {
    if !tokens.first().is_some_and(|t| t.is("within")) {
        return String::new();
    }
    tokens[1..]
        .iter()
        .take_while(|t| !t.is(";"))
        .map(|t| t.text)
        .collect()
}

/// `annotation(…)` の直下の `Documentation(…)` の括弧から閉じ括弧までのトークン列。
fn documentation_args<'t, 'a>(annotation: &'t [Token<'a>]) -> Option<&'t [Token<'a>]> {
    let mut depth = 0;
    for (i, t) in annotation.iter().enumerate() {
        depth += i32::from(t.is("(")) - i32::from(t.is(")"));
        if depth == 1 && t.is("Documentation") && annotation.get(i + 1).is_some_and(|s| s.is("(")) {
            return Some(&annotation[i + 1..skip_group(annotation, i + 1).ok()?]);
        }
    }
    None
}

/// `(info = "a" + "b", revisions = "c")` から `name` の値（無ければ空）。
fn string_arg(args: &[Token<'_>], name: &str) -> String {
    let mut depth = 0;
    for (i, t) in args.iter().enumerate() {
        depth += i32::from(t.is("(") || t.is("[") || t.is("{"))
            - i32::from(t.is(")") || t.is("]") || t.is("}"));
        if depth == 1 && t.is(name) && args.get(i + 1).is_some_and(|s| s.is("=")) {
            let mut out = String::new();
            let mut j = i + 2;
            while let Some(s) = args.get(j).filter(|s| s.kind == Kind::Str) {
                out.push_str(&unescape(s.text));
                j += if args.get(j + 1).is_some_and(|p| p.is("+")) {
                    2
                } else {
                    break;
                };
            }
            return out;
        }
    }
    String::new()
}

/// 引用符つきの文字列の字句を、エスケープを戻した本文にする。
pub(crate) fn unescape(token: &str) -> String {
    let body = token
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(token);
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('a') => out.push('\u{7}'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('v') => out.push('\u{b}'),
            Some(e @ ('"' | '\'' | '?' | '\\')) => out.push(e),
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"within P.Q;
package Pkg "p"
  model M "m"
    Real x annotation(Documentation(info = "no"));
  equation
    der(x) = 1;
    annotation(Documentation(info = "<p>Say \"hi\" \\ <b>there</b></p>\n" + "<img src=\"modelica://P/a.png\">", revisions = "<ul><li>v1</li></ul>"), Icon(graphics = {Text(textString = "Documentation")}));
  end M;
  model Bare "b"
  end Bare;
  type T = Real annotation(Documentation(info = "t"));
  annotation(Documentation(info = "pkg", __OpenModelica_infoHeader = "h"));
end Pkg;
"#;

    #[test]
    fn info_and_revisions_are_unescaped() {
        let d = documentation(SRC, "P.Q.Pkg.M").unwrap();
        assert_eq!(
            d.info,
            "<p>Say \"hi\" \\ <b>there</b></p>\n<img src=\"modelica://P/a.png\">"
        );
        assert_eq!(d.revisions, "<ul><li>v1</li></ul>");
    }

    #[test]
    fn each_class_has_its_own_and_a_missing_one_is_empty_or_none() {
        let info = |c| documentation(SRC, c).map(|d| d.info);
        assert_eq!(info("P.Q.Pkg"), Some("pkg".into()));
        assert_eq!(info("P.Q.Pkg.T"), Some("t".into()));
        assert_eq!(info("P.Q.Pkg.Bare"), Some(String::new()));
        assert_eq!(info("P.Q.Pkg.Nope"), None);
        assert_eq!(info("Pkg.M"), None, "within を含む名前で引く");
        assert_eq!(documentation("model M", "M"), None, "壊れた構文");
    }

    #[test]
    fn the_class_at_an_offset_is_the_innermost() {
        let at = |needle: &str| class_at(SRC, SRC.find(needle).unwrap());
        assert_eq!(at("model M").as_deref(), Some("P.Q.Pkg.M"));
        assert_eq!(at("der(x)").as_deref(), Some("P.Q.Pkg.M"));
        assert_eq!(at("model Bare").as_deref(), Some("P.Q.Pkg.Bare"));
        assert_eq!(at("package Pkg").as_deref(), Some("P.Q.Pkg"));
        assert_eq!(at("within"), None);
        assert_eq!(
            class_at(&format!("\u{feff}{SRC}"), 3 + SRC.find("model M").unwrap()).as_deref(),
            Some("P.Q.Pkg.M")
        );
    }
}

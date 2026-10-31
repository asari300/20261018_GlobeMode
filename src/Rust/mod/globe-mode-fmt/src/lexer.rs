//! 字句解析。コメントもトークンとして残し、直前の空白に含まれる改行の数だけを覚える。

use crate::{FormatError, err};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ident,
    Number,
    Str,
    LineComment,
    BlockComment,
    Punct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: Kind,
    pub text: &'a str,
    /// 入力のバイト位置。合成したトークンは `usize::MAX`
    pub start: usize,
    /// 直前の空白に含まれていた改行の数（2 以上なら空行があった）
    pub newlines_before: usize,
}

impl Token<'_> {
    pub fn is(&self, text: &str) -> bool {
        self.text == text && matches!(self.kind, Kind::Ident | Kind::Punct)
    }
    pub fn is_comment(&self) -> bool {
        matches!(self.kind, Kind::LineComment | Kind::BlockComment)
    }
    pub fn is_keyword(&self) -> bool {
        self.kind == Kind::Ident
            && crate::KEYWORDS.contains(&self.text)
            && !self.text.starts_with(|c: char| c.is_ascii_uppercase())
    }
    /// 合成したトークン（`origin = {0, 0}` の補完に使う）
    pub fn synthetic(kind: Kind, text: &'static str) -> Token<'static> {
        Token {
            kind,
            text,
            start: usize::MAX,
            newlines_before: 0,
        }
    }
}

const TWO_CHAR: &[&str] = &[":=", "<=", ">=", "==", "<>", ".+", ".-", ".*", "./", ".^"];
const ONE_CHAR: &str = "()[]{};,:.=+-*/^<>";

pub fn lex(src: &str) -> Result<Vec<Token<'_>>, FormatError> {
    let b = src.as_bytes();
    let n = b.len();
    let mut i = 0;
    let mut newlines = 0;
    let mut out = Vec::new();
    while i < n {
        let c = b[i];
        if c.is_ascii_whitespace() {
            if c == b'\n' {
                newlines += 1;
            }
            i += 1;
            continue;
        }
        let start = i;
        let kind = if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            if b[i - 1] == b'\r' {
                i -= 1;
            }
            Kind::LineComment
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            match src[i + 2..].find("*/") {
                Some(p) => i = i + 2 + p + 2,
                None => return Err(err("閉じていないブロックコメント", start)),
            }
            Kind::BlockComment
        } else if c == b'"' || c == b'\'' {
            i += 1;
            loop {
                if i >= n {
                    return Err(err("閉じていない文字列", start));
                }
                if b[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if b[i] == c {
                    i += 1;
                    break;
                }
                i += 1;
            }
            if c == b'"' { Kind::Str } else { Kind::Ident }
        } else if c.is_ascii_digit() {
            while i < n && b[i].is_ascii_digit() {
                i += 1;
            }
            if i < n && b[i] == b'.' {
                i += 1;
                while i < n && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < n && (b[i] == b'e' || b[i] == b'E') {
                let mut j = i + 1;
                if j < n && (b[j] == b'+' || b[j] == b'-') {
                    j += 1;
                }
                if j < n && b[j].is_ascii_digit() {
                    i = j;
                    while i < n && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            Kind::Number
        } else if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] >= 0x80) {
                i += 1;
            }
            Kind::Ident
        } else if i + 1 < n && TWO_CHAR.contains(&&src[i..i + 2]) {
            i += 2;
            Kind::Punct
        } else if ONE_CHAR.as_bytes().contains(&c) {
            i += 1;
            Kind::Punct
        } else {
            return Err(err(
                format!("読めない文字 {:?}", src[i..].chars().next().unwrap_or('?')),
                start,
            ));
        };
        out.push(Token {
            kind,
            text: &src[start..i],
            start,
            newlines_before: newlines,
        });
        newlines = 0;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_tokens_and_blank_lines() {
        let t = lex("model M \"d\"\n\n  Real x(start=-1.5e-3); // c\n  /* b */ x := 'q';\nend M;")
            .unwrap();
        let texts: Vec<_> = t.iter().map(|t| t.text).collect();
        assert_eq!(
            texts,
            [
                "model", "M", "\"d\"", "Real", "x", "(", "start", "=", "-", "1.5e-3", ")", ";",
                "// c", "/* b */", "x", ":=", "'q'", ";", "end", "M", ";"
            ]
        );
        assert_eq!(t[3].newlines_before, 2);
        assert_eq!(t[9].kind, Kind::Number);
        assert_eq!(t[16].kind, Kind::Ident);
        assert!(lex("\"open").is_err());
        assert!(lex("/* open").is_err());
        assert!(lex("a # b").is_err());
    }
}

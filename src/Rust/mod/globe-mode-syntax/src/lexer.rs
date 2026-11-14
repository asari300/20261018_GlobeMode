//! Modelica 3.6 の字句解析。全バイトをどれかのトークンに入れる（空白・コメント・誤りも）ので、
//! トークンの文字列をつなぐと元の文字列に戻る。

use crate::kind::{KEYWORDS, SyntaxKind, SyntaxKind::*};

/// 字句 1 つ（`len` はバイト数）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: usize,
}

pub fn lex(text: &str) -> Vec<Token> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let (kind, end) = one(text, b, i);
        debug_assert!(end > i && text.is_char_boundary(end));
        out.push(Token { kind, len: end - i });
        i = end;
    }
    out
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn one(text: &str, b: &[u8], i: usize) -> (SyntaxKind, usize) {
    let c = b[i];
    let at = |j: usize| b.get(j).copied();
    let digits = |mut j: usize| {
        while at(j).is_some_and(|d| d.is_ascii_digit()) {
            j += 1;
        }
        j
    };
    match c {
        b' ' | b'\t' | b'\r' | b'\n' => {
            let mut j = i;
            while at(j).is_some_and(|d| matches!(d, b' ' | b'\t' | b'\r' | b'\n')) {
                j += 1;
            }
            (Whitespace, j)
        }
        // BOM
        0xEF if text[i..].starts_with('\u{feff}') => (Whitespace, i + 3),
        b'/' if at(i + 1) == Some(b'/') => {
            let j = text[i..].find('\n').map_or(b.len(), |n| i + n);
            (LineComment, j)
        }
        b'/' if at(i + 1) == Some(b'*') => {
            let j = text[i + 2..].find("*/").map_or(b.len(), |n| i + 2 + n + 2);
            (BlockComment, j)
        }
        b'"' => {
            let mut j = i + 1;
            while j < b.len() {
                match b[j] {
                    b'\\' => j += 2,
                    b'"' => {
                        j += 1;
                        break;
                    }
                    _ => j += 1,
                }
            }
            (Str, next_boundary(text, j.min(b.len())))
        }
        b'\'' => {
            let mut j = i + 1;
            while j < b.len() {
                match b[j] {
                    b'\\' => j += 2,
                    b'\'' => {
                        j += 1;
                        break;
                    }
                    _ => j += 1,
                }
            }
            (Ident, next_boundary(text, j.min(b.len())))
        }
        _ if is_ident_start(c) => {
            let mut j = i + 1;
            while at(j).is_some_and(|d| is_ident_start(d) || d.is_ascii_digit()) {
                j += 1;
            }
            let word = &text[i..j];
            let kind = KEYWORDS
                .iter()
                .find(|(k, _)| *k == word)
                .map_or(Ident, |(_, k)| *k);
            (kind, j)
        }
        _ if c.is_ascii_digit() => {
            let mut j = digits(i);
            let mut real = false;
            if at(j) == Some(b'.') {
                real = true;
                j = digits(j + 1);
            }
            if matches!(at(j), Some(b'e' | b'E')) {
                let k = if matches!(at(j + 1), Some(b'+' | b'-')) {
                    j + 2
                } else {
                    j + 1
                };
                if at(k).is_some_and(|d| d.is_ascii_digit()) {
                    real = true;
                    j = digits(k);
                }
            }
            (if real { UReal } else { UInt }, j)
        }
        b'.' => match at(i + 1) {
            Some(b'+') => (DotPlus, i + 2),
            Some(b'-') => (DotMinus, i + 2),
            Some(b'*') => (DotStar, i + 2),
            Some(b'/') => (DotSlash, i + 2),
            Some(b'^') => (DotCaret, i + 2),
            _ => (Dot, i + 1),
        },
        b'=' if at(i + 1) == Some(b'=') => (EqEq, i + 2),
        b'<' if at(i + 1) == Some(b'>') => (Ne, i + 2),
        b'<' if at(i + 1) == Some(b'=') => (Le, i + 2),
        b'>' if at(i + 1) == Some(b'=') => (Ge, i + 2),
        b':' if at(i + 1) == Some(b'=') => (Assign, i + 2),
        b'(' => (LParen, i + 1),
        b')' => (RParen, i + 1),
        b'[' => (LBracket, i + 1),
        b']' => (RBracket, i + 1),
        b'{' => (LBrace, i + 1),
        b'}' => (RBrace, i + 1),
        b',' => (Comma, i + 1),
        b';' => (Semi, i + 1),
        b':' => (Colon, i + 1),
        b'=' => (Eq, i + 1),
        b'<' => (Lt, i + 1),
        b'>' => (Gt, i + 1),
        b'+' => (Plus, i + 1),
        b'-' => (Minus, i + 1),
        b'*' => (Star, i + 1),
        b'/' => (Slash, i + 1),
        b'^' => (Caret, i + 1),
        _ => {
            // 知らない文字は 1 文字（UTF-8 の境目まで）を誤りのトークンにする
            let n = text[i..].chars().next().map_or(1, char::len_utf8);
            (ErrorToken, i + n)
        }
    }
}

/// `\` の後ろを読み飛ばして文字の途中で終わったときに、次の文字境界まで進める。
fn next_boundary(text: &str, mut j: usize) -> usize {
    while j < text.len() && !text.is_char_boundary(j) {
        j += 1;
    }
    j.min(text.len())
}

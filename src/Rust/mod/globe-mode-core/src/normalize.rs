//! ソース本文の正規化（防火壁 0、ADR-0005）。
//!
//! コメント（`//` と `/* */`）を除き、空白と改行を 1 個の空白に畳む。文字列 `"…"` と
//! 引用識別子 `'…'` の中は触らない。空白やコメントだけの編集では結果が前と等しくなるので、
//! salsa がコンパイル（`check` / `flat`）を再実行しない。
//!
//! - コメントは空白と同じく「区切り」として扱う（`a/*c*/b` は `a b` になる。`ab` にはしない）。
//! - 先頭と末尾の区切りは落とす。
//! - 閉じていないブロックコメント・文字列・引用識別子は、そこから末尾までをそのまま残す
//!   （構文の誤りを、コンパイラが元の本文と同じように見つけられるように）。
//! - 走査はバイト単位。区切りに使う文字はすべて ASCII なので、UTF-8 の多バイト文字の途中で
//!   切ることはない。

/// 正規化した本文と、位置の対応表。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SourceShape {
    /// 正規化した本文
    pub text: String,
    /// `(正規化後のバイト位置, 元の本文のバイト位置)`。トークンの先頭ごとに 1 項目、昇順
    pub map: Vec<(u32, u32)>,
}

impl SourceShape {
    /// 正規化後の位置を、元の本文の位置に戻す。
    pub fn to_original(&self, offset: u32) -> u32 {
        to_original(&self.map, offset)
    }
}

/// [`SourceShape::map`] だけを使って、正規化後の位置を元の本文の位置に戻す
/// （salsa のクエリで対応表だけを別に持つとき用）。
pub fn to_original(map: &[(u32, u32)], offset: u32) -> u32 {
    match map.binary_search_by_key(&offset, |(n, _)| *n) {
        Ok(i) => map[i].1,
        Err(0) => offset,
        Err(i) => {
            let (n, o) = map[i - 1];
            o + (offset - n)
        }
    }
}

/// 本文を正規化する。
pub fn source_shape(text: &str) -> SourceShape {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut map: Vec<(u32, u32)> = Vec::new();
    // 直前に写したバイトの、元の本文での直後の位置（区切りを挟まずに続くなら、ここから写す）
    let mut next_orig: Option<usize> = None;
    // 区切り（空白・コメント）を読んだが、まだ空白を出していない
    let mut pending_space = false;

    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // 区切り: 空白
        if is_space(b) {
            pending_space = true;
            i += 1;
            continue;
        }
        // 区切り: 行コメント
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            pending_space = true;
            i = bytes[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(bytes.len(), |p| i + p + 1);
            continue;
        }
        // 区切り: ブロックコメント（閉じていなければ、末尾までをそのまま写す）
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            if let Some(p) = find(&bytes[i + 2..], b"*/") {
                pending_space = true;
                i = i + 2 + p + 2;
                continue;
            }
            emit(
                text,
                i,
                bytes.len(),
                &mut out,
                &mut map,
                &mut next_orig,
                &mut pending_space,
            );
            break;
        }
        // トークン: 文字列と引用識別子は中を触らずに写す
        let end = if b == b'"' || b == b'\'' {
            quoted_end(bytes, i, b)
        } else {
            // ふつうのトークン: 次の区切り・文字列・引用識別子の手前まで
            let mut j = i + 1;
            while j < bytes.len() {
                let c = bytes[j];
                if is_space(c)
                    || c == b'"'
                    || c == b'\''
                    || (c == b'/' && matches!(bytes.get(j + 1), Some(b'/' | b'*')))
                {
                    break;
                }
                j += 1;
            }
            j
        };
        emit(
            text,
            i,
            end,
            &mut out,
            &mut map,
            &mut next_orig,
            &mut pending_space,
        );
        i = end;
    }
    SourceShape { text: out, map }
}

/// `text[start..end]` を写す。区切りが溜まっていれば、先に空白を 1 個出す。
fn emit(
    text: &str,
    start: usize,
    end: usize,
    out: &mut String,
    map: &mut Vec<(u32, u32)>,
    next_orig: &mut Option<usize>,
    pending_space: &mut bool,
) {
    if *pending_space && !out.is_empty() {
        // 空白は、区切りが始まった位置（＝直前に写したものの直後）に対応する
        out.push(' ');
        *next_orig = next_orig.map(|o| o + 1);
    }
    *pending_space = false;
    if *next_orig != Some(start) {
        map.push((out.len() as u32, start as u32));
    }
    out.push_str(&text[start..end]);
    *next_orig = Some(end);
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

/// `bytes[start]` の引用符（`"` か `'`）で始まる文字列の終わり（閉じる引用符の直後）。
/// `\` の後の 1 文字は読み飛ばす。閉じていなければ末尾。
fn quoted_end(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut j = start + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            c if c == quote => return j + 1,
            _ => j += 1,
        }
    }
    bytes.len()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "model A\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend A;\n";

    #[test]
    fn collapses_whitespace_and_trims() {
        let s = source_shape("  model\tA\r\n\n  Real x;\nend A;  \n");
        assert_eq!(s.text, "model A Real x; end A;");
    }

    #[test]
    fn whitespace_only_edits_give_the_same_text() {
        let edited = "model A\n\n\n    Real x(start = 1);   \nequation\n\tder(x) = -x;\nend A;";
        assert_eq!(source_shape(MODEL).text, source_shape(edited).text);
    }

    #[test]
    fn comment_only_edits_give_the_same_text() {
        let edited = "// 先頭のコメント\nmodel A /* ブロック\n コメント */\n  Real x(start = 1); // 行末\nequation\n  der(x) = -x;\nend A;\n// 末尾";
        assert_eq!(source_shape(MODEL).text, source_shape(edited).text);
    }

    #[test]
    fn crlf_and_lf_give_the_same_text() {
        let crlf = MODEL.replace('\n', "\r\n");
        assert_eq!(source_shape(MODEL).text, source_shape(&crlf).text);
    }

    #[test]
    fn a_comment_between_tokens_still_separates_them() {
        assert_eq!(source_shape("a/*c*/b").text, "a b");
        assert_eq!(source_shape("x = 1;// c\ny = 2;").text, "x = 1; y = 2;");
    }

    #[test]
    fn strings_and_quoted_identifiers_are_kept_verbatim() {
        let text = "Real x \"a // not a comment  /* nor this */\";\nReal 'q // id' = 1;";
        assert_eq!(
            source_shape(text).text,
            "Real x \"a // not a comment  /* nor this */\"; Real 'q // id' = 1;"
        );
        // 改行を含む文字列と、エスケープした引用符
        let text = "String s = \"line1\n  line2 \\\" // still\";  // comment";
        assert_eq!(
            source_shape(text).text,
            "String s = \"line1\n  line2 \\\" // still\";"
        );
    }

    #[test]
    fn unterminated_block_comment_is_kept_for_the_parser() {
        let s = source_shape("model A /* never closed\nend A;");
        assert_eq!(s.text, "model A /* never closed\nend A;");
    }

    #[test]
    fn to_original_maps_back_into_the_third_token() {
        let text = "model   Foo\n\n   extends Bar;";
        let s = source_shape(text);
        assert_eq!(s.text, "model Foo extends Bar;");
        // 3 番目のトークン `extends` の 2 文字目
        let n = s.text.find("extends").unwrap() as u32 + 1;
        let o = s.to_original(n) as usize;
        assert_eq!(&text[o..o + 6], "xtends");
        // 先頭と、末尾（半開区間の終わり）
        assert_eq!(s.to_original(0), 0);
        assert_eq!(s.to_original(s.text.len() as u32) as usize, text.len());
        // トークンの位置は、元の本文の同じトークンを指す
        for token in ["model", "Foo", "extends", "Bar;"] {
            let n = s.text.find(token).unwrap() as u32;
            let o = s.to_original(n) as usize;
            assert!(text[o..].starts_with(token), "{token}");
        }
    }

    #[test]
    fn non_ascii_comments_and_strings_are_safe() {
        let text =
            "// 日本語のコメント\nmodel A \"説明：日本語\" /* 注釈 */ Real x \"電位\";\nend A;";
        let s = source_shape(text);
        assert_eq!(s.text, "model A \"説明：日本語\" Real x \"電位\"; end A;");
        let n = s.text.find("Real").unwrap() as u32;
        assert!(text[s.to_original(n) as usize..].starts_with("Real x"));
    }

    #[test]
    fn empty_and_comment_only_texts() {
        assert_eq!(source_shape("").text, "");
        assert_eq!(source_shape("  // c\n /* d */ ").text, "");
        assert!(source_shape("").map.is_empty());
    }
}

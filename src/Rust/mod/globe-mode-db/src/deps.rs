//! `deps_closure`（Rumoca に渡すファイルの絞り込み）の字句の走査。
//!
//! 本文（`source_shape` で正規化したもの）から識別子を集める。クラス名との突き合わせにしか使わないので、
//! キーワードも残す（広めに見積もる）。文字列 `"…"` の中は除く（説明文や `Documentation` の HTML が
//! クラス名に当たらないように）。

use std::collections::BTreeSet;

/// 本文に現れる識別子の集合。
///
/// - ふつうの識別子 `[A-Za-z_][A-Za-z0-9_]*`
/// - 引用識別子 `'…'`: 引用符ごとの形（`'+'`）と、中身（`+`）の両方を入れる（どちらの綴りの名前とも突き合うように）
/// - 文字列 `"…"` の中は読み飛ばす。`\` の後の 1 文字も読み飛ばす
/// - 数（`1e5`、`2.5E-3`）の中の英字は識別子にしない
/// - 閉じていない文字列・引用識別子は、末尾までを読み飛ばす（引用識別子は中身を入れる）
pub(crate) fn identifiers(text: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            i = quoted_end(bytes, i);
        } else if b == b'\'' {
            let end = quoted_end(bytes, i);
            let quoted = &text[i..end];
            out.insert(quoted.to_string());
            let inner = quoted
                .strip_prefix('\'')
                .map(|s| s.strip_suffix('\'').unwrap_or(s))
                .unwrap_or(quoted);
            if !inner.is_empty() {
                out.insert(inner.to_string());
            }
            i = end;
        } else if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.insert(text[start..i].to_string());
        } else if b.is_ascii_digit() {
            // 数: 数字・英字・`_`・`.` が続く間を 1 つの字句として読み飛ばす（`1e5` の `e5` を識別子にしない）
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
            {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `bytes[start]` の引用符（`"` か `'`）で始まる字句の終わり（閉じる引用符の直後）。閉じていなければ末尾。
fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<String> {
        identifiers(text).into_iter().collect()
    }

    #[test]
    fn collects_identifiers_and_keywords() {
        assert_eq!(
            ids("within A.B; model M_1 _x.y z; end M_1;"),
            ["A", "B", "M_1", "_x", "end", "model", "within", "y", "z"]
        );
    }

    #[test]
    fn skips_strings_and_escapes() {
        assert_eq!(
            ids(r#"model M "説明 Resistor \" Capacitor" Real x(unit = "V"); end M;"#),
            ["M", "Real", "end", "model", "unit", "x"]
        );
    }

    #[test]
    fn quoted_identifiers_keep_both_spellings() {
        assert_eq!(
            ids("operator '+' 'a b'.c"),
            ["'+'", "'a b'", "+", "a b", "c", "operator"]
        );
    }

    #[test]
    fn numbers_do_not_produce_identifiers() {
        assert_eq!(ids("x = 1e5 + 2.5E-3 * y[1].z;"), ["x", "y", "z"]);
    }

    #[test]
    fn unclosed_quotes_run_to_the_end() {
        assert_eq!(ids(r#"model M "unclosed Resistor"#), ["M", "model"]);
        assert_eq!(ids("model M 'q x"), ["'q x", "M", "model", "q x"]);
    }

    #[test]
    fn multibyte_text_outside_strings_is_skipped() {
        assert_eq!(ids("model 抵抗 R; end"), ["R", "end", "model"]);
    }
}

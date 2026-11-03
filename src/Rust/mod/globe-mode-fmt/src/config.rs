//! 設定ファイル `globe-mode.toml`（S130、ruff の `ruff.toml` に倣う）。feature `config` のときだけ。
//!
//! ```toml
//! [fmt]
//! indent = 2
//! width = 80
//! insert_origin = true
//! blank_around_sections = true
//! format_html = true
//!
//! [lint]
//! disable = ["missing-description", "GM-L0004"]
//! ```

use crate::{LintOptions, RULES};
use std::path::Path;

pub const CONFIG_FILE: &str = "globe-mode.toml";

impl LintOptions {
    /// 設定ファイルの本文を読む。書いていないキーは既定。知らない表・キー・規則、型の違う値は `Err`。
    /// `disable` は規則の名前（`missing-description`）でもコード（`GM-L0003`）でもよく、名前にそろえる。
    pub fn from_toml(src: &str) -> Result<LintOptions, String> {
        let table: toml::Table = src.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let mut o = LintOptions::default();
        for (section, value) in &table {
            let keys = value
                .as_table()
                .filter(|_| section == "fmt" || section == "lint")
                .ok_or_else(|| format!("知らない表 [{section}]（[fmt] と [lint] だけ）"))?;
            for (key, v) in keys {
                let at = format!("{section}.{key}");
                let uint = || {
                    v.as_integer()
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or_else(|| format!("{at} は 0 以上の整数"))
                };
                let boolean = || v.as_bool().ok_or_else(|| format!("{at} は true か false"));
                match (section.as_str(), key.as_str()) {
                    ("fmt", "indent") => o.format.indent = uint()?,
                    ("fmt", "width") => o.format.width = uint()?,
                    ("fmt", "insert_origin") => o.format.insert_origin = boolean()?,
                    ("fmt", "blank_around_sections") => o.format.blank_around_sections = boolean()?,
                    ("fmt", "format_html") => o.format.format_html = boolean()?,
                    ("lint", "disable") => {
                        let list = v.as_array().ok_or_else(|| format!("{at} は文字列の配列"))?;
                        for r in list {
                            let r = r.as_str().ok_or_else(|| format!("{at} は文字列の配列"))?;
                            let (_, name) = RULES
                                .iter()
                                .find(|(code, name)| r == *code || r == *name)
                                .ok_or_else(|| format!("{at} の {r} は知らない規則"))?;
                            o.disabled.push(name.to_string());
                        }
                    }
                    _ => return Err(format!("知らないキー {at}")),
                }
            }
        }
        Ok(o)
    }

    /// `path`（ファイルでもディレクトリでも）から上へたどって最初の `globe-mode.toml` を読む。無ければ `None`。
    pub fn find(path: &Path) -> Result<Option<LintOptions>, String> {
        let abs = std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(file) = abs
            .ancestors()
            .map(|d| d.join(CONFIG_FILE))
            .find(|f| f.is_file())
        else {
            return Ok(None);
        };
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        LintOptions::from_toml(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", file.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_toml_reads_fmt_and_lint() {
        let o = LintOptions::from_toml(
            "[fmt]\nindent = 4\ninsert_origin = false\n[lint]\ndisable = [\"GM-L0003\", \"line-too-long\"]\n",
        )
        .unwrap();
        assert_eq!(o.format.indent, 4);
        assert_eq!(o.format.width, 80);
        assert!(!o.format.insert_origin && o.format.blank_around_sections);
        assert_eq!(o.disabled, ["missing-description", "line-too-long"]);
        assert_eq!(LintOptions::from_toml("").unwrap(), LintOptions::default());
    }

    #[test]
    fn from_toml_rejects_unknown_keys_and_bad_values() {
        for (src, want) in [
            ("[fmt]\nindnt = 4\n", "知らないキー fmt.indnt"),
            ("[format]\nindent = 4\n", "知らない表 [format]"),
            ("[fmt]\nindent = -1\n", "fmt.indent は 0 以上の整数"),
            ("[fmt]\nwidth = \"80\"\n", "fmt.width は 0 以上の整数"),
            (
                "[lint]\ndisable = [\"GM-L9999\"]\n",
                "GM-L9999 は知らない規則",
            ),
            ("[fmt\n", ""),
        ] {
            let e = LintOptions::from_toml(src).unwrap_err();
            assert!(e.contains(want), "{src:?}: {e}");
        }
    }
}

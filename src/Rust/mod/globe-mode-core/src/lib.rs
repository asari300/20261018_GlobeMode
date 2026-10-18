//! GlobeMode の共通型: ソース上の範囲、診断、シミュレーション前の検査段階。
//!
//! どのフロントエンド（Phase A の Rumoca、Track B の自前コア）からも同じ形で
//! 診断を返すため、ここには外部の crate の型を出さない。

use serde::Serialize;

/// ソース中のバイト範囲（半開区間 `start..end`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

impl TextRange {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// 1 始まりの行と列（列は文字単位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct LineCol {
    pub line: u32,
    pub column: u32,
}

/// バイト位置を行と列に直す。範囲外なら末尾として扱う。
pub fn line_col(text: &str, offset: u32) -> LineCol {
    let offset = (offset as usize).min(text.len());
    let before = &text[..floor_char_boundary(text, offset)];
    let line = before.matches('\n').count() as u32 + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
    LineCol { line, column }
}

fn floor_char_boundary(text: &str, mut i: usize) -> usize {
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 診断の重さ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// シミュレーション前の検査段階（doc/architecture.md §3.2）。
///
/// 段階の順に検査し、Error が出た段階で止める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Stage {
    /// 構文
    S0,
    /// 名前解決
    S1,
    /// 型と次元
    S2,
    /// instantiate
    S3,
    /// 接続
    S4,
    /// 方程式数と変数数
    S5,
    /// 構造特異
    S6,
    /// インデックス
    S7,
    /// 初期化
    S8,
    /// 代数ループ
    S9,
    /// プリフライト
    S10,
}

impl Stage {
    /// 段階の短い説明。
    pub const fn title(self) -> &'static str {
        match self {
            Stage::S0 => "構文",
            Stage::S1 => "名前解決",
            Stage::S2 => "型と次元",
            Stage::S3 => "instantiate",
            Stage::S4 => "接続",
            Stage::S5 => "方程式数と変数数",
            Stage::S6 => "構造特異",
            Stage::S7 => "インデックス",
            Stage::S8 => "初期化",
            Stage::S9 => "代数ループ",
            Stage::S10 => "プリフライト",
        }
    }
}

/// 診断。コードは `GM-E…`（誤り）/ `GM-W…`（警告）/ `GM-I…`（情報）/ `GM-U…`（コンパイラが未対応）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub stage: Stage,
    pub message: String,
    /// 対象のファイル名（分かるときだけ）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// 対象の範囲（分かるときだけ）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
    /// 補足。フロントエンド固有のコード（例: `rumoca: EI008`）もここに入れる。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, stage: Stage, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            stage,
            message: message.into(),
            file: None,
            range: None,
            notes: Vec::new(),
        }
    }

    pub fn at(mut self, file: impl Into<String>, range: Option<TextRange>) -> Self {
        self.file = Some(file.into());
        self.range = range;
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// 診断コード。番号の 1 桁目（百の位）が検査段階に対応する。
pub mod codes {
    /// S0: 構文の誤り
    pub const SYNTAX: &str = "GM-E0001";
    /// S1: 名前解決・その他のフロントエンドの失敗（段階を特定できないもの）
    pub const RESOLVE: &str = "GM-E1001";
    /// S1: 指定したモデルが見つからない
    pub const MODEL_NOT_FOUND: &str = "GM-E1002";
    /// S2: 型検査の誤り
    pub const TYPECHECK: &str = "GM-E2001";
    /// S3: instantiate の誤り
    pub const INSTANTIATE: &str = "GM-E3001";
    /// S3: outer に対応する inner が無い
    pub const MISSING_INNER: &str = "GM-E3002";
    /// S4: flatten（接続の展開を含む）の誤り
    pub const FLATTEN: &str = "GM-E4001";
    /// S5: DAE への変換の誤り
    pub const TO_DAE: &str = "GM-E5001";
    /// S5: 方程式数と変数数が合わない
    pub const UNBALANCED: &str = "GM-E5002";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_counts_lines_and_chars() {
        let text = "model A\n  Real x;\nend A;";
        assert_eq!(line_col(text, 0), LineCol { line: 1, column: 1 });
        assert_eq!(line_col(text, 10), LineCol { line: 2, column: 3 });
        assert_eq!(line_col(text, 999).line, 3);
    }

    #[test]
    fn line_col_is_safe_inside_multibyte_chars() {
        let text = "// 日本語\nmodel A end A;";
        // 「日」の途中を指しても panic しない
        assert_eq!(line_col(text, 4).line, 1);
        assert_eq!(
            line_col(text, text.find("model").unwrap() as u32),
            LineCol { line: 2, column: 1 }
        );
    }

    #[test]
    fn stages_are_ordered() {
        assert!(Stage::S0 < Stage::S5);
        assert!(Stage::S9 < Stage::S10);
    }
}

//! GlobeMode の共通型: ソース上の範囲、診断、シミュレーション前の検査段階。
//!
//! どのフロントエンド（Phase A の Rumoca、Track B の自前コア）からも同じ形で
//! 診断を返すため、ここには外部の crate の型を出さない。

use serde::Serialize;

pub mod normalize;

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
    /// 実行（S0〜S10 を通った後のシミュレーション。S011、ADR-0013）。検査段階ではないが、
    /// 実行の失敗も同じ診断の形で返すので段階として持つ
    Run,
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
            Stage::Run => "実行",
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

    pub fn warning(code: &'static str, stage: Stage, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, stage, message)
        }
    }

    pub fn info(code: &'static str, stage: Stage, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            ..Self::error(code, stage, message)
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

    /// 「コンパイラが未対応」（`GM-U…`）。モデルの誤りと分けて表示する。実行は Error と同じく拒む。
    pub fn is_unsupported(&self) -> bool {
        self.code.starts_with("GM-U")
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
    /// S1: ライブラリ（MSL など）を読み込めない
    pub const LIBRARY: &str = "GM-E1003";
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
    /// S6: 構造特異（`x` と `der(x)` をまとめても、方程式と未知数の完全マッチングが無い）
    pub const SINGULAR: &str = "GM-E6001";
    /// S7: 微分インデックスが 1 より大きい見込み（der の形では完全マッチングが無い。情報）
    pub const HIGH_INDEX: &str = "GM-I7001";
    /// S7: 状態選択が要る見込み（Pantelides 法で階数が上がった状態。dummy derivative の候補。情報）
    pub const STATE_SELECTION: &str = "GM-I7002";
    /// S7: インデックス低減（Pantelides 法）が終わらない（構造特異、または回数の上限）
    pub const INDEX_REDUCTION_FAILED: &str = "GM-E7001";
    /// S8: 初期条件が多すぎる（初期方程式と fixed = true の状態が、状態と fixed = false のパラメータより多い）
    pub const INIT_OVERDETERMINED: &str = "GM-E8001";
    /// S8: 初期条件が足りない（不足分は start 値を推定値として使う見込み。警告）
    pub const INIT_UNDERDETERMINED: &str = "GM-W8001";
    /// S8: start 値の無い状態変数（初期条件が足りないときだけ。警告）
    pub const STATE_WITHOUT_START: &str = "GM-W8002";
    /// S8: 値（束縛）の無いパラメータ（警告）
    pub const PARAMETER_WITHOUT_VALUE: &str = "GM-W8003";
    /// S8: 初期条件が多い見込みだが、離散変数を含むため確定できない（警告。MLS 8.6 の暗黙の初期方程式で数が揺れる）
    pub const INIT_OVERDETERMINED_UNCERTAIN: &str = "GM-W8004";
    /// S9: 代数ループがある（BLT の大きさ 2 以上のブロック。数と大きさ、線形・非線形の見込み。情報。S010）
    pub const ALGEBRAIC_LOOP: &str = "GM-I9001";
    /// S9: 非線形の代数ループの反復変数（tearing 変数）に start 値が無い（既定の 0 から反復する。警告。S010）
    pub const LOOP_ITERATION_WITHOUT_START: &str = "GM-W9001";
    /// S10: パラメータの値が有限にならない（0 除算、NaN、範囲外の添字など。S010）
    pub const PARAMETER_NOT_FINITE: &str = "GM-E10001";
    /// S10: パラメータの評価で assert が失敗した（S010）
    pub const PARAMETER_ASSERTION: &str = "GM-E10002";
    /// S10: start 値が有限にならない（S010）
    pub const START_NOT_FINITE: &str = "GM-E10003";
    /// S10: experiment annotation が妥当でない（StopTime ≤ StartTime、Tolerance ≤ 0、Interval ≤ 0 など。S010）
    pub const EXPERIMENT_INVALID: &str = "GM-E10004";
    /// S10: experiment annotation の値が疑わしい（Interval が区間より長い、Tolerance が大きすぎるなど。警告。S010）
    pub const EXPERIMENT_SUSPICIOUS: &str = "GM-W10001";
    /// S10: experiment annotation が無い（既定の区間で実行する見込み。情報。S010）
    pub const EXPERIMENT_MISSING: &str = "GM-I10001";
    /// S10: 静的に評価できないパラメータ（外部関数・外部オブジェクトなど。実行時に決まる。情報。S010）
    pub const PARAMETER_NOT_STATIC: &str = "GM-I10002";
    /// S10: t0 で式の値が有限にならない（start 値で評価した代数変数・導関数が NaN / inf。実行の直前の数値の検査。S011）
    pub const T0_NOT_FINITE: &str = "GM-E10005";
    /// S10: 指定したソルバが使えない（名前が無い、またはまだ実装していない。S011）
    pub const SOLVER_UNAVAILABLE: &str = "GM-E10006";
    /// S10: ソルバの別名を置き換えた（例: dassl → bdf。同等の手法で、結果は一致しない。情報。S011）
    pub const SOLVER_ALIAS: &str = "GM-I10003";
    /// 実行: 初期化に失敗した（初期化系が解けない。S011）
    pub const RUN_INIT_FAILED: &str = "GM-E11001";
    /// 実行: 積分に失敗した（刻みが小さくなりすぎた、Newton が収束しないなど。S011）
    pub const RUN_INTEGRATION_FAILED: &str = "GM-E11002";
    /// 実行: イベントの反復が収束しない（S011）
    pub const RUN_EVENT_FAILED: &str = "GM-E11003";
    /// 実行: モデルの assert が失敗して止まった（S011）
    pub const RUN_ASSERTION: &str = "GM-E11004";
    /// 実行: 時間切れ（壁時計の予算を超えた。S011）
    pub const RUN_TIMEOUT: &str = "GM-E11005";
    /// 実行: 結果に NaN / inf が出た（S011）
    pub const RUN_NOT_FINITE: &str = "GM-E11006";
    /// 実行: terminate() で終了時刻の前に終わった（情報。S011）
    pub const RUN_TERMINATED: &str = "GM-I11001";
    /// コンパイラが未対応の言語機能（フロントエンドのコードで決まる。例: Rumoca の ED019）
    pub const UNSUPPORTED: &str = "GM-U0001";
    /// コンパイラの内部エラー（例: Rumoca の ED008）
    pub const COMPILER_BUG: &str = "GM-U0002";
    /// 既知の制約の一覧（src/Modelica/msl-known-unsupported.txt）に当たった
    pub const KNOWN_LIMITATION: &str = "GM-U0003";
    /// 実行系が未対応（Rumoca の実行系の lowering・組み立てで止まった。S011）
    pub const RUN_UNSUPPORTED: &str = "GM-U0004";
    /// 実行系の内部エラー（panic、Rumoca の実行時の契約違反。S011）
    pub const RUN_INTERNAL: &str = "GM-U0005";
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

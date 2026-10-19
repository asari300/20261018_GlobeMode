//! プリフライト（S10）の入力。`check` が DAE から取り出す（S010）。
//!
//! 値（f64）は `Eq` にならないので、評価できたものは数だけを持ち、失敗したものだけを名前と位置付きで持つ。
//! experiment annotation の値は `f64::to_bits` で持つ（[`Bits`]）。

use globe_mode_core::TextRange;
use serde::Serialize;

/// プリフライトの入力（S10）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct PreflightStructure {
    /// 評価したパラメータのスカラーの数（失敗したものを含む）
    pub parameters_evaluated: usize,
    /// 評価に失敗したパラメータ（スカラー）
    pub parameter_failures: Vec<EvaluationFailure>,
    /// 評価した start 値のスカラーの数（失敗したものを含む）
    pub starts_evaluated: usize,
    /// 評価に失敗した start 値（スカラー）
    pub start_failures: Vec<EvaluationFailure>,
    /// モデルの experiment annotation。無ければ None
    pub experiment: Option<Experiment>,
}

impl PreflightStructure {
    /// 位置をすべて落とした写し（`preflight_shape` クエリの防火壁）。
    pub fn without_ranges(&self) -> PreflightStructure {
        let strip = |f: &EvaluationFailure| EvaluationFailure {
            range: None,
            ..f.clone()
        };
        PreflightStructure {
            parameter_failures: self.parameter_failures.iter().map(strip).collect(),
            start_failures: self.start_failures.iter().map(strip).collect(),
            ..self.clone()
        }
    }
}

/// 評価に失敗した値 1 つ（スカラー）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct EvaluationFailure {
    /// 変数の表示名（例: `r.R`、`x[2]`）
    pub name: String,
    pub kind: FailureKind,
    /// 評価器の言葉（原文）
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
}

/// 評価の失敗の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// 値が有限にならない（0 除算、NaN、無限大）
    NotFinite,
    /// assert が失敗した
    Assertion,
    /// 外部関数・外部オブジェクトで、静的に評価できない（実行時に決まる）
    External,
    /// 値（束縛・start）が無い
    Missing,
    /// そのほか（評価器が対応していない演算など）
    Other,
}

/// f64 をビット列で持つ（`Eq` と `Hash` のため）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Bits(pub u64);

impl Bits {
    pub fn new(value: f64) -> Self {
        Bits(value.to_bits())
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// experiment annotation（MLS 18.4）。書かれていない項目は None。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct Experiment {
    pub start_time: Option<Bits>,
    pub stop_time: Option<Bits>,
    pub interval: Option<Bits>,
    pub tolerance: Option<Bits>,
}

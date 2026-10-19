//! 参照結果との比較（矩形の管、csv-compare の既定のアルゴリズム）。

use globe_mode_ir::Trajectory;
use serde::Serialize;

/// 管の幅。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TubeOptions {
    /// 相対の許容差（時間と値の両方向。csv-compare の既定は 0.002）
    pub tolerance: f64,
}

impl Default for TubeOptions {
    fn default() -> Self {
        TubeOptions { tolerance: 0.002 }
    }
}

/// 信号 1 つの比較。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalComparison {
    pub name: String,
    /// 管の外に出た点が 0 で、信号がそろっていれば true
    pub passed: bool,
    /// 試験側にその信号が無い
    pub missing: bool,
    /// 判定した点の数（参照の区間の中にある試験側の点）
    pub checked: usize,
    /// 管の外に出た点の数
    pub errors: usize,
    /// 管からはみ出した量の最大（値の単位）
    pub max_excess: f64,
    /// 最初に管の外に出た時刻
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_error_time: Option<f64>,
}

/// モデル 1 本の比較。
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Comparison {
    pub signals: Vec<SignalComparison>,
}

impl Comparison {
    /// すべての信号が合格なら true（信号が 1 つも無ければ false）。
    pub fn passed(&self) -> bool {
        !self.signals.is_empty() && self.signals.iter().all(|s| s.passed)
    }
}

/// `signals`（`time` を除いた信号の名前。MAP-LIB の `comparisonSignals.txt`）を、参照結果の管で判定する。
pub fn compare(
    reference: &Trajectory,
    actual: &Trajectory,
    signals: &[&str],
    options: &TubeOptions,
) -> Comparison {
    let _ = (reference, actual, signals, options);
    Comparison::default()
}

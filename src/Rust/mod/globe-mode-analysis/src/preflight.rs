//! S10（プリフライト）: パラメータと start 値の評価、experiment annotation。S010。
//!
//! 骨格（Phase 0）。中身はこれから書く。

use globe_mode_core::Diagnostic;
use globe_mode_ir::PreflightStructure;
use serde::Serialize;

/// S10 の結果。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct PreflightReport {
    /// 評価したパラメータのスカラーの数
    pub parameters_evaluated: usize,
}

impl PreflightReport {
    /// 診断（S10）。`preflight` は、この結果を作ったときと同じ（位置のある）ものを渡す。
    pub fn diagnostics(&self, _preflight: &PreflightStructure) -> Vec<Diagnostic> {
        Vec::new()
    }
}

/// S10 の解析。
pub fn analyze_preflight(preflight: &PreflightStructure) -> PreflightReport {
    PreflightReport {
        parameters_evaluated: preflight.parameters_evaluated,
    }
}

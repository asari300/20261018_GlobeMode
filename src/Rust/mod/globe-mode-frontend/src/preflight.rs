//! プリフライト（S10）の入力の取り出し: パラメータと start 値の評価、experiment annotation。S010。
//!
//! 骨格（Phase 0）。中身はこれから書く。

use globe_mode_ir::PreflightStructure;
use rumoca_compile::compile::CompilationResult;

/// コンパイルの結果から、S10 の入力を取り出す。取り出せなければ理由（`check` の notes に入る）。
pub(crate) fn extract(_result: &CompilationResult) -> Result<PreflightStructure, String> {
    Ok(PreflightStructure::default())
}

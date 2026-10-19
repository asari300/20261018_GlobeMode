//! S9（代数ループ）: BLT 分解、tearing、非線形の反復変数の start 値。S010。
//!
//! 骨格（Phase 0）。中身はこれから書く。

use globe_mode_core::Diagnostic;
use globe_mode_ir::{DaeStructure, InitStructure};
use serde::Serialize;

use crate::pantelides::IndexReport;

/// S9 の結果。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct LoopReport {
    /// BLT のブロックの数
    pub blocks: usize,
}

impl LoopReport {
    /// 診断（S9）。`structure` と `init` は、この結果を作ったときと同じ（位置のある）ものを渡す。
    pub fn diagnostics(
        &self,
        _structure: &DaeStructure,
        _init: Option<&InitStructure>,
    ) -> Vec<Diagnostic> {
        Vec::new()
    }
}

/// S9 の解析。`init` は start 値の有無に使う（無ければ start 値はすべて無いとみなす）。
pub fn analyze_loops(
    _structure: &DaeStructure,
    _index: &IndexReport,
    _init: Option<&InitStructure>,
) -> LoopReport {
    LoopReport::default()
}

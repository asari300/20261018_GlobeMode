//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）の実行系のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイル（とこの下のモジュール）だけ。

use globe_mode_core::Diagnostic;
use globe_mode_ir::{ResolvedSettings, SimRun};

use crate::CompiledModel;

pub(crate) fn preflight(model: &CompiledModel, settings: &ResolvedSettings) -> Vec<Diagnostic> {
    let _ = (model.rumoca_dae(), settings);
    Vec::new()
}

pub(crate) fn simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
) -> Result<SimRun, Vec<Diagnostic>> {
    let _ = (
        model.rumoca_dae(),
        settings,
        rumoca_sim::SimOptions::default(),
    );
    Err(Vec::new())
}

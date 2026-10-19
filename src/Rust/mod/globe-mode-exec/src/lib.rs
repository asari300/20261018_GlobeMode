//! GlobeMode の実行系（S011、ADR-0013）。
//!
//! Phase A は Rumoca の実行系（`rumoca-sim`。diffsol の BDF と Rumoca の Dormand–Prince 5(4)）で、
//! フロントエンドが DAE まで変換したモデル（[`CompiledModel`]）を実行する。Rumoca の型は外へ出さず、
//! 結果は中立の [`SimRun`]（`globe-mode-ir`）、失敗は GlobeMode の診断（`Stage::S10` と `Stage::Run`）で返す。
//!
//! - [`preflight`] … S10 の数値の部分（t0 で式の値が有限か）。積分の前に一度だけ。
//! - [`simulate`] … 実行。失敗は段階ごとのコード（GM-E11001〜GM-E11006、GM-U0004 / GM-U0005）にする。
//! - [`run`] … 設定を決め（`globe_mode_sim::settings::resolve`）、プリフライトを通してから実行する。
//!
//! Rumoca の時間切れは協調型で、初期化と積分器の 1 歩の内側では止まらない。確実に止めたいときは
//! 別プロセスで回して外から止める（xtask の `msl-sim`）。

use globe_mode_core::Diagnostic;
pub use globe_mode_frontend::CompiledModel;
use globe_mode_ir::{ResolvedSettings, SimRun, SimSettings};

#[cfg(feature = "rumoca-exec")]
mod rumoca;

/// [`run`] の結果。
#[derive(Debug, Clone, Default)]
pub struct SimReport {
    /// 設定・プリフライト・実行の診断（情報と警告を含む）
    pub diagnostics: Vec<Diagnostic>,
    /// 実行できたときの結果
    pub run: Option<SimRun>,
}

impl SimReport {
    /// 結果があり、Error の診断が無ければ true。
    pub fn succeeded(&self) -> bool {
        self.run.is_some() && !self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// S10 の数値の部分（積分の前の検査）。t0 で start 値から評価した代数変数と導関数が有限か（GM-E10005）。
pub fn preflight(model: &CompiledModel, settings: &ResolvedSettings) -> Vec<Diagnostic> {
    #[cfg(feature = "rumoca-exec")]
    {
        rumoca::preflight(model, settings)
    }
    #[cfg(not(feature = "rumoca-exec"))]
    {
        let _ = (model, settings);
        Vec::new()
    }
}

/// モデルを実行する。成功なら結果（警告・情報の診断は [`run`] が集める）、失敗なら診断を返す。
pub fn simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
) -> Result<SimRun, Vec<Diagnostic>> {
    #[cfg(feature = "rumoca-exec")]
    {
        rumoca::simulate(model, settings)
    }
    #[cfg(not(feature = "rumoca-exec"))]
    {
        let _ = (model, settings);
        Err(Vec::new())
    }
}

/// 設定を決め、プリフライト（S10 の数値の部分）を通してから実行する。
pub fn run(model: &CompiledModel, requested: &SimSettings) -> SimReport {
    let mut report = SimReport::default();
    let settings = match globe_mode_sim::settings::resolve(requested, model.experiment.as_ref()) {
        Ok((settings, diagnostics)) => {
            report.diagnostics.extend(diagnostics);
            settings
        }
        Err(diagnostics) => {
            report.diagnostics.extend(diagnostics);
            return report;
        }
    };
    let checked = preflight(model, &settings);
    let stop = checked.iter().any(Diagnostic::is_error);
    report.diagnostics.extend(checked);
    if stop {
        return report;
    }
    match simulate(model, &settings) {
        Ok(run) => report.run = Some(run),
        Err(diagnostics) => report.diagnostics.extend(diagnostics),
    }
    report
}

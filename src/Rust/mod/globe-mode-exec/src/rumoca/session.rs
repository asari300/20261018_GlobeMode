//! lowering と実行を分けて呼ぶ道（S012、ADR-0013 の追記）。
//!
//! Rumoca の FMI ME 風の host（`rumoca_solver::fmi_me`）に、数値積分のプラグイン（`MeIntegratorBackend`）を
//! 差して回す。ハイブリッドの driver（初期化・イベント・根探索・出力の格子・時間切れ）は Rumoca の host が
//! 受け持ち、こちらは積分器だけを選ぶ。
//!
//! 使う場面は 2 つある。
//!
//! - wasm32: rumoca-sim の `build_simulation`（diffsol.rs:200,206）と `rk45::simulate`（rk45.rs:107,112）は
//!   std の `Instant::now` を呼び、wasm32-unknown-unknown では panic する。この道は呼ばない（Rumoca の時間切れは
//!   wasm32 では `instant` を使う。`rumoca-solver/src/runtime/timeout.rs`）。
//! - 自前の積分器（`esdirk34`・`trbdf2`・`tsit45`・`euler`・`heun`・`rk4`・`impeuler`・`trapezoid`・`esdirk23`・`esdirk45`・`sdirk34hw`・`dopri853`・`cerk23`・`cerk34`・`cerk45`・`radau5`・`qss`・`cvode`・`ida`。[`super::plugins`]）。
//!
//! 呼ぶ順は rumoca-sim の `me_backend::simulate_artifact`（Apache-2.0、rev 276e82bd）と同じで、公開の関数だけを
//! 使う（コードは写していない）: `lower_correlated_for_simulation_with_overrides` → `finish_fmi_component` →
//! `MeRetainedComponent::instantiate` → `into_lease`（初期化）→ `numerical_setup` → `into_session` →
//! `run_to_stop`。Cranelift の JIT の可否を決める関数（`admitted_native_execution_backend`）は公開されていない
//! ので、この道はネイティブでもインタプリタで回る。

use globe_mode_ir::{ResolvedSettings, ResultParameter, SolverId};
use rumoca_ir_dae::Dae;
use rumoca_sim::{SimError, SimFailureStage, SimOptions, SimResult, SimulationDiagnosticError};
use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeInstanceConfig, MeIntegrationError,
    MeIntegratorBackend, MeModelArtifact, MeNumericalSetup, MeStepCandidate,
    driver::{batch_output_cursor, batch_session_options},
    session::MeRetainedComponent,
};

use super::{Hooks, Outcome, Phase, Stopwatch, plugins, result_parameters};
use crate::ExecStage;

/// Rumoca の host に差す数値積分のプラグイン。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Integrator {
    /// Rumoca の BDF（`rumoca_solver_diffsol::model_exchange_integrator`。ネイティブの `build_simulation` と同じ部品）
    RumocaBdf,
    /// Rumoca の Dormand–Prince 5(4)（`rumoca_solver_rk45::model_exchange_integrator`）
    RumocaRk45,
    /// 自前の積分器（[`plugins::plugin`]）
    Own(SolverId),
}

/// 1 本の実行の名前（Rumoca の FMI のインスタンス名）。
const INSTANCE_NAME: &str = "globe-mode";

/// lowering から積分までを回す。
pub(super) fn run(
    dae: &Dae,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    integrator: Integrator,
    hooks: &mut Hooks<'_>,
) -> Outcome {
    let mut build_seconds = 0.0;
    let mut run_seconds = 0.0;
    let mut parameters = Vec::new();
    let result = (|| {
        let started = Stopwatch::start();
        let built = build(dae, opts, settings, &mut parameters, hooks);
        build_seconds = started.seconds();
        let (artifact, retained) = built.map_err(|e| (e, Phase::Build))?;
        let started = Stopwatch::start();
        let result = integrate(&artifact, retained, opts, settings, integrator, hooks);
        run_seconds = started.seconds();
        result
    })();
    Outcome {
        parameters,
        ..Outcome::new(result, build_seconds, run_seconds)
    }
}

/// lowering と FMI の部品の組み立て。lowering の結果から、実行に使う tunable なパラメータの値を読み、
/// プリフライトを行う。
fn build(
    dae: &Dae,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    parameters: &mut Vec<ResultParameter>,
    hooks: &mut Hooks<'_>,
) -> Result<(MeModelArtifact, MeRetainedComponent), SimError> {
    hooks.enter(ExecStage::Lowering)?;
    let lowered = rumoca_sim::lower_correlated_for_simulation_with_overrides(dae, opts)
        .map_err(|e| lowering_error(&e))?;
    *parameters = result_parameters(dae, &lowered);
    hooks.lowered(dae, settings, &lowered);
    let component = rumoca_phase_solve::fmi::finish_fmi_component(lowered).map_err(|e| {
        lowering_error(&SimulationDiagnosticError::RuntimePreparation {
            message: e.to_string(),
            span: e.span(),
        })
    })?;
    let artifact = MeModelArtifact::new(component);
    let config = MeInstanceConfig::new(INSTANCE_NAME, opts.rtol, opts.t_start, opts.t_end)
        .map_err(|e| SimError::from(e).at_stage(SimFailureStage::BackendBuild))?;
    let retained = MeRetainedComponent::instantiate(artifact.source(), &config, None)
        .map_err(|e| SimError::from(e).at_stage(SimFailureStage::BackendBuild))?;
    Ok((artifact, retained))
}

/// `into_session`（プラグインの始動。BDF はここでヤコビ行列を作る）の失敗の段。ネイティブの型付きの BDF の道は
/// 同じ呼び出しを `check_prepared_initialization`（初期化）の中で回すので Initialization。rk45 と自前の積分器は
/// 型付きの道も積分の段なので Integration（#5）。
pub(super) fn start_phase(integrator: Integrator) -> Phase {
    match integrator {
        Integrator::RumocaBdf => Phase::Initialization,
        Integrator::RumocaRk45 | Integrator::Own(_) => Phase::Integration,
    }
}

/// 初期化（`into_lease`）と積分（`run_to_stop`）。
fn integrate(
    artifact: &MeModelArtifact,
    retained: MeRetainedComponent,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    integrator: Integrator,
    hooks: &mut Hooks<'_>,
) -> Result<SimResult, (SimError, Phase)> {
    let options = batch_session_options(
        &artifact.root_location(),
        opts.t_start,
        opts.t_end,
        opts.rtol,
        opts.atol,
        settings.interval,
        opts.max_wall_seconds,
    )
    .map_err(|e| (SimError::from(e), Phase::Build))?;
    let mut cursor =
        batch_output_cursor(&options).map_err(|e| (SimError::from(e), Phase::Build))?;
    hooks
        .enter(ExecStage::Initialization)
        .map_err(|e| (e, Phase::Initialization))?;
    let host = retained
        .into_lease(options)
        .map_err(|e| (SimError::from(e), Phase::Initialization))?;
    if host.is_terminated() {
        return Ok(host.finish());
    }
    let plugin = if host.state_count() == 0 {
        None
    } else {
        let setup = host
            .numerical_setup(Some(initial_step_hint(settings)))
            .map_err(|e| (SimError::from(e), Phase::Initialization))?;
        Some(backend(integrator, setup, settings).map_err(|e| (e, Phase::Build))?)
    };
    hooks
        .enter(ExecStage::Integration)
        .map_err(|e| (e, Phase::Integration))?;
    let (ran, trace) = {
        let plugin = plugin.map(|inner| -> Box<dyn MeIntegratorBackend + '_> {
            Box::new(Observed {
                inner,
                hooks: &mut *hooks,
            })
        });
        let mut session = host
            .into_session(plugin)
            .map_err(|e| (SimError::from(e), start_phase(integrator)))?;
        let ran = session.run_to_stop(&mut cursor);
        (ran, session.finish())
    };
    let end = trace.times.last().copied().unwrap_or(opts.t_start);
    match ran {
        Ok(()) => {}
        // 積分器の 1 歩の前で止めた。そこまでの列は壊れていない
        Err(_) if hooks.cancelled => hooks.cancelled_at = Some(end),
        Err(e) => return Err((SimError::from(e), Phase::Integration)),
    }
    hooks.emit(ExecStage::Integration, Some(end));
    Ok(trace)
}

/// 積分器を包み、1 歩ごとに中断を見て t を知らせる（ADR-0033）。区切りの時刻を足さないので、刻みも数値も
/// 包まないときと同じ（`advance_to` を出力の点で区切ると、区切りが刻みの上限になり数値が変わる。S123）。
struct Observed<'h, 'a> {
    inner: Box<dyn MeIntegratorBackend>,
    hooks: &'h mut Hooks<'a>,
}

impl MeIntegratorBackend for Observed<'_, '_> {
    fn initialize(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: MeDerivativeHandle,
    ) -> Result<(), MeIntegrationError> {
        self.inner.initialize(point, derivatives)
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        if self.hooks.is_cancelled() {
            return Err(MeIntegrationError::Contract {
                reason: "呼び手が止めた".to_string(),
            });
        }
        self.hooks.tick(request.current().time());
        self.inner.advance(request)
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        self.inner.sample(time, states)
    }

    fn truncate_reset(&mut self, point: &MeContinuousPoint) -> Result<(), MeIntegrationError> {
        self.inner.truncate_reset(point)
    }
}

/// プラグインを作る。
fn backend(
    integrator: Integrator,
    setup: MeNumericalSetup,
    settings: &ResolvedSettings,
) -> Result<Box<dyn MeIntegratorBackend + 'static>, SimError> {
    match integrator {
        Integrator::RumocaBdf => Ok(rumoca_solver_diffsol::model_exchange_integrator(setup)),
        Integrator::RumocaRk45 => Ok(rumoca_solver_rk45::model_exchange_integrator(setup)),
        Integrator::Own(id) => plugins::plugin(id, setup, settings).map_err(|message| {
            SimError::SolverError(message).at_stage(SimFailureStage::BackendBuild)
        }),
    }
}

/// 最初の刻みの目安（rumoca-sim の `me_backend::default_step_size` と同じ: 出力の間隔、ただし 0.01 まで）。
/// 固定刻みのソルバは刻み幅そのもの。
fn initial_step_hint(settings: &ResolvedSettings) -> f64 {
    match settings.step {
        Some(h) if h.is_finite() && h > 0.0 => h,
        _ if settings.interval.is_finite() && settings.interval > 0.0 => {
            settings.interval.min(0.01)
        }
        _ => 1.0e-3,
    }
}

/// lowering の失敗を、rumoca-sim の `diagnostic_sim_error`（diffsol.rs:423）と同じ形の SimError にする
/// （`build_simulation` の道と同じ段になる）。
pub(super) fn lowering_error(e: &SimulationDiagnosticError) -> SimError {
    let stage = match e {
        SimulationDiagnosticError::SolveLowering(_) if e.is_structural() => {
            SimFailureStage::StructuralAnalysis
        }
        SimulationDiagnosticError::SolveLowering(_)
        | SimulationDiagnosticError::InvalidOverride { .. } => SimFailureStage::SolveLowering,
        SimulationDiagnosticError::RuntimePreparation { .. } => SimFailureStage::BackendBuild,
        SimulationDiagnosticError::Solver(_) => SimFailureStage::Integration,
    };
    SimError::SolveIr(format!("[{}] {e}", e.diagnostic_code())).at_stage(stage)
}

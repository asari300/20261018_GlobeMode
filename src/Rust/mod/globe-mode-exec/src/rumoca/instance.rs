//! 状態・パラメータ・時刻から微分を返す公開の API（S099、P5b の 2 回目。ADR-0032 の追記）。
//!
//! Rumoca の FMI ME 風の host（[`super::session`]）は set time / set states / get derivatives を crate の外に
//! 出していない（`SolveMeKernel` の関数は `pub(crate)`、積分器に渡す `MeDerivativeHandle` は host の呼び出しの
//! 間だけ使える）。そこで host と同じ部品の `rumoca_solver::SolveRuntime`（公開）を直接持つ: lowering は
//! [`crate::tunable_parameters`] と同じ `lower_for_simulation_with_overrides`、初期化は
//! `settle_initialization_system`（[`super::resettle`] の 1 回目と同じ）、微分は `eval_state_derivatives`。

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use globe_mode_core::{Diagnostic, Stage, codes};
use rumoca_ir_dae::Dae;
use rumoca_sim::SimOptions;
use rumoca_solver::SolveRuntime;

use super::{Phase, SETTLE_MAX_ITERS, failure_diagnostic, panic_diagnostic, session};
use crate::CompiledModel;

/// 代数ループの Newton 法の許容誤差（[`super::probe_start`] と同じ）。
const EVAL_TOL: f64 = 1.0e-10;

/// 1 つのモデルのインスタンス。時刻と連続の状態を外から与え、微分を返す（FMI 2.0 ME の
/// `fmi2SetTime`・`fmi2SetContinuousStates`・`fmi2GetDerivatives` に当たる）。
///
/// shortcut: 事象（根の条件）のあるモデルは [`ModelInstance::new`] で断る。事象の表示関数と事象の処理は
/// 載せていない（P5b の持ち越し）。
pub struct ModelInstance {
    runtime: SolveRuntime,
    /// パラメータの枠（初期化が `fixed = false` のパラメータを書き込んだ後の値）
    params: Vec<f64>,
    states: Vec<f64>,
    time: f64,
    state_names: Vec<String>,
    parameters: Vec<(String, f64)>,
}

impl std::fmt::Debug for ModelInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelInstance")
            .field("time", &self.time)
            .field("state_names", &self.state_names)
            .field("states", &self.states)
            .field("parameters", &self.parameters)
            .finish_non_exhaustive()
    }
}

impl ModelInstance {
    /// lowering し、`start_time` で初期化する。`parameters` は tunable なパラメータの上書き
    /// （[`globe_mode_ir::ResolvedSettings::parameters`] と同じ。lowering で反映するので、依存するパラメータにも届く）。
    pub fn new(
        model: &CompiledModel,
        parameters: &BTreeMap<String, f64>,
        start_time: f64,
    ) -> Result<Self, Vec<Diagnostic>> {
        let dae: &Dae = model.rumoca_dae();
        let opts = SimOptions {
            param_overrides: parameters.iter().map(|(n, v)| (n.clone(), *v)).collect(),
            ..SimOptions::default()
        };
        let built = catch_unwind(AssertUnwindSafe(|| {
            let solve =
                rumoca_sim::lower_for_simulation_with_overrides(dae, &opts).map_err(|e| {
                    failure_diagnostic(dae, &session::lowering_error(&e), Phase::Build, None)
                })?;
            let tunable = rumoca_sim::build_tunable_parameter_meta(dae, &solve).map_err(|e| {
                failure_diagnostic(dae, &session::lowering_error(&e), Phase::Build, None)
            })?;
            let runtime = SolveRuntime::new(&solve)
                .map_err(|e| run_error(codes::RUN_INTERNAL, e.to_string()))?;
            Ok::<_, Diagnostic>((runtime, tunable))
        }));
        let (runtime, tunable) = match built {
            Ok(Ok(built)) => built,
            Ok(Err(d)) => return Err(vec![d]),
            Err(payload) => return Err(vec![panic_diagnostic(payload.as_ref(), None)]),
        };
        if runtime.root_condition_count() > 0
            || !runtime
                .model
                .problem
                .clocks
                .periodic_event_schedules
                .is_empty()
        {
            return Err(vec![run_error(
                codes::RUN_UNSUPPORTED,
                "事象（根の条件・sample）のあるモデルはまだ載せられない".to_string(),
            )]);
        }
        let n = runtime.state_count;
        let mut y = runtime.model.initial_y.clone();
        let mut params = runtime.model.parameters.clone();
        runtime
            .settle_initialization_system(
                &mut y,
                &mut params,
                start_time,
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| {
                vec![run_error(
                    codes::RUN_INIT_FAILED,
                    format!("初期化に失敗: {e}"),
                )]
            })?;
        Ok(Self {
            state_names: runtime.model.problem.solve_layout.solver_maps.names[..n].to_vec(),
            parameters: tunable
                .into_iter()
                .map(|m| (m.name, m.default_value))
                .collect(),
            states: y[..n].to_vec(),
            params,
            time: start_time,
            runtime,
        })
    }

    /// 連続の状態の数。
    pub fn n_states(&self) -> usize {
        self.states.len()
    }

    /// 連続の状態の名前（[`Self::states`] と同じ順）。
    pub fn state_names(&self) -> &[String] {
        &self.state_names
    }

    /// tunable なパラメータの名前と、このインスタンスで使っている値（上書きを反映した値）。
    pub fn parameters(&self) -> &[(String, f64)] {
        &self.parameters
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn set_time(&mut self, time: f64) {
        self.time = time;
    }

    /// 連続の状態（初期化の直後は初期値）。
    pub fn states(&self) -> &[f64] {
        &self.states
    }

    /// 連続の状態を置く。長さが [`Self::n_states`] と違えば断る。
    pub fn set_states(&mut self, states: &[f64]) -> Result<(), String> {
        if states.len() != self.states.len() {
            return Err(format!(
                "状態は {} 個（{} 個を渡した）",
                self.states.len(),
                states.len()
            ));
        }
        self.states.copy_from_slice(states);
        Ok(())
    }

    /// 今の時刻と状態での微分（[`Self::states`] と同じ順）。
    pub fn derivatives(&self) -> Result<Vec<f64>, String> {
        self.runtime
            .eval_state_derivatives(
                self.time,
                &self.states,
                &self.params,
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| e.to_string())
    }
}

fn run_error(code: &'static str, message: String) -> Diagnostic {
    Diagnostic::error(code, Stage::Run, message)
}

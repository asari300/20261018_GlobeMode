//! 状態・パラメータ・時刻から微分を返す公開の API（S099、P5b の 2 回目。ADR-0032 の追記）。
//!
//! Rumoca の FMI ME 風の host（[`super::session`]）は set time / set states / get derivatives を crate の外に
//! 出していない（`SolveMeKernel` の関数は `pub(crate)`、積分器に渡す `MeDerivativeHandle` は host の呼び出しの
//! 間だけ使える）。そこで host と同じ部品の `rumoca_solver::SolveRuntime`（公開）を直接持つ: lowering は
//! [`crate::tunable_parameters`] と同じ `lower_for_simulation_with_overrides`、初期化は
//! `settle_initialization_system`（[`super::resettle`] の 1 回目と同じ）、微分は `eval_state_derivatives`。
//!
//! S109: 事象（FMI 2.0 ME の事象指示関数と Event Mode）。事象指示関数は `eval_root_conditions`、事象の処理は
//! host の `SolveMeKernel::apply_discrete_event_updates` と同じ部品（公開）: `apply_projected_event_update` →
//! `commit_pre_params_after_event_at`。次の時刻事象（`sample`）は `next_runtime_event_stop`。
//!
//! S112: 代数・離散の変数の値。結果の列と同じ `visible_values`（host の `get_outputs`・`record_outputs` と同じ部品）を
//! `full_solver_y` の上で評価し、役割は `variable_meta`（結果の `variable_meta` と同じ）から決める。
//!
//! S114: 入力（`input`）。Rumoca は入力をパラメータの枠の `input_parameter_index` に置く（host の `set_float64` が
//! 書く所と同じ）。lowering の前の値は FMI の export と同じく `start`（`SimOptions::initial_inputs` で渡す）。

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_ir::VariableRole;
use rumoca_ir_dae::Dae;
use rumoca_sim::SimOptions;
use rumoca_solver::runtime::schedule::ScheduledEventConsumption;
use rumoca_solver::{
    EventActionOutcome, EventUpdateRowFilter, ProjectedEventUpdateInput,
    ProjectedInitialEventInput, SolveRuntime, SolveStopSchedule, commit_pre_params_after_event_at,
    convert_variable_meta, orient_typed_root_zeros, root_crossings_with_relation_memory,
};

use super::{Phase, SETTLE_MAX_ITERS, failure_diagnostic, panic_diagnostic, session};
use crate::CompiledModel;

/// 代数ループの Newton 法の許容誤差（[`super::probe_start`] と同じ）。
const EVAL_TOL: f64 = 1.0e-10;

/// 時刻事象を探す先の端。FMI の ME は終了時刻を知らせなくてよいので無限の代わりに置く
/// （Rumoca の `next_periodic_event_time` は有限の端を要る）。
const HORIZON: f64 = f64::MAX;

/// 1 つのモデルのインスタンス。時刻と連続の状態を外から与え、微分を返す（FMI 2.0 ME の
/// `fmi2SetTime`・`fmi2SetContinuousStates`・`fmi2GetDerivatives` に当たる）。
///
/// 事象は [`Self::event_indicators`]・[`Self::enter_event_mode`]・[`Self::update_discrete_states`]（S109）。
///
/// S237: 初期化の事象（`when initial()` など）は [`ModelInstance::new`] が回す（`apply_projected_initial_event_boundary`）。
pub struct ModelInstance {
    runtime: SolveRuntime,
    /// パラメータの枠（初期化が `fixed = false` のパラメータを書き込んだ後の値）
    params: Vec<f64>,
    states: Vec<f64>,
    time: f64,
    state_names: Vec<String>,
    parameters: Vec<(String, f64)>,
    /// 静的な時刻事象と `sample` の予定
    schedule: SolveStopSchedule,
    /// 直前の事象の後（または初期化の後）の事象指示関数の値。根を越えたかをこれと比べる
    roots: Vec<f64>,
    /// Event Mode に入ったときの（solver の y、パラメータの枠）＝ `pre` の値
    event_entry: Option<(Vec<f64>, Vec<f64>)>,
    /// 代数・出力・離散・入力の変数と、Rumoca の見える変数（`visible_names`）での位置
    variables: Vec<(Variable, usize)>,
    /// solver の y 全体（S237）。状態でない欄に事象で決まった離散の値（`when` の `y_last = …` など）が載る。状態から作り直すと
    /// 初期値に戻ってしまうので、Rumoca の FMI ME と同じく前回の y を guess にして状態だけ上書きする
    held_y: std::cell::RefCell<Vec<f64>>,
}

/// 状態とパラメータ以外の変数 1 つ（S112。値は [`ModelInstance::variable_values`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    /// [`VariableRole::Algebraic`]・[`VariableRole::Output`]・[`VariableRole::Discrete`]・
    /// [`VariableRole::Input`]（S114。[`ModelInstance::set_input`] で書ける）のどれか
    pub role: VariableRole,
    pub value_type: ValueType,
}

/// [`Variable`] の型（値はどれも f64 で返す。Integer は整数の値、Boolean は 0 か 1、Enumeration は ordinal）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Real,
    Integer,
    Boolean,
    /// S224: 値は 1 始まりの ordinal（Integer と同じ f64 の整数）。型の名前とリテラルは `CompiledModel::enum_type_of`
    Enumeration,
}

/// [`ModelInstance::update_discrete_states`] の結果（FMI 2.0 の `fmi2EventInfo` に当たる）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteUpdate {
    /// `reinit` で連続の状態が変わった
    pub states_changed: bool,
    /// `terminate()` が呼ばれた
    pub terminate: bool,
    /// 次の時刻事象（無ければ None）
    pub next_event_time: Option<f64>,
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
        let initial_inputs =
            input_starts(dae).map_err(|e| vec![run_error(codes::RUN_INTERNAL, e)])?;
        let opts = SimOptions {
            param_overrides: parameters.iter().map(|(n, v)| (n.clone(), *v)).collect(),
            initial_inputs,
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
        // S237: 関係式の記憶（`abs(w) >= w0` のような noEvent でない関係式の今の側）を初期化の点から決める。Rumoca の FMI ME の
        // `exit_initialization_mode` と同じ順（初期化系 → 関係式の記憶）。無いと記憶は既定の false のまま始まり、
        // 最初の事象まで違う枝で微分を返す。解き直しはしない（Rumoca も解き直さない）
        runtime
            .update_relation_memory_from_state(
                start_time,
                &y[..n],
                &mut params,
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| {
                vec![run_error(
                    codes::RUN_INIT_FAILED,
                    format!("関係式の記憶の初期化に失敗: {e}"),
                )]
            })?;
        let meta: std::collections::HashMap<_, _> =
            convert_variable_meta(&runtime.model.variable_meta)
                .into_iter()
                .map(|m| (m.name.clone(), m))
                .collect();
        // shortcut: String の変数（パラメータ以外）は載せない。FMU で要るときに足す
        let variables = (runtime.model.visible_names.iter().enumerate())
            .filter_map(|(i, name)| {
                let m = meta.get(name);
                let role = super::role(m);
                let value_type = match m.and_then(|m| m.value_type.as_deref()) {
                    Some("Integer") => ValueType::Integer,
                    Some("Boolean") => ValueType::Boolean,
                    Some("Enumeration") => ValueType::Enumeration,
                    Some("Real") | None => ValueType::Real,
                    Some(_) => return None,
                };
                matches!(
                    role,
                    VariableRole::Algebraic
                        | VariableRole::Output
                        | VariableRole::Discrete
                        | VariableRole::Input
                )
                .then(|| {
                    let name = name.clone();
                    (
                        Variable {
                            name,
                            role,
                            value_type,
                        },
                        i,
                    )
                })
            })
            .collect();
        let mut inst = Self {
            variables,
            held_y: Default::default(),
            schedule: SolveStopSchedule::new(&runtime.model.problem, start_time, HORIZON),
            roots: Vec::new(),
            event_entry: None,
            state_names: runtime.model.problem.solve_layout.solver_maps.names[..n].to_vec(),
            parameters: tunable
                .into_iter()
                .map(|m| (m.name, m.default_value))
                .collect(),
            states: y[..n].to_vec(),
            params,
            time: start_time,
            runtime,
        };
        inst.run_initial_event()
            .map_err(|e| vec![run_error(codes::RUN_INIT_FAILED, e)])?;
        inst.roots = inst
            .event_indicators()
            .map_err(|e| vec![run_error(codes::RUN_INIT_FAILED, e)])?;
        Ok(inst)
    }

    /// 初期化の事象（`when initial()`、`pre` の種、時刻 0 に当たる `when`）。Rumoca の FMI ME の `run_initial_event_boundary` と
    /// 同じ部品（S237。無いと StateGraph のように初期の離散の状態を `pre` で決めるモデルが違う状態で始まる）。
    fn run_initial_event(&mut self) -> Result<(), String> {
        let t = self.time;
        let mut y = self
            .runtime
            .full_solver_y(t, &self.states, &self.params, EVAL_TOL, SETTLE_MAX_ITERS)
            .map_err(|e| e.to_string())?;
        let (pre_y, pre_p) = (y.clone(), self.params.clone());
        let dynamic_event = self
            .runtime
            .current_dynamic_time_event_stop(&y, &self.params, t)
            .map_err(|e| e.to_string())?;
        let runtime = &self.runtime;
        let outcome = runtime
            .apply_projected_initial_event_boundary(
                ProjectedInitialEventInput {
                    y: &mut y,
                    p: &mut self.params,
                    t_start: t,
                    t_end: HORIZON,
                    tol: EVAL_TOL,
                    event_pre_y: &pre_y,
                    event_pre_p: &pre_p,
                    max_iters: SETTLE_MAX_ITERS,
                    dynamic_event,
                },
                |y, p, t| {
                    let before = y.to_vec();
                    runtime.refresh_event_dependency_slots_certified(
                        t,
                        y,
                        p,
                        EVAL_TOL,
                        SETTLE_MAX_ITERS,
                    )?;
                    Ok(before != y)
                },
            )
            .map_err(|e| e.to_string())?;
        let n = self.states.len();
        self.states.copy_from_slice(&y[..n]);
        *self.held_y.borrow_mut() = y;
        match outcome.action {
            EventActionOutcome::AssertionFailed { time, message } => {
                Err(format!("t = {time} で assert が偽: {message}"))
            }
            _ => Ok(()),
        }
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
            .eval_state_derivatives_with_guess(
                self.time,
                &self.states,
                &self.params,
                &mut self.held_y.borrow_mut(),
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| e.to_string())
    }

    /// 今の時刻・状態での solver の y（状態でない欄は直前の事象の後の値を保つ）。
    fn solver_y(&self) -> Result<Vec<f64>, String> {
        let mut g = self.held_y.borrow_mut();
        self.runtime
            .full_solver_y_with_guess(
                self.time,
                &self.states,
                &self.params,
                &mut g,
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| e.to_string())?;
        Ok(g.clone())
    }

    /// 入力 `name` の値を置く（S114）。次の [`Self::derivatives`]・[`Self::variable_values`] から効く。
    pub fn set_input(&mut self, name: &str, value: f64) -> Result<(), String> {
        let layout = &self.runtime.model.problem.solve_layout;
        let i =
            (layout.input_parameter_index(name)).ok_or_else(|| format!("{name} は入力でない"))?;
        if !value.is_finite() {
            return Err(format!("入力 {name} の値 {value} が有限でない"));
        }
        self.params[i] = value;
        Ok(())
    }

    /// 状態とパラメータ以外の変数（代数・出力・離散・入力。S112・S114）。
    pub fn variables(&self) -> impl ExactSizeIterator<Item = &Variable> {
        self.variables.iter().map(|(v, _)| v)
    }

    /// 今の時刻と状態での [`Self::variables`] の値（同じ順）。離散の変数は直前の事象の後の値。
    pub fn variable_values(&self) -> Result<Vec<f64>, String> {
        let y = self.solver_y()?;
        let all = self
            .runtime
            .visible_values(&y, &self.params, self.time)
            .map_err(|e| e.to_string())?;
        Ok(self.variables.iter().map(|&(_, i)| all[i]).collect())
    }
}

impl ModelInstance {
    /// 事象指示関数の数（root の条件の数）。
    pub fn n_event_indicators(&self) -> usize {
        self.runtime.root_condition_count()
    }

    /// 今の時刻と状態での事象指示関数（符号が変わったら状態事象）。ちょうど 0 は関係式の向きの側に寄せる。
    pub fn event_indicators(&self) -> Result<Vec<f64>, String> {
        let mut z = self
            .runtime
            .eval_root_conditions(
                self.time,
                &self.states,
                &self.params,
                EVAL_TOL,
                SETTLE_MAX_ITERS,
            )
            .map_err(|e| e.to_string())?;
        orient_typed_root_zeros(&mut z, &self.runtime.model.problem.events.root_zero_domains);
        Ok(z)
    }

    /// Event Mode に入る（今の時刻と状態を `pre` の値として控える）。
    pub fn enter_event_mode(&mut self) -> Result<(), String> {
        let y = self.solver_y()?;
        self.event_entry = Some((y, self.params.clone()));
        Ok(())
    }

    /// 離散の状態を更新する（`when` の式と `reinit`）。Event Mode でなければ（初期化の直後）次の時刻事象を返すだけ。
    pub fn update_discrete_states(&mut self) -> Result<DiscreteUpdate, String> {
        let t = self.time;
        let mut terminate = false;
        let mut states_changed = false;
        if let Some((pre_y, pre_p)) = self.event_entry.take() {
            // S246: この時刻の時刻事象（`when sample(...)`）がもう処理済みなら、同じ時刻の 2 回目の事象（状態事象）では
            // 時計に属する行を回さない（Rumoca の FMI ME の `StateTimeCoincidence::Consumed` と同じ。回すと `Mean` の
            // `y_last = f*pre(x)` が reinit の後の x で書き直される）
            let row_filter = match self.schedule.scheduled_event_coincidence_at(t) {
                Some(c) if c.consumption == ScheduledEventConsumption::Consumed => {
                    EventUpdateRowFilter::UnownedOnly
                }
                _ => EventUpdateRowFilter::All,
            };
            // 前の事象の後から符号の変わった根を、越えた後の側に固定する（host の pending_root_crossings と同じ）
            let after = self.event_indicators()?;
            let mut overrides: Vec<(usize, f64)> = root_crossings_with_relation_memory(
                &self.roots,
                &after,
                EVAL_TOL,
                &self
                    .runtime
                    .model
                    .problem
                    .events
                    .root_relation_memory_targets,
                &self.params,
            )
            .into_iter()
            .map(|c| (c.index, c.post_relation_memory_value))
            .collect();
            let mut y = pre_y.clone();
            let runtime = &self.runtime;
            let outcome = runtime
                .apply_projected_event_update(
                    ProjectedEventUpdateInput {
                        y: &mut y,
                        p: &mut self.params,
                        t,
                        tol: EVAL_TOL,
                        event_pre_y: &pre_y,
                        event_pre_p: &pre_p,
                        max_iters: SETTLE_MAX_ITERS,
                        row_filter,
                        root_relation_overrides: &mut overrides,
                    },
                    |y, p| {
                        let before = y.to_vec();
                        runtime.refresh_event_dependency_slots_certified(
                            t,
                            y,
                            p,
                            EVAL_TOL,
                            SETTLE_MAX_ITERS,
                        )?;
                        Ok(before != y)
                    },
                )
                .map_err(|e| e.to_string())?;
            commit_pre_params_after_event_at(
                &runtime.model,
                &y,
                &mut self.params,
                Some(t),
                EVAL_TOL,
            );
            self.schedule.advance_past(t);
            let n = self.states.len();
            states_changed = self.states[..] != y[..n];
            self.states.copy_from_slice(&y[..n]);
            *self.held_y.borrow_mut() = y;
            match outcome {
                EventActionOutcome::Continue => {}
                EventActionOutcome::Terminated { .. } => terminate = true,
                EventActionOutcome::AssertionFailed { time, message } => {
                    return Err(format!("t = {time} で assert が偽: {message}"));
                }
            }
        }
        self.roots = self.event_indicators()?;
        let y = if self.runtime.dynamic_time_event_stop_reads_solver_y() {
            self.solver_y()?
        } else {
            Vec::new()
        };
        let (next, event) = self
            .runtime
            .next_runtime_event_stop(&y, &self.params, &mut self.schedule, t, HORIZON)
            .map_err(|e| e.to_string())?;
        Ok(DiscreteUpdate {
            states_changed,
            terminate,
            next_event_time: event.map(|_| next),
        })
    }
}

/// 束縛の無い入力の `start`（スカラーの名前ごと）。Rumoca の `host_driven_input_seeds`（crate の外から呼べない）と同じ:
/// 束縛のある入力と `start` の無い入力は除く（後者は lowering が入力の名前を挙げて断る）。
fn input_starts(dae: &Dae) -> Result<Vec<(String, f64)>, String> {
    dae.inspect(|view| {
        let mut out = Vec::new();
        for (_, v) in view.variables() {
            let start = match v.start() {
                Some(s)
                    if v.role() == rumoca_ir_dae::VariableRole::Input && v.binding().is_none() =>
                {
                    s
                }
                _ => continue,
            };
            let values = rumoca_phase_solve::fmi::numeric_attribute_values(view, v, Some(start))
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            for k in 0..v.scalar_count() {
                let name = v.scalar_name(k).ok_or("入力のスカラーに名前が無い")?;
                // 1 つの start が配列の入力の全要素に効く（`input Real u[3](start = 0)`）
                let value = match values.as_slice() {
                    [one] => *one,
                    many => *many
                        .get(k)
                        .ok_or(format!("入力 {name} の start の数が合わない"))?,
                };
                out.push((name, value));
            }
        }
        Ok(out)
    })
}

fn run_error(code: &'static str, message: String) -> Diagnostic {
    Diagnostic::error(code, Stage::Run, message)
}

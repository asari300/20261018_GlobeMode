//! diffsol の Runge–Kutta 法の積分器（`tsit45`・`esdirk34`・`trbdf2`）。刻み幅の制御は diffsol が受け持つ。
//!
//! 出典と改変: Rumoca（Apache-2.0）の `crates/rumoca-solver-diffsol/src/me_integrator.rs`（rev 276e82bd）の
//! BDF の積分器を写して改変した。変えたのは、方法を diffsol の BDF から陽的 RK（`tsit45_solver`）と SDIRK
//! （`esdirk34_solver`・`tr_bdf2_solver`）に替えたこと、状態を `BdfState` から `RkState` に替えたこと、2 つの方法を 1 つの型（[`DiffsolRk`]）にまとめたこと。
//! ホストとの約束（区間の端の補間、終点の上限、`has_failed` と `take_discard`、疎の形の探り）は元のまま。
//!
//! 陽的 RK も `rhs_implicit`（ヤコビ行列とベクトルの積つき）の問題で作る。diffsol の `.rhs()` が作る
//! `ClosureNoJac` は公開されておらず、self_cell に入れる型を書けないため。陽的 RK はヤコビ行列を呼ばない
//! （build のときの疎の形の探りだけ。部品が状態ヤコビ行列の構造を持たなければ空の形で答える）。

use std::{cell::Cell, rc::Rc};

use diffsol::{
    Closure, ConstantClosure, DiffsolError, FaerSparseLU, FaerSparseMat, MatrixCommon, OdeBuilder,
    OdeSolverMethod, OdeSolverProblem, OdeSolverState, OdeSolverStopReason, RkState, UnitCallable,
    VectorHost, error::OdeSolverError,
};
use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeIntegrationError,
    MeIntegratorBackend, MeNumericalFailure, MeNumericalSetup, MeStepCandidate,
    accepted_interval_contains, accepted_step_roundoff,
};
use self_cell::self_cell;

type Matrix = FaerSparseMat<f64>;
type Vector = <Matrix as MatrixCommon>::V;
type LinearSolver = FaerSparseLU<f64>;

type RhsFn = Box<dyn Fn(&Vector, &Vector, f64, &mut Vector)>;
type JacobianFn = Box<dyn Fn(&Vector, &Vector, f64, &Vector, &mut Vector)>;
type InitialFn = Box<dyn Fn(&Vector, f64, &mut Vector)>;
type Initial = ConstantClosure<Matrix, InitialFn>;
type Empty = UnitCallable<Matrix>;
type Equations = diffsol::OdeSolverEquations<
    Matrix,
    Closure<Matrix, RhsFn, JacobianFn>,
    Initial,
    Empty,
    Empty,
    Empty,
    Empty,
>;
type Problem = OdeSolverProblem<Equations>;
type ExplicitSolver<'p> = diffsol::ExplicitRk<'p, Equations>;
type SdirkSolver<'p> = diffsol::Sdirk<'p, Equations, LinearSolver>;

self_cell!(
    /// diffsol の問題と、それを借りる陽的 RK の方法（`rumoca-solver-diffsol` の BDF と同じ形）。
    struct ExplicitCell {
        owner: Problem,

        #[not_covariant]
        dependent: ExplicitSolver,
    }
);

self_cell!(
    /// diffsol の問題と、それを借りる SDIRK の方法。
    struct SdirkCell {
        owner: Problem,

        #[not_covariant]
        dependent: SdirkSolver,
    }
);

/// diffsol の方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    /// Tsitouras 5(4)（陽的）
    Tsit45,
    /// ESDIRK 3(4)
    Esdirk34,
    /// TR-BDF2
    TrBdf2,
}

impl Method {
    const fn name(self) -> &'static str {
        match self {
            Method::Tsit45 => "diffsol-tsit45",
            Method::Esdirk34 => "diffsol-esdirk34",
            Method::TrBdf2 => "diffsol-trbdf2",
        }
    }

    /// 最初の刻みの見積もり（`RkState::set_step_size`）に渡す方法の次数。
    const fn order(self) -> usize {
        match self {
            Method::Tsit45 => 5,
            Method::Esdirk34 => 4,
            Method::TrBdf2 => 2,
        }
    }
}

enum Solver {
    Explicit(ExplicitCell),
    Sdirk(SdirkCell),
}

/// 問題と方法を 1 つの式で扱う（2 つの self_cell の型を同じ本体で回す）。
macro_rules! with_method {
    ($solver:expr, |$problem:ident, $method:ident| $body:expr) => {
        match $solver {
            Solver::Explicit(cell) => cell.with_dependent(|$problem, $method| $body),
            Solver::Sdirk(cell) => cell.with_dependent(|$problem, $method| $body),
        }
    };
}

macro_rules! with_method_mut {
    ($solver:expr, |$problem:ident, $method:ident| $body:expr) => {
        match $solver {
            Solver::Explicit(cell) => cell.with_dependent_mut(|$problem, $method| $body),
            Solver::Sdirk(cell) => cell.with_dependent_mut(|$problem, $method| $body),
        }
    };
}

pub(super) struct DiffsolRk {
    method: Method,
    setup: MeNumericalSetup,
    derivatives: Option<Rc<MeDerivativeHandle>>,
    solver: Option<Solver>,
    accepted_interval: Option<(f64, f64)>,
}

impl DiffsolRk {
    pub(super) fn new(method: Method, setup: MeNumericalSetup) -> Self {
        Self {
            method,
            setup,
            derivatives: None,
            solver: None,
            accepted_interval: None,
        }
    }

    fn numerical(
        &self,
        category: MeNumericalFailure,
        error: impl std::fmt::Display,
    ) -> MeIntegrationError {
        MeIntegrationError::numerical(self.method.name(), category, error.to_string())
    }

    fn rebuild(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: Rc<MeDerivativeHandle>,
    ) -> Result<(), MeIntegrationError> {
        let failure = MeNumericalFailure::Construction;
        self.require_width(point, derivatives.as_ref(), failure)?;
        let atol = scaled_absolute_tolerances(&self.setup);
        let h0 = self.setup.initial_step_hint().unwrap_or(1.0e-3);
        let rtol = self.setup.relative_tolerance();
        let initial_derivatives = derivatives
            .derivatives(point.time(), point.states())
            .map_err(MeIntegrationError::from)?;
        let method = self.method;
        let numerical =
            |e: DiffsolError| MeIntegrationError::numerical(method.name(), failure, e.to_string());
        let solver = match method {
            Method::Tsit45 => {
                let problem = build_problem(point, Rc::clone(&derivatives), rtol, atol, h0)
                    .map_err(numerical)?;
                Solver::Explicit(ExplicitCell::try_new(problem, |problem| {
                    let state = initial_state(problem, point, &initial_derivatives, method.order())
                        .map_err(numerical)?;
                    problem.tsit45_solver(state).map_err(numerical)
                })?)
            }
            Method::Esdirk34 | Method::TrBdf2 => {
                if derivatives.state_jacobian_columns().is_none() {
                    return Err(self.numerical(
                        failure,
                        "the component carries no certified state-Jacobian relation to color the \
                         SDIRK Jacobian with",
                    ));
                }
                let problem = build_problem(point, Rc::clone(&derivatives), rtol, atol, h0)
                    .map_err(numerical)?;
                Solver::Sdirk(SdirkCell::try_new(problem, |problem| {
                    let state = initial_state(problem, point, &initial_derivatives, method.order())
                        .map_err(numerical)?;
                    match method {
                        Method::Esdirk34 => problem.esdirk34_solver::<LinearSolver>(state),
                        _ => problem.tr_bdf2_solver::<LinearSolver>(state),
                    }
                    .map_err(numerical)
                })?)
            }
        };
        if derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        self.derivatives = Some(derivatives);
        self.solver = Some(solver);
        self.accepted_interval = None;
        Ok(())
    }

    fn require_width(
        &self,
        point: &MeContinuousPoint,
        derivatives: &MeDerivativeHandle,
        failure: MeNumericalFailure,
    ) -> Result<(), MeIntegrationError> {
        let width = self.setup.state_nominals().len();
        if point.width() == width && derivatives.state_count() == width {
            return Ok(());
        }
        Err(self.numerical(
            failure,
            format!(
                "numerical setup has width {width}, point has width {}, and derivative source has width {}",
                point.width(),
                derivatives.state_count()
            ),
        ))
    }

    fn require_solver(&self) -> Result<&Solver, MeIntegrationError> {
        self.solver.as_ref().ok_or_else(|| {
            self.numerical(
                MeNumericalFailure::Construction,
                "the host has not initialized the Runge-Kutta problem",
            )
        })
    }

    fn require_current_point(&self, request: &MeAdvanceRequest) -> Result<(), MeIntegrationError> {
        let current = request.current();
        let same = with_method!(self.require_solver()?, |_problem, method| {
            let state = method.state();
            state.t.to_bits() == current.time().to_bits() && state.y.as_slice() == current.states()
        });
        if same {
            return Ok(());
        }
        Err(self.numerical(
            MeNumericalFailure::AdvanceExhausted,
            "the retained Runge-Kutta state does not start at the host-issued current point",
        ))
    }

    /// 問題（とヤコビ行列の疎の形）を残したまま、状態だけを `point` に差し替える。
    fn reset_in_place(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: Rc<MeDerivativeHandle>,
    ) -> Result<(), MeIntegrationError> {
        let failure = MeNumericalFailure::Reset;
        self.require_width(point, derivatives.as_ref(), failure)?;
        let values = derivatives
            .derivatives(point.time(), point.states())
            .map_err(MeIntegrationError::from)?;
        let method_id = self.method;
        let solver = self.solver.as_mut().ok_or_else(|| {
            MeIntegrationError::numerical(
                method_id.name(),
                MeNumericalFailure::Reset,
                "the host has not initialized the Runge-Kutta problem",
            )
        })?;
        let numerical = |e: DiffsolError| {
            MeIntegrationError::numerical(method_id.name(), failure, e.to_string())
        };
        with_method_mut!(solver, |problem, method| {
            let fresh =
                initial_state(problem, point, &values, method_id.order()).map_err(numerical)?;
            method.set_state(fresh);
            match method.set_stop_time(point.time()) {
                Ok(())
                | Err(DiffsolError::OdeSolverError(OdeSolverError::StopTimeAtCurrentTime)) => {
                    Ok(())
                }
                Err(error) => Err(numerical(error)),
            }
        })?;
        if derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        self.derivatives = Some(derivatives);
        self.accepted_interval = None;
        Ok(())
    }
}

impl MeIntegratorBackend for DiffsolRk {
    fn initialize(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: MeDerivativeHandle,
    ) -> Result<(), MeIntegrationError> {
        self.rebuild(point, Rc::new(derivatives))
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        self.require_current_point(request)?;
        let latest = request.latest_accepted_time();
        let method_id = self.method;
        let numerical = |e: DiffsolError| {
            MeIntegrationError::numerical(
                method_id.name(),
                MeNumericalFailure::AdvanceExhausted,
                e.to_string(),
            )
        };
        let solver = self.solver.as_mut().ok_or_else(|| {
            MeIntegrationError::numerical(
                method_id.name(),
                MeNumericalFailure::Construction,
                "the host has not initialized the Runge-Kutta problem",
            )
        })?;
        let candidate = with_method_mut!(solver, |_problem, method| {
            method.set_stop_time(latest).map_err(numerical)?;
            let stop = method.step().map_err(numerical)?;
            if matches!(stop, OdeSolverStopReason::RootFound(..)) {
                return Err(MeIntegrationError::numerical(
                    method_id.name(),
                    MeNumericalFailure::AdvanceExhausted,
                    "Diffsol reported a root even though the numerical plugin owns no root function",
                ));
            }
            let state = method.state();
            let states = state.y.as_slice().to_vec();
            if !states.iter().all(|value| value.is_finite()) {
                return Err(MeIntegrationError::numerical(
                    method_id.name(),
                    MeNumericalFailure::AdvanceExhausted,
                    "Diffsol accepted a non-finite endpoint",
                ));
            }
            let order = u32::try_from(method.order()).unwrap_or(u32::MAX).max(1);
            Ok(MeStepCandidate::new(state.t, states, order))
        })?;
        if let Some(derivatives) = self.derivatives.as_ref() {
            if derivatives.has_failed() {
                return Err(MeIntegrationError::DerivativeRefused);
            }
            // 断られた試しの点は NaN で返り、その刻みは誤差の検査か Newton 法で棄てられて小さい刻みでやり直した。
            // 受理した刻みは有限の値で収まったので、断られたことはここで受け取る（元の BDF と同じ）
            derivatives.take_discard();
        }
        self.accepted_interval = Some((request.current().time(), candidate.accepted_time()));
        Ok(candidate)
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        let (start, end) = self.accepted_interval.ok_or_else(|| {
            self.numerical(
                MeNumericalFailure::Interpolation,
                "no accepted Runge-Kutta interval is available",
            )
        })?;
        if states.len() != self.setup.state_nominals().len() {
            return Err(self.numerical(
                MeNumericalFailure::Interpolation,
                "the sampler output width differs from the initialized component width",
            ));
        }
        if !accepted_interval_contains(start, end, time) {
            return Err(self.numerical(
                MeNumericalFailure::Interpolation,
                format!("sample time {time} lies outside [{start}, {end}]"),
            ));
        }
        // 丸めの幅で外れた時刻は端の値にする（端も内側も同じ補間から取る）
        let time = time.clamp(start, end);
        let sampled = with_method!(self.require_solver()?, |_problem, method| method
            .interpolate(time))
        .map_err(|e| self.numerical(MeNumericalFailure::Interpolation, e))?;
        states.copy_from_slice(sampled.as_slice());
        Ok(())
    }

    fn truncate_reset(&mut self, point: &MeContinuousPoint) -> Result<(), MeIntegrationError> {
        let derivatives = self.derivatives.as_ref().cloned().ok_or_else(|| {
            self.numerical(
                MeNumericalFailure::Reset,
                "the host has not initialized the derivative capability",
            )
        })?;
        self.reset_in_place(point, derivatives)
    }
}

/// 問題。ヤコビ行列の疎の形は、build の間だけ部品の状態ヤコビ行列の構造から答える（元の BDF と同じ）。
fn build_problem(
    point: &MeContinuousPoint,
    derivatives: Rc<MeDerivativeHandle>,
    rtol: f64,
    atol: Vec<f64>,
    h0: f64,
) -> Result<Problem, DiffsolError> {
    let rhs_derivatives = Rc::clone(&derivatives);
    let rhs: RhsFn = Box::new(move |state, _parameters, time, output| {
        rhs_derivatives.derivatives_into(time, state.as_slice(), output.as_mut_slice());
    });
    let probing = Rc::new(Cell::new(true));
    let jacobian_probe = Rc::clone(&probing);
    let jacobian: JacobianFn = Box::new(move |state, _parameters, time, seed, output| {
        if jacobian_probe.get() {
            probe_pattern(
                derivatives.state_jacobian_columns().unwrap_or_default(),
                seed.as_slice(),
                output.as_mut_slice(),
            );
            return;
        }
        derivatives.directional_derivative_into(
            time,
            state.as_slice(),
            seed.as_slice(),
            output.as_mut_slice(),
        );
    });
    let problem = OdeBuilder::<Matrix>::new()
        .t0(point.time())
        .h0(h0)
        .rtol(rtol)
        .atol(atol)
        .p(Vec::new())
        .rhs_implicit(rhs, jacobian)
        .init(initial_fn(point), point.width())
        .build();
    probing.set(false);
    let mut problem = problem?;
    problem.ode_options.min_timestep = scaled_step_floor(point.time(), h0);
    Ok(problem)
}

fn initial_fn(point: &MeContinuousPoint) -> InitialFn {
    let initial = point.states().to_vec();
    Box::new(move |_parameters, _time, output| {
        output.as_mut_slice().copy_from_slice(&initial);
    })
}

/// 疎の形の探り: 種を入れた列が、構造の表に載った行だけに届く（元の BDF と同じ）。
fn probe_pattern(columns: &[Vec<usize>], seed: &[f64], output: &mut [f64]) {
    output.fill(0.0);
    for (column, value) in seed.iter().copied().enumerate() {
        if value == 0.0 {
            continue;
        }
        for &row in columns.get(column).map_or(&[][..], Vec::as_slice) {
            output[row] += value.abs();
        }
    }
}

/// diffsol の刻みの絶対の下限。
const ABSOLUTE_STEP_FLOOR: f64 = 1e-13;

/// 刻みの下限（元の BDF の `scaled_step_floor` と同じ: ホストの丸めの幅の 2 倍以上、1e-13 以下）。
fn scaled_step_floor(time: f64, initial_step: f64) -> f64 {
    (2.0 * accepted_step_roundoff(time, 0.0))
        .max(4.0 * f64::EPSILON * initial_step.abs())
        .min(ABSOLUTE_STEP_FLOOR)
}

/// `point` から始める RK の状態（最初の刻みは diffsol の見積もり）。
fn initial_state(
    problem: &Problem,
    point: &MeContinuousPoint,
    derivatives: &[f64],
    order: usize,
) -> Result<RkState<Vector>, DiffsolError> {
    let mut state = RkState::<Vector>::new_without_initialise(problem)?;
    {
        let values = state.as_mut();
        values.y.as_mut_slice().copy_from_slice(point.states());
        values.dy.as_mut_slice().copy_from_slice(derivatives);
        *values.t = point.time();
    }
    state.set_step_size(problem.h0, &problem.atol, problem.rtol, &problem.eqn, order);
    Ok(state)
}

fn scaled_absolute_tolerances(setup: &MeNumericalSetup) -> Vec<f64> {
    let absolute_tolerance = setup.absolute_tolerance();
    setup
        .state_nominals()
        .iter()
        .map(|nominal| (absolute_tolerance * nominal).clamp(f64::MIN_POSITIVE, f64::MAX))
        .collect()
}

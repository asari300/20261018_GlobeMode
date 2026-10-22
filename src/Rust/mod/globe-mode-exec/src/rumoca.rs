//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）の実行系のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイル（とこの下のモジュール）だけ。
//!
//! ソルバごとの呼び方（どれもエラーに型が付いた入口を使う。`simulate_with_diagnostics` はエラーを文字列に
//! するので使わない）:
//!
//! - `bdf` … `build_simulation`（lowering の結果が見える版。パラメータの値を読む）→
//!   `check_prepared_initialization` → `run_prepared_simulation`。Rumoca の MSL のワーカーと同じ道で、
//!   どこで失敗したか（組み立て・初期化・積分）がこちらの呼んだ位置で分かる。
//! - `dopri45` … `rk45::simulate`（組み立てから積分までを 1 回で呼ぶ。初期化・イベント・積分の失敗には
//!   Rumoca が段を付けるので、`bdf` と同じように分けられる。lowering の失敗だけは段が無く、`[EX…]` の印で見分ける）。
//! - `auto` … `bdf` の道で回し、BDF が積分で失敗したか Rumoca のサンプラの食い違いで止まったとき（S012）に
//!   `dopri45` で回し直す（情報 GM-I11002。[`fallback`]）。方向微分が作れないときは切り替えない（S013。ADR-0015）。
//! - 自前の積分器（S012）と wasm32 のすべてのソルバ … lowering と実行を分けて呼ぶ session の道（[`session`]）。
//!
//! パラメータの上書き（`ResolvedSettings::parameters`）は `SimOptions::param_overrides` に渡す。Rumoca が断った
//! もの（`InvalidOverride`、`[EX003]`）は GM-E10006（S10）にする。
//!
//! 失敗の振り分けは、Rumoca が付けた段（`SimFailureStage`・`MeStage`）、エラーの種類（`SimError::kind`、
//! FMI ME のセッションの `MeSessionError`・`MeIntegrationError`・`MeError`）、こちらの呼んだ位置（[`Phase`]）の順に
//! 見る。型に無いもの（Rumoca が文言に描き込んだ時刻・有限でない変数・位置、rk45 の lowering の `[EX…]` の印、
//! `eval_dae_at` の error の文言）だけを文言から読む。
//!
//! プリフライト（S10 の数値の部分）は、[`run`](crate::run) からは組み立ての lowering の結果で行う（bdf・auto と
//! session の道。S014）。lowering を自分で呼ばないネイティブの dopri45（`rk45::simulate`）と、単独の
//! [`preflight`] だけが、プリフライトのために lowering をもう 1 回する。
//!
//! 実行が止まらない Rumoca の診断（`SimResult::diagnostics`。warning レベルの assert の WX001 など）は
//! 警告 GM-W11001 にする。

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{
    ResolvedSettings, ResultParameter, SimRun, SimStats, SolverId, Termination, Trajectory,
    TrajectoryVariable, TunableParameter, VariableRole,
};
use rumoca_ir_dae::Dae;
use rumoca_sim::{
    SimError, SimFailureStage, SimOptions, SimResult, SimSolverMode, SimVariableMeta,
};
use rumoca_solver::fmi_me::{MeError, MeIntegrationError, MeSessionError, MeStage};
use rumoca_solver::{SimDiagnostic, WARNING_ASSERTION_CODE};

use crate::{CompiledModel, ExecStage, Simulated, time_text};

mod plugins;
mod session;

/// GM-W10002 の本文に並べる名前の数の上限。
const LISTED_NAMES: usize = 10;

/// 診断の補足に写す Rumoca の文言の長さの上限（文字数）。MSL のモデルでは式の全文が入ることがある。
const NOTE_CHARS: usize = 2000;

/// 決まった設定を Rumoca の `SimOptions` にする。許容誤差は相対・絶対の両方に同じ値を使う
/// （Rumoca の MSL のハーネスと同じ）。
fn options(settings: &ResolvedSettings) -> SimOptions {
    SimOptions {
        t_start: settings.start_time,
        t_end: settings.stop_time,
        rtol: settings.tolerance,
        atol: settings.tolerance,
        dt: Some(settings.interval),
        max_wall_seconds: settings.max_wall_seconds,
        solver_mode: match settings.solver {
            SolverId::Bdf => SimSolverMode::Bdf,
            SolverId::Dopri45 => SimSolverMode::RkLike,
            SolverId::Auto => SimSolverMode::Auto,
            // 自前の積分器は session の道で回すので、Rumoca の振り分けは使わない
            _ => SimSolverMode::Bdf,
        },
        // tunable なパラメータの上書き。変えられないものは Rumoca が lowering の前に InvalidOverride（EX003）で
        // 断る（[`override_diagnostic`]）
        param_overrides: settings
            .parameters
            .iter()
            .map(|(name, value)| (name.clone(), *value))
            .collect(),
        ..SimOptions::default()
    }
}

// ---------------------------------------------------------------------------------------------
// プリフライト（S10 の数値の部分）
// ---------------------------------------------------------------------------------------------

/// 単独のプリフライト（GM-W10002）。実行の組み立てと同じ lowering を 1 回して評価する。初期化の解き直し
/// （GM-W10004）は実行の t0 の値と比べて決めるので、[`checked_simulate`] だけが行う。
pub(crate) fn preflight(model: &CompiledModel, settings: &ResolvedSettings) -> Vec<Diagnostic> {
    preflight_alone(model, settings, false).0
}

/// 単独のプリフライト。`with_resettle` なら初期化の解き直しも行う（ネイティブの dopri45 の [`checked_simulate`]）。
fn preflight_alone(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    with_resettle: bool,
) -> (Vec<Diagnostic>, Option<Resettle>) {
    let opts = options(settings);
    let dae: &Dae = model.rumoca_dae();
    let probe = catch_unwind(AssertUnwindSafe(|| {
        let lowered =
            rumoca_sim::lower_correlated_for_simulation_with_overrides(dae, &opts).ok()?;
        probe_start(&lowered, settings, with_resettle)
    }));
    preflight_diagnostics(dae, settings, probe.ok().flatten())
}

/// 組み立ての途中の lowering の結果でプリフライトを行う（lowering を増やさない。S014）。初期化の解き直しも行う。
fn preflight_lowered(
    dae: &Dae,
    settings: &ResolvedSettings,
    lowered: &rumoca_phase_solve::LoweredSolveModel<'_>,
) -> (Vec<Diagnostic>, Option<Resettle>) {
    let probe = catch_unwind(AssertUnwindSafe(|| probe_start(lowered, settings, true)));
    preflight_diagnostics(dae, settings, probe.ok().flatten())
}

/// プリフライトの評価（[`probe_start`]）の結果。
struct StartProbe {
    /// start 値での評価（GM-W10002）
    eval: rumoca_sim::EvalAtReport,
    /// 初期化の解き直し（GM-W10004）。行わなかったか、初期化が通らなければ None
    resettle: Option<Resettle>,
}

/// 初期化の結果を初期化系にもう一度入れた結果（[`resettle`]、S018）。
struct Resettle {
    /// 2 回目で動いた変数（名前、1 回目の値、2 回目の値）
    moved: Vec<(String, f64, f64)>,
    /// 動いたと見る差（[`resettle`] と同じ規則を、実行の t0 の値との比較にも使う）
    eps: f64,
}

/// 評価そのものができない（lowering の失敗、パラメータの上書きの誤り、panic。`probe` が None）ときは黙る。
/// 実行の側で同じ失敗を診断にする。
fn preflight_diagnostics(
    dae: &Dae,
    settings: &ResolvedSettings,
    probe: Option<StartProbe>,
) -> (Vec<Diagnostic>, Option<Resettle>) {
    let Some(probe) = probe else {
        return (Vec::new(), None);
    };
    (
        t0_not_finite_diagnostics(dae, settings, &probe.eval),
        probe.resettle,
    )
}

/// start 値（Rumoca の initial_y）を状態に入れ、代数変数を Newton 法で合わせてから、代数変数と der(x) を
/// 評価した結果を診断にする。初期化の残差ではない（fixed = false の状態は初期化で動く）ので、見つけても警告に
/// とどめる。
fn t0_not_finite_diagnostics(
    dae: &Dae,
    settings: &ResolvedSettings,
    probe: &rumoca_sim::EvalAtReport,
) -> Vec<Diagnostic> {
    let mut bad: Vec<String> = probe
        .nonfinite()
        .map(|(_, slot)| format!("{} = {}", slot.name, slot.nonfinite_kind().unwrap_or("NaN")))
        .collect();
    // Rumoca は有限でない値を変数に書かずに評価を止め、その名前を error の文言でだけ知らせる
    let stopped = probe.error.as_deref().and_then(non_finite_in);
    if let Some((name, kind)) = &stopped
        && !bad.iter().any(|b| b.starts_with(&format!("{name} = ")))
    {
        bad.push(format!("{name} = {kind}"));
    }
    if bad.is_empty() {
        return Vec::new();
    }
    let mut message = format!(
        "start 値で評価すると t = {} で式の値が有限にならない: {}",
        time_text(settings.start_time),
        bad.iter()
            .take(LISTED_NAMES)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    if bad.len() > LISTED_NAMES {
        message.push_str(&format!(
            " ほか {} 個（全 {} 個）",
            bad.len() - LISTED_NAMES,
            bad.len()
        ));
    }
    let mut d = Diagnostic::warning(codes::T0_NOT_FINITE, Stage::S10, message).note(
        "初期化では fixed = false の状態や反復変数が動くので、実行では有限になることもある（確定できないので警告）",
    );
    if let Some(error) = &probe.error {
        d = locate(d, dae, error).note(rumoca_note(None, &without_span(error)));
    }
    vec![d]
}

/// 初期化の解き直し（2 回目）で動いた変数のうち、実行の t0 の値（結果の最初の行）も 2 回目の値と違うものを警告に
/// する（GM-W10004）。Rumoca の FMI ME の host は初期化のあとに初期イベントの反復を行い、離散変数が変わる
/// モデルではそこで初期化系を解き直す（MSL の `IMC_Initialize` は 1 回目と 2 回目が違っても、実行は 2 回目の値で
/// 始まる）。そこで 1 回目との差ではなく、実行が実際に始まった値との差で決める。結果に名前が無い変数は数えない。
fn resettle_diagnostic(
    settings: &ResolvedSettings,
    resettle: Option<&Resettle>,
    trajectory: &Trajectory,
) -> Option<Diagnostic> {
    let resettle = resettle?;
    let moved: Vec<(&str, f64, f64)> = resettle
        .moved
        .iter()
        .filter_map(|(name, _, second)| {
            let started = *trajectory.variable(name)?.values.first()?;
            let scale = started.abs().max(second.abs()).max(1.0);
            ((started - second).abs() > resettle.eps * scale).then_some((
                name.as_str(),
                started,
                *second,
            ))
        })
        .collect();
    if moved.is_empty() {
        return None;
    }
    let listed: Vec<String> = moved
        .iter()
        .take(LISTED_NAMES)
        .map(|(name, started, second)| {
            format!(
                "{name} = {} → {}",
                value_text(*started),
                value_text(*second)
            )
        })
        .collect();
    let mut message = format!(
        "初期化の結果を初期化系にもう一度入れると t = {} で値が変わる: {}",
        time_text(settings.start_time),
        listed.join(", ")
    );
    if moved.len() > LISTED_NAMES {
        message.push_str(&format!(
            " ほか {} 個（全 {} 個）",
            moved.len() - LISTED_NAMES,
            moved.len()
        ));
    }
    Some(
        Diagnostic::warning(codes::INIT_RESETTLE_MOVED, Stage::S10, message)
            .note(
                "初期化の結果が初期化方程式を満たしていない恐れがある。初期化方程式が t0 で反転する関係式（if v < 0 など）を読むと、\
                 Rumoca の初期化は関係式の既定値の枝で解き、関係式が反転しても解き直さない（S016 の EngineV6）。実行は左の値で始まっている",
            )
            .note(
                "右（もう一度解いた値）のほうが正しい見込み。初期化方程式が関係式を読まないようにする（例: initType = InitialState にして start 値を与える）と避けられる",
            ),
    )
}

/// 診断に書く値。小数 6 桁で丸め、末尾の 0 を落とす（1.0000000000000002 は 1、563.186235502294 は 563.186236）。
fn value_text(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    let text = format!("{v:.6}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "" | "-" | "-0" => "0".to_string(),
        t => t.to_string(),
    }
}

/// 初期化の反復の上限（rumoca-sim の `prepared_vectors.rs` の `SETTLE_MAX_ITERS` と同じ）。
const SETTLE_MAX_ITERS: usize = 64;

/// 実行時オブジェクトを作り、(1) Rumoca の `eval_dae_at` と同じ評価（start 値の状態、Newton 法の許容誤差 1e-10・
/// 反復 32 回。rumoca-sim の `solve_lowering/probe.rs`）と、(2) 初期化の解き直し（[`resettle`]）を、パラメータの
/// 上書きを反映した lowering の結果で行う（`eval_dae_at` は上書きを見ない）。実行時オブジェクトが作れなければ None。
fn probe_start(
    lowered: &rumoca_phase_solve::LoweredSolveModel<'_>,
    settings: &ResolvedSettings,
    with_resettle: bool,
) -> Option<StartProbe> {
    let model = lowered.model();
    let states = model.state_scalar_count();
    let mut state = vec![0.0; states];
    for (dst, src) in state.iter_mut().zip(model.initial_y.iter().copied()) {
        *dst = src;
    }
    let runtime = rumoca_solver::SolveRuntime::new(model).ok()?;
    let eval = runtime.eval_at(settings.start_time, &state, &model.parameters, 1.0e-10, 32);
    let resettle = with_resettle
        .then(|| resettle(&runtime, lowered, settings, &eval))
        .flatten();
    Some(StartProbe { eval, resettle })
}

/// 初期化系を 2 回解く（S018）。1 回目は Rumoca の初期化（FMI ME の `exit_initialization_mode`、rumoca-sim の
/// `refresh_prepared_vectors`）と同じ順（初期化系 → 関係式の記憶 → 代数変数。許容誤差も同じ）で、実行が始まる点を
/// 作る。2 回目はその点からもう一度初期化系を解く。初期化の結果が初期化方程式を満たしていれば 2 回目は動かない。
/// 動くのは、1 回目が関係式の既定値（false）の枝で解き、その後に関係式の記憶（パラメータの枠にある）が反転したとき
/// （S016 の EngineV6。Rumoca は解き直さない）。1 回目が失敗したら None（実行の側が GM-E11001 にする）。2 回目が
/// 失敗しても None（homotopy のあるモデルでは、解いた点から λ = 0 の掃引をやり直すと収束しないことがある。MSL の
/// `TanksWithOverflow`。失敗は実行が始まる点の誤りを示さない）。
///
/// Rumoca は初期化方程式の残差を評価する関数を公開していない（`eval_initial_residual` は crate の中の trait）ので、
/// 残差ではなく「もう一度解いて動くか」で見る。警告にするかは、実行の t0 の値と比べて決める
/// （[`resettle_diagnostic`]）。
fn resettle(
    runtime: &rumoca_solver::SolveRuntime,
    lowered: &rumoca_phase_solve::LoweredSolveModel<'_>,
    settings: &ResolvedSettings,
    eval: &rumoca_sim::EvalAtReport,
) -> Option<Resettle> {
    let model = lowered.model();
    let t = settings.start_time;
    let tol = settings.tolerance;
    let states = model.state_scalar_count();
    let mut y = model.initial_y.clone();
    let mut p = model.parameters.clone();
    runtime
        .settle_initialization_system(&mut y, &mut p, t, tol, SETTLE_MAX_ITERS)
        .ok()?;
    let state = y[..states.min(y.len())].to_vec();
    runtime
        .update_relation_memory_from_state(t, &state, &mut p, tol, SETTLE_MAX_ITERS)
        .ok()?;
    runtime
        .refresh_algebraic_and_output_slots(t, &mut y, &p, tol, SETTLE_MAX_ITERS)
        .ok()?;
    let (mut y2, mut p2) = (y.clone(), p.clone());
    runtime
        .settle_initialization_system(&mut y2, &mut p2, t, tol, SETTLE_MAX_ITERS)
        .ok()?;
    runtime
        .refresh_algebraic_and_output_slots(t, &mut y2, &p2, tol, SETTLE_MAX_ITERS)
        .ok()?;
    // 動いたと見る差: 許容誤差の 10 倍（1e-6 より小さければ 1e-6）を、値の大きさ（1 より小さければ 1）に掛けたもの
    let eps = (10.0 * tol).max(1.0e-6);
    let moved = y
        .iter()
        .zip(&y2)
        .enumerate()
        .filter(|(_, (a, b))| (*a - *b).abs() > eps * a.abs().max(b.abs()).max(1.0))
        .map(|(i, (a, b))| {
            let name = eval
                .solver_y
                .get(i)
                .map_or_else(|| format!("y[{i}]"), |slot| slot.name.clone());
            (name, *a, *b)
        })
        .collect();
    Some(Resettle { moved, eps })
}

// ---------------------------------------------------------------------------------------------
// tunable なパラメータ
// ---------------------------------------------------------------------------------------------

/// 再コンパイル無しに変えられるパラメータの一覧（既定値は上書きを反映しない値）。
pub(crate) fn tunable_parameters(
    model: &CompiledModel,
) -> Result<Vec<TunableParameter>, Vec<Diagnostic>> {
    let dae: &Dae = model.rumoca_dae();
    let listed = catch_unwind(AssertUnwindSafe(|| {
        let solve = rumoca_sim::lower_for_simulation_with_overrides(dae, &SimOptions::default())?;
        rumoca_sim::build_tunable_parameter_meta(dae, &solve)
    }));
    match listed {
        Ok(Ok(meta)) => Ok(meta
            .into_iter()
            .map(|m| TunableParameter {
                name: m.name,
                default: m.default_value,
                unit: m.unit.filter(|u| !u.is_empty()),
                min: m.min_value,
                max: m.max_value,
                description: m.description.filter(|d| !d.is_empty()),
            })
            .collect()),
        Ok(Err(e)) => {
            let error = session::lowering_error(&e);
            Err(vec![failure_diagnostic(dae, &error, Phase::Build, None)])
        }
        Err(payload) => Err(vec![panic_diagnostic(payload.as_ref(), None)]),
    }
}

/// lowering の結果（上書きを反映した Solve IR）から、実行に使った tunable なパラメータの値を読む。読めなければ空。
fn result_parameters(
    dae: &Dae,
    lowered: &rumoca_phase_solve::LoweredSolveModel<'_>,
) -> Vec<ResultParameter> {
    rumoca_sim::build_tunable_parameter_meta(dae, lowered.model())
        .map(|meta| {
            meta.into_iter()
                .map(|m| ResultParameter {
                    name: m.name,
                    value: m.default_value,
                    unit: m.unit.filter(|u| !u.is_empty()),
                    description: m.description.filter(|d| !d.is_empty()),
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// 実行
// ---------------------------------------------------------------------------------------------

/// こちらが Rumoca を呼んだ位置。Rumoca が段を付けなかった失敗の既定の段に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// lowering と実行系の組み立て（`build_simulation`）
    Build,
    /// 初期化（`check_prepared_initialization`、session の道の `into_lease`）
    Initialization,
    /// 積分（`run_prepared_simulation`、`rk45::simulate`、session の道の `run_to_stop`）
    Integration,
}

/// 1 回の実行の結果と、かかった時間。
struct Outcome {
    result: Result<SimResult, (SimError, Phase)>,
    build_seconds: f64,
    run_seconds: f64,
    /// 実行に使った tunable なパラメータの値（lowering の結果から読む。分からなければ空）
    parameters: Vec<ResultParameter>,
    /// 実行の途中で決まった情報（auto の切り替えの GM-I11002）
    info: Vec<Diagnostic>,
}

impl Outcome {
    fn new(
        result: Result<SimResult, (SimError, Phase)>,
        build_seconds: f64,
        run_seconds: f64,
    ) -> Self {
        Self {
            result,
            build_seconds,
            run_seconds,
            parameters: Vec::new(),
            info: Vec::new(),
        }
    }
}

/// 実行の途中で呼ぶもの: 段の知らせと、組み立ての lowering の結果で行うプリフライト（S014）。
pub(crate) struct Hooks<'a> {
    on_stage: &'a mut dyn FnMut(ExecStage),
    preflight: Preflight,
}

enum Preflight {
    /// 行わない（[`crate::simulate`]）
    Skip,
    /// 最初の lowering の結果で行う
    Wanted,
    /// 行った。診断（GM-W10002）と、実行のあとに t0 の値と比べる初期化の解き直し（GM-W10004）
    Done {
        diagnostics: Vec<Diagnostic>,
        resettle: Option<Resettle>,
    },
}

impl<'a> Hooks<'a> {
    fn new(on_stage: &'a mut dyn FnMut(ExecStage), preflight: bool) -> Self {
        Self {
            on_stage,
            preflight: if preflight {
                Preflight::Wanted
            } else {
                Preflight::Skip
            },
        }
    }

    fn stage(&mut self, stage: ExecStage) {
        (self.on_stage)(stage);
    }

    /// lowering が終わった。プリフライトを待っていれば、その結果で行う（auto の dopri45 は BDF の lowering の
    /// 結果を使う）。続けて組み立ての段に入る。
    fn lowered(
        &mut self,
        dae: &Dae,
        settings: &ResolvedSettings,
        lowered: &rumoca_phase_solve::LoweredSolveModel<'_>,
    ) {
        if matches!(self.preflight, Preflight::Wanted) {
            self.stage(ExecStage::Preflight);
            let (diagnostics, resettle) = preflight_lowered(dae, settings, lowered);
            self.preflight = Preflight::Done {
                diagnostics,
                resettle,
            };
        }
        self.stage(ExecStage::Build);
    }

    /// プリフライトの診断と初期化の解き直し（lowering の前に失敗して行えなかったら空）。
    fn take_preflight(&mut self) -> (Vec<Diagnostic>, Option<Resettle>) {
        match std::mem::replace(&mut self.preflight, Preflight::Skip) {
            Preflight::Done {
                diagnostics,
                resettle,
            } => (diagnostics, resettle),
            _ => (Vec::new(), None),
        }
    }
}

/// Rumoca の呼び方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// ネイティブの bdf・dopri45・auto は rumoca-sim の型付きの入口（Cranelift の JIT）、自前の積分器は
    /// session の道。wasm32 では [`Route::Session`] と同じ
    Default,
    /// どのソルバも session の道（[`session`]。wasm32 の道。ネイティブでは試験で型付きの道と比べる）
    Session,
}

/// 実行する。成功なら結果と、実行が止まらない診断（Rumoca の警告 GM-W11001、auto の切り替え GM-I11002）を返す。
/// 失敗なら、失敗の診断を先頭に置いた診断を返す。
pub(crate) fn simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    route: Route,
) -> Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>> {
    simulate_with(model, settings, route, &mut Hooks::new(&mut |_| {}, false))
}

/// プリフライトと実行（[`crate::run`]）。プリフライトは組み立ての lowering の結果で行う。lowering を自分で
/// 呼ばないネイティブの dopri45 だけは、実行の前に単独のプリフライトをする。
pub(crate) fn checked_simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    on_stage: &mut dyn FnMut(ExecStage),
) -> (Vec<Diagnostic>, Simulated) {
    let mut hooks = Hooks::new(on_stage, true);
    if cfg!(not(target_arch = "wasm32")) && settings.solver == SolverId::Dopri45 {
        hooks.stage(ExecStage::Preflight);
        let (diagnostics, resettle) = preflight_alone(model, settings, true);
        hooks.preflight = Preflight::Done {
            diagnostics,
            resettle,
        };
    }
    let result = simulate_with(model, settings, Route::Default, &mut hooks);
    let (mut diagnostics, resettle) = hooks.take_preflight();
    // 初期化の解き直し（GM-W10004）は、実行が実際に始まった t0 の値と比べて決める。実行が失敗したら出さない
    if let Ok((run, _)) = &result {
        diagnostics.extend(resettle_diagnostic(
            settings,
            resettle.as_ref(),
            &run.trajectory,
        ));
    }
    (diagnostics, result)
}

fn simulate_with(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    route: Route,
    hooks: &mut Hooks<'_>,
) -> Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>> {
    let opts = options(settings);
    let dae: &Dae = model.rumoca_dae();
    // Rumoca の統計（刻みの数、射影のやり直し）はスレッドごとの数なので、実行ごとに 0 に戻す。
    // NaN の追跡（nan_trace）はプロセス全体のスイッチなので触らない
    rumoca_sim::reset_step_counts();
    rumoca_sim::reset_projection_fallbacks();
    // wasm32 は Instant::now を呼ばない session の道だけを使う
    #[cfg(target_arch = "wasm32")]
    let route = {
        let _ = route;
        Route::Session
    };
    // PreparedSimulation は Send でない（Rc / RefCell）。作るのも使い終えるのもこのスレッドの中で行う。
    // wasm32-unknown-unknown は panic=abort なので catch_unwind は何も捕まえない（panic は JS の例外になり、
    // インスタンスは使えなくなる）
    let caught = catch_unwind(AssertUnwindSafe(|| match route {
        #[cfg(not(target_arch = "wasm32"))]
        Route::Default => run_typed(dae, &opts, settings, hooks),
        _ => run_session(dae, &opts, settings, hooks),
    }));
    let mut outcome = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            return Err(vec![panic_diagnostic(
                payload.as_ref(),
                settings.max_wall_seconds,
            )]);
        }
    };
    let info = std::mem::take(&mut outcome.info);
    let run = |trajectory: Trajectory, termination: Termination, states: usize| SimRun {
        model: model.model.clone(),
        settings: settings.clone(),
        stats: SimStats {
            build_seconds: outcome.build_seconds,
            run_seconds: outcome.run_seconds,
            states,
            points: trajectory.time.len(),
        },
        trajectory,
        termination,
        parameters: outcome.parameters.clone(),
    };
    match outcome.result {
        Ok(result) => {
            let termination = match &result.termination {
                Some(t) => Termination::Terminated {
                    time: t.time,
                    message: t.message.clone(),
                },
                None => Termination::StopTime,
            };
            let states = result.n_states;
            let mut diagnostics = info;
            diagnostics.extend(warning_diagnostics(dae, &result.diagnostics));
            Ok((run(trajectory(result), termination, states), diagnostics))
        }
        Err((error, phase)) => match error.kind() {
            // terminate() は失敗ではない。v0.10.2 のバッチの道はこれを返さず結果の termination に入れるが、
            // 返ってきたら時系列の無い成功にする
            SimError::Terminated { time, message } => Ok((
                run(
                    Trajectory::default(),
                    Termination::Terminated {
                        time: *time,
                        message: message.clone(),
                    },
                    0,
                ),
                info,
            )),
            _ => {
                let failed = match override_diagnostic(&error) {
                    Some(d) => d,
                    None => failure_diagnostic(dae, &error, phase, settings.max_wall_seconds),
                };
                let mut diagnostics = vec![failed];
                diagnostics.extend(info);
                Err(diagnostics)
            }
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// ネイティブの既定の道。
fn run_typed(
    dae: &Dae,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    hooks: &mut Hooks<'_>,
) -> Outcome {
    match settings.solver {
        SolverId::Bdf => run_bdf(dae, opts, settings, hooks),
        SolverId::Dopri45 => run_rk(dae, opts, hooks),
        SolverId::Auto => run_auto(
            dae,
            settings,
            hooks,
            |h| run_bdf(dae, opts, settings, h),
            |h| run_rk(dae, opts, h),
        ),
        own => session::run(dae, opts, settings, session::Integrator::Own(own), hooks),
    }
}

/// session の道（wasm32 の道）。bdf・dopri45 は Rumoca の積分器を Rumoca の host に差す。
fn run_session(
    dae: &Dae,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    hooks: &mut Hooks<'_>,
) -> Outcome {
    use session::Integrator;
    let with = |integrator, h: &mut Hooks<'_>| session::run(dae, opts, settings, integrator, h);
    match settings.solver {
        SolverId::Bdf => with(Integrator::RumocaBdf, hooks),
        SolverId::Dopri45 => with(Integrator::RumocaRk45, hooks),
        SolverId::Auto => run_auto(
            dae,
            settings,
            hooks,
            |h| with(Integrator::RumocaBdf, h),
            |h| with(Integrator::RumocaRk45, h),
        ),
        own => with(Integrator::Own(own), hooks),
    }
}

/// 壁時計。wasm32-unknown-unknown では std の `Instant::now` が panic するので測らない（0 秒）。
struct Stopwatch(Option<std::time::Instant>);

impl Stopwatch {
    fn start() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self(Some(std::time::Instant::now()))
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self(None)
        }
    }

    fn seconds(&self) -> f64 {
        self.0.map_or(0.0, |t| t.elapsed().as_secs_f64())
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// BDF（diffsol）。組み立て・初期化・積分を分けて呼ぶ。組み立ての途中の lowering の結果から、実行に使う
/// tunable なパラメータの値を読み、プリフライトを行う（lowering を増やさない）。
fn run_bdf(
    dae: &Dae,
    opts: &SimOptions,
    settings: &ResolvedSettings,
    hooks: &mut Hooks<'_>,
) -> Outcome {
    let started = Stopwatch::start();
    let mut parameters = Vec::new();
    hooks.stage(ExecStage::Lowering);
    let prepared = rumoca_sim::build_simulation_with_stage_timing_and_lowered_model(
        dae,
        opts,
        |_| {},
        |lowered| {
            parameters = result_parameters(dae, lowered);
            hooks.lowered(dae, settings, lowered);
        },
    );
    let build_seconds = started.seconds();
    let prepared = match prepared {
        Ok((p, _)) => p,
        Err(e) => {
            return Outcome {
                parameters,
                ..Outcome::new(Err((e, Phase::Build)), build_seconds, 0.0)
            };
        }
    };
    let started = Stopwatch::start();
    hooks.stage(ExecStage::Initialization);
    if let Err(e) = rumoca_sim::check_prepared_initialization(&prepared) {
        return Outcome {
            parameters,
            ..Outcome::new(
                Err((e, Phase::Initialization)),
                build_seconds,
                started.seconds(),
            )
        };
    }
    hooks.stage(ExecStage::Integration);
    let result =
        rumoca_sim::run_prepared_simulation(&prepared).map_err(|e| (e, Phase::Integration));
    Outcome {
        parameters,
        ..Outcome::new(result, build_seconds, started.seconds())
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Dormand–Prince 5(4)（Rumoca の rk45）。組み立てから積分までを 1 回で呼ぶので、組み立ての時間は分からない
/// （`build_seconds` は 0、全体を `run_seconds` に入れる）。パラメータの値も分からない（空）。段も分けられないので、
/// 積分の段だけを知らせる。
fn run_rk(dae: &Dae, opts: &SimOptions, hooks: &mut Hooks<'_>) -> Outcome {
    let started = Stopwatch::start();
    hooks.stage(ExecStage::Integration);
    let result = rumoca_sim::rk45::simulate(dae, opts).map_err(|e| {
        // lowering の失敗は段の無い SolveIr（"[EX…] …" の形。rk45 は段を付けない）で返る
        let phase = if lowering_tag(&e).is_some() {
            Phase::Build
        } else {
            Phase::Integration
        };
        (e, phase)
    });
    Outcome::new(result, 0.0, started.seconds())
}

/// auto: BDF で回し、[`fallback`] の規則に当たれば dopri45 で回し直す（情報 GM-I11002）。ネイティブの
/// 型付きの道と session の道（wasm32）で同じ規則を使う。
fn run_auto(
    dae: &Dae,
    settings: &ResolvedSettings,
    hooks: &mut Hooks<'_>,
    bdf: impl FnOnce(&mut Hooks<'_>) -> Outcome,
    rk: impl FnOnce(&mut Hooks<'_>) -> Outcome,
) -> Outcome {
    let first = bdf(hooks);
    let (reason, failed) = match &first.result {
        Err((error, phase)) => match fallback(error, *phase) {
            Some(reason) => (
                reason,
                failure_diagnostic(dae, error, *phase, settings.max_wall_seconds),
            ),
            None => return first,
        },
        Ok(_) => return first,
    };
    let mut second = rk(hooks);
    second.build_seconds += first.build_seconds;
    second.run_seconds += first.run_seconds;
    if second.parameters.is_empty() {
        second.parameters = first.parameters;
    }
    second.info.insert(0, fallback_diagnostic(reason, &failed));
    second
}

/// auto が BDF から dopri45 に切り替える理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fallback {
    /// BDF が積分で失敗した（GM-E11002）
    Integration,
    /// BDF の連続な補間（サンプラ）が受け入れた歩の端の点と食い違った（Rumoca の実行時の契約違反。GM-U0005）
    SamplerDisagreement,
}

/// BDF の失敗が dopri45 に切り替える失敗か。方向微分が作れない失敗（GM-U0004）では、組み立て・初期化でも
/// 積分の途中でも切り替えない（Rumoca の Auto は初期化なら RK に切り替えるが、MSL の `GenerationOfFMUs` は
/// RK で終了時刻まで進んで温度が動かない誤った結果になる。ADR-0015、doc/research/msl-sim-failures.md）。
fn fallback(error: &SimError, phase: Phase) -> Option<Fallback> {
    if sampler_disagreement(error) {
        Some(Fallback::SamplerDisagreement)
    } else if classify(error, phase) == Kind::Integration {
        Some(Fallback::Integration)
    } else {
        None
    }
}

fn fallback_diagnostic(reason: Fallback, failed: &Diagnostic) -> Diagnostic {
    let message = match reason {
        Fallback::Integration => "BDF が積分で失敗したので、dopri45 で回し直した",
        Fallback::SamplerDisagreement => {
            "BDF の補間が受け入れた歩の端の点と食い違って止まったので、dopri45 で回し直した"
        }
    };
    let mut d = Diagnostic::info(codes::RUN_SOLVER_FALLBACK, Stage::Run, message)
        .note(format!("BDF の失敗: {}（{}）", failed.message, failed.code));
    for note in &failed.notes {
        d = d.note(note.clone());
    }
    d.note("dopri45 は陽的な方法なので、硬い系では遅い。結果は参照結果などと比べて確かめる")
}

/// Rumoca のサンプラの食い違い（`accepted-interval left endpoint sampler state <i> is <a>, but the checked
/// point carries <b>; …`。rumoca-solver の `fmi_me/root.rs` の `require_endpoint_agreement`）か。型は
/// `MeSessionError::Contract` で、ほかの契約違反と分ける印は文言にしか無い。
fn sampler_disagreement(error: &SimError) -> bool {
    match error.kind() {
        SimError::ModelExchangeSession(MeSessionError::Contract { reason }) => {
            reason.contains(" sampler state ") && reason.contains("the checked point carries")
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------------------------
// パラメータの上書きの誤り
// ---------------------------------------------------------------------------------------------

/// Rumoca が断ったパラメータの上書き（`InvalidOverride`。文言の頭に `[EX003]` が付く）を GM-E10006 にする。
/// 上書きの誤りでなければ None。
fn override_diagnostic(error: &SimError) -> Option<Diagnostic> {
    if lowering_tag(error).as_deref() != Some("EX003") {
        return None;
    }
    let SimError::SolveIr(text) = error.kind() else {
        return None;
    };
    let reason = text.split_once("] ").map_or(text.as_str(), |(_, r)| r);
    // 名前は最初の `…` の中（rumoca-sim の `solve_lowering/overrides.rs` の `tunable_param_overrides`）
    let name = reason
        .split_once('`')
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(name, _)| name.to_string());
    let shown = name.as_deref().unwrap_or("?");
    let message = if reason.contains("is structural or constant") {
        format!(
            "パラメータ {shown} は実行の前に変えられない（構造パラメータ・final・Evaluate = true・定数のどれか）。\
             値を変えるには、モデルを書き換えて再コンパイルする"
        )
    } else if reason.contains("is not a tunable parameter") {
        format!("このモデルに {shown} という変えられる（tunable な）パラメータは無い")
    } else if reason.starts_with("override for") && reason.contains("must be finite") {
        format!("パラメータ {shown} の上書きの値が有限でない")
    } else {
        "パラメータの上書きを実行系（Rumoca）が受け付けない".to_string()
    };
    Some(
        Diagnostic::error(codes::PARAMETER_OVERRIDE_INVALID, Stage::S10, message)
            .note(rumoca_note(None, text)),
    )
}

// ---------------------------------------------------------------------------------------------
// 結果の変換
// ---------------------------------------------------------------------------------------------

/// Rumoca の結果（列ごと。初期の行、出力の格子の行、イベントの左極限と後の行）を中立の時系列にする。
fn trajectory(result: SimResult) -> Trajectory {
    let SimResult {
        times,
        names,
        data,
        variable_meta,
        ..
    } = result;
    let meta: HashMap<&str, &SimVariableMeta> =
        variable_meta.iter().map(|m| (m.name.as_str(), m)).collect();
    let variables = names
        .iter()
        .zip(data)
        .map(|(name, values)| {
            let m = meta.get(name.as_str()).copied();
            TrajectoryVariable {
                name: name.clone(),
                role: role(m),
                unit: m
                    .and_then(|m| m.unit.as_deref())
                    .filter(|u| !u.is_empty())
                    .map(str::to_string),
                description: m
                    .and_then(|m| m.description.as_deref())
                    .filter(|d| !d.is_empty())
                    .map(str::to_string),
                values,
            }
        })
        .collect();
    Trajectory {
        time: times,
        variables,
    }
}

/// Rumoca の変数の役割（rumoca-phase-solve の `role_name`）を中立の役割にする。
fn role(meta: Option<&SimVariableMeta>) -> VariableRole {
    let Some(meta) = meta else {
        return VariableRole::Algebraic;
    };
    if meta.is_state {
        return VariableRole::State;
    }
    match meta.role.as_str() {
        "state" => VariableRole::State,
        "output" => VariableRole::Output,
        "input" => VariableRole::Input,
        "discrete-real" | "discrete-valued" => VariableRole::Discrete,
        "parameter" | "constant" => VariableRole::Parameter,
        _ => VariableRole::Algebraic,
    }
}

// ---------------------------------------------------------------------------------------------
// 失敗の振り分け
// ---------------------------------------------------------------------------------------------

/// 失敗の分類（GlobeMode のコードを決める単位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Initialization,
    Integration,
    Event,
    Assertion,
    Timeout,
    /// 実行系が未対応（lowering・構造解析・組み立て、ソルバの方式、方向微分）
    Unsupported,
    /// 実行系の内部エラー（実行時の契約違反など）
    Internal,
    /// 実行の設定（区間・許容誤差・出力の間隔・予算）を Rumoca のセッションが受け付けない。
    /// 設定は `globe_mode_sim::settings::resolve` で検査済みのはずなので、内部エラーと同じコードにする
    Options,
}

/// 失敗の詳細（時刻、assert の文言、有限でない変数、Rumoca の段の名前）。型にあるものは型から、
/// 無いものは文言から読む（[`detail`]）。
#[derive(Debug, Default, PartialEq)]
struct Detail {
    time: Option<f64>,
    assertion: Option<String>,
    non_finite: Option<(String, String)>,
    stage: Option<String>,
}

fn failure_diagnostic(
    dae: &Dae,
    error: &SimError,
    phase: Phase,
    max_wall_seconds: Option<f64>,
) -> Diagnostic {
    let text = error.to_string();
    let detail = detail(error, &text);
    let at = |t: Option<f64>| {
        t.map(|t| format!("t = {} で", time_text(t)))
            .unwrap_or_default()
    };
    // 有限でない値で止まったときは、その変数を本文に出す
    let cause = detail
        .non_finite
        .as_ref()
        .map(|(name, kind)| format!("（{name} の値が {kind} になった）"))
        .unwrap_or_default();
    let d = match classify(error, phase) {
        Kind::Initialization => Diagnostic::error(
            codes::RUN_INIT_FAILED,
            Stage::Run,
            if cause.is_empty() {
                "初期化に失敗した（初期化系が解けない）".to_string()
            } else {
                format!("初期化に失敗した{cause}")
            },
        ),
        Kind::Integration => Diagnostic::error(
            codes::RUN_INTEGRATION_FAILED,
            Stage::Run,
            format!("{}積分に失敗した{cause}", at(detail.time)),
        ),
        Kind::Event => Diagnostic::error(
            codes::RUN_EVENT_FAILED,
            Stage::Run,
            format!("{}イベントの反復が収束しない", at(detail.time)),
        ),
        Kind::Assertion => {
            let at = detail
                .time
                .map(|t| format!("t = {} で ", time_text(t)))
                .unwrap_or_default();
            Diagnostic::error(
                codes::RUN_ASSERTION,
                Stage::Run,
                format!(
                    "{at}assert が失敗した: {}",
                    detail.assertion.as_deref().unwrap_or_default()
                ),
            )
        }
        Kind::Timeout => timeout_diagnostic(max_wall_seconds),
        Kind::Unsupported => Diagnostic::error(
            codes::RUN_UNSUPPORTED,
            Stage::Run,
            format!(
                "実行系（Rumoca）がこのモデルに未対応: {}で止まった",
                unsupported_where(error, phase)
            ),
        ),
        Kind::Internal => Diagnostic::error(
            codes::RUN_INTERNAL,
            Stage::Run,
            "実行系（Rumoca）の内部エラー（実行時の契約違反）",
        ),
        Kind::Options => Diagnostic::error(
            codes::RUN_INTERNAL,
            Stage::Run,
            "実行系（Rumoca）が実行の設定を受け付けない",
        ),
    };
    let d = locate(d, dae, &text);
    d.note(rumoca_note(detail.stage.as_deref(), &without_span(&text)))
}

fn timeout_diagnostic(max_wall_seconds: Option<f64>) -> Diagnostic {
    Diagnostic::error(
        codes::RUN_TIMEOUT,
        Stage::Run,
        match max_wall_seconds {
            Some(s) => format!("時間切れ（壁時計の予算 {s} 秒を超えた）"),
            None => "時間切れ（壁時計の予算を超えた）".to_string(),
        },
    )
}

/// 失敗を分類する。Rumoca が付けた段 → エラーの種類 → こちらの呼んだ位置の順に見る。
fn classify(error: &SimError, phase: Phase) -> Kind {
    match error.kind() {
        SimError::Timeout { .. } => Kind::Timeout,
        SimError::AssertionFailed { .. } => Kind::Assertion,
        SimError::UnsupportedSolverMode { .. }
        | SimError::EmptySystem
        | SimError::DirectionalDerivativeUnavailable { .. } => Kind::Unsupported,
        SimError::RuntimeContract { .. } | SimError::ExecutionPolicyContradiction { .. } => {
            Kind::Internal
        }
        SimError::ModelExchangeSession(session) => classify_session(session, phase),
        // SolveIr / SolverError は段で分ける（段が無ければ、こちらの呼んだ位置）
        SimError::SolveIr(_) | SimError::SolverError(_) => {
            match error.stage().unwrap_or(match phase {
                Phase::Build if lowering_tag(error).is_some() => SimFailureStage::SolveLowering,
                Phase::Build => SimFailureStage::BackendBuild,
                Phase::Initialization => SimFailureStage::Initialization,
                Phase::Integration => SimFailureStage::Integration,
            }) {
                SimFailureStage::SolveLowering
                | SimFailureStage::StructuralAnalysis
                | SimFailureStage::BackendBuild => Kind::Unsupported,
                SimFailureStage::Initialization => Kind::Initialization,
                SimFailureStage::EventIteration => Kind::Event,
                SimFailureStage::ManifoldProjection
                | SimFailureStage::TargetIsolation
                | SimFailureStage::Integration => Kind::Integration,
            }
        }
        // kind() は段の注釈を剥がすので Staged は来ない。Terminated は失敗にしない（simulate が先に拾う）
        SimError::Staged { .. } | SimError::Terminated { .. } => Kind::Internal,
    }
}

/// FMI ME のセッションのエラー（Rumoca の実行の本体。bdf も rk45 もこれを返す）を分類する。
fn classify_session(session: &MeSessionError, phase: Phase) -> Kind {
    match session {
        MeSessionError::Timeout { .. } => Kind::Timeout,
        MeSessionError::Component(e) => classify_component(e, phase),
        MeSessionError::Integration(e) => classify_integration(e, phase),
        MeSessionError::EventIterationDiverged { .. }
        | MeSessionError::RootApplicationUnavailable { .. } => Kind::Event,
        MeSessionError::Options { .. } => Kind::Options,
        // ホストの契約違反・記憶域・受け入れた点を失った（試していた操作より復元の失敗が優先。rumoca-solver の
        // 約束）・使えなくなったセッションなど
        MeSessionError::Allocation { .. }
        | MeSessionError::Contract { .. }
        | MeSessionError::RootScanUnrepresentable { .. }
        | MeSessionError::PluginArity { .. }
        | MeSessionError::AcceptedPointLost { .. }
        | MeSessionError::SessionNotReusable { .. } => Kind::Internal,
    }
}

/// 積分器（diffsol・rk45）の失敗。初期化の中（積分器の準備）でも起きる。
fn classify_integration(error: &MeIntegrationError, phase: Phase) -> Kind {
    match error {
        MeIntegrationError::Component(e) => classify_component(e, phase),
        MeIntegrationError::ComponentDiscard { .. }
        | MeIntegrationError::StepSizeUnderflow { .. }
        | MeIntegrationError::Numerical { .. } => match phase {
            Phase::Initialization => Kind::Initialization,
            Phase::Build | Phase::Integration => Kind::Integration,
        },
        MeIntegrationError::Contract { .. }
        | MeIntegrationError::DerivativeCapabilityInactive { .. }
        | MeIntegrationError::DerivativeRefused
        | MeIntegrationError::Allocation { .. } => Kind::Internal,
    }
}

/// FMI の部品（モデルの評価）の失敗。評価の失敗は、部品が付けた段（MeStage）で分ける。
fn classify_component(error: &MeError, phase: Phase) -> Kind {
    match error.kind() {
        MeError::Assertion { .. } => Kind::Assertion,
        MeError::NoContinuousStates
        | MeError::UnsupportedModel { .. }
        | MeError::DirectionalDerivativeUnavailable { .. } => Kind::Unsupported,
        MeError::Contract { .. } | MeError::Allocation { .. } | MeError::Staged { .. } => {
            Kind::Internal
        }
        MeError::Evaluation { .. } | MeError::NonFiniteDerivative { .. } => match error.stage() {
            Some(MeStage::Instantiate) => Kind::Unsupported,
            Some(MeStage::Initialization) => Kind::Initialization,
            Some(MeStage::EventIteration) => Kind::Event,
            Some(MeStage::ManifoldProjection | MeStage::Integration) => Kind::Integration,
            None => match phase {
                Phase::Build => Kind::Unsupported,
                Phase::Initialization => Kind::Initialization,
                Phase::Integration => Kind::Integration,
            },
        },
    }
}

/// 失敗の詳細を読む。型にあるもの（段、assert の時刻と文言、積分器の時刻、状態の名前）は型から、無いもの
/// （評価の失敗の文言に描き込まれた時刻と変数）は文言から。
fn detail(error: &SimError, text: &str) -> Detail {
    let mut d = Detail {
        stage: error.stage().map(|s| s.to_string()),
        ..Detail::default()
    };
    match error.kind() {
        SimError::AssertionFailed { time, message } => {
            d.time = Some(*time);
            d.assertion = Some(message.clone());
        }
        SimError::ModelExchangeSession(session) => match session {
            MeSessionError::Component(e) => component_detail(e, &mut d),
            MeSessionError::Integration(e) => match e {
                MeIntegrationError::Component(c) => component_detail(c, &mut d),
                MeIntegrationError::ComponentDiscard { time } => d.time = Some(*time),
                MeIntegrationError::StepSizeUnderflow { from_time, .. } => {
                    d.time = Some(*from_time)
                }
                _ => {}
            },
            MeSessionError::EventIterationDiverged { time, .. }
            | MeSessionError::RootApplicationUnavailable { time, .. }
            | MeSessionError::AcceptedPointLost { time, .. } => d.time = Some(*time),
            _ => {}
        },
        _ => {}
    }
    if d.time.is_none() {
        d.time = time_in(text);
    }
    if d.non_finite.is_none() {
        d.non_finite = non_finite_in(text);
    }
    d
}

fn component_detail(error: &MeError, d: &mut Detail) {
    if d.stage.is_none() {
        d.stage = error.stage().map(|s| me_stage_name(s).to_string());
    }
    match error.kind() {
        MeError::Assertion { time, message } => {
            d.time = Some(*time);
            d.assertion = Some(message.clone());
        }
        MeError::NonFiniteDerivative { state_name } => {
            d.non_finite = Some((format!("der({state_name})"), "NaN / inf".to_string()));
        }
        _ => {}
    }
}

/// MeStage の名前（`rumoca: [段]` の補足に使う。SimFailureStage の表示に合わせる）。
const fn me_stage_name(stage: MeStage) -> &'static str {
    match stage {
        MeStage::Instantiate => "Instantiate",
        MeStage::Initialization => "Initialization",
        MeStage::EventIteration => "EventIteration",
        MeStage::ManifoldProjection => "ManifoldProjection",
        MeStage::Integration => "Integration",
    }
}

/// 未対応で止まった場所の説明。
fn unsupported_where(error: &SimError, phase: Phase) -> &'static str {
    // 方向微分は積分器の準備で作るが、積分の途中（代数変数の射影の感度）でも作り直す。途中で作れなくなったものを
    // 「準備」と書かない（doc/research/msl-sim-failures.md）
    let derivative = if during_integration(error, phase) {
        "積分の途中の方向微分（ヤコビアン）の計算"
    } else {
        "方向微分（ヤコビアン）の準備"
    };
    match error.kind() {
        SimError::UnsupportedSolverMode { .. } => "ソルバの方式の選択",
        SimError::EmptySystem => "解く方程式の無い系",
        SimError::DirectionalDerivativeUnavailable { .. } => derivative,
        SimError::ModelExchangeSession(MeSessionError::Component(e))
            if matches!(e.kind(), MeError::DirectionalDerivativeUnavailable { .. }) =>
        {
            derivative
        }
        _ => match error.stage() {
            Some(SimFailureStage::SolveLowering) => "Solve IR への変換",
            Some(SimFailureStage::StructuralAnalysis) => {
                "構造解析（インデックスの低減・マッチング・tearing）"
            }
            Some(SimFailureStage::BackendBuild) => "実行系の組み立て",
            _ if lowering_tag(error).is_some() => "Solve IR への変換",
            _ => match phase {
                Phase::Build => "実行系の組み立て",
                Phase::Initialization => "初期化の準備",
                Phase::Integration => "積分の準備",
            },
        },
    }
}

/// 積分の途中の失敗か。Rumoca が付けた段（`SimFailureStage`、部品の `MeStage`）を先に見て、無ければこちらの
/// 呼んだ位置。
fn during_integration(error: &SimError, phase: Phase) -> bool {
    let me_stage = match error.kind() {
        SimError::ModelExchangeSession(MeSessionError::Component(e)) => e.stage(),
        SimError::ModelExchangeSession(MeSessionError::Integration(
            MeIntegrationError::Component(e),
        )) => e.stage(),
        _ => None,
    };
    match (error.stage(), me_stage) {
        (Some(stage), _) => matches!(
            stage,
            SimFailureStage::Integration
                | SimFailureStage::ManifoldProjection
                | SimFailureStage::TargetIsolation
        ),
        (None, Some(stage)) => matches!(stage, MeStage::Integration | MeStage::ManifoldProjection),
        (None, None) => phase == Phase::Integration,
    }
}

/// lowering・準備の失敗に Rumoca が付ける `[EX…]` などのコード（rumoca-sim の `diagnostic_sim_error`）。
/// rk45 の道は lowering の失敗を段の無い `SolveIr` の文言にするので、ここだけは文言から読む。
fn lowering_tag(error: &SimError) -> Option<String> {
    let SimError::SolveIr(text) = error.kind() else {
        return None;
    };
    let rest = text.strip_prefix('[')?;
    let (tag, _) = rest.split_once("] ")?;
    (!tag.is_empty() && tag.chars().all(|c| c.is_ascii_alphanumeric())).then(|| tag.to_string())
}

/// 文言の中の最初の `t=<数>` の数。評価の失敗（`MeError::Evaluation`）は文言しか持たないので、時刻はここから読む。
fn time_in(text: &str) -> Option<f64> {
    let mut rest = text;
    while let Some(i) = rest.find("t=") {
        let before = rest[..i].chars().last();
        let after = &rest[i + 2..];
        let number: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
            .collect();
        // "start=" などの語の途中は飛ばす。文末の "." は数に含めない
        if !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && let Ok(t) = number.trim_end_matches('.').parse::<f64>()
        {
            return Some(t);
        }
        rest = after;
    }
    None
}

/// Rumoca が有限でない値で評価を止めたときの、変数の名前と種類（`inf` / `NaN`）。rumoca-solver の
/// `RuntimeSolveError::NonFiniteValue`（"non-finite (inf) value computed for `y`"）と `NonFiniteDerivative`
/// （"non-finite derivative evaluation for state 'x'"）は、実行系に届く前に文言（`MeError::Evaluation`、
/// `SimError::SolveIr`、`eval_dae_at` の error）に描かれるので、文言から読む。
fn non_finite_in(text: &str) -> Option<(String, String)> {
    if let Some(rest) = text.split_once("non-finite (").map(|(_, r)| r) {
        let (kind, rest) = rest.split_once(") value computed for `")?;
        let (name, _) = rest.split_once('`')?;
        return Some((name.to_string(), kind.to_string()));
    }
    let rest = text
        .split_once("non-finite derivative evaluation for state '")?
        .1;
    let (name, _) = rest.split_once('\'')?;
    Some((format!("der({name})"), "NaN / inf".to_string()))
}

/// 文言の中の位置（`@ Span { source: SourceId(N), start: BytePos(a), end: BytePos(b) }`）。位置は Rumoca が
/// 文言に描き込んだもの（型の Span は残らない）なので、文言から読む。
fn span_in(text: &str) -> Option<(u64, usize, usize)> {
    let rest = text.split_once("Span { source: SourceId(")?.1;
    let (id, rest) = rest.split_once("), start: BytePos(")?;
    let (start, rest) = rest.split_once("), end: BytePos(")?;
    let (end, _) = rest.split_once(')')?;
    Some((id.parse().ok()?, start.parse().ok()?, end.parse().ok()?))
}

/// 文言の中の位置を診断に付ける。
fn locate(d: Diagnostic, dae: &Dae, text: &str) -> Diagnostic {
    match span_in(text) {
        Some((source, start, end)) => located(d, dae, source, start, end),
        None => d,
    }
}

/// 位置（SourceId の数とバイト範囲）を、DAE のソースの表でファイル名とバイト範囲に直して診断に付ける。
/// 範囲がその本文に収まらなければファイル名だけ。
fn located(d: Diagnostic, dae: &Dae, source: u64, start: usize, end: usize) -> Diagnostic {
    let map = dae.source_map();
    let Some((name, id)) = map.source_ids().into_iter().find(|(_, id)| id.0 == source) else {
        return d;
    };
    let range = map
        .get_source(id)
        .filter(|(_, content)| start < end && end <= content.len())
        .map(|_| TextRange::new(start as u32, end as u32));
    d.at(name, range)
}

/// 文言から位置（` @ Span { … }`）を除く（位置は診断の file / range に入れる）。
fn without_span(text: &str) -> String {
    match text.split_once(" @ Span { ") {
        Some((head, rest)) => match rest.split_once('}') {
            Some((_, tail)) => format!("{head}{tail}"),
            None => head.to_string(),
        },
        None => text.to_string(),
    }
}

/// Rumoca の文言を補足にする（`rumoca: [段] 文言`。長すぎるものは切る）。
fn rumoca_note(stage: Option<&str>, text: &str) -> String {
    let text = text.trim();
    let mut short: String = text.chars().take(NOTE_CHARS).collect();
    if short.len() < text.len() {
        short.push_str(" …");
    }
    match stage {
        Some(s) => format!("rumoca: [{s}] {short}"),
        None => format!("rumoca: {short}"),
    }
}

// ---------------------------------------------------------------------------------------------
// 実行が止まらない警告
// ---------------------------------------------------------------------------------------------

/// Rumoca の止めない診断（`SimResult::diagnostics`。v0.10.2 は warning レベルの assert の WX001 だけ）を
/// 警告 GM-W11001 にする。位置は Rumoca の Span（型で持つ）から付ける。
fn warning_diagnostics(dae: &Dae, diagnostics: &[SimDiagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|w| {
            let when = format!("t = {} で", time_text(w.time));
            let message = if w.code == WARNING_ASSERTION_CODE {
                format!(
                    "{when} assert（AssertionLevel.warning）の条件が破れた: {}",
                    w.message
                )
            } else {
                format!("{when}実行系（Rumoca）の警告（{}）: {}", w.code, w.message)
            };
            let mut d = Diagnostic::warning(codes::RUN_WARNING, Stage::Run, message);
            if !w.span.is_dummy() {
                d = located(d, dae, w.span.source.0, w.span.start.0, w.span.end.0);
            }
            // origin（v0.10.2 は "source DAE object" だけ）は位置の代わりにならないので写さない
            let original = format!("{}: {}", w.code, w.message);
            d.note(rumoca_note(None, &original))
        })
        .collect()
}

/// panic を診断にする。Rumoca の積分器の期限切れの panic（`SolverTimeoutPanic`）は時間切れ、そのほかは内部エラー。
fn panic_diagnostic(
    payload: &(dyn std::any::Any + Send),
    max_wall_seconds: Option<f64>,
) -> Diagnostic {
    if rumoca_sim::is_solver_timeout_panic(payload) {
        return timeout_diagnostic(max_wall_seconds);
    }
    let message = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default();
    Diagnostic::error(
        codes::RUN_INTERNAL,
        Stage::Run,
        "実行系（Rumoca）の内部エラー（panic）",
    )
    .note(rumoca_note(None, &format!("panic: {message}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumoca_sim::SimulationDiagnosticError;

    #[test]
    fn time_is_read_from_rumoca_messages() {
        assert_eq!(
            time_in("event iteration at t=0.5000000000000001 cycles among relations []"),
            Some(0.5000000000000001)
        );
        assert_eq!(
            time_in("bdf step size underflowed while advancing from t=0.25 toward t=0.3"),
            Some(0.25)
        );
        assert_eq!(time_in("diverged at t=1e-3."), Some(1e-3));
        // 語の途中（start=）は時刻ではない
        assert_eq!(time_in("x start=2 failed"), None);
        assert_eq!(time_in("no time here"), None);
    }

    #[test]
    fn non_finite_values_are_read_from_rumoca_messages() {
        assert_eq!(
            non_finite_in(
                "non-finite (inf) value computed for `y` @ Span { source: SourceId(1), start: BytePos(3), end: BytePos(9) }"
            ),
            Some(("y".to_string(), "inf".to_string()))
        );
        assert_eq!(
            non_finite_in("non-finite derivative evaluation for state 'body.v[2]'"),
            Some(("der(body.v[2])".to_string(), "NaN / inf".to_string()))
        );
        assert_eq!(
            non_finite_in("initial variable projection did not satisfy"),
            None
        );
    }

    #[test]
    fn spans_are_read_and_removed() {
        let text = "non-finite (NaN) value computed for `y` @ Span { source: SourceId(42), start: BytePos(503), end: BytePos(509) }";
        assert_eq!(span_in(text), Some((42, 503, 509)));
        assert_eq!(
            without_span(text),
            "non-finite (NaN) value computed for `y`"
        );
        assert_eq!(span_in("no span"), None);
        assert_eq!(without_span("no span"), "no span");
    }

    #[test]
    fn assertion_time_and_message_are_read_from_the_types() {
        let typed = SimError::AssertionFailed {
            time: 0.25,
            message: "typed".to_string(),
        };
        let d = detail(&typed, &typed.to_string());
        assert_eq!(
            (d.time, d.assertion.as_deref()),
            (Some(0.25), Some("typed"))
        );
        // セッションの中の部品の assert（MeError::Assertion。段は EventIteration）
        let session = SimError::from(MeSessionError::Component(
            MeError::Assertion {
                time: 0.5,
                message: "x が上限: 0.5".to_string(),
            }
            .at_stage(MeStage::EventIteration),
        ));
        let d = detail(&session, &session.to_string());
        assert_eq!(
            d,
            Detail {
                time: Some(0.5),
                assertion: Some("x が上限: 0.5".to_string()),
                non_finite: None,
                stage: Some("EventIteration".to_string()),
            }
        );
        assert_eq!(classify(&session, Phase::Integration), Kind::Assertion);
        // 積分器の時刻と、状態の名前
        let underflow = SimError::from(MeSessionError::Integration(
            MeIntegrationError::StepSizeUnderflow {
                method: "bdf",
                from_time: 0.25,
                to_time: 0.3,
            },
        ));
        assert_eq!(detail(&underflow, "").time, Some(0.25));
        let derivative = SimError::from(MeSessionError::Component(
            MeError::NonFiniteDerivative {
                state_name: "x".to_string(),
            }
            .at_stage(MeStage::Integration),
        ));
        assert_eq!(
            detail(&derivative, "").non_finite,
            Some(("der(x)".to_string(), "NaN / inf".to_string()))
        );
    }

    #[test]
    fn typed_errors_are_classified_by_stage_then_kind_then_position() {
        let staged = |stage| SimError::SolveIr("boom".to_string()).at_stage(stage);
        let kind = |e: &SimError, phase| classify(e, phase);
        assert_eq!(
            kind(&staged(SimFailureStage::StructuralAnalysis), Phase::Build),
            Kind::Unsupported
        );
        assert_eq!(
            kind(&staged(SimFailureStage::Initialization), Phase::Integration),
            Kind::Initialization
        );
        assert_eq!(
            kind(&staged(SimFailureStage::EventIteration), Phase::Integration),
            Kind::Event
        );
        assert_eq!(
            kind(
                &staged(SimFailureStage::TargetIsolation),
                Phase::Integration
            ),
            Kind::Integration
        );
        // 段が無ければ、こちらの呼んだ位置で決める
        let plain = SimError::SolverError("boom".to_string());
        assert_eq!(kind(&plain, Phase::Initialization), Kind::Initialization);
        assert_eq!(kind(&plain, Phase::Integration), Kind::Integration);
        assert_eq!(kind(&plain, Phase::Build), Kind::Unsupported);
        // rk45 の lowering の失敗（段の無い "[EX…]"）は未対応
        let lowering = SimError::SolveIr("[EX002] cannot lower".to_string());
        assert_eq!(lowering_tag(&lowering).as_deref(), Some("EX002"));
        assert_eq!(kind(&lowering, Phase::Build), Kind::Unsupported);
        assert_eq!(
            unsupported_where(&lowering, Phase::Build),
            "Solve IR への変換"
        );
        // 種類で決まるもの（段の注釈は剥がして見る）
        let timeout = SimError::Timeout { seconds: 1.0 }.at_stage(SimFailureStage::Initialization);
        assert_eq!(kind(&timeout, Phase::Initialization), Kind::Timeout);
        assert_eq!(
            kind(&SimError::EmptySystem, Phase::Build),
            Kind::Unsupported
        );
        let contract = SimError::RuntimeContract {
            reason: "lease".to_string(),
        };
        assert_eq!(kind(&contract, Phase::Integration), Kind::Internal);
    }

    #[test]
    fn session_errors_are_classified_by_their_types() {
        use rumoca_solver::fmi_me::{MeNumericalFailure, MeSessionLoss};
        let k = |session: MeSessionError, phase| classify(&SimError::from(session), phase);
        let evaluation = |stage| {
            MeSessionError::Component(
                MeError::Evaluation {
                    message: "boom".to_string(),
                }
                .at_stage(stage),
            )
        };
        assert_eq!(
            k(evaluation(MeStage::Initialization), Phase::Integration),
            Kind::Initialization
        );
        assert_eq!(
            k(evaluation(MeStage::EventIteration), Phase::Integration),
            Kind::Event
        );
        assert_eq!(
            k(
                evaluation(MeStage::ManifoldProjection),
                Phase::Initialization
            ),
            Kind::Integration
        );
        assert_eq!(
            k(evaluation(MeStage::Instantiate), Phase::Build),
            Kind::Unsupported
        );
        // 段の無い評価の失敗は、こちらの呼んだ位置
        let plain = || {
            MeSessionError::Component(MeError::Evaluation {
                message: "boom".to_string(),
            })
        };
        assert_eq!(k(plain(), Phase::Initialization), Kind::Initialization);
        assert_eq!(k(plain(), Phase::Integration), Kind::Integration);
        assert_eq!(
            k(
                MeSessionError::Component(MeError::UnsupportedModel {
                    reason: "x".to_string()
                }),
                Phase::Build
            ),
            Kind::Unsupported
        );
        assert_eq!(
            k(
                MeSessionError::Component(MeError::Contract {
                    reason: "x".to_string()
                }),
                Phase::Integration
            ),
            Kind::Internal
        );
        assert_eq!(
            k(
                MeSessionError::EventIterationDiverged {
                    time: 0.5,
                    limit: 100
                },
                Phase::Integration
            ),
            Kind::Event
        );
        assert_eq!(
            k(
                MeSessionError::Options {
                    reason: "x".to_string()
                },
                Phase::Initialization
            ),
            Kind::Options
        );
        assert_eq!(
            k(
                MeSessionError::SessionNotReusable {
                    loss: MeSessionLoss::Restart
                },
                Phase::Integration
            ),
            Kind::Internal
        );
        assert_eq!(
            k(MeSessionError::Timeout { seconds: 1.0 }, Phase::Integration),
            Kind::Timeout
        );
        // 積分器の数値の失敗は、初期化の中なら初期化の失敗
        let numerical = || {
            MeSessionError::Integration(MeIntegrationError::Numerical {
                method: "bdf",
                category: MeNumericalFailure::AdvanceExhausted,
                detail: String::new(),
            })
        };
        assert_eq!(k(numerical(), Phase::Initialization), Kind::Initialization);
        assert_eq!(k(numerical(), Phase::Integration), Kind::Integration);
        assert_eq!(
            k(
                MeSessionError::Integration(MeIntegrationError::DerivativeRefused),
                Phase::Integration
            ),
            Kind::Internal
        );
        // 方向微分が作れない（実行系が未対応。auto も切り替えない）
        let unavailable = SimError::from(MeSessionError::Component(
            MeError::DirectionalDerivativeUnavailable {
                reason: "x".to_string(),
            }
            .at_stage(MeStage::Initialization),
        ));
        assert_eq!(
            classify(&unavailable, Phase::Initialization),
            Kind::Unsupported
        );
    }

    #[test]
    fn auto_falls_back_to_dopri45_only_for_the_listed_failures() {
        use rumoca_solver::fmi_me::MeNumericalFailure;
        let session = |e: MeSessionError| SimError::from(e);
        // BDF が積分で失敗した（GM-E11002）
        let numerical = session(MeSessionError::Integration(MeIntegrationError::Numerical {
            method: "bdf",
            category: MeNumericalFailure::AdvanceExhausted,
            detail: String::new(),
        }));
        assert_eq!(
            fallback(&numerical, Phase::Integration),
            Some(Fallback::Integration)
        );
        let underflow = session(MeSessionError::Integration(
            MeIntegrationError::StepSizeUnderflow {
                method: "bdf",
                from_time: 0.25,
                to_time: 0.3,
            },
        ));
        assert_eq!(
            fallback(&underflow, Phase::Integration),
            Some(Fallback::Integration)
        );
        // 段の付いた積分の失敗（代数変数の射影の収束など）
        let staged = SimError::SolveIr("[EX004] singular".to_string())
            .at_stage(SimFailureStage::Integration);
        assert_eq!(
            fallback(&staged, Phase::Integration),
            Some(Fallback::Integration)
        );
        // サンプラの食い違い（GM-U0005）。ほかの契約違反では切り替えない
        let sampler = session(MeSessionError::Contract {
            reason: "accepted-interval left endpoint sampler state 1 is 31.2994, but the checked \
                     point carries 31.2995; the disagreement exceeds 0.0000312"
                .to_string(),
        });
        assert_eq!(classify(&sampler, Phase::Integration), Kind::Internal);
        assert_eq!(
            fallback(&sampler, Phase::Integration),
            Some(Fallback::SamplerDisagreement)
        );
        let other = session(MeSessionError::Contract {
            reason: "lease".to_string(),
        });
        assert_eq!(fallback(&other, Phase::Integration), None);
        // 方向微分: 初期化でも積分の途中でも切り替えない（S013。Rumoca の Auto は初期化なら切り替える）
        let derivative = |stage| {
            session(MeSessionError::Component(
                MeError::DirectionalDerivativeUnavailable {
                    reason: "singular".to_string(),
                }
                .at_stage(stage),
            ))
        };
        assert_eq!(
            fallback(&derivative(MeStage::Initialization), Phase::Initialization),
            None
        );
        assert_eq!(
            fallback(&derivative(MeStage::Integration), Phase::Integration),
            None
        );
        let typed = SimError::DirectionalDerivativeUnavailable {
            reason: "x".to_string(),
        }
        .at_stage(SimFailureStage::Integration);
        assert_eq!(fallback(&typed, Phase::Integration), None);
        // 初期化・assert・イベント・時間切れ・lowering では切り替えない
        let init =
            SimError::SolverError("boom".to_string()).at_stage(SimFailureStage::Initialization);
        assert_eq!(fallback(&init, Phase::Initialization), None);
        let assertion = SimError::AssertionFailed {
            time: 0.5,
            message: "x".to_string(),
        };
        assert_eq!(fallback(&assertion, Phase::Integration), None);
        let event = session(MeSessionError::EventIterationDiverged {
            time: 0.5,
            limit: 100,
        });
        assert_eq!(fallback(&event, Phase::Integration), None);
        let timeout = session(MeSessionError::Timeout { seconds: 1.0 });
        assert_eq!(fallback(&timeout, Phase::Integration), None);
        let lowering = SimError::SolveIr("[EX002] cannot lower".to_string());
        assert_eq!(fallback(&lowering, Phase::Build), None);
    }

    #[test]
    fn the_fallback_is_reported_with_the_bdf_failure() {
        let failed = Diagnostic::error(
            codes::RUN_INTEGRATION_FAILED,
            Stage::Run,
            "t = 0.3 で積分に失敗した",
        )
        .note("rumoca: [Integration] boom");
        let d = fallback_diagnostic(Fallback::Integration, &failed);
        assert_eq!(d.code, codes::RUN_SOLVER_FALLBACK);
        assert_eq!(d.stage, Stage::Run);
        assert!(!d.is_error());
        assert_eq!(d.message, "BDF が積分で失敗したので、dopri45 で回し直した");
        assert_eq!(
            d.notes[0],
            "BDF の失敗: t = 0.3 で積分に失敗した（GM-E11002）"
        );
        assert_eq!(d.notes[1], "rumoca: [Integration] boom");
    }

    #[test]
    fn rejected_overrides_are_split_into_three_messages() {
        let rejected = |message: &str| {
            let e = session::lowering_error(&SimulationDiagnosticError::InvalidOverride {
                message: message.to_string(),
            });
            override_diagnostic(&e).expect("上書きの誤り")
        };
        let d = rejected("`n` is structural or constant; change it by recompiling");
        assert_eq!(d.code, codes::PARAMETER_OVERRIDE_INVALID);
        assert_eq!(d.stage, Stage::S10);
        assert!(d.is_error());
        assert!(
            d.message
                .starts_with("パラメータ n は実行の前に変えられない"),
            "{}",
            d.message
        );
        assert!(d.message.contains("再コンパイル"), "{}", d.message);
        assert_eq!(
            d.notes,
            ["rumoca: [EX003] `n` is structural or constant; change it by recompiling"]
        );
        let d = rejected("`nosuch` is not a tunable parameter of this model");
        assert_eq!(
            d.message,
            "このモデルに nosuch という変えられる（tunable な）パラメータは無い"
        );
        let d = rejected("override for `k` must be finite");
        assert_eq!(d.message, "パラメータ k の上書きの値が有限でない");
        let d = rejected("override for `k` was not applied while evaluating dependent bindings");
        assert_eq!(
            d.message,
            "パラメータの上書きを実行系（Rumoca）が受け付けない"
        );
        // 上書きの誤りでないもの
        assert!(override_diagnostic(&SimError::SolveIr("[EX002] x".to_string())).is_none());
        assert!(override_diagnostic(&SimError::EmptySystem).is_none());
    }

    #[test]
    fn a_derivative_lost_during_integration_is_not_called_preparation() {
        let derivative = |stage| {
            SimError::from(MeSessionError::Component(
                MeError::DirectionalDerivativeUnavailable {
                    reason: "singular".to_string(),
                }
                .at_stage(stage),
            ))
        };
        assert_eq!(
            unsupported_where(&derivative(MeStage::Initialization), Phase::Initialization),
            "方向微分（ヤコビアン）の準備"
        );
        assert_eq!(
            unsupported_where(&derivative(MeStage::Integration), Phase::Integration),
            "積分の途中の方向微分（ヤコビアン）の計算"
        );
        // Rumoca の段（`[Integration] directional derivative is unavailable: …`）
        let typed = SimError::DirectionalDerivativeUnavailable {
            reason: "x".to_string(),
        }
        .at_stage(SimFailureStage::Integration);
        assert_eq!(
            unsupported_where(&typed, Phase::Initialization),
            "積分の途中の方向微分（ヤコビアン）の計算"
        );
        let untagged = SimError::DirectionalDerivativeUnavailable {
            reason: "x".to_string(),
        };
        assert_eq!(
            unsupported_where(&untagged, Phase::Build),
            "方向微分（ヤコビアン）の準備"
        );
    }

    #[test]
    fn notes_are_tagged_and_truncated() {
        assert_eq!(
            rumoca_note(Some("Initialization"), " boom "),
            "rumoca: [Initialization] boom"
        );
        let long = "x".repeat(NOTE_CHARS + 10);
        let note = rumoca_note(None, &long);
        assert!(note.ends_with(" …"));
        assert_eq!(note.chars().count(), "rumoca: ".len() + NOTE_CHARS + 2);
    }

    #[test]
    fn roles_follow_the_rumoca_metadata() {
        let meta = |role: &str, is_state: bool| SimVariableMeta {
            name: "v".to_string(),
            role: role.to_string(),
            is_state,
            value_type: None,
            variability: None,
            time_domain: None,
            unit: None,
            start: None,
            min: None,
            max: None,
            nominal: None,
            fixed: None,
            description: None,
        };
        assert_eq!(role(Some(&meta("state", true))), VariableRole::State);
        assert_eq!(
            role(Some(&meta("algebraic", false))),
            VariableRole::Algebraic
        );
        assert_eq!(role(Some(&meta("output", false))), VariableRole::Output);
        assert_eq!(role(Some(&meta("input", false))), VariableRole::Input);
        assert_eq!(
            role(Some(&meta("discrete-real", false))),
            VariableRole::Discrete
        );
        assert_eq!(
            role(Some(&meta("discrete-valued", false))),
            VariableRole::Discrete
        );
        assert_eq!(role(None), VariableRole::Algebraic);
    }

    #[test]
    fn settings_map_to_rumoca_options() {
        let o = globe_mode_ir::SettingOrigin::Default;
        let settings = ResolvedSettings {
            start_time: 1.0,
            stop_time: 3.0,
            tolerance: 1e-7,
            interval: 0.01,
            solver: SolverId::Dopri45,
            max_wall_seconds: Some(5.0),
            step: None,
            parameters: Default::default(),
            origins: globe_mode_ir::SettingOrigins {
                start_time: o,
                stop_time: o,
                tolerance: o,
                interval: o,
                solver: o,
                step: None,
            },
        };
        let opts = options(&settings);
        assert_eq!((opts.t_start, opts.t_end), (1.0, 3.0));
        assert_eq!((opts.rtol, opts.atol), (1e-7, 1e-7));
        assert_eq!(opts.dt, Some(0.01));
        assert_eq!(opts.max_wall_seconds, Some(5.0));
        assert_eq!(opts.solver_mode, SimSolverMode::RkLike);
        let bdf = options(&ResolvedSettings {
            solver: SolverId::Bdf,
            ..settings.clone()
        });
        assert_eq!(bdf.solver_mode, SimSolverMode::Bdf);
        let auto = options(&ResolvedSettings {
            solver: SolverId::Auto,
            ..settings
        });
        assert_eq!(auto.solver_mode, SimSolverMode::Auto);
    }
}

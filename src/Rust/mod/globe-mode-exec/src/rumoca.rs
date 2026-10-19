//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）の実行系のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイル（とこの下のモジュール）だけ。
//!
//! ソルバごとの呼び方（どれもエラーに型が付いた入口を使う。`simulate_with_diagnostics` はエラーを文字列に
//! するので使わない）:
//!
//! - `bdf` … `build_simulation` → `check_prepared_initialization` → `run_prepared_simulation`。Rumoca の
//!   MSL のワーカーと同じ道で、どこで失敗したか（組み立て・初期化・積分）がこちらの呼んだ位置で分かる。
//! - `dopri45` … `rk45::simulate`（組み立てから積分までを 1 回で呼ぶ。初期化・イベント・積分の失敗には
//!   Rumoca が段を付けるので、`bdf` と同じように分けられる。lowering の失敗だけは段が無く、`[EX…]` の印で見分ける）。
//! - `auto` … Rumoca の選び方（BDF の初期の線形化が作れないときだけ陽的 RK にする）を、`bdf` の道でなぞる。
//!
//! 失敗の振り分けは、Rumoca が付けた段（`SimFailureStage`・`MeStage`）、エラーの種類（`SimError::kind`、
//! FMI ME のセッションの `MeSessionError`・`MeIntegrationError`・`MeError`）、こちらの呼んだ位置（[`Phase`]）の順に
//! 見る。型に無いもの（Rumoca が文言に描き込んだ時刻・有限でない変数・位置、rk45 の lowering の `[EX…]` の印、
//! `eval_dae_at` の error の文言）だけを文言から読む。
//!
//! 実行が止まらない Rumoca の診断（`SimResult::diagnostics`。warning レベルの assert の WX001 など）は
//! 警告 GM-W11001 にする。

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{
    ResolvedSettings, SimRun, SimStats, SolverId, Termination, Trajectory, TrajectoryVariable,
    VariableRole,
};
use rumoca_ir_dae::Dae;
use rumoca_sim::{
    SimError, SimFailureStage, SimOptions, SimResult, SimSolverMode, SimVariableMeta,
};
use rumoca_solver::fmi_me::{MeError, MeIntegrationError, MeSessionError, MeStage};
use rumoca_solver::{SimDiagnostic, WARNING_ASSERTION_CODE};

use crate::{CompiledModel, time_text};

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
        },
        ..SimOptions::default()
    }
}

// ---------------------------------------------------------------------------------------------
// プリフライト（S10 の数値の部分）
// ---------------------------------------------------------------------------------------------

pub(crate) fn preflight(model: &CompiledModel, settings: &ResolvedSettings) -> Vec<Diagnostic> {
    let opts = options(settings);
    let dae: &Dae = model.rumoca_dae();
    // start 値（Rumoca の initial_y）を状態に入れ、代数変数を Newton 法で合わせてから、代数変数と der(x) を
    // 評価する。初期化の残差ではない（fixed = false の状態は初期化で動く）ので、見つけても警告にとどめる。
    let probe = catch_unwind(AssertUnwindSafe(|| {
        rumoca_sim::eval_dae_at(dae, &opts, &[], settings.start_time)
    }));
    // 評価そのものができない（lowering の失敗、panic）ときは黙る。実行の側で同じ失敗を診断にする
    let Ok(Ok(probe)) = probe else {
        return Vec::new();
    };
    let mut bad: Vec<String> = probe
        .report
        .nonfinite()
        .map(|(_, slot)| format!("{} = {}", slot.name, slot.nonfinite_kind().unwrap_or("NaN")))
        .collect();
    // Rumoca は有限でない値を変数に書かずに評価を止め、その名前を error の文言でだけ知らせる
    let stopped = probe.report.error.as_deref().and_then(non_finite_in);
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
    if let Some(error) = &probe.report.error {
        d = locate(d, dae, error).note(rumoca_note(None, &without_span(error)));
    }
    vec![d]
}

// ---------------------------------------------------------------------------------------------
// 実行
// ---------------------------------------------------------------------------------------------

/// こちらが Rumoca を呼んだ位置。Rumoca が段を付けなかった失敗の既定の段に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// lowering と実行系の組み立て（`build_simulation`）
    Build,
    /// 初期化（`check_prepared_initialization`）
    Initialization,
    /// 積分（`run_prepared_simulation`、`rk45::simulate`）
    Integration,
}

/// 1 回の実行の結果と、かかった時間。
struct Outcome {
    result: Result<SimResult, (SimError, Phase)>,
    build_seconds: f64,
    run_seconds: f64,
}

/// 実行する。成功なら結果と、実行が止まらない Rumoca の警告（GM-W11001）を返す。
pub(crate) fn simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
) -> Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>> {
    let opts = options(settings);
    let dae: &Dae = model.rumoca_dae();
    // Rumoca の統計（刻みの数、射影のやり直し）はスレッドごとの数なので、実行ごとに 0 に戻す。
    // NaN の追跡（nan_trace）はプロセス全体のスイッチなので触らない
    rumoca_sim::reset_step_counts();
    rumoca_sim::reset_projection_fallbacks();
    // PreparedSimulation は Send でない（Rc / RefCell）。作るのも使い終えるのもこのスレッドの中で行う
    let caught = catch_unwind(AssertUnwindSafe(|| match settings.solver {
        SolverId::Bdf => run_bdf(dae, &opts),
        SolverId::Dopri45 => run_rk(dae, &opts),
        SolverId::Auto => run_auto(dae, &opts),
    }));
    let outcome = match caught {
        Ok(outcome) => outcome,
        Err(payload) => return Err(vec![panic_diagnostic(payload.as_ref(), settings)]),
    };
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
            let warnings = warning_diagnostics(dae, &result.diagnostics);
            Ok((run(trajectory(result), termination, states), warnings))
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
                Vec::new(),
            )),
            _ => Err(vec![failure_diagnostic(dae, &error, phase, settings)]),
        },
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

/// BDF（diffsol）。組み立て・初期化・積分を分けて呼ぶ。
fn run_bdf(dae: &Dae, opts: &SimOptions) -> Outcome {
    let started = Stopwatch::start();
    let prepared = rumoca_sim::build_simulation(dae, opts);
    let build_seconds = started.seconds();
    let prepared = match prepared {
        Ok(p) => p,
        Err(e) => {
            return Outcome {
                result: Err((e, Phase::Build)),
                build_seconds,
                run_seconds: 0.0,
            };
        }
    };
    let started = Stopwatch::start();
    if let Err(e) = rumoca_sim::check_prepared_initialization(&prepared) {
        return Outcome {
            result: Err((e, Phase::Initialization)),
            build_seconds,
            run_seconds: started.seconds(),
        };
    }
    let result =
        rumoca_sim::run_prepared_simulation(&prepared).map_err(|e| (e, Phase::Integration));
    Outcome {
        result,
        build_seconds,
        run_seconds: started.seconds(),
    }
}

/// Dormand–Prince 5(4)（Rumoca の rk45）。組み立てから積分までを 1 回で呼ぶので、組み立ての時間は分からない
/// （`build_seconds` は 0、全体を `run_seconds` に入れる）。
fn run_rk(dae: &Dae, opts: &SimOptions) -> Outcome {
    let started = Stopwatch::start();
    let result = rumoca_sim::rk45::simulate(dae, opts).map_err(|e| {
        // lowering の失敗は段の無い SolveIr（"[EX…] …" の形。rk45 は段を付けない）で返る
        let phase = if lowering_tag(&e).is_some() {
            Phase::Build
        } else {
            Phase::Integration
        };
        (e, phase)
    });
    Outcome {
        result,
        build_seconds: 0.0,
        run_seconds: started.seconds(),
    }
}

/// Rumoca の Auto をなぞる: BDF で組み立てて初期化し、初期の線形化（方向微分）が作れないときだけ陽的 RK に
/// 切り替える（rumoca-sim の `select_auto_integrator` と同じ判断。積分の途中の失敗では切り替えない）。
fn run_auto(dae: &Dae, opts: &SimOptions) -> Outcome {
    let bdf = run_bdf(dae, opts);
    match &bdf.result {
        Err((e, Phase::Build | Phase::Initialization)) if directional_derivative_unavailable(e) => {
            let rk = run_rk(dae, opts);
            Outcome {
                build_seconds: bdf.build_seconds + rk.build_seconds,
                run_seconds: bdf.run_seconds + rk.run_seconds,
                result: rk.result,
            }
        }
        _ => bdf,
    }
}

/// BDF の初期の線形化が作れない失敗か（rumoca-sim の `classify_bdf_capability` と同じ判定）。
fn directional_derivative_unavailable(error: &SimError) -> bool {
    match error.kind() {
        SimError::DirectionalDerivativeUnavailable { .. } => true,
        SimError::ModelExchangeSession(MeSessionError::Component(e)) => {
            matches!(e.kind(), MeError::DirectionalDerivativeUnavailable { .. })
        }
        _ => false,
    }
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
    settings: &ResolvedSettings,
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
        Kind::Timeout => timeout_diagnostic(settings),
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

fn timeout_diagnostic(settings: &ResolvedSettings) -> Diagnostic {
    Diagnostic::error(
        codes::RUN_TIMEOUT,
        Stage::Run,
        match settings.max_wall_seconds {
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
    match error.kind() {
        SimError::UnsupportedSolverMode { .. } => "ソルバの方式の選択",
        SimError::EmptySystem => "解く方程式の無い系",
        SimError::DirectionalDerivativeUnavailable { .. } => "方向微分（ヤコビアン）の準備",
        SimError::ModelExchangeSession(MeSessionError::Component(e))
            if matches!(e.kind(), MeError::DirectionalDerivativeUnavailable { .. }) =>
        {
            "方向微分（ヤコビアン）の準備"
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
    settings: &ResolvedSettings,
) -> Diagnostic {
    if rumoca_sim::is_solver_timeout_panic(payload) {
        return timeout_diagnostic(settings);
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
        // 方向微分が作れない（auto が rk45 に切り替える条件）
        let unavailable = SimError::from(MeSessionError::Component(
            MeError::DirectionalDerivativeUnavailable {
                reason: "x".to_string(),
            }
            .at_stage(MeStage::Initialization),
        ));
        assert!(directional_derivative_unavailable(&unavailable));
        assert_eq!(
            classify(&unavailable, Phase::Initialization),
            Kind::Unsupported
        );
        assert!(!directional_derivative_unavailable(&SimError::EmptySystem));
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
            origins: globe_mode_ir::SettingOrigins {
                start_time: o,
                stop_time: o,
                tolerance: o,
                interval: o,
                solver: o,
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

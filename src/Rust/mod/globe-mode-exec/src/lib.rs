//! GlobeMode の実行系（S011、ADR-0013）。
//!
//! Phase A は Rumoca の実行系（`rumoca-sim`。diffsol の BDF と Rumoca の Dormand–Prince 5(4)）で、
//! フロントエンドが DAE まで変換したモデル（[`CompiledModel`]）を実行する。Rumoca の型は外へ出さず、
//! 結果は中立の [`SimRun`]（`globe-mode-ir`）、失敗は GlobeMode の診断（`Stage::S10` と `Stage::Run`）で返す。
//!
//! - [`preflight`] … S10 の数値の部分（start 値で評価して t0 で式の値が有限か。警告 GM-W10002）。積分の前に一度だけ。
//! - [`simulate`] … 実行。失敗は段階ごとのコード（GM-E11001〜GM-E11006、GM-U0004 / GM-U0005）にする。
//! - [`run`] … 設定を決め（`globe_mode_sim::settings::resolve`）、プリフライトを通してから実行する。
//!   bdf・auto と session の道は、組み立ての lowering の結果でプリフライトを行う（lowering は 1 回。S014）。
//! - [`ModelInstance`] … 時刻と状態を外から与えて微分を返す（FMI 2.0 ME の形。S099、ADR-0032）。
//! - [`run_with_progress`] … [`run`] と同じで、実行系の中の段（[`ExecStage`]）に入るたびに知らせる（S014）。
//! - [`run_observed`] … [`run`] と同じで、進捗（[`Progress`]）を知らせ、[`CancelToken`] で止められる（S123、ADR-0033）。
//!
//! Rumoca の時間切れは協調型で、初期化と積分器の 1 歩の内側では止まらない。確実に止めたいときは
//! 別プロセスで回して外から止める（xtask の `msl-sim`）。

use globe_mode_core::{Diagnostic, Stage, codes};
pub use globe_mode_frontend::CompiledModel;
use globe_mode_ir::{ResolvedSettings, SimRun, SimSettings, Termination, TunableParameter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "rumoca-exec")]
mod rumoca;
/// [`Variable::role`] の型
pub use globe_mode_ir::VariableRole;
/// 状態・パラメータ・時刻から微分を返す公開の API（S099。自前の FMU の `globe-mode-fmu` が使う）
#[cfg(feature = "rumoca-exec")]
pub use rumoca::instance::{DiscreteUpdate, ModelInstance, ValueType, Variable};

/// 実行系の中の段（[`run_with_progress`] が知らせる。S014）。ソルバと道によって通らない段がある。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecStage {
    /// S10 の数値の部分（t0 での評価）
    Preflight,
    /// DAE から実行できる形への変換（Rumoca の構造解析と Solve IR への変換）
    Lowering,
    /// 実行系の部品の組み立て
    Build,
    /// 初期化
    Initialization,
    /// 積分（ネイティブの dopri45 は lowering から積分までを 1 回で呼ぶので、この段だけを知らせる）
    Integration,
}

/// 進捗（[`run_observed`] が知らせる。ADR-0033）。段が変わるたびに `time: None` で 1 回、積分の段では
/// session の道だけ t を 100 ms に 1 回まで、積分の終わり（止めた時刻）に 1 回。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub stage: ExecStage,
    /// 積分の時刻（session の道の積分の段だけ）
    pub time: Option<f64>,
    pub start_time: f64,
    pub stop_time: f64,
}

/// 呼び手が実行を止める合図（ADR-0033）。実行系は段の前・`auto` の回し直しの前・session の道の積分器の
/// 1 歩ごとに見る。lowering・組み立て・ネイティブの型付きの道の積分の中では見られない。
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// [`run_observed`] に渡すもの。`now` は壁時計（ミリ秒。wasm は `performance.now()`）で、t の間引きに使う。
pub struct Observer<'a> {
    pub on_progress: &'a mut dyn FnMut(&Progress),
    pub cancel: Option<CancelToken>,
    pub now: &'a dyn Fn() -> f64,
}

impl std::fmt::Debug for Observer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observer")
            .field("cancel", &self.cancel)
            .finish_non_exhaustive()
    }
}

/// [`run`] の結果。
#[derive(Debug, Clone, Default)]
pub struct SimReport {
    /// 設定・プリフライト・実行の診断（情報と警告を含む）
    pub diagnostics: Vec<Diagnostic>,
    /// 決まった設定（`resolve` が成功した時点で入る。プリフライトや実行で失敗しても残る）
    pub settings: Option<ResolvedSettings>,
    /// 実行できたときの結果
    pub run: Option<SimRun>,
}

impl SimReport {
    /// 結果があり、Error の診断が無ければ true。
    pub fn succeeded(&self) -> bool {
        self.run.is_some() && !self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// S10 の数値の部分（積分の前の検査）。t0 で start 値から評価した代数変数と導関数が有限か（GM-W10002）。
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

/// モデルを実行する。成功なら結果、失敗なら診断（先頭が失敗の診断）を返す。実行が止まらない診断（Rumoca の
/// 警告 GM-W11001、auto の切り替え GM-I11002）と、結果から分かる診断（GM-I11001・GM-E11006）は [`run`] だけが
/// 集める。
pub fn simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
) -> Result<SimRun, Vec<Diagnostic>> {
    simulate_with_warnings(model, settings, false).map(|(run, _)| run)
}

/// [`simulate`] と同じだが、ネイティブでも bdf・dopri45・auto を wasm32 と同じ session の道（Rumoca の FMI ME 風の
/// host に Rumoca の積分器を差す道。インタプリタで回る）で回す。型付きの道と結果を比べる試験と、wasm の試験の
/// 手本に使う。成功なら、実行が止まらない診断（GM-W11001・GM-I11002）も返す。
#[doc(hidden)]
pub fn simulate_via_session(
    model: &CompiledModel,
    settings: &ResolvedSettings,
) -> Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>> {
    simulate_with_warnings(model, settings, true)
}

/// 再コンパイル無しに変えられる（tunable な）パラメータの一覧（`globe simulate --list-parameters`、結果の
/// パラメータの表）。既定値はモデルの値（上書きを反映しない）。lowering に失敗したら、その診断を返す。
pub fn tunable_parameters(model: &CompiledModel) -> Result<Vec<TunableParameter>, Vec<Diagnostic>> {
    #[cfg(feature = "rumoca-exec")]
    {
        rumoca::tunable_parameters(model)
    }
    #[cfg(not(feature = "rumoca-exec"))]
    {
        let _ = model;
        Ok(Vec::new())
    }
}

/// [`simulate`] と同じ。成功なら、実行が止まらない診断（GM-W11001・GM-I11002）も返す。
fn simulate_with_warnings(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    via_session: bool,
) -> Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>> {
    #[cfg(feature = "rumoca-exec")]
    {
        let route = if via_session {
            rumoca::Route::Session
        } else {
            rumoca::Route::Default
        };
        rumoca::simulate(model, settings, route)
    }
    #[cfg(not(feature = "rumoca-exec"))]
    {
        let _ = (model, settings, via_session);
        Err(Vec::new())
    }
}

/// 実行の結果と、実行の側で出た診断。
pub(crate) type Simulated = Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>>;

/// プリフライト（警告だけを出し、実行は止めない）と実行。
fn checked_simulate(
    model: &CompiledModel,
    settings: &ResolvedSettings,
    observer: &mut Observer<'_>,
) -> (Vec<Diagnostic>, Simulated) {
    #[cfg(feature = "rumoca-exec")]
    {
        rumoca::checked_simulate(model, settings, observer)
    }
    #[cfg(not(feature = "rumoca-exec"))]
    {
        let _ = (model, settings, observer);
        (Vec::new(), Err(Vec::new()))
    }
}

/// 設定を決め、プリフライト（S10 の数値の部分）を通してから実行する。
///
/// 実行の後に、実行が止まらない Rumoca の警告（GM-W11001。warning レベルの assert など）と、結果から分かる
/// 診断を足す: `terminate()` で止まったら情報 GM-I11001、時系列に NaN / inf があれば GM-E11006（結果は
/// [`SimReport::run`] に残す）。プリフライトが GM-W10002 を出したうえで初期化か
/// 積分に失敗したら、その失敗の診断に、t0 で有限にならなかった式を補足として付ける。
pub fn run(model: &CompiledModel, requested: &SimSettings) -> SimReport {
    run_with_progress(model, requested, &mut |_| {})
}

/// [`run`] と同じだが、実行系の中の段に入るたびに `on_stage` を呼ぶ（別プロセスのワーカーが、外から止めたものが
/// どこで止まったかを親に知らせる。xtask の `msl-sim`）。
pub fn run_with_progress(
    model: &CompiledModel,
    requested: &SimSettings,
    on_stage: &mut dyn FnMut(ExecStage),
) -> SimReport {
    let mut on_progress = |p: &Progress| {
        if p.time.is_none() {
            on_stage(p.stage)
        }
    };
    run_observed(
        model,
        requested,
        &mut Observer {
            on_progress: &mut on_progress,
            cancel: None,
            now: &|| 0.0,
        },
    )
}

/// [`run`] と同じだが、進捗を `observer.on_progress` に知らせ、`observer.cancel` が立てられたら次の区切りで止まる
/// （ADR-0033）。積分の途中で止めたら `Termination::Cancelled` とそこまでの列、それより前なら結果は無く、
/// どちらも情報 GM-I11004 を足す。
pub fn run_observed(
    model: &CompiledModel,
    requested: &SimSettings,
    observer: &mut Observer<'_>,
) -> SimReport {
    let mut report = SimReport::default();
    let settings = match globe_mode_sim::settings::resolve(requested, model.experiment.as_ref()) {
        Ok((settings, diagnostics)) => {
            report.diagnostics.extend(diagnostics);
            report.settings = Some(settings.clone());
            settings
        }
        Err(diagnostics) => {
            report.diagnostics.extend(diagnostics);
            return report;
        }
    };
    let (checked, simulated) = checked_simulate(model, &settings, observer);
    let not_finite_at_t0 = checked
        .iter()
        .find(|d| d.code == codes::T0_NOT_FINITE)
        .map(|d| format!("{}（{}）", d.message, d.code));
    report.diagnostics.extend(checked);
    match simulated {
        Ok((run, warnings)) => {
            report.diagnostics.extend(warnings);
            report.diagnostics.extend(outcome_diagnostics(&run));
            report.run = Some(run);
        }
        Err(diagnostics) => {
            report
                .diagnostics
                .extend(diagnostics.into_iter().map(|d| match &not_finite_at_t0 {
                    Some(note)
                        if d.code == codes::RUN_INIT_FAILED
                            || d.code == codes::RUN_INTEGRATION_FAILED =>
                    {
                        d.note(format!("プリフライトの警告: {note}"))
                    }
                    _ => d,
                }))
        }
    }
    report
}

/// 成功した実行の結果から分かる診断（`terminate()` での終了、時系列の NaN / inf）。
fn outcome_diagnostics(run: &SimRun) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if let Termination::Terminated { time, message } = &run.termination {
        let mut d = Diagnostic::info(
            codes::RUN_TERMINATED,
            Stage::Run,
            format!(
                "t = {} で terminate() により終了時刻の前に止まった",
                time_text(*time)
            ),
        );
        if !message.is_empty() {
            d = d.note(format!("terminate の文言: {message}"));
        }
        out.push(d);
    }
    if let Termination::Cancelled { time } = &run.termination {
        out.push(cancelled_diagnostic(Some(*time)));
    }
    let bad = run.trajectory.non_finite();
    if !bad.is_empty() {
        let first = run
            .trajectory
            .variables
            .iter()
            .filter_map(|v| {
                let i = v.values.iter().position(|x| !x.is_finite())?;
                Some((run.trajectory.time.get(i).copied()?, v.name.as_str()))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let mut message = format!(
            "結果に NaN / inf が出た（{} 個の変数）: {}",
            bad.len(),
            bad.iter().take(10).copied().collect::<Vec<_>>().join(", ")
        );
        if bad.len() > 10 {
            message.push_str(" ほか");
        }
        let mut d = Diagnostic::error(codes::RUN_NOT_FINITE, Stage::Run, message);
        if let Some((t, name)) = first {
            d = d.note(format!("最初に出たのは t = {} の {name}", time_text(t)));
        }
        out.push(d);
    }
    out
}

/// 呼び手が止めた（GM-I11004）。`time` は積分の途中で止めた時刻。
pub(crate) fn cancelled_diagnostic(time: Option<f64>) -> Diagnostic {
    let message = match time {
        Some(t) => format!("呼び手が t = {} で止めた", time_text(t)),
        None => "呼び手が積分の前に止めた".to_string(),
    };
    Diagnostic::info(codes::RUN_CANCELLED, Stage::Run, message)
}

/// 時刻の表示（9 桁で丸め、末尾の 0 を除く。0.5000000000000001 → 0.5）。
pub(crate) fn time_text(t: f64) -> String {
    let s = format!("{t:.9}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_ir::{
        SettingOrigin, SettingOrigins, SimStats, SolverId, Trajectory, TrajectoryVariable,
        VariableRole,
    };

    fn finished(termination: Termination, values: Vec<f64>) -> SimRun {
        let o = SettingOrigin::Default;
        SimRun {
            model: "M".to_string(),
            settings: ResolvedSettings {
                start_time: 0.0,
                stop_time: 1.0,
                tolerance: 1e-6,
                interval: 0.5,
                solver: SolverId::Bdf,
                max_wall_seconds: None,
                step: None,
                parameters: Default::default(),
                origins: SettingOrigins {
                    start_time: o,
                    stop_time: o,
                    tolerance: o,
                    interval: o,
                    solver: o,
                    step: None,
                },
            },
            trajectory: Trajectory {
                time: vec![0.0, 0.5, 1.0],
                variables: vec![TrajectoryVariable {
                    name: "x".to_string(),
                    role: VariableRole::State,
                    unit: None,
                    description: None,
                    values,
                }],
            },
            termination,
            stats: SimStats::default(),
            parameters: Vec::new(),
        }
    }

    #[test]
    fn a_clean_run_has_no_outcome_diagnostics() {
        let run = finished(Termination::StopTime, vec![1.0, 0.5, 0.25]);
        assert!(outcome_diagnostics(&run).is_empty());
    }

    #[test]
    fn terminate_is_information() {
        let run = finished(
            Termination::Terminated {
                time: 0.5000000000000001,
                message: "done".to_string(),
            },
            vec![1.0, 0.5, 0.25],
        );
        let d = outcome_diagnostics(&run);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, codes::RUN_TERMINATED);
        assert!(!d[0].is_error());
        assert_eq!(
            d[0].message,
            "t = 0.5 で terminate() により終了時刻の前に止まった"
        );
        assert_eq!(d[0].notes, ["terminate の文言: done"]);
    }

    #[test]
    fn non_finite_results_are_an_error_but_the_run_is_kept() {
        let run = finished(Termination::StopTime, vec![1.0, f64::NAN, f64::INFINITY]);
        let d = outcome_diagnostics(&run);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, codes::RUN_NOT_FINITE);
        assert!(d[0].is_error());
        assert!(d[0].message.ends_with(": x"), "{}", d[0].message);
        assert_eq!(d[0].notes, ["最初に出たのは t = 0.5 の x"]);
    }

    #[test]
    fn times_are_shown_short() {
        assert_eq!(time_text(0.5000000000000001), "0.5");
        assert_eq!(time_text(1.0), "1");
        assert_eq!(time_text(0.0), "0");
        assert_eq!(time_text(-0.0), "0");
        assert_eq!(time_text(1e-9), "0.000000001");
        assert_eq!(time_text(-2.25), "-2.25");
    }
}

//! 実行系（Rumoca の rumoca-sim）を、src/Modelica/GlobeModeValidation/Simulation のモデルで確かめる（S011）。
//!
//! - 解析解のあるモデル（Decay・HarmonicOscillator・NonlinearLoop・NoStates）は終了時刻の値を解析解と比べる。
//! - イベントのモデル（BouncingBall・SampleCounter）は回数と、イベントの時刻に依る値を比べる。
//! - 失敗するモデルは、診断のコード（GM-E11001〜GM-E11005）と、本文・位置を確かめる。
//! - 止まらない警告（warning レベルの assert）は GM-W11001 になり、実行は成功する。
//!
//! 設定は `ResolvedSettings` を直接作る（experiment annotation の値を写す）。`run` だけは
//! `globe_mode_sim::settings::resolve` を通す。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globe_mode_core::{Diagnostic, Severity, Stage, codes};
use globe_mode_exec::{
    CompiledModel, ExecStage, preflight, run, run_with_progress, simulate, simulate_via_session,
    tunable_parameters,
};
use globe_mode_frontend::{Checker, Source};
use globe_mode_ir::{
    ResolvedSettings, SettingOrigin, SettingOrigins, SimRun, SimSettings, SolverId, Termination,
    VariableRole,
};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
}

/// パッケージの全 `.mo`（名前、本文）。テストのスレッドで共有する（読むのは 1 回）
fn package_files() -> &'static [(String, String)] {
    static FILES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    FILES.get_or_init(|| {
        let mut paths = Vec::new();
        collect(&package_dir(), &mut paths);
        paths.sort();
        paths
            .into_iter()
            .map(|p| {
                (
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                )
            })
            .collect()
    })
}

/// `GlobeModeValidation.Simulation.<name>` を DAE まで変換する。
fn compile(name: &str) -> CompiledModel {
    let sources: Vec<Source<'_>> = package_files()
        .iter()
        .map(|(n, t)| Source { name: n, text: t })
        .collect();
    let checker = Checker::new(&[]).unwrap();
    let model = format!("GlobeModeValidation.Simulation.{name}");
    checker
        .compile_retaining(&sources, &model)
        .unwrap_or_else(|d| panic!("{model} を変換できない: {d:#?}"))
}

/// experiment annotation の値を写した設定（無い項目は既定値: 0〜1、1e-6、区間の 1/500）。
fn settings(model: &CompiledModel, solver: SolverId) -> ResolvedSettings {
    let e = model.experiment.clone().unwrap_or_default();
    let start_time = e.start_time.map_or(0.0, |b| b.get());
    let stop_time = e.stop_time.map_or(start_time + 1.0, |b| b.get());
    let o = SettingOrigin::Default;
    ResolvedSettings {
        start_time,
        stop_time,
        tolerance: e.tolerance.map_or(1e-6, |b| b.get()),
        interval: e
            .interval
            .map_or((stop_time - start_time) / 500.0, |b| b.get()),
        solver,
        max_wall_seconds: Some(30.0),
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
    }
}

fn simulate_ok(name: &str, solver: SolverId) -> SimRun {
    let model = compile(name);
    match simulate(&model, &settings(&model, solver)) {
        Ok(run) => {
            check_trajectory(&run);
            run
        }
        Err(d) => panic!("{name}（{}）が失敗した: {d:#?}", solver.name()),
    }
}

fn simulate_err(name: &str, solver: SolverId) -> Diagnostic {
    let model = compile(name);
    match simulate(&model, &settings(&model, solver)) {
        Ok(run) => panic!(
            "{name}（{}）が成功した: {:?}",
            solver.name(),
            run.termination
        ),
        Err(d) => {
            assert_eq!(d.len(), 1, "{d:#?}");
            let d = d.into_iter().next().unwrap();
            assert_eq!(d.stage, Stage::Run, "{d:#?}");
            assert!(
                d.notes.iter().any(|n| n.starts_with("rumoca: ")),
                "Rumoca の文言が補足に無い: {d:#?}"
            );
            d
        }
    }
}

/// 時系列の形: 時刻は減らず、どの変数も時刻と同じ長さで、統計の点の数と合う。
fn check_trajectory(run: &SimRun) {
    let t = &run.trajectory;
    assert!(!t.time.is_empty());
    assert!(
        t.time.windows(2).all(|w| w[0] <= w[1]),
        "時刻が減っている: {:?}",
        t.time
    );
    assert_eq!(t.time[0], run.settings.start_time);
    assert_eq!(run.stats.points, t.time.len());
    for v in &t.variables {
        assert_eq!(v.values.len(), t.time.len(), "{} の長さ", v.name);
    }
    assert!(run.stats.build_seconds >= 0.0 && run.stats.run_seconds > 0.0);
}

/// 終了時刻の値（最後の行）。
fn last(run: &SimRun, name: &str) -> f64 {
    let v = run
        .trajectory
        .variable(name)
        .unwrap_or_else(|| panic!("{name} が結果に無い"));
    *v.values.last().unwrap()
}

fn assert_close(actual: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: {actual} と {expected} の差が {tol} を超える"
    );
}

#[test]
fn decay_matches_the_analytic_solution() {
    let run = simulate_ok("Decay", SolverId::Bdf);
    assert_eq!(run.termination, Termination::StopTime);
    assert_eq!(*run.trajectory.time.last().unwrap(), 1.0);
    // experiment の Tolerance = 1e-8
    assert_close(last(&run, "x"), (-1.0f64).exp(), 1e-6, "x(1)");
    let x = run.trajectory.variable("x").unwrap();
    assert_eq!(x.role, VariableRole::State);
    assert_eq!(run.stats.states, 1);
    // 出力の間隔は区間の 1/500（初めの行を含めて 501 行。イベントは無い）
    assert_eq!(run.stats.points, 501);
}

#[test]
fn decay_runs_with_every_solver() {
    for solver in [SolverId::Bdf, SolverId::Dopri45, SolverId::Auto] {
        let model = compile("Decay");
        let settings = ResolvedSettings {
            tolerance: 1e-6,
            ..settings(&model, solver)
        };
        let run =
            simulate(&model, &settings).unwrap_or_else(|d| panic!("{}: {d:#?}", solver.name()));
        check_trajectory(&run);
        assert_eq!(run.settings.solver, solver);
        assert_close(last(&run, "x"), (-1.0f64).exp(), 1e-5, solver.name());
    }
}

#[test]
fn oscillator_matches_the_analytic_solution() {
    for solver in [SolverId::Bdf, SolverId::Dopri45] {
        let run = simulate_ok("HarmonicOscillator", solver);
        assert_eq!(run.stats.states, 2);
        assert_close(last(&run, "x"), 2.0f64.cos(), 1e-5, "x(1)");
        assert_close(last(&run, "v"), -2.0 * 2.0f64.sin(), 1e-5, "v(1)");
    }
}

#[test]
fn nonlinear_loop_is_solved_at_every_point() {
    let run = simulate_ok("NonlinearLoop", SolverId::Bdf);
    assert_close(last(&run, "x"), 1.5, 1e-6, "x(1)");
    let y = run.trajectory.variable("y").unwrap();
    let z = run.trajectory.variable("z").unwrap();
    assert_eq!(y.role, VariableRole::Algebraic);
    // 各点で start 値の側の解（y = 1 + t、z = 3 + t）にいる
    for (i, t) in run.trajectory.time.iter().enumerate() {
        assert_close(y.values[i], 1.0 + t, 1e-6, "y");
        assert_close(z.values[i], 3.0 + t, 1e-6, "z");
    }
}

#[test]
fn a_model_without_states_simulates() {
    for solver in [SolverId::Bdf, SolverId::Dopri45, SolverId::Auto] {
        let run = simulate_ok("NoStates", solver);
        assert_eq!(run.stats.states, 0);
        let y = run.trajectory.variable("y").unwrap();
        for (i, t) in run.trajectory.time.iter().enumerate() {
            assert_close(y.values[i], t.sin(), 1e-12, "y");
        }
        assert_eq!(*run.trajectory.time.last().unwrap(), 1.0);
    }
}

#[test]
fn bouncing_ball_bounces_three_times() {
    for solver in [SolverId::Bdf, SolverId::Dopri45] {
        let run = simulate_ok("BouncingBall", solver);
        let n = run.trajectory.variable("n").unwrap();
        assert_eq!(n.role, VariableRole::Discrete);
        assert_eq!(last(&run, "n"), 3.0);
        // 床を突き抜けない（イベントの位置を突き止めてから反転する）
        let h = run.trajectory.variable("h").unwrap();
        let lowest = h.values.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(lowest > -1e-6, "h の最小 {lowest}");
        // 3 回目の後の放物線（解析解 h(2) ≈ 0.260739）
        assert_close(last(&run, "h"), 0.260739, 1e-4, "h(2)");
        // イベントでは、左極限（v < 0）とイベントの後（v > 0）の行が続く。Rumoca は 2 行の時刻を数 ulp ずらして
        // 書く（同じ時刻にはならない）
        let v = run.trajectory.variable("v").unwrap();
        let t = &run.trajectory.time;
        let jumps = (1..t.len())
            .filter(|&i| v.values[i - 1] < 0.0 && v.values[i] > 0.0 && t[i] - t[i - 1] < 1e-9)
            .count();
        assert_eq!(jumps, 3, "速度の反転の数");
    }
}

#[test]
fn sample_counter_counts_time_events() {
    for solver in [SolverId::Bdf, SolverId::Dopri45] {
        let run = simulate_ok("SampleCounter", solver);
        assert_eq!(last(&run, "n"), 10.0);
        assert_close(last(&run, "xs"), 0.95, 1e-9, "xs");
        assert_close(last(&run, "x"), 1.0, 1e-9, "x(1)");
    }
}

#[test]
fn terminate_ends_the_run_early_without_failing() {
    let run = simulate_ok("Terminate", SolverId::Bdf);
    match &run.termination {
        Termination::Terminated { time, message } => {
            assert_close(*time, 0.5, 1e-9, "terminate の時刻");
            assert_eq!(message, "x が limit に達した");
        }
        other => panic!("terminate で止まっていない: {other:?}"),
    }
    assert_close(
        *run.trajectory.time.last().unwrap(),
        0.5,
        1e-9,
        "最後の時刻",
    );
}

#[test]
fn runtime_assert_is_reported_with_its_time_and_message() {
    for solver in [SolverId::Bdf, SolverId::Dopri45] {
        let d = simulate_err("RuntimeAssert", solver);
        assert_eq!(d.code, codes::RUN_ASSERTION, "{d:#?}");
        assert_eq!(d.message, "t = 0.5 で assert が失敗した: x が上限に達した");
    }
}

#[test]
fn event_iteration_that_never_settles_is_reported() {
    let d = simulate_err("EventNotSettling", SolverId::Bdf);
    assert_eq!(d.code, codes::RUN_EVENT_FAILED, "{d:#?}");
    assert!(d.message.starts_with("t = 0.5 で"), "{}", d.message);
}

#[test]
fn initialization_without_a_real_solution_fails() {
    for solver in [SolverId::Bdf, SolverId::Dopri45, SolverId::Auto] {
        let d = simulate_err("InitNoSolution", solver);
        assert_eq!(d.code, codes::RUN_INIT_FAILED, "{d:#?}");
        assert!(
            d.notes
                .iter()
                .any(|n| n.starts_with("rumoca: [Initialization]")),
            "{d:#?}"
        );
    }
}

#[test]
fn not_finite_at_start_is_warned_and_then_fails_at_initialization() {
    let model = compile("NotFiniteAtStart");
    let s = settings(&model, SolverId::Bdf);
    // S10: y = 1 / x は t0 で inf（x の start 値 0 は有限なので、静的な検査は通る）
    let warned = preflight(&model, &s);
    assert_eq!(warned.len(), 1, "{warned:#?}");
    let w = &warned[0];
    assert_eq!(w.code, codes::T0_NOT_FINITE);
    assert_eq!(w.severity, Severity::Warning);
    assert_eq!(w.stage, Stage::S10);
    assert!(w.message.ends_with(": y = inf"), "{}", w.message);
    let text = &package_files()
        .iter()
        .find(|(n, _)| n.ends_with("NotFiniteAtStart.mo"))
        .unwrap()
        .1;
    let excerpt = |d: &Diagnostic| {
        assert!(d.file.as_deref().unwrap().ends_with("NotFiniteAtStart.mo"));
        let r = d.range.unwrap();
        text[r.start as usize..r.end as usize].to_string()
    };
    // 位置は y の宣言（Rumoca が有限でない値を名指しする位置）
    assert_eq!(excerpt(w), "Real y");
    // 実行: x は fixed = true なので初期化でも 0 のまま。初期化で y が inf になって止まる
    let d = simulate_err("NotFiniteAtStart", SolverId::Bdf);
    assert_eq!(d.code, codes::RUN_INIT_FAILED, "{d:#?}");
    assert_eq!(d.message, "初期化に失敗した（y の値が inf になった）");
    assert_eq!(excerpt(&d), "Real y");
}

/// S018: 初期化方程式が t0 で反転する関係式を読むと、Rumoca の初期化は関係式の既定値（false）の枝で解き、
/// 反転しても解き直さない（S016 の EngineV6 の最小の再現）。`run` は初期化系をもう一度解き、実行が始まった t0 の値と
/// 違う変数を GM-W10004 で知らせる。実行は 1 回目の値（x(0) = 2。正しくは 1）で始まる。Rumoca がこれを直したら警告は
/// 出なくなり x(0) = 1 になるので、そのときはこの試験の期待を入れ替える。単独の `preflight` は GM-W10002 だけで、
/// この警告は出さない（実行の t0 の値が要るため）。
#[test]
fn an_initialization_that_reads_a_relation_flipped_at_t0_is_warned() {
    let model = compile("InitRelationBranch");
    let s = settings(&model, SolverId::Bdf);
    let alone = preflight(&model, &s);
    assert!(alone.is_empty(), "{alone:#?}");
    let expected = "初期化の結果を初期化系にもう一度入れると t = 0 で値が変わる: x = 2 → 1";
    // run はプリフライトの警告を診断に入れ、実行は 1 回目の値で始まる（どの道でも同じ）
    for solver in ["bdf", "dopri45", "auto", "rk4"] {
        let report = run(
            &model,
            &SimSettings {
                solver: Some(solver.to_string()),
                ..SimSettings::default()
            },
        );
        let warned: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::INIT_RESETTLE_MOVED)
            .collect();
        assert_eq!(warned.len(), 1, "{solver}: {:#?}", report.diagnostics);
        assert_eq!(warned[0].message, expected, "{solver}");
        assert_eq!(warned[0].severity, Severity::Warning, "{solver}");
        assert_eq!(warned[0].stage, Stage::S10, "{solver}");
        let run = report
            .run
            .as_ref()
            .unwrap_or_else(|| panic!("{solver}: {:#?}", report.diagnostics));
        let x = run.trajectory.variable("x").expect("x が結果に無い");
        let u = run.trajectory.variable("u").expect("u が結果に無い");
        assert_close(u.values[0], 1.0, 1e-9, &format!("{solver}: u(0)"));
        assert_close(
            x.values[0],
            2.0,
            1e-9,
            &format!("{solver}: x(0)（Rumoca の不具合の値）"),
        );
    }
}

/// `run` のプリフライトは組み立ての lowering の結果で行う（S014）。どのソルバでも単独の `preflight` と同じ警告になる。
#[test]
fn run_preflights_on_the_lowering_of_every_solver() {
    let model = compile("NotFiniteAtStart");
    let alone = preflight(&model, &settings(&model, SolverId::Bdf));
    for solver in ["bdf", "dopri45", "auto", "rk4"] {
        let report = run(
            &model,
            &SimSettings {
                solver: Some(solver.to_string()),
                ..SimSettings::default()
            },
        );
        let warned: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::T0_NOT_FINITE)
            .collect();
        assert_eq!(warned.len(), 1, "{solver}: {:#?}", report.diagnostics);
        assert_eq!(warned[0].message, alone[0].message, "{solver}");
        assert_eq!(warned[0].range, alone[0].range, "{solver}");
    }
}

#[test]
fn run_reports_the_stages_inside_the_executor() {
    use ExecStage::*;
    let model = compile("Decay");
    for (solver, expected) in [
        (
            "bdf",
            vec![Lowering, Preflight, Build, Initialization, Integration],
        ),
        // auto は BDF で通るので bdf と同じ
        (
            "auto",
            vec![Lowering, Preflight, Build, Initialization, Integration],
        ),
        // session の道
        (
            "rk4",
            vec![Lowering, Preflight, Build, Initialization, Integration],
        ),
        // rk45::simulate は lowering から積分までを 1 回で呼ぶ。プリフライトは単独で先に行う
        ("dopri45", vec![Preflight, Integration]),
    ] {
        let mut seen = Vec::new();
        let report = run_with_progress(
            &model,
            &SimSettings {
                solver: Some(solver.to_string()),
                ..SimSettings::default()
            },
            &mut |stage| seen.push(stage),
        );
        assert!(report.succeeded(), "{solver}: {:#?}", report.diagnostics);
        assert_eq!(seen, expected, "{solver}");
    }
    // 初期化で止まるモデルは積分の段に入らない
    let mut seen = Vec::new();
    let report = run_with_progress(
        &compile("InitNoSolution"),
        &SimSettings {
            solver: Some("bdf".to_string()),
            ..SimSettings::default()
        },
        &mut |stage| seen.push(stage),
    );
    assert!(!report.succeeded());
    assert_eq!(seen, vec![Lowering, Preflight, Build, Initialization]);
}

#[test]
fn models_that_are_finite_at_start_get_no_preflight_warning() {
    for name in [
        "Decay",
        "HarmonicOscillator",
        "NonlinearLoop",
        "NoStates",
        "BouncingBall",
        "SampleCounter",
        "Terminate",
        "RuntimeAssert",
        "WarningAssert",
        "EventNotSettling",
        "InitNoSolution",
    ] {
        let model = compile(name);
        let warned = preflight(&model, &settings(&model, SolverId::Bdf));
        assert!(warned.is_empty(), "{name}: {warned:#?}");
    }
}

#[test]
fn an_exhausted_wall_clock_budget_is_a_timeout() {
    for solver in [SolverId::Bdf, SolverId::Dopri45] {
        let model = compile("HarmonicOscillator");
        let settings = ResolvedSettings {
            stop_time: 100.0,
            interval: 0.01,
            max_wall_seconds: Some(1e-9),
            ..settings(&model, solver)
        };
        let d = simulate(&model, &settings).expect_err("時間切れにならない");
        assert_eq!(d[0].code, codes::RUN_TIMEOUT, "{d:#?}");
        assert!(d[0].message.contains("0.000000001 秒"), "{}", d[0].message);
    }
}

#[test]
fn run_resolves_settings_from_the_experiment_annotation() {
    let model = compile("Decay");
    let report = run(&model, &SimSettings::default());
    assert!(report.succeeded(), "{:#?}", report.diagnostics);
    let r = report.run.unwrap();
    assert_eq!(r.settings.stop_time, 1.0);
    assert_eq!(r.settings.tolerance, 1e-8);
    assert_eq!(r.settings.origins.tolerance, SettingOrigin::Experiment);
    assert_close(last(&r, "x"), (-1.0f64).exp(), 1e-6, "x(1)");
}

#[test]
fn run_reports_terminate_as_information() {
    let report = run(&compile("Terminate"), &SimSettings::default());
    assert!(report.succeeded(), "{:#?}", report.diagnostics);
    let info: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::RUN_TERMINATED)
        .collect();
    assert_eq!(info.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(info[0].severity, Severity::Info);
    assert!(info[0].message.starts_with("t = 0.5 で terminate()"));
    assert!(matches!(
        report.run.unwrap().termination,
        Termination::Terminated { .. }
    ));
}

#[test]
fn run_links_a_preflight_warning_to_the_failure() {
    let report = run(&compile("NotFiniteAtStart"), &SimSettings::default());
    assert!(!report.succeeded());
    assert!(report.run.is_none());
    let codes_seen: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
    assert!(codes_seen.contains(&codes::T0_NOT_FINITE), "{codes_seen:?}");
    let failure = report
        .diagnostics
        .iter()
        .find(|d| d.code == codes::RUN_INIT_FAILED)
        .unwrap_or_else(|| panic!("{:#?}", report.diagnostics));
    assert!(
        failure
            .notes
            .iter()
            .any(|n| n.starts_with("プリフライトの警告: ") && n.contains("y = inf")),
        "{failure:#?}"
    );
}

#[test]
fn run_reports_warning_level_assertions_without_failing() {
    let report = run(&compile("WarningAssert"), &SimSettings::default());
    assert!(report.succeeded(), "{:#?}", report.diagnostics);
    let r = report.run.as_ref().unwrap();
    assert_eq!(r.termination, Termination::StopTime);
    assert_close(last(r, "x"), 1.0, 1e-9, "x(1)");
    let warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::RUN_WARNING)
        .collect();
    assert_eq!(warnings.len(), 1, "{:#?}", report.diagnostics);
    let w = warnings[0];
    assert_eq!(w.severity, Severity::Warning);
    assert_eq!(w.stage, Stage::Run);
    assert!(
        w.message
            .ends_with("assert（AssertionLevel.warning）の条件が破れた: x が上限を超えた"),
        "{}",
        w.message
    );
    assert!(w.message.starts_with("t = 0.5"), "{}", w.message);
    assert!(
        w.notes.iter().any(|n| n.starts_with("rumoca: WX001: ")),
        "{w:#?}"
    );
    assert!(
        w.file.as_deref().unwrap().ends_with("WarningAssert.mo"),
        "{w:#?}"
    );
    // dopri45 でも止まらない（error レベルの RuntimeAssert は止まる）
    let model = compile("WarningAssert");
    assert!(simulate(&model, &settings(&model, SolverId::Dopri45)).is_ok());
}

#[test]
fn run_fails_the_assertion_model() {
    let report = run(&compile("RuntimeAssert"), &SimSettings::default());
    assert!(!report.succeeded());
    // 失敗しても、使った設定は残る
    let s = report.settings.as_ref().expect("設定が残る");
    assert_eq!((s.start_time, s.stop_time), (0.0, 1.0));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::RUN_ASSERTION),
        "{:#?}",
        report.diagnostics
    );
}

/// `Simulation.*` の 13 本（S011、S018 で 1 本足した）。session の道と型付きの道を比べる。
const VALIDATION_MODELS: [&str; 13] = [
    "Decay",
    "HarmonicOscillator",
    "NonlinearLoop",
    "NoStates",
    "BouncingBall",
    "SampleCounter",
    "Terminate",
    "RuntimeAssert",
    "WarningAssert",
    "EventNotSettling",
    "NotFiniteAtStart",
    "InitNoSolution",
    "InitRelationBranch",
];

/// 結果の要約（行の数、終わり方、変数ごとの最後の値）か、失敗の診断（コードと本文）。
#[derive(Debug)]
enum Summary {
    Ok {
        rows: usize,
        termination: Termination,
        last: Vec<(String, f64)>,
        warnings: Vec<&'static str>,
    },
    /// 診断（補足と位置まで比べる）
    Err(Vec<Diagnostic>),
}

fn summary(result: Result<(SimRun, Vec<Diagnostic>), Vec<Diagnostic>>) -> Summary {
    match result {
        Ok((run, warnings)) => {
            check_trajectory(&run);
            Summary::Ok {
                rows: run.trajectory.time.len(),
                termination: run.termination.clone(),
                last: run
                    .trajectory
                    .variables
                    .iter()
                    .map(|v| (v.name.clone(), *v.values.last().unwrap()))
                    .collect(),
                warnings: warnings.iter().map(|d| d.code).collect(),
            }
        }
        Err(d) => Summary::Err(d),
    }
}

/// session の道（wasm32 の道）の Rumoca の BDF が、ネイティブの型付きの道（Cranelift の JIT）と同じ行の数・
/// 最後の値（ビット単位で同じ）・診断（コード・本文・補足・位置）になる。wasm の試験（Node で同じ 12 本を
/// 回す）の正しさは、これに依る。
#[test]
fn the_session_route_matches_the_typed_route() {
    for name in VALIDATION_MODELS {
        let model = compile(name);
        let s = settings(&model, SolverId::Bdf);
        let typed = summary(simulate(&model, &s).map(|run| (run, Vec::new())));
        let session = summary(simulate_via_session(&model, &s));
        match (&typed, &session) {
            (
                Summary::Ok {
                    rows: r1,
                    termination: t1,
                    last: l1,
                    ..
                },
                Summary::Ok {
                    rows: r2,
                    termination: t2,
                    last: l2,
                    warnings,
                },
            ) => {
                assert_eq!(r1, r2, "{name}: 行の数");
                assert_eq!(t1, t2, "{name}: 終わり方");
                assert_eq!(l1.len(), l2.len(), "{name}: 変数の数");
                for ((n1, v1), (n2, v2)) in l1.iter().zip(l2) {
                    assert_eq!(n1, n2, "{name}: 変数の並び");
                    assert_eq!(v1.to_bits(), v2.to_bits(), "{name}.{n1}: {v1} と {v2}");
                }
                // 止まらない警告は session の道でも出る（型付きの道は simulate が捨てるので run で比べる）
                if name == "WarningAssert" {
                    assert_eq!(warnings, &[codes::RUN_WARNING], "{name}");
                } else {
                    assert!(warnings.is_empty(), "{name}: {warnings:?}");
                }
            }
            (Summary::Err(d1), Summary::Err(d2)) => assert_eq!(d1, d2, "{name}: 診断"),
            _ => panic!("{name}: 型付きの道 {typed:#?}、session の道 {session:#?}"),
        }
        eprintln!("{name}: {session:?}");
    }
}

/// session の道の dopri45 と auto も、型付きの道と同じ結果になる（解析解のあるモデルとイベントのモデル）。
#[test]
fn the_session_route_runs_dopri45_and_auto() {
    for name in ["Decay", "HarmonicOscillator", "BouncingBall", "NoStates"] {
        for solver in [SolverId::Dopri45, SolverId::Auto] {
            let model = compile(name);
            let s = settings(&model, solver);
            let typed = simulate(&model, &s).unwrap_or_else(|d| panic!("{name}: {d:#?}"));
            let (session, info) =
                simulate_via_session(&model, &s).unwrap_or_else(|d| panic!("{name}: {d:#?}"));
            assert!(info.is_empty(), "{name}: {info:#?}");
            assert_eq!(
                typed.trajectory.time.len(),
                session.trajectory.time.len(),
                "{name}（{}）: 行の数",
                solver.name()
            );
            for v in &typed.trajectory.variables {
                assert_close(
                    last(&session, &v.name),
                    *v.values.last().unwrap(),
                    1e-9,
                    &format!("{name}.{}（{}）", v.name, solver.name()),
                );
            }
        }
    }
}

/// パラメータの上書きの設定。
fn with_parameters(model: &CompiledModel, solver: SolverId, p: &[(&str, f64)]) -> ResolvedSettings {
    ResolvedSettings {
        parameters: p.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
        ..settings(model, solver)
    }
}

#[test]
fn a_tunable_parameter_is_overridden_without_recompiling() {
    let model = compile("Parameters");
    for solver in [SolverId::Bdf, SolverId::Dopri45, SolverId::Auto] {
        let s = with_parameters(&model, solver, &[("k", 2.0)]);
        for (route, run) in [
            ("型付き", simulate(&model, &s)),
            ("session", simulate_via_session(&model, &s).map(|(r, _)| r)),
        ] {
            let what = format!("{}・{route}", solver.name());
            let run = run.unwrap_or_else(|d| panic!("{what}: {d:#?}"));
            check_trajectory(&run);
            // der(x) = -k x、k = 2 で x(1) = exp(-2)。c = 2 k は k に従う
            assert_close(last(&run, "x"), (-2.0f64).exp(), 1e-6, &what);
            assert_close(last(&run, "y[1]"), 4.0, 1e-12, &what);
            assert_close(last(&run, "y[2]"), 4.0, 1e-12, &what);
            assert_eq!(run.settings.parameters.get("k"), Some(&2.0), "{what}");
            // 実行に使った値（rk45 の型付きの道は lowering が見えないので空）
            if solver == SolverId::Dopri45 && route == "型付き" {
                assert!(run.parameters.is_empty(), "{what}: {:?}", run.parameters);
            } else {
                let value = |n: &str| {
                    run.parameters
                        .iter()
                        .find(|p| p.name == n)
                        .unwrap_or_else(|| panic!("{what}: {n} が無い: {:?}", run.parameters))
                        .value
                };
                assert_eq!(value("k"), 2.0, "{what}");
                assert_eq!(value("c"), 4.0, "{what}");
                let k = run.parameters.iter().find(|p| p.name == "k").unwrap();
                assert_eq!(k.description.as_deref(), Some("減衰の速さ（tunable）"));
            }
        }
    }
    // 上書きが無ければモデルの値
    let run = simulate(&model, &settings(&model, SolverId::Bdf)).unwrap();
    assert_close(last(&run, "x"), (-1.0f64).exp(), 1e-6, "k = 1");
    assert_close(last(&run, "y[1]"), 2.0, 1e-12, "c = 2");
}

/// 構造的な上書き（S024）。既定を変換した後に同じ検査器で上書きを変換する（変換済みの Session に足すと ED020）。
#[test]
fn structural_overrides_reach_bindings_inlined_by_same_name_modifications() {
    let sources: Vec<Source<'_>> = package_files()
        .iter()
        .map(|(n, t)| Source { name: n, text: t })
        .collect();
    let checker = Checker::new(&[]).unwrap();
    let name = "GlobeModeValidation.Simulation.InlinedParameters";
    let with = |p: &[(&str, f64)]| {
        let overrides = p.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        checker
            .compile_retaining_with_overrides(&sources, name, &overrides)
            .unwrap_or_else(|d| panic!("{p:?}: {d:#?}"))
    };
    let y_z = |model: &CompiledModel| {
        let run = simulate(model, &settings(model, SolverId::Bdf)).unwrap();
        (last(&run, "src.y"), last(&run, "z"))
    };
    // 既定: startTime = 1 なので t = 1 で y = 0、z = n = 2
    assert_eq!(y_z(&with(&[])), (0.0, 2.0));
    // 実行時の上書きだけでは内側の r.startTime が 1 のまま（y = 0）。構造的に上書きすれば t = 1 で y = 0.5
    let model = with(&[("src.startTime", 0.5)]);
    let s = with_parameters(&model, SolverId::Bdf, &[("src.startTime", 0.5)]);
    let run = simulate(&model, &s).unwrap();
    assert_close(last(&run, "src.y"), 0.5, 1e-9, "src.startTime = 0.5");
    // Integer は Int、Boolean は Bool で渡る（Real で渡すと ET000）。実行時の上書きは構造パラメータを断るので渡さない
    assert_eq!(y_z(&with(&[("n", 3.0)])).1, 3.0);
    assert_eq!(y_z(&with(&[("on", 0.0)])).1, 0.0);
    // 知らない名前と有限でない値は渡さない（実行時の上書きが GM-E10006 で断る）
    assert_eq!(
        y_z(&with(&[("nosuch", 1.0), ("src.startTime", f64::NAN)])).0,
        0.0
    );
    // final の修飾（final startTime = startTime）は上書きできず、変換の診断になる（EI028）
    let overrides = [("src.r.startTime".to_string(), 2.0)].into_iter().collect();
    let d = checker
        .compile_retaining_with_overrides(&sources, name, &overrides)
        .expect_err("final の上書き");
    assert!(d.iter().any(|d| d.message.contains("final")), "{d:#?}");
}

#[test]
fn parameters_that_cannot_change_are_rejected_before_running() {
    let model = compile("Parameters");
    let cases: [(&str, f64, &str); 7] = [
        ("n", 3.0, "パラメータ n は実行の前に変えられない"),
        ("e", 2.0, "パラメータ e は実行の前に変えられない"),
        ("f", 2.0, "パラメータ f は実行の前に変えられない"),
        ("g", 2.0, "パラメータ g は実行の前に変えられない"),
        ("x", 2.0, "このモデルに x という"),
        ("nosuch", 1.0, "このモデルに nosuch という"),
        ("k", f64::NAN, "パラメータ k の上書きの値が有限でない"),
    ];
    for (name, value, expected) in cases {
        for solver in [SolverId::Bdf, SolverId::Dopri45] {
            let s = with_parameters(&model, solver, &[(name, value)]);
            for (route, result) in [
                ("型付き", simulate(&model, &s)),
                ("session", simulate_via_session(&model, &s).map(|(r, _)| r)),
            ] {
                let what = format!("{name} = {value}（{}・{route}）", solver.name());
                let d = result.expect_err(&what);
                assert_eq!(d.len(), 1, "{what}: {d:#?}");
                let d = &d[0];
                assert_eq!(d.code, codes::PARAMETER_OVERRIDE_INVALID, "{what}: {d:#?}");
                assert_eq!(d.stage, Stage::S10, "{what}");
                assert!(d.message.starts_with(expected), "{what}: {}", d.message);
                assert!(
                    d.notes.iter().any(|n| n.starts_with("rumoca: [EX003] ")),
                    "{what}: {d:#?}"
                );
            }
        }
    }
    // run でも同じ（プリフライトは上書きの誤りで黙り、実行が GM-E10006 を出す）
    let report = run(
        &model,
        &SimSettings {
            parameters: [("n".to_string(), 3.0)].into(),
            ..SimSettings::default()
        },
    );
    assert!(!report.succeeded());
    let found: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        found,
        [codes::PARAMETER_OVERRIDE_INVALID],
        "{:#?}",
        report.diagnostics
    );
}

#[test]
fn tunable_parameters_are_listed() {
    let model = compile("Parameters");
    let listed = tunable_parameters(&model).unwrap_or_else(|d| panic!("{d:#?}"));
    let names: Vec<_> = listed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["k", "c"], "{listed:#?}");
    assert_eq!(listed[0].default, 1.0);
    assert_eq!(listed[1].default, 2.0);
    assert_eq!(
        listed[0].description.as_deref(),
        Some("減衰の速さ（tunable）")
    );
    // 状態の無いモデルにはパラメータが無い
    assert!(tunable_parameters(&compile("NoStates")).unwrap().is_empty());
}

#[test]
fn run_passes_parameter_overrides_through() {
    let report = run(
        &compile("Parameters"),
        &SimSettings {
            parameters: [("k".to_string(), 2.0)].into(),
            ..SimSettings::default()
        },
    );
    assert!(report.succeeded(), "{:#?}", report.diagnostics);
    let r = report.run.unwrap();
    assert_close(last(&r, "x"), (-2.0f64).exp(), 1e-6, "x(1)");
}

fn msl_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl")
}

/// MSL の例題を 1 本実行する（ChuaCircuit。状態 3 のカオス回路で、StopTime = 5e4、Interval = 1）。
/// 値はカオスなので比べず、成功すること・点の数・状態の数・NaN が無いことを見る。`data/msl` が無ければ何もしない。
#[test]
#[ignore = "MSL を読む（遅い）。cargo test -p globe-mode-exec --test simulate -- --ignored"]
fn msl_chua_circuit_simulates() {
    let msl = msl_dir();
    if !msl.is_dir() {
        eprintln!("{} が無いので確かめない", msl.display());
        return;
    }
    // テストのスレッドの既定（2 MiB）では、MSL のコンパイルでスタックがあふれることがある（frontend のテストと同じ）
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let checker = Checker::new(&[msl.as_path()]).expect("MSL を読める");
            let name = "Modelica.Electrical.Analog.Examples.ChuaCircuit";
            let started = std::time::Instant::now();
            let model = checker
                .compile_retaining(&[], name)
                .unwrap_or_else(|d| panic!("{d:#?}"));
            let compile_seconds = started.elapsed().as_secs_f64();
            let report = run(
                &model,
                &SimSettings {
                    max_wall_seconds: Some(600.0),
                    ..SimSettings::default()
                },
            );
            assert!(report.succeeded(), "{:#?}", report.diagnostics);
            let r = report.run.unwrap();
            check_trajectory(&r);
            assert_eq!(r.settings.stop_time, 5e4);
            assert_eq!(r.settings.interval, 1.0);
            assert_eq!(r.stats.states, 3);
            assert!(r.stats.points >= 50_001, "{}", r.stats.points);
            assert!(r.trajectory.non_finite().is_empty());
            for v in ["L.i", "C1.v", "C2.v"] {
                assert_eq!(
                    r.trajectory.variable(v).map(|v| v.role),
                    Some(VariableRole::State),
                    "{v}"
                );
            }
            eprintln!(
                "{name}: コンパイル {compile_seconds:.1} 秒、組み立て {:.2} 秒、初期化と積分 {:.2} 秒、{} 行・変数 {}",
                r.stats.build_seconds,
                r.stats.run_seconds,
                r.stats.points,
                r.trajectory.variables.len()
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

/// MSL を読む試験の下ごしらえ（スタックを 64 MiB にしたスレッドで回す）。`data/msl` が無ければ何もしない。
fn with_msl(f: impl FnOnce(&Checker) + Send + 'static) {
    let msl = msl_dir();
    if !msl.is_dir() {
        eprintln!("{} が無いので確かめない", msl.display());
        return;
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || f(&Checker::new(&[msl.as_path()]).expect("MSL を読める")))
        .unwrap()
        .join()
        .unwrap();
}

/// auto の切り替え（S012）: `TwoMass` は BDF の積分で特異な代数ブロックに入って止まる（GM-E11002）が、dopri45 では
/// 終了時刻まで進む（doc/research/msl-sim-failures.md の #38）。
#[test]
#[ignore = "MSL を読む（遅い）。cargo test -p globe-mode-exec --test simulate -- --ignored"]
fn msl_auto_falls_back_to_dopri45_when_bdf_fails_to_integrate() {
    with_msl(|checker| {
        let name = "Modelica.Thermal.FluidHeatFlow.Examples.TwoMass";
        let model = checker
            .compile_retaining(&[], name)
            .unwrap_or_else(|d| panic!("{d:#?}"));
        let requested = |solver: &str| SimSettings {
            solver: Some(solver.to_string()),
            max_wall_seconds: Some(300.0),
            ..SimSettings::default()
        };
        let bdf = run(&model, &requested("bdf"));
        assert!(
            bdf.diagnostics
                .iter()
                .any(|d| d.code == codes::RUN_INTEGRATION_FAILED),
            "{:#?}",
            bdf.diagnostics
        );
        let auto = run(&model, &requested("auto"));
        assert!(auto.succeeded(), "{:#?}", auto.diagnostics);
        let info: Vec<_> = auto
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::RUN_SOLVER_FALLBACK)
            .collect();
        assert_eq!(info.len(), 1, "{:#?}", auto.diagnostics);
        assert_eq!(info[0].severity, Severity::Info);
        assert!(
            info[0].notes[0].starts_with("BDF の失敗: ")
                && info[0].notes[0].ends_with("（GM-E11002）"),
            "{:#?}",
            info[0]
        );
        let r = auto.run.unwrap();
        assert_eq!(r.termination, Termination::StopTime);
        assert!(
            !r.parameters.is_empty(),
            "BDF の lowering から読んだ値が残る"
        );
    });
}

/// 単独のプリフライト（lowering をもう 1 回する。S013 までの `run` はこの分を毎回払っていた）と、`run`（S014 から
/// プリフライトを組み立ての lowering の結果で行う）の時間を比べる（doc/research/msl-sim-failures.md の
/// 「GlobeMode で直すもの」の 4）。数を出すだけで、合否は見ない。
#[test]
#[ignore = "MSL を読む（遅い）。cargo test -p globe-mode-exec --test simulate -- --ignored --nocapture"]
fn msl_preflight_time_against_the_build() {
    with_msl(|checker| {
        for name in [
            "Modelica.Electrical.Analog.Examples.ChuaCircuit",
            "Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum",
            "Modelica.Fluid.Examples.HeatingSystem",
        ] {
            let Ok(model) = checker.compile_retaining(&[], name) else {
                eprintln!("{name}: 変換できない");
                continue;
            };
            let started = std::time::Instant::now();
            let report = run(
                &model,
                &SimSettings {
                    stop_time: Some(1e-3),
                    max_wall_seconds: Some(60.0),
                    ..SimSettings::default()
                },
            );
            let run_seconds = started.elapsed().as_secs_f64();
            let Some(s) = report.settings.clone() else {
                continue;
            };
            let started = std::time::Instant::now();
            let _ = preflight(&model, &s);
            let preflight_seconds = started.elapsed().as_secs_f64();
            let build = report
                .run
                .as_ref()
                .map(|r| format!("{:.3} 秒", r.stats.build_seconds))
                .unwrap_or_else(|| "失敗".to_string());
            eprintln!(
                "{name}: run {run_seconds:.3} 秒（組み立て {build}）、単独のプリフライト {preflight_seconds:.3} 秒"
            );
        }
    });
}

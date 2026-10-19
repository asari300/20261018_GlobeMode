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
use globe_mode_exec::{CompiledModel, preflight, run, simulate};
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
        origins: SettingOrigins {
            start_time: o,
            stop_time: o,
            tolerance: o,
            interval: o,
            solver: o,
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

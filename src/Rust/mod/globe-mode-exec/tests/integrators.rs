//! 自前の積分器（S012。`esdirk34`・`trbdf2`・`tsit45`・`euler`・`heun`・`rk4`）を、`globe_mode_exec::run` を通して
//! src/Modelica/GlobeModeValidation/Simulation のモデルで確かめる。
//!
//! - Decay（x' = -x）: euler・dt 0.1 なら x(1) = 0.9^10。刻み幅の制御のあるものと rk4 は exp(-1) に近い。
//! - HarmonicOscillator: 解析解 x = cos(2t)、v = -2 sin(2t)。
//! - BouncingBall: 跳ねた回数と最後の高さを、bdf の結果と比べる（イベントの後も格子に戻る）。
//!
//! MSL の例題での確かめ（`#[ignore]`）は data/msl と data/msl-ref が要る。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globe_mode_exec::{CompiledModel, SimReport, run};
use globe_mode_frontend::{Checker, Source};
use globe_mode_ir::{SimRun, SimSettings, SolverId, Termination};

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

/// `run` で実行する（区間・許容誤差は experiment annotation のまま。`step` は固定刻みの幅）。
fn run_with(model: &CompiledModel, solver: SolverId, step: Option<f64>) -> SimRun {
    let requested = SimSettings {
        solver: Some(solver.name().to_owned()),
        step,
        max_wall_seconds: Some(60.0),
        ..SimSettings::default()
    };
    let report: SimReport = run(model, &requested);
    assert!(
        report.succeeded(),
        "{}（{}）が失敗した: {:#?}",
        model.model,
        solver.name(),
        report.diagnostics
    );
    let run = report.run.unwrap();
    assert_eq!(run.settings.solver, solver);
    assert_eq!(run.termination, Termination::StopTime);
    assert_eq!(*run.trajectory.time.last().unwrap(), run.settings.stop_time);
    assert!(
        run.trajectory.time.windows(2).all(|w| w[0] <= w[1]),
        "時刻が減っている"
    );
    for v in &run.trajectory.variables {
        assert_eq!(
            v.values.len(),
            run.trajectory.time.len(),
            "{} の長さ",
            v.name
        );
    }
    run
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
    eprintln!(
        "{what}: {actual:.15}（期待 {expected:.15}、差 {:.3e}）",
        actual - expected
    );
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: {actual} と {expected} の差 {} が {tol} を超える",
        (actual - expected).abs()
    );
}

const ADAPTIVE: [SolverId; 3] = [SolverId::Tsit45, SolverId::Esdirk34, SolverId::Trbdf2];

#[test]
fn euler_on_decay_is_the_geometric_sequence() {
    let model = compile("Decay");
    let run = run_with(&model, SolverId::Euler, Some(0.1));
    assert_eq!(run.settings.step, Some(0.1));
    assert_close(last(&run, "x"), 0.9f64.powi(10), 1e-14, "euler x(1)");
    // 格子の点（0.1 刻み）では Euler の値そのもの。間の出力の点は Hermite 補間
    let t = &run.trajectory.time;
    let x = &run.trajectory.variable("x").unwrap().values;
    for k in 1..=10 {
        let tk = f64::from(k) * 0.1;
        let i = t
            .iter()
            .position(|&ti| (ti - tk).abs() < 1e-12)
            .unwrap_or_else(|| panic!("t = {tk} の行が無い"));
        assert!(
            (x[i] - 0.9f64.powi(k)).abs() < 1e-14,
            "x({tk}) = {}（期待 {}）",
            x[i],
            0.9f64.powi(k)
        );
    }
}

#[test]
fn fixed_step_methods_converge_on_decay_with_their_order() {
    let model = compile("Decay");
    let exact = (-1.0f64).exp();
    for (solver, order) in [
        (SolverId::Euler, 1),
        (SolverId::Heun, 2),
        (SolverId::Rk4, 4),
    ] {
        let coarse = (last(&run_with(&model, solver, Some(0.1)), "x") - exact).abs();
        let fine = (last(&run_with(&model, solver, Some(0.05)), "x") - exact).abs();
        let observed = (coarse / fine).log2();
        eprintln!(
            "{}: 誤差 {coarse:.3e}（h 0.1）、{fine:.3e}（h 0.05）、次数 {observed:.3}",
            solver.name()
        );
        assert!(
            (observed - f64::from(order)).abs() < 0.2,
            "{} の大域誤差の次数 {observed}（期待 {order}）",
            solver.name()
        );
    }
    // rk4・h 0.1 は 1e-6 より近い
    let rk4 = run_with(&model, SolverId::Rk4, Some(0.1));
    assert_close(last(&rk4, "x"), exact, 1e-6, "rk4 x(1)");
}

#[test]
fn adaptive_methods_match_decay() {
    let model = compile("Decay");
    for solver in ADAPTIVE {
        let run = run_with(&model, solver, None);
        // experiment の Tolerance = 1e-8
        assert_close(
            last(&run, "x"),
            (-1.0f64).exp(),
            1e-6,
            &format!("{} x(1)", solver.name()),
        );
        // 出力の間隔は区間の 1/500（初めの行を含めて 501 行）
        assert_eq!(run.stats.points, 501, "{}", solver.name());
    }
}

#[test]
fn oscillator_matches_the_analytic_solution() {
    let model = compile("HarmonicOscillator");
    let (x1, v1) = (2.0f64.cos(), -2.0 * 2.0f64.sin());
    for solver in ADAPTIVE {
        let run = run_with(&model, solver, None);
        assert_eq!(run.stats.states, 2);
        assert_close(
            last(&run, "x"),
            x1,
            1e-5,
            &format!("{} x(1)", solver.name()),
        );
        assert_close(
            last(&run, "v"),
            v1,
            1e-5,
            &format!("{} v(1)", solver.name()),
        );
    }
    // 固定刻み（h 0.01）。rk4 は 4 次、heun は 2 次、euler は 1 次（エネルギーが増える）
    for (solver, tol) in [
        (SolverId::Rk4, 1e-8),
        (SolverId::Heun, 1e-3),
        (SolverId::Euler, 5e-2),
    ] {
        let run = run_with(&model, solver, Some(0.01));
        assert_close(last(&run, "x"), x1, tol, &format!("{} x(1)", solver.name()));
        assert_close(last(&run, "v"), v1, tol, &format!("{} v(1)", solver.name()));
    }
}

/// 速度の反転（左極限 v < 0 の行の直後に v > 0 の行）の数。
fn jumps(run: &SimRun) -> usize {
    let v = &run.trajectory.variable("v").unwrap().values;
    let t = &run.trajectory.time;
    (1..t.len())
        .filter(|&i| v[i - 1] < 0.0 && v[i] > 0.0 && t[i] - t[i - 1] < 1e-9)
        .count()
}

#[test]
fn bouncing_ball_matches_bdf() {
    let model = compile("BouncingBall");
    let bdf = run_with(&model, SolverId::Bdf, None);
    assert_eq!(last(&bdf, "n"), 3.0);
    let h_bdf = last(&bdf, "h");
    // 3 回目の後の放物線（解析解 h(2) ≈ 0.260739）
    for (solver, step, tol) in [
        (SolverId::Tsit45, None, 1e-4),
        (SolverId::Esdirk34, None, 1e-4),
        (SolverId::Trbdf2, None, 1e-4),
        // 固定刻み（h 0.01）。重力だけの区間は 2 次の多項式なので heun・rk4 は区間の中では正確で、
        // 誤差はイベントの位置（根探索の許容誤差）だけ
        (SolverId::Rk4, Some(0.01), 1e-4),
        (SolverId::Heun, Some(0.01), 1e-4),
        (SolverId::Euler, Some(0.001), 5e-2),
    ] {
        let run = run_with(&model, solver, step);
        let name = solver.name();
        assert_eq!(last(&run, "n"), 3.0, "{name} の跳ねた回数");
        assert_eq!(jumps(&run), 3, "{name} の速度の反転の数");
        let h = &run.trajectory.variable("h").unwrap().values;
        let lowest = h.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(lowest > -1e-3, "{name}: h の最小 {lowest}");
        assert_close(
            last(&run, "h"),
            h_bdf,
            tol,
            &format!("{name} h(2)（bdf と）"),
        );
        assert_close(
            last(&run, "h"),
            0.260739,
            tol + 1e-4,
            &format!("{name} h(2)（解析解と）"),
        );
    }
}

/// 固定刻みは、イベントで格子の外に止まった後も t0 + k·h の格子に戻る（出力の格子と刻みを揃えると、
/// イベントの行の他は格子の上に乗る）。
#[test]
fn fixed_step_returns_to_its_grid_after_events() {
    let model = compile("BouncingBall");
    let run = run_with(&model, SolverId::Rk4, Some(0.004));
    let t = &run.trajectory.time;
    let off_grid = t
        .iter()
        .filter(|&&ti| {
            let k = (ti / 0.004).round();
            (ti - k * 0.004).abs() > 1e-12
        })
        .count();
    // イベント 3 回。host は 1 回に 3 行を書く（左極限、イベントの後、その数 ulp 後。時刻はどれも格子の外）
    assert!(off_grid <= 9, "格子の外の行が {off_grid} 行");
    assert_eq!(t.len(), 501 + off_grid, "行の数");
}

/// 固定刻みの幅が無ければ出力の間隔を使う。
#[test]
fn fixed_step_defaults_to_the_output_interval() {
    let model = compile("Decay");
    let run = run_with(&model, SolverId::Rk4, None);
    assert_eq!(run.settings.step, Some(1.0 / 500.0));
    assert_close(
        last(&run, "x"),
        (-1.0f64).exp(),
        1e-12,
        "rk4 x(1)（h 1/500）",
    );
}

/// SDIRK は Newton 法の失敗が積分全体で 50 回を超えても止まらない（diffsol の累積の上限を外した。S021）。
/// 矩形波の電流でダイオードを 98 回切り替えると、直す前の esdirk34 は途中で
/// 「Exceeded maximum number of nonlinear solver failures (50)」で止まった。
#[test]
fn sdirk_survives_more_than_fifty_newton_failures() {
    let model = compile("DiodeSwitching");
    let bdf = run_with(&model, SolverId::Bdf, None);
    // trbdf2 も同じ直しで通る（release で 1.7 s、v の差 2e-6）が、debug では 60 s の予算を超えるので回さない
    let run = run_with(&model, SolverId::Esdirk34, None);
    assert_eq!(last(&run, "k"), 98.0, "esdirk34 の切り替えの回数");
    // S022 でヤコビ行列を刻みの始めの状態で取るようにして、bdf との差は 1.3e-5 から 1e-6 以下になった
    assert_close(
        last(&run, "v"),
        last(&bdf, "v"),
        1e-5,
        "esdirk34 v(0.0985)（bdf と）",
    );
}

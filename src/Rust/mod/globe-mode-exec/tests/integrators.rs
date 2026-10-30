//! 自前の積分器（S012。`esdirk34`・`trbdf2`・`tsit45`・`euler`・`heun`・`rk4`。S026 の `impeuler`・`trapezoid`。S027 の `esdirk23`・`esdirk45`・`sdirk34hw`。S028 の `dopri853`。S029 の `cerk23`・`cerk34`・`cerk45`。S030 の `radau5`。S031 の `qss`。feature `sundials` の S032 の `cvode`、S033 の `ida`）を、`globe_mode_exec::run` を通して
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

const ADAPTIVE: &[SolverId] = &[
    SolverId::Tsit45,
    SolverId::Dopri853,
    SolverId::Esdirk34,
    SolverId::Trbdf2,
    SolverId::Esdirk23,
    SolverId::Esdirk45,
    SolverId::Sdirk34hw,
    SolverId::Cerk23,
    SolverId::Cerk34,
    SolverId::Cerk45,
    SolverId::Radau5,
    SolverId::Qss,
    // S032: SUNDIALS の CVODE（feature `sundials` のときだけ）
    #[cfg(feature = "sundials")]
    SolverId::Cvode,
    // S033: SUNDIALS の IDA（残差 y' − f。feature `sundials` のときだけ）
    #[cfg(feature = "sundials")]
    SolverId::Ida,
];

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

/// 陰的 Euler 法・台形公式は、x' = -x で 1 歩の増幅率 1/(1+h)・(1-h/2)/(1+h/2) の等比数列になる
/// （Newton 法が線形の段の方程式を解き切る）。
#[test]
fn implicit_fixed_step_methods_on_decay_are_geometric_sequences() {
    let model = compile("Decay");
    let h = 0.1;
    for (solver, ratio) in [
        (SolverId::Impeuler, 1.0 / (1.0 + h)),
        (SolverId::Trapezoid, (1.0 - h / 2.0) / (1.0 + h / 2.0)),
    ] {
        let run = run_with(&model, solver, Some(h));
        assert_close(
            last(&run, "x"),
            f64::powi(ratio, 10),
            1e-13,
            &format!("{} x(1)", solver.name()),
        );
    }
}

/// 硬い系（x' = -1000 x、h 0.1 は h·λ = -100）。陽的 Euler 法は発散するが、陰的 Euler 法・台形公式は
/// 安定で、陰的 Euler 法は 0 に潰れる（増幅率 1/101）。台形公式は A 安定だが L 安定でなく振動して減る（増幅率 -49/51）。
#[test]
fn implicit_fixed_step_methods_are_stable_on_a_stiff_decay() {
    let model = compile("Parameters");
    let stiff = |solver: SolverId| {
        let requested = SimSettings {
            solver: Some(solver.name().to_owned()),
            step: Some(0.1),
            parameters: [("k".to_owned(), 1000.0)].into(),
            ..SimSettings::default()
        };
        let report = run(&model, &requested);
        (report.succeeded(), report.run)
    };
    for (solver, ratio) in [
        (SolverId::Impeuler, 1.0 / 101.0),
        (SolverId::Trapezoid, -49.0 / 51.0),
    ] {
        let (ok, run) = stiff(solver);
        assert!(ok, "{}", solver.name());
        assert_close(
            last(&run.unwrap(), "x"),
            f64::powi(ratio, 10),
            1e-13,
            &format!("{} x(1)", solver.name()),
        );
    }
    let (_, euler) = stiff(SolverId::Euler);
    // 陽的 Euler 法の増幅率は 1 - 100 = -99、10 歩で (-99)^10。失敗するか、巨大な値になる
    assert!(euler.is_none_or(|r| last(&r, "x").abs() > 1e10));
}

#[test]
fn fixed_step_methods_converge_on_decay_with_their_order() {
    let model = compile("Decay");
    let exact = (-1.0f64).exp();
    for (solver, order) in [
        (SolverId::Euler, 1),
        (SolverId::Heun, 2),
        (SolverId::Rk4, 4),
        (SolverId::Impeuler, 1),
        (SolverId::Trapezoid, 2),
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
    for &solver in ADAPTIVE {
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
    for &solver in ADAPTIVE {
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
        (SolverId::Trapezoid, 1e-3),
        (SolverId::Impeuler, 5e-2),
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
        (SolverId::Dopri853, None, 1e-4),
        (SolverId::Esdirk34, None, 1e-4),
        (SolverId::Trbdf2, None, 1e-4),
        (SolverId::Esdirk23, None, 1e-4),
        (SolverId::Esdirk45, None, 1e-4),
        (SolverId::Sdirk34hw, None, 1e-4),
        (SolverId::Cerk23, None, 1e-4),
        (SolverId::Cerk34, None, 1e-4),
        (SolverId::Cerk45, None, 1e-4),
        (SolverId::Radau5, None, 1e-4),
        (SolverId::Qss, None, 1e-4),
        #[cfg(feature = "sundials")]
        (SolverId::Cvode, None, 1e-4),
        #[cfg(feature = "sundials")]
        (SolverId::Ida, None, 1e-4),
        // 固定刻み（h 0.01）。重力だけの区間は 2 次の多項式なので heun・rk4 は区間の中では正確で、
        // 誤差はイベントの位置（根探索の許容誤差）だけ
        (SolverId::Rk4, Some(0.01), 1e-4),
        (SolverId::Heun, Some(0.01), 1e-4),
        (SolverId::Euler, Some(0.001), 5e-2),
        // 台形公式は導関数が 1 次式なら正確（heun と同じ）。陰的 Euler 法は 1 次
        (SolverId::Trapezoid, Some(0.01), 1e-4),
        (SolverId::Impeuler, Some(0.001), 5e-2),
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

/// ほぼ 0 の状態から大きな導関数で再開しても、RK は最初の刻みを下限より上に取って進む（S023）。
/// 直す前は esdirk34・trbdf2・tsit45 とも t = 1 の再開で「Step size is too small」で止まった。
#[test]
fn rk_restarts_from_a_tiny_state_with_a_large_derivative() {
    let model = compile("TinyStateRestart");
    for &solver in ADAPTIVE {
        let run = run_with(&model, solver, None);
        assert_close(
            last(&run, "x"),
            1e5,
            1e-6,
            &format!("{} x(2)", solver.name()),
        );
    }
}

/// `dopri853` の主解は 8 次なので、刻み幅を 5 次の埋め込みの見積もりで決めても、同じ許容誤差（1e-8）で
/// `tsit45`（誤差 1e-10〜1e-8）よりずっと正確になる（S028。実測は Decay・Oscillator とも 1e-12 台）。
#[test]
fn dopri853_is_far_more_accurate_than_tsit45_at_the_same_tolerance() {
    let decay = compile("Decay");
    let oscillator = compile("HarmonicOscillator");
    let (x1, v1) = (2.0f64.cos(), -2.0 * 2.0f64.sin());
    let errors = |solver| {
        let d = run_with(&decay, solver, None);
        let o = run_with(&oscillator, solver, None);
        [
            (last(&d, "x") - (-1.0f64).exp()).abs(),
            (last(&o, "x") - x1).abs(),
            (last(&o, "v") - v1).abs(),
        ]
    };
    let (tsit, dop) = (errors(SolverId::Tsit45), errors(SolverId::Dopri853));
    for (t, d) in tsit.iter().zip(dop) {
        assert!(d < 1e-10, "dopri853 の誤差 {d}");
        assert!(
            d * 10.0 < *t,
            "dopri853 {d} は tsit45 {t} の 1/10 より小さいはず"
        );
    }
}

/// `cerk23`・`cerk34`・`cerk45`（S029）は、出力の点（区間の 1/500）で刻みの間を自前の連続の補間
/// （主解と同じ次数）から取る。Decay の出力すべての行が解析解 exp(-t) に、許容誤差（1e-8）に見合う精度で乗る。
/// 実測の最大誤差は cerk23 1.8e-9、cerk34 9.2e-10、cerk45 8.4e-10。
#[test]
fn cerk_dense_output_tracks_the_analytic_solution_at_every_output_row() {
    let model = compile("Decay");
    for (solver, tol) in [
        (SolverId::Cerk23, 1e-6),
        (SolverId::Cerk34, 1e-7),
        (SolverId::Cerk45, 1e-7),
    ] {
        let run = run_with(&model, solver, None);
        let t = &run.trajectory.time;
        let x = &run.trajectory.variable("x").unwrap().values;
        let worst = t
            .iter()
            .zip(x)
            .map(|(t, x)| (x - (-t).exp()).abs())
            .fold(0.0, f64::max);
        eprintln!("{} の全行の最大誤差 {worst:.3e}", solver.name());
        assert!(worst < tol, "{} の最大誤差 {worst}", solver.name());
    }
}

//! Reference-FMUs（v0.0.42、`cargo xtask fetch-reference-fmus`。2-Clause BSD）の Model Exchange を、この crate の積分器
//! （[`globe_mode_fmi_import::run_me`]）で回す（S205、ADR-0026 (l)）。CS と違い、積分器が FMU の外（こちら）にあるので、
//! 参照の CSV（FMU の CS を刻み 0.01 などで回した前進 Euler）とはビット一致しない。**解析解**を主な相手にし、CSV は目安として比べる。
//! 取得物が無ければ飛ばす（`GLOBE_REQUIRE_REFERENCE_FMUS` があれば落とす）。

use std::io::Read;
use std::path::PathBuf;

use globe_mode_fmi_import::{EventKind, Fmu, MeIntegrator, MeOptions, MeRun, run_me};

const VERSIONS: [&str; 2] = ["2.0", "3.0"];

fn fmu_path(version: &str, name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../data/reference-fmus")
        .join(version)
        .join(format!("{name}.fmu"));
    if p.is_file() {
        Some(p)
    } else {
        assert!(
            std::env::var_os("GLOBE_REQUIRE_REFERENCE_FMUS").is_none(),
            "{p:?} が無い"
        );
        eprintln!("skip: {p:?} が無い（cargo xtask fetch-reference-fmus）");
        None
    }
}

/// 参照 CSV（時刻の列が先頭）。
fn reference_csv(path: &std::path::Path, name: &str) -> Vec<Vec<f64>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut text = String::new();
    zip.by_name(&format!("extra/org.fmi-standard.fmi-ls-ref/{name}_out.csv"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text.lines()
        .skip(1)
        .map(|l| l.split(',').map(|c| c.trim().parse().unwrap()).collect())
        .collect()
}

fn run(version: &str, name: &str, opts: MeOptions) -> Option<(Fmu, MeRun)> {
    let fmu = Fmu::open(&fmu_path(version, name)?).unwrap();
    let r = run_me(&fmu, &opts).unwrap();
    Some((fmu, r))
}

/// `run` の各行と、`want(t)` の各列の最大の絶対差。
fn max_abs_diff(r: &MeRun, want: impl Fn(f64) -> Vec<f64>) -> f64 {
    r.rows
        .iter()
        .flat_map(|(t, v)| v.iter().zip(want(*t)).map(|(g, w)| (g - w).abs()))
        .fold(0.0, f64::max)
}

#[test]
fn dahlquist_matches_the_analytic_solution() {
    // x' = -k x、k = 1、x(0) = 1（DefaultExperiment: 0〜10、刻み 0.1）。解析解 exp(-t)
    for version in VERSIONS {
        // 許容誤差 1e-9 の誤差制御つき: 最大の絶対誤差 5e-9 未満（実測は 1e-9 台）
        let Some((_, r)) = run(
            version,
            "Dahlquist",
            MeOptions {
                tolerance: Some(1e-9),
                ..Default::default()
            },
        ) else {
            return;
        };
        let e = max_abs_diff(&r, |t| vec![(-t).exp()]);
        eprintln!(
            "Dahlquist {version} dopri5 rtol=1e-9: 最大の絶対誤差 {e:e}、歩数 {}",
            r.steps
        );
        assert!(e < 5e-9, "{version}: {e}");
        assert!(r.events.is_empty());
        // 既定（DefaultExperiment に tolerance が無いので 1e-6）: 5e-6 未満
        let (_, r) = run(version, "Dahlquist", MeOptions::default()).unwrap();
        let e = max_abs_diff(&r, |t| vec![(-t).exp()]);
        eprintln!("Dahlquist {version} dopri5 既定の許容誤差: 最大の絶対誤差 {e:e}");
        assert!(e < 5e-6, "{version}: {e}");
        // 固定刻みの RK4（刻み 0.01）: 4 次なので誤差は h^4 の桁。2e-10 未満
        let (_, r) = run(
            version,
            "Dahlquist",
            MeOptions {
                integrator: MeIntegrator::Rk4,
                step: Some(0.01),
                ..Default::default()
            },
        )
        .unwrap();
        let e = max_abs_diff(&r, |t| vec![(-t).exp()]);
        eprintln!("Dahlquist {version} rk4 h=0.01: 最大の絶対誤差 {e:e}");
        assert!(e < 2e-10, "{version}: {e}");
    }
}

#[test]
fn dahlquist_parameter_k_changes_the_decay() {
    for version in VERSIONS {
        let Some((_, r)) = run(
            version,
            "Dahlquist",
            MeOptions {
                stop_time: Some(2.0),
                set: vec![("k".into(), 3.0)],
                tolerance: Some(1e-10),
                ..Default::default()
            },
        ) else {
            return;
        };
        let e = max_abs_diff(&r, |t| vec![(-3.0 * t).exp()]);
        assert!(e < 1e-8, "{version}: {e}");
    }
}

/// BouncingBall の解析解。h(0) = 1、v(0) = 0、g = 9.81、反発係数 e = 0.7、衝突の後の速さが 0.1 を下回ったら止まる。
/// `(衝突の時刻の列, h(t))`。
fn bouncing_ball() -> (Vec<f64>, impl Fn(f64) -> f64) {
    let (g, e, v_min): (f64, f64, f64) = (9.81, 0.7, 0.1);
    let mut times = vec![(2.0f64 / g).sqrt()];
    let mut speeds = vec![g * times[0]]; // 衝突の直前の速さ
    loop {
        let v = e * speeds.last().unwrap();
        if v < v_min {
            break;
        }
        times.push(times.last().unwrap() + 2.0 * v / g);
        speeds.push(v);
    }
    let (t, s) = (times.clone(), speeds.clone());
    let h = move |time: f64| {
        // 最後に起きた衝突（あれば）の後の放物運動。止まった後は 0
        let k = t.iter().rposition(|tk| *tk <= time);
        match k {
            None => 1.0 - 0.5 * g * time * time,
            Some(k) if k + 1 == t.len() => 0.0, // 最後の衝突の後は止まる（速さが v_min 未満になる）
            Some(k) => {
                let (dt, v) = (time - t[k], e * s[k]);
                v * dt - 0.5 * g * dt * dt
            }
        }
    };
    (times, h)
}

#[test]
fn bouncing_ball_events_match_the_analytic_bounces() {
    let (times, h) = bouncing_ball();
    assert_eq!(times.len(), 11, "解析解の衝突の回数");
    for version in VERSIONS {
        let Some((_, r)) = run(version, "BouncingBall", MeOptions::default()) else {
            return;
        };
        // 衝突は state event。回数と時刻
        assert_eq!(r.events.len(), 11, "{version}: 衝突の回数 {:?}", r.events);
        assert!(r.events.iter().all(|e| e.kind == EventKind::State));
        let dt = r
            .events
            .iter()
            .zip(&times)
            .map(|(e, t)| (e.time - t).abs())
            .fold(0.0, f64::max);
        eprintln!("BouncingBall {version}: 衝突の時刻の最大の誤差 {dt:e}");
        assert!(dt < 1e-9, "{version}: {dt}");
        // 高さ（出力の列は h, v の順。止まった後は DBL_MIN なので絶対差で比べる）
        let col_h = |t: f64| vec![h(t)];
        let max_h = r
            .rows
            .iter()
            .map(|(t, v)| (v[0] - col_h(*t)[0]).abs())
            .fold(0.0, f64::max);
        eprintln!("BouncingBall {version}: 高さの最大の絶対誤差 {max_h:e}");
        assert!(max_h < 1e-8, "{version}: {max_h}");
        // 止まった後（t = 3）は h ≈ 0、v = 0
        let last = r.rows.last().unwrap();
        assert_eq!(last.0, 3.0);
        assert!(last.1[0].abs() < 1e-300 && last.1[1] == 0.0, "{last:?}");
    }
}

#[test]
fn bouncing_ball_bounces_fall_in_the_reference_csv_intervals() {
    // 参照の CSV は FMU の CS（前進 Euler、刻み 0.01）で、衝突の時刻が格子に丸まるので、点ごとの比較（衝突の前後で v の符号が
    // 逆になる）は意味がない。CSV の v が負から正に変わる区間 [t_i, t_i+1] を衝突の区間とみなし、
    // こちらの state event の時刻との差を見る。回数も同じ
    for version in VERSIONS {
        let Some(path) = fmu_path("3.0", "BouncingBall") else {
            return;
        };
        let rows = reference_csv(&path, "BouncingBall");
        let (_, r) = run(version, "BouncingBall", MeOptions::default()).unwrap();
        assert_eq!(r.rows.len(), rows.len());
        let csv_bounces: Vec<f64> = rows
            .windows(2)
            .filter(|w| w[0][2] < 0.0 && w[1][2] > 0.0)
            .map(|w| w[1][0])
            .collect();
        assert_eq!(csv_bounces.len(), r.events.len(), "{version}: 衝突の回数");
        let diffs: Vec<f64> = r
            .events
            .iter()
            .zip(&csv_bounces)
            .map(|(e, c)| c - e.time)
            .collect();
        eprintln!("BouncingBall {version}: 差の列 {diffs:?}");
        // 1 回目は格子 1 つ（0.01）の内。以降は、前進 Euler の遅れが衝突ごとに積もって 0.1 までずれる（実測の最大は 0.081）。
        // CSV の区間の終わりを取るので、差は負にならない
        assert!(diffs[0] <= 0.01 + 1e-9, "{version}: {diffs:?}");
        assert!(
            diffs.iter().all(|d| (0.0..0.1).contains(d)),
            "{version}: {diffs:?}"
        );
    }
}

#[test]
fn van_der_pol_agrees_across_integrators_and_with_the_early_reference_csv() {
    for version in VERSIONS {
        // 独立した 2 つの積分（RK4 刻み 1e-3 と、誤差制御つき 1e-10）が 1e-7 で一致する
        let Some((_, fine)) = run(
            version,
            "VanDerPol",
            MeOptions {
                integrator: MeIntegrator::Rk4,
                step: Some(1e-3),
                ..Default::default()
            },
        ) else {
            return;
        };
        let (_, tight) = run(
            version,
            "VanDerPol",
            MeOptions {
                step: Some(1e-3),
                tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(fine.rows.len(), tight.rows.len());
        let d = fine
            .rows
            .iter()
            .zip(&tight.rows)
            .flat_map(|(a, b)| a.1.iter().zip(&b.1).map(|(x, y)| (x - y).abs()))
            .fold(0.0, f64::max);
        eprintln!("VanDerPol {version}: rk4(1e-3) と dopri5(1e-10) の最大の絶対差 {d:e}");
        assert!(d < 1e-7, "{version}: {d}");
        // 既定の設定（刻み 0.01、許容誤差 1e-6）と、tight との差が 1e-4 未満
        let (_, def) = run(version, "VanDerPol", MeOptions::default()).unwrap();
        let d = def
            .rows
            .iter()
            .zip(tight.rows.iter().step_by(10))
            .flat_map(|(a, b)| a.1.iter().zip(&b.1).map(|(x, y)| (x - y).abs()))
            .fold(0.0, f64::max);
        eprintln!("VanDerPol {version}: 既定の設定と dopri5(1e-10) の最大の絶対差 {d:e}");
        assert!(d < 1e-4, "{version}: {d}");
        // 参照の CSV（CS の前進 Euler、刻み 0.01）。Euler の大域誤差は時刻とともに増える（極限周期軌道の位相がずれる）ので、
        // 最初の 2 秒だけを 0.05 未満で比べ、全体の差は目安として出す
        let rows = reference_csv(&fmu_path("3.0", "VanDerPol").unwrap(), "VanDerPol");
        assert_eq!(def.rows.len(), rows.len());
        let diff = |from: f64, to: f64| {
            def.rows
                .iter()
                .zip(&rows)
                .filter(|((t, _), _)| (from..=to).contains(t))
                .map(|((_, v), c)| (v[0] - c[1]).abs().max((v[1] - c[2]).abs()))
                .fold(0.0, f64::max)
        };
        eprintln!(
            "VanDerPol {version}: 参照の CSV との最大の絶対差 0〜2 s で {:e}、全体で {:e}",
            diff(0.0, 2.0),
            diff(0.0, 20.0)
        );
        assert!(diff(0.0, 2.0) < 0.05, "{version}");
    }
}

#[test]
fn stair_has_time_events_each_second() {
    // Stair は連続状態が無く、`nextEventTime` で 1 秒ごとに counter を増やす（time event）。counter は Integer なので
    // 出力には出ない（Float64 だけ）。事象の時刻と種類を見る
    for version in VERSIONS {
        let Some((_, r)) = run(version, "Stair", MeOptions::default()) else {
            return;
        };
        let times: Vec<f64> = r.events.iter().map(|e| e.time).collect();
        assert_eq!(
            times,
            (1..=9).map(f64::from).collect::<Vec<_>>(),
            "{version}"
        );
        assert!(r.events.iter().all(|e| e.kind == EventKind::Time));
        // counter が 10 になると FMU が terminateSimulation を求める
        assert!(r.terminated_by_fmu, "{version}");
        assert_eq!(r.rows.last().unwrap().0, 9.0);
    }
}

#[test]
fn errors_are_reported_not_panics() {
    let Some(path) = fmu_path("3.0", "Dahlquist") else {
        return;
    };
    let fmu = Fmu::open(&path).unwrap();
    for (opts, text) in [
        (
            MeOptions {
                outputs: vec!["nope".into()],
                ..Default::default()
            },
            "nope",
        ),
        (
            MeOptions {
                stop_time: Some(0.0),
                ..Default::default()
            },
            "終了時刻",
        ),
        (
            MeOptions {
                step: Some(-1.0),
                ..Default::default()
            },
            "刻み",
        ),
        // S206: 出力の格子が細かすぎる（1 億点）なら、実行の前に断る
        (
            MeOptions {
                stop_time: Some(100.0),
                step: Some(1e-6),
                ..Default::default()
            },
            "--step",
        ),
        (
            MeOptions {
                tolerance: Some(0.0),
                ..Default::default()
            },
            "許容誤差",
        ),
    ] {
        let e = run_me(&fmu, &opts).unwrap_err();
        assert!(e.to_string().contains(text), "{e}");
    }
}

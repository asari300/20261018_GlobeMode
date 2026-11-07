//! `globe fmu info` / `globe fmu run`（S204、P5 の FMI Import の最初の一切れ。S205 で Model Exchange の `--kind me`）を実際に起動して確かめる。
//!
//! - Reference-FMUs（`cargo xtask fetch-reference-fmus`。無ければ飛ばす）の要約と実行
//! - **往復**: `globe fmu export --build` で作った Decay の FMU（P5a、Rumoca のテンプレート）を、自分の `globe fmu run` で
//!   読み込んで x(1) = e⁻¹ になる（cc・uv・FMPy（ヘッダのため）が無ければ飛ばす。Linux の x86_64 だけ）

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn reference_fmu(version: &str, name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../data/reference-fmus")
        .join(version)
        .join(format!("{name}.fmu"));
    if p.is_file() {
        Some(p)
    } else {
        // fmu-import.yml は取得に失敗したら飛ばさず落とす
        assert!(
            std::env::var_os("GLOBE_REQUIRE_REFERENCE_FMUS").is_none(),
            "{p:?} が無い"
        );
        eprintln!("skip: {p:?} が無い（cargo xtask fetch-reference-fmus）");
        None
    }
}

fn globe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .output()
        .expect("run globe")
}

/// CSV の最後の行（時刻, 値...）。
fn last_row(csv: &str) -> Vec<f64> {
    csv.lines()
        .last()
        .unwrap()
        .split(',')
        .map(|c| c.parse().unwrap())
        .collect()
}

#[test]
fn info_summarises_without_running_anything() {
    let Some(fmu) = reference_fmu("3.0", "Dahlquist") else {
        return;
    };
    let o = globe(&["fmu", "info", fmu.to_str().unwrap()]);
    assert!(o.status.success(), "{o:?}");
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("Dahlquist（FMI 3.0）"), "{out}");
    assert!(out.contains("CoSimulation: Dahlquist"), "{out}");
    assert!(out.contains("k (Parameter, Float64)"), "{out}");
    assert!(!String::from_utf8_lossy(&o.stderr).contains("ネイティブコード"));
}

#[test]
fn info_shows_model_exchange_states_and_event_indicators() {
    for (version, name, text) in [
        (
            "2.0",
            "BouncingBall",
            "ModelExchange: BouncingBall（連続状態 2、event indicator 1）",
        ),
        (
            "3.0",
            "BouncingBall",
            "ModelExchange: BouncingBall（連続状態 2、event indicator 1）",
        ),
        (
            "3.0",
            "Stair",
            "ModelExchange: Stair（連続状態 0、event indicator 0）",
        ),
    ] {
        let Some(fmu) = reference_fmu(version, name) else {
            return;
        };
        let o = globe(&["fmu", "info", fmu.to_str().unwrap()]);
        assert!(o.status.success(), "{o:?}");
        let out = String::from_utf8_lossy(&o.stdout);
        assert!(out.contains(text), "{version} {name}: {out}");
        assert!(out.contains("CoSimulation: "), "{out}");
    }
}

#[test]
fn run_model_exchange_reports_events_and_matches_the_analytic_solution() {
    for version in ["2.0", "3.0"] {
        let Some(fmu) = reference_fmu(version, "BouncingBall") else {
            return;
        };
        let o = globe(&["fmu", "run", fmu.to_str().unwrap(), "--kind", "me"]);
        assert!(o.status.success(), "{o:?}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains("ネイティブコード"), "{err}");
        assert!(err.contains("事象 11（state 11、time 0、step 0）"), "{err}");
        let out = String::from_utf8_lossy(&o.stdout);
        assert!(out.starts_with("time,h,v\n"), "{out}");
        assert_eq!(out.lines().count(), 302, "301 行 + ヘッダ");
        let row = last_row(&out);
        assert!((row[0] - 3.0).abs() < 1e-9 && row[2] == 0.0, "{row:?}");
        // 既定（CS を持つので CS）は前進 Euler の CSV と同じ行数で、事象の報告は出ない
        let o = globe(&["fmu", "run", fmu.to_str().unwrap()]);
        assert!(
            !String::from_utf8_lossy(&o.stderr).contains("Model Exchange"),
            "{o:?}"
        );
    }
    // ME の Dahlquist は解析解 exp(-t)
    let Some(fmu) = reference_fmu("3.0", "Dahlquist") else {
        return;
    };
    let o = globe(&["fmu", "run", fmu.to_str().unwrap(), "--kind", "me"]);
    assert!(o.status.success(), "{o:?}");
    let x = last_row(&String::from_utf8_lossy(&o.stdout))[1];
    assert!((x - (-10.0f64).exp()).abs() < 1e-8, "{x}");
}

#[test]
fn run_prints_a_warning_and_the_csv() {
    for version in ["2.0", "3.0"] {
        let Some(fmu) = reference_fmu(version, "Dahlquist") else {
            return;
        };
        let o = globe(&[
            "fmu",
            "run",
            fmu.to_str().unwrap(),
            "--stop-time",
            "1",
            "--set",
            "k=2",
        ]);
        assert!(o.status.success(), "{o:?}");
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("FMU の中のネイティブコードを実行する")
        );
        let out = String::from_utf8_lossy(&o.stdout);
        assert!(out.starts_with("time,x\n"), "{out}");
        let row = last_row(&out);
        assert!((row[0] - 1.0).abs() < 1e-9);
        assert!(
            (row[1] - 0.8f64.powi(10)).abs() < 1e-12,
            "{version}: {row:?}"
        ); // 内部刻み 0.1 の前進 Euler
    }
}

#[test]
fn run_reports_errors_with_exit_code_1() {
    let Some(fmu) = reference_fmu("3.0", "Dahlquist") else {
        return;
    };
    let o = globe(&[
        "fmu",
        "run",
        fmu.to_str().unwrap(),
        "--stop-time",
        "1",
        "--output",
        "nope",
    ]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(String::from_utf8_lossy(&o.stderr).contains("nope"));
    // S206: 出力の格子が細かすぎるなら、実行の前に `--step` を勧めて終了コード 1
    for kind in ["cs", "me"] {
        let o = globe(&[
            "fmu",
            "run",
            fmu.to_str().unwrap(),
            "--kind",
            kind,
            "--stop-time",
            "100",
            "--step",
            "1e-6",
        ]);
        assert_eq!(o.status.code(), Some(1), "{o:?}");
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("--step"),
            "{o:?}"
        );
    }
    let o = globe(&["fmu", "run", "/no/such.fmu"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
}

/// FMPy の `c-code`（`fmi2Functions.h` などのあるディレクトリ）。`uv` か FMPy が無ければ None。
fn fmi_headers() -> Option<String> {
    let code = "import fmpy, os; print(os.path.join(os.path.dirname(fmpy.__file__), 'c-code'))";
    let o = Command::new("uv")
        .args([
            "run",
            "--no-project",
            "--with",
            "fmpy",
            "python",
            "-I",
            "-c",
            code,
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

/// 自分で Export した Decay の FMU を自分で Import して、x(1) = e⁻¹ になる。
#[test]
fn round_trip_export_then_import() {
    let linux64 = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");
    if !linux64 || Command::new("cc").arg("--version").output().is_err() {
        eprintln!("skip: x86_64 の Linux でないか、cc が無い");
        return;
    }
    let Some(headers) = fmi_headers() else {
        eprintln!("skip: uv か FMPy が無い（FMI のヘッダを借りる）");
        return;
    };
    let package =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    for v in ["2", "3"] {
        let out =
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-fmu-import-rt{v}"));
        let o = globe(&[
            "fmu",
            "export",
            package.to_str().unwrap(),
            "-m",
            "GlobeModeValidation.Simulation.Decay",
            "--out",
            out.to_str().unwrap(),
            "--fmi-version",
            v,
            "--build",
            "--fmi-headers",
            &headers,
        ]);
        assert!(o.status.success(), "{o:?}");
        let fmu = out.join(format!("GlobeModeValidation_Simulation_Decay-fmi{v}.fmu"));
        let o = globe(&[
            "fmu",
            "run",
            fmu.to_str().unwrap(),
            "--stop-time",
            "1",
            "--step",
            "0.001",
            "--output",
            "x",
        ]);
        assert!(o.status.success(), "FMI {v}: {o:?}");
        let x1 = last_row(&String::from_utf8_lossy(&o.stdout))[1];
        eprintln!("FMI {v} 往復: x(1) - e⁻¹ = {:e}", x1 - (-1.0f64).exp());
        assert!((x1 - (-1.0f64).exp()).abs() < 1e-6, "FMI {v}: x(1) = {x1}");
        // Model Exchange: 自分の積分器（Dormand–Prince 5(4)、許容誤差 1e-6 が既定）で積分する
        let o = globe(&[
            "fmu",
            "run",
            fmu.to_str().unwrap(),
            "--kind",
            "me",
            "--stop-time",
            "1",
            "--step",
            "0.1",
            "--output",
            "x",
        ]);
        assert!(o.status.success(), "FMI {v} ME: {o:?}");
        let x1 = last_row(&String::from_utf8_lossy(&o.stdout))[1];
        eprintln!(
            "FMI {v} 往復（ME）: x(1) - e⁻¹ = {:e}",
            x1 - (-1.0f64).exp()
        );
        assert!(
            (x1 - (-1.0f64).exp()).abs() < 1e-5,
            "FMI {v} ME: x(1) = {x1}"
        );
    }
}

//! `globe simulate` を実際に起動して確かめる（S012）。モデルは src/Modelica/GlobeModeValidation/Simulation。
//!
//! - `--list-solvers`（モデル無し）、`--list-parameters`（表と JSON）
//! - `-o x.mat`（MAT v4。読み戻して値・パラメータ・上書きを確かめる）と `--format csv`
//! - `--param` の誤り（終了コード 2）と、有限でない値（GM-E10006、終了コード 1）

use std::path::PathBuf;
use std::process::{Command, Output};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn globe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .output()
        .expect("run globe")
}

fn simulate(model: &str, extra: &[&str]) -> Output {
    let dir = package_dir();
    let model = format!("GlobeModeValidation.Simulation.{model}");
    let mut args = vec!["simulate", dir.to_str().unwrap(), "-m", &model];
    args.extend_from_slice(extra);
    globe(&args)
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn out_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-cli-{name}"))
}

#[test]
fn list_solvers_needs_no_model() {
    let o = globe(&["simulate", "--list-solvers"]);
    assert!(o.status.success(), "{o:?}");
    let text = stdout(&o);
    assert!(text.lines().any(|l| l.starts_with("rk4 ")), "{text}");
    let o = globe(&["simulate", "--list-solvers", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let names: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(&names[..3], ["bdf", "dopri45", "auto"]);
    assert_eq!(names.len(), globe_mode_sim::solvers::SOLVERS.len());
}

#[test]
fn list_parameters_shows_only_tunable_ones() {
    let o = simulate("Parameters", &["--list-parameters", "--json"]);
    assert!(o.status.success(), "{o:?}");
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let names: Vec<&str> = v["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"k"), "{names:?}");
    for fixed in ["n", "e", "f", "g"] {
        assert!(!names.contains(&fixed), "{fixed}: {names:?}");
    }
    let o = simulate("Parameters", &["--list-parameters"]);
    assert!(stdout(&o).contains("    k = 1"), "{}", stdout(&o));
}

#[test]
fn mat_output_carries_parameters_and_overrides() {
    let path = out_path("parameters.mat");
    let o = simulate(
        "Parameters",
        &["--param", "k=2", "-o", path.to_str().unwrap()],
    );
    assert!(o.status.success(), "{o:?}");
    assert!(
        stdout(&o).contains("パラメータの上書き k = 2"),
        "{}",
        stdout(&o)
    );
    let mut file = std::fs::File::open(&path).unwrap();
    let m = globe_mode_sim::mat::read_mat4(&mut file).unwrap();
    let k = m.parameters.iter().find(|p| p.name == "k").expect("k");
    assert_eq!(k.value, 2.0);
    let x = m.trajectory.variable("x").expect("x");
    let last = *x.values.last().unwrap();
    assert!((last - (-2.0f64).exp()).abs() < 1e-5, "{last}");
    assert_eq!(m.trajectory.time.first(), Some(&0.0));
    assert_eq!(m.trajectory.time.last(), Some(&1.0));
}

#[test]
fn format_overrides_the_extension_and_step_is_shown() {
    let path = out_path("decay-as-csv.mat");
    let o = simulate(
        "Decay",
        &[
            "--solver",
            "rk4",
            "--step",
            "0.01",
            "--format",
            "csv",
            "-o",
            path.to_str().unwrap(),
        ],
    );
    assert!(o.status.success(), "{o:?}");
    let text = stdout(&o);
    assert!(text.contains("刻み幅 0.01（指定）"), "{text}");
    // experiment annotation の Tolerance は固定刻みでは使わない（GM-I10004）
    assert!(text.contains("GM-I10004"), "{text}");
    let csv = std::fs::read_to_string(&path).unwrap();
    assert!(
        csv.starts_with("\"time\","),
        "{}",
        &csv[..40.min(csv.len())]
    );
}

#[test]
fn bad_params_are_rejected() {
    let o = simulate("Decay", &["--param", "k"]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(String::from_utf8_lossy(&o.stderr).contains("「名前=値」の形でない"));
    let o = simulate("Decay", &["--param", "k=inf", "--json"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let codes: Vec<&str> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"GM-E10006"), "{codes:?}");
}

//! `globe linearize`（S254）。解析的に分かる線形系の行列が JSON と人が読む形で出る。

use std::path::PathBuf;
use std::process::Command;

fn run(extra: &[&str]) -> std::process::Output {
    run_model("LinearSystem", extra)
}

fn run_model(model: &str, extra: &[&str]) -> std::process::Output {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .arg("linearize")
        .arg(dir)
        .args(["-m", &format!("GlobeModeValidation.Simulation.{model}")])
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn json_has_the_state_space_matrices() {
    let out = run(&["--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["u"], serde_json::json!(["u"]));
    assert_eq!(v["y"], serde_json::json!(["y1", "y2"]));
    assert!((v["D"][0][0].as_f64().unwrap() - 3.0).abs() < 1e-6);
    assert_eq!(v["A"].as_array().unwrap().len(), 2);
}

#[test]
fn text_prints_the_four_matrices() {
    let out = run(&[]);
    let text = String::from_utf8_lossy(&out.stdout);
    for m in ["A = df/dx", "B = df/du", "C = dy/dx", "D = dy/du"] {
        assert!(text.contains(m), "{text}");
    }
}

/// S286: A = [-2 1; 1 -2] の固有値は -1 と -3（安定）。
#[test]
fn eigenvalues_of_the_linear_system_are_minus_one_and_minus_three() {
    let out = run(&["--json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut re: Vec<f64> = v["eigenvalues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["re"].as_f64().unwrap())
        .collect();
    re.sort_by(f64::total_cmp);
    assert!(
        (re[0] + 3.0).abs() < 1e-6 && (re[1] + 1.0).abs() < 1e-6,
        "{re:?}"
    );
    assert_eq!(v["stability"], "stable");
    let text = String::from_utf8_lossy(&run(&[]).stdout).into_owned();
    assert!(
        text.contains("固有値（A）") && text.contains("安定（すべて Re < 0）"),
        "{text}"
    );
}

/// S286: 減衰のない振り子（c = 0）の θ = 0 は ±i で境界。減衰のある振り子（c = 0.5）は安定。
#[test]
fn undamped_pendulum_is_marginal_and_damped_is_stable() {
    let out = run_model(
        "Pendulum",
        &["--json", "--param", "c=0", "--param", "th0=0"],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let e = v["eigenvalues"].as_array().unwrap();
    assert_eq!(e.len(), 2);
    for x in e {
        assert!(x["re"].as_f64().unwrap().abs() < 1e-6);
        assert!((x["im"].as_f64().unwrap().abs() - 1.0).abs() < 1e-6, "{x}");
    }
    assert_eq!(v["stability"], "marginal");
    let text = String::from_utf8_lossy(
        &run_model("Pendulum", &["--param", "c=0", "--param", "th0=0"]).stdout,
    )
    .into_owned();
    assert!(text.contains("境界"), "{text}");
    let damped = run_model("Pendulum", &["--json"]);
    let v: serde_json::Value = serde_json::from_slice(&damped.stdout).unwrap();
    assert_eq!(v["stability"], "stable");
}

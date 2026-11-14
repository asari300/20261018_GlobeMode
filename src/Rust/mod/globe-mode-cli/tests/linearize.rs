//! `globe linearize`（S254）。解析的に分かる線形系の行列が JSON と人が読む形で出る。

use std::path::PathBuf;
use std::process::Command;

fn run(extra: &[&str]) -> std::process::Output {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .arg("linearize")
        .arg(dir)
        .args(["-m", "GlobeModeValidation.Simulation.LinearSystem"])
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

//! `globe fmu export`（S094、P5a）を実際に起動して確かめる。モデルは GlobeModeValidation.Simulation.Decay。
//!
//! - ソースの FMU（FMI 2.0、Rumoca の `fmi2` のテンプレート）が書けること
//! - `--build` で共有ライブラリまで作り、FMPy（別プロセス、BSD-2）で ME を回して x(1) ≈ e⁻¹ になること
//!   （`cc` か `uv` が無い、または FMPy を取れない環境では飛ばす）

use std::path::{Path, PathBuf};
use std::process::Command;

const MODEL: &str = "GlobeModeValidation.Simulation.Decay";

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn out_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-fmu-{name}"))
}

fn export(out: &Path, extra: &[&str]) -> std::process::Output {
    let dir = package_dir();
    let mut args = vec!["fmu", "export", dir.to_str().unwrap(), "-m", MODEL, "--out"];
    args.push(out.to_str().unwrap());
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(&args)
        .output()
        .expect("run globe")
}

#[test]
fn exports_a_source_fmu() {
    let out = out_dir("source");
    let o = export(&out, &[]);
    assert!(o.status.success(), "{o:?}");
    let root = out.join("GlobeModeValidation_Simulation_Decay");
    let xml = std::fs::read_to_string(root.join("modelDescription.xml")).unwrap();
    let md = globe_mode_fmi::parse_model_description(&xml).unwrap();
    assert_eq!(md.fmi_version, "2.0");
    assert!(md.model_exchange.is_some());
    assert!(md.variables.iter().any(|v| v.name == "x"), "{xml}");
    assert!(root.join("sources/model.c").is_file());
    assert!(
        out.join("GlobeModeValidation_Simulation_Decay.fmu")
            .is_file()
    );
    // 対象外のモデルを渡す前に失敗を返すこと（モデルが無い）
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args([
            "fmu",
            "export",
            package_dir().to_str().unwrap(),
            "-m",
            "No.Such",
            "--out",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!o.status.success());
}

/// FMPy の入った Python を `uv` で動かす（無ければ None）。
fn fmpy(args: &[&str]) -> Option<std::process::Output> {
    Command::new("uv")
        .args(["run", "--no-project", "--with", "fmpy", "python", "-I"])
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
}

#[test]
fn fmpy_simulates_the_built_fmu() {
    let linux64 = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");
    if !linux64 || Command::new("cc").arg("--version").output().is_err() {
        eprintln!("skip: x86_64 の Linux でないか、cc が無い");
        return;
    }
    let code = "import fmpy, os; print(os.path.join(os.path.dirname(fmpy.__file__), 'c-code'))";
    let Some(o) = fmpy(&["-c", code]) else {
        eprintln!("skip: uv か FMPy が無い");
        return;
    };
    let headers = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    let out = out_dir("build");
    let o = export(&out, &["--build", "--fmi-headers", &headers]);
    assert!(o.status.success(), "{o:?}");
    let fmu = out.join("GlobeModeValidation_Simulation_Decay.fmu");
    let csv = out.join("decay.csv");
    let script =
        "import sys; from fmpy.cli import main; sys.argv = ['fmpy'] + sys.argv[1:]; main()";
    let o = fmpy(&["-c", script, "validate", fmu.to_str().unwrap()]);
    assert!(o.is_some(), "fmpy validate が通らない");
    let o = Command::new("uv")
        .args([
            "run",
            "--no-project",
            "--with",
            "fmpy",
            "python",
            "-I",
            "-c",
            script,
        ])
        .args([
            "simulate",
            fmu.to_str().unwrap(),
            "--interface-type",
            "ModelExchange",
        ])
        .args(["--relative-tolerance", "1e-8", "--stop-time", "1"])
        .args(["--output-variables", "x", "--output-file"])
        .arg(&csv)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    let text = std::fs::read_to_string(&csv).unwrap();
    let last = text.lines().last().unwrap();
    let cols: Vec<f64> = last.split(',').map(|c| c.trim().parse().unwrap()).collect();
    assert!((cols[0] - 1.0).abs() < 1e-9, "{last}");
    let expect = (-1.0f64).exp();
    assert!(
        (cols[1] - expect).abs() < 1e-6,
        "x(1) = {} (e⁻¹ = {expect})",
        cols[1]
    );
}

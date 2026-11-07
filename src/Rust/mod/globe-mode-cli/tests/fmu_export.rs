//! `globe fmu export`（S094、P5a）を実際に起動して確かめる。モデルは GlobeModeValidation.Simulation.Decay。
//!
//! - ソースの FMU（FMI 2.0 / 3.0、Rumoca の `fmi2` / `fmi3` のテンプレート）が書けること
//! - `--build` で共有ライブラリまで作り、FMPy（別プロセス、BSD-2）で validate と simulate を回して
//!   x(1) ≈ e⁻¹ になること（2.0 は ME、3.0 は ME と CS。S098）
//!   （`cc` か `uv` が無い、または FMPy を取れない環境では飛ばす）
//! - `--target x86_64-windows-gnu`（zig cc）で `binaries/win64/*.dll` ができること（S105。zig・uv・FMPy が無ければ飛ばす。
//!   Windows・macOS での実行は fmu-export.yml の fmu-cross で確かめる）

use std::path::{Path, PathBuf};
use std::process::Command;

const MODEL: &str = "GlobeModeValidation.Simulation.Decay";

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

/// 試験ごとの書き出し先。CI で `modelDescription.xml` の無い展開先が残っていた（rust-cache が戻した【推測】）。それが
/// あると、FMU の展開先でないフォルダは消さない守り（S212、#23）が断るので、先に消しておく。
fn out_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-fmu-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
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
    // #48: モデルの experiment(StopTime = 1, Tolerance = 1e-8) が <DefaultExperiment> に出る（FMPy・globe fmu run の既定になる）
    let e = md.default_experiment.as_ref().expect(&xml);
    assert_eq!(e.stop_time.as_deref(), Some("1.0"), "{xml}");
    assert_eq!(e.tolerance.as_deref(), Some("1e-8"), "{xml}");
    assert!(e.start_time.is_none(), "{xml}");
    assert!(md.model_exchange.is_some());
    assert!(md.variables.iter().any(|v| v.name == "x"), "{xml}");
    // S117: P5a の XML（Rumoca のテンプレート）も P5b の書く側を通して読み直すと同じ表になる
    let again = globe_mode_fmi::write_model_description_2(&md).unwrap();
    assert_eq!(
        globe_mode_fmi::parse_model_description(&again).unwrap(),
        md,
        "{again}"
    );
    assert!(root.join("sources/model.c").is_file());
    assert!(
        root.join("documentation/licenses/Rumoca-LICENSE.txt")
            .is_file()
    );
    assert!(
        out.join("GlobeModeValidation_Simulation_Decay-fmi2.fmu")
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

#[test]
fn exports_a_fmi3_source_fmu() {
    let out = out_dir("source3");
    let o = export(&out, &["--fmi-version", "3"]);
    assert!(o.status.success(), "{o:?}");
    let root = out.join("GlobeModeValidation_Simulation_Decay");
    let xml = std::fs::read_to_string(root.join("modelDescription.xml")).unwrap();
    let md = globe_mode_fmi::parse_model_description(&xml).unwrap();
    assert_eq!(md.major(), Some(3));
    // #48: モデルの experiment(StopTime = 1, Tolerance = 1e-8) が <DefaultExperiment> に出る（FMPy・globe fmu run の既定になる）
    let e = md.default_experiment.as_ref().expect(&xml);
    assert_eq!(e.stop_time.as_deref(), Some("1.0"), "{xml}");
    assert_eq!(e.tolerance.as_deref(), Some("1e-8"), "{xml}");
    assert!(e.start_time.is_none(), "{xml}");
    assert!(md.model_exchange.is_some() && md.co_simulation.is_some());
    // S124: 3.0 の XML も 3.0 の書く側を通して読み直すと同じ表になる
    let again = globe_mode_fmi::write_model_description_3(&md).unwrap();
    assert_eq!(
        globe_mode_fmi::parse_model_description(&again).unwrap(),
        md,
        "{again}"
    );
    assert!(root.join("sources/buildDescription.xml").is_file());
    let license =
        std::fs::read_to_string(root.join("documentation/licenses/Rumoca-LICENSE.txt")).unwrap();
    assert!(license.contains("Apache License"), "{license}");
    // S189: 2.0 と 3.0 を同じ出力先に続けて書いても、版つきの名前で上書きし合わない
    let o = export(&out, &[]);
    assert!(o.status.success(), "{o:?}");
    for v in [2, 3] {
        let fmu = out.join(format!("GlobeModeValidation_Simulation_Decay-fmi{v}.fmu"));
        assert!(fmu.is_file(), "{fmu:?}");
    }
    assert!(
        !out.join("GlobeModeValidation_Simulation_Decay.fmu")
            .exists()
    );
}

/// S189: Rumoca の限界（イベントのあるモデル）は、英語の原文の前に日本語の 1 文が付く。
#[test]
fn unsupported_model_says_it_cannot_be_exported() {
    let dir = package_dir();
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "export", dir.to_str().unwrap(), "-m"])
        .arg("GlobeModeValidation.Simulation.BouncingBall")
        .arg("--out")
        .arg(out_dir("unsupported"))
        .output()
        .expect("run globe");
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("このモデルは FMU に書き出せない（理由: "),
        "{err}"
    );
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

const FMPY_MAIN: &str =
    "import sys; from fmpy.cli import main; sys.argv = ['fmpy'] + sys.argv[1:]; main()";

/// `--build` で作った FMU を FMPy で validate し、`interfaces` ごとに 0〜1 s を回して x(1) を返す。
/// cc・uv・FMPy が無ければ None。
fn fmpy_x1(name: &str, fmi_version: &str, interfaces: &[&str]) -> Option<Vec<f64>> {
    let linux64 = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");
    if !linux64 || Command::new("cc").arg("--version").output().is_err() {
        eprintln!("skip: x86_64 の Linux でないか、cc が無い");
        return None;
    }
    let code = "import fmpy, os; print(os.path.join(os.path.dirname(fmpy.__file__), 'c-code'))";
    let Some(o) = fmpy(&["-c", code]) else {
        eprintln!("skip: uv か FMPy が無い");
        return None;
    };
    let headers = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    let out = out_dir(name);
    let o = export(
        &out,
        &[
            "--fmi-version",
            fmi_version,
            "--build",
            "--fmi-headers",
            &headers,
        ],
    );
    assert!(o.status.success(), "{o:?}");
    let fmu = out.join(format!(
        "GlobeModeValidation_Simulation_Decay-fmi{fmi_version}.fmu"
    ));
    let o = fmpy(&["-c", FMPY_MAIN, "validate", fmu.to_str().unwrap()]);
    assert!(o.is_some(), "fmpy validate が通らない");
    let x1 = interfaces.iter().map(|interface| {
        let csv = out.join(format!("decay-{interface}.csv"));
        let o = Command::new("uv")
            .args([
                "run",
                "--no-project",
                "--with",
                "fmpy",
                "python",
                "-I",
                "-c",
            ])
            .arg(FMPY_MAIN)
            .args([
                "simulate",
                fmu.to_str().unwrap(),
                "--interface-type",
                interface,
            ])
            .args(["--relative-tolerance", "1e-8", "--stop-time", "1"])
            .args(["--output-variables", "x", "--output-file"])
            .arg(&csv)
            .output()
            .unwrap();
        assert!(o.status.success(), "{interface}: {o:?}");
        let text = std::fs::read_to_string(&csv).unwrap();
        let last = text.lines().last().unwrap();
        let cols: Vec<f64> = last.split(',').map(|c| c.trim().parse().unwrap()).collect();
        assert!((cols[0] - 1.0).abs() < 1e-9, "{interface}: {last}");
        eprintln!(
            "FMI {fmi_version} {interface}: x(1) - e⁻¹ = {:e}",
            cols[1] - (-1.0f64).exp()
        );
        cols[1]
    });
    Some(x1.collect())
}

fn assert_e_inv(x1: Option<Vec<f64>>) {
    let expect = (-1.0f64).exp();
    for x in x1.into_iter().flatten() {
        assert!((x - expect).abs() < 1e-6, "x(1) = {x} (e⁻¹ = {expect})");
    }
}

#[test]
fn fmpy_simulates_the_built_fmu() {
    assert_e_inv(fmpy_x1("build", "2", &["ModelExchange"]));
}

#[test]
fn fmpy_simulates_the_built_fmi3_fmu() {
    assert_e_inv(fmpy_x1("build3", "3", &["ModelExchange", "CoSimulation"]));
}

#[test]
fn zig_cross_builds_a_win64_dll() {
    if Command::new("zig").arg("version").output().is_err() {
        eprintln!("skip: zig が無い");
        return;
    }
    let code = "import fmpy, os; print(os.path.join(os.path.dirname(fmpy.__file__), 'c-code'))";
    let Some(o) = fmpy(&["-c", code]) else {
        eprintln!("skip: uv か FMPy が無い");
        return;
    };
    let headers = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    for (v, platform) in [("2", "win64"), ("3", "x86_64-windows")] {
        let out = out_dir(&format!("cross{v}"));
        let o = export(
            &out,
            &[
                "--fmi-version",
                v,
                "--build",
                "--fmi-headers",
                &headers,
                "--target",
                "x86_64-windows-gnu",
            ],
        );
        assert!(o.status.success(), "{o:?}");
        // 名前は modelIdentifier（Rumoca_<uuid>）。.pdb・.lib は残さない
        let bin = out.join(format!(
            "GlobeModeValidation_Simulation_Decay/binaries/{platform}"
        ));
        let files: Vec<_> = std::fs::read_dir(&bin)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1, "{files:?}");
        let dll = &files[0];
        assert_eq!(dll.extension().unwrap(), "dll");
        let bytes = std::fs::read(dll).unwrap();
        assert_eq!(&bytes[..2], b"MZ", "{} は PE でない", dll.display());
    }
}

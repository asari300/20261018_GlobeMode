//! `globe fmu export`（S094、P5a）を実際に起動して確かめる。モデルは GlobeModeValidation.Simulation.Decay。
//!
//! - ソースの FMU（FMI 2.0 / 3.0、Rumoca の `fmi2` / `fmi3` のテンプレート）が書けること
//! - `--build` で共有ライブラリまで作り、FMPy（別プロセス、BSD-2）で validate と simulate を回して
//!   x(1) ≈ e⁻¹ になること（2.0 は ME、3.0 は ME と CS。S098）
//!   （`cc` か `uv` が無い、または FMPy を取れない環境では飛ばす）
//! - `--target x86_64-windows-gnu`・`x86_64-macos`・`aarch64-macos`（zig cc）で `binaries/<名前>/*.dll`・`*.dylib` ができること（S234 で macOS を足した）（S105。zig・uv・FMPy が無ければ飛ばす。
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
    // S266: der(x) = -k x は状態 x だけに依る（索引は変数の表の位置 + 1）
    let at = |n: &str| md.variables.iter().position(|v| v.name == n).unwrap() as u32 + 1;
    assert_eq!(
        md.model_structure.dependencies.get(&at("der(x)")),
        Some(&vec![at("x")]),
        "{xml}"
    );
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
    // S266: 3.0 は valueReference で、der(x) = -k x は状態 x とパラメータ k に依る（2.0 はパラメータを並べない）
    let vr = |n: &str| {
        md.variables
            .iter()
            .find(|v| v.name == n)
            .unwrap()
            .value_reference
    };
    assert_eq!(
        md.model_structure.dependencies.get(&vr("der(x)")),
        Some(&vec![vr("k"), vr("x")]),
        "{xml}"
    );
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

/// S224: FMI 2.0 の Enumeration は `<Enumeration declaredType>` と `TypeDefinitions`（SimpleType）で、String のパラメータは
/// start つきで出る。3.0 は Rumoca の fmi3 の C が Int64 を未対応なので、Rumoca のまま（Int32）。
/// Enumeration の無いモデル（Decay）の XML は変わらない（上の 2 本が見る）。
#[test]
fn exports_enumeration_and_string_types() {
    for v in ["2", "3"] {
        let out = out_dir(&format!("enum{v}"));
        let o = Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "export", package_dir().to_str().unwrap(), "-m"])
            .arg("GlobeModeValidation.Simulation.EnumString")
            .arg("--out")
            .arg(&out)
            .args(["--fmi-version", v])
            .output()
            .unwrap();
        assert!(o.status.success(), "{o:?}");
        let xml = std::fs::read_to_string(
            out.join("GlobeModeValidation_Simulation_EnumString/modelDescription.xml"),
        )
        .unwrap();
        let md = globe_mode_fmi::parse_model_description(&xml).unwrap();
        let var = |n: &str| md.variables.iter().find(|x| x.name == n).expect(&xml);
        assert_eq!(var("label").ty, globe_mode_fmi::VarType::String, "{xml}");
        assert_eq!(var("label").start.as_deref(), Some("decay"), "{xml}");
        if v == "3" {
            assert!(
                md.enum_types.is_empty() && !xml.contains("<Enumeration"),
                "{xml}"
            );
            continue;
        }
        assert_eq!(
            md.enum_types
                .iter()
                .map(|t| (t.name.as_str(), t.items.as_slice()))
                .collect::<Vec<_>>(),
            [(
                "Speed",
                &[("slow".to_string(), 1), ("fast".to_string(), 2)][..]
            )],
            "{xml}"
        );
        for n in ["speed", "request", "level"] {
            assert_eq!(
                var(n).ty,
                globe_mode_fmi::VarType::Enumeration,
                "{n}\n{xml}"
            );
            assert_eq!(var(n).declared_type.as_deref(), Some("Speed"));
        }
        assert_eq!(var("speed").start.as_deref(), Some("2"));
        // TypeDefinitions は LogCategories の前（XSD の順）
        assert!(xml.find("<TypeDefinitions>").unwrap() < xml.find("<LogCategories").unwrap());
        // 読み→書き→読みでも同じ表になる
        let again = globe_mode_fmi::write_model_description_2(&md).unwrap();
        assert_eq!(
            globe_mode_fmi::parse_model_description(&again).unwrap(),
            md,
            "{again}"
        );
    }
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
fn zig_cross_builds_win_and_macos_binaries() {
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
    // S234: Windows（PE の MZ）と macOS（64 bit の Mach-O、magic 0xfeedfacf のリトルエンディアン）。macOS は arch ごと
    for (v, target, platform, ext, magic) in [
        ("2", "x86_64-windows-gnu", "win64", "dll", &b"MZ"[..]),
        (
            "3",
            "x86_64-windows-gnu",
            "x86_64-windows",
            "dll",
            &b"MZ"[..],
        ),
        (
            "2",
            "x86_64-macos",
            "darwin64",
            "dylib",
            &[0xcf, 0xfa, 0xed, 0xfe][..],
        ),
        (
            "3",
            "x86_64-macos",
            "x86_64-darwin",
            "dylib",
            &[0xcf, 0xfa, 0xed, 0xfe][..],
        ),
        (
            "3",
            "aarch64-macos",
            "aarch64-darwin",
            "dylib",
            &[0xcf, 0xfa, 0xed, 0xfe][..],
        ),
    ] {
        let out = out_dir(&format!("cross{v}-{target}"));
        let o = export(
            &out,
            &[
                "--fmi-version",
                v,
                "--build",
                "--fmi-headers",
                &headers,
                "--target",
                target,
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
        assert_eq!(dll.extension().unwrap(), ext);
        let bytes = std::fs::read(dll).unwrap();
        assert_eq!(
            &bytes[..magic.len()],
            magic,
            "{} の先頭が違う",
            dll.display()
        );
    }
}

/// S237: MSL の例題を FMU にして FMPy の ME・CS で回す（`data/msl` が無い・cc か uv が無ければ飛ばす）。
/// `Modelica.Mechanics.Translational.Examples.Accelerate` は事象が無く、Rumoca のテンプレート（P5a）が受ける。
/// `mass.s(1) = 1`（`globe simulate` も 1.0000000098）。事象のある例題は Rumoca が断る（`unsupported_msl_example_says_why`）。
#[test]
fn fmpy_simulates_an_msl_example() {
    let msl = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    let linux64 = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");
    if !msl.is_dir() || !linux64 || Command::new("cc").arg("--version").output().is_err() {
        eprintln!("skip: data/msl か x86_64 の Linux の cc が無い");
        return;
    }
    let code = "import fmpy, os; print(os.path.join(os.path.dirname(fmpy.__file__), 'c-code'))";
    let Some(o) = fmpy(&["-c", code]) else {
        eprintln!("skip: uv か FMPy が無い");
        return;
    };
    let headers = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    let model = "Modelica.Mechanics.Translational.Examples.Accelerate";
    let out = out_dir("msl");
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "export", "-L"])
        .arg(&msl)
        .args(["-m", model, "--out"])
        .arg(&out)
        .args(["--build", "--fmi-headers", &headers])
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    let fmu = out.join("Modelica_Mechanics_Translational_Examples_Accelerate-fmi2.fmu");
    assert!(
        fmpy(&["-c", FMPY_MAIN, "validate", fmu.to_str().unwrap()]).is_some(),
        "fmpy validate が通らない"
    );
    for interface in ["ModelExchange", "CoSimulation"] {
        let csv = out.join(format!("accelerate-{interface}.csv"));
        let o = fmpy(&[
            "-c",
            FMPY_MAIN,
            "simulate",
            fmu.to_str().unwrap(),
            "--interface-type",
            interface,
            "--stop-time",
            "1",
            "--output-variables",
            "mass.s",
            "--output-file",
            csv.to_str().unwrap(),
        ]);
        assert!(o.is_some(), "{interface}: fmpy simulate が通らない");
        let text = std::fs::read_to_string(&csv).unwrap();
        let last = text.lines().last().unwrap();
        let s: f64 = last.split(',').nth(1).unwrap().trim().parse().unwrap();
        assert!((s - 1.0).abs() < 1e-3, "{interface}: mass.s(1) = {s}");
    }
}

/// S237: 事象のある MSL の例題（`Modelica.Mechanics.Rotational.Examples.First`、ステップ入力）は P5a では断られる。
/// 理由は Rumoca の原文（FMI C profile）が付く。
#[test]
fn unsupported_msl_example_says_why() {
    let msl = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    if !msl.is_dir() {
        eprintln!("skip: data/msl が無い");
        return;
    }
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "export", "-L"])
        .arg(&msl)
        .args([
            "-m",
            "Modelica.Mechanics.Rotational.Examples.First",
            "--out",
        ])
        .arg(out_dir("msl-first"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("FMI C profile"), "{err}");
}

/// S238: `--runtime p5b` に渡す `globe-mode-fmu-cdylib` の共有ライブラリ（debug。`cargo build` で作る。作れなければ None）。
fn p5b_runtime() -> Option<PathBuf> {
    let cargo = std::env::var_os("CARGO").unwrap_or("cargo".into());
    let ok = Command::new(cargo)
        .args(["build", "-q", "-p", "globe-mode-fmu-cdylib", "--lib"])
        .status()
        .ok()?
        .success();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_globe"));
    let name = format!(
        "{}globe_mode_fmu_cdylib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let lib = exe.parent()?.join(name);
    (ok && lib.is_file()).then_some(lib)
}

/// P5b の FMU を `globe fmu run`（CS）で 1 秒回し、`column` の最後の値を返す。
fn run_p5b_fmu(fmu: &Path, column: &str) -> f64 {
    let csv = fmu.with_extension("csv");
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "run"])
        .arg(fmu)
        .args(["--stop-time", "1", "--output", column, "--out"])
        .arg(&csv)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    let text = std::fs::read_to_string(&csv).unwrap();
    text.lines()
        .last()
        .unwrap()
        .split(',')
        .nth(1)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// S238: `--runtime p5b` が Decay の P5b の FMU（共有ライブラリ・modelDescription.xml・resources）を作り、`globe fmu run` で
/// x(1) ≈ e⁻¹ になる。p5a の旗（--build）との組み合わせは断る（終了コード 2）。共有ライブラリを作れなければ飛ばす。
#[test]
fn p5b_runtime_exports_a_runnable_fmu() {
    let Some(lib) = p5b_runtime() else {
        eprintln!("skip: cargo build -p globe-mode-fmu-cdylib ができない");
        return;
    };
    let mo = package_dir().join("Simulation/Decay.mo");
    let out = out_dir("p5b");
    let export = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "export", mo.to_str().unwrap(), "-m", MODEL, "--out"])
            .arg(&out)
            .args(["--runtime", "p5b"])
            .args(extra)
            .output()
            .unwrap()
    };
    let o = export(&["--runtime-lib", lib.to_str().unwrap()]);
    assert!(o.status.success(), "{o:?}");
    let fmu = out.join("GlobeModeValidation_Simulation_Decay-p5b-fmi2.fmu");
    let root = out.join("GlobeModeValidation_Simulation_Decay-p5b");
    assert!(root.join("modelDescription.xml").is_file());
    assert!(root.join(format!("resources/{MODEL}.mo")).is_file());
    assert!(
        !root.join("resources/lib").exists(),
        "ライブラリを使わないモデルには resources/lib が要らない"
    );
    let x = run_p5b_fmu(&fmu, "x");
    assert!((x - (-1f64).exp()).abs() < 1e-3, "x(1) = {x}");
    // p5a のもの（--build）とは組み合わせない。共有ライブラリが無い・FMU でないフォルダを消さない守りも p5a と同じ
    let o = export(&["--build", "--fmi-headers", "."]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    let o = export(&["--runtime-lib", "/no/such/lib"]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
}

/// S251: `--kind se` は FMI 3.0 の p5b だけ。書けた FMU の modelDescription.xml に `ScheduledExecution` と周期の `Clock` が出る。
/// （SE を実際に回すのは globe-mode-fmu の `fmpy_scheduled_execution`。fmusim は SE を回せない）
#[test]
fn kind_se_writes_a_scheduled_execution_fmu() {
    let Some(lib) = p5b_runtime() else {
        eprintln!("skip: cargo build -p globe-mode-fmu-cdylib ができない");
        return;
    };
    let mo = package_dir().join("Simulation/Decay.mo");
    let out = out_dir("p5b-se");
    let export = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "export", mo.to_str().unwrap(), "-m", MODEL, "--out"])
            .arg(&out)
            .args(["--runtime", "p5b", "--runtime-lib", lib.to_str().unwrap()])
            .args(["--kind", "se"])
            .args(extra)
            .output()
            .unwrap()
    };
    let o = export(&["--fmi-version", "3"]);
    assert!(o.status.success(), "{o:?}");
    let root = out.join("GlobeModeValidation_Simulation_Decay-p5b-se");
    let xml = std::fs::read_to_string(root.join("modelDescription.xml")).unwrap();
    assert!(
        xml.contains("<ScheduledExecution ") && xml.contains("<Clock "),
        "{xml}"
    );
    assert!(
        out.join("GlobeModeValidation_Simulation_Decay-p5b-se-fmi3.fmu")
            .is_file()
    );
    // FMI 2.0 には SE が無い
    let o = export(&[]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
}

/// S238・S243: MSL のモデルの P5b の FMU は、使うクラスのファイルだけ（S243 から使うクラスだけ）を `resources/lib` に入れる（MSL は 2556 ファイル）。
/// 入れたものだけで instantiate でき、`mass.s(1) = 1`。`data/msl` か共有ライブラリが無ければ飛ばす。
#[test]
fn p5b_runtime_ships_only_the_used_library_files() {
    let msl = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    let Some(lib) = msl.is_dir().then(p5b_runtime).flatten() else {
        eprintln!("skip: data/msl か globe-mode-fmu の共有ライブラリが無い");
        return;
    };
    let model = "Modelica.Mechanics.Translational.Examples.Accelerate";
    let out = out_dir("p5b-msl");
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "export", "-L"])
        .arg(&msl)
        .args(["-m", model, "--out"])
        .arg(&out)
        .args(["--runtime", "p5b", "--runtime-lib", lib.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    let mut files = Vec::new();
    globe_mode::collect_mo_files(
        &out.join("Modelica_Mechanics_Translational_Examples_Accelerate-p5b/resources/lib"),
        &mut files,
    )
    .unwrap();
    assert!(
        (50..150).contains(&files.len()),
        "resources/lib の .mo は {} 個",
        files.len()
    );
    // S243: 使わない入れ子のクラスも落とす（ファイルだけの閉包は 2.4 MB、クラス単位で約 0.4 MB）
    let bytes: u64 = files
        .iter()
        .map(|f| std::fs::metadata(f).unwrap().len())
        .sum();
    assert!(bytes < 1_000_000, "resources/lib は {bytes} バイト");
    let fmu = out.join("Modelica_Mechanics_Translational_Examples_Accelerate-p5b-fmi2.fmu");
    let s = run_p5b_fmu(&fmu, "mass.s");
    assert!((s - 1.0).abs() < 1e-3, "mass.s(1) = {s}");
}

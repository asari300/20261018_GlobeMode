//! S095・S099: FMU を関数で直接呼ぶ検査と、FMPy で回す検査（FMPy が回せない環境では skip）。
//!
//! モデルは `src/Modelica/GlobeModeValidation/Simulation/` の 1 ファイルを `resources/<完全修飾名>.mo` に置く。

use std::collections::BTreeMap;
use std::ffi::{CString, c_int};
use std::path::PathBuf;
use std::process::Command;

use globe_mode_fmu::*;

fn source(name: &str) -> (String, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../Modelica/GlobeModeValidation/Simulation/{name}.mo"
    ));
    (
        format!("GlobeModeValidation.Simulation.{name}"),
        std::fs::read_to_string(path).unwrap(),
    )
}

/// `resources/<完全修飾名>.mo` を置いた一時の置き場（FMU を展開したときの `resources` に当たる）。
/// 試験は並行に走るので、置き場は試験ごとの `tag` で分ける。debug の cdylib は 700 MB あり、tmpfs の /tmp に
/// 置くと溢れるので、target の下（`CARGO_TARGET_TMPDIR`）に置く。
fn resources(tag: &str, model: &str, text: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "globe_mode_fmu_{}_{tag}/resources",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{model}.mo")), text).unwrap();
    dir
}

#[test]
fn model_description_lists_states_derivatives_and_parameters() {
    let (model, text) = source("HarmonicOscillator");
    let md = model_description(&model, &text).unwrap();
    assert!(md.contains(&format!("guid=\"{}\"", guid(&model, &text))));
    for v in [
        r#"name="x" valueReference="0""#,
        r#"name="v" valueReference="1""#,
        r#"name="der(x)" valueReference="2""#,
        r#"name="der(v)" valueReference="3""#,
        r#"name="w" valueReference="4""#,
        r#"<Real start="2.0"/>"#,
        r#"stopTime="1.0" tolerance="1e-8""#,
    ] {
        assert!(md.contains(v), "{v} が無い:\n{md}");
    }
}

#[test]
fn explicit_euler_through_the_c_abi() {
    let (model, text) = source("Decay");
    let dir = resources("c_abi", &model, &text);
    let location = CString::new(format!("file://{}", dir.display())).unwrap();
    let guid = CString::new(guid(&model, &text)).unwrap();
    let instantiate = |kind: c_int, g: &CString| unsafe {
        fmi2Instantiate(
            c"d".as_ptr(),
            kind,
            g.as_ptr(),
            location.as_ptr(),
            std::ptr::null(),
            0,
            0,
        )
    };
    unsafe {
        assert!(instantiate(1, &guid).is_null(), "CS は断る");
        assert!(
            instantiate(0, &CString::new("{x}").unwrap()).is_null(),
            "guid 違いは断る"
        );
        let c = instantiate(0, &guid);
        assert!(!c.is_null());
        assert_eq!(fmi2EnterInitializationMode(c), 0);
        assert_eq!(fmi2SetReal(c, [2].as_ptr(), 1, [2.0].as_ptr()), 0);
        assert_eq!(fmi2ExitInitializationMode(c), 0);
        let (mut x, mut dx) = ([0.0], [0.0]);
        let n = 100_000;
        for _ in 0..n {
            assert_eq!(fmi2GetContinuousStates(c, x.as_mut_ptr(), 1), 0);
            assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
            x[0] += dx[0] / f64::from(n);
            assert_eq!(fmi2SetContinuousStates(c, x.as_ptr(), 1), 0);
        }
        assert!((x[0] - (-2.0f64).exp()).abs() < 1e-4, "x(1) = {}", x[0]);
        let mut v = [0.0; 3];
        assert_eq!(fmi2GetReal(c, [0, 1, 2].as_ptr(), 3, v.as_mut_ptr()), 0);
        assert_eq!(v, [x[0], -2.0 * x[0], 2.0]);
        assert_eq!(
            fmi2GetReal(c, [9].as_ptr(), 1, v.as_mut_ptr()),
            3,
            "無い値参照"
        );
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 2), 3, "状態の数違い");
        let (mut e, mut t): (c_int, c_int) = (1, 1);
        assert_eq!(fmi2CompletedIntegratorStep(c, 1, &mut e, &mut t), 0);
        assert_eq!((e, t), (0, 0));
        assert_eq!(fmi2Reset(c), 0);
        assert_eq!(fmi2GetReal(c, [0, 2].as_ptr(), 2, v.as_mut_ptr()), 0);
        assert_eq!(&v[..2], [1.0, 1.0], "Reset で既定に戻る");
        fmi2FreeInstance(c);
    }
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

/// cdylib を作り、`model` を FMU に詰めて FMPy の ME で回す。終了時刻での `names` の値を返す。FMPy が使えなければ None。
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn fmpy(name: &str, names: &[&str]) -> Option<Vec<f64>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let uv = |args: &[&std::ffi::OsStr]| {
        Command::new("uv")
            // FMPy が FMU を展開する置き場も target の下にする
            .env("TMPDIR", &tmp)
            .args(["run", "--no-project", "--with", "fmpy", "python", "-I"])
            .args(args)
            .output()
    };
    match uv(&["-c".as_ref(), "import fmpy".as_ref()]) {
        Ok(o) if o.status.success() => {}
        other => {
            eprintln!("skip: FMPy を使えない: {other:?}");
            return None;
        }
    }
    // 結合テストは rlib だけを作るので、cdylib を cargo で作らせて置き場を JSON の出力から取る
    let build = Command::new(env!("CARGO"))
        .args(["build", "--lib", "--message-format=json", "--manifest-path"])
        .arg(manifest.join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let json = String::from_utf8_lossy(&build.stdout);
    let end = json
        .find("libglobe_mode_fmu.so\"")
        .expect("cdylib の置き場が出ない")
        + "libglobe_mode_fmu.so".len();
    let so = PathBuf::from(&json[json[..end].rfind('"').unwrap() + 1..end]);
    let (model, text) = source(name);
    let dir = resources(name, &model, &text);
    let work = dir.parent().unwrap();
    let md = work.join("modelDescription.xml");
    std::fs::write(&md, model_description(&model, &text).unwrap()).unwrap();
    let fmu = work.join("model.fmu");
    let out = uv(&[
        manifest.join("tests/fmpy_run.py").as_os_str(),
        so.as_os_str(),
        md.as_os_str(),
        dir.join(format!("{model}.mo")).as_os_str(),
        model.as_ref(),
        fmu.as_os_str(),
        names.join(",").as_ref(),
    ])
    .unwrap();
    let _ = std::fs::remove_dir_all(work);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "FMPy が失敗: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!("FMPy {name}: {}", stdout.trim());
    Some(names.iter().map(|n| value(&stdout, n)).collect())
}

fn value(stdout: &str, name: &str) -> f64 {
    stdout
        .split(&format!("{name}="))
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn fmpy_simulates_decay() {
    let Some(x) = fmpy("Decay", &["x"]) else {
        return;
    };
    assert!((x[0] - (-1.0f64).exp()).abs() < 1e-4, "x(1) = {}", x[0]);
}

/// 状態 2 つ・事象なしの HarmonicOscillator を、FMPy と `globe_mode_exec::run`（bdf の既定）で回して x(1)・v(1) を突き合わせる。
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn fmpy_matches_exec_on_harmonic_oscillator() {
    let Some(fmu) = fmpy("HarmonicOscillator", &["x", "v"]) else {
        return;
    };
    let (model, text) = source("HarmonicOscillator");
    let report = globe_mode_exec::run(&compile(&model, &text).unwrap(), &Default::default());
    let run = report.run.expect("exec で回る");
    let exec: Vec<f64> = ["x", "v"]
        .iter()
        .map(|n| {
            let var = run
                .trajectory
                .variables
                .iter()
                .find(|v| v.name == *n)
                .unwrap();
            *var.values.last().unwrap()
        })
        .collect();
    eprintln!("exec HarmonicOscillator: x={} v={}", exec[0], exec[1]);
    for (f, e) in fmu.iter().zip(&exec) {
        assert!((f - e).abs() < 1e-4, "FMPy {fmu:?} と exec {exec:?}");
    }
}

#[test]
fn events_are_refused() {
    let (model, text) = source("BouncingBall");
    let e = globe_mode_exec::ModelInstance::new(
        &compile(&model, &text).unwrap(),
        &BTreeMap::new(),
        0.0,
    )
    .unwrap_err();
    assert_eq!(e[0].code, "GM-U0004", "{e:?}");
}

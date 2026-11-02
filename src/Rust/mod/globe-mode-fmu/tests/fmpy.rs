//! S095: Decay の FMU を関数で直接呼ぶ検査と、FMPy で回す検査（FMPy が回せない環境では skip）。

use std::ffi::{CString, c_int};
use std::path::PathBuf;
use std::process::Command;

use globe_mode_fmu::*;

#[test]
fn guid_matches_model_description() {
    let md = include_str!("../modelDescription.xml");
    assert!(md.contains(&format!("guid=\"{GUID}\"")));
}

#[test]
fn explicit_euler_through_the_c_abi() {
    let guid = CString::new(GUID).unwrap();
    unsafe {
        assert!(
            fmi2Instantiate(
                c"d".as_ptr(),
                1,
                guid.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0
            )
            .is_null(),
            "CS は断る"
        );
        assert!(
            fmi2Instantiate(
                c"d".as_ptr(),
                0,
                c"{x}".as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0
            )
            .is_null(),
            "guid 違いは断る"
        );
        let c = fmi2Instantiate(
            c"d".as_ptr(),
            0,
            guid.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
        );
        assert!(!c.is_null());
        assert_eq!(fmi2SetReal(c, [2].as_ptr(), 1, [2.0].as_ptr()), 0);
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
        let (mut e, mut t): (c_int, c_int) = (1, 1);
        assert_eq!(fmi2CompletedIntegratorStep(c, 1, &mut e, &mut t), 0);
        assert_eq!((e, t), (0, 0));
        fmi2FreeInstance(c);
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn fmpy_simulates_decay() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fmu = std::env::temp_dir().join(format!("globe_mode_fmu_{}.fmu", std::process::id()));
    let uv = |args: &[&std::ffi::OsStr]| {
        Command::new("uv")
            .args(["run", "--no-project", "--with", "fmpy", "python", "-I"])
            .args(args)
            .output()
    };
    // FMPy（と uv）が使えない環境では skip
    match uv(&["-c".as_ref(), "import fmpy".as_ref()]) {
        Ok(o) if o.status.success() => {}
        other => {
            eprintln!("skip: FMPy を使えない: {other:?}");
            return;
        }
    }
    // 結合テストは rlib だけを作るので、cdylib を cargo で作らせて置き場を JSON の出力から取る
    let build = Command::new(env!("CARGO"))
        .args(["build", "--lib", "--message-format=json", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
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
    let out = uv(&[
        dir.join("tests/fmpy_decay.py").as_os_str(),
        so.as_os_str(),
        dir.join("modelDescription.xml").as_os_str(),
        fmu.as_os_str(),
    ])
    .unwrap();
    let _ = std::fs::remove_file(&fmu);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "FMPy が失敗: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let x: f64 = stdout
        .split("x(1)=")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap()
        .parse()
        .unwrap();
    assert!((x - (-1.0f64).exp()).abs() < 1e-4, "x(1) = {x}（{stdout}）");
    eprintln!("FMPy: {}", stdout.trim());
}

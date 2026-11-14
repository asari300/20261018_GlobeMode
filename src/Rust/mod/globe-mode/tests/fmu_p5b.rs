//! S239: `Workspace::export_fmu` の `runtime = p5b`（facade の `export_fmu_p5b`）。共有ライブラリは `runtime_lib` で渡すので cargo は呼ばない
//! （実際に FMPy 相当で回す試験は globe-mode-cli の `p5b_runtime_exports_a_runnable_fmu`）。
#![cfg(feature = "fmu-p5b")]

use std::path::PathBuf;

use globe_mode::{FmuExportKind, FmuOptions, FmuRuntime, Workspace};

const DECAY: &str =
    "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";

fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("globe-facade-p5b-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn p5b(lib: &std::path::Path) -> FmuOptions {
    FmuOptions {
        runtime: FmuRuntime::P5b,
        runtime_lib: Some(lib.to_path_buf()),
        ..Default::default()
    }
}

#[test]
fn workspace_exports_a_p5b_fmu_with_the_runtime_copied_and_stripped() {
    let out = out_dir("decay");
    let mut ws = Workspace::new();
    ws.set_file("Decay.mo", DECAY);
    // 中身は何でもよいが、strip の効きを見るため本物の実行ファイル（試験自身。シンボルが多い）を渡す
    let lib = std::env::current_exe().unwrap();
    let fmu = ws.export_fmu("Decay", &out, &p5b(&lib)).unwrap();
    assert_eq!(fmu, out.join("Decay-p5b-fmi2.fmu"));
    let root = out.join("Decay-p5b");
    assert!(root.join("modelDescription.xml").is_file());
    assert_eq!(
        std::fs::read_to_string(root.join("resources/Decay.mo")).unwrap(),
        DECAY
    );
    let bin = std::fs::read_dir(root.join("binaries"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let copied = std::fs::read_dir(&bin)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let (before, after) = (
        std::fs::metadata(&lib).unwrap().len(),
        std::fs::metadata(&copied).unwrap().len(),
    );
    if cfg!(target_os = "linux")
        && std::process::Command::new("strip")
            .arg("--version")
            .output()
            .is_ok()
    {
        assert!(after < before, "strip していない: {before} → {after}");
    }
    assert!(lib.is_file(), "渡された runtime_lib は触らない");

    // p5a の旗とは組み合わせない。無い共有ライブラリは断る
    let mut bad = p5b(&lib);
    bad.build = true;
    assert!(
        ws.export_fmu("Decay", &out, &bad)
            .unwrap_err()
            .contains("p5a のもの")
    );
    let missing = out.join("none");
    assert!(
        ws.export_fmu("Decay", &out, &p5b(&missing))
            .unwrap_err()
            .contains("runtime_lib")
    );
    // p5a の自由関数は p5b を受けない
    let compiled = ws.compile("Decay", &Default::default()).unwrap();
    assert!(globe_mode::export_fmu(&compiled, &out, &p5b(&lib)).is_err());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn options_json_reads_runtime() {
    let o = FmuOptions::from_json(Some(r#"{"runtime": "p5b", "runtime_lib": "x.so"}"#)).unwrap();
    assert_eq!(
        (o.runtime, o.runtime_lib),
        (FmuRuntime::P5b, Some("x.so".into()))
    );
    assert!(FmuOptions::from_json(Some(r#"{"runtime": "p6"}"#)).is_err());
}

/// S251: `kind = se` は FMI 3.0 の p5b だけ。書けた FMU の modelDescription.xml は ScheduledExecution と周期の Clock を持つ。
#[test]
fn se_kind_writes_a_scheduled_execution_fmu_for_fmi3_p5b_only() {
    assert_eq!(
        FmuOptions::from_json(Some(r#"{"kind": "se"}"#))
            .unwrap()
            .kind,
        FmuExportKind::Se
    );
    let out = out_dir("se");
    let mut ws = Workspace::new();
    ws.set_file("Decay.mo", DECAY);
    let lib = std::env::current_exe().unwrap();
    let se = |fmi_version| FmuOptions {
        fmi_version,
        kind: FmuExportKind::Se,
        ..p5b(&lib)
    };
    let err = ws.export_fmu("Decay", &out, &se(2)).unwrap_err();
    assert!(err.contains("FMI 3.0"), "{err}");
    let fmu = ws.export_fmu("Decay", &out, &se(3)).unwrap();
    assert_eq!(fmu, out.join("Decay-p5b-se-fmi3.fmu"));
    let xml = std::fs::read_to_string(out.join("Decay-p5b-se/modelDescription.xml")).unwrap();
    assert!(
        xml.contains("<ScheduledExecution ") && xml.contains("<Clock "),
        "{xml}"
    );
    // p5a は SE を出せない
    let compiled = ws.compile("Decay", &Default::default()).unwrap();
    let p5a = FmuOptions {
        kind: FmuExportKind::Se,
        ..Default::default()
    };
    assert!(globe_mode::export_fmu(&compiled, &out, &p5a).is_err());
    std::fs::remove_dir_all(&out).unwrap();
}

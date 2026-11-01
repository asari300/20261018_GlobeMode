//! `Workspace::with_libraries`（閉包だけ読む。S054）と `with_libraries_full`（全部読む）で、`check` の JSON が一致する。

use std::path::{Path, PathBuf};

use globe_mode::Workspace;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// `dir` の下のモデル（`package.mo` 以外）の完全修飾名。`base` は MODELICAPATH の根。
fn models(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            models(base, &p, out);
        } else if p.extension().is_some_and(|e| e == "mo") && p.file_name().unwrap() != "package.mo"
        {
            let rel = p.strip_prefix(base).unwrap().with_extension("");
            out.push(
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("."),
            );
        }
    }
}

/// 同じ順に検査して JSON を比べる（分割は 2 つ目以降のモデルで閉包を読み足す）。
fn same(libs: &[PathBuf], models: &[String]) {
    let mut split = Workspace::with_libraries(libs).expect("閉包の読み込み");
    let mut full = Workspace::with_libraries_full(libs).expect("全部の読み込み");
    for m in models {
        let a = serde_json::to_string(&split.check(m)).unwrap();
        let b = serde_json::to_string(&full.check(m)).unwrap();
        assert_eq!(a, b, "{m}");
    }
}

#[test]
fn validation_package_matches() {
    let base = root().join("src/Modelica");
    let mut ms = Vec::new();
    models(&base, &base.join("GlobeModeValidation"), &mut ms);
    assert!(ms.len() > 20, "{ms:?}");
    same(&[base], &ms);
}

#[test]
fn msl_examples_match() {
    let msl = root().join("data/msl");
    if !msl.is_dir() {
        eprintln!("{} が無いので飛ばす", msl.display());
        return;
    }
    let ms: Vec<String> = [
        "Modelica.Electrical.Analog.Examples.ChuaCircuit",
        "Modelica.Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot",
        "Modelica.Fluid.Examples.HeatingSystem",
    ]
    .map(String::from)
    .into();
    // MSL の検査は深く再帰するので、テストの既定のスタック（2 MiB）では足りない
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || same(&[msl], &ms))
        .unwrap()
        .join()
        .unwrap();
}

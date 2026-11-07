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

/// 構文の誤りのファイル（`a/A/Broken.mo`）があっても、in-memory の読み込み（分割と全部）が断らず、
/// 読み飛ばしを `GM-W1001` で知らせる（S071。ディスクの `Checker::new` と同じ寛容さ。S066）。
#[test]
fn broken_library_file_is_skipped_in_memory() {
    let a = root().join("src/Rust/mod/globe-mode-frontend/tests/data/tolerant/a");
    let mut files = std::collections::BTreeMap::new();
    for f in std::fs::read_dir(a.join("A")).unwrap() {
        let p = f.unwrap().path();
        let name = p
            .strip_prefix(&a)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(name, std::fs::read_to_string(&p).unwrap());
    }
    let mut split = Workspace::with_library_sources_split("a", &files).expect("分割で読める");
    let mut full = Workspace::with_library_sources("a", &files).expect("全部で読める");
    let mut dirs = Workspace::with_libraries(&[a]).expect("ディレクトリの分割で読める");
    for ws in [&mut split, &mut full, &mut dirs] {
        let r = ws.check("A.Decay");
        assert!(r.passed, "{:?}", r.diagnostics);
        // 分割は Broken.mo を読んだ（A.Broken を引いた）ときから警告を付ける
        let r = ws.check("A.Broken");
        assert!(!r.passed);
        let warned: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == "GM-W1001")
            .collect();
        assert_eq!(warned.len(), 1, "{:?}", r.diagnostics);
        assert!(warned[0].notes.iter().any(|n| n.contains("Broken.mo")));
    }
}

/// #31: `X.mo` と `package.mo` の無い `X/` が並ぶ配置（AixLib の `RadiatorTypes`）は `X.mo` だけを読み（S068）、
/// ドットのフォルダは飛ばす。ディレクトリの分割読み込みも全部読みと同じ結果になる。
#[test]
fn sibling_file_wins_in_the_directory_split() {
    let dir = std::env::temp_dir().join(format!("gm-s212-split-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in [
        ("P/package.mo", "package P end P;"),
        ("P/X.mo", "within P; package X type T = Real; end X;"),
        ("P/X/T.mo", "within P.X; type T = Integer;"),
        ("P/.hidden/Y.mo", "within P; package Y end Y;"),
        ("M.mo", "model M parameter P.X.T t = 1; end M;"),
    ] {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let mut split = Workspace::with_libraries(std::slice::from_ref(&dir)).unwrap();
    let r = split.check("M");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(r.passed, "{:?}", r.diagnostics);
}

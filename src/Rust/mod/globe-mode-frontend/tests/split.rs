//! ライブラリの分割読み込み（S037）が、全部を読んだときと同じ検査結果を返すこと。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode_core::codes;
use globe_mode_frontend::{Checker, Source};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// `dir` の下の `.mo` を（`root` からの相対の名前 → 本文で）読む。
fn read_mo(dir: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                walk(&p, root, out);
            } else if p.extension().is_some_and(|e| e == "mo") {
                let name = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(name, std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let root = root();
    let mut out = BTreeMap::new();
    walk(&root.join(dir), &root, &mut out);
    out
}

fn sources(files: &BTreeMap<String, String>) -> Vec<Source<'_>> {
    files
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect()
}

#[test]
fn split_library_checks_like_the_full_load() {
    let files = read_mo(Path::new("src/Modelica/GlobeModeValidation"));
    let full = Checker::with_library_sources("V", &sources(&files)).unwrap();
    let split = Checker::with_library_sources_split("V", &sources(&files)).unwrap();
    let models = [
        "GlobeModeValidation.Simulation.Decay",
        "GlobeModeValidation.Simulation.HarmonicOscillator",
        "GlobeModeValidation.Connect.Hierarchical",
        "GlobeModeValidation.Invalid.StructurallySingular",
        "GlobeModeValidation.Nothing.Here",
    ];
    for model in models {
        let a = full.check(&[], model);
        let b = split.check(&[], model);
        assert_eq!(a, b, "{model}");
    }
    // 作業中の文書がライブラリのモデルを extends する。後から別のファイルが要るモデルを足しても、読んだものは外さない
    let mine =
        "model Faster\n  extends GlobeModeValidation.Simulation.Decay(k = 2);\nend Faster;\n";
    let docs = [Source {
        name: "Faster.mo",
        text: mine,
    }];
    let a = full.check_retaining(&docs, "Faster");
    let b = split.check_retaining(&docs, "Faster");
    assert!(a.passed(), "{:?}", a.diagnostics);
    assert_eq!(a, b);
    assert_eq!(
        full.check_retaining(&[], "GlobeModeValidation.Simulation.Decay"),
        split.check_retaining(&[], "GlobeModeValidation.Simulation.Decay"),
    );
    assert_eq!(split.document_names(), ["Faster.mo"]);
}

#[test]
fn split_library_reports_what_it_cannot_read() {
    let mut files = BTreeMap::new();
    // クラスの見つからないファイルは、読み込むときに構文を確かめる
    files.insert("Broken.mo".to_string(), "x = (\n".to_string());
    let err = Checker::with_library_sources_split("broken", &sources(&files)).unwrap_err();
    assert_eq!(err[0].code, codes::LIBRARY, "{err:?}");
    let err = Checker::with_library_sources_split("empty", &[]).unwrap_err();
    assert_eq!(err[0].code, codes::LIBRARY, "{err:?}");
    // 構文の誤りのあるファイルは、そのファイルを使うモデルの検査で GM-E1003 になる
    files.insert(
        "Bad.mo".to_string(),
        "model Bad\n  Real x\nend Bad;\n".to_string(),
    );
    files.insert(
        "Good.mo".to_string(),
        "model Good\n  Real x;\nequation\n  der(x) = -x;\nend Good;\n".to_string(),
    );
    files.remove("Broken.mo");
    let checker = Checker::with_library_sources_split("mixed", &sources(&files)).unwrap();
    assert!(checker.check(&[], "Good").passed());
    let report = checker.check(&[], "Bad");
    assert_eq!(
        report.diagnostics[0].code,
        codes::LIBRARY,
        "{:?}",
        report.diagnostics
    );
}

/// MSL の突き合わせ用の 20 本（`src/Modelica/msl-compare.txt`）が、全部を読んだ検査器と同じ結果になること。
/// MSL（`cargo xtask fetch-msl`）が要る。重い（release で 4 GB 近く）ので既定では回さない:
/// `cargo test --release -p globe-mode-frontend --test split -- --ignored --nocapture`
#[test]
#[ignore = "MSL が要る（data/msl）。重い"]
fn msl_examples_match_the_full_load() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let msl = root().join("data/msl");
            assert!(msl.is_dir(), "data/msl が無い（cargo xtask fetch-msl）");
            let models: Vec<String> =
                std::fs::read_to_string(root().join("src/Modelica/msl-compare.txt"))
                    .unwrap()
                    .lines()
                    .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                    .map(str::to_string)
                    .collect();
            let files = read_mo(Path::new("data/msl"));
            let full = Checker::with_library_sources("MSL", &sources(&files)).unwrap();
            let split = Checker::with_library_sources_split("MSL", &sources(&files)).unwrap();
            for model in &models {
                let a = full.check(&[], model);
                full.reset();
                let b = split.check(&[], model);
                eprintln!(
                    "{model}: {} 件の診断、passed = {}",
                    a.diagnostics.len(),
                    a.passed()
                );
                assert_eq!(a, b, "{model}");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

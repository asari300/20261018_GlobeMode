//! ライブラリを使うときのモデル名の誤りに候補が出ること、`-L` にパッケージ 1 つだけを渡したときの案内（S180）。
//!
//! MSL を要らない小さなライブラリで確かめる。

use std::fs;
use std::path::{Path, PathBuf};

use globe_mode_core::{Diagnostic, codes};
use globe_mode_frontend::{Checker, Source};

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn first_error(checker: &Checker, model: &str) -> Diagnostic {
    let report = checker.check(&[], model);
    report.diagnostics.into_iter().next().expect("診断が出る")
}

/// `A`（package）の下に `A.Sub`（package）、`A.Sub.Decay`（検査を通る）、`A.Sub.Broken`（未定義の型を使う）。
fn small_library(name: &str) -> PathBuf {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&base);
    write(&base, "A/package.mo", "package A\nend A;\n");
    write(
        &base,
        "A/Sub/package.mo",
        "within A;\npackage Sub\nend Sub;\n",
    );
    write(
        &base,
        "A/Sub/Decay.mo",
        "within A.Sub;\nmodel Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n",
    );
    write(
        &base,
        "A/Sub/Broken.mo",
        "within A.Sub;\nmodel Broken\n  NoSuchType x;\nend Broken;\n",
    );
    base
}

#[test]
fn a_wrong_model_name_in_a_library_gets_candidates() {
    let base = small_library("hints_names");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    assert!(checker.check(&[], "A.Sub.Decay").passed());
    // 短い名前（末尾の一致）、綴りの誤り、`.mo` つき
    for model in ["Decay", "A.Sub.Decya", "A.Sub.Decay.mo"] {
        let d = first_error(&checker, model);
        assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{model}: {d:?}");
        assert!(
            d.notes
                .iter()
                .any(|n| n.starts_with("候補") && n.contains("A.Sub.Decay")),
            "{model}: {d:?}"
        );
    }
    // 似た名前が無いときは、完全修飾名で渡すよう案内する
    let d = first_error(&checker, "Zzzzzzzzzz");
    assert_eq!(d.code, codes::MODEL_NOT_FOUND);
    assert!(d.notes.iter().any(|n| n.contains("完全修飾名")), "{d:?}");
}

#[test]
fn an_existing_class_failing_for_another_reason_is_not_reported_as_missing() {
    let base = small_library("hints_existing");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    // 名前は正しいが、中の型が解決できない
    let d = first_error(&checker, "A.Sub.Broken");
    assert_ne!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
}

#[test]
fn a_package_name_says_it_is_a_package_and_lists_what_is_inside() {
    let base = small_library("hints_package");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    let d = first_error(&checker, "A.Sub");
    assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
    assert!(d.message.contains("パッケージ"), "{d:?}");
    assert!(
        d.notes
            .iter()
            .any(|n| n.starts_with("直下のモデル") && n.contains("A.Sub.Decay")),
        "{d:?}"
    );
    // モデルが直下に無ければ、パッケージを案内する
    let d = first_error(&checker, "A");
    assert!(
        d.notes
            .iter()
            .any(|n| n.starts_with("直下のパッケージ") && n.contains("A.Sub")),
        "{d:?}"
    );
}

#[test]
fn short_or_distant_names_get_no_candidates_and_open_documents_come_first() {
    let base = small_library("hints_threshold");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    // 4 文字で半分以上違う、3 文字以下は、距離では候補にしない
    for model in ["Dkay", "Dc", "T"] {
        let d = first_error(&checker, model);
        assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{model}");
        assert!(
            d.notes.iter().all(|n| !n.starts_with("候補")),
            "{model}: {d:?}"
        );
        assert!(
            d.notes.iter().any(|n| n.contains("完全修飾名")),
            "{model}: {d:?}"
        );
    }
    // 開いている文書のモデルは、名前が遠くても、ライブラリのモデルより先に候補に出る
    let text =
        "model Net\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Net;\n";
    let sources = [Source {
        name: "net.mo",
        text,
    }];
    let d = checker
        .check(&sources, "T")
        .diagnostics
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
    assert!(d.notes.contains(&"候補: Net".to_string()), "{d:?}");
}

/// S199: 遠い開いている文書のモデルを、ライブラリの近い一致より先に出さない（段が先、同じ段のなかで開いている文書が先）。
#[test]
fn a_distant_open_document_does_not_outrank_a_library_match() {
    let base = small_library("hints_order");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    let text =
        "model Net\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Net;\n";
    let sources = [Source {
        name: "net.mo",
        text,
    }];
    let d = checker
        .check(&sources, "Decay")
        .diagnostics
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
    assert_eq!(d.notes, ["候補: A.Sub.Decay, Net"], "{d:?}");
}

/// S199: 長すぎる `-m` は編集距離を飛ばす（100 KB で数十秒かかりうる）。綴りの候補は出ず、完全修飾名の案内になる。
#[test]
fn a_very_long_model_name_skips_the_spelling_distance() {
    let base = small_library("hints_long");
    let checker = Checker::new(&[base.as_path()]).unwrap();
    let started = std::time::Instant::now();
    let d = first_error(&checker, &"D".repeat(100_000));
    assert_eq!(d.code, codes::MODEL_NOT_FOUND);
    assert!(d.notes.iter().any(|n| n.contains("完全修飾名")), "{d:?}");
    assert!(started.elapsed().as_secs() < 5);
}

/// MSL 4.1.0 の `PID_Controller`（`Blocks/package.mo` の入れ子）。`data/msl` が無ければ何もしない。
#[test]
#[ignore]
fn msl_short_and_misspelled_names_get_candidates() {
    let msl = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    if !msl.is_dir() {
        return;
    }
    let checker = Checker::new(&[msl.as_path()]).unwrap();
    for model in ["PID_Controller", "Modelica.Blocks.Examples.PID_Contoller"] {
        let d = first_error(&checker, model);
        assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{model}: {d:?}");
        assert_eq!(
            d.notes,
            ["候補: Modelica.Blocks.Examples.PID_Controller"],
            "{model}"
        );
    }
}

/// MSL 4.1.0 を `-L` に渡したときの 4 つの案内（S187）。`data/msl` が無ければ何もしない。
#[test]
#[ignore]
fn msl_hints_for_subfolder_package_and_unrelated_names() {
    let msl = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    if !msl.is_dir() {
        return;
    }
    let sub = msl.join("Modelica 4.1.0");
    // 親のフォルダを案内する。CLI と同じ組で読む道で確かめる
    let mut texts = Vec::new();
    collect(&sub, &mut texts);
    let files: Vec<Source<'_>> = texts
        .iter()
        .map(|(n, t)| Source { name: n, text: t })
        .collect();
    let split = Checker::with_library_sources_split(sub.to_str().unwrap(), &files).unwrap();
    let d = first_error(&split, "Modelica.Blocks.Examples.PID_Controller");
    assert_eq!(d.code, codes::RESOLVE, "{d:?}");
    assert!(d.notes.iter().any(|n| n.contains("親のフォルダ")), "{d:?}");
    // パッケージ名と、遠い名前
    let checker = Checker::new(&[msl.as_path()]).unwrap();
    let d = first_error(&checker, "Modelica.Blocks.Examples");
    assert_eq!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
    assert!(
        d.notes.iter().any(|n| n.starts_with("直下のモデル")),
        "{d:?}"
    );
    for model in ["Nope", "T"] {
        let d = first_error(&checker, model);
        assert!(
            d.notes.iter().all(|n| !n.starts_with("候補")),
            "{model}: {d:?}"
        );
    }
}

fn collect(dir: &Path, out: &mut Vec<(String, String)>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push((p.display().to_string(), fs::read_to_string(&p).unwrap()));
        }
    }
}

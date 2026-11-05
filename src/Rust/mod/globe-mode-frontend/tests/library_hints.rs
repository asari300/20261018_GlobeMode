//! ライブラリを使うときのモデル名の誤りに候補が出ること、`-L` にパッケージ 1 つだけを渡したときの案内（S180）。
//!
//! MSL を要らない小さなライブラリで確かめる。

use std::fs;
use std::path::{Path, PathBuf};

use globe_mode_core::{Diagnostic, codes};
use globe_mode_frontend::Checker;

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
    // パッケージはクラスの索引にある（モデルではないが「見つからない」ではない）
    let d = first_error(&checker, "A.Sub");
    assert_ne!(d.code, codes::MODEL_NOT_FOUND, "{d:?}");
}

#[test]
fn a_subfolder_passed_as_the_library_points_to_its_parent() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hints_subfolder");
    let _ = fs::remove_dir_all(&base);
    write(
        &base,
        "ModelicaServices/package.mo",
        "package ModelicaServices\n  package Machine\n    constant Real eps = 1e-15;\n  end Machine;\nend ModelicaServices;\n",
    );
    write(
        &base,
        "Modelica/package.mo",
        "package Modelica\nend Modelica;\n",
    );
    write(
        &base,
        "Modelica/M.mo",
        "within Modelica;\nmodel M\n  parameter Real e = ModelicaServices.Machine.eps;\n  Real x(start = e, fixed = true);\nequation\n  der(x) = -x;\nend M;\n",
    );
    let hint = |roots: &[&Path], model: &str| {
        let d = first_error(&Checker::new(roots).unwrap(), model);
        d.notes.iter().find(|n| n.contains("親のフォルダ")).cloned()
    };
    let sub = base.join("Modelica");
    let note = hint(&[sub.as_path()], "Modelica.M").expect("案内が出る");
    assert!(note.contains(base.to_str().unwrap()), "{note}");
    // 親を渡せば、通る（案内も要らない）
    assert!(
        Checker::new(&[base.as_path()])
            .unwrap()
            .check(&[], "Modelica.M")
            .passed()
    );
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

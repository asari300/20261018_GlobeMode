//! ライブラリに構文の誤りのファイルが 1 つあっても、`Checker::new` が断らずに読めるファイルだけで読み、
//! 読み飛ばしたファイルを S1 の警告（`GM-W1001`）で知らせること（S066）。ルート 1 つと 2 つの両方。
//!
//! fixture は `tests/data/tolerant/`。`a/A/Broken.mo` が構文の誤り、`b/B/M.mo` は `a` の `A.Units.T` を使う。

use std::path::{Path, PathBuf};

use globe_mode_core::{Severity, codes};
use globe_mode_frontend::Checker;

fn root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/tolerant")
        .join(name)
}

fn assert_skipped_broken(checker: &Checker) {
    let skipped = checker.skipped_library_files();
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(skipped[0].path.ends_with("A/Broken.mo"), "{skipped:?}");
}

fn assert_warned(diagnostics: &[globe_mode_core::Diagnostic]) {
    let warned: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == codes::LIBRARY_PARTIAL)
        .collect();
    assert_eq!(warned.len(), 1, "{diagnostics:?}");
    assert_eq!(warned[0].severity, Severity::Warning);
    assert!(warned[0].notes.iter().any(|n| n.contains("Broken.mo")));
    assert!(
        !warned[0].notes[0].contains("Failed to parse"),
        "{:?}",
        warned[0].notes
    );
}

#[test]
fn one_root_with_a_broken_file_still_loads() {
    let a = root("a");
    let checker = Checker::new(&[a.as_path()]).expect("構文の誤りのファイルがあっても読める");
    assert_skipped_broken(&checker);
    let report = checker.check(&[], "A.Decay");
    assert!(report.passed(), "{:?}", report.diagnostics);
    assert_warned(&report.diagnostics);
    // 読み飛ばしたファイルのクラスは無い
    let report = checker.check(&[], "A.Broken");
    assert!(!report.passed());
}

#[test]
fn two_roots_with_a_broken_file_still_load() {
    let (a, b) = (root("a"), root("b"));
    let checker = Checker::new(&[a.as_path(), b.as_path()]).expect("2 つのルートを読める");
    assert_skipped_broken(&checker);
    let report = checker.check(&[], "B.M");
    assert!(report.passed(), "{:?}", report.diagnostics);
    assert_warned(&report.diagnostics);
    // 読み飛ばしたクラスを使うモデルは失敗する（警告も付く）
    let report = checker.check(&[], "B.NeedsBroken");
    assert!(!report.passed());
    assert_warned(&report.diagnostics);
}

#[test]
fn clean_roots_have_no_warning() {
    let b = root("b");
    // b だけ（A を引くモデルは解決できないが、読み込みは成り立つ）
    let checker = Checker::new(&[b.as_path()]).expect("読める");
    assert!(checker.skipped_library_files().is_empty());
    assert!(
        !checker
            .check(&[], "B.M")
            .diagnostics
            .iter()
            .any(|d| d.code == codes::LIBRARY_PARTIAL)
    );
}

#[test]
fn a_directory_without_package_mo_is_loaded_with_a_warning() {
    // Rumoca は `package.mo` の無いディレクトリ（PKG-006）でルート全体を断る。構文は全部読めるので、読み飛ばしは無い
    let c = root("c");
    let checker = Checker::new(&[c.as_path()]).expect("読める");
    assert!(checker.skipped_library_files().is_empty());
    let report = checker.check(&[], "C.M");
    assert!(report.passed(), "{:?}", report.diagnostics);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::LIBRARY_PARTIAL
                && d.notes.iter().any(|n| n.contains("PKG-006"))),
        "{:?}",
        report.diagnostics
    );
}

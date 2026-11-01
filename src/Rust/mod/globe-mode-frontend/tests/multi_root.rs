//! 2 つ以上のライブラリのルートを `Checker::new` に渡しても、ルートの間で名前を引けること（S063）。
//!
//! ルートごとに別の組として読むと、Rumoca がルートごとに作る要約の中で別のルートの名前が解決できず、
//! `import A.Units.T` が ER002（`GM-E1001`）になった（MSL のほかに Chemical などを読んだとき）。

use std::fs;
use std::path::{Path, PathBuf};

use globe_mode_frontend::Checker;

fn write(dir: &Path, rel: &str, text: &str) -> PathBuf {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_library_can_import_from_another_root() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("multi_root");
    let _ = fs::remove_dir_all(&base);
    let (a, b) = (base.join("a"), base.join("b"));
    write(&a, "A/package.mo", "package A\nend A;\n");
    write(
        &a,
        "A/Units.mo",
        "within A;\npackage Units\n  type T = Real(unit = \"K\");\nend Units;\n",
    );
    write(&b, "B/package.mo", "package B\nend B;\n");
    write(
        &b,
        "B/M.mo",
        "within B;\nmodel M\n  import A.Units.T;\n  T x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend M;\n",
    );
    for roots in [[a.as_path(), b.as_path()], [b.as_path(), a.as_path()]] {
        let checker = Checker::new(&roots).expect("2 つのルートを読める");
        let report = checker.check(&[], "B.M");
        assert!(report.passed(), "{:?}", report.diagnostics);
    }
}

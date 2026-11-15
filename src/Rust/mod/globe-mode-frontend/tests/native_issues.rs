//! S320: native の検査のライブラリの読み方（#113 開いて直したファイルが真、#114 UTF-8 でないファイルも読む）。

#![cfg(feature = "native")]

use std::fs;
use std::path::{Path, PathBuf};

use globe_mode_frontend::{Frontend, NativeFrontend, Source};

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn lib(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&d);
    d
}

/// #113: ディスクの `Lib/B.mo` は釣り合うが、エディタで `Real y;` を足した版（`Source`）が真
#[test]
fn edited_library_file_wins_over_disk() {
    let d = lib("s320-edited");
    write(&d, "Lib/package.mo", b"package Lib end Lib;\n");
    let b_path = d.join("Lib/B.mo");
    write(
        &d,
        "Lib/B.mo",
        b"within Lib; model B Real x; equation der(x) = -x; end B;\n",
    );
    let a = Source {
        name: "A.mo",
        text: "model A extends Lib.B; end A;\n",
    };
    let edited = Source {
        name: &b_path.display().to_string(),
        text: "within Lib; model B Real x; Real y; equation der(x) = -x; end B;\n",
    };
    let libs = [d.as_path()];
    let r = NativeFrontend::new().check(&[a, edited], &libs, "A");
    assert!(
        !r.passed(),
        "編集後の版（y が余る）で見る: {:?}",
        r.diagnostics
    );
    let ok = NativeFrontend::new().check(&[a], &libs, "A");
    assert!(ok.passed(), "{:?}", ok.diagnostics);
}

/// #114: Latin-1 の 1 バイトを含むライブラリのファイルも読む（U+FFFD にして）
#[test]
fn non_utf8_library_file_is_read() {
    let d = lib("s320-latin1");
    write(&d, "Lib/package.mo", b"package Lib end Lib;\n");
    write(
        &d,
        "Lib/B.mo",
        b"within Lib; model B \"caf\xe9 \xb0\" Real x; equation der(x) = -x; end B;\n",
    );
    let a = Source {
        name: "A.mo",
        text: "model A extends Lib.B; end A;\n",
    };
    let r = NativeFrontend::new().check(&[a], &[d.as_path()], "A");
    assert!(r.passed(), "{:?}", r.diagnostics);
}

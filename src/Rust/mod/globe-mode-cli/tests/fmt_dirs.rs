//! `globe fmt` / `globe lint` のディレクトリの走査（S060、ruff に倣う）。
//! .gitignore と隠しディレクトリを除く、壊れたファイルがあっても続ける、要約を出す、パイプの標準入力は今までどおり。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const MESSY: &str = "model A\nReal x(start=1);\nend A;\n";
const TIDY: &str = "model B\n\n  Real x(start = 1);\n\nend B;\n";

fn globe(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn tree(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("globe-fmt-dirs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    for sub in ["sub", "ignored", ".hidden"] {
        fs::create_dir_all(d.join(sub)).unwrap();
    }
    fs::write(d.join(".gitignore"), "ignored/\n").unwrap();
    fs::write(d.join("a.mo"), MESSY).unwrap();
    fs::write(d.join("sub/b.mo"), TIDY).unwrap();
    fs::write(d.join("ignored/c.mo"), MESSY).unwrap();
    fs::write(d.join(".hidden/d.mo"), MESSY).unwrap();
    fs::write(d.join("broken.mo"), "model E\n  Real x(\nend E;\n").unwrap();
    d
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn fmt_walks_like_ruff() {
    let d = tree("fmt");
    let o = globe(&d, &["fmt", "--check", "."]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(2), "{out}"); // broken.mo があるので 2
    assert!(out.contains("整形が要る: ./a.mo"), "{out}");
    assert!(!out.contains("c.mo") && !out.contains("d.mo"), "{out}");
    assert!(
        out.contains("1 ファイルは整形が要る、1 ファイルは整形済み"),
        "{out}"
    );
    assert_eq!(fs::read_to_string(d.join("a.mo")).unwrap(), MESSY); // --check は書かない

    let o = globe(&d, &["fmt", "a.mo", "sub"]);
    assert_eq!(o.status.code(), Some(0), "{}", stdout(&o));
    assert!(stdout(&o).contains("1 ファイルを整形した、1 ファイルは変更なし"));
    assert_ne!(fs::read_to_string(d.join("a.mo")).unwrap(), MESSY);
    assert_eq!(fs::read_to_string(d.join("ignored/c.mo")).unwrap(), MESSY);
    // 除外されたディレクトリの中でも、明示したファイルは整形する
    assert_eq!(globe(&d, &["fmt", "ignored/c.mo"]).status.code(), Some(0));
    assert_ne!(fs::read_to_string(d.join("ignored/c.mo")).unwrap(), MESSY);
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn lint_walks_and_summarizes() {
    let d = tree("lint");
    fs::remove_file(d.join("broken.mo")).unwrap();
    let o = globe(&d, &["lint", "."]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(out.contains("./a.mo:1:7: missing-description"), "{out}");
    assert!(!out.contains("c.mo") && !out.contains("d.mo"), "{out}");
    assert!(
        out.lines().last().unwrap().ends_with("（2 ファイル）"),
        "{out}"
    );
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn piped_stdin_still_formats_stdin() {
    let mut c = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmt"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(MESSY.as_bytes()).unwrap();
    let o = c.wait_with_output().unwrap();
    assert_eq!(stdout(&o), "model A\n\n  Real x(start = 1);\n\nend A;\n");
}

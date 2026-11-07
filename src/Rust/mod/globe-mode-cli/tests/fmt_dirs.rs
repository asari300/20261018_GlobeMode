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

/// `./a.mo` を OS の区切りで（Windows は `.\a.mo`）
fn rel(p: &str) -> String {
    Path::new(".").join(p).display().to_string()
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
    assert!(
        out.contains(&format!("整形が要る: {}", rel("a.mo"))),
        "{out}"
    );
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
    assert!(
        out.contains(&format!("{}:1:7: missing-description", rel("a.mo"))),
        "{out}"
    );
    assert!(!out.contains("c.mo") && !out.contains("d.mo"), "{out}");
    assert!(
        out.lines().last().unwrap().ends_with("（2 ファイル）"),
        "{out}"
    );
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn piped_stdin_formats_stdin_with_a_dash() {
    // S199: 標準入力は `-` を明示したときだけ（パスなしで標準入力は読まない。`--json` も同じ。S209）
    let mut c = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmt", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(MESSY.as_bytes()).unwrap();
    let o = c.wait_with_output().unwrap();
    assert_eq!(stdout(&o), "model A\n\n  Real x(start = 1);\n\nend A;\n");
}

#[test]
fn lint_reports_connect_with_different_types_across_files() {
    let d = std::env::temp_dir().join(format!("globe-lint-types-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    fs::write(
        d.join("Parts.mo"),
        "package Parts \"p\"\n\n  connector Pin \"p\"\n    Real v;\n    flow Real i;\n  end Pin;\n\n  \
         connector Port \"p\"\n    Real T;\n    flow Real Q;\n  end Port;\n\n  \
         model R \"p\"\n    Pin p;\n    Port h;\n  end R;\n\nend Parts;\n",
    )
    .unwrap();
    fs::write(
        d.join("Top.mo"),
        "model Top \"t\"\n\n  Parts.R a;\n  Parts.R b;\n\nequation\n\n  connect(a.p, b.p);\n  connect(a.p, b.h);\n\nend Top;\n",
    )
    .unwrap();
    let o = globe(&d, &["lint", "."]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(
        out.contains(&format!(
            "{}:9:11: connect-type-mismatch: a.p は Parts.Pin、b.h は Parts.Port で、型が合わない",
            rel("Top.mo")
        )),
        "{out}"
    );
    assert!(out.contains("1 件（2 ファイル）"), "{out}");
    let o = globe(&d, &["lint", "--disable", "connect-type-mismatch", "."]);
    assert_eq!(o.status.code(), Some(0), "{}", stdout(&o));
}

/// S130: 対象から上へたどった `globe-mode.toml` の `[fmt]`・`[lint]` が効き、コマンドラインの引数が優先する
#[test]
fn config_file_changes_fmt_and_lint() {
    let d = std::env::temp_dir().join(format!("globe-fmt-config-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("sub")).unwrap();
    fs::write(d.join("sub/b.mo"), TIDY).unwrap();
    let check = |args: &[&str]| {
        globe(&d, &[&["fmt", "--check"], args].concat())
            .status
            .code()
    };
    assert_eq!(check(&["sub"]), Some(0));
    fs::write(d.join("globe-mode.toml"), "[fmt]\nindent = 4\n").unwrap();
    assert_eq!(check(&["sub"]), Some(1)); // 2 字下げの TIDY は 4 では整形が要る
    assert_eq!(check(&["--indent", "2", "sub"]), Some(0)); // 引数が優先

    let lint = |d: &Path| globe(d, &["lint", "sub/b.mo"]);
    assert!(stdout(&lint(&d)).contains("missing-description"));
    fs::write(
        d.join("globe-mode.toml"),
        "[lint]\ndisable = [\"GM-L0003\"]\n",
    )
    .unwrap();
    let o = lint(&d);
    assert_eq!(o.status.code(), Some(0), "{}", stdout(&o));

    fs::write(d.join("globe-mode.toml"), "[fmt]\nindnt = 4\n").unwrap();
    let o = globe(&d, &["fmt", "--check", "sub"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("知らないキー fmt.indnt"));
    fs::remove_dir_all(&d).unwrap();
}

fn globe_stdin(args: &[&str], input: &str) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    c.wait_with_output().unwrap()
}

#[test]
fn check_with_stdin_exits_1_when_formatting_is_needed() {
    // S211（#26）: `--check -` は書き換えを出さず、整形が要れば 1、要らなければ 0
    let o = globe_stdin(&["fmt", "--check", "-"], MESSY);
    assert_eq!(o.status.code(), Some(1), "{}", stdout(&o));
    assert_eq!(stdout(&o), "");
    let o = globe_stdin(&["fmt", "--check", "-"], TIDY);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(stdout(&o), "");
    // 壊れた入力は今までどおり 1（--check が無くても同じ）
    let o = globe_stdin(&["fmt", "--check", "-"], "model E\n  Real x(\nend E;\n");
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn bom_files_are_formatted_and_keep_their_bom() {
    // S211（#32）: UTF-8 の BOM つきでも整形でき、書き戻しても BOM は残る
    let d = tree("bom");
    let p = d.join("bom.mo");
    fs::write(&p, format!("\u{feff}{MESSY}")).unwrap();
    assert_eq!(
        globe(&d, &["fmt", "--check", "bom.mo"]).status.code(),
        Some(1)
    );
    assert_eq!(globe(&d, &["fmt", "bom.mo"]).status.code(), Some(0));
    let text = fs::read_to_string(&p).unwrap();
    assert!(text.starts_with("\u{feff}model A\n"), "{text:?}");
    assert_eq!(
        globe(&d, &["fmt", "--check", "bom.mo"]).status.code(),
        Some(0)
    );
    assert_eq!(globe(&d, &["lint", "bom.mo"]).status.code(), Some(1)); // missing-description だけ
    let o = globe_stdin(&["fmt", "-"], &format!("\u{feff}{MESSY}"));
    assert!(
        stdout(&o).starts_with("\u{feff}model A\n"),
        "{}",
        stdout(&o)
    );
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn fmt_writes_through_a_temp_file_and_keeps_mode_and_symlink() {
    // S211（#27）: 書き戻しは同じフォルダの一時ファイルから rename。一時ファイルを残さず、権限とリンクを保つ
    let d = tree("atomic");
    let names = |dir: &Path| -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };
    let before = names(&d);
    assert_eq!(globe(&d, &["fmt", "a.mo"]).status.code(), Some(0));
    assert_eq!(names(&d), before, "一時ファイルが残った");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        fs::write(d.join("m.mo"), MESSY).unwrap();
        fs::set_permissions(d.join("m.mo"), fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(globe(&d, &["fmt", "m.mo"]).status.code(), Some(0));
        let mode = fs::metadata(d.join("m.mo")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
        // リンクは置き換えず、指す先を書き換える
        fs::write(d.join("real.mo"), MESSY).unwrap();
        symlink("real.mo", d.join("link.mo")).unwrap();
        assert_eq!(globe(&d, &["fmt", "link.mo"]).status.code(), Some(0));
        assert!(
            fs::symlink_metadata(d.join("link.mo"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_ne!(fs::read_to_string(d.join("real.mo")).unwrap(), MESSY);
        // 読み取り専用のファイルは今までどおり書けない（rename で黙って上書きしない）
        fs::write(d.join("ro.mo"), MESSY).unwrap();
        fs::set_permissions(d.join("ro.mo"), fs::Permissions::from_mode(0o444)).unwrap();
        if fs::OpenOptions::new()
            .write(true)
            .open(d.join("ro.mo"))
            .is_err()
        {
            // root では 0o444 でも書けるので、書けないときだけ確かめる
            assert_eq!(globe(&d, &["fmt", "ro.mo"]).status.code(), Some(2));
            assert_eq!(fs::read_to_string(d.join("ro.mo")).unwrap(), MESSY);
        }
    }
    fs::remove_dir_all(&d).unwrap();
}

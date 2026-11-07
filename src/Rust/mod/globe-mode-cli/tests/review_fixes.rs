//! S199（CLI のレビュー指摘）: 利用者のファイルを壊さない、終了コードの体系（0 / 1 / 2 / 3）、`| head` と書き込みの失敗、
//! `fmt` の標準入力は `-` を明示したときだけ。パスの区切りが出る文面は `contains` で確かめる。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const DECAY_SRC: &str =
    "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
/// `1 / 0` は評価できず、Rumoca が未対応として止まる（GM-U）
const DIV_SRC: &str = "model Div\n  parameter Real p = 1 / 0;\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Div;\n";
const MESSY: &str = "model A\nReal x(start=1);\nend A;\n";

fn globe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run globe")
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

/// 空の作業フォルダ（テストごとに別の名前）
fn work(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-s199-{name}"));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// 指摘 1: `-o` に入力の `.mo` やライブラリのファイルを渡しても、ソースを上書きしない（終了コード 2、計算の前）。
#[test]
fn output_never_overwrites_sources() {
    let d = work("overwrite");
    let model = d.join("m.mo");
    fs::write(&model, DECAY_SRC).unwrap();
    let o = globe(&["simulate", s(&model), "-m", "Decay", "-o", s(&model)]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(err(&o).contains("m.mo"), "{}", err(&o));
    assert!(out(&o).is_empty(), "計算の前に止まる: {}", out(&o));
    assert_eq!(fs::read_to_string(&model).unwrap(), DECAY_SRC);

    // 拡張子が .mo なら、まだ無いファイルも作らない
    let new_mo = d.join("new.MO");
    let o = globe(&["simulate", s(&model), "-m", "Decay", "-o", s(&new_mo)]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(!new_mo.exists());

    // 拡張子が .mo でない入力のファイルも同じ
    let txt = d.join("m.txt");
    fs::write(&txt, DECAY_SRC).unwrap();
    let o = globe(&["simulate", s(&txt), "-m", "Decay", "-o", s(&txt)]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert_eq!(fs::read_to_string(&txt).unwrap(), DECAY_SRC);

    // ライブラリの配下
    let lib = d.join("lib");
    fs::create_dir_all(lib.join("A")).unwrap();
    let package = lib.join("A/package.mo");
    fs::write(&package, "package A\nend A;\n").unwrap();
    for target in [package.clone(), lib.join("A/result.csv")] {
        let o = globe(&[
            "simulate",
            s(&model),
            "-L",
            s(&lib),
            "-m",
            "Decay",
            "-o",
            s(&target),
        ]);
        assert_eq!(o.status.code(), Some(2), "{target:?}: {o:?}");
    }
    assert_eq!(fs::read_to_string(&package).unwrap(), "package A\nend A;\n");
    assert!(!lib.join("A/result.csv").exists());

    // 普通の出力先は今までどおり
    let ok = d.join("ok.csv");
    let o = globe(&["simulate", s(&model), "-m", "Decay", "-o", s(&ok)]);
    assert!(o.status.success(), "{o:?}");
    assert!(ok.is_file());
}

/// 指摘 1（FMU）: `fmu export --out` の先に入力・ライブラリのフォルダがあるとき、それを消さない。
#[test]
fn fmu_export_does_not_remove_a_source_folder() {
    let d = work("fmu-root");
    // `Decay` というモデルの書き出し先 `<out>/Decay/` が、ライブラリのパッケージ `Decay/` と重なる
    let lib = d.join("Decay");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("package.mo"), "package Decay\nend Decay;\n").unwrap();
    let model = d.join("m.mo");
    fs::write(&model, DECAY_SRC).unwrap();
    let o = globe(&[
        "fmu",
        "export",
        s(&model),
        "-L",
        s(&d),
        "-m",
        "Decay",
        "--out",
        s(&d),
    ]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(lib.join("package.mo").is_file(), "{}", err(&o));
}

/// 指摘 2: 壊れた symlink を消さず、リンク先に空のファイルを作らない。
#[cfg(unix)]
#[test]
fn a_broken_symlink_is_neither_removed_nor_followed() {
    let d = work("symlink");
    let model = d.join("m.mo");
    fs::write(&model, DECAY_SRC).unwrap();
    let target = d.join("missing-target.csv");
    let link = d.join("link.csv");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let o = globe(&["simulate", s(&model), "-m", "Decay", "-o", s(&link)]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        link.symlink_metadata().unwrap().file_type().is_symlink(),
        "リンクが消えた"
    );
    assert!(!target.exists(), "リンク先に空のファイルができた");
}

/// 指摘 10: 事前検査は、既にあるファイルの中身・権限・更新時刻を変えず、計算が失敗したら新しいファイルを残さない。
#[cfg(unix)]
#[test]
fn the_precheck_leaves_files_alone_and_a_failed_run_leaves_none() {
    use std::os::unix::fs::PermissionsExt;
    let d = work("precheck");
    let model = d.join("m.mo");
    fs::write(&model, DECAY_SRC).unwrap();
    let existing = d.join("existing.csv");
    fs::write(&existing, "keep\n").unwrap();
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o640)).unwrap();
    let before = fs::metadata(&existing).unwrap();
    // 負の終了時刻は実行の設定の誤り（計算に入らない）
    let o = globe(&[
        "simulate",
        s(&model),
        "-m",
        "Decay",
        "--stop-time",
        "-1",
        "-o",
        s(&existing),
    ]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let after = fs::metadata(&existing).unwrap();
    assert_eq!(fs::read_to_string(&existing).unwrap(), "keep\n");
    assert_eq!(after.permissions().mode(), before.permissions().mode());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());

    let fresh = d.join("fresh.csv");
    let o = globe(&[
        "simulate",
        s(&model),
        "-m",
        "Decay",
        "--stop-time",
        "-1",
        "-o",
        s(&fresh),
    ]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(!fresh.exists(), "失敗した計算が新しいファイルを残した");
}

/// 指摘 6: ライブラリのパッケージ名の診断（直下のモデルの案内つき）に、ワークスペースのモデルの「候補」を足さない。
#[test]
fn a_package_name_gets_no_workspace_candidates() {
    let d = work("package-candidates");
    let lib = d.join("lib");
    fs::create_dir_all(lib.join("A/Sub")).unwrap();
    fs::write(lib.join("A/package.mo"), "package A\nend A;\n").unwrap();
    fs::write(
        lib.join("A/Sub/package.mo"),
        "within A;\npackage Sub\nend Sub;\n",
    )
    .unwrap();
    fs::write(lib.join("A/Sub/M.mo"), "within A.Sub;\nmodel M\nend M;\n").unwrap();
    let model = d.join("m.mo");
    fs::write(&model, DECAY_SRC).unwrap();
    let o = globe(&["check", s(&model), "-L", s(&lib), "-m", "A.Sub"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let text = out(&o);
    assert!(text.contains("直下のモデル: A.Sub.M"), "{text}");
    assert!(!text.contains("候補"), "{text}");
}

/// 指摘 3・9: `-o -` のとき、stderr の診断に位置と補足が付き、ライブラリを読めない報告も stderr に出る（stdout は CSV だけ）。
#[test]
fn with_stdout_output_diagnostics_go_to_stderr_in_full() {
    let d = work("stderr-diag");
    let model = d.join("m.mo");
    fs::write(
        &model,
        "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -y;\nend Decay;\n",
    )
    .unwrap();
    let o = globe(&["simulate", s(&model), "-m", "Decay", "-o", "-"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(out(&o).is_empty(), "stdout は CSV だけ: {}", out(&o));
    let e = err(&o);
    // 位置（ファイル:行:列）が付く
    assert!(e.contains("m.mo:4:"), "{e}");

    // ライブラリを読めない（ディレクトリでない）ではなく、中身が壊れている
    let lib = d.join("lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("Bad.mo"), "model Bad\n  Real x(\nend Bad;\n").unwrap();
    let o = globe(&[
        "simulate",
        s(&model),
        "-L",
        s(&lib),
        "-m",
        "Decay",
        "-o",
        "-",
    ]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(out(&o).is_empty(), "stdout は CSV だけ: {}", out(&o));
    assert!(!err(&o).is_empty());
}

/// 指摘 4: `-o - | head` で読み手が閉じても、`--json` の報告を stderr に出して終わる。
#[cfg(unix)]
#[test]
fn closed_stdout_still_reports_the_run() {
    use std::io::{BufRead, BufReader};
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    let mut child = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["simulate", s(&dir), "-m"])
        .arg("GlobeModeValidation.Simulation.Decay")
        .args(["--interval", "1e-5", "-o", "-", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert!(err(&o).contains("\"succeeded\": true"), "{}", err(&o));
}

/// 標準出力を読まずに閉じて起動する（`| head` で先に閉じられたのと同じ）。
#[cfg(unix)]
fn run_with_closed_stdout(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    child.wait_with_output().unwrap()
}

/// 指摘 5: 読み手が先に閉じても、検査・整形の結果の終了コードを保つ（成功は 0、失敗は 1）。panic も 141 もない。
#[cfg(unix)]
#[test]
fn closed_stdout_keeps_the_exit_code() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    let decay = "GlobeModeValidation.Simulation.Decay";
    let no_noise = |o: &Output| {
        assert!(!err(o).contains("panicked"), "{}", err(o));
        assert!(!err(o).contains("Broken pipe"), "{}", err(o));
    };

    let o = run_with_closed_stdout(&["check", s(&dir), "-m", decay]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    no_noise(&o);

    let o = run_with_closed_stdout(&["check", s(&dir), "-m", "GlobeModeValidation.Nope"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    no_noise(&o);

    let d = work("closed-fmt");
    fs::write(d.join("a.mo"), MESSY).unwrap();
    fs::write(d.join("b.mo"), MESSY).unwrap();
    let o = run_with_closed_stdout(&["fmt", "--check", s(&d)]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    no_noise(&o);

    let o = run_with_closed_stdout(&["deps", s(&dir), "-m", decay]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    no_noise(&o);
}

/// 指摘 5: 本物の書き込みの失敗（ディスク満杯）は panic ではなく、stderr に 1 行出して終了コード 2。
#[cfg(unix)]
#[test]
fn a_full_disk_is_exit_code_two_without_a_panic() {
    let Ok(full) = fs::OpenOptions::new().write(true).open("/dev/full") else {
        return;
    };
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args([
            "check",
            s(&dir),
            "-m",
            "GlobeModeValidation.Simulation.Decay",
        ])
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    let e = err(&o);
    assert!(!e.contains("panicked"), "{e}");
    assert_eq!(e.trim().lines().count(), 1, "{e}");
    assert!(e.contains("標準出力に書けない"), "{e}");
}

/// 終了コード 3: 未対応（GM-U）だけで止まったときは、モデルの誤り（1）と分ける。`check`・`simulate` とも。
#[test]
fn unsupported_only_is_exit_code_three() {
    let d = work("unsupported");
    let div = d.join("Div.mo");
    fs::write(&div, DIV_SRC).unwrap();
    for args in [
        vec!["check", s(&div), "-m", "Div"],
        vec!["check", s(&div), "-m", "Div", "--json"],
        vec!["simulate", s(&div), "-m", "Div"],
        vec!["simulate", s(&div), "-m", "Div", "--json"],
    ] {
        let o = globe(&args);
        assert_eq!(o.status.code(), Some(3), "{args:?}: {o:?}");
    }
    let o = globe(&["check", s(&div), "-m", "Div"]);
    assert!(out(&o).contains("unsupported["), "{}", out(&o));

    // モデルの誤りは 1 のまま
    let o = globe(&["check", s(&div), "-m", "Nope"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
}

/// `fmt` / `lint` の標準入力は `-` を明示したときだけ（`--json` も同じ。S209）。パスも `-` も無ければ使い方で終了コード 2。
#[test]
fn stdin_needs_an_explicit_dash() {
    use std::io::Write;
    let run = |args: &[&str], input: &str| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = c.stdin.take().unwrap().write_all(input.as_bytes());
        c.wait_with_output().unwrap()
    };
    // S209: `--json` もパスか `-` が要る
    for args in [
        &["fmt"][..],
        &["fmt", "--check"],
        &["lint"],
        &["fmt", "--json"],
        &["lint", "--json"],
    ] {
        let o = run(args, MESSY);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {o:?}");
        assert!(
            err(&o).contains(&format!("globe {} -", args[0])),
            "{}",
            err(&o)
        );
        assert!(out(&o).is_empty(), "{}", out(&o));
    }
    let o = run(&["fmt", "-"], MESSY);
    assert_eq!(
        out(&o),
        "model A\n\n  Real x(start = 1);\n\nend A;\n",
        "{o:?}"
    );
    let o = run(&["fmt", "--json", "-"], MESSY);
    assert!(out(&o).contains("\"ok\":true"), "{}", out(&o));
    let o = run(&["lint", "--json", "-"], MESSY);
    assert!(out(&o).contains("\"messages\""), "{}", out(&o));
}

/// ヘルプの末尾に終了コードの表がある。
#[test]
fn help_lists_the_exit_codes() {
    let o = globe(&["--help"]);
    let text = out(&o);
    assert!(text.contains("終了コード"), "{text}");
    for code in ["0", "1", "2", "3"] {
        assert!(
            text.lines().any(|l| l.trim_start().starts_with(code)),
            "{code}: {text}"
        );
    }
    assert!(text.contains("未対応"), "{text}");
}

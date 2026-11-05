//! CLI の使える流れ（S182）: `simulate` の `-o` の有無、書けない出力先、`| head`、負の数、ヘルプ、
//! `--force` の重複、最後の行の理由、info のまとめ。パスの区切りが違う文面は `contains` で確かめる。

use std::path::PathBuf;
use std::process::{Command, Output};

fn modelica(package: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica");
    p.join(package).to_str().unwrap().to_string()
}

fn globe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .output()
        .expect("run globe")
}

const DECAY: &str = "GlobeModeValidation.Simulation.Decay";

fn decay(extra: &[&str]) -> Output {
    let dir = modelica("GlobeModeValidation");
    let mut args = vec!["simulate", &dir, "-m", DECAY];
    args.extend_from_slice(extra);
    globe(&args)
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n")
}

fn tmp(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-cli-flow-{name}"))
}

/// 1: `-o` が無ければ、結果を書いていないと言う。あれば書いた先と行数を言う。`--json` は stderr に何も足さない。
#[test]
fn simulate_says_whether_the_result_was_written() {
    let o = decay(&[]);
    assert!(o.status.success(), "{o:?}");
    assert!(
        err(&o).contains("結果は書いていない（t = 1 まで、501 行、変数 1）"),
        "{}",
        err(&o)
    );
    assert!(err(&o).contains("-o <file.csv>"), "{}", err(&o));

    let path = tmp("written.csv");
    let o = decay(&["-o", path.to_str().unwrap()]);
    assert!(o.status.success(), "{o:?}");
    assert!(err(&o).is_empty(), "{}", err(&o));
    let text = out(&o);
    assert!(text.contains("書いた: "), "{text}");
    assert!(text.contains("（CSV、501 行）"), "{text}");

    let o = decay(&["--json"]);
    assert!(err(&o).is_empty(), "{}", err(&o));
}

/// 2: 書けない出力先は計算の前に分かり、パスと理由を言う（`os error` は出さない）。
#[test]
fn unwritable_destinations_are_reported_before_computing() {
    let missing = tmp("no-such-dir").join("x.csv");
    let o = decay(&["-o", missing.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    let e = err(&o);
    assert!(e.contains("に書けない: 親のフォルダ"), "{e}");
    assert!(e.contains("x.csv"), "{e}");
    assert!(!e.contains("os error"), "{e}");
    // 計算の前に止まるので、要約は出ていない
    assert!(out(&o).is_empty(), "{}", out(&o));
    assert!(!missing.exists());

    let dir = tmp("a-directory");
    std::fs::create_dir_all(&dir).unwrap();
    let o = decay(&["-o", dir.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        err(&o).contains("に書けない: ディレクトリである"),
        "{}",
        err(&o)
    );

    let o = globe(&["check", "no-such-file.mo", "-m", "X"]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    let e = err(&o);
    assert!(
        e.contains("no-such-file.mo を読めない: 見つからない"),
        "{e}"
    );
    assert!(!e.contains("os error"), "{e}");

    // FMU の書き出し先がファイルを指している
    let file = tmp("a-file");
    std::fs::write(&file, "x").unwrap();
    let dir = modelica("GlobeModeValidation");
    let o = globe(&[
        "fmu",
        "export",
        &dir,
        "-m",
        DECAY,
        "--out",
        file.to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        err(&o).contains("に書けない: ディレクトリではない"),
        "{}",
        err(&o)
    );
}

/// 3: 標準出力が閉じられても（`| head`）、黙って終了コード 0。
#[cfg(unix)]
#[test]
fn closed_stdout_is_not_an_error() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    let dir = modelica("GlobeModeValidation");
    let mut child = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args([
            "simulate",
            &dir,
            "-m",
            DECAY,
            "--interval",
            "1e-5",
            "-o",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert!(first.starts_with("\"time\""), "{first}");
    // stdout を閉じた（BufReader を落とした）ので、あとの書き込みは BrokenPipe になる
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert!(!err(&o).contains("Broken pipe"), "{}", err(&o));
    assert!(!err(&o).contains("panicked"), "{}", err(&o));
}

/// 4: 負の時刻とパラメータ。負の終了時刻は意味の誤り（GM-E10004）で、clap の誤りではない。
#[test]
fn negative_numbers_are_values() {
    let o = decay(&["--start-time", "-2", "--stop-time", "3"]);
    assert!(o.status.success(), "{o:?}");
    assert!(out(&o).contains("区間 -2 → 3"), "{}", out(&o));

    let o = decay(&["--stop-time", "-1"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(out(&o).contains("GM-E10004"), "{}", out(&o));
    assert!(!err(&o).contains("unexpected argument"), "{}", err(&o));

    let dir = modelica("GlobeModeValidation");
    let o = globe(&[
        "simulate",
        &dir,
        "-m",
        "GlobeModeValidation.Simulation.Parameters",
        "--param",
        "k=-1",
        "--stop-time",
        "0.1",
    ]);
    assert!(o.status.success(), "{o:?}");
    assert!(out(&o).contains("k = -1"), "{}", out(&o));
}

/// 5: ヘルプ。検査の段階は S0〜S10、内部のセッション番号（S123 など）は出さない。各副命令に例がある。
#[test]
fn help_has_examples_and_no_session_numbers() {
    let check = out(&globe(&["check", "--help"]));
    assert!(check.contains("S0〜S10"), "{check}");
    assert!(!check.contains("S0〜S8"), "{check}");
    let subcommands = [
        &["outline"][..],
        &["graph"],
        &["check"],
        &["fmt"],
        &["lint"],
        &["lsp"],
        &["deps"],
        &["simulate"],
        &["flat"],
        &["fmu", "export"],
    ];
    for sub in subcommands {
        let mut args = sub.to_vec();
        args.push("--help");
        let text = out(&globe(&args));
        assert!(text.contains("例:"), "{sub:?}\n{text}");
        let b = text.as_bytes();
        let numbered = (0..b.len().saturating_sub(3)).any(|i| {
            b[i] == b'S'
                && b[i + 1..i + 4].iter().all(u8::is_ascii_digit)
                && !b[i - 1].is_ascii_alphanumeric()
        });
        assert!(!numbered, "{sub:?} にセッション番号\n{text}");
    }
    for top in [out(&globe(&["--help"])), out(&globe(&["fmu", "--help"]))] {
        assert!(!top.contains("S094"), "{top}");
    }
}

/// 6: `--force` で同じ診断を 2 回出さず、最後の行に最初の error のコードと 1 文を入れる。
#[test]
fn force_does_not_repeat_diagnostics_and_last_line_gives_the_reason() {
    let dir = modelica("GlobeModeExamples");
    let o = globe(&[
        "simulate",
        &dir,
        "-m",
        "GlobeModeExamples.Unbalanced",
        "--force",
    ]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let text = out(&o);
    assert_eq!(text.matches("error[GM-E5002]").count(), 1, "{text}");
    let last = text.lines().last().unwrap();
    assert!(
        last.contains(
            "DAE まで変換できないので実行できない。最初の error: GM-E5002 unbalanced model"
        ),
        "{last}"
    );

    let o = decay(&["--stop-time", "-1"]);
    let text = out(&o);
    let last = text.lines().last().unwrap();
    assert!(
        last.contains("実行に失敗した。最初の error: GM-E10004 "),
        "{last}"
    );

    // --json の refused は変えない
    let o = decay(&["--stop-time", "-1", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(
        !v["refused"].as_str().unwrap_or("").contains("最初の error"),
        "{v}"
    );
}

/// 7: info は件数だけ（`-v` で全部）。`--json` には全部入る。
#[test]
fn info_diagnostics_are_summarised_unless_verbose() {
    let fixed = ["--solver", "rk4", "--step", "0.01", "--stop-time", "0.1"];
    let text = out(&decay(&fixed));
    assert!(
        text.contains("info 1 件を省いた（GM-I10004。-v で全部表示）"),
        "{text}"
    );
    assert!(!text.contains("info[GM-I10004]"), "{text}");

    let mut verbose = fixed.to_vec();
    verbose.push("-v");
    let text = out(&decay(&verbose));
    assert!(text.contains("info[GM-I10004]"), "{text}");
    assert!(!text.contains("件を省いた"), "{text}");

    let mut json = fixed.to_vec();
    json.push("--json");
    assert!(out(&decay(&json)).contains("\"GM-I10004\""));
}

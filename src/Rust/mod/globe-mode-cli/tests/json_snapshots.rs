//! `--json` の形の snapshot（S035）。CLI と wasm の JSON は `globe-mode-protocol` の型から出すので、
//! 型のフィールドを足す・並べ替えると、ここが落ちる。意図した変更なら `UPDATE_SNAPSHOTS=1` で書き直す。
//!
//! 時計の値（`stats.*_seconds`）を含む出力は snapshot にしない（検査の JSON と、実行を断った JSON だけ）。

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn globe_json(args: &[&str]) -> String {
    // ファイル名が診断に入るので、リポジトリの根から相対パスで呼ぶ
    let o = Command::new(env!("CARGO_BIN_EXE_globe"))
        .current_dir(root())
        .args(args)
        .output()
        .expect("run globe");
    String::from_utf8(o.stdout).unwrap()
}

fn assert_snapshot(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.json"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    // Windows の checkout は autocrlf で fixture が CRLF になる（run 41018689866）。改行だけそろえる
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        expected == actual,
        "{name}: snapshot と違う（意図した変更なら UPDATE_SNAPSHOTS=1 で書き直す）\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

const EX: &str = "src/Modelica/GlobeModeExamples/package.mo";
const VA: &str = "src/Modelica/GlobeModeValidation";

#[test]
fn check_json_keeps_its_shape() {
    assert_snapshot(
        "check-rc",
        &globe_json(&["check", EX, "-m", "GlobeModeExamples.RC", "--json"]),
    );
    assert_snapshot(
        "check-unbalanced",
        &globe_json(&["check", EX, "-m", "GlobeModeExamples.Unbalanced", "--json"]),
    );
}

#[test]
fn simulate_json_keeps_its_shape() {
    assert_snapshot(
        "simulate-unbalanced",
        &globe_json(&[
            "simulate",
            EX,
            "-m",
            "GlobeModeExamples.Unbalanced",
            "--json",
        ]),
    );
    assert_snapshot(
        "list-parameters",
        &globe_json(&[
            "simulate",
            VA,
            "-m",
            "GlobeModeValidation.Simulation.Parameters",
            "--list-parameters",
            "--json",
        ]),
    );
}

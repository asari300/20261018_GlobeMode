//! `globe results`（S219）。omc 1.27.2 が書いた MAT（globe-mode-sim の tests/data）を CSV に、CSV を MAT に直す。

use std::path::PathBuf;
use std::process::{Command, Output};

const OMC_MAT: &[u8] = include_bytes!("../../globe-mode-sim/tests/data/omc_decay_res.mat");

fn globe(args: &[&std::path::Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .arg("results")
        .args(args)
        .output()
        .expect("run globe")
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-results-{name}"))
}

#[test]
fn mat_to_csv_on_stdout_and_back_to_mat() {
    let mat = path("omc.mat");
    std::fs::write(&mat, OMC_MAT).unwrap();
    let o = globe(&[&mat]);
    assert!(o.status.success(), "{o:?}");
    let csv = String::from_utf8(o.stdout).unwrap();
    assert!(csv.starts_with("\"time\",\"x\""), "{csv}");
    assert_eq!(csv.lines().count(), 1 + 6, "{csv}");

    let csv_path = path("omc.csv");
    let o = globe(&[&mat, "-o".as_ref(), &csv_path]);
    assert!(o.status.success(), "{o:?}");
    assert_eq!(std::fs::read_to_string(&csv_path).unwrap(), csv);

    // CSV -> MAT -> CSV で同じ
    let mat2 = path("again.mat");
    let o = globe(&[&csv_path, "-o".as_ref(), &mat2]);
    assert!(o.status.success(), "{o:?}");
    let o = globe(&[&mat2]);
    assert_eq!(String::from_utf8(o.stdout).unwrap(), csv);
}

#[test]
fn refuses_to_overwrite_the_input_and_reports_unreadable_input() {
    let mat = path("keep.mat");
    std::fs::write(&mat, OMC_MAT).unwrap();
    let o = globe(&[&mat, "-o".as_ref(), &mat]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert_eq!(std::fs::read(&mat).unwrap(), OMC_MAT);

    let o = globe(&[&path("none.mat")]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(String::from_utf8_lossy(&o.stderr).contains("見つからない"));

    let bad = path("bad.mat");
    std::fs::write(&bad, b"not a mat file at all").unwrap();
    let o = globe(&[&bad]);
    assert_eq!(o.status.code(), Some(2), "{o:?}");
}

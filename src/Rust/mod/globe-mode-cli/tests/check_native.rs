//! `globe check --frontend native`（S305）: 自前の段の検査が、Rumoca の道と同じ合否と釣り合いを出し、
//! 構文の誤り・未解決の名前は位置つきで、未対応は `GM-U0006`（終了コード 3）で断ること。

use std::path::PathBuf;
use std::process::{Command, Output};

fn validation() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    p.to_str().unwrap().to_string()
}

fn globe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(args)
        .output()
        .expect("run globe")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

fn check(frontend: &str, model: &str) -> Output {
    globe(&["check", &validation(), "-m", model, "--frontend", frontend])
}

/// 「方程式 N / 変数 M」の行（モデル名の後ろ）
fn balance_line(o: &Output) -> String {
    out(o)
        .lines()
        .find(|l| l.contains(": 方程式 "))
        .unwrap_or_default()
        .to_string()
}

fn temp_model(name: &str, text: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check-native");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn same_verdict_and_balance_as_rumoca() {
    for (m, code) in [
        ("Simulation.Decay", Some(0)),
        ("Index.Pendulum", Some(0)),
        ("Connect.RealCircuit", Some(0)),
        ("Invalid.StructurallySingular", Some(1)),
    ] {
        let model = format!("GlobeModeValidation.{m}");
        let (r, n) = (check("rumoca", &model), check("native", &model));
        assert_eq!(r.status.code(), code, "{m} rumoca: {}", out(&r));
        assert_eq!(n.status.code(), code, "{m} native: {}", out(&n));
        assert_eq!(balance_line(&r), balance_line(&n), "{m}");
    }
    let n = check("native", "GlobeModeValidation.Invalid.StructurallySingular");
    assert!(out(&n).contains("GM-E6001"), "{}", out(&n));
    let ok = check("native", "GlobeModeValidation.Simulation.Decay");
    assert!(out(&ok).contains("native: S0〜S6 を通過"), "{}", out(&ok));
}

#[test]
fn syntax_error_and_unresolved_name_have_positions() {
    let bad = temp_model(
        "s.mo",
        "model S\n  Real x;\nequation\n  der(x) = ;\nend S;\n",
    );
    let o = globe(&["check", &bad, "-m", "S", "--frontend", "native"]);
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    assert!(
        out(&o).contains("s.mo:4:12: error[GM-E0001]"),
        "{}",
        out(&o)
    );

    let unres = temp_model("n.mo", "model N\n  Foo x;\nend N;\n");
    let o = globe(&["check", &unres, "-m", "N", "--frontend", "native"]);
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    assert!(out(&o).contains("n.mo:2:3: error[GM-E1001]"), "{}", out(&o));
}

#[test]
fn unsupported_is_reported_not_switched_to_rumoca() {
    let m = temp_model(
        "e.mo",
        "expandable connector EC\n  Real a;\nend EC;\nmodel E\n  EC ec;\nend E;\n",
    );
    let o = globe(&["check", &m, "-m", "E", "--frontend", "native"]);
    assert_eq!(o.status.code(), Some(3), "{}", out(&o));
    assert!(out(&o).contains("unsupported[GM-U0006]"), "{}", out(&o));
    assert!(
        out(&o).contains("native では未対応: expandable"),
        "{}",
        out(&o)
    );
}

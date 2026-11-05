//! 診断の重複（`--force`）と 1 行の要約 `summary`（S184）。どの面も facade の同じ応答を見る。

use globe_mode::{SimulateRequest, Workspace};

const DECAY: &str = "model Decay Real x(start = 1, fixed = true); equation der(x) = -x; end Decay;";
const UNBALANCED: &str =
    "model Unbalanced Real x(start = 1); Real y; equation der(x) = -x; end Unbalanced;";
const OVER: &str = "model Over Real x(start=1, fixed=true); initial equation x = 1; equation der(x) = -x; end Over;";
const DIV: &str = "model Div parameter Real p = 1 / 0; Real x(start = 1, fixed = true); equation der(x) = -x; end Div;";

fn workspace() -> Workspace {
    let mut ws = Workspace::new();
    for (name, src) in [
        ("Decay", DECAY),
        ("Unbalanced", UNBALANCED),
        ("Over", OVER),
        ("Div", DIV),
    ] {
        ws.set_file(&format!("{name}.mo"), src);
    }
    ws
}

fn run(ws: &mut Workspace, model: &str, force: bool) -> globe_mode::SimulateResponse {
    ws.simulate(&SimulateRequest {
        model: model.into(),
        force,
        no_csv: true,
        ..Default::default()
    })
}

/// 強行でコンパイルに失敗しても、同じコード・位置・文面の診断は 1 つだけ
#[test]
fn forced_simulate_does_not_repeat_diagnostics() {
    let mut ws = workspace();
    let r = run(&mut ws, "Unbalanced", true);
    assert!(!r.ok);
    let n = |code: &str| r.diagnostics.iter().filter(|d| d.code == code).count();
    assert_eq!(n("GM-E5002"), 1, "{:?}", r.diagnostics);
    assert!(
        r.refused
            .as_deref()
            .unwrap()
            .contains("DAE まで変換できない")
    );
    // 強行して実行に進んだ結果にも重複は無い
    let r = run(&mut ws, "Over", true);
    assert!(r.final_values.is_some() && r.summary.is_none());
    let keys: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code, &d.file, d.range, &d.message))
        .collect();
    for (i, k) in keys.iter().enumerate() {
        assert!(!keys[..i].contains(k), "{keys:?}");
    }
}

#[test]
fn check_summary_lines() {
    let mut ws = workspace();
    let ok = ws.check("Decay");
    assert_eq!(ok.summary, "Decay: OK（S0〜S10 を通過）");
    let ng = ws.check("Unbalanced");
    assert!(
        ng.summary.starts_with("Unbalanced: NG（S5 "),
        "{}",
        ng.summary
    );
    // 検査に入る前の失敗も同じ文面
    let early = globe_mode::CheckResponse::failed("M", Vec::new());
    assert_eq!(
        early.summary,
        format!("M: NG（S1 {} で停止）", globe_mode::Stage::S1.title())
    );
}

#[test]
fn simulate_summary_names_the_first_error() {
    let mut ws = workspace();
    assert!(run(&mut ws, "Decay", false).summary.is_none());
    let s = run(&mut ws, "Unbalanced", false).summary.unwrap();
    assert!(
        s.starts_with("Unbalanced: 実行しない（S5 ") && s.contains("。最初の error: GM-E5002 "),
        "{s}"
    );
    // 強行しても DAE にできない
    let s = run(&mut ws, "Unbalanced", true).summary.unwrap();
    assert!(
        s.contains("DAE まで変換できないので実行できない。最初の error: GM-E5002 "),
        "{s}"
    );
    let r = run(&mut ws, "Over", false);
    assert!(r.can_force && r.summary.unwrap().contains("force で強行できる"));
    // GM-U だけ
    let s = run(&mut ws, "Div", false).summary.unwrap();
    assert!(s.contains("。最初の未対応: GM-U"), "{s}");
}

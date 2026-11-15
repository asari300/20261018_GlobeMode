//! S313: `Workspace::check_with` で検査のフロントエンドを選ぶ。既定の `check` は Rumoca のまま。
//! 自前の段が断る形（expandable）は、feature `native` の有無にかかわらず `GM-U0006`（Rumoca には切り替えない）。

use globe_mode::{CheckFrontend, Workspace};

const EXPANDABLE: &str = "expandable connector EC\n  Real a;\nend EC;\nmodel E\n  EC ec;\nend E;\n";
const DECAY: &str = "model Decay Real x(start = 1, fixed = true); equation der(x) = -x; end Decay;";

#[test]
fn native_is_chosen_per_call_and_unsupported_is_not_switched() {
    let mut ws = Workspace::new();
    ws.set_file("E.mo", EXPANDABLE);
    ws.set_file("Decay.mo", DECAY);
    let native = ws.check_with("E", CheckFrontend::Native);
    assert!(!native.passed);
    assert!(
        native.diagnostics.iter().any(|d| d.code == "GM-U0006"),
        "{native:?}"
    );
    // 既定（Rumoca）は GM-U0006 を出さない
    assert!(
        !ws.check("E")
            .diagnostics
            .iter()
            .any(|d| d.code == "GM-U0006")
    );
    assert_eq!("native".parse(), Ok(CheckFrontend::Native));
    assert!("omc".parse::<CheckFrontend>().is_err());
    #[cfg(feature = "native")]
    {
        let ok = ws.check_with("Decay", CheckFrontend::Native);
        assert!(ok.passed, "{ok:?}");
        assert!(ok.summary.contains("native: S0〜S6"), "{}", ok.summary);
    }
}

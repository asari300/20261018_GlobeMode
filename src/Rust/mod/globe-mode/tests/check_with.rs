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

/// S347: ファイルの組で読んだライブラリ（wasm の MSL）は native から見えない。名前が引けない診断に、その旨の注が付く（Rumoca は通る）。
#[cfg(feature = "native")]
#[test]
fn native_explains_a_library_it_cannot_see() {
    let lib = std::collections::BTreeMap::from([(
        "Lib/package.mo".to_string(),
        "package Lib\n  model R\n    Real x;\n  equation\n    x = 1;\n  end R;\nend Lib;\n"
            .to_string(),
    )]);
    let mut ws = Workspace::with_library_sources("Lib", &lib).unwrap();
    ws.set_file("U.mo", "model U\n  Lib.R r;\nend U;\n");
    assert!(ws.check("U").passed);
    let native = ws.check_with("U", CheckFrontend::Native);
    let d = native
        .diagnostics
        .iter()
        .find(|d| d.code == "GM-E1001")
        .expect("名前が引けない");
    assert!(
        d.notes.iter().any(|n| n.contains("Rumoca で確かめる")),
        "{d:?}"
    );
    // 自分のファイルだけで済むモデルには注を付けない
    ws.set_file("D.mo", DECAY);
    assert!(ws.check_with("Decay", CheckFrontend::Native).passed);
}

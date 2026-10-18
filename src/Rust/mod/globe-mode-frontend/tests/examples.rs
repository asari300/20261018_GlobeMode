//! src/Modelica のサンプルで、Rumoca アダプタの outline と check を確かめる。

use std::path::PathBuf;

use globe_mode_core::{Stage, codes};
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_ir::ClassKind;

fn modelica(rel: &str) -> (String, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../Modelica")
        .join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    (rel.to_string(), text)
}

const PACKAGE: &str = "GlobeModeExamples/package.mo";

#[test]
fn outline_lists_classes_components_and_connects() {
    let (name, text) = modelica(PACKAGE);
    let o = RumocaFrontend::new()
        .outline(Source {
            name: &name,
            text: &text,
        })
        .expect("parses");
    let classes = o.qualified_classes();
    let names: Vec<&str> = classes.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"GlobeModeExamples.RC"));
    assert!(names.contains(&"GlobeModeExamples.Pin"));

    let pin = classes
        .iter()
        .find(|(n, _)| n == "GlobeModeExamples.Pin")
        .unwrap()
        .1;
    assert_eq!(pin.kind, ClassKind::Connector);
    let two_pin = classes
        .iter()
        .find(|(n, _)| n == "GlobeModeExamples.TwoPin")
        .unwrap()
        .1;
    assert!(two_pin.partial);

    let rc = classes
        .iter()
        .find(|(n, _)| n == "GlobeModeExamples.RC")
        .unwrap()
        .1;
    assert_eq!(rc.components.len(), 4);
    let connects: Vec<(&str, &str)> = rc
        .connects
        .iter()
        .map(|c| (c.lhs.as_str(), c.rhs.as_str()))
        .collect();
    assert_eq!(
        connects,
        [
            ("src.p", "r.p"),
            ("r.n", "c.p"),
            ("c.n", "src.n"),
            ("src.n", "gnd.p")
        ]
    );
    // 範囲はソースの中を指す
    assert!(text[rc.connects[0].range.start as usize..].starts_with("src.p"));

    let resistor = classes
        .iter()
        .find(|(n, _)| n == "GlobeModeExamples.Resistor")
        .unwrap()
        .1;
    assert_eq!(resistor.extends, ["TwoPin"]);
}

#[test]
fn rc_passes_with_balanced_counts() {
    let (name, text) = modelica(PACKAGE);
    let r = RumocaFrontend::new().check(
        &[Source {
            name: &name,
            text: &text,
        }],
        &[],
        "GlobeModeExamples.RC",
    );
    assert!(r.passed(), "{:?}", r.diagnostics);
    let b = r.balance.expect("balance");
    assert_eq!((b.equations, b.unknowns, b.states), (20, 20, 1));
}

#[test]
fn unbalanced_stops_at_s5() {
    let (name, text) = modelica(PACKAGE);
    let r = RumocaFrontend::new().check(
        &[Source {
            name: &name,
            text: &text,
        }],
        &[],
        "GlobeModeExamples.Unbalanced",
    );
    assert_eq!(r.failed_stage(), Some(Stage::S5));
    assert_eq!(r.diagnostics[0].code, codes::UNBALANCED);
}

#[test]
fn syntax_error_stops_at_s0_with_range() {
    let (name, text) = modelica("SyntaxError.mo");
    let r = RumocaFrontend::new().check(
        &[Source {
            name: &name,
            text: &text,
        }],
        &[],
        "Broken",
    );
    assert_eq!(r.failed_stage(), Some(Stage::S0));
    let d = &r.diagnostics[0];
    assert_eq!(d.code, codes::SYNTAX);
    assert_eq!(d.file.as_deref(), Some("SyntaxError.mo"));
    assert!(d.range.is_some());

    let outline = RumocaFrontend::new().outline(Source {
        name: &name,
        text: &text,
    });
    assert!(outline.is_err());
}

#[test]
fn unknown_model_is_reported_with_candidates() {
    let (name, text) = modelica(PACKAGE);
    let r = RumocaFrontend::new().check(
        &[Source {
            name: &name,
            text: &text,
        }],
        &[],
        "GlobeModeExamples.Nope",
    );
    assert_eq!(r.diagnostics[0].code, codes::MODEL_NOT_FOUND);
    assert!(r.diagnostics[0].notes[0].contains("GlobeModeExamples.RC"));
}

/// サンプルのパッケージを一時ディレクトリにコピーし、ライブラリとして読ませる（MSL の代わり）。
fn temp_library(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("globe-mode-lib-{tag}-{}", std::process::id()));
    let pkg = root.join("GlobeModeExamples");
    std::fs::create_dir_all(&pkg).unwrap();
    let (_, text) = modelica(PACKAGE);
    std::fs::write(pkg.join("package.mo"), text).unwrap();
    root
}

#[test]
fn model_inside_a_library_can_be_checked_without_sources() {
    let lib = temp_library("ok");
    let r = RumocaFrontend::new().check(&[], &[lib.as_path()], "GlobeModeExamples.RC");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let d = r.dae.expect("dae counts");
    assert_eq!(
        (d.states, d.algebraics, d.outputs, d.continuous_equations),
        (1, 19, 0, 20)
    );
    let _ = std::fs::remove_dir_all(lib);
}

#[test]
fn missing_library_is_reported_at_s1() {
    let missing = std::env::temp_dir().join("globe-mode-no-such-library");
    let r = RumocaFrontend::new().check(&[], &[missing.as_path()], "Anything");
    assert_eq!(r.failed_stage(), Some(Stage::S1));
    assert_eq!(r.diagnostics[0].code, codes::LIBRARY);
}

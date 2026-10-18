//! src/Modelica のサンプルで、Rumoca アダプタの outline と check を確かめる。

use std::path::PathBuf;

use globe_mode_core::{Stage, codes};
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_ir::ClassKind;

fn modelica(rel: &str) -> (String, String) {
    // わざと誤ったファイルは src/Modelica（ほかの IDE でも開くライブラリ）に置かず、tests/data に置く
    let base = if rel == "SyntaxError.mo" {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica")
    };
    let path = base.join(rel);
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

/// GlobeModeValidation の .mo をすべて文書として読む（ワークスペースとして扱う）。
fn validation_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "mo") {
                out.push(p);
            }
        }
    }
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            (
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

fn check_validation(model: &str) -> globe_mode_frontend::CheckReport {
    let files = validation_sources();
    let sources: Vec<Source<'_>> = files
        .iter()
        .map(|(n, t)| Source { name: n, text: t })
        .collect();
    RumocaFrontend::new().check(&sources, &[], &format!("GlobeModeValidation.{model}"))
}

#[test]
fn validation_models_without_msl_pass() {
    // (モデル, 状態, 離散, スカラーの方程式数)。手で数えた値と一致することを確かめてある（S004）
    for (model, states, discrete, equations) in [
        ("Connect.RealCircuit", 1, 0, 20),
        ("Connect.RealSignals", 0, 0, 5),
        ("Connect.BooleanSignals", 0, 4, 7),
        ("Connect.Hierarchical", 2, 0, 40),
    ] {
        let r = check_validation(model);
        assert!(r.passed(), "{model}: {:?}", r.diagnostics);
        let b = r.balance.unwrap();
        assert_eq!(
            (b.states, b.discrete, b.equations),
            (states, discrete, equations),
            "{model}"
        );
        assert!(b.is_balanced(), "{model}");
    }
}

#[test]
fn signal_type_mismatch_stops_at_s4() {
    let r = check_validation("Invalid.SignalTypeMismatch");
    assert_eq!(r.failed_stage(), Some(Stage::S4));
    assert_eq!(r.diagnostics[0].code, codes::FLATTEN);
}

#[test]
fn medium_model_needs_msl() {
    // Modelica.Media は MSL にある。MSL 無しでは名前解決（S1）で止まる。MSL ありの検査は CI の msl ジョブで行う
    let r = check_validation("Connect.MediumFlow");
    assert_eq!(r.failed_stage(), Some(Stage::S1));
}

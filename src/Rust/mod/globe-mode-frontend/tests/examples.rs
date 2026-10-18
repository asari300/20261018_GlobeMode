//! src/Modelica のサンプルで、Rumoca アダプタの outline と check を確かめる。

use std::path::PathBuf;

use globe_mode_core::{Stage, codes};
use globe_mode_frontend::{Checker, Frontend, RumocaFrontend, Source};
use globe_mode_ir::{ClassKind, EquationOrigin, UnknownKind};

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

// --------------------------------------------------------------------------- Checker（Session の使い回し）

#[test]
fn one_checker_checks_two_models_in_a_row() {
    let (name, text) = modelica(PACKAGE);
    let sources = [Source {
        name: &name,
        text: &text,
    }];
    let checker = Checker::new(&[]).expect("ライブラリ無し");
    let rc = checker.check(&sources, "GlobeModeExamples.RC");
    assert!(rc.passed(), "{:?}", rc.diagnostics);
    let unbalanced = checker.check(&sources, "GlobeModeExamples.Unbalanced");
    assert_eq!(unbalanced.failed_stage(), Some(Stage::S5));
    // 同じモデルをもう一度（Session のキャッシュから）
    let again = checker.check(&sources, "GlobeModeExamples.RC");
    assert_eq!(again, rc);
}

#[test]
fn recheck_after_replacing_a_document_reflects_the_edit() {
    let (name, text) = modelica(PACKAGE);
    let checker = Checker::new(&[]).unwrap();
    let before = checker.check(
        &[Source {
            name: &name,
            text: &text,
        }],
        "GlobeModeExamples.Unbalanced",
    );
    assert_eq!(before.failed_stage(), Some(Stage::S5));

    // y の方程式を足すと、数が合う
    let fixed = text.replace("    der(x) = -x;\n", "    der(x) = -x;\n    y = 2 * x;\n");
    assert_ne!(fixed, text);
    let after = checker.check(
        &[Source {
            name: &name,
            text: &fixed,
        }],
        "GlobeModeExamples.Unbalanced",
    );
    assert!(after.passed(), "{:?}", after.diagnostics);
    assert_eq!(after.balance.unwrap().equations, 2);
}

#[test]
fn documents_not_passed_again_are_removed() {
    let (name, text) = modelica(PACKAGE);
    let checker = Checker::new(&[]).unwrap();
    let extra = "model Extra\n  Real x;\nequation\n  x = 1;\nend Extra;\n";
    let both = [
        Source {
            name: &name,
            text: &text,
        },
        Source {
            name: "Extra.mo",
            text: extra,
        },
    ];
    assert!(checker.check(&both, "Extra").passed());
    // Extra.mo を渡さなければ、Extra は見つからない
    let r = checker.check(&both[..1], "Extra");
    assert_eq!(r.diagnostics[0].code, codes::MODEL_NOT_FOUND);
}

#[test]
fn rc_structure_is_square_and_matches_rumoca() {
    let (name, text) = modelica(PACKAGE);
    let sources = [Source {
        name: &name,
        text: &text,
    }];
    let checker = Checker::new(&[]).unwrap();
    let r = checker.check(&sources, "GlobeModeExamples.RC");
    let s = r.structure.as_ref().expect("structure");
    assert_eq!((s.unknowns.len(), s.equations.len()), (20, 20));
    assert!(s.incidence.iter().all(|row| !row.is_empty()));
    assert_eq!(
        s.unknowns
            .iter()
            .filter(|u| u.kind == UnknownKind::Derivative)
            .count(),
        1
    );
    // 位置はソースの中（方程式 `v = R * i` は Resistor の中）
    let e = s
        .equations
        .iter()
        .find(|e| e.label == "v = R * i")
        .expect("Resistor の方程式");
    assert_eq!(e.origin, EquationOrigin::Source);
    let range = e.range.unwrap();
    assert_eq!(&text[range.start as usize..range.end as usize], "v = R * i");

    // Rumoca 自身の構造解析と同じ大きさ
    let reference = checker
        .reference_structure(&sources, "GlobeModeExamples.RC")
        .expect("reference");
    assert_eq!(
        (
            reference.n_equations,
            reference.n_unknowns,
            reference.matching
        ),
        (20, 20, 20)
    );
    // flat の要約
    let flat = r.flat.as_ref().expect("flat");
    assert!(!flat.partial);
    assert!(flat.equations_by_component.iter().any(|(c, _)| c == "r"));
}

#[test]
fn hierarchical_structure_is_40_by_40() {
    let r = check_validation("Connect.Hierarchical");
    let s = r.structure.expect("structure");
    assert_eq!((s.unknowns.len(), s.equations.len()), (40, 40));
    assert!(s.incidence.iter().all(|row| !row.is_empty()));
    let ders: Vec<&str> = s
        .unknowns
        .iter()
        .filter(|u| u.kind == UnknownKind::Derivative)
        .map(|u| u.name.as_str())
        .collect();
    assert_eq!(ders.len(), 2, "{ders:?}");
}

#[test]
fn medium_model_without_msl_has_no_structure() {
    let r = check_validation("Connect.MediumFlow");
    assert_eq!(r.failed_stage(), Some(Stage::S1));
    assert!(r.structure.is_none());
}

#[test]
fn unbalanced_reports_equations_by_component() {
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
    let flat = r.flat.as_ref().expect("ToDae で失敗しても flat は取れる");
    assert_eq!(flat.equations, 1);
    assert_eq!(flat.equations_by_component, [(String::new(), 1)]);
    assert!(
        r.diagnostics[0]
            .notes
            .iter()
            .any(|n| n.contains("（トップレベル） 1")),
        "{:?}",
        r.diagnostics[0].notes
    );
    assert!(r.structure.is_none());
}

#[test]
fn checker_can_be_shared_and_unwound() {
    fn shared<T: Send + Sync + std::panic::RefUnwindSafe>() {}
    shared::<Checker>();
    let checker = Checker::new(&[]).unwrap();
    // panic を catch_unwind で受けても、次の検査は使える（Session はライブラリを読んだ直後に戻る）
    let caught = std::panic::catch_unwind(|| {
        let _ = checker.check(&[], "Nope");
        panic!("わざと");
    });
    assert!(caught.is_err());
    let (name, text) = modelica(PACKAGE);
    let r = checker.check(
        &[Source {
            name: &name,
            text: &text,
        }],
        "GlobeModeExamples.RC",
    );
    assert!(r.passed(), "{:?}", r.diagnostics);
}

#[test]
fn compile_failures_point_into_the_source() {
    // S4 の誤り（Real の出力を Boolean の入力につないだ）の位置は、そのモデルのファイルの中
    let r = check_validation("Invalid.SignalTypeMismatch");
    let d = &r.diagnostics[0];
    let file = d.file.as_deref().expect("file");
    assert!(file.ends_with("SignalTypeMismatch.mo"), "{file}");
    let range = d.range.expect("range");
    let text = std::fs::read_to_string(file).unwrap();
    // connect 文の行（6 行目）
    assert_eq!(
        globe_mode_core::line_col(&text, range.start).line,
        6,
        "{d:?}"
    );
}

//! S8 の入力（`CheckReport::init`、初期化系の構造）を、src/Modelica/GlobeModeValidation のモデルで確かめる（S006）。
//!
//! ここでは `InitStructure` の中身（状態の並び、fixed / start / 束縛、初期方程式の数と位置）だけを見る。
//! 診断コード（GM-E8001 など）の照合は globe-mode-analysis の単体テストと、db のテストで行う
//! （frontend を analysis に依存させないため）。

use std::path::{Path, PathBuf};

use globe_mode_frontend::{CheckReport, Checker, Source};
use globe_mode_ir::{InitStructure, InitVariable, UnknownKind};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
}

/// パッケージの全 `.mo`（名前、本文）
fn package_files() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    collect(&package_dir(), &mut paths);
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

struct Package {
    files: Vec<(String, String)>,
    checker: Checker,
}

impl Package {
    fn new() -> Self {
        Self {
            files: package_files(),
            checker: Checker::new(&[]).unwrap(),
        }
    }

    fn check(&self, model: &str) -> CheckReport {
        let sources: Vec<Source<'_>> = self
            .files
            .iter()
            .map(|(n, t)| Source { name: n, text: t })
            .collect();
        let model = format!("GlobeModeValidation.{model}");
        let report = self.checker.check(&sources, &model);
        assert!(
            report.passed(),
            "{model}: {:?} {:?}",
            report.diagnostics,
            report.notes
        );
        report
    }

    fn init(&self, model: &str) -> InitStructure {
        let report = self.check(model);
        report
            .init
            .unwrap_or_else(|| panic!("{model}: init が無い: {:?}", report.notes))
    }

    /// 位置が指す本文の一部
    fn text(&self, file: Option<&str>, range: Option<globe_mode_core::TextRange>) -> &str {
        let file = file.expect("ファイル名");
        let range = range.expect("範囲");
        let (_, text) = self
            .files
            .iter()
            .find(|(n, _)| n == file)
            .unwrap_or_else(|| panic!("{file} が無い"));
        &text[range.start as usize..range.end as usize]
    }
}

/// (名前, fixed, has_start, has_binding)
fn flags(vars: &[InitVariable]) -> Vec<(&str, bool, bool, bool)> {
    vars.iter()
        .map(|v| (v.name.as_str(), v.fixed, v.has_start, v.has_binding))
        .collect()
}

fn labels(init: &InitStructure) -> Vec<&str> {
    init.initial_equations
        .iter()
        .map(|e| e.label.as_str())
        .collect()
}

#[test]
fn balanced_has_one_fixed_state_and_one_initial_equation() {
    let p = Package::new();
    let init = p.init("Init.Balanced");
    assert_eq!(
        flags(&init.states),
        [("x", true, true, false), ("v", false, false, false)]
    );
    assert_eq!(
        flags(&init.parameters),
        [
            ("m", true, true, true),
            ("k", true, true, true),
            ("d", true, true, true)
        ]
    );
    assert!(init.discretes.is_empty());
    assert!(init.algebraics.is_empty());
    assert_eq!(labels(&init), ["v = 0"]);
    let e = &init.initial_equations[0];
    assert!(e.file.as_deref().unwrap().ends_with("Balanced.mo"));
    assert_eq!(p.text(e.file.as_deref(), e.range), "v = 0");
    // 状態の位置は宣言（型と名前）
    let x = &init.states[0];
    assert_eq!(p.text(x.file.as_deref(), x.range), "Real x");
}

#[test]
fn overdetermined_counts_fixed_start_once() {
    // fixed = true の start は初期方程式にならない（初期方程式は書いた x = 2 の 1 本だけ）
    let p = Package::new();
    let init = p.init("Init.OverDetermined");
    assert_eq!(flags(&init.states), [("x", true, true, false)]);
    assert_eq!(labels(&init), ["x = 2"]);
    let e = &init.initial_equations[0];
    assert_eq!(p.text(e.file.as_deref(), e.range), "x = 2");
}

#[test]
fn fixed_start_alone_adds_no_initial_equation() {
    // initial equation を書かず fixed = true だけのモデルでは、初期方程式は 0 本
    let p = Package::new();
    for model in ["Init.ParameterWithoutValue", "Invalid.HighIndexDuplicate"] {
        let init = p.init(model);
        assert!(
            init.initial_equations.is_empty(),
            "{model}: {:?}",
            labels(&init)
        );
        assert!(init.states.iter().all(|s| s.fixed), "{model}");
        assert_eq!(init.states.len(), 1, "{model}");
    }
}

#[test]
fn underdetermined_has_no_initial_conditions_and_one_state_without_start() {
    let p = Package::new();
    let init = p.init("Init.UnderDetermined");
    assert_eq!(
        flags(&init.states),
        [("h1", false, true, false), ("h2", false, false, false)]
    );
    assert!(init.initial_equations.is_empty());
    let h2 = &init.states[1];
    assert_eq!(p.text(h2.file.as_deref(), h2.range), "Real h2");
}

#[test]
fn parameter_without_value_has_neither_binding_nor_start() {
    let p = Package::new();
    let init = p.init("Init.ParameterWithoutValue");
    assert_eq!(flags(&init.parameters), [("T", true, false, false)]);
    assert_eq!(flags(&init.states), [("x", true, true, false)]);
    let t = &init.parameters[0];
    assert_eq!(p.text(t.file.as_deref(), t.range), "Real T");
}

#[test]
fn free_parameter_is_fixed_false_and_set_by_initial_equation() {
    let p = Package::new();
    let init = p.init("Init.FreeParameter");
    assert_eq!(
        flags(&init.parameters),
        [("T", true, true, true), ("u0", false, false, false)]
    );
    assert_eq!(flags(&init.states), [("x", true, true, false)]);
    assert_eq!(labels(&init), ["der(x) = 0"]);
}

#[test]
fn states_follow_the_derivative_unknowns_of_the_structure() {
    // InitStructure.states の並びは、DaeStructure の Derivative の未知数（der(x) の x）と同じ
    let p = Package::new();
    for model in [
        "Connect.RealCircuit",
        "Connect.Hierarchical",
        "Connect.BooleanSignals",
        "Invalid.HighIndexDuplicate",
        "Init.Balanced",
        "Init.OverDetermined",
        "Init.UnderDetermined",
        "Init.ParameterWithoutValue",
        "Init.FreeParameter",
    ] {
        let report = p.check(model);
        let structure = report.structure.as_ref().expect(model);
        let init = report.init.as_ref().expect(model);
        let ders: Vec<String> = structure
            .unknowns
            .iter()
            .filter(|u| u.kind == UnknownKind::Derivative)
            .map(|u| u.name.clone())
            .collect();
        let states: Vec<String> = init
            .states
            .iter()
            .map(|s| format!("der({})", s.name))
            .collect();
        assert_eq!(states, ders, "{model}");
    }
}

#[test]
fn real_circuit_has_one_state_without_start() {
    // RC 回路のコンデンサの電圧は start も fixed も無い（S8 では初期条件が 1 つ足りない警告になる）
    let p = Package::new();
    let init = p.init("Connect.RealCircuit");
    assert_eq!(flags(&init.states), [("capacitor.v", false, false, false)]);
    assert!(init.initial_equations.is_empty());
    assert!(init.parameters.iter().all(|v| v.fixed && v.has_binding));
    // 代数変数（DaeStructure の Algebraic / Output の未知数と同じ数）。fixed は無い
    let report = p.check("Connect.RealCircuit");
    let structure = report.structure.as_ref().unwrap();
    let algebraic_unknowns = structure
        .unknowns
        .iter()
        .filter(|u| u.kind != UnknownKind::Derivative)
        .count();
    assert_eq!(init.algebraics.len(), algebraic_unknowns);
    assert_eq!(init.algebraics.len(), 19);
    assert!(init.algebraics.iter().all(|v| !v.fixed && !v.has_binding));
    assert!(init.discretes.is_empty());
    // 宣言の位置は、部品（TwoPin）の宣言を指す
    let v = &init.states[0];
    assert!(v.file.as_deref().unwrap().ends_with("Components.mo"));
    assert_eq!(p.text(v.file.as_deref(), v.range), "Real v");
}

#[test]
fn boolean_signals_list_discretes() {
    // Boolean の信号（BooleanStep と BooleanNot の出力）は離散変数として並ぶ
    let p = Package::new();
    let init = p.init("Connect.BooleanSignals");
    assert!(!init.discretes.is_empty());
    assert!(init.discretes.iter().all(|v| !v.fixed));
    assert!(init.states.is_empty());
}

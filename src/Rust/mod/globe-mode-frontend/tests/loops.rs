//! S9 の入力（`DaeStructure::nonlinear`、方程式ごとの非線形の参照）を、
//! src/Modelica/GlobeModeValidation/Loops のモデルで確かめる（S010）。
//!
//! 代数ループの大きさは Rumoca の構造解析（`Checker::reference_structure` の `loops`）と照らす。
//! 診断コード（GM-I9001・GM-W9001）の照合は globe-mode-analysis の単体テストで行う
//! （frontend を analysis に依存させないため）。

use std::path::{Path, PathBuf};

use globe_mode_frontend::{Checker, ReferenceStructure, Source};
use globe_mode_ir::DaeStructure;

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

    fn sources(&self) -> Vec<Source<'_>> {
        self.files
            .iter()
            .map(|(n, t)| Source { name: n, text: t })
            .collect()
    }

    fn structure(&self, model: &str) -> DaeStructure {
        let model = format!("GlobeModeValidation.Loops.{model}");
        let report = self.checker.check(&self.sources(), &model);
        assert!(
            report.passed(),
            "{model}: {:?} {:?}",
            report.diagnostics,
            report.notes
        );
        report
            .structure
            .unwrap_or_else(|| panic!("{model}: structure が無い: {:?}", report.notes))
    }

    fn reference(&self, model: &str) -> ReferenceStructure {
        let model = format!("GlobeModeValidation.Loops.{model}");
        self.checker
            .reference_structure(&self.sources(), &model)
            .unwrap_or_else(|| panic!("{model}: Rumoca の構造解析が無い"))
    }
}

/// 未知数の添字（名前から）
fn unknown(s: &DaeStructure, name: &str) -> u32 {
    s.unknowns
        .iter()
        .position(|u| u.name == name)
        .unwrap_or_else(|| panic!("未知数 {name} が無い: {:?}", s.unknowns)) as u32
}

/// ラベルが `label` の方程式の、非線形の参照
fn nonlinear_of<'a>(s: &'a DaeStructure, label: &str) -> &'a [u32] {
    let e = s
        .equations
        .iter()
        .position(|e| e.label == label)
        .unwrap_or_else(|| panic!("方程式 {label} が無い: {:?}", s.equations));
    &s.nonlinear[e]
}

/// 形の約束: 長さは方程式の数、各行は昇順・重複なしで incidence の部分集合
fn assert_well_formed(s: &DaeStructure) {
    assert_eq!(s.nonlinear.len(), s.equations.len());
    for (row, nonlinear) in s.incidence.iter().zip(&s.nonlinear) {
        assert!(nonlinear.windows(2).all(|w| w[0] < w[1]), "昇順・重複なし");
        assert!(
            nonlinear.iter().all(|u| row.contains(u)),
            "incidence の部分集合"
        );
    }
}

#[test]
fn linear_loop_has_no_nonlinear_references() {
    let p = Package::new();
    let s = p.structure("LinearLoop");
    assert_well_formed(&s);
    assert_eq!(s.equations.len(), 2);
    assert!(s.nonlinear.iter().all(Vec::is_empty), "{:?}", s.nonlinear);
    assert!(s.incidence.iter().all(|r| r.len() == 2));
    assert_eq!(p.reference("LinearLoop").loops, [2]);
}

#[test]
fn nonlinear_loop_marks_both_factors_of_the_product() {
    let p = Package::new();
    for model in ["NonlinearLoopWithStart", "NonlinearLoopWithoutStart"] {
        let s = p.structure(model);
        assert_well_formed(&s);
        let (x, y) = (unknown(&s, "x"), unknown(&s, "y"));
        let mut both = vec![x, y];
        both.sort_unstable();
        assert_eq!(nonlinear_of(&s, "x * y = 1"), both, "{model}");
        assert!(nonlinear_of(&s, "x + y = 3").is_empty(), "{model}");
        assert_eq!(p.reference(model).loops, [2], "{model}");
    }
}

#[test]
fn model_without_loops_keeps_nonlinear_rows_outside_loops() {
    let p = Package::new();
    let s = p.structure("NoLoop");
    assert_well_formed(&s);
    let y = unknown(&s, "y");
    assert_eq!(nonlinear_of(&s, "z = y ^ 2"), [y]);
    assert!(nonlinear_of(&s, "y = 2 * x").is_empty());
    assert!(nonlinear_of(&s, "der(x) = -x").is_empty());
    assert!(p.reference("NoLoop").loops.is_empty());
}

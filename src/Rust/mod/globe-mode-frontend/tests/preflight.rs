//! S10 の入力（`CheckReport::preflight`、パラメータと start 値の評価、experiment annotation）を、
//! src/Modelica/GlobeModeValidation/Preflight のモデルで確かめる（S010）。
//!
//! ここでは `PreflightStructure` の中身（評価した数、失敗の名前・種類・位置、experiment の値）だけを見る。
//! 診断コード（GM-E10001 など）の照合は globe-mode-analysis の単体テストで行う
//! （frontend を analysis に依存させないため）。

use std::path::{Path, PathBuf};

use globe_mode_frontend::{CheckReport, Checker, Source};
use globe_mode_ir::{EvaluationFailure, Experiment, FailureKind, PreflightStructure};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn msl_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl")
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
        let model = format!("GlobeModeValidation.Preflight.{model}");
        let report = self.checker.check(&sources, &model);
        assert!(
            report.passed(),
            "{model}: {:?} {:?}",
            report.diagnostics,
            report.notes
        );
        report
    }

    fn preflight(&self, model: &str) -> PreflightStructure {
        let report = self.check(model);
        report
            .preflight
            .unwrap_or_else(|| panic!("{model}: preflight が無い: {:?}", report.notes))
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

/// (名前, 種類)
fn failures(f: &[EvaluationFailure]) -> Vec<(&str, FailureKind)> {
    f.iter().map(|f| (f.name.as_str(), f.kind)).collect()
}

fn values(e: &Experiment) -> [Option<f64>; 4] {
    [e.start_time, e.stop_time, e.interval, e.tolerance].map(|v| v.map(|b| b.get()))
}

#[test]
fn valid_model_evaluates_everything() {
    let p = Package::new().preflight("Valid");
    assert_eq!(p.parameters_evaluated, 3, "tau, T, k");
    assert_eq!(p.starts_evaluated, 2, "x, y");
    assert!(
        p.parameter_failures.is_empty(),
        "{:?}",
        p.parameter_failures
    );
    assert!(p.start_failures.is_empty(), "{:?}", p.start_failures);
    let e = p.experiment.expect("experiment");
    assert_eq!(values(&e), [Some(0.0), Some(2.0), Some(0.01), Some(1e-6)]);
}

#[test]
fn not_finite_parameter_is_reported_at_its_declaration() {
    let pkg = Package::new();
    let p = pkg.preflight("NotFiniteParameter");
    assert_eq!(p.parameters_evaluated, 3);
    // rate = 2 * r は r の失敗を受け継いだだけなので、r の 1 件
    assert_eq!(
        failures(&p.parameter_failures),
        [("r", FailureKind::NotFinite)]
    );
    let f = &p.parameter_failures[0];
    assert!(
        f.file
            .as_deref()
            .unwrap()
            .ends_with("NotFiniteParameter.mo")
    );
    assert_eq!(pkg.text(f.file.as_deref(), f.range), "Real r");
    assert!(f.message.contains("`sqrt(area)`"), "{}", f.message);
}

#[test]
fn not_finite_start_is_reported() {
    let pkg = Package::new();
    let p = pkg.preflight("StartNotFinite");
    assert!(
        p.parameter_failures.is_empty(),
        "{:?}",
        p.parameter_failures
    );
    assert_eq!(failures(&p.start_failures), [("x", FailureKind::NotFinite)]);
    let f = &p.start_failures[0];
    assert_eq!(pkg.text(f.file.as_deref(), f.range), "Real x");
}

#[test]
fn function_assertion_is_reported() {
    let p = Package::new().preflight("FunctionAssertion");
    assert_eq!(
        failures(&p.parameter_failures),
        [("d", FailureKind::Assertion)]
    );
    let m = &p.parameter_failures[0].message;
    assert!(m.contains("assert(v >= 0"), "{m}");
}

#[test]
fn external_function_parameter_is_not_static() {
    let p = Package::new().preflight("ExternalParameter");
    assert_eq!(
        failures(&p.parameter_failures),
        [("g", FailureKind::External)]
    );
}

#[test]
fn free_parameter_and_its_dependent_are_not_evaluated() {
    let p = Package::new().preflight("FreeParameterDependency");
    assert_eq!(
        p.parameters_evaluated, 1,
        "T だけ（u0 と k は初期化で決まる）"
    );
    assert!(
        p.parameter_failures.is_empty(),
        "{:?}",
        p.parameter_failures
    );
    assert!(p.start_failures.is_empty(), "{:?}", p.start_failures);
}

#[test]
fn missing_experiment_is_none() {
    let p = Package::new().preflight("NoExperiment");
    assert_eq!(p.experiment, None);
}

/// experiment の生の値（Rumoca が捨てる値も残る）。
#[test]
fn experiment_annotations_keep_raw_values() {
    let pkg = Package::new();
    let cases: [(&str, [Option<f64>; 4]); 3] = [
        ("StopBeforeStart", [Some(1.0), Some(0.5), None, None]),
        (
            "InvalidExperiment",
            [None, Some(1.0), Some(-0.1), Some(0.0)],
        ),
        (
            "SuspiciousExperiment",
            [None, Some(1.0), Some(5.0), Some(0.5)],
        ),
    ];
    for (model, expected) in cases {
        let e = pkg.preflight(model).experiment.expect(model);
        assert_eq!(values(&e), expected, "{model}");
        assert!(e.non_literal.is_empty());
        assert!(
            pkg.text(e.file.as_deref(), e.range)
                .starts_with("experiment("),
            "{model}"
        );
    }
}

/// MSL の正しい例題で、Error になる失敗（NotFinite、Assertion）が無いこと。`data/msl` が無ければ何もしない。
/// 1 つの Checker で順に検査する（debug で 1 本 20 秒・1 GB 程度）。
#[test]
#[ignore = "MSL を読む（遅い）。cargo test -p globe-mode-frontend --test preflight -- --ignored"]
fn msl_examples_have_no_preflight_errors() {
    let msl = msl_dir();
    if !msl.is_dir() {
        eprintln!("{} が無いので確かめない", msl.display());
        return;
    }
    // テストのスレッドの既定（2 MiB）では、Fluid の例題のコンパイルでスタックがあふれる（CLI の主スレッドは 8 MiB）
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || check_msl_examples(&msl))
        .unwrap()
        .join()
        .unwrap();
}

fn check_msl_examples(msl: &Path) {
    let checker = Checker::new(&[msl]).expect("MSL を読める");
    let models = [
        "Modelica.Blocks.Examples.PID_Controller",
        // CombiTimeTable（外部オブジェクト）を使う
        "Modelica.Thermal.HeatTransfer.Examples.Motor",
        "Modelica.Media.Examples.SimpleLiquidWater",
        "Modelica.Media.Examples.WaterIF97",
        "Modelica.Fluid.Examples.HeatingSystem",
        "Modelica.Electrical.Analog.Examples.ChuaCircuit",
    ];
    let mut errors = Vec::new();
    for model in models {
        let report = checker.check(&[], model);
        checker.reset();
        assert!(report.passed(), "{model}: {:?}", report.diagnostics);
        let p = report
            .preflight
            .unwrap_or_else(|| panic!("{model}: preflight が無い: {:?}", report.notes));
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        for f in p.parameter_failures.iter().chain(&p.start_failures) {
            *kinds.entry(format!("{:?}", f.kind)).or_default() += 1;
        }
        eprintln!(
            "{model}: パラメータ {}（失敗 {}）、start {}（失敗 {}）、種類 {kinds:?}、experiment {:?}",
            p.parameters_evaluated,
            p.parameter_failures.len(),
            p.starts_evaluated,
            p.start_failures.len(),
            p.experiment.as_ref().map(values),
        );
        for f in p.parameter_failures.iter().chain(&p.start_failures) {
            eprintln!("    {:?} {}: {}", f.kind, f.name, f.message);
            if matches!(f.kind, FailureKind::NotFinite | FailureKind::Assertion) {
                errors.push(format!("{model}: {} ({:?}): {}", f.name, f.kind, f.message));
            }
        }
        assert!(p.experiment.is_some(), "{model}: experiment がある");
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

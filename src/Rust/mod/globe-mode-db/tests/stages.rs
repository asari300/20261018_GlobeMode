//! S7（インデックス）と S8（初期化）の端から端までのテスト（S006）。
//! 入力は src/Modelica/GlobeModeValidation（MSL を使わないモデルだけ）。
//! 各モデルの期待は src/Modelica/README.md の表と同じ。

use std::path::PathBuf;

use globe_mode_core::{Severity, Stage, codes};
use globe_mode_db::{FullReport, GlobeDb};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn check(model: &str) -> FullReport {
    let mut db = GlobeDb::new();
    db.load_dir(&package_dir())
        .expect("GlobeModeValidation を読める");
    db.check_model(&format!("GlobeModeValidation.{model}"))
}

fn codes_of(r: &FullReport) -> Vec<&'static str> {
    r.diagnostics.iter().map(|d| d.code).collect()
}

fn has(r: &FullReport, code: &str, severity: Severity) -> bool {
    r.diagnostics
        .iter()
        .any(|d| d.code == code && d.severity == severity)
}

// --------------------------------------------------------------------------- S7

#[test]
fn pendulum_is_index_3_and_passes() {
    let r = check("Index.Pendulum");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s7 = r.index.as_ref().expect("S7");
    assert!(!s7.failed);
    assert_eq!(s7.index, 3);
    assert_eq!(s7.n_differentiated, 3, "拘束と der(x) = vx と der(y) = vy");
    assert_eq!(s7.differentiations.iter().max(), Some(&2));
    assert_eq!(s7.dynamic_states, 2);
    assert!(has(&r, codes::HIGH_INDEX, Severity::Info));
    assert!(has(&r, codes::STATE_SELECTION, Severity::Info));
    // S8: 動的な状態 2 に fixed が 2 つで合う
    let s8 = r.init.as_ref().expect("S8");
    assert!(s8.is_balanced(), "{s8:?}");
    assert!(
        !codes_of(&r)
            .iter()
            .any(|c| c.starts_with("GM-W8") || c.starts_with("GM-E8"))
    );
}

#[test]
fn parallel_capacitors_is_index_2_with_one_dynamic_state() {
    let r = check("Index.ParallelCapacitors");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!(s7.index, 2);
    assert_eq!(s7.dynamic_states, 1);
    assert!(has(&r, codes::STATE_SELECTION, Severity::Info));
    let s8 = r.init.as_ref().expect("S8");
    assert!(s8.is_balanced(), "{s8:?}");
}

#[test]
fn real_circuit_is_index_1_without_differentiation() {
    let r = check("Connect.RealCircuit");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!(s7.index, 1);
    assert_eq!(s7.n_differentiated, 0);
    assert!(!has(&r, codes::HIGH_INDEX, Severity::Info));
    assert!(!has(&r, codes::STATE_SELECTION, Severity::Info));
}

#[test]
fn high_index_duplicate_has_zero_dynamic_states_and_an_uncertain_s8_warning() {
    // 構造的には vC が代数的に決まる（動的な状態 0）ので、vC(fixed = true) が 1 つ余る。
    // 余った fixed が矛盾しないかは数値で決まるので、Error ではなく警告（GM-W8004）
    let r = check("Invalid.HighIndexDuplicate");
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!(s7.index, 2);
    assert_eq!(s7.dynamic_states, 0);
    assert!(r.passed(), "{:?}", r.diagnostics);
    assert!(has(&r, codes::STATE_SELECTION, Severity::Info));
    assert!(has(
        &r,
        codes::INIT_OVERDETERMINED_UNCERTAIN,
        Severity::Warning
    ));
    assert!(!has(&r, codes::INIT_OVERDETERMINED, Severity::Error));
}

#[test]
fn structurally_singular_still_stops_at_s6_without_s7() {
    let r = check("Invalid.StructurallySingular");
    assert_eq!(r.failed_stage(), Some(Stage::S6));
    assert!(r.index.is_none(), "S6 で止まれば S7 は回さない");
    assert!(r.init.is_none(), "S6 で止まれば S8 は回さない");
}

// --------------------------------------------------------------------------- S8

#[test]
fn init_balanced_has_no_s8_diagnostics() {
    let r = check("Init.Balanced");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s8 = r.init.as_ref().expect("S8");
    assert!(s8.is_balanced());
    assert!(
        !codes_of(&r)
            .iter()
            .any(|c| c.starts_with("GM-W8") || c.starts_with("GM-E8"))
    );
}

#[test]
fn init_over_determined_is_an_error_with_position() {
    let r = check("Init.OverDetermined");
    assert_eq!(r.failed_stage(), Some(Stage::S8), "{:?}", r.diagnostics);
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == codes::INIT_OVERDETERMINED)
        .expect("GM-E8001");
    assert_eq!(d.severity, Severity::Error);
    assert!(d.range.is_some(), "初期方程式の位置が付く: {d:?}");
    assert!(
        d.file
            .as_deref()
            .is_some_and(|f| f.ends_with("OverDetermined.mo")),
        "{d:?}"
    );
}

#[test]
fn init_under_determined_warns_and_names_the_state_without_start() {
    let r = check("Init.UnderDetermined");
    assert!(r.passed(), "警告だけなので通る: {:?}", r.diagnostics);
    assert!(has(&r, codes::INIT_UNDERDETERMINED, Severity::Warning));
    let without_start: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::STATE_WITHOUT_START)
        .collect();
    assert_eq!(without_start.len(), 1, "{:?}", r.diagnostics);
    assert!(
        without_start[0].message.contains("h2"),
        "{:?}",
        without_start[0]
    );
    assert!(without_start[0].range.is_some());
}

#[test]
fn init_parameter_without_value_warns() {
    let r = check("Init.ParameterWithoutValue");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == codes::PARAMETER_WITHOUT_VALUE)
        .expect("GM-W8003");
    assert_eq!(d.severity, Severity::Warning);
    assert!(d.message.contains('T'), "{d:?}");
    assert!(d.range.is_some());
}

#[test]
fn init_free_parameter_is_balanced() {
    let r = check("Init.FreeParameter");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s8 = r.init.as_ref().expect("S8");
    assert!(s8.is_balanced(), "{s8:?}");
    assert_eq!(s8.free_parameters.len(), 1);
    assert!(
        !codes_of(&r)
            .iter()
            .any(|c| c.starts_with("GM-W8") || c.starts_with("GM-E8"))
    );
}

#[test]
fn real_circuit_is_under_determined_but_passes() {
    // fixed の無い RC 回路。不足分は start 値を推定値として使う見込み（警告だけ）
    let r = check("Connect.RealCircuit");
    assert!(r.passed());
    assert!(has(&r, codes::INIT_UNDERDETERMINED, Severity::Warning));
    assert!(has(&r, codes::STATE_WITHOUT_START, Severity::Warning));
}

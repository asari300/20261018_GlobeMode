//! S7（インデックス）と S8（初期化）の端から端までのテスト（S007）。
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

// --------------------------------------------------------------------------- S8 のマッチング（S009）

#[test]
fn init_matching_is_available_for_models_that_reach_s8() {
    for model in ["Init.Balanced", "Index.Pendulum", "Init.FreeParameter"] {
        let r = check(model);
        assert!(r.passed(), "{model}: {:?}", r.diagnostics);
        let s8 = r.init.as_ref().expect("S8");
        let m = s8
            .matching
            .as_ref()
            .unwrap_or_else(|| panic!("{model}: マッチングが無い"));
        assert!(m.is_balanced(), "{model}: {m:?}");
        assert_eq!(m.n_unknowns, m.n_equations, "{model}: {m:?}");
        assert_eq!(m.matching, m.n_equations, "{model}: {m:?}");
    }
}

#[test]
fn init_over_determined_is_a_pure_redundancy_in_the_matching() {
    let r = check("Init.OverDetermined");
    let s8 = r.init.as_ref().expect("S8");
    let m = s8.matching.as_ref().expect("マッチング");
    assert_eq!((m.extra, m.missing), (1, 0), "{m:?}");
    assert!(!m.over_with_dae, "拘束と重ならない: 確かな過剰");
    assert_eq!(
        m.redundant.len(),
        2,
        "初期方程式 x = 2 と x の fixed = true: {m:?}"
    );
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == codes::INIT_OVERDETERMINED)
        .expect("GM-E8001");
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("冗長な初期条件の候補") && n.contains("`x = 2`")),
        "{:?}",
        d.notes
    );
}

#[test]
fn redundant_fixed_and_missing_state_are_both_named() {
    // 数は 1 / 1 で合うが、x の fixed = true は拘束 x = pb と重なり、y には初期条件が無い
    let r = check("Init.RedundantAndMissing");
    assert!(r.passed(), "警告だけなので通る: {:?}", r.diagnostics);
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!((s7.index, s7.dynamic_states), (2, 1), "{s7:?}");
    let s8 = r.init.as_ref().expect("S8");
    assert_eq!((s8.required, s8.provided), (1, 1), "数は合う: {s8:?}");
    let m = s8.matching.as_ref().expect("マッチング");
    assert_eq!((m.extra, m.missing), (1, 1), "{m:?}");
    assert!(m.over_with_dae, "x の fixed は拘束と重なる");
    assert!(!s8.is_balanced());
    assert!(has(
        &r,
        codes::INIT_OVERDETERMINED_UNCERTAIN,
        Severity::Warning
    ));
    assert!(!has(&r, codes::INIT_OVERDETERMINED, Severity::Error));
    assert!(has(&r, codes::INIT_UNDERDETERMINED, Severity::Warning));
    let without_start: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::STATE_WITHOUT_START)
        .collect();
    assert_eq!(without_start.len(), 1, "{:?}", r.diagnostics);
    assert!(
        without_start[0].message.contains("`y`"),
        "{:?}",
        without_start[0]
    );
    let over = r
        .diagnostics
        .iter()
        .find(|d| d.code == codes::INIT_OVERDETERMINED_UNCERTAIN)
        .unwrap();
    assert!(
        over.notes.iter().any(|n| n.contains("`x` の fixed = true")),
        "{:?}",
        over.notes
    );
    assert!(
        over.file
            .as_deref()
            .is_some_and(|f| f.ends_with("RedundantAndMissing.mo"))
            && over.range.is_some(),
        "位置は x の宣言: {over:?}"
    );
}

#[test]
fn ungrounded_loop_passes_the_structural_checks() {
    // 接地の無い部品の単体: 数値的には特異だが、二部グラフでは見えない（限界の記録。omc は止まる）
    let r = check("Invalid.UngroundedLoop");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let s6 = r.structural.as_ref().expect("S6");
    assert!(s6.is_regular(), "{s6:?}");
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!(s7.index, 1);
}

#[test]
fn outer_modification_is_lost_by_the_compiler() {
    // Rumoca v0.10.2 の制約の記録: inner の修正（FixedInitial）が outer に伝わらず、if の中の初期方程式が消える。
    // 正しくは初期方程式 2 本で 2 / 2。Rumoca が直ってこのテストが落ちたら、README と research/rumoca.md の記録を直す
    let r = check("Init.OuterModification");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let init = r.check.init.as_ref().expect("S8 の入力");
    assert_eq!(init.initial_equations.len(), 0, "Rumoca が直った: {init:?}");
    let s8 = r.init.as_ref().expect("S8");
    assert_eq!((s8.provided, s8.required), (0, 2), "{s8:?}");
    assert!(has(&r, codes::INIT_UNDERDETERMINED, Severity::Warning));
}

#[test]
fn modifier_leaks_into_subcomponents_in_the_compiler() {
    // Rumoca v0.10.2 の制約の記録: part(theta1(fixed = true)) が part.joint.theta1 にも効き、内側の fixed = false を
    // 上書きする。正しくは 2 / 2 で合う。Rumoca が直ってこのテストが落ちたら、README と research/rumoca.md の記録を直す
    let r = check("Init.ModifierScope");
    assert!(r.passed(), "{:?}", r.diagnostics);
    let init = r.check.init.as_ref().expect("S8 の入力");
    let joint_theta1 = init
        .states
        .iter()
        .find(|s| s.name == "part.joint.theta1")
        .expect("part.joint.theta1");
    assert!(joint_theta1.fixed, "Rumoca が直った: {init:?}");
    let s8 = r.init.as_ref().expect("S8");
    let m = s8.matching.as_ref().expect("マッチング");
    assert_eq!((m.extra, m.missing), (2, 0), "{m:?}");
    assert!(has(
        &r,
        codes::INIT_OVERDETERMINED_UNCERTAIN,
        Severity::Warning
    ));
}

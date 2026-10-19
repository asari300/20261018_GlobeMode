//! 実行系に渡す DAE（`GlobeDb::compiled_model`、S011）のテスト。
//! 入力は src/Modelica/GlobeModeValidation（MSL を使わないモデルだけ）。

use std::path::PathBuf;

use globe_mode_core::Stage;
use globe_mode_db::GlobeDb;

fn db() -> GlobeDb {
    let mut db = GlobeDb::new();
    db.load_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation"),
    )
    .expect("GlobeModeValidation を読める");
    db
}

#[test]
fn compiled_model_carries_the_experiment_annotation() {
    let mut db = db();
    let model = "GlobeModeValidation.Simulation.Decay";
    assert!(db.check_model(model).passed());
    let compiled = db.compiled_model(model).expect("DAE まで変換できる");
    assert_eq!(compiled.model, model);
    let experiment = compiled.experiment.expect("experiment annotation がある");
    assert_eq!(experiment.stop_time.map(|b| b.get()), Some(1.0));
    assert_eq!(experiment.tolerance.map(|b| b.get()), Some(1e-8));
}

#[test]
fn compiled_model_fails_like_check_with_original_positions() {
    let mut db = db();
    let model = "GlobeModeValidation.Invalid.SignalTypeMismatch";
    let checked = db.check_model(model);
    let err = db
        .compiled_model(model)
        .expect_err("型の誤りのあるモデルは DAE にならない");
    assert!(err.iter().all(|d| d.is_error()));
    assert!(err.iter().all(|d| d.stage <= Stage::S5), "{err:?}");
    assert_eq!(checked.failed_stage(), err.iter().map(|d| d.stage).min());
}

#[test]
fn unknown_model_is_not_found() {
    let mut db = db();
    let err = db
        .compiled_model("GlobeModeValidation.NoSuchModel")
        .expect_err("無いモデル");
    assert!(err.iter().any(|d| d.stage == Stage::S1), "{err:?}");
}

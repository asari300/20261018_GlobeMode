//! `Workspace::simulate_many`（S322、ADR-0040）: 逐次と並列が同じ結果、比較の合否、最大次数を持たないソルバの注記、依頼の誤り。

use globe_mode::{
    CancelToken, CompareRequest, ManyHandle, ManyStatus, SimulateManyRequest, Workspace, now_ms,
};
use globe_mode_ir::SimSettings;

const DECAY: &str = "model Decay Real x(start = 1, fixed = true); equation der(x) = -x; \
                     annotation(experiment(StopTime = 1, Interval = 0.01)); end Decay;";

fn request(solvers: &[&str]) -> SimulateManyRequest {
    SimulateManyRequest {
        model: "Decay".into(),
        settings: SimSettings {
            tolerance: Some(1e-9),
            max_order: Some(3),
            ..SimSettings::default()
        },
        solvers: solvers.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    }
}

fn run(r: &SimulateManyRequest) -> globe_mode::ManyResult {
    let mut ws = Workspace::new();
    ws.set_file("Decay.mo", DECAY);
    ws.simulate_many(r, &now_ms, ManyHandle::default())
}

#[test]
fn sequential_and_parallel_give_the_same_results_and_compare_passes() {
    let mut r = request(&["bdf", "esdirk34", "tsit45"]);
    r.compare = Some(CompareRequest {
        threshold_exp: 4,
        reference: None,
    });
    let seq = run(&r);
    assert!(seq.response.ok, "{:#?}", seq.response);
    assert_eq!((seq.response.mode.as_str(), seq.response.workers), ("sequential", 1));
    let c = seq.response.comparison.as_ref().unwrap();
    assert!(c.passed && c.reference == "bdf" && c.pairs.len() == 2, "{c:#?}");

    r.parallel = true;
    let par = run(&r);
    let cpus = std::thread::available_parallelism().map_or(1, usize::from);
    if cpus > 1 {
        assert_eq!(par.response.mode, "parallel");
        assert_eq!(par.response.workers, 3.min(cpus));
    }
    for (a, b) in seq.response.runs.iter().zip(&par.response.runs) {
        assert_eq!(a.solver, b.solver);
        assert_eq!(a.response.final_values, b.response.final_values, "{}", a.solver);
    }
    for (a, b) in seq.runs.iter().zip(&par.runs) {
        assert_eq!(a.run.as_ref().unwrap().trajectory, b.run.as_ref().unwrap().trajectory);
    }
    assert!(par.response.comparison.unwrap().passed);
}

#[test]
fn compare_fails_when_a_solver_is_too_coarse_and_notes_the_ignored_max_order() {
    let mut r = request(&["bdf", "euler", "tsit45"]);
    r.compare = Some(CompareRequest {
        threshold_exp: 6,
        reference: Some("tsit45".into()),
    });
    let out = run(&r).response;
    assert_eq!(out.runs.len(), 3);
    assert!(out.runs.iter().all(|x| x.status == ManyStatus::Ok));
    // 陽的 Euler（刻み 0.01）は管の幅 1e-6 に収まらない。基準は指定した tsit45
    let c = out.comparison.unwrap();
    assert_eq!(c.reference, "tsit45");
    assert!(!c.passed);
    let euler = c.pairs.iter().find(|p| p.solver == "euler").unwrap();
    assert!(!euler.passed && euler.failures[0].name == "x", "{euler:#?}");
    assert!(c.pairs.iter().find(|p| p.solver == "bdf").unwrap().passed);
    // 最大次数を持たないソルバには注記が付き、実行は止まらない
    assert!(out.runs.iter().all(|x| x.notes.len() == 1 && x.notes[0].contains("最大次数")));
}

#[test]
fn request_problems_are_refused_and_cancel_reaches_every_run() {
    let mut r = request(&["bdf", "bdf"]);
    assert!(run(&r).response.refused.unwrap().contains("2 回"));
    r.solvers = vec![];
    assert!(run(&r).response.refused.unwrap().contains("空"));
    r.solvers = vec!["bdf".into(), "rk4".into()];
    r.compare = Some(CompareRequest {
        threshold_exp: 6,
        reference: Some("dopri45".into()),
    });
    assert!(run(&r).response.refused.unwrap().contains("solvers にない"));

    r.compare = None;
    let token = CancelToken::new();
    token.cancel();
    let mut ws = Workspace::new();
    ws.set_file("Decay.mo", DECAY);
    let out = ws.simulate_many(
        &r,
        &now_ms,
        ManyHandle {
            on_progress: None,
            cancel: Some(token),
        },
    );
    assert!(out.response.runs.iter().all(|x| x.status == ManyStatus::Cancelled));
}

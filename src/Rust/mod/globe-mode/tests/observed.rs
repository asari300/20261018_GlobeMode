//! `Workspace::simulate_observed` の進捗と中断（S123、ADR-0033 の「結果」の試験 (a)〜(d)）。

use std::cell::Cell;
use std::time::Instant;

use globe_mode::{CancelToken, ExecStage, Progress, SimulateHandle, SimulateRequest, Workspace};
use globe_mode_ir::{SimSettings, Termination};

const DECAY: &str = "model Decay Real x(start = 1, fixed = true); equation der(x) = -x; \
                     annotation(experiment(StopTime = 1, Interval = 0.01)); end Decay;";

fn workspace() -> Workspace {
    let mut ws = Workspace::new();
    ws.set_file("Decay.mo", DECAY);
    ws
}

fn request(solver: &str) -> SimulateRequest {
    SimulateRequest {
        model: "Decay".into(),
        settings: SimSettings {
            solver: Some(solver.to_string()),
            ..SimSettings::default()
        },
        no_csv: true,
        ..Default::default()
    }
}

/// (b) 進捗で t ≥ 0.5 を見てトークンを立てると、次の 1 歩の前で止まり、そこまでの列（最後は t ≥ 0.5 の出力の点）が返る
#[test]
fn cancel_during_integration_returns_the_partial_trajectory() {
    let mut ws = workspace();
    let token = CancelToken::new();
    // 呼ぶたびに 100 ms 進む時計（t を間引かずに全部知らせる）
    let clock = Cell::new(0.0);
    let now = || {
        clock.set(clock.get() + 100.0);
        clock.get()
    };
    let canceller = token.clone();
    let mut on_progress = |p: &Progress| {
        if p.time.is_some_and(|t| t >= 0.5) {
            canceller.cancel();
        }
    };
    let out = ws.simulate_observed(
        &request("esdirk34"),
        &now,
        SimulateHandle {
            on_progress: Some(&mut on_progress),
            cancel: Some(token),
        },
    );
    let r = &out.response;
    assert!(r.ok, "{:#?}", r.diagnostics);
    let Some(Termination::Cancelled { time }) = r.termination else {
        panic!("{:?}", r.termination);
    };
    assert!((0.5..1.0).contains(&time), "{time}");
    let run = out.run.unwrap();
    assert_eq!(*run.trajectory.time.last().unwrap(), time);
    assert!(r.diagnostics.iter().any(|d| d.code == "GM-I11004"));
}

/// (c) 先に立てたトークンなら、ネイティブの bdf でも lowering に入らず GM-I11004 だけ
#[test]
fn a_cancelled_token_stops_before_lowering() {
    for solver in ["bdf", "dopri45", "esdirk34"] {
        let mut ws = workspace();
        let token = CancelToken::new();
        token.cancel();
        let mut stages = Vec::new();
        let mut on_progress = |p: &Progress| stages.push(p.stage);
        let out = ws.simulate_observed(
            &request(solver),
            &|| 0.0,
            SimulateHandle {
                on_progress: Some(&mut on_progress),
                cancel: Some(token),
            },
        );
        let r = &out.response;
        assert!(!r.ok && out.run.is_none(), "{solver}");
        assert!(stages.is_empty(), "{solver}: {stages:?}");
        let codes: Vec<_> = r.diagnostics.iter().map(|d| d.code).collect();
        assert!(codes.contains(&"GM-I11004"), "{solver}: {codes:?}");
        assert!(
            !r.diagnostics.iter().any(|d| d.is_error()),
            "{solver}: {codes:?}"
        );
    }
}

/// (a)・(d) 止めなければ結果は simulate_with と同じ。t は増えていき、回数は「段の数 + 壁時計 / 0.1 s + 1」以下
#[test]
fn progress_is_throttled_and_does_not_change_the_result() {
    let mut ws = workspace();
    let plain = ws.simulate(&request("esdirk34"));
    let started = Instant::now();
    let now = || started.elapsed().as_secs_f64() * 1e3;
    let mut seen: Vec<Progress> = Vec::new();
    let mut on_progress = |p: &Progress| seen.push(*p);
    let out = ws.simulate_observed(
        &request("esdirk34"),
        &now,
        SimulateHandle {
            on_progress: Some(&mut on_progress),
            cancel: Some(CancelToken::new()),
        },
    );
    let wall = started.elapsed().as_secs_f64();
    let r = &out.response;
    assert!(r.ok);
    assert_eq!(r.termination, Some(Termination::StopTime));
    let bits = |m: &Option<std::collections::BTreeMap<String, f64>>| {
        m.as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.to_bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(bits(&plain.final_values), bits(&r.final_values));
    let stages: Vec<_> = seen
        .iter()
        .filter(|p| p.time.is_none())
        .map(|p| p.stage)
        .collect();
    use ExecStage::*;
    assert_eq!(
        stages,
        [Lowering, Preflight, Build, Initialization, Integration]
    );
    let times: Vec<f64> = seen.iter().filter_map(|p| p.time).collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");
    assert_eq!(times.last(), Some(&1.0));
    assert!(
        seen.iter()
            .all(|p| p.start_time == 0.0 && p.stop_time == 1.0)
    );
    let bound = stages.len() + (wall / 0.1) as usize + 1;
    assert!(seen.len() <= bound, "{} > {bound}", seen.len());
}

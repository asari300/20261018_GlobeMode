//! Model Exchange（S205、ADR-0026 (l)）。FMU が出す微分をこの crate の積分器（[`crate::ode`]）で積分し、事象を扱う。
//!
//! 呼び出しの順は FMI 2.0.5 の ME の疑似コード（仕様 3.2「Pseudocode Example」）と、3.0.2 の状態機械
//! （2.3「State Machine and Semantics」・3.2）に従う。2.0 と 3.0 の違いは `abi2.rs`・`abi3.rs` が吸収する（解説書の表）。

use crate::ode::{self, Tol};
use crate::{Error, Fmu, ModelDescription, Result, find_float, num, output_names};

/// `update_discrete_states`（2.0 の `fmi2NewDiscreteStates`、3.0 の `fmi3UpdateDiscreteStates`）の戻り。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EventInfo {
    /// もう一度呼ぶ必要がある（2.0 の `newDiscreteStatesNeeded`、3.0 の `discreteStatesNeedUpdate`）。
    pub discrete_states_need_update: bool,
    pub terminate: bool,
    pub nominals_changed: bool,
    /// 連続状態の値が変わった。`get_continuous_states` で取り直す。
    pub values_changed: bool,
    /// 次の time event の時刻（`nextEventTimeDefined` が偽なら `None`）。
    pub next_event_time: Option<f64>,
}

/// `completed_integrator_step` の戻り。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StepInfo {
    /// step event（FMU が事象モードに入ることを求めた）。
    pub enter_event_mode: bool,
    pub terminate: bool,
}

/// Model Exchange の FMU 1 つ。drop で `Terminate` と `FreeInstance` を呼ぶ。
/// `initialize` の後は Event Mode にいる（2.0・3.0 とも `ExitInitializationMode` が Event Mode に入る）。
pub trait FmuMe {
    fn n_states(&self) -> usize;
    fn n_event_indicators(&self) -> usize;
    /// `SetupExperiment`（2.0）→ `EnterInitializationMode` → `ExitInitializationMode`。パラメータの設定は前に。
    fn initialize(
        &mut self,
        start_time: f64,
        stop_time: Option<f64>,
        tolerance: Option<f64>,
    ) -> Result<()>;
    fn get_real(&mut self, value_references: &[u32]) -> Result<Vec<f64>>;
    fn set_real(&mut self, value_references: &[u32], values: &[f64]) -> Result<()>;
    fn set_time(&mut self, time: f64) -> Result<()>;
    fn get_continuous_states(&mut self) -> Result<Vec<f64>>;
    fn set_continuous_states(&mut self, x: &[f64]) -> Result<()>;
    fn get_nominals(&mut self) -> Result<Vec<f64>>;
    fn get_derivatives(&mut self) -> Result<Vec<f64>>;
    fn get_event_indicators(&mut self) -> Result<Vec<f64>>;
    fn enter_event_mode(&mut self) -> Result<()>;
    fn update_discrete_states(&mut self) -> Result<EventInfo>;
    fn enter_continuous_time_mode(&mut self) -> Result<()>;
    fn completed_integrator_step(&mut self) -> Result<StepInfo>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeIntegrator {
    /// 誤差制御つきの Dormand–Prince 5(4)。許容誤差は [`MeOptions::tolerance`]（既定は `DefaultExperiment` の `tolerance`、無ければ 1e-6）。
    #[default]
    Dopri5,
    /// 固定刻みの古典 RK4。刻みは出力の刻み（[`MeOptions::step`]）。
    Rk4,
}

#[derive(Debug, Clone, Default)]
pub struct MeOptions {
    /// 省略時は `DefaultExperiment` の値（無ければ 0）。
    pub start_time: Option<f64>,
    /// 省略時は `DefaultExperiment` の値（無ければエラー）。
    pub stop_time: Option<f64>,
    /// 出力の刻み。省略時は `DefaultExperiment` の `stepSize`、無ければ (stop - start) / 500。
    pub step: Option<f64>,
    /// 集める変数の名前。空なら causality が output の Float64 すべて。
    pub outputs: Vec<String>,
    /// 初期化の前に与える Float64 のパラメータ（名前, 値）。
    pub set: Vec<(String, f64)>,
    pub integrator: MeIntegrator,
    /// [`MeIntegrator::Dopri5`] の相対許容誤差。
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// event indicator の領域（> 0 か ≤ 0）が変わった。
    State,
    /// `nextEventTime` に着いた。
    Time,
    /// `completedIntegratorStep` が事象モードを求めた。
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeEvent {
    pub time: f64,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeRun {
    pub names: Vec<String>,
    /// `(時刻, 各変数の値)`。先頭は開始時刻（最初の事象の処理の後）の値。各行は出力の刻みの格子点で、事象の処理の後の値。
    pub rows: Vec<(f64, Vec<f64>)>,
    /// 事象の時刻（state event は二分法で詰めた時刻）。同じ時刻に複数の理由があれば 1 つにまとめ、`State` > `Time` > `Step` の順で種類を選ぶ。
    pub events: Vec<MeEvent>,
    /// 受理した積分の歩数。
    pub steps: u64,
    /// FMU が打ち切りを求めた（`terminateSimulation`）。
    pub terminated_by_fmu: bool,
}

/// 無限ループの防止。
const MAX_STEPS: u64 = 20_000_000;
const MAX_EVENTS: usize = 100_000;
const MAX_EVENTS_AT_ONE_TIME: usize = 100;
const MAX_EVENT_ITERATIONS: usize = 1000;

fn domain_changed(before: &[f64], after: &[f64]) -> bool {
    before
        .iter()
        .zip(after)
        .any(|(a, b)| (*a > 0.0) != (*b > 0.0))
}

fn deriv(inst: &mut dyn FmuMe, t: f64, x: &[f64]) -> Result<Vec<f64>> {
    inst.set_time(t)?;
    if !x.is_empty() {
        inst.set_continuous_states(x)?;
    }
    inst.get_derivatives()
}

/// 事象モードで `update_discrete_states` を収束するまで繰り返し、連続時間モードに戻る。`enter` が偽なら（初期化の直後）既に Event Mode。
fn event_iteration(inst: &mut dyn FmuMe, enter: bool) -> Result<EventInfo> {
    if enter {
        inst.enter_event_mode()?;
    }
    let mut acc = EventInfo::default();
    for _ in 0..MAX_EVENT_ITERATIONS {
        let info = inst.update_discrete_states()?;
        acc.terminate |= info.terminate;
        acc.nominals_changed |= info.nominals_changed;
        acc.values_changed |= info.values_changed;
        acc.next_event_time = info.next_event_time;
        if info.terminate || !info.discrete_states_need_update {
            if !info.terminate {
                inst.enter_continuous_time_mode()?;
            }
            return Ok(acc);
        }
    }
    Err(Error::Fmu(format!(
        "事象の反復が {MAX_EVENT_ITERATIONS} 回で収束しない"
    )))
}

fn tolerance_of(md: &ModelDescription, opts: &MeOptions) -> f64 {
    opts.tolerance
        .or_else(|| {
            md.default_experiment
                .as_ref()
                .and_then(|e| num(&e.tolerance))
        })
        .unwrap_or(1e-6)
}

/// Model Exchange を終了時刻まで回す。
pub fn run_me(fmu: &Fmu, opts: &MeOptions) -> Result<MeRun> {
    let md = &fmu.description;
    let exp = md.default_experiment.clone().unwrap_or_default();
    let start = opts.start_time.or(num(&exp.start_time)).unwrap_or(0.0);
    let stop = opts
        .stop_time
        .or(num(&exp.stop_time))
        .ok_or_else(|| Error::Fmu("終了時刻が無い（DefaultExperiment にも無い）".into()))?;
    if stop.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("終了時刻が開始時刻以下".into()));
    }
    let step = opts
        .step
        .or(fmu.default_step_size)
        .unwrap_or((stop - start) / 500.0);
    if step.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("刻みが正でない".into()));
    }
    let rtol = tolerance_of(md, opts);
    if opts.integrator == MeIntegrator::Dopri5
        && rtol.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
    {
        return Err(Error::Fmu("許容誤差が正でない".into()));
    }
    let names = output_names(md, &opts.outputs);
    let vrs = names
        .iter()
        .map(|n| find_float(md, n))
        .collect::<Result<Vec<_>>>()?;

    let mut inst = fmu.instantiate_me("globe")?;
    let inst = &mut *inst;
    for (name, value) in &opts.set {
        inst.set_real(&[find_float(md, name)?], &[*value])?;
    }
    inst.initialize(
        start,
        Some(stop),
        (opts.integrator == MeIntegrator::Dopri5).then_some(rtol),
    )?;
    drive(
        inst,
        &Plan {
            names,
            vrs,
            start,
            stop,
            step,
            integrator: opts.integrator,
            rtol,
        },
    )
}

struct Plan {
    names: Vec<String>,
    vrs: Vec<u32>,
    start: f64,
    stop: f64,
    step: f64,
    integrator: MeIntegrator,
    rtol: f64,
}

/// 初期化を終えた（Event Mode にいる）インスタンスを終了時刻まで進める。
fn drive(inst: &mut dyn FmuMe, plan: &Plan) -> Result<MeRun> {
    let (start, stop, step, rtol) = (plan.start, plan.stop, plan.step, plan.rtol);
    let vrs = &plan.vrs[..];
    let nx = inst.n_states();

    // 初期の事象モード（initialEventMode）
    let mut run = MeRun {
        names: plan.names.clone(),
        rows: Vec::new(),
        events: Vec::new(),
        steps: 0,
        terminated_by_fmu: false,
    };
    let info = event_iteration(inst, false)?;
    let mut t_next = info.next_event_time;
    if info.terminate {
        run.terminated_by_fmu = true;
        return Ok(run);
    }
    let mut x = inst.get_continuous_states()?;
    let mut nominals = inst.get_nominals()?;
    let mut z_prev = inst.get_event_indicators()?;
    run.rows.push((start, inst.get_real(vrs)?));

    let n_grid = ((stop - start) / step - 1e-9).ceil().max(1.0) as u64;
    let mut t = start;
    let mut h = step;
    let mut events_here = 0usize;
    'grid: for k in 1..=n_grid {
        let t_grid = if k == n_grid {
            stop
        } else {
            start + k as f64 * step
        };
        while t < t_grid {
            let t_end = t_next.map_or(t_grid, |tn| tn.min(t_grid));
            // 1 歩（t_end が現在以下の time event は、歩を進めずにそのまま事象にする）
            let (t1, x1) = if t_end <= t {
                (t, x.clone())
            } else if nx == 0 {
                (t_end, Vec::new())
            } else {
                match plan.integrator {
                    MeIntegrator::Rk4 => {
                        let hh = step.min(t_end - t);
                        let reaches = hh >= t_end - t;
                        let x1 = ode::rk4_step(&mut |tt, xx| deriv(inst, tt, xx), t, &x, hh)?;
                        (if reaches { t_end } else { t + hh }, x1)
                    }
                    MeIntegrator::Dopri5 => {
                        let atol: Vec<f64> = nominals
                            .iter()
                            .map(|n| rtol * n.abs().max(1e-300))
                            .collect();
                        let a = ode::dopri5_accepted(
                            &mut |tt, xx| deriv(inst, tt, xx),
                            t,
                            &x,
                            h,
                            t_end,
                            &Tol { rtol, atol: &atol },
                        )?;
                        h = a.h_next;
                        (a.t, a.x)
                    }
                }
            };
            if t1 > t {
                run.steps += 1;
                if run.steps > MAX_STEPS {
                    return Err(Error::Fmu(format!("積分の歩数が {MAX_STEPS} を超えた")));
                }
            }
            inst.set_time(t1)?;
            if nx > 0 {
                inst.set_continuous_states(&x1)?;
            }
            let z1 = inst.get_event_indicators()?;
            let (mut t1, mut x1) = (t1, x1);
            let mut kind = None;
            if domain_changed(&z_prev, &z1) {
                // 二分法で、領域が変わった最初の時刻を詰める（hi は変わった側）
                let (mut lo, mut hi) = (t, t1);
                while hi - lo > 1e-12 * hi.abs().max(1.0) {
                    let mid = 0.5 * (lo + hi);
                    let xm = if nx == 0 {
                        Vec::new()
                    } else {
                        ode::dopri5_step(&mut |tt, xx| deriv(inst, tt, xx), t, &x, mid - t)?.0
                    };
                    inst.set_time(mid)?;
                    if nx > 0 {
                        inst.set_continuous_states(&xm)?;
                    }
                    if domain_changed(&z_prev, &inst.get_event_indicators()?) {
                        (hi, x1) = (mid, xm);
                    } else {
                        lo = mid;
                    }
                }
                t1 = hi;
                inst.set_time(t1)?;
                if nx > 0 {
                    inst.set_continuous_states(&x1)?;
                }
                kind = Some(EventKind::State);
            }
            let sc = inst.completed_integrator_step()?;
            let time_event = t_next.is_some_and(|tn| t1 >= tn);
            if time_event && kind.is_none() {
                kind = Some(EventKind::Time);
            }
            if sc.enter_event_mode && kind.is_none() {
                kind = Some(EventKind::Step);
            }
            events_here = if t1 == t { events_here + 1 } else { 0 };
            (t, x) = (t1, x1);
            if sc.terminate {
                run.terminated_by_fmu = true;
                break 'grid;
            }
            if let Some(kind) = kind {
                if run.events.len() >= MAX_EVENTS || events_here > MAX_EVENTS_AT_ONE_TIME {
                    return Err(Error::Fmu(format!(
                        "t = {t} で事象が止まらない（Zeno かもしれない）"
                    )));
                }
                run.events.push(MeEvent { time: t, kind });
                let info = event_iteration(inst, true)?;
                t_next = info.next_event_time;
                if info.terminate {
                    run.terminated_by_fmu = true;
                    break 'grid;
                }
                if info.values_changed && nx > 0 {
                    x = inst.get_continuous_states()?;
                }
                if info.nominals_changed && nx > 0 {
                    nominals = inst.get_nominals()?;
                }
                // 事象の後は積分を最初から（刻みも出力の刻みに戻す）
                h = step;
                z_prev = inst.get_event_indicators()?;
            } else {
                z_prev = z1;
            }
        }
        run.rows.push((t, inst.get_real(vrs)?));
    }
    // FMU が途中で打ち切りを求めたときも、止まった時刻の値を最後の行に残す
    if run.terminated_by_fmu && run.rows.last().is_some_and(|r| r.0 < t) {
        run.rows.push((t, inst.get_real(vrs)?));
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// x' = 1、event indicator z = 1 - x の模擬 FMU。
    /// - 初期化: 0.25 に time event を予告する
    /// - t >= 0.5 で `completed_integrator_step` が 1 回だけ step event を求める
    /// - state event（x = 1）で x を 0 に戻す（`values_changed`）。2 回目で `terminate`
    #[derive(Default)]
    struct Mock {
        t: f64,
        x: f64,
        initial_done: bool,
        step_event_done: bool,
        state_events: u32,
        in_event_mode: bool,
    }

    impl FmuMe for Mock {
        fn n_states(&self) -> usize {
            1
        }
        fn n_event_indicators(&self) -> usize {
            1
        }
        fn initialize(&mut self, _: f64, _: Option<f64>, _: Option<f64>) -> Result<()> {
            self.in_event_mode = true;
            Ok(())
        }
        fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
            Ok(vrs.iter().map(|_| self.x).collect())
        }
        fn set_real(&mut self, _: &[u32], _: &[f64]) -> Result<()> {
            Ok(())
        }
        fn set_time(&mut self, t: f64) -> Result<()> {
            self.t = t;
            Ok(())
        }
        fn get_continuous_states(&mut self) -> Result<Vec<f64>> {
            Ok(vec![self.x])
        }
        fn set_continuous_states(&mut self, x: &[f64]) -> Result<()> {
            self.x = x[0];
            Ok(())
        }
        fn get_nominals(&mut self) -> Result<Vec<f64>> {
            Ok(vec![1.0])
        }
        fn get_derivatives(&mut self) -> Result<Vec<f64>> {
            Ok(vec![1.0])
        }
        fn get_event_indicators(&mut self) -> Result<Vec<f64>> {
            Ok(vec![1.0 - self.x])
        }
        fn enter_event_mode(&mut self) -> Result<()> {
            self.in_event_mode = true;
            Ok(())
        }
        fn update_discrete_states(&mut self) -> Result<EventInfo> {
            assert!(self.in_event_mode, "Event Mode の外で呼ばれた");
            let mut info = EventInfo::default();
            if !self.initial_done {
                self.initial_done = true;
                info.next_event_time = Some(0.25);
            } else if self.x >= 1.0 {
                self.state_events += 1;
                self.x = 0.0;
                info.values_changed = true;
                info.terminate = self.state_events == 2;
            }
            Ok(info)
        }
        fn enter_continuous_time_mode(&mut self) -> Result<()> {
            self.in_event_mode = false;
            Ok(())
        }
        fn completed_integrator_step(&mut self) -> Result<StepInfo> {
            let enter = self.t >= 0.5 && !self.step_event_done;
            self.step_event_done |= enter;
            Ok(StepInfo {
                enter_event_mode: enter,
                terminate: false,
            })
        }
    }

    #[test]
    fn time_step_and_state_events_are_handled_in_order() {
        for integrator in [MeIntegrator::Dopri5, MeIntegrator::Rk4] {
            let plan = Plan {
                names: vec!["x".into()],
                vrs: vec![0],
                start: 0.0,
                stop: 5.0,
                step: 0.1,
                integrator,
                rtol: 1e-8,
            };
            let mut mock = Mock::default();
            mock.initialize(0.0, Some(5.0), None).unwrap();
            let r = drive(&mut mock, &plan).unwrap();
            let got: Vec<(EventKind, f64)> = r.events.iter().map(|e| (e.kind, e.time)).collect();
            assert_eq!(got.len(), 4, "{integrator:?}: {got:?}");
            assert_eq!(got[0], (EventKind::Time, 0.25));
            assert_eq!(got[1].0, EventKind::Step);
            assert!((got[1].1 - 0.5).abs() < 1e-12);
            // x は 0.25 の time event で変わらないので x = t。state event は t = 1 と、リセットの後の t = 2
            assert_eq!(got[2].0, EventKind::State);
            assert!((got[2].1 - 1.0).abs() < 1e-11, "{got:?}");
            assert_eq!(got[3].0, EventKind::State);
            assert!((got[3].1 - 2.0).abs() < 1e-11, "{got:?}");
            // 2 回目の state event で FMU が打ち切りを求め、その時刻の値が最後の行に残る
            assert!(r.terminated_by_fmu);
            assert!((r.rows.last().unwrap().0 - 2.0).abs() < 1e-11);
        }
    }

    #[test]
    fn events_that_never_advance_time_are_an_error_not_a_hang() {
        // 毎回 time event を「いま」と言い張る FMU
        #[derive(Default)]
        struct Stuck(Mock);
        impl FmuMe for Stuck {
            fn n_states(&self) -> usize {
                0
            }
            fn n_event_indicators(&self) -> usize {
                0
            }
            fn initialize(&mut self, a: f64, b: Option<f64>, c: Option<f64>) -> Result<()> {
                self.0.initialize(a, b, c)
            }
            fn get_real(&mut self, v: &[u32]) -> Result<Vec<f64>> {
                self.0.get_real(v)
            }
            fn set_real(&mut self, a: &[u32], b: &[f64]) -> Result<()> {
                self.0.set_real(a, b)
            }
            fn set_time(&mut self, t: f64) -> Result<()> {
                self.0.set_time(t)
            }
            fn get_continuous_states(&mut self) -> Result<Vec<f64>> {
                Ok(vec![])
            }
            fn set_continuous_states(&mut self, _: &[f64]) -> Result<()> {
                Ok(())
            }
            fn get_nominals(&mut self) -> Result<Vec<f64>> {
                Ok(vec![])
            }
            fn get_derivatives(&mut self) -> Result<Vec<f64>> {
                Ok(vec![])
            }
            fn get_event_indicators(&mut self) -> Result<Vec<f64>> {
                Ok(vec![])
            }
            fn enter_event_mode(&mut self) -> Result<()> {
                self.0.enter_event_mode()
            }
            fn update_discrete_states(&mut self) -> Result<EventInfo> {
                Ok(EventInfo {
                    next_event_time: Some(self.0.t),
                    ..Default::default()
                })
            }
            fn enter_continuous_time_mode(&mut self) -> Result<()> {
                self.0.enter_continuous_time_mode()
            }
            fn completed_integrator_step(&mut self) -> Result<StepInfo> {
                Ok(StepInfo::default())
            }
        }
        let plan = Plan {
            names: vec![],
            vrs: vec![],
            start: 0.0,
            stop: 1.0,
            step: 0.1,
            integrator: MeIntegrator::Dopri5,
            rtol: 1e-6,
        };
        let e = drive(&mut Stuck::default(), &plan).unwrap_err();
        assert!(e.to_string().contains("事象が止まらない"), "{e}");
    }
}

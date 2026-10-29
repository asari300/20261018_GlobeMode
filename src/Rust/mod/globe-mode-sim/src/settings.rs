//! 実行の設定を決める（利用者の指定 > experiment annotation > 既定値）。
//!
//! 項目ごとに、指定があればそれを、無ければ experiment annotation の値を、それも無ければ既定値を使う。
//! 既定値は StartTime 0、StopTime は StartTime + 1、Tolerance 1e-6、Interval は区間の 1/500、ソルバ `bdf`。
//! Interval の既定値は、決まった StartTime と StopTime から計算する（指定で区間を変えれば Interval も変わる）。
//!
//! experiment annotation の StopTime が StartTime と同じ（MSL の `Modelica.Math.Examples.Inverse_sine` の
//! `StopTime = 0` など）ときだけは、Rumoca の実行系（rumoca-worker の `simulation_settings`）と同じく既定の
//! StartTime + 1 まで実行し、GM-W10001（警告）を出す（S10 の静的な検査も同じ値を誤りにしない）。
//! 指定した StopTime が StartTime と同じなら誤りのまま。StopTime が StartTime より前なら、どこから来ても誤り。
//!
//! 妥当でない値は GM-E10004（Error）にし、値が指定・experiment annotation・既定値のどれから来たかを書く。
//! experiment annotation の不正は S10 の静的な検査（`globe-mode-analysis` の preflight）も同じコードで報告する。
//! ソルバが使えないときは GM-E10005、別名を置き換えたときは GM-I10003（情報）にする。
//!
//! S012 から: Tolerance が下限 [`MIN_TOLERANCE`]（1e-10）より小さければ下限にして GM-W10003（警告）。
//! 固定刻みのソルバ（`euler`・`heun`・`rk4`・`impeuler`・`trapezoid`）は Tolerance を使わず、指定か experiment annotation にあれば
//! GM-I10004（情報）。刻み幅は正の有限の数で区間より長くないこと（だめなら GM-E10004）。固定刻みでない
//! ソルバに刻み幅を指定したら GM-W10001（使わない）。パラメータの上書きの値が有限でなければ GM-E10006
//! （tunable かどうかは実行系が確かめる）。

use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_ir::{
    Experiment, ResolvedSettings, SettingOrigin, SettingOrigins, SimSettings, SolverId,
};

use crate::solvers::{self, SolverLookup};
use crate::store::format_number;

/// 既定の StartTime。
pub const DEFAULT_START_TIME: f64 = 0.0;
/// 既定の区間の長さ（StopTime = StartTime + これ）。
pub const DEFAULT_SPAN: f64 = 1.0;
/// 既定の Tolerance。
pub const DEFAULT_TOLERANCE: f64 = 1e-6;
/// 既定の Interval は区間をこの数で割ったもの。
pub const DEFAULT_INTERVALS: f64 = 500.0;
/// 許容誤差の下限。これより小さい値は下限にして GM-W10003（警告）を出す（S012。Rumoca の実行系は 1e-10 より
/// 小さい許容誤差で、受け入れた歩の端の点とサンプラの照合が丸めの誤差に負ける。doc/research/msl-sim-failures.md）。
pub const MIN_TOLERANCE: f64 = 1e-10;
/// 既定のソルバ。
pub const DEFAULT_SOLVER: SolverId = SolverId::Bdf;

/// 設定を決める。成功なら設定と、止めない診断（GM-I10003 の別名など）を返す。
/// 決められない（ソルバが無い、区間が空など）なら Error の診断を返す（止めない診断も、出た順に一緒に入れる）。
pub fn resolve(
    requested: &SimSettings,
    experiment: Option<&Experiment>,
) -> Result<(ResolvedSettings, Vec<Diagnostic>), Vec<Diagnostic>> {
    let mut out = Diagnostics {
        list: Vec::new(),
        experiment,
    };
    let from_experiment = |get: fn(&Experiment) -> Option<globe_mode_ir::Bits>| {
        experiment.and_then(get).map(|b| b.get())
    };

    let start = pick(
        "StartTime",
        requested.start_time,
        from_experiment(|e| e.start_time),
    )
    .unwrap_or(Value::default_value("StartTime", DEFAULT_START_TIME, ""));
    let mut stop = pick(
        "StopTime",
        requested.stop_time,
        from_experiment(|e| e.stop_time),
    )
    .unwrap_or(Value::default_value(
        "StopTime",
        start.value + DEFAULT_SPAN,
        "（StartTime + 1）",
    ));
    let tolerance = pick(
        "Tolerance",
        requested.tolerance,
        from_experiment(|e| e.tolerance),
    )
    .unwrap_or(Value::default_value("Tolerance", DEFAULT_TOLERANCE, ""));

    if stop.origin == SettingOrigin::Experiment
        && start.value.is_finite()
        && stop.value == start.value
    {
        let fallback =
            Value::default_value("StopTime", start.value + DEFAULT_SPAN, "（StartTime + 1）");
        out.push(
            SettingOrigin::Experiment,
            Diagnostic::warning(
                codes::EXPERIMENT_SUSPICIOUS,
                Stage::S10,
                format!(
                    "{}（{}）が{}（{}）と同じなので、既定の StartTime + 1（{}）まで実行する",
                    stop.label(),
                    format_number(stop.value),
                    spaced(&start.label()),
                    format_number(start.value),
                    format_number(fallback.value)
                ),
            ),
        );
        stop = fallback;
    }

    let start_ok = out.finite(&start);
    let stop_ok = out.finite(&stop);
    let span = stop.value - start.value;
    let span_ok = start_ok && stop_ok && out.span(&start, &stop, span);
    if out.finite(&tolerance) && tolerance.value <= 0.0 {
        out.invalid(&tolerance, "が正でない");
    }

    let interval = match pick(
        "Interval",
        requested.interval,
        from_experiment(|e| e.interval),
    ) {
        Some(mut interval) => {
            if out.finite(&interval) {
                if interval.value <= 0.0 {
                    out.invalid(&interval, "が正でない");
                } else if span_ok && interval.value > span {
                    out.push(
                        interval.origin,
                        Diagnostic::warning(
                            codes::EXPERIMENT_SUSPICIOUS,
                            Stage::S10,
                            format!(
                                "{}（{}）が区間（{}〜{}）より長いので、区間の長さ（{}）にした（出力は始めと終わりの 2 点）",
                                interval.label(),
                                format_number(interval.value),
                                format_number(start.value),
                                format_number(stop.value),
                                format_number(span)
                            ),
                        ),
                    );
                    interval.value = span;
                }
            }
            interval
        }
        None => {
            let interval =
                Value::default_value("Interval", span / DEFAULT_INTERVALS, "（区間の 1/500）");
            // 区間が誤りなら、その診断だけを出す（既定の Interval の誤りはそこから来る）
            if span_ok && !(interval.value > 0.0 && interval.value.is_finite()) {
                out.invalid(&interval, "が正の有限の数にならない");
            }
            interval
        }
    };

    if let Some(seconds) = requested.max_wall_seconds
        && !(seconds.is_finite() && seconds > 0.0)
    {
        out.push(
            SettingOrigin::Requested,
            Diagnostic::error(
                codes::EXPERIMENT_INVALID,
                Stage::S10,
                format!(
                    "指定した壁時計の予算（{} 秒）が正の有限の数でない",
                    format_number(seconds)
                ),
            )
            .note("無制限にするときは指定しない"),
        );
    }

    let (solver, solver_origin) = match requested.solver.as_deref() {
        None => (DEFAULT_SOLVER, SettingOrigin::Default),
        Some(name) => (
            out.solver(name).unwrap_or(DEFAULT_SOLVER),
            SettingOrigin::Requested,
        ),
    };

    let fixed = solver.is_fixed_step();
    // 許容誤差: 固定刻みのソルバは使わない（指定か experiment annotation にあれば GM-I10004）。
    // 下限より小さければ下限にする（固定刻みのソルバでは使わないので、知らせずに直す）
    let mut tolerance = tolerance;
    if fixed && tolerance.origin != SettingOrigin::Default {
        out.push(
            tolerance.origin,
            Diagnostic::info(
                codes::FIXED_STEP_IGNORES_TOLERANCE,
                Stage::S10,
                format!(
                    "{}（{}）は固定刻みのソルバ {} では使わない（精度は刻み幅で決まる）",
                    tolerance.label(),
                    format_number(tolerance.value),
                    solver.name()
                ),
            ),
        );
    }
    if tolerance.value.is_finite() && tolerance.value > 0.0 && tolerance.value < MIN_TOLERANCE {
        if !fixed {
            out.push(
                tolerance.origin,
                Diagnostic::warning(
                    codes::TOLERANCE_CLAMPED,
                    Stage::S10,
                    format!(
                        "{}（{}）が下限（{}）より小さいので、下限にした",
                        tolerance.label(),
                        format_number(tolerance.value),
                        format_number(MIN_TOLERANCE)
                    ),
                )
                .note(
                    "Rumoca の実行系は 1e-10 より小さい許容誤差で、受け入れた歩の端の点の照合が丸めの誤差に負ける",
                ),
            );
        }
        tolerance.value = MIN_TOLERANCE;
    }

    // 固定刻みの幅（固定刻みのソルバのときだけ。指定が無ければ出力の間隔）
    let (step, step_origin) = if fixed {
        match requested.step {
            Some(h) => {
                let problem = if !h.is_finite() {
                    Some("が有限でない".to_string())
                } else if h <= 0.0 {
                    Some("が正でない".to_string())
                } else if span_ok && h > span {
                    Some(format!(
                        "が区間（{}〜{}）より長い",
                        format_number(start.value),
                        format_number(stop.value)
                    ))
                } else {
                    None
                };
                if let Some(problem) = problem {
                    out.push(
                        SettingOrigin::Requested,
                        Diagnostic::error(
                            codes::EXPERIMENT_INVALID,
                            Stage::S10,
                            format!("指定した刻み幅（{}）{problem}", format_number(h)),
                        ),
                    );
                }
                (Some(h), Some(SettingOrigin::Requested))
            }
            None => (Some(interval.value), Some(SettingOrigin::Default)),
        }
    } else {
        if let Some(h) = requested.step {
            out.push(
                SettingOrigin::Requested,
                Diagnostic::warning(
                    codes::EXPERIMENT_SUSPICIOUS,
                    Stage::S10,
                    format!(
                        "指定した刻み幅（{}）はソルバ {} では使わない（固定刻みのソルバ euler・heun・rk4・impeuler・trapezoid だけが使う）",
                        format_number(h),
                        solver.name()
                    ),
                ),
            );
        }
        (None, None)
    };

    // パラメータの上書きの値（tunable かどうかは実行系が確かめる）
    for (name, &value) in &requested.parameters {
        if !value.is_finite() {
            out.push(
                SettingOrigin::Requested,
                Diagnostic::error(
                    codes::PARAMETER_OVERRIDE_INVALID,
                    Stage::S10,
                    format!(
                        "パラメータ {name} の上書きの値（{}）が有限でない",
                        format_number(value)
                    ),
                ),
            );
        }
    }

    let diagnostics = out.list;
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(diagnostics);
    }
    let settings = ResolvedSettings {
        start_time: start.value,
        stop_time: stop.value,
        tolerance: tolerance.value,
        interval: interval.value,
        solver,
        max_wall_seconds: requested.max_wall_seconds,
        step,
        parameters: requested.parameters.clone(),
        origins: SettingOrigins {
            start_time: start.origin,
            stop_time: stop.origin,
            tolerance: tolerance.origin,
            interval: interval.origin,
            solver: solver_origin,
            step: step_origin,
        },
    };
    Ok((settings, diagnostics))
}

/// 出どころ付きの値。
#[derive(Debug, Clone, Copy)]
struct Value {
    /// experiment annotation の項目名（`StartTime` など）
    key: &'static str,
    value: f64,
    origin: SettingOrigin,
    /// 既定値の決め方（「（StartTime + 1）」など。既定値のときだけ）
    rule: &'static str,
}

impl Value {
    const fn default_value(key: &'static str, value: f64, rule: &'static str) -> Self {
        Value {
            key,
            value,
            origin: SettingOrigin::Default,
            rule,
        }
    }

    /// 「指定した StopTime」「experiment annotation の StopTime」「既定の StopTime（StartTime + 1）」。
    fn label(&self) -> String {
        match self.origin {
            SettingOrigin::Requested => format!("指定した {}", self.key),
            SettingOrigin::Experiment => format!("experiment annotation の {}", self.key),
            SettingOrigin::Default => format!("既定の {}{}", self.key, self.rule),
        }
    }
}

/// 助詞の後に続ける名前。英字で始まるときだけ前に空白を入れる（「が experiment annotation の」「が指定した」）。
fn spaced(label: &str) -> String {
    if label.starts_with(|c: char| c.is_ascii_alphabetic()) {
        format!(" {label}")
    } else {
        label.to_string()
    }
}

/// 指定 > experiment annotation の順に値を選ぶ（どちらも無ければ None）。
fn pick(key: &'static str, requested: Option<f64>, experiment: Option<f64>) -> Option<Value> {
    let (value, origin) = match (requested, experiment) {
        (Some(v), _) => (v, SettingOrigin::Requested),
        (None, Some(v)) => (v, SettingOrigin::Experiment),
        (None, None) => return None,
    };
    Some(Value {
        key,
        value,
        origin,
        rule: "",
    })
}

/// 診断を集める。experiment annotation から来た値の診断には、注釈の位置を付ける。
struct Diagnostics<'a> {
    list: Vec<Diagnostic>,
    experiment: Option<&'a Experiment>,
}

impl Diagnostics<'_> {
    fn push(&mut self, origin: SettingOrigin, mut d: Diagnostic) {
        if origin == SettingOrigin::Experiment
            && let Some(e) = self.experiment
        {
            match &e.file {
                Some(file) => d = d.at(file.clone(), e.range),
                None => d.range = e.range,
            }
        }
        self.list.push(d);
    }

    fn invalid(&mut self, value: &Value, problem: &str) {
        let d = Diagnostic::error(
            codes::EXPERIMENT_INVALID,
            Stage::S10,
            format!(
                "{}（{}）{problem}",
                value.label(),
                format_number(value.value)
            ),
        );
        self.push(value.origin, d);
    }

    /// 有限なら true。そうでなければ Error を足して false。
    fn finite(&mut self, value: &Value) -> bool {
        if value.value.is_finite() {
            return true;
        }
        self.invalid(value, "が有限でない");
        false
    }

    /// 区間が空でなく、長さが有限なら true。そうでなければ Error を足して false。
    fn span(&mut self, start: &Value, stop: &Value, span: f64) -> bool {
        let problem = if span <= 0.0 {
            "以下で、区間が空"
        } else if !span.is_finite() {
            "からの区間の長さが有限にならない"
        } else {
            return true;
        };
        // StopTime の側に位置を付ける（どちらも既定値なら位置は無い）
        let origin = match (start.origin, stop.origin) {
            (SettingOrigin::Experiment, _) | (_, SettingOrigin::Experiment) => {
                SettingOrigin::Experiment
            }
            _ => stop.origin,
        };
        let d = Diagnostic::error(
            codes::EXPERIMENT_INVALID,
            Stage::S10,
            format!(
                "{}（{}）が{}（{}）{problem}",
                stop.label(),
                format_number(stop.value),
                spaced(&start.label()),
                format_number(start.value)
            ),
        );
        self.push(origin, d);
        false
    }

    /// ソルバの名前を引く。使えなければ Error を足して None。
    fn solver(&mut self, name: &str) -> Option<SolverId> {
        let available = solvers::available_names().collect::<Vec<_>>().join("、");
        match solvers::lookup(name) {
            SolverLookup::Available(id) => Some(id),
            SolverLookup::Alias { requested, id } => {
                let mut d = Diagnostic::info(
                    codes::SOLVER_ALIAS,
                    Stage::S10,
                    format!(
                        "ソルバ {requested} → {}（同等の手法で、結果は一致しない）",
                        id.name()
                    ),
                );
                if let Some(e) = solvers::entry(&requested.to_ascii_lowercase()) {
                    d = d.note(e.description);
                }
                self.list.push(d);
                Some(id)
            }
            SolverLookup::Planned { name } => {
                let mut d = Diagnostic::error(
                    codes::SOLVER_UNAVAILABLE,
                    Stage::S10,
                    format!("ソルバ {name} はまだ実装していない"),
                );
                if let Some(e) = solvers::entry(&name) {
                    d = d.note(e.description);
                    if let Some(instead) = e.instead {
                        d = d.note(format!("近い手法で使えるもの: {}", instead.name()));
                    }
                }
                self.list.push(d.note(format!("使えるソルバ: {available}")));
                None
            }
            SolverLookup::Unknown { name, suggestions } => {
                let mut d = Diagnostic::error(
                    codes::SOLVER_UNAVAILABLE,
                    Stage::S10,
                    format!("ソルバ「{name}」は無い"),
                );
                if !suggestions.is_empty() {
                    d = d.note(format!("近い名前: {}", suggestions.join("、")));
                }
                self.list.push(d.note(format!("使えるソルバ: {available}")));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use globe_mode_core::{Severity, TextRange};
    use globe_mode_ir::Bits;

    use super::*;

    fn experiment(
        start: Option<f64>,
        stop: Option<f64>,
        interval: Option<f64>,
        tolerance: Option<f64>,
    ) -> Experiment {
        Experiment {
            start_time: start.map(Bits::new),
            stop_time: stop.map(Bits::new),
            interval: interval.map(Bits::new),
            tolerance: tolerance.map(Bits::new),
            file: Some("M.mo".into()),
            range: Some(TextRange::new(10, 40)),
            ..Experiment::default()
        }
    }

    fn ok(requested: &SimSettings, e: Option<&Experiment>) -> (ResolvedSettings, Vec<Diagnostic>) {
        resolve(requested, e).unwrap_or_else(|d| panic!("{d:#?}"))
    }

    fn err(requested: &SimSettings, e: Option<&Experiment>) -> Vec<Diagnostic> {
        match resolve(requested, e) {
            Ok(s) => panic!("{s:#?}"),
            Err(d) => d,
        }
    }

    fn codes_of(d: &[Diagnostic]) -> Vec<(&'static str, Severity)> {
        d.iter().map(|d| (d.code, d.severity)).collect()
    }

    #[test]
    fn defaults_without_experiment() {
        let (s, d) = ok(&SimSettings::default(), None);
        assert!(d.is_empty());
        assert_eq!(s.start_time, 0.0);
        assert_eq!(s.stop_time, 1.0);
        assert_eq!(s.tolerance, 1e-6);
        assert_eq!(s.interval, 1.0 / 500.0);
        assert_eq!(s.solver, SolverId::Bdf);
        assert_eq!(s.max_wall_seconds, None);
        assert_eq!(
            s.origins,
            SettingOrigins {
                start_time: SettingOrigin::Default,
                stop_time: SettingOrigin::Default,
                tolerance: SettingOrigin::Default,
                interval: SettingOrigin::Default,
                solver: SettingOrigin::Default,
                step: None,
            }
        );
    }

    #[test]
    fn experiment_values_are_used() {
        let e = experiment(Some(1.0), Some(11.0), Some(0.5), Some(1e-4));
        let (s, d) = ok(&SimSettings::default(), Some(&e));
        assert!(d.is_empty());
        assert_eq!(
            (s.start_time, s.stop_time, s.interval, s.tolerance),
            (1.0, 11.0, 0.5, 1e-4)
        );
        assert_eq!(s.origins.start_time, SettingOrigin::Experiment);
        assert_eq!(s.origins.interval, SettingOrigin::Experiment);
        assert_eq!(s.origins.solver, SettingOrigin::Default);
    }

    #[test]
    fn requested_values_win_over_experiment() {
        let e = experiment(Some(1.0), Some(11.0), Some(0.5), Some(1e-4));
        let requested = SimSettings {
            start_time: Some(2.0),
            stop_time: Some(4.0),
            tolerance: Some(1e-8),
            interval: Some(0.1),
            solver: Some("dopri45".into()),
            max_wall_seconds: Some(30.0),
            ..SimSettings::default()
        };
        let (s, d) = ok(&requested, Some(&e));
        assert!(d.is_empty());
        assert_eq!(
            (s.start_time, s.stop_time, s.interval, s.tolerance),
            (2.0, 4.0, 0.1, 1e-8)
        );
        assert_eq!(s.solver, SolverId::Dopri45);
        assert_eq!(s.max_wall_seconds, Some(30.0));
        assert_eq!(
            s.origins,
            SettingOrigins {
                start_time: SettingOrigin::Requested,
                stop_time: SettingOrigin::Requested,
                tolerance: SettingOrigin::Requested,
                interval: SettingOrigin::Requested,
                solver: SettingOrigin::Requested,
                step: None,
            }
        );
    }

    #[test]
    fn missing_items_fall_back_per_item() {
        // StartTime だけの注釈: StopTime は StartTime + 1、Interval は区間の 1/500
        let e = experiment(Some(2.0), None, None, None);
        let (s, _) = ok(&SimSettings::default(), Some(&e));
        assert_eq!((s.start_time, s.stop_time), (2.0, 3.0));
        assert_eq!(s.interval, 1.0 / 500.0);
        assert_eq!(s.origins.start_time, SettingOrigin::Experiment);
        assert_eq!(s.origins.stop_time, SettingOrigin::Default);
        // 指定で区間を変えると、既定の Interval も変わる
        let e = experiment(Some(0.0), Some(10.0), None, None);
        let requested = SimSettings {
            stop_time: Some(5.0),
            ..SimSettings::default()
        };
        let (s, _) = ok(&requested, Some(&e));
        assert_eq!((s.start_time, s.stop_time, s.interval), (0.0, 5.0, 0.01));
        assert_eq!(s.origins.stop_time, SettingOrigin::Requested);
        assert_eq!(s.origins.interval, SettingOrigin::Default);
    }

    #[test]
    fn stop_not_after_start_is_an_error_naming_both_origins() {
        let e = experiment(None, Some(5.0), None, None);
        let requested = SimSettings {
            start_time: Some(10.0),
            ..SimSettings::default()
        };
        let d = err(&requested, Some(&e));
        assert_eq!(codes_of(&d), [(codes::EXPERIMENT_INVALID, Severity::Error)]);
        assert_eq!(
            d[0].message,
            "experiment annotation の StopTime（5）が指定した StartTime（10）以下で、区間が空"
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));

        let requested = SimSettings {
            start_time: Some(3.0),
            stop_time: Some(3.0),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert!(
            d[0].message.starts_with("指定した StopTime（3）"),
            "{}",
            d[0].message
        );
        assert_eq!(d[0].file, None);
    }

    #[test]
    fn experiment_stop_equal_to_start_runs_one_second_with_a_warning() {
        // Modelica.Math.Examples.Inverse_sine: experiment(StopTime = 0)
        let e = experiment(None, Some(0.0), None, None);
        let (s, d) = ok(&SimSettings::default(), Some(&e));
        assert_eq!(
            (s.start_time, s.stop_time, s.interval),
            (0.0, 1.0, 1.0 / 500.0)
        );
        assert_eq!(s.origins.stop_time, SettingOrigin::Default);
        assert_eq!(
            codes_of(&d),
            [(codes::EXPERIMENT_SUSPICIOUS, Severity::Warning)]
        );
        assert_eq!(
            d[0].message,
            "experiment annotation の StopTime（0）が既定の StartTime（0）と同じなので、既定の StartTime + 1（1）まで実行する"
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));
        // 指定した StartTime と同じでも同じ扱い
        let requested = SimSettings {
            start_time: Some(2.0),
            ..SimSettings::default()
        };
        let e = experiment(None, Some(2.0), Some(0.5), None);
        let (s, d) = ok(&requested, Some(&e));
        assert_eq!((s.start_time, s.stop_time, s.interval), (2.0, 3.0, 0.5));
        assert_eq!(
            codes_of(&d),
            [(codes::EXPERIMENT_SUSPICIOUS, Severity::Warning)]
        );
        // StopTime が StartTime より前なら誤りのまま（S10 と同じ）
        let e = experiment(Some(1.0), Some(0.0), None, None);
        let d = err(&SimSettings::default(), Some(&e));
        assert_eq!(codes_of(&d), [(codes::EXPERIMENT_INVALID, Severity::Error)]);
    }

    #[test]
    fn default_stop_that_does_not_move_is_an_error() {
        // StartTime + 1 が丸めで StartTime と同じになる
        let requested = SimSettings {
            start_time: Some(1e17),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(codes_of(&d), [(codes::EXPERIMENT_INVALID, Severity::Error)]);
        assert!(
            d[0].message.contains("既定の StopTime（StartTime + 1）"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn non_finite_values_are_errors() {
        let requested = SimSettings {
            start_time: Some(f64::NAN),
            stop_time: Some(f64::INFINITY),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(
            codes_of(&d),
            [
                (codes::EXPERIMENT_INVALID, Severity::Error),
                (codes::EXPERIMENT_INVALID, Severity::Error)
            ]
        );
        assert_eq!(d[0].message, "指定した StartTime（NaN）が有限でない");
        assert_eq!(d[1].message, "指定した StopTime（Inf）が有限でない");

        // 両端は有限でも、区間の長さがあふれる
        let requested = SimSettings {
            start_time: Some(-f64::MAX),
            stop_time: Some(f64::MAX),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("有限にならない"), "{}", d[0].message);
    }

    #[test]
    fn tolerance_must_be_positive() {
        let requested = SimSettings {
            tolerance: Some(0.0),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(d[0].message, "指定した Tolerance（0）が正でない");

        let e = experiment(None, None, None, Some(-1e-6));
        let d = err(&SimSettings::default(), Some(&e));
        assert_eq!(codes_of(&d), [(codes::EXPERIMENT_INVALID, Severity::Error)]);
        assert_eq!(
            d[0].message,
            "experiment annotation の Tolerance（-1e-6）が正でない"
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));
        assert_eq!(d[0].range, Some(TextRange::new(10, 40)));
    }

    #[test]
    fn interval_must_be_positive_and_finite() {
        let e = experiment(None, None, Some(0.0), None);
        let d = err(&SimSettings::default(), Some(&e));
        assert_eq!(
            d[0].message,
            "experiment annotation の Interval（0）が正でない"
        );
        let requested = SimSettings {
            interval: Some(f64::NAN),
            ..SimSettings::default()
        };
        let d = err(&requested, Some(&e));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].message, "指定した Interval（NaN）が有限でない");
    }

    #[test]
    fn interval_longer_than_span_is_clamped_with_a_warning() {
        let e = experiment(Some(0.0), Some(2.0), Some(5.0), None);
        let (s, d) = ok(&SimSettings::default(), Some(&e));
        assert_eq!(s.interval, 2.0);
        assert_eq!(s.origins.interval, SettingOrigin::Experiment);
        assert_eq!(
            codes_of(&d),
            [(codes::EXPERIMENT_SUSPICIOUS, Severity::Warning)]
        );
        assert!(
            d[0].message
                .starts_with("experiment annotation の Interval（5）"),
            "{}",
            d[0].message
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));
    }

    #[test]
    fn max_wall_seconds_must_be_positive_and_finite() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let requested = SimSettings {
                max_wall_seconds: Some(bad),
                ..SimSettings::default()
            };
            let d = err(&requested, None);
            assert_eq!(
                codes_of(&d),
                [(codes::EXPERIMENT_INVALID, Severity::Error)],
                "{bad}"
            );
            assert!(d[0].message.starts_with("指定した壁時計の予算"));
        }
    }

    #[test]
    fn solver_alias_is_replaced_with_an_info() {
        let requested = SimSettings {
            solver: Some("DASSL".into()),
            ..SimSettings::default()
        };
        let (s, d) = ok(&requested, None);
        assert_eq!(s.solver, SolverId::Bdf);
        assert_eq!(s.origins.solver, SettingOrigin::Requested);
        assert_eq!(codes_of(&d), [(codes::SOLVER_ALIAS, Severity::Info)]);
        assert_eq!(
            d[0].message,
            "ソルバ DASSL → bdf（同等の手法で、結果は一致しない）"
        );
        assert!(!d[0].notes.is_empty());
    }

    #[test]
    fn planned_solver_is_an_error_with_a_substitute() {
        let requested = SimSettings {
            solver: Some("ida".into()),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(codes_of(&d), [(codes::SOLVER_UNAVAILABLE, Severity::Error)]);
        assert_eq!(d[0].message, "ソルバ ida はまだ実装していない");
        assert!(
            d[0].notes.iter().any(|n| n == "近い手法で使えるもの: bdf"),
            "{:?}",
            d[0].notes
        );
        assert!(
            d[0].notes.iter().any(|n| n
                == "使えるソルバ: bdf、dopri45、auto、esdirk34、trbdf2、tsit45、euler、heun、rk4、impeuler、trapezoid、esdirk23、esdirk45、sdirk34hw、dopri853、cerk23、cerk34、cerk45、radau5")
        );
    }

    #[test]
    fn unknown_solver_is_an_error_with_suggestions() {
        let requested = SimSettings {
            solver: Some("dopri".into()),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(codes_of(&d), [(codes::SOLVER_UNAVAILABLE, Severity::Error)]);
        assert_eq!(d[0].message, "ソルバ「dopri」は無い");
        assert_eq!(d[0].notes[0], "近い名前: dopri5、dopri45");
        let requested = SimSettings {
            solver: Some("xyzzy-solver".into()),
            ..SimSettings::default()
        };
        let d = err(&requested, None);
        assert_eq!(
            d[0].notes,
            [
                "使えるソルバ: bdf、dopri45、auto、esdirk34、trbdf2、tsit45、euler、heun、rk4、impeuler、trapezoid、esdirk23、esdirk45、sdirk34hw、dopri853、cerk23、cerk34、cerk45、radau5"
            ]
        );
    }

    #[test]
    fn all_errors_are_reported_together_with_infos() {
        let e = experiment(None, None, None, Some(0.0));
        let requested = SimSettings {
            solver: Some("lsodar".into()),
            interval: Some(-1.0),
            max_wall_seconds: Some(0.0),
            ..SimSettings::default()
        };
        let d = err(&requested, Some(&e));
        assert_eq!(
            codes_of(&d),
            [
                (codes::EXPERIMENT_INVALID, Severity::Error),
                (codes::EXPERIMENT_INVALID, Severity::Error),
                (codes::EXPERIMENT_INVALID, Severity::Error),
                (codes::SOLVER_ALIAS, Severity::Info),
            ]
        );
    }

    #[test]
    fn tolerance_below_the_floor_is_raised_with_a_warning() {
        let requested = SimSettings {
            tolerance: Some(1e-12),
            ..SimSettings::default()
        };
        let (s, d) = ok(&requested, None);
        assert_eq!(s.tolerance, MIN_TOLERANCE);
        assert_eq!(s.origins.tolerance, SettingOrigin::Requested);
        assert_eq!(
            codes_of(&d),
            [(codes::TOLERANCE_CLAMPED, Severity::Warning)]
        );
        assert_eq!(
            d[0].message,
            "指定した Tolerance（1e-12）が下限（1e-10）より小さいので、下限にした"
        );
        assert_eq!(d[0].file, None);
        // experiment annotation の値（MSL の SimpleTriacCircuit は 1e-12）も同じ。注釈の位置を付ける
        let e = experiment(None, None, None, Some(1e-12));
        let (s, d) = ok(&SimSettings::default(), Some(&e));
        assert_eq!(s.tolerance, MIN_TOLERANCE);
        assert_eq!(
            codes_of(&d),
            [(codes::TOLERANCE_CLAMPED, Severity::Warning)]
        );
        assert!(
            d[0].message
                .starts_with("experiment annotation の Tolerance（1e-12）")
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));
        // 下限ちょうどと、それより大きいものはそのまま
        for tol in [1e-10, 1e-9] {
            let requested = SimSettings {
                tolerance: Some(tol),
                ..SimSettings::default()
            };
            let (s, d) = ok(&requested, None);
            assert_eq!(s.tolerance, tol);
            assert!(d.is_empty());
        }
    }

    fn fixed(step: Option<f64>) -> SimSettings {
        SimSettings {
            solver: Some("rk4".into()),
            step,
            ..SimSettings::default()
        }
    }

    #[test]
    fn fixed_step_defaults_to_the_interval() {
        let (s, d) = ok(&fixed(None), None);
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(s.solver, SolverId::Rk4);
        assert_eq!(s.step, Some(1.0 / 500.0));
        assert_eq!(s.origins.step, Some(SettingOrigin::Default));
        let (s, _) = ok(&fixed(Some(0.01)), None);
        assert_eq!(s.step, Some(0.01));
        assert_eq!(s.origins.step, Some(SettingOrigin::Requested));
        // 区間の長さちょうどは 1 歩
        let (s, _) = ok(&fixed(Some(1.0)), None);
        assert_eq!(s.step, Some(1.0));
    }

    #[test]
    fn fixed_step_must_be_positive_finite_and_within_the_span() {
        for (h, message) in [
            (0.0, "指定した刻み幅（0）が正でない"),
            (-0.1, "指定した刻み幅（-0.1）が正でない"),
            (f64::NAN, "指定した刻み幅（NaN）が有限でない"),
            (f64::INFINITY, "指定した刻み幅（Inf）が有限でない"),
            (2.0, "指定した刻み幅（2）が区間（0〜1）より長い"),
        ] {
            let d = err(&fixed(Some(h)), None);
            assert_eq!(
                codes_of(&d),
                [(codes::EXPERIMENT_INVALID, Severity::Error)],
                "{h}"
            );
            assert_eq!(d[0].message, message);
        }
    }

    #[test]
    fn step_is_ignored_by_adaptive_solvers_with_a_warning() {
        let requested = SimSettings {
            step: Some(0.01),
            ..SimSettings::default()
        };
        let (s, d) = ok(&requested, None);
        assert_eq!((s.step, s.origins.step), (None, None));
        assert_eq!(
            codes_of(&d),
            [(codes::EXPERIMENT_SUSPICIOUS, Severity::Warning)]
        );
        assert!(
            d[0].message.contains("ソルバ bdf では使わない"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn fixed_step_solvers_report_an_unused_tolerance() {
        // 既定の Tolerance なら何も出さない
        let (_, d) = ok(&fixed(None), None);
        assert!(d.is_empty());
        let requested = SimSettings {
            tolerance: Some(1e-8),
            ..fixed(None)
        };
        let (s, d) = ok(&requested, None);
        assert_eq!(s.tolerance, 1e-8);
        assert_eq!(
            codes_of(&d),
            [(codes::FIXED_STEP_IGNORES_TOLERANCE, Severity::Info)]
        );
        assert_eq!(
            d[0].message,
            "指定した Tolerance（1e-8）は固定刻みのソルバ rk4 では使わない（精度は刻み幅で決まる）"
        );
        // experiment annotation の値は位置付き。下限より小さくても警告は出さない（使わないので）
        let e = experiment(None, None, None, Some(1e-12));
        let (s, d) = ok(&fixed(None), Some(&e));
        assert_eq!(s.tolerance, MIN_TOLERANCE);
        assert_eq!(
            codes_of(&d),
            [(codes::FIXED_STEP_IGNORES_TOLERANCE, Severity::Info)]
        );
        assert_eq!(d[0].file.as_deref(), Some("M.mo"));
    }

    #[test]
    fn parameter_overrides_must_be_finite() {
        let mut requested = SimSettings::default();
        requested.parameters.insert("R".into(), 10.0);
        let (s, d) = ok(&requested, None);
        assert!(d.is_empty());
        assert_eq!(s.parameters.get("R"), Some(&10.0));
        requested.parameters.insert("C".into(), f64::NAN);
        requested.parameters.insert("L".into(), f64::NEG_INFINITY);
        let d = err(&requested, None);
        assert_eq!(
            codes_of(&d),
            [
                (codes::PARAMETER_OVERRIDE_INVALID, Severity::Error),
                (codes::PARAMETER_OVERRIDE_INVALID, Severity::Error)
            ]
        );
        assert_eq!(d[0].message, "パラメータ C の上書きの値（NaN）が有限でない");
        assert_eq!(
            d[1].message,
            "パラメータ L の上書きの値（-Inf）が有限でない"
        );
    }
}

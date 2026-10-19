//! S10（プリフライト）: パラメータと start 値の評価、experiment annotation。S010。
//!
//! 入力は [`PreflightStructure`]（フロントエンドが評価した、パラメータと start 値の失敗と、
//! experiment annotation の生の値）。今回は静的な部分だけを判定する。t0 での残差、初期化のヤコビ行列の
//! 階数、ソルバの能力は、実行系ができてから足す。
//!
//! - パラメータの値が有限にならない（0 除算、NaN、無限大）→ `GM-E10001`（Error。1 件ずつ）
//! - パラメータの評価で（関数の中の）assert が失敗した → `GM-E10002`（Error。1 件ずつ）
//! - start 値が有限にならない → `GM-E10003`（Error。1 件ずつ）
//! - 外部関数・外部オブジェクトで静的に評価できないパラメータ → `GM-I10002`（Info。まとめて 1 件）
//! - そのほかの評価の失敗（評価器が対応していない演算など）と、値の無いパラメータ（S8 の `GM-W8003` と
//!   重なる）は出さない。評価器（Rumoca）の限界であって、モデルの誤りとは言えないため
//!   （MSL の例題の結果は frontend の `tests/preflight.rs` の `msl_examples_have_no_preflight_errors`）。
//! - experiment annotation:
//!   - 無い → `GM-I10001`（Info）
//!   - StopTime < StartTime、Tolerance ≤ 0、Interval ≤ 0 → `GM-E10004`（Error。項目ごと）
//!   - Interval が区間（StopTime − StartTime）より長い、Tolerance ≥ 0.1、値が数の字面でない
//!     → `GM-W10001`（Warning。項目ごと）
//!   - StopTime = StartTime は出さない（MSL の `Modelica.Media.Examples.SolveOneNonlinearEquation.Inverse_sine`
//!     は `StopTime = 0` で、t = 0 の 1 点だけを計算する）
//!   - StartTime が無いときは 0 とみなす（omc の既定。MLS は既定を決めていない）。StopTime が無いときは、
//!     区間に関わる判定をしない
//!
//! Rumoca に依存しない。wasm32 でも動く。

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{EvaluationFailure, Experiment, FailureKind, PreflightStructure};
use serde::Serialize;

/// Tolerance がこれ以上なら疑わしい（相対誤差 10 %）
const SUSPICIOUS_TOLERANCE: f64 = 0.1;
/// 補足に名前を並べる上限
const NOTE_LIST_LIMIT: usize = 10;

/// S10 の結果。添字は [`PreflightStructure`] の `parameter_failures` / `start_failures` のもの（昇順）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct PreflightReport {
    /// 評価したパラメータのスカラーの数
    pub parameters_evaluated: usize,
    /// 評価した start 値のスカラーの数
    pub starts_evaluated: usize,
    /// 値が有限にならないパラメータ（GM-E10001）
    pub not_finite_parameters: Vec<usize>,
    /// assert が失敗したパラメータ（GM-E10002）
    pub assertion_parameters: Vec<usize>,
    /// 静的に評価できないパラメータ（外部関数など。GM-I10002）
    pub external_parameters: Vec<usize>,
    /// そのほかの理由で評価できなかったパラメータ（診断は出さない）
    pub other_parameters: Vec<usize>,
    /// 値が有限にならない start 値（GM-E10003）
    pub not_finite_starts: Vec<usize>,
    /// そのほかの理由で評価できなかった start 値（診断は出さない）
    pub other_starts: Vec<usize>,
    /// experiment annotation の判定
    pub experiment: ExperimentVerdict,
}

/// experiment annotation の判定。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum ExperimentVerdict {
    /// annotation が無い（GM-I10001）
    #[default]
    Missing,
    /// annotation がある。誤り（GM-E10004）と疑わしい点（GM-W10001）
    Checked {
        invalid: Vec<ExperimentIssue>,
        suspicious: Vec<ExperimentIssue>,
    },
}

/// experiment annotation の問題 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentIssue {
    /// StopTime < StartTime
    StopBeforeStart,
    /// Tolerance ≤ 0
    NonPositiveTolerance,
    /// Interval ≤ 0
    NonPositiveInterval,
    /// Interval が区間より長い（出力が 1 点になる）
    IntervalLongerThanSpan,
    /// Tolerance ≥ 0.1
    LargeTolerance,
    /// 値が数の字面でない（項目の名前）
    NonLiteral(String),
}

impl PreflightReport {
    /// 誤り（Error）があるか。
    pub fn has_errors(&self) -> bool {
        !self.not_finite_parameters.is_empty()
            || !self.assertion_parameters.is_empty()
            || !self.not_finite_starts.is_empty()
            || matches!(&self.experiment, ExperimentVerdict::Checked { invalid, .. } if !invalid.is_empty())
    }

    /// 診断（S10）。`preflight` は、この結果を作ったときと同じ（位置のある）ものを渡す。
    pub fn diagnostics(&self, preflight: &PreflightStructure) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let parameter = |i: usize| preflight.parameter_failures.get(i);
        let start = |i: usize| preflight.start_failures.get(i);

        for f in self
            .not_finite_parameters
            .iter()
            .filter_map(|&i| parameter(i))
        {
            out.push(
                failure(
                    Diagnostic::error(
                        codes::PARAMETER_NOT_FINITE,
                        Stage::S10,
                        format!("パラメータ `{}` の値が有限にならない", f.name),
                    ),
                    f,
                )
                .note("0 除算、負の数の平方根や対数などが無いか、値と、それが使うパラメータを確かめる"),
            );
        }
        for f in self
            .assertion_parameters
            .iter()
            .filter_map(|&i| parameter(i))
        {
            out.push(
                failure(
                    Diagnostic::error(
                        codes::PARAMETER_ASSERTION,
                        Stage::S10,
                        format!("パラメータ `{}` の評価で assert が失敗した", f.name),
                    ),
                    f,
                )
                .note("関数の引数（パラメータの値）が、関数の assert の条件を満たすようにする"),
            );
        }
        for f in self.not_finite_starts.iter().filter_map(|&i| start(i)) {
            out.push(
                failure(
                    Diagnostic::error(
                        codes::START_NOT_FINITE,
                        Stage::S10,
                        format!("`{}` の start 値が有限にならない", f.name),
                    ),
                    f,
                )
                .note("start の式と、それが使うパラメータの値を確かめる"),
            );
        }
        if !self.external_parameters.is_empty() {
            let names: Vec<&str> = self
                .external_parameters
                .iter()
                .filter_map(|&i| parameter(i))
                .map(|f| f.name.as_str())
                .collect();
            let first = parameter(self.external_parameters[0]);
            let mut d = Diagnostic::info(
                codes::PARAMETER_NOT_STATIC,
                Stage::S10,
                format!(
                    "パラメータ {} 個は外部関数で決まり、静的には評価できない（実行時に決まる）",
                    names.len()
                ),
            )
            .note(format!("パラメータ: {}", list(&names)));
            if let Some(f) = first {
                d = locate(d, f.file.as_deref(), f.range);
            }
            out.push(d);
        }

        match (&self.experiment, &preflight.experiment) {
            (ExperimentVerdict::Missing, _) | (_, None) => out.push(
                Diagnostic::info(
                    codes::EXPERIMENT_MISSING,
                    Stage::S10,
                    "experiment annotation が無い（実行ではツールの既定の区間を使う見込み）",
                )
                .note("annotation(experiment(StartTime = 0, StopTime = …)) を書く"),
            ),
            (
                ExperimentVerdict::Checked {
                    invalid,
                    suspicious,
                },
                Some(e),
            ) => {
                for issue in invalid {
                    out.push(locate(
                        Diagnostic::error(codes::EXPERIMENT_INVALID, Stage::S10, issue.message(e)),
                        e.file.as_deref(),
                        e.range,
                    ));
                }
                for issue in suspicious {
                    out.push(locate(
                        Diagnostic::warning(
                            codes::EXPERIMENT_SUSPICIOUS,
                            Stage::S10,
                            issue.message(e),
                        ),
                        e.file.as_deref(),
                        e.range,
                    ));
                }
            }
        }
        out
    }
}

impl ExperimentIssue {
    fn message(&self, e: &Experiment) -> String {
        let value = |v: Option<globe_mode_ir::Bits>| v.map(|b| b.get()).unwrap_or_default();
        match self {
            ExperimentIssue::StopBeforeStart => format!(
                "experiment annotation の StopTime（{}）が StartTime（{}）より前",
                value(e.stop_time),
                value(e.start_time)
            ),
            ExperimentIssue::NonPositiveTolerance => format!(
                "experiment annotation の Tolerance（{}）が正でない",
                value(e.tolerance)
            ),
            ExperimentIssue::NonPositiveInterval => format!(
                "experiment annotation の Interval（{}）が正でない",
                value(e.interval)
            ),
            ExperimentIssue::IntervalLongerThanSpan => format!(
                "experiment annotation の Interval（{}）が区間（{}〜{}）より長い（出力が 1 点になる）",
                value(e.interval),
                value(e.start_time),
                value(e.stop_time)
            ),
            ExperimentIssue::LargeTolerance => format!(
                "experiment annotation の Tolerance（{}）が大きすぎる（{SUSPICIOUS_TOLERANCE} 以上）",
                value(e.tolerance)
            ),
            ExperimentIssue::NonLiteral(key) => format!(
                "experiment annotation の {key} が数の字面でない（MLS の文法では数に限る。値を読めない）"
            ),
        }
    }
}

/// S10 の解析。
pub fn analyze_preflight(preflight: &PreflightStructure) -> PreflightReport {
    let mut report = PreflightReport {
        parameters_evaluated: preflight.parameters_evaluated,
        starts_evaluated: preflight.starts_evaluated,
        experiment: preflight
            .experiment
            .as_ref()
            .map_or(ExperimentVerdict::Missing, check_experiment),
        ..PreflightReport::default()
    };
    for (i, f) in preflight.parameter_failures.iter().enumerate() {
        match f.kind {
            FailureKind::NotFinite => report.not_finite_parameters.push(i),
            FailureKind::Assertion => report.assertion_parameters.push(i),
            FailureKind::External => report.external_parameters.push(i),
            // 値の無いパラメータは S8 の GM-W8003 が受け持つ
            FailureKind::Missing => {}
            FailureKind::Other => report.other_parameters.push(i),
        }
    }
    for (i, f) in preflight.start_failures.iter().enumerate() {
        match f.kind {
            FailureKind::NotFinite => report.not_finite_starts.push(i),
            // start の assert・外部関数も、数値として誤りとは言えない（パラメータの側で報告される）
            FailureKind::Assertion
            | FailureKind::External
            | FailureKind::Missing
            | FailureKind::Other => report.other_starts.push(i),
        }
    }
    report
}

/// experiment annotation の値を判定する。
fn check_experiment(e: &Experiment) -> ExperimentVerdict {
    let get = |v: Option<globe_mode_ir::Bits>| v.map(|b| b.get());
    let (start, stop, interval, tolerance) = (
        get(e.start_time),
        get(e.stop_time),
        get(e.interval),
        get(e.tolerance),
    );
    let mut invalid = Vec::new();
    let mut suspicious = Vec::new();
    // StartTime が無ければ 0 とみなす
    let span = stop.map(|stop| stop - start.unwrap_or(0.0));
    if span.is_some_and(|s| s < 0.0) {
        invalid.push(ExperimentIssue::StopBeforeStart);
    }
    if tolerance.is_some_and(|t| t <= 0.0) {
        invalid.push(ExperimentIssue::NonPositiveTolerance);
    }
    if interval.is_some_and(|i| i <= 0.0) {
        invalid.push(ExperimentIssue::NonPositiveInterval);
    }
    if let (Some(i), Some(s)) = (interval, span)
        && i > 0.0
        && s > 0.0
        && i > s
    {
        suspicious.push(ExperimentIssue::IntervalLongerThanSpan);
    }
    if tolerance.is_some_and(|t| t >= SUSPICIOUS_TOLERANCE) {
        suspicious.push(ExperimentIssue::LargeTolerance);
    }
    suspicious.extend(
        e.non_literal
            .iter()
            .cloned()
            .map(ExperimentIssue::NonLiteral),
    );
    ExperimentVerdict::Checked {
        invalid,
        suspicious,
    }
}

/// 失敗の位置と、評価器の言葉を診断に付ける。
fn failure(d: Diagnostic, f: &EvaluationFailure) -> Diagnostic {
    locate(d, f.file.as_deref(), f.range).note(format!("評価器: {}", f.message))
}

fn locate(mut d: Diagnostic, file: Option<&str>, range: Option<TextRange>) -> Diagnostic {
    match file {
        Some(f) => d.at(f, range),
        None => {
            d.range = range;
            d
        }
    }
}

/// 名前を `` `a`, `b` `` の形に並べる。[`NOTE_LIST_LIMIT`] を超えた分は「ほか k 件」にまとめる。
fn list(names: &[&str]) -> String {
    let mut s: Vec<String> = names
        .iter()
        .take(NOTE_LIST_LIMIT)
        .map(|n| format!("`{n}`"))
        .collect();
    if names.len() > NOTE_LIST_LIMIT {
        s.push(format!("ほか {} 件", names.len() - NOTE_LIST_LIMIT));
    }
    s.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_core::Severity;
    use globe_mode_ir::Bits;

    const FILE: &str = "M.mo";

    fn fail(name: &str, kind: FailureKind, at: u32) -> EvaluationFailure {
        EvaluationFailure {
            name: name.into(),
            kind,
            message: "numeric evaluation produced a non-finite result: `sqrt(a)`".into(),
            file: Some(FILE.into()),
            range: Some(TextRange::new(at, at + 10)),
        }
    }

    fn experiment(values: [Option<f64>; 4]) -> Experiment {
        let [start_time, stop_time, interval, tolerance] = values.map(|v| v.map(Bits::new));
        Experiment {
            start_time,
            stop_time,
            interval,
            tolerance,
            non_literal: Vec::new(),
            file: Some(FILE.into()),
            range: Some(TextRange::new(100, 140)),
        }
    }

    fn with_experiment(e: Option<Experiment>) -> PreflightStructure {
        PreflightStructure {
            experiment: e,
            ..PreflightStructure::default()
        }
    }

    /// (コード, 重さ)
    fn codes_of(p: &PreflightStructure) -> Vec<(&'static str, Severity)> {
        analyze_preflight(p)
            .diagnostics(p)
            .iter()
            .map(|d| (d.code, d.severity))
            .collect()
    }

    fn valid() -> Experiment {
        experiment([Some(0.0), Some(1.0), Some(0.01), Some(1e-6)])
    }

    #[test]
    fn clean_model_has_no_diagnostics() {
        let p = PreflightStructure {
            parameters_evaluated: 3,
            starts_evaluated: 2,
            experiment: Some(valid()),
            ..PreflightStructure::default()
        };
        let r = analyze_preflight(&p);
        assert!(!r.has_errors());
        assert_eq!(r.parameters_evaluated, 3);
        assert_eq!(r.starts_evaluated, 2);
        assert!(r.diagnostics(&p).is_empty());
    }

    #[test]
    fn failures_map_to_codes_by_kind() {
        let p = PreflightStructure {
            parameters_evaluated: 6,
            parameter_failures: vec![
                fail("r", FailureKind::NotFinite, 10),
                fail("d", FailureKind::Assertion, 20),
                fail("g", FailureKind::External, 30),
                fail("h", FailureKind::External, 40),
                fail("m", FailureKind::Missing, 50),
                fail("o", FailureKind::Other, 60),
            ],
            starts_evaluated: 2,
            start_failures: vec![
                fail("x", FailureKind::NotFinite, 70),
                fail("y", FailureKind::Other, 80),
            ],
            experiment: Some(valid()),
        };
        let r = analyze_preflight(&p);
        assert_eq!(r.not_finite_parameters, [0]);
        assert_eq!(r.assertion_parameters, [1]);
        assert_eq!(r.external_parameters, [2, 3]);
        assert_eq!(r.other_parameters, [5]);
        assert_eq!(r.not_finite_starts, [0]);
        assert_eq!(r.other_starts, [1]);
        assert!(r.has_errors());
        let d = r.diagnostics(&p);
        assert_eq!(
            d.iter().map(|d| (d.code, d.severity)).collect::<Vec<_>>(),
            [
                (codes::PARAMETER_NOT_FINITE, Severity::Error),
                (codes::PARAMETER_ASSERTION, Severity::Error),
                (codes::START_NOT_FINITE, Severity::Error),
                // 外部関数は 1 件にまとめる。Missing と Other は出さない
                (codes::PARAMETER_NOT_STATIC, Severity::Info),
            ]
        );
        assert_eq!(d[0].stage, Stage::S10);
        assert_eq!(d[0].file.as_deref(), Some(FILE));
        assert_eq!(d[0].range, Some(TextRange::new(10, 20)));
        assert!(d[0].message.contains("`r`"));
        assert!(d[0].notes.iter().any(|n| n.contains("sqrt(a)")));
        assert!(d[2].message.contains("`x`"));
        assert!(d[3].message.contains("2 個"));
        assert!(d[3].notes[0].contains("`g`, `h`"));
        assert_eq!(d[3].range, Some(TextRange::new(30, 40)));
    }

    #[test]
    fn missing_experiment_is_info() {
        let p = with_experiment(None);
        let r = analyze_preflight(&p);
        assert_eq!(r.experiment, ExperimentVerdict::Missing);
        assert!(!r.has_errors());
        assert_eq!(codes_of(&p), [(codes::EXPERIMENT_MISSING, Severity::Info)]);
    }

    #[test]
    fn stop_before_start_is_invalid() {
        let p = with_experiment(Some(experiment([Some(1.0), Some(0.5), None, None])));
        let r = analyze_preflight(&p);
        assert!(r.has_errors());
        let d = r.diagnostics(&p);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, codes::EXPERIMENT_INVALID);
        assert_eq!(
            d[0].message,
            "experiment annotation の StopTime（0.5）が StartTime（1）より前"
        );
        assert_eq!(d[0].range, Some(TextRange::new(100, 140)));
    }

    #[test]
    fn stop_before_default_start_is_invalid() {
        // StartTime が無ければ 0
        let p = with_experiment(Some(experiment([None, Some(-1.0), None, None])));
        assert_eq!(codes_of(&p), [(codes::EXPERIMENT_INVALID, Severity::Error)]);
    }

    #[test]
    fn equal_start_and_stop_is_accepted() {
        // MSL の Inverse_sine は StopTime = 0（t = 0 の 1 点だけを計算する）
        let p = with_experiment(Some(experiment([None, Some(0.0), None, None])));
        assert!(codes_of(&p).is_empty());
        let p = with_experiment(Some(experiment([Some(2.0), Some(2.0), Some(0.1), None])));
        assert!(
            codes_of(&p).is_empty(),
            "区間が 0 なら Interval とも比べない"
        );
    }

    #[test]
    fn non_positive_tolerance_and_interval_are_invalid() {
        let p = with_experiment(Some(experiment([None, Some(1.0), Some(-0.1), Some(0.0)])));
        let r = analyze_preflight(&p);
        assert_eq!(
            r.experiment,
            ExperimentVerdict::Checked {
                invalid: vec![
                    ExperimentIssue::NonPositiveTolerance,
                    ExperimentIssue::NonPositiveInterval
                ],
                suspicious: vec![],
            }
        );
        assert_eq!(
            codes_of(&p),
            [
                (codes::EXPERIMENT_INVALID, Severity::Error),
                (codes::EXPERIMENT_INVALID, Severity::Error)
            ]
        );
    }

    #[test]
    fn long_interval_large_tolerance_and_non_literal_are_suspicious() {
        let mut e = experiment([None, Some(1.0), Some(5.0), Some(0.5)]);
        e.non_literal = vec!["StartTime".into()];
        let p = with_experiment(Some(e));
        let r = analyze_preflight(&p);
        assert!(!r.has_errors());
        assert_eq!(
            r.experiment,
            ExperimentVerdict::Checked {
                invalid: vec![],
                suspicious: vec![
                    ExperimentIssue::IntervalLongerThanSpan,
                    ExperimentIssue::LargeTolerance,
                    ExperimentIssue::NonLiteral("StartTime".into()),
                ],
            }
        );
        let d = r.diagnostics(&p);
        assert!(
            d.iter()
                .all(|d| d.code == codes::EXPERIMENT_SUSPICIOUS && d.severity == Severity::Warning)
        );
        assert_eq!(
            d[0].message,
            "experiment annotation の Interval（5）が区間（0〜1）より長い（出力が 1 点になる）"
        );
    }

    #[test]
    fn interval_without_stop_time_is_not_compared() {
        let p = with_experiment(Some(experiment([None, None, Some(5.0), Some(1e-4)])));
        assert!(codes_of(&p).is_empty());
    }

    #[test]
    fn external_note_is_truncated() {
        let failures: Vec<EvaluationFailure> = (0..12)
            .map(|i| fail(&format!("p{i}"), FailureKind::External, i * 10))
            .collect();
        let p = PreflightStructure {
            parameters_evaluated: 12,
            parameter_failures: failures,
            experiment: Some(valid()),
            ..PreflightStructure::default()
        };
        let d = analyze_preflight(&p).diagnostics(&p);
        assert_eq!(d.len(), 1);
        assert!(d[0].notes[0].ends_with("ほか 2 件"), "{}", d[0].notes[0]);
    }
}

//! プリフライト（S10）の入力の取り出し: パラメータと start 値の評価、experiment annotation。S010。
//!
//! - パラメータ: `VariableRole::Parameter` と `Constant` を、Rumoca の `NumericEvaluator::initial_value`
//!   （束縛、無ければ start を辿って評価する）で、変数ごとに評価する。数はスカラーごとに数える。
//!   - `fixed = false` のスカラー（初期化で決まる。S8 の扱い）は評価しない。束縛や start が（推移的に）
//!     `fixed = false` のパラメータを参照する変数も評価しない（評価器はその start を値として使ってしまい、
//!     初期化の前の仮の値で 0 除算などを誤って報告するため）。
//!   - 失敗は変数ごとに 1 つ（配列の評価はまとめて失敗するので、どのスカラーかは分からない）。
//!     失敗の原因が、参照する別のパラメータの失敗と同じ（種類と位置が同じ）なら、元の 1 つだけを残す。
//! - start 値: 状態・代数・出力・離散変数を `initial_value`（start、無ければ束縛）で評価する。値が無いのは
//!   既定の 0 なので失敗にしない。実行時の値（ほかの未知数）を参照するもの（`NonStaticCoordinate`）も、
//!   静的な値ではないので失敗にしない。
//! - String の変数（MSL の表の `tableName` など）は数値ではないので、評価も数えもしない。
//! - experiment annotation: Rumoca の `CompilationResult::experiment_*` は、不正な値を捨てた後のもの
//!   （`rumoca-compile/src/experiment.rs` の `sanitize_experiment_settings`。StopTime < 0、
//!   StopTime < StartTime、Tolerance ≤ 0、Interval ≤ 0 を None にする）なので、それでは誤りを見つけられない。
//!   そこで、根のクラスを定義するファイルを DAE のソースの表（`Dae::source_map`）から探して Rumoca の
//!   parser で読み直し、そのクラスの annotation から生の値を取る（[`extract_for_model`]）。
//!   見つからなければ Rumoca の値で代える（位置は None）。
//!
//! Rumoca v0.10.2 の評価器（`rumoca-eval-dae/src/numeric.rs`）について、ソースで確かめたこと:
//!
//! - 値が有限でなければ必ず失敗にする（`require_finite`。式ごとと変数の値ごと）。
//!   `NumericEvaluationErrorKind::InvalidValue` で、言葉は "numeric evaluation produced a non-finite result"。
//!   だから、評価できた値は常に有限で、値そのものを調べ直す必要はない。
//! - `InvalidValue` はほかに、`div` / `mod` / `rem` の 0 除算（"quotient divisor must be nonzero"）と、
//!   値の形の誤り（条件が Boolean でない、範囲の刻みが 0 など）にも使われる。前の 2 つを NotFinite、
//!   残りを Other にする。
//! - `Overflow` は添字や要素の数の計算のあふれで、数値のあふれではない（数値は f64 の無限大になり、
//!   上の `require_finite` で `InvalidValue` になる）。Other にする。
//! - 関数の中の `assert`（level = Error）は、条件が false なら `AssertionFailed`
//!   （"function assertion failed"。assert の文言は含まない。位置は assert の文）。level = Warning は無視する。
//!   モデルの方程式の節の `assert` は評価しない（実行時の検査）。
//! - 外部関数（`external "C"`）は評価できず `ExternalFunction` になる。
//!
//! 取り出せないときは Err（理由）。panic しても Err にする。

use std::collections::{HashMap, HashSet};

use globe_mode_core::TextRange;
use globe_mode_ir::{Bits, EvaluationFailure, Experiment, FailureKind, PreflightStructure};
use rumoca_compile::compile::{
    CompilationResult, CoordinateView, Dae, DaeView, VariableRole, VariableView,
};
use rumoca_compile::parsing::ast;
use rumoca_eval_dae::{
    NumericEvaluationError, NumericEvaluationErrorKind, NumericEvaluator,
    ScalarCoordinateProjectionCache, for_each_scalar_coordinate_cached,
};
use rumoca_ir_dae::ScalarType;

use crate::structure::location;

/// 失敗の言葉に添える、式の本文の長さ（文字）
const EXCERPT_CHARS: usize = 60;

/// コンパイルの結果から、S10 の入力を取り出す。experiment は Rumoca の（不正な値を捨てた後の）値。
/// 生の値との比べ（テスト）にだけ使う。
#[cfg(test)]
pub(crate) fn extract(result: &CompilationResult) -> Result<PreflightStructure, String> {
    extract_inner(result, None)
}

/// コンパイルの結果から、S10 の入力を取り出す。`model`（根のクラスの完全な名前）の annotation を読み直し、
/// experiment の生の値と位置を取る（Rumoca の値は不正な値を捨てた後のものなので）。根のクラスが見つからなければ
/// Rumoca の値で代える。取り出せなければ理由（`check` の notes に入る）。
pub(crate) fn extract_for_model(
    result: &CompilationResult,
    model: &str,
) -> Result<PreflightStructure, String> {
    extract_inner(result, Some(model))
}

/// モデルの experiment annotation だけ（S011。実行の設定に使う）。根のクラスの本文を読み直した生の値で、
/// 読み直せなければ Rumoca の（不正な値を捨てた後の）値。
pub(crate) fn experiment_for_model(result: &CompilationResult, model: &str) -> Option<Experiment> {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        raw_experiment(&result.dae, model).unwrap_or_else(|| rumoca_experiment(result))
    }));
    caught.ok().flatten()
}

fn extract_inner(
    result: &CompilationResult,
    model: Option<&str>,
) -> Result<PreflightStructure, String> {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let dae = &result.dae;
        let mut preflight = dae.inspect(|view| evaluate(dae, view))?;
        preflight.experiment = model
            .and_then(|m| raw_experiment(dae, m))
            .unwrap_or_else(|| rumoca_experiment(result));
        Ok(preflight)
    }));
    match caught {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "プリフライトの入力を取り出す途中で panic した: {}",
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        )),
    }
}

/// 1 つの変数の評価の結果（失敗の位置を、依存元との重複の判定に使う）。
struct Failed {
    variable: u32,
    kind: FailureKind,
    message: String,
    /// 評価器の失敗の位置（ソースの番号、始まり、終わり）
    at: (u64, usize, usize),
}

fn evaluate<'dae>(dae: &Dae, view: DaeView<'dae>) -> Result<PreflightStructure, String> {
    let mut preflight = PreflightStructure::default();

    // パラメータと定数が参照するパラメータと定数（変数の添字）。束縛（無ければ start）の式から
    let mut cache = ScalarCoordinateProjectionCache::default();
    let mut references: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut referenced = |variable: VariableView<'dae>, expression| -> Option<Vec<u32>> {
        let mut out = Vec::new();
        for scalar in 0..variable.scalar_count() {
            for_each_scalar_coordinate_cached(
                view,
                expression,
                scalar,
                None,
                &mut cache,
                |coordinate, _| {
                    if let CoordinateView::Parameter(p) = coordinate {
                        out.push(p.index());
                    }
                },
            )
            .ok()?;
        }
        out.sort_unstable();
        out.dedup();
        Some(out)
    };

    // fixed = false のパラメータ（初期化の未知数）と、それに（推移的に）依存する変数
    let mut free: HashSet<u32> = HashSet::new();
    for (id, variable) in view.variables() {
        let role = variable.role();
        if matches!(role, VariableRole::Parameter | VariableRole::Constant) {
            if role == VariableRole::Parameter
                && (0..variable.scalar_count()).any(|s| variable.fixed_scalar(s) == Some(false))
            {
                free.insert(id.index());
            }
            if let Some(expression) = variable.binding().or(variable.start())
                && let Some(refs) = referenced(variable, expression)
            {
                references.insert(id.index(), refs);
            }
        }
    }
    let tainted = reaching(&references, &free);

    let mut evaluator = NumericEvaluator::new(view);
    let mut parameter_failures: Vec<Failed> = Vec::new();
    let mut start_failures: Vec<Failed> = Vec::new();
    for (id, variable) in view.variables() {
        let role = variable.role();
        let count = variable.scalar_count();
        let is_parameter = match role {
            VariableRole::Parameter | VariableRole::Constant => true,
            VariableRole::State
            | VariableRole::Algebraic
            | VariableRole::Output
            | VariableRole::DiscreteReal
            | VariableRole::DiscreteValue => false,
            VariableRole::Input => continue,
        };
        // String は数値ではない（評価器は UnsupportedOperation を返す。MSL の表の `tableName` など）
        if count == 0 || variable.value_type().scalar_type() == ScalarType::String {
            continue;
        }
        if is_parameter {
            // fixed = false のスカラーは初期化で決まる。一部だけ fixed = false の配列も、まとめて評価しない
            if free.contains(&id.index()) || tainted.contains(&id.index()) {
                continue;
            }
            preflight.parameters_evaluated += count;
        } else {
            // 束縛や start が fixed = false のパラメータを参照する start も、初期化の前には決まらない
            if let Some(expression) = variable.start().or(variable.binding())
                && referenced(variable, expression).is_some_and(|refs| {
                    refs.iter().any(|r| free.contains(r) || tainted.contains(r))
                })
            {
                continue;
            }
            preflight.starts_evaluated += count;
        }
        let Err(error) = evaluator.initial_value(id) else {
            continue;
        };
        let Some(kind) = failure_kind(&error, is_parameter) else {
            continue;
        };
        let failed = Failed {
            variable: id.index(),
            kind,
            message: message(dae, &error),
            at: (
                error.span().source.0,
                error.span().start.0,
                error.span().end.0,
            ),
        };
        if is_parameter {
            parameter_failures.push(failed);
        } else {
            start_failures.push(failed);
        }
    }

    // 依存元の失敗をそのまま受け継いだもの（種類と位置が同じ）は、元の 1 つにまとめる
    let roots: HashSet<(u32, (u64, usize, usize))> = parameter_failures
        .iter()
        .map(|f| (f.variable, f.at))
        .collect();
    let inherited = |f: &Failed| {
        references.get(&f.variable).is_some_and(|refs| {
            refs.iter()
                .any(|r| *r != f.variable && roots.contains(&(*r, f.at)))
        })
    };
    let to_failure = |f: &Failed| -> Option<EvaluationFailure> {
        let variable = view.variables().find(|(id, _)| id.index() == f.variable)?.1;
        let (file, range) = location(dae, variable.declaration());
        Some(EvaluationFailure {
            name: if variable.scalar_count() == 1 {
                variable
                    .scalar_name(0)
                    .unwrap_or_else(|| variable.name().to_string())
            } else {
                variable.name().to_string()
            },
            kind: f.kind,
            message: f.message.clone(),
            file,
            range,
        })
    };
    let start_from_parameter = |f: &Failed| {
        // start の式がパラメータの失敗をそのまま受け継いだもの（パラメータの側で報告する）
        roots.iter().any(|(_, at)| *at == f.at)
    };
    preflight.parameter_failures = parameter_failures
        .iter()
        .filter(|f| !inherited(f))
        .filter_map(to_failure)
        .collect();
    preflight.start_failures = start_failures
        .iter()
        .filter(|f| !start_from_parameter(f))
        .filter_map(to_failure)
        .collect();
    Ok(preflight)
}

/// `free` から、参照を逆に辿って届く変数（`free` そのものは含まない）。
fn reaching(references: &HashMap<u32, Vec<u32>>, free: &HashSet<u32>) -> HashSet<u32> {
    let mut reached: HashSet<u32> = HashSet::new();
    if free.is_empty() {
        return reached;
    }
    loop {
        let mut changed = false;
        for (variable, refs) in references {
            if free.contains(variable) || reached.contains(variable) {
                continue;
            }
            if refs.iter().any(|r| free.contains(r) || reached.contains(r)) {
                reached.insert(*variable);
                changed = true;
            }
        }
        if !changed {
            return reached;
        }
    }
}

/// 評価器の失敗の種類を振り分ける。失敗にしないもの（start の値が無い、実行時の値を参照する）は None。
fn failure_kind(error: &NumericEvaluationError, is_parameter: bool) -> Option<FailureKind> {
    use NumericEvaluationErrorKind as K;
    Some(match error.kind() {
        K::InvalidValue => {
            let text = error.to_string();
            if text.contains("non-finite") || text.contains("divisor must be nonzero") {
                FailureKind::NotFinite
            } else {
                FailureKind::Other
            }
        }
        K::AssertionFailed => FailureKind::Assertion,
        K::ExternalFunction => FailureKind::External,
        // start の既定は 0（MLS §4.8）。パラメータの値が無いのは S8 の GM-W8003 が受け持つ
        K::MissingValue if is_parameter => FailureKind::Missing,
        K::MissingValue => return None,
        // start が実行時の値を参照する（静的な値ではない）
        K::NonStaticCoordinate if !is_parameter => return None,
        K::CyclicDependency
        | K::NonStaticCoordinate
        | K::UnsupportedOperation
        | K::ShapeMismatch
        | K::OutOfBounds
        | K::Overflow
        | K::InvalidOverride => FailureKind::Other,
    })
}

/// 評価器の言葉に、失敗した式の本文（1 行、60 文字まで）を添える。
///
/// assert の失敗の位置は `assert` の語だけを指すので、文の終わり（次の `;`）まで延ばす。
fn message(dae: &Dae, error: &NumericEvaluationError) -> String {
    let span = error.span();
    let excerpt = (!span.is_dummy())
        .then(|| dae.source_map().get_source(span.source))
        .flatten()
        .and_then(|(_, text)| {
            let end = if error.kind() == NumericEvaluationErrorKind::AssertionFailed {
                text.get(span.end.0..)
                    .and_then(|rest| rest.find(';'))
                    .map_or(span.end.0, |i| span.end.0 + i)
            } else {
                span.end.0
            };
            text.get(span.start.0..end)
        })
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty());
    match excerpt {
        Some(t) if t.chars().count() > EXCERPT_CHARS => {
            let cut: String = t.chars().take(EXCERPT_CHARS - 1).collect();
            format!("{error}: `{cut}…`")
        }
        Some(t) => format!("{error}: `{t}`"),
        None => error.to_string(),
    }
}

/// Rumoca の（不正な値を捨てた後の）experiment。値が 1 つも無ければ None（無いのと区別できない）。
fn rumoca_experiment(result: &CompilationResult) -> Option<Experiment> {
    let e = Experiment {
        start_time: result.experiment_start_time.map(Bits::new),
        stop_time: result.experiment_stop_time.map(Bits::new),
        interval: result.experiment_interval.map(Bits::new),
        tolerance: result.experiment_tolerance.map(Bits::new),
        non_literal: Vec::new(),
        file: None,
        range: None,
    };
    (e != Experiment::default()).then_some(e)
}

/// 根のクラスの annotation から、experiment の生の値を取る。クラスが見つからなければ None
/// （呼び手は Rumoca の値で代える）。見つかって experiment が無ければ Some(None)。
fn raw_experiment(dae: &Dae, model: &str) -> Option<Option<Experiment>> {
    let (file, text) = defining_source(dae, model)?;
    let parsed = rumoca_compile::parsing::parse_source_to_ast_with_errors(text, file).ok()?;
    let class = find_class(&parsed, model)?;
    let mut found: Option<Experiment> = None;
    for expression in &class.annotation {
        if let Some((modifications, span)) = experiment_modifications(expression) {
            let e = found.get_or_insert_with(Experiment::default);
            for m in modifications {
                apply_entry(m, e);
            }
            let (start, end) = (span.start.0, span.end.0);
            e.file = Some(file.to_string());
            e.range = (start < end && end <= text.len())
                .then(|| TextRange::new(start as u32, end as u32));
        }
    }
    Some(found)
}

/// `model` を定義していそうなファイル（名前、本文）。ファイル名（`package.mo` ならディレクトリ名）が
/// 名前のどれかの区切りと同じで、クラスの短い名前を含むものを先に、次に短い名前を含むものを試す。
fn defining_source<'a>(dae: &'a Dae, model: &str) -> Option<(&'a str, &'a str)> {
    let segments: Vec<&str> = model.split('.').collect();
    let short = *segments.last()?;
    let map = dae.source_map();
    let mut sources: Vec<(&str, &str)> = map
        .source_ids()
        .into_values()
        .filter_map(|id| map.get_source(id))
        .filter(|(_, text)| text.contains(short))
        .collect();
    sources.sort_unstable();
    let by_path = |name: &str| {
        let path = std::path::Path::new(name);
        let stem = path.file_stem().and_then(|s| s.to_str());
        let stem = if stem == Some("package") {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                // MSL のディレクトリは `Modelica 4.1.0` のように版が付く
                .map(|s| s.split(' ').next().unwrap_or(s))
        } else {
            stem
        };
        stem.is_some_and(|s| segments.contains(&s))
    };
    let (likely, rest): (Vec<_>, Vec<_>) = sources.into_iter().partition(|(n, _)| by_path(n));
    likely.into_iter().chain(rest).find(|(file, text)| {
        rumoca_compile::parsing::parse_source_to_ast_with_errors(text, file)
            .ok()
            .is_some_and(|parsed| find_class(&parsed, model).is_some())
    })
}

/// 構文木の中から、完全な名前が `model` のクラス。
fn find_class<'a>(parsed: &'a ast::StoredDefinition, model: &str) -> Option<&'a ast::ClassDef> {
    let within: Vec<&str> = parsed
        .within
        .as_ref()
        .map(|w| w.name.iter().map(|t| t.text.as_ref()).collect())
        .unwrap_or_default();
    let segments: Vec<&str> = model.split('.').collect();
    let rest = segments.strip_prefix(within.as_slice())?;
    let (first, inner) = rest.split_first()?;
    let mut class = parsed.classes.get(*first)?;
    for segment in inner {
        class = class.classes.get(*segment)?;
    }
    Some(class)
}

/// annotation の 1 項目が `experiment(...)` なら、その中身と位置。
fn experiment_modifications(
    expression: &ast::Expression,
) -> Option<(&[ast::Expression], rumoca_core::Span)> {
    match expression {
        ast::Expression::ClassModification {
            target,
            modifications,
            span,
            ..
        } if last_ident(target) == Some("experiment") => Some((modifications.as_slice(), *span)),
        ast::Expression::FunctionCall {
            comp, args, span, ..
        } if last_ident(comp) == Some("experiment") => Some((args.as_slice(), *span)),
        ast::Expression::NamedArgument { name, value, .. }
            if name.text.as_ref() == "experiment" =>
        {
            experiment_modifications(value)
        }
        ast::Expression::Modification { target, value, .. }
            if last_ident(target) == Some("experiment") =>
        {
            experiment_modifications(value)
        }
        _ => None,
    }
}

fn last_ident(reference: &ast::ComponentReference) -> Option<&str> {
    reference.parts.last().map(|p| p.ident.text.as_ref())
}

/// `experiment(...)` の 1 項目を読む。数の字面でない値は None にし、名前を `non_literal` に残す。
/// 名前は大文字と小文字を区別する（Rumoca は区別しないが、MLS の文法どおりにする）。
fn apply_entry(entry: &ast::Expression, e: &mut Experiment) {
    let (key, value) = match entry {
        ast::Expression::NamedArgument { name, value, .. } => (name.text.as_ref(), value.as_ref()),
        ast::Expression::Modification { target, value, .. } => {
            let Some(key) = last_ident(target) else {
                return;
            };
            (key, value.as_ref())
        }
        _ => return,
    };
    let (key, slot) = match key {
        "StartTime" => ("StartTime", &mut e.start_time),
        "StopTime" => ("StopTime", &mut e.stop_time),
        "Interval" => ("Interval", &mut e.interval),
        "Tolerance" => ("Tolerance", &mut e.tolerance),
        _ => return,
    };
    *slot = number(value).map(Bits::new);
    e.non_literal.retain(|k| k != key);
    if slot.is_none() {
        e.non_literal.push(key.to_string());
    }
}

/// 数の字面（符号と括弧を含む）。
fn number(expression: &ast::Expression) -> Option<f64> {
    match expression {
        ast::Expression::Terminal {
            terminal_type: ast::TerminalType::UnsignedReal | ast::TerminalType::UnsignedInteger,
            token,
            ..
        } => token.text.parse::<f64>().ok(),
        ast::Expression::Unary { op, rhs, .. } => match op {
            rumoca_core::OpUnary::Minus => number(rhs).map(|v| -v),
            rumoca_core::OpUnary::Plus => number(rhs),
            _ => None,
        },
        ast::Expression::Parenthesized { inner, .. } => number(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rumoca_compile::{Session, SessionConfig};

    use super::*;

    fn compile(name: &str, text: &str) -> PreflightStructure {
        let mut session = Session::new(SessionConfig::default());
        session.add_document("M.mo", text).unwrap();
        let compilation = session
            .compile_model_strict(name)
            .unwrap_or_else(|_| panic!("{name} compiles"));
        extract_for_model(compilation.result(), name).expect("preflight")
    }

    /// (名前, 種類)
    fn failures(f: &[EvaluationFailure]) -> Vec<(&str, FailureKind)> {
        f.iter().map(|f| (f.name.as_str(), f.kind)).collect()
    }

    fn values(e: &Experiment) -> [Option<f64>; 4] {
        [e.start_time, e.stop_time, e.interval, e.tolerance].map(|v| v.map(Bits::get))
    }

    #[test]
    fn non_finite_parameter_is_reported_once_at_its_root() {
        // q = 2 * r は r の失敗を受け継いだだけなので、r の 1 件だけ
        let p = compile(
            "M",
            "model M parameter Real a = -1; parameter Real r = sqrt(a); parameter Real q = 2 * r;
             Real x(start = 0, fixed = true); equation der(x) = q - x; end M;",
        );
        assert_eq!(p.parameters_evaluated, 3);
        assert_eq!(
            failures(&p.parameter_failures),
            [("r", FailureKind::NotFinite)]
        );
        let f = &p.parameter_failures[0];
        assert!(f.message.contains("non-finite"), "{}", f.message);
        assert!(f.message.contains("`sqrt(a)`"), "{}", f.message);
        assert!(p.start_failures.is_empty());
        assert_eq!(p.starts_evaluated, 1);
    }

    #[test]
    fn non_finite_start_is_reported() {
        let p = compile(
            "M",
            "model M parameter Real x0 = 0; Real x(start = log(x0), fixed = true);
             equation der(x) = -x; end M;",
        );
        assert!(p.parameter_failures.is_empty());
        assert_eq!(failures(&p.start_failures), [("x", FailureKind::NotFinite)]);
    }

    #[test]
    fn start_inheriting_a_parameter_failure_is_not_repeated() {
        // x の start は r の値を使うだけ。失敗は r の側で 1 件
        let p = compile(
            "M",
            "model M parameter Real r = sqrt(-1); Real x(start = r, fixed = true);
             equation der(x) = -x; end M;",
        );
        assert_eq!(
            failures(&p.parameter_failures),
            [("r", FailureKind::NotFinite)]
        );
        assert!(p.start_failures.is_empty(), "{:?}", p.start_failures);
    }

    #[test]
    fn function_assertion_is_an_assertion_failure() {
        let p = compile(
            "M",
            r#"model M
                 function f input Real v; output Real r;
                 algorithm assert(v >= 0, "negative"); r := sqrt(v); end f;
                 function w input Real v; output Real r;
                 algorithm assert(v >= 0, "negative", AssertionLevel.warning); r := v; end w;
                 parameter Real d = f(-1);
                 parameter Real e = w(-1);
                 Real x(start = 0, fixed = true);
               equation der(x) = d + e - x; end M;"#,
        );
        // level = warning の assert は値に関わらない（MLS §8.3.7）
        assert_eq!(
            failures(&p.parameter_failures),
            [("d", FailureKind::Assertion)]
        );
        let m = &p.parameter_failures[0].message;
        assert!(m.contains("assert(v >= 0"), "{m}");
    }

    #[test]
    fn external_function_is_not_static() {
        let p = compile(
            "M",
            r#"model M
                 function f input Real u; output Real y; external "C" y = sin(u); end f;
                 parameter Real g = f(1);
                 Real x(start = 0, fixed = true);
               equation der(x) = g - x; end M;"#,
        );
        assert_eq!(
            failures(&p.parameter_failures),
            [("g", FailureKind::External)]
        );
    }

    #[test]
    fn free_parameters_and_their_dependents_are_not_evaluated() {
        // u0（fixed = false）の start は既定の 0。k = 1 / u0 をそれで評価すると 0 除算になってしまう
        let p = compile(
            "M",
            "model M parameter Real u0(fixed = false); parameter Real k = 1 / u0; parameter Real T = 2;
             Real x(start = 2, fixed = true); Real z(start = k);
             initial equation der(x) = 0; equation der(x) = u0 - x; z = k * x; end M;",
        );
        assert_eq!(p.parameters_evaluated, 1, "T だけ");
        assert!(
            p.parameter_failures.is_empty(),
            "{:?}",
            p.parameter_failures
        );
        assert_eq!(
            p.starts_evaluated, 1,
            "x だけ（z の start は k を参照する）"
        );
        assert!(p.start_failures.is_empty());
    }

    #[test]
    fn missing_values_are_not_failures() {
        // 値の無いパラメータ（S8 の GM-W8003 が受け持つ）と、start の無い変数（既定の 0）
        let p = compile(
            "M",
            "model M parameter Real T; Real x(fixed = true); Real y; equation T * der(x) = 1 - x; y = x; end M;",
        );
        // Rumoca は値の無いパラメータにも既定の start（0）を補うので、評価は失敗しない（Missing にならない）
        assert!(
            p.parameter_failures.is_empty(),
            "{:?}",
            p.parameter_failures
        );
        assert_eq!(p.parameters_evaluated, 1);
        assert!(p.start_failures.is_empty(), "{:?}", p.start_failures);
        assert_eq!(p.starts_evaluated, 2);
    }

    #[test]
    fn array_parameter_failure_names_the_variable() {
        let p = compile(
            "M",
            "model M parameter Real a[2] = {1, -1}; parameter Real r[2] = sqrt(a);
             Real x(start = 0, fixed = true); equation der(x) = r[1] - x; end M;",
        );
        assert_eq!(p.parameters_evaluated, 4);
        assert_eq!(
            failures(&p.parameter_failures),
            [("r", FailureKind::NotFinite)]
        );
    }

    #[test]
    fn raw_experiment_keeps_values_rumoca_drops() {
        // Rumoca は StopTime < StartTime の StopTime と、Tolerance ≤ 0・Interval ≤ 0 を捨てる
        let text = "model M Real x(start = 1, fixed = true); equation der(x) = -x;
             annotation(experiment(StartTime = 1, StopTime = 0.5, Interval = -0.1, Tolerance = 0)); end M;";
        let mut session = Session::new(SessionConfig::default());
        session.add_document("M.mo", text).unwrap();
        let compilation = session
            .compile_model_strict("M")
            .unwrap_or_else(|_| panic!());
        let result = compilation.result();
        assert_eq!(result.experiment_stop_time, None);
        assert_eq!(result.experiment_interval, None);
        assert_eq!(result.experiment_tolerance, None);

        let raw = extract_for_model(result, "M").unwrap().experiment.unwrap();
        assert_eq!(values(&raw), [Some(1.0), Some(0.5), Some(-0.1), Some(0.0)]);
        let range = raw.range.unwrap();
        assert!(text[range.start as usize..range.end as usize].starts_with("experiment("));
        assert_eq!(raw.file.as_deref(), Some("M.mo"));

        // モデルの名前が無ければ Rumoca の値（位置は無い）
        let rumoca = extract(result).unwrap().experiment.unwrap();
        assert_eq!(values(&rumoca), [Some(1.0), None, None, None]);
        assert_eq!(rumoca.range, None);
    }

    #[test]
    fn missing_experiment_is_none() {
        let p = compile(
            "M",
            "model M Real x(start = 1, fixed = true); equation der(x) = -x; annotation(Documentation(info = \"x\")); end M;",
        );
        assert_eq!(p.experiment, None);
    }

    #[test]
    fn experiment_of_a_nested_class_and_non_literal_values() {
        // within とパッケージの入れ子を辿る。式（2 * 3）の値は None（experiment はある）
        let text = "within Lib; package Sub
               model Other annotation(experiment(StopTime = 9)); end Other;
               model M Real x(start = 1, fixed = true); equation der(x) = -x;
                 annotation(experiment(StopTime = 2 * 3, Tolerance = -1e-6)); end M;
             end Sub;";
        let mut session = Session::new(SessionConfig::default());
        session
            .add_document("Lib/package.mo", "package Lib end Lib;")
            .unwrap();
        session.add_document("Lib/Sub.mo", text).unwrap();
        let compilation = session
            .compile_model_strict("Lib.Sub.M")
            .unwrap_or_else(|_| panic!());
        let e = extract_for_model(compilation.result(), "Lib.Sub.M")
            .unwrap()
            .experiment
            .unwrap();
        assert_eq!(values(&e), [None, None, None, Some(-1e-6)]);
        assert_eq!(e.non_literal, ["StopTime"]);
        assert_eq!(e.file.as_deref(), Some("Lib/Sub.mo"));
    }

    #[test]
    fn validation_package_experiments_are_read_raw() {
        // src/Modelica/GlobeModeValidation（ディレクトリのパッケージ）。ファイル名で根のクラスのファイルを探す
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../Modelica/GlobeModeValidation");
        let mut session = Session::new(SessionConfig::default());
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "mo") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    session
                        .add_document(&p.display().to_string(), &text)
                        .unwrap();
                }
            }
        }
        let cases: [(&str, Option<[Option<f64>; 4]>); 5] = [
            ("StopBeforeStart", Some([Some(1.0), Some(0.5), None, None])),
            (
                "InvalidExperiment",
                Some([None, Some(1.0), Some(-0.1), Some(0.0)]),
            ),
            (
                "SuspiciousExperiment",
                Some([None, Some(1.0), Some(5.0), Some(0.5)]),
            ),
            (
                "Valid",
                Some([Some(0.0), Some(2.0), Some(0.01), Some(1e-6)]),
            ),
            ("NoExperiment", None),
        ];
        for (model, expected) in cases {
            let name = format!("GlobeModeValidation.Preflight.{model}");
            let compilation = session
                .compile_model_strict(&name)
                .unwrap_or_else(|_| panic!("{name} compiles"));
            let e = extract_for_model(compilation.result(), &name)
                .unwrap()
                .experiment;
            assert_eq!(e.as_ref().map(values), expected, "{model}");
            if let Some(e) = e {
                assert!(e.file.unwrap().ends_with(&format!("{model}.mo")), "{model}");
                assert!(e.range.is_some());
            }
        }
    }

    #[test]
    fn string_parameters_are_skipped() {
        let p = compile(
            "M",
            r#"model M parameter String name = "x"; parameter Real k = 1;
               Real x(start = 0, fixed = true); equation der(x) = k - x; end M;"#,
        );
        assert_eq!(p.parameters_evaluated, 1);
        assert!(
            p.parameter_failures.is_empty(),
            "{:?}",
            p.parameter_failures
        );
    }
}

//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイルだけ。外へは中立 IR と GlobeMode の診断だけを出す。

use std::path::Path;

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{
    BalanceCount, Causality, ClassKind, ClassOutline, ComponentOutline, Connect, ConnectPrefix,
    DaeCounts, Outline,
};
use rumoca_compile::compile::{Dae, FailedPhase, PhaseResult, SourceRootKind, VariableRole};
use rumoca_compile::parsing::{ParseError, ast};
use rumoca_compile::{Session, SessionConfig};

use crate::{CheckReport, Frontend, Source};

/// Phase A のフロントエンド。
#[derive(Debug, Clone, Default)]
pub struct RumocaFrontend {
    _private: (),
}

impl RumocaFrontend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Frontend for RumocaFrontend {
    fn outline(&self, source: Source<'_>) -> Result<Outline, Vec<Diagnostic>> {
        match rumoca_compile::parsing::parse_source_to_ast_with_errors(source.text, source.name) {
            Ok(def) => Ok(Outline {
                within: def.within.as_ref().map(ToString::to_string),
                classes: def.classes.values().map(class_outline).collect(),
            }),
            Err(errors) => Err(errors
                .iter()
                .map(|e| parse_diagnostic(source.name, e))
                .collect()),
        }
    }

    fn check(&self, sources: &[Source<'_>], libraries: &[&Path], model: &str) -> CheckReport {
        let mut report = CheckReport {
            model: model.to_string(),
            balance: None,
            dae: None,
            diagnostics: Vec::new(),
        };

        // S0: 先にファイルごとに構文を見る。誤りがあればそこで止める（段階ごとに止める約束）。
        for s in sources {
            if let Err(errors) =
                rumoca_compile::parsing::parse_source_to_ast_with_errors(s.text, s.name)
            {
                report
                    .diagnostics
                    .extend(errors.iter().map(|e| parse_diagnostic(s.name, e)));
            }
        }
        if !report.passed() {
            return report;
        }

        let mut session = Session::new(SessionConfig::default());

        // ライブラリ（MSL など）。めったに変わらないので DurableExternal として読む。
        // Rumoca は解析結果をキャッシュするので、2 回目からは速い。
        for lib in libraries {
            let load = session.load_source_root_tolerant(
                &format!("library:{}", lib.display()),
                SourceRootKind::DurableExternal,
                lib,
                None,
            );
            if !load.diagnostics.is_empty() || load.parsed_file_count == 0 {
                let mut d = Diagnostic::error(
                    codes::LIBRARY,
                    Stage::S1,
                    format!("ライブラリ {} を読み込めない", lib.display()),
                );
                for msg in load.diagnostics.iter().take(5) {
                    d = d.note(msg.clone());
                }
                report.diagnostics.push(d);
                return report;
            }
        }

        for s in sources {
            if let Err(e) = session.add_document(s.name, s.text) {
                report.diagnostics.push(
                    Diagnostic::error(codes::RESOLVE, Stage::S1, e.to_string()).at(s.name, None),
                );
                return report;
            }
        }

        // 候補を出すための事前確認。ユーザーのファイルだけのときに限る。
        // model_names() は、ライブラリ（MSL）の解決時の警告（WR006 など）でも Err を返すので、
        // Err のときは止めずにコンパイルに進む（見つからなければ compile が失敗を返す）。
        if libraries.is_empty()
            && let Ok(names) = session.model_names()
            && !names.iter().any(|n| n == model)
        {
            let mut d = Diagnostic::error(
                codes::MODEL_NOT_FOUND,
                Stage::S1,
                format!("モデル `{model}` が見つからない"),
            );
            if !names.is_empty() {
                d = d.note(format!("候補: {}", names.join(", ")));
            }
            report.diagnostics.push(d);
            return report;
        }

        // Rumoca の CLI と同じ strict-reachable のコンパイル。MSL を解決したときの警告では失敗しない。
        match session.compile_model_strict(model) {
            Ok(compilation) => {
                let b = &compilation.result().balance_detail;
                let balance = BalanceCount {
                    equations: b.equations(),
                    unknowns: b.unknowns(),
                    states: b.state_unknowns,
                    algebraics: b.algebraic_unknowns,
                    outputs: b.output_unknowns,
                    discrete: b.discrete_real_unknowns + b.discrete_value_unknowns,
                };
                if !balance.is_balanced() {
                    report.diagnostics.push(Diagnostic::error(
                        codes::UNBALANCED,
                        Stage::S5,
                        format!(
                            "方程式が {} 個、変数が {} 個で、数が合わない",
                            balance.equations, balance.unknowns
                        ),
                    ));
                }
                report.balance = Some(balance);
                report.dae = Some(dae_counts(&compilation.result().dae));
            }
            Err(strict) => {
                let d = match &strict.requested_result {
                    Some(PhaseResult::NeedsInner { missing_inners, .. }) => Diagnostic::error(
                        codes::MISSING_INNER,
                        Stage::S3,
                        format!(
                            "outer に対応する inner が無い: {}",
                            missing_inners.join(", ")
                        ),
                    ),
                    Some(PhaseResult::Failed {
                        phase,
                        error,
                        error_code,
                        ..
                    }) => phase_diagnostic(Some(*phase), error_code.as_deref(), error.clone()),
                    // 要求したモデル自体ではなく、依存先のクラスで失敗したとき
                    _ => match strict.failures.first() {
                        Some(f) => {
                            phase_diagnostic(f.phase, f.error_code.as_deref(), f.error.clone())
                                .note(format!("失敗したクラス: {}", f.model_name))
                        }
                        None => {
                            Diagnostic::error(codes::RESOLVE, Stage::S1, strict.failure_summary(5))
                        }
                    },
                };
                let more = strict.failures.len().saturating_sub(1);
                report.diagnostics.push(if more > 0 {
                    d.note(format!("ほかに {more} 件の失敗がある"))
                } else {
                    d
                });
            }
        }
        report
    }
}

/// DAE の変数を役割ごとに数える（Rumoca の CLI の `dae_counts` と同じ数え方）。
fn dae_counts(dae: &Dae) -> DaeCounts {
    dae.inspect(|view| {
        let mut c = DaeCounts {
            continuous_equations: view.continuous_equation_count(),
            ..DaeCounts::default()
        };
        for (_, v) in view.variables() {
            match v.role() {
                VariableRole::State => c.states += 1,
                VariableRole::Algebraic => c.algebraics += 1,
                VariableRole::Output => c.outputs += 1,
                VariableRole::Input => c.inputs += 1,
                VariableRole::Parameter => c.parameters += 1,
                VariableRole::Constant
                | VariableRole::DiscreteReal
                | VariableRole::DiscreteValue => {}
            }
        }
        c
    })
}

/// Rumoca の失敗段階とエラーコードを、GlobeMode の検査段階と GM コードに対応づける（ADR-0002）。
fn phase_diagnostic(
    phase: Option<FailedPhase>,
    error_code: Option<&str>,
    error: String,
) -> Diagnostic {
    let (code, stage) = match (phase, error_code) {
        // rumoca の ED001 は「方程式数と変数数が合わない」
        (Some(FailedPhase::ToDae), Some("ED001")) => (codes::UNBALANCED, Stage::S5),
        (Some(FailedPhase::Instantiate), _) => (codes::INSTANTIATE, Stage::S3),
        (Some(FailedPhase::Typecheck), _) => (codes::TYPECHECK, Stage::S2),
        (Some(FailedPhase::Flatten), _) => (codes::FLATTEN, Stage::S4),
        (Some(FailedPhase::ToDae), _) => (codes::TO_DAE, Stage::S5),
        (None, _) => (codes::RESOLVE, Stage::S1),
    };
    let d = Diagnostic::error(code, stage, error);
    match error_code {
        Some(c) => d.note(format!("rumoca: {c}")),
        None => d,
    }
}

fn parse_diagnostic(file: &str, e: &ParseError) -> Diagnostic {
    let span = e.span();
    let range = (!span.is_dummy()).then(|| TextRange::new(span.start.0 as u32, span.end.0 as u32));
    let message = match e {
        ParseError::SyntaxError { message, .. } => message.clone(),
        ParseError::NoAstProduced { .. } => "構文木を作れなかった".to_string(),
        ParseError::IoError { path, message, .. } => format!("{path}: {message}"),
    };
    Diagnostic::error(codes::SYNTAX, Stage::S0, message).at(file, range)
}

fn class_outline(c: &ast::ClassDef) -> ClassOutline {
    let connects = c
        .equations
        .iter()
        .filter_map(|eq| match eq {
            ast::Equation::Connect { lhs, rhs } => Some(Connect {
                lhs: lhs.to_string(),
                rhs: rhs.to_string(),
                range: TextRange::new(lhs.span.start.0 as u32, rhs.span.end.0 as u32),
            }),
            _ => None,
        })
        .collect();

    ClassOutline {
        name: c.name.text.to_string(),
        kind: class_kind(c.class_type.as_str()),
        partial: c.partial,
        causality: causality(&c.causality),
        range: TextRange::new(c.location.start, c.location.end),
        extends: c.extends.iter().map(|e| e.base_name.to_string()).collect(),
        components: c
            .components
            .values()
            .map(|comp| ComponentOutline {
                name: comp.name.clone(),
                type_name: comp.type_name.to_string(),
                prefix: match comp.connection {
                    ast::Connection::Flow(_) => ConnectPrefix::Flow,
                    ast::Connection::Stream(_) => ConnectPrefix::Stream,
                    ast::Connection::Empty => ConnectPrefix::Potential,
                },
                causality: causality(&comp.causality),
                range: TextRange::new(comp.name_token.location.start, comp.name_token.location.end),
            })
            .collect(),
        connects,
        classes: c.classes.values().map(class_outline).collect(),
    }
}

/// rumoca の Causality は公開されていないので、Debug の表記で見分ける（`Input(..)` / `Output(..)` / `Empty`）。
fn causality(c: &impl std::fmt::Debug) -> Causality {
    let s = format!("{c:?}");
    if s.starts_with("Input") {
        Causality::Input
    } else if s.starts_with("Output") {
        Causality::Output
    } else {
        Causality::None
    }
}

fn class_kind(keyword: &str) -> ClassKind {
    // rumoca の ClassType は公開されていないので、`as_str()` のキーワードで対応づける。
    match keyword {
        "class" => ClassKind::Class,
        "block" => ClassKind::Block,
        "connector" => ClassKind::Connector,
        "record" => ClassKind::Record,
        "type" => ClassKind::Type,
        "package" => ClassKind::Package,
        "function" => ClassKind::Function,
        "operator" => ClassKind::Operator,
        _ => ClassKind::Model,
    }
}

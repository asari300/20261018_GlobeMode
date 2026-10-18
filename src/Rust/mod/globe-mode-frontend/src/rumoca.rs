//! Rumoca（Apache-2.0、https://github.com/CogniPilot/rumoca）のアダプタ。
//!
//! Rumoca の型を使ってよいのはこのファイルだけ。外へは中立 IR と GlobeMode の診断だけを出す。

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{BalanceCount, ClassKind, ClassOutline, ComponentOutline, Connect, Outline};
use rumoca_compile::compile::{FailedPhase, PhaseResult};
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

    fn check(&self, sources: &[Source<'_>], model: &str) -> CheckReport {
        let mut report = CheckReport {
            model: model.to_string(),
            balance: None,
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
        for s in sources {
            if let Err(e) = session.add_document(s.name, s.text) {
                report.diagnostics.push(
                    Diagnostic::error(codes::RESOLVE, Stage::S1, e.to_string()).at(s.name, None),
                );
                return report;
            }
        }

        match session.model_names() {
            Ok(names) if !names.iter().any(|n| n == model) => {
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
            Err(e) => {
                report.diagnostics.push(Diagnostic::error(
                    codes::RESOLVE,
                    Stage::S1,
                    e.to_string(),
                ));
                return report;
            }
            Ok(_) => {}
        }

        match session.compile_model_phases(model) {
            Ok(PhaseResult::Success(result)) => {
                let b = &result.balance_detail;
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
            }
            Ok(PhaseResult::NeedsInner { missing_inners, .. }) => {
                report.diagnostics.push(Diagnostic::error(
                    codes::MISSING_INNER,
                    Stage::S3,
                    format!(
                        "outer に対応する inner が無い: {}",
                        missing_inners.join(", ")
                    ),
                ));
            }
            Ok(PhaseResult::Failed {
                phase,
                error,
                error_code,
                ..
            }) => {
                let (code, stage) = match (phase, error_code.as_deref()) {
                    // rumoca の ED001 は「方程式数と変数数が合わない」
                    (FailedPhase::ToDae, Some("ED001")) => (codes::UNBALANCED, Stage::S5),
                    (FailedPhase::Instantiate, _) => (codes::INSTANTIATE, Stage::S3),
                    (FailedPhase::Typecheck, _) => (codes::TYPECHECK, Stage::S2),
                    (FailedPhase::Flatten, _) => (codes::FLATTEN, Stage::S4),
                    (FailedPhase::ToDae, _) => (codes::TO_DAE, Stage::S5),
                };
                let mut d = Diagnostic::error(code, stage, error);
                if let Some(c) = error_code {
                    d = d.note(format!("rumoca: {c}"));
                }
                report.diagnostics.push(d);
            }
            Err(e) => {
                report
                    .diagnostics
                    .push(Diagnostic::error(codes::RESOLVE, Stage::S1, e.to_string()))
            }
        }
        report
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
        range: TextRange::new(c.location.start, c.location.end),
        extends: c.extends.iter().map(|e| e.base_name.to_string()).collect(),
        components: c
            .components
            .values()
            .map(|comp| ComponentOutline {
                name: comp.name.clone(),
                type_name: comp.type_name.to_string(),
                range: TextRange::new(comp.name_token.location.start, comp.name_token.location.end),
            })
            .collect(),
        connects,
        classes: c.classes.values().map(class_outline).collect(),
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

//! 自前のフロントエンド（feature `native`、S305。Track B の最初の検査へのつなぎ）。
//!
//! 自前の parse → resolve → flat → dae（`globe-mode-syntax`・`-resolve`・`-flat`・`-dae`）で、
//! 構文・名前解決・平坦化・釣り合い（S5）と構造の正則（S6）まで検査する。S7 以降は検査しない。
//! 自前で断ったもの（未対応）は、Rumoca に切り替えず `GM-U0006` で返す。

use std::collections::HashSet;
use std::path::Path;

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_flat::Project;
use globe_mode_ir::Outline;
use globe_mode_resolve::{Reason, RefKind, Resolution};

use crate::split_library::SplitLibrary;
use crate::{CheckReport, Frontend, Source, blank_bom};

/// 名前の未解決を何件まで診断にするか（ライブラリの渡し忘れで全部が未解決になっても溢れさせない）
const MAX_UNRESOLVED: usize = 20;

#[derive(Debug, Default)]
pub struct NativeFrontend {
    _private: (),
}

impl NativeFrontend {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `model` と `user_texts` が使うライブラリのファイルだけの（名前, 本文）。Rumoca の道の分割読み込み
/// （[`SplitLibrary`]、ADR-0025）と同じ閉包（外側の `package.mo` と、クラスの見つからないファイルを含む）。
/// 全部読む `Project::from_dirs` と平坦化の結果が変わらないことは `tests/native_closure.rs`（`#[ignore]`）で見る。
pub fn closure_sources(
    libraries: &[&Path],
    model: &str,
    user_texts: &[&str],
) -> std::io::Result<Vec<(String, String)>> {
    let mut split = SplitLibrary::new(Project::read_dirs(libraries)?);
    let mut need = split.closure(model, user_texts.iter().copied());
    need.extend(split.unindexed());
    Ok(need
        .into_iter()
        .map(|n| {
            let (name, text) = split.file(n);
            (name.to_string(), text.to_string())
        })
        .collect())
}

fn syntax_diagnostics(sources: &[Source<'_>]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for s in sources {
        for e in &globe_mode_syntax::parse(&blank_bom(s.text)).errors {
            let mut d = Diagnostic::error(codes::SYNTAX, Stage::S0, e.message.clone());
            d.file = Some(s.name.to_string());
            d.range = Some(TextRange::new(e.range.start().into(), e.range.end().into()));
            out.push(d);
        }
    }
    out
}

fn unsupported(stage: Stage, what: &str) -> Diagnostic {
    Diagnostic::error(
        codes::NATIVE_UNSUPPORTED,
        stage,
        format!("native では未対応: {what}"),
    )
}

impl Frontend for NativeFrontend {
    /// 構文の誤りだけ返す。アウトラインを作る道は自前の段に無い（LSP は `globe-mode-syntax` を直接読む）ので、
    /// 読めるファイルは未対応（`GM-U0006`）にする。
    fn outline(&self, source: Source<'_>) -> Result<Outline, Vec<Diagnostic>> {
        let mut d = syntax_diagnostics(&[source]);
        if d.is_empty() {
            d.push(unsupported(
                Stage::S0,
                "アウトライン（`outline` は Rumoca の道だけ）",
            ));
        }
        Err(d)
    }

    fn check(&self, sources: &[Source<'_>], libraries: &[&Path], model: &str) -> CheckReport {
        let mut report = CheckReport::new(model);
        // S0: ライブラリを読む前に、入力のファイルの誤りを全部（位置つきで）出す
        report.diagnostics = syntax_diagnostics(sources);
        if !report.passed() {
            return report;
        }
        let texts: Vec<_> = sources
            .iter()
            .map(|s| (s.name, blank_bom(s.text)))
            .collect();
        let extra: Vec<_> = texts.iter().map(|(n, t)| (*n, &**t)).collect();
        let user_texts: Vec<&str> = extra.iter().map(|(_, t)| *t).collect();
        // 開いて直したライブラリのファイルは作業中の本文が真（ディスクの版と二重に入れない。S320・#113）
        let same = |a: &str, b: &str| a.replace('\\', "/") == b.replace('\\', "/");
        let project = match closure_sources(libraries, model, &user_texts).map(|lib| {
            Project::from_sources(
                lib.iter()
                    .filter(|(n, _)| !extra.iter().any(|(m, _)| same(n, m)))
                    .map(|(n, t)| (n.as_str(), t.as_str()))
                    .chain(extra.iter().copied()),
            )
        }) {
            Ok(p) => p,
            Err(e) => {
                report.diagnostics.push(Diagnostic::error(
                    codes::LIBRARY,
                    Stage::S1,
                    format!("ライブラリを読めない: {e}"),
                ));
                return report;
            }
        };
        let Some(cid) = project.lib.class_by_name(model) else {
            report.diagnostics.push(Diagnostic::error(
                codes::MODEL_NOT_FOUND,
                Stage::S1,
                format!("モデル `{model}` が見つからない"),
            ));
            return report;
        };

        // S1: モデルから届くクラス（入れ子・`extends`・部品の型）の参照の解決
        let mut by_scope = vec![Vec::new(); project.lib.classes.len()];
        for (i, r) in project.lib.refs.iter().enumerate() {
            by_scope[r.scope].push(i);
        }
        let (mut seen, mut stack, mut total) = (HashSet::new(), vec![cid], 0);
        while let Some(c) = stack.pop() {
            if !seen.insert(c) {
                continue;
            }
            stack.extend(project.lib.classes[c].classes.values().copied());
            for &i in &by_scope[c] {
                let r = &project.lib.refs[i];
                let mut d = match project.lib.resolve(r) {
                    Resolution::Class(b) if matches!(r.kind, RefKind::Type | RefKind::Extends) => {
                        stack.push(b);
                        continue;
                    }
                    // 継承元の失敗の結果。元の参照が別に出る
                    Resolution::Unresolved(Reason::Inherited) => continue,
                    Resolution::Unresolved(Reason::Redeclare) => unsupported(
                        Stage::S1,
                        &format!(
                            "`{}`（`redeclare` / `replaceable` が関わる名前）",
                            r.parts.join(".")
                        ),
                    ),
                    Resolution::Unresolved(why) => Diagnostic::error(
                        codes::RESOLVE,
                        Stage::S1,
                        format!("名前 `{}` を解決できない（{why:?}）", r.parts.join(".")),
                    ),
                    _ => continue,
                };
                total += 1;
                if total <= MAX_UNRESOLVED {
                    d.file = project.lib.files.get(r.file as usize).cloned();
                    d.range = Some(TextRange::new(r.range.start().into(), r.range.end().into()));
                    report.diagnostics.push(d);
                }
            }
        }
        if total > MAX_UNRESOLVED {
            report
                .notes
                .push(format!("未解決の名前はほか {} 件", total - MAX_UNRESOLVED));
        }
        if !report.passed() {
            return report;
        }

        // S4: 平坦化、S5: DAE。断った理由は `kind: detail`
        let flat = match project.flatten(model) {
            Ok(f) => f,
            Err(e) => {
                report
                    .diagnostics
                    .push(unsupported(Stage::S4, &e.to_string()));
                return report;
            }
        };
        let dae = match globe_mode_dae::build(&flat) {
            Ok(d) => d,
            Err(e) => {
                report
                    .diagnostics
                    .push(unsupported(Stage::S5, &e.to_string()));
                return report;
            }
        };
        report.balance = Some(dae.counts);
        if !dae.is_balanced() {
            let mut d = Diagnostic::error(
                codes::UNBALANCED,
                Stage::S5,
                format!(
                    "方程式が {} 個、変数が {} 個で、数が合わない",
                    dae.counts.equations, dae.counts.unknowns
                ),
            );
            // 原因の目安: 多い側の、マッチングで相手のいない方程式（宣言・文の位置）か未知数（宣言の位置）
            let st = &dae.structure;
            let s6 = globe_mode_analysis::analyze(st);
            let cause = if dae.counts.equations > dae.counts.unknowns {
                s6.unmatched_equations
                    .first()
                    .and_then(|&i| st.equations.get(i))
                    .map(|r| ("方程式", r.label.clone(), r.file.clone(), r.range))
            } else {
                s6.unmatched_unknowns
                    .first()
                    .and_then(|&i| st.unknowns.get(i))
                    .map(|u| ("未知数", u.name.clone(), u.file.clone(), u.range))
            };
            if let Some((kind, name, file, range)) = cause {
                d = d.note(format!("対応のない{kind}の 1 つ: `{name}`"));
                if let Some(f) = file {
                    d = d.at(f, range);
                }
            }
            report.diagnostics.push(d);
        } else {
            // S6（釣り合っているときだけ。db の検査と同じ）
            let s6 = globe_mode_analysis::analyze(&dae.structure);
            report.diagnostics.extend(s6.diagnostics(&dae.structure));
            report.notes.push(format!(
                "S6: 完全マッチング {} / {}（native。S7 以降は検査していない）",
                s6.structural_matching, s6.n_equations
            ));
        }
        report.structure = Some(dae.structure);
        report
    }
}

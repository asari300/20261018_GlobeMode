//! `globe check` の人間向けの表示のうち、S7〜S10 の行（`main.rs` から行を移しただけ。S199）。

use globe_mode::db::{ExperimentVerdict, FullReport};

/// S7 の行（構造インデックスの見込みと、状態選択の候補）。
pub(crate) fn print_s7(report: &FullReport) {
    let Some(s7) = &report.index else {
        return;
    };
    if s7.failed {
        println!(
            "{}: S7: Pantelides 法を打ち切った（構造インデックスを決められない）",
            report.model
        );
        return;
    }
    if s7.index <= 1 {
        println!(
            "{}: S7: 構造インデックス {}（微分なし）",
            report.model, s7.index
        );
        return;
    }
    let max = s7.differentiations.iter().max().copied().unwrap_or(0);
    // 状態選択の候補は、GM-I7002 と同じく「動的な状態 < 状態の数」のときだけ出す
    let unknowns = report.check.structure.as_ref().map(|s| &s.unknowns);
    let states = unknowns.map_or(0, |u| {
        u.iter()
            .filter(|u| u.kind == globe_mode_ir::UnknownKind::Derivative)
            .count()
    });
    let selection = if s7.dynamic_states < states {
        let names: Vec<String> = s7
            .dummy_candidates
            .iter()
            .map(|&i| {
                unknowns
                    .and_then(|u| u.get(i))
                    .map_or_else(|| format!("#{i}"), |u| state_name(&u.name).to_string())
            })
            .collect();
        format!(
            "。動的な状態 {} / {states}、状態選択の候補: {}",
            s7.dynamic_states,
            names.join(", ")
        )
    } else {
        String::new()
    };
    println!(
        "{}: S7: 構造インデックス {} の見込み（微分した方程式 {} 本、最大 {} 回）{selection}",
        report.model, s7.index, s7.n_differentiated, max
    );
}

/// `der(x[2])` → `x[2]`（状態選択の候補は状態の名前で出す）。
fn state_name(unknown: &str) -> &str {
    unknown
        .strip_prefix("der(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(unknown)
}

/// S8 の行（初期条件の数と、過剰・不足）。
pub(crate) fn print_s8(report: &FullReport) {
    let Some(s8) = &report.init else {
        return;
    };
    let init = report.check.init.as_ref();
    let initial_equations = init.map_or(0, |i| i.initial_equations.len());
    let states = init.map_or(0, |i| i.states.len());
    let discretes = init.map_or(0, |i| i.discretes.len());
    // fixed でない離散変数の分の不足は確定しない（analysis::init の柔らかい警告と同じ条件）
    let open_discretes = init.map_or(0, |i| i.discretes.iter().filter(|d| !d.fixed).count());
    let fixed_discretes = discretes - open_discretes;
    let fixed_algebraics = init.map_or(0, |i| i.algebraics.iter().filter(|a| a.fixed).count());
    // 過剰が確定するのは、離散変数が無く、インデックスの低減で状態が減っていないときだけ（analysis::init と同じ条件）
    let index_reduced = report
        .index
        .as_ref()
        .is_some_and(|s7| !s7.failed && s7.dynamic_states < states);
    let balance = match s8.provided.cmp(&s8.required) {
        _ if s8.matching.is_some() => String::new(),
        std::cmp::Ordering::Equal => String::new(),
        std::cmp::Ordering::Greater if discretes > 0 || index_reduced => format!(
            "。{} 個多い見込み（離散変数かインデックスの低減のため確定できない）",
            s8.provided - s8.required
        ),
        std::cmp::Ordering::Greater => {
            format!("。{} 個多い（過剰）", s8.provided - s8.required)
        }
        std::cmp::Ordering::Less if s8.required - s8.provided <= open_discretes => format!(
            "。{} 個足りない見込み（離散変数の pre の初期値に start を使えば足りる）",
            s8.required - s8.provided
        ),
        std::cmp::Ordering::Less => format!(
            "。{} 個足りない（start 値を推定値として使う見込み）",
            s8.required - s8.provided
        ),
    };
    // 要る数は「動的な状態（S7）+ fixed = false のパラメータ」なので、S7 の数を添える
    let dynamic = report
        .index
        .as_ref()
        .filter(|s7| !s7.failed)
        .map_or_else(String::new, |s7| {
            format!("動的な状態 {} / ", s7.dynamic_states)
        });
    println!(
        "{}: S8: 初期条件 {} / {}（{dynamic}状態 {states}・離散 {discretes}・初期方程式 {}・fixed = true の状態 {}・離散 {fixed_discretes}・代数 {fixed_algebraics}・fixed = false のパラメータ {}）{balance}",
        report.model,
        s8.provided,
        s8.required,
        initial_equations,
        s8.fixed_states.len(),
        s8.free_parameters.len()
    );
    // S009: 初期化系のマッチング（冗長な初期条件と、初期条件の無い未知数）
    if let Some(m) = &s8.matching {
        let verdict = match (m.extra, m.missing) {
            (0, 0) => "合う".to_string(),
            (e, 0) => format!("冗長 {e}"),
            (0, u) => format!("決まらない {u}"),
            (e, u) => format!("冗長 {e}・決まらない {u}"),
        };
        println!(
            "{}: S8: 初期化系のマッチング: 方程式 {}（DAE とその微分、初期方程式、fixed = true）/ 未知数 {}、マッチング {}。{verdict}{}",
            report.model,
            m.n_equations,
            m.n_unknowns,
            m.matching,
            if m.extra > 0 && m.over_with_dae {
                "（冗長な初期条件は拘束と重なる。整合する値なら冗長なだけ）"
            } else {
                ""
            }
        );
    } else if report.check.init.as_ref().is_some_and(|i| !i.has_incidence) {
        println!(
            "{}: S8: 初期方程式の参照を取り出せなかったので、数と属性だけで判定した",
            report.model
        );
    }
}

/// S9 の行（代数ループの数と大きさ、非線形のループの反復変数）。
pub(crate) fn print_s9(report: &FullReport) {
    let Some(s9) = &report.loops else {
        return;
    };
    if let Some(reason) = &s9.skipped {
        println!("{}: S9: 代数ループを解析しなかった: {reason}", report.model);
        return;
    }
    let reduced = if s9.reduced {
        "。インデックス低減の最高階の系で数えた（状態選択の前の見込み）"
    } else {
        ""
    };
    if s9.n_loops() == 0 {
        println!(
            "{}: S9: 代数ループなし（BLT のブロック {}）{reduced}",
            report.model, s9.blocks
        );
        return;
    }
    let start = if s9.start_checked {
        format!("（start 値なし {}）", s9.n_iteration_without_start())
    } else {
        String::new()
    };
    println!(
        "{}: S9: 代数ループ {}（線形 {}・非線形 {}、最大 {} 本の方程式）、BLT のブロック {}。非線形のループの反復変数 {}{start}{reduced}",
        report.model,
        s9.n_loops(),
        s9.n_linear(),
        s9.n_nonlinear(),
        s9.max_loop_size(),
        s9.blocks,
        s9.n_iteration_variables()
    );
}

/// S10 の行（評価したパラメータと start 値の数、失敗の数、experiment annotation）。
pub(crate) fn print_s10(report: &FullReport) {
    let Some(s10) = &report.preflight else {
        return;
    };
    let failures = s10.not_finite_parameters.len()
        + s10.assertion_parameters.len()
        + s10.not_finite_starts.len();
    let external = if s10.external_parameters.is_empty() {
        String::new()
    } else {
        format!(
            "、静的に評価できないもの {}（外部関数）",
            s10.external_parameters.len()
        )
    };
    let experiment = match &s10.experiment {
        ExperimentVerdict::Missing => "experiment annotation なし".to_string(),
        ExperimentVerdict::Checked {
            invalid,
            suspicious,
        } if invalid.is_empty() && suspicious.is_empty() => {
            "experiment annotation は妥当".to_string()
        }
        ExperimentVerdict::Checked {
            invalid,
            suspicious,
        } => format!(
            "experiment annotation の誤り {}・疑わしい値 {}",
            invalid.len(),
            suspicious.len()
        ),
    };
    println!(
        "{}: S10: パラメータ {}・start 値 {} を評価し、有限にならないか assert が失敗したもの {}{external}。{experiment}",
        report.model, s10.parameters_evaluated, s10.starts_evaluated, failures
    );
}

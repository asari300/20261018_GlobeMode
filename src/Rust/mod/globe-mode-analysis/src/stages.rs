//! 変換の段ごとの要約（S280。変換デバッガの最初の一切れ）。
//!
//! flat → DAE → インデックスの低減（Pantelides 法）→ BLT → tearing の各段の数と、名前が取れる差分を並べる。
//! 新しい解析はしない。既存の結果（`FlatSummary`・`DaeCounts`・`DaeStructure`・`IndexReport`・`BltReport`・
//! `LoopReport`）を 1 つにまとめるだけ。flat → DAE は数だけ（Rumoca の DAE は別名を畳まない。変数名の対応は取れない）。
//!
//! Rumoca に依存しない。wasm32 でも動く。

use globe_mode_ir::{BalanceCount, DaeCounts, DaeStructure, FlatSummary, UnknownKind};
use serde::Serialize;

use crate::loops::{BltReport, LoopReport, column_name};
use crate::pantelides::IndexReport;

/// 段 1: flat モデル。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlatStage {
    pub variables: usize,
    pub equations: usize,
    pub initial_equations: usize,
}

/// 段 2: DAE。`variables` は連続の状態・代数・出力・入力・パラメータの宣言の数（配列は 1 個）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DaeStage {
    pub states: usize,
    pub algebraics: usize,
    pub outputs: usize,
    pub inputs: usize,
    pub parameters: usize,
    pub variables: usize,
    /// `der` の数（状態の微分の未知数。スカラー）
    pub derivatives: usize,
    /// 方程式 × 未知数の系（スカラー）
    pub equations: usize,
    pub unknowns: usize,
    /// flat からの増減（DAE − flat）。別名・定数の畳み込みや離散変数の別枠の合計で、名前は取れない
    pub variables_delta: i64,
    pub equations_delta: i64,
}

/// 段 3: インデックスの低減。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexStage {
    pub index: usize,
    pub failed: bool,
    /// 微分した方程式（ラベルと回数）
    pub differentiated: Vec<String>,
    /// 微分で増えた方程式の数（回数の合計）
    pub added_equations: usize,
    /// 微分で増えた未知数（高階の列。`der(der(x))` の形）
    pub added_unknowns: Vec<String>,
    pub dynamic_states: usize,
    /// 状態選択の候補
    pub dummy_candidates: Vec<String>,
}

/// 段 4: BLT。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BltStage {
    pub reduced: bool,
    pub blocks: usize,
    pub algebraic_loops: usize,
    pub max_size: usize,
    /// （ブロックの大きさ, 個数）。大きさの昇順
    pub size_histogram: Vec<(usize, usize)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

/// 段 5: tearing（代数ループ 1 つ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TearingLoop {
    pub size: usize,
    pub nonlinear: bool,
    /// tearing 変数（反復変数）
    pub variables: Vec<String>,
    /// 残差の方程式
    pub residuals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TearingStage {
    pub loops: Vec<TearingLoop>,
    /// 非線形のループの tearing 変数の数（Newton 法の反復変数）
    pub iteration_variables: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StagesReport {
    pub flat: Option<FlatStage>,
    pub dae: Option<DaeStage>,
    pub index: IndexStage,
    pub blt: BltStage,
    pub tearing: TearingStage,
}

fn signed(a: usize, b: usize) -> i64 {
    a as i64 - b as i64
}

/// 段ごとの要約を作る。`structure` / `index` / `blt` / `loops` は同じモデルの結果。
pub fn analyze_stages(
    flat: Option<&FlatSummary>,
    dae: Option<&DaeCounts>,
    balance: Option<&BalanceCount>,
    structure: &DaeStructure,
    index: &IndexReport,
    blt: &BltReport,
    loops: &LoopReport,
) -> StagesReport {
    let label = |e: usize| {
        structure
            .equations
            .get(e)
            .map_or_else(|| format!("#{e}"), |r| r.label.clone())
    };
    let name = |u: usize| {
        structure
            .unknowns
            .get(u)
            .map_or_else(|| format!("#{u}"), |x| x.name.clone())
    };

    let derivatives = structure
        .unknowns
        .iter()
        .filter(|u| u.kind == UnknownKind::Derivative)
        .count();
    let dae_stage = dae.map(|d| {
        let variables = d.states + d.algebraics + d.outputs + d.inputs + d.parameters;
        DaeStage {
            states: d.states,
            algebraics: d.algebraics,
            outputs: d.outputs,
            inputs: d.inputs,
            parameters: d.parameters,
            variables,
            derivatives,
            equations: structure.equations.len(),
            unknowns: structure.unknowns.len(),
            variables_delta: flat.map_or(0, |f| signed(variables, f.variables)),
            equations_delta: flat
                .zip(balance)
                .map_or(0, |(f, b)| signed(b.equations, f.equations)),
        }
    });

    let differentiated = (0..index.differentiations.len())
        .filter(|&e| index.differentiations[e] > 0)
        .map(|e| format!("{} (x{})", label(e), index.differentiations[e]))
        .collect();
    let mut added_unknowns = Vec::new();
    for (u, x) in structure.unknowns.iter().enumerate() {
        let base = u32::from(x.kind == UnknownKind::Derivative);
        for k in base + 1..=index.orders.get(u).copied().unwrap_or(0) {
            added_unknowns.push(column_name(&x.name, x.kind, Some(k)));
        }
    }
    let index_stage = IndexStage {
        index: index.index,
        failed: index.failed,
        differentiated,
        added_equations: index.differentiations.iter().map(|&n| n as usize).sum(),
        added_unknowns,
        dynamic_states: index.dynamic_states,
        dummy_candidates: index.dummy_candidates.iter().map(|&u| name(u)).collect(),
    };

    let mut sizes = std::collections::BTreeMap::new();
    for b in &blt.blocks {
        *sizes.entry(b.size).or_insert(0usize) += 1;
    }
    let blt_stage = BltStage {
        reduced: blt.reduced,
        blocks: blt.blocks.len(),
        algebraic_loops: blt.blocks.iter().filter(|b| b.algebraic_loop).count(),
        max_size: sizes.keys().next_back().copied().unwrap_or(0),
        size_histogram: sizes.into_iter().collect(),
        skipped: blt.skipped.clone(),
    };

    let tearing = TearingStage {
        loops: loops
            .loops
            .iter()
            .map(|l| {
                let column = |u: usize| match structure.unknowns.get(u) {
                    Some(x) => column_name(
                        &x.name,
                        x.kind,
                        l.unknowns
                            .binary_search(&u)
                            .ok()
                            .and_then(|k| l.orders.get(k).copied()),
                    ),
                    None => format!("#{u}"),
                };
                TearingLoop {
                    size: l.size(),
                    nonlinear: l.nonlinear,
                    variables: l.tearing.iter().map(|&u| column(u)).collect(),
                    residuals: l.residuals.iter().map(|&e| label(e)).collect(),
                }
            })
            .collect(),
        iteration_variables: loops.n_iteration_variables(),
    };

    StagesReport {
        flat: flat.map(|f| FlatStage {
            variables: f.variables,
            equations: f.equations,
            initial_equations: f.initial_equations,
        }),
        dae: dae_stage,
        index: index_stage,
        blt: blt_stage,
        tearing,
    }
}

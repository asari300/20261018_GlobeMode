//! S8 の精密化（S009）: 初期化系のマッチング。
//!
//! 数と属性だけの判定（[`crate::init`]）では、数が合っていても「拘束で決まる状態に付けた余計な `fixed = true`」や
//! 「初期条件の無い状態」を見つけられない（MSL の `Fluid.Examples.NonCircularPipes` は、数では「4 個足りない」だが、
//! omc は「冗長な初期方程式 2 本」と言う）。そこで、初期化の時点で成り立つ方程式をすべて集めた二部グラフを作り、
//! 最大マッチングと粗い DM 分解（[`crate::dulmage_mendelsohn`]）で、過剰な部分（冗長な初期条件）と
//! 不足している部分（初期条件の無い未知数）を名指しする。
//!
//! # 初期化系の組み方（Pantelides 1988 の consistent initialization、Pryce の Σ 法と同じ考え）
//!
//! - **未知数（列）**: 未知数の族ごとに階数 0..=top（S7 の [`IndexReport::orders`]）。状態 `x` の族は
//!   `x`、`der(x)`、…、代数 `z` の族は `z`、`der(z)`、…。これに `fixed = false` で束縛の無いパラメータと、
//!   離散変数（`d` と `pre(d)` を 1 つの列にまとめる）を足す。
//! - **方程式（行）**:
//!   - DAE の方程式 e と、それを 1..=d[e] 回微分したもの（d は [`IndexReport::differentiations`]）。
//!     e を j 回微分した行は、元の参照（族 f、階数 o）ごとに階数 o..=o+j を参照する。
//!     DAE は t0 でも成り立ち、その微分も成り立つので、どれも初期化系の方程式になる。
//!     初期化で決めるパラメータへの参照（[`InitStructure::equation_parameter_refs`]）は、微分した行にも残す
//!     （パラメータは定数なので式の形は変わらないが、未知数としては現れ続ける）。
//!   - 束縛が初期化で決めるパラメータに依存するパラメータの束縛（[`InitStructure::parameter_binding_refs`]。
//!     `s.k = c` の形。そのパラメータも列になる）。
//!   - 初期方程式（[`InitStructure::initial_incidence`] などの参照。`der(x)` は階数 1、`x` と `pre(x)` は階数 0）。
//!   - `fixed = true` の状態・代数変数・離散変数の `v = start`（階数 0 の列を 1 つ参照）。
//! - 列 − 行 = 動的な状態（S7）+ 離散変数 + 初期化で決めるパラメータ − 与えた初期条件 = 要る数 − 与えた数
//!   なので、数だけの判定と食い違わない（不足 − 過剰 = 要る数 − 与えた数）。
//!
//! # 読み方
//!
//! - **過剰な部分**（対応の無い行から交互路で届く行と列）: 行が列より `extra` 本多い。この部分にある初期条件
//!   （初期方程式と `fixed = true`）が冗長の候補 [`InitMatching::redundant`]。過剰な部分に DAE の行（拘束や
//!   その微分）が含まれるなら（[`InitMatching::over_with_dae`]）、初期条件が拘束と同じ変数を決めていて、
//!   整合する値なら冗長なだけ（omc は冗長な初期方程式を外して進む）。数値でしか決まらないので Error にしない。
//! - **不足している部分**（対応の無い列から交互路で届く列と行）: 列が行より `missing` 個多い。この部分の未知数
//!   [`InitMatching::undetermined`] には初期条件が無い（start 値を推定値として使う見込み）。
//!
//! 離散変数の列は DAE の行から参照されない（[`DaeStructure`] は離散変数を既知として数える）ので、初期条件の無い
//! 離散変数は常に不足している部分に入る。when の外の式で決まる離散変数には初期条件が要らない（MLS §8.6）が、
//! その区別は持っていないので、[`crate::init`] の柔らかい警告の約束をそのまま使う。
//!
//! Rumoca に依存しない。wasm32 でも動く。

use globe_mode_ir::{DaeStructure, InitStructure, UnknownKind};
use serde::Serialize;

use crate::dm::dulmage_mendelsohn;
use crate::matching::hopcroft_karp;
use crate::pantelides::IndexReport;

/// 初期条件 1 つ（過剰な部分にあるもの）。添字は [`InitStructure`] のもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
pub enum InitCondition {
    /// 初期方程式（`initial_equations` の添字）
    InitialEquation(usize),
    /// `fixed = true` の状態（`states` の添字）
    FixedState(usize),
    /// `fixed = true` の代数変数（`algebraics` の添字）
    FixedAlgebraic(usize),
    /// `fixed = true` の離散変数（`discretes` の添字）
    FixedDiscrete(usize),
}

/// 初期化の未知数 1 つ（不足している部分にあるもの）。添字は [`InitStructure`] のもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InitUnknown {
    /// 状態の族（`states` の添字）の階数 `order`（0 は `x`、1 は `der(x)`、2 以上は高階の微分）
    State { index: usize, order: u32 },
    /// 代数・出力の族（`algebraics` の添字）の階数 `order`（0 は `z`、1 以上は微分）
    Algebraic { index: usize, order: u32 },
    /// 初期化で決めるパラメータ（`parameters` の添字）
    Parameter { index: usize },
    /// 離散変数（`discretes` の添字。`pre(d)`）
    Discrete { index: usize },
}

/// 初期化系のマッチングの結果。位置を持たない。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct InitMatching {
    /// 初期化系の方程式の数（DAE とその微分 + 初期方程式 + `fixed = true`）
    pub n_equations: usize,
    /// 初期化系の未知数の数（族ごとの階数 0..=top + 初期化で決めるパラメータ + 離散変数）
    pub n_unknowns: usize,
    /// 最大マッチングの大きさ
    pub matching: usize,
    /// 過剰な部分の方程式 − 未知数（冗長な初期条件の数）
    pub extra: usize,
    /// 不足している部分の未知数 − 方程式（初期条件の無い未知数の数）
    pub missing: usize,
    /// 過剰な部分にある初期条件（冗長の候補。初期方程式の順、次に fixed の状態・代数・離散の順）
    pub redundant: Vec<InitCondition>,
    /// 過剰な部分に DAE の方程式（拘束やその微分）が含まれる（整合するかは数値で決まる）
    pub over_with_dae: bool,
    /// 不足している部分の未知数（初期条件が無い）
    pub undetermined: Vec<InitUnknown>,
}

impl InitMatching {
    /// 過剰も不足も無い。
    pub fn is_balanced(&self) -> bool {
        self.extra == 0 && self.missing == 0
    }
}

/// 行の種類（内部）。
#[derive(Debug, Clone, Copy)]
enum Row {
    /// DAE の方程式 e を j 回微分したもの
    Dae,
    Condition(InitCondition),
}

/// 初期化系を組んでマッチングする。
///
/// 組めないときは None: `init.has_incidence` が false、S7 が打ち切り、長さが合わない（`structure` と `index` と
/// `init` が同じ DAE から出たものでない）、初期化で決めるパラメータがあるのに DAE の方程式からの参照が無い。
pub fn match_init_system(
    structure: &DaeStructure,
    index: &IndexReport,
    init: &InitStructure,
) -> Option<InitMatching> {
    let n = structure.equations.len();
    let m = structure.unknowns.len();
    let k = init.initial_equations.len();
    if !init.has_incidence
        || index.failed
        || index.differentiations.len() != n
        || index.orders.len() != m
        || structure.incidence.len() != n
        || structure.state_refs.len() != n
        || init.initial_incidence.len() != k
        || init.initial_state_refs.len() != k
        || init.initial_parameter_refs.len() != k
        || init.initial_discrete_refs.len() != k
    {
        return None;
    }
    // 未知数の族 → InitStructure の添字（Derivative は states の順、それ以外は algebraics の順）
    let mut family: Vec<(bool, usize)> = Vec::with_capacity(m);
    let (mut n_states, mut n_algebraics) = (0, 0);
    for u in &structure.unknowns {
        if u.kind == UnknownKind::Derivative {
            family.push((true, n_states));
            n_states += 1;
        } else {
            family.push((false, n_algebraics));
            n_algebraics += 1;
        }
    }
    if n_states != init.states.len() || n_algebraics != init.algebraics.len() {
        return None;
    }
    let base = |u: usize| u32::from(family[u].0);
    let top: Vec<u32> = (0..m).map(|u| index.orders[u].max(base(u))).collect();

    // 列: 族ごとに階数 0..=top、次にパラメータ、次に離散変数
    let mut columns: Vec<InitUnknown> = Vec::new();
    let mut offset: Vec<usize> = Vec::with_capacity(m);
    for u in 0..m {
        offset.push(columns.len());
        let (is_state, i) = family[u];
        for order in 0..=top[u] {
            columns.push(if is_state {
                InitUnknown::State { index: i, order }
            } else {
                InitUnknown::Algebraic { index: i, order }
            });
        }
    }
    let column = |u: usize, order: u32| -> u32 { (offset[u] + order as usize) as u32 };
    let mut parameter_column: Vec<Option<u32>> = vec![None; init.parameters.len()];
    for (i, p) in init.parameters.iter().enumerate() {
        if !p.fixed && !p.has_binding {
            parameter_column[i] = Some(columns.len() as u32);
            columns.push(InitUnknown::Parameter { index: i });
        }
    }
    // 束縛を通して依存するパラメータも列（束縛は下で行にする）
    for (i, _) in &init.parameter_binding_refs {
        let i = *i as usize;
        if i < init.parameters.len() && parameter_column[i].is_none() {
            parameter_column[i] = Some(columns.len() as u32);
            columns.push(InitUnknown::Parameter { index: i });
        }
    }
    let has_free_parameters = parameter_column.iter().any(Option::is_some);
    if has_free_parameters && init.equation_parameter_refs.len() != n {
        return None;
    }
    let discrete_offset = columns.len();
    for i in 0..init.discretes.len() {
        columns.push(InitUnknown::Discrete { index: i });
    }
    let discrete_column = |d: usize| (discrete_offset + d) as u32;

    // 行
    let mut rows: Vec<Row> = Vec::new();
    let mut incidence: Vec<Vec<u32>> = Vec::new();
    let mut push = |row: Row, mut cols: Vec<u32>| {
        cols.sort_unstable();
        cols.dedup();
        rows.push(row);
        incidence.push(cols);
    };
    // DAE の方程式と、その微分
    for e in 0..n {
        let refs: Vec<(usize, u32)> = structure.incidence[e]
            .iter()
            .map(|&u| u as usize)
            .filter(|&u| u < m)
            .map(|u| (u, base(u)))
            .chain(
                structure.state_refs[e]
                    .iter()
                    .map(|&u| u as usize)
                    .filter(|&u| u < m)
                    .map(|u| (u, 0)),
            )
            .collect();
        let parameters: Vec<u32> = init
            .equation_parameter_refs
            .get(e)
            .map(|r| {
                r.iter()
                    .filter_map(|&p| parameter_column.get(p as usize).copied().flatten())
                    .collect()
            })
            .unwrap_or_default();
        for j in 0..=index.differentiations[e] {
            let mut cols = parameters.clone();
            for &(u, o) in &refs {
                for order in o..=(o + j).min(top[u]) {
                    cols.push(column(u, order));
                }
            }
            push(Row::Dae, cols);
        }
    }
    // 初期方程式
    for i in 0..k {
        let mut cols = Vec::new();
        cols.extend(
            init.initial_incidence[i]
                .iter()
                .map(|&u| u as usize)
                .filter(|&u| u < m)
                .map(|u| column(u, base(u))),
        );
        cols.extend(
            init.initial_state_refs[i]
                .iter()
                .map(|&u| u as usize)
                .filter(|&u| u < m)
                .map(|u| column(u, 0)),
        );
        cols.extend(
            init.initial_parameter_refs[i]
                .iter()
                .filter_map(|&p| parameter_column.get(p as usize).copied().flatten()),
        );
        cols.extend(
            init.initial_discrete_refs[i]
                .iter()
                .map(|&d| d as usize)
                .filter(|&d| d < init.discretes.len())
                .map(discrete_column),
        );
        push(Row::Condition(InitCondition::InitialEquation(i)), cols);
    }
    // 束縛（s.k = c）: 依存するパラメータの列と、束縛が参照する未知数のパラメータの列
    for (i, refs) in &init.parameter_binding_refs {
        let Some(col) = parameter_column.get(*i as usize).copied().flatten() else {
            continue;
        };
        let mut cols = vec![col];
        cols.extend(
            refs.iter()
                .filter_map(|&p| parameter_column.get(p as usize).copied().flatten()),
        );
        push(Row::Dae, cols);
    }
    // fixed = true（状態は der(x) の族の階数 0、代数はその族の階数 0、離散はその列）
    let state_family: Vec<usize> = (0..m).filter(|&u| family[u].0).collect();
    let algebraic_family: Vec<usize> = (0..m).filter(|&u| !family[u].0).collect();
    for (i, s) in init.states.iter().enumerate() {
        if s.fixed {
            push(
                Row::Condition(InitCondition::FixedState(i)),
                vec![column(state_family[i], 0)],
            );
        }
    }
    for (i, a) in init.algebraics.iter().enumerate() {
        if a.fixed {
            push(
                Row::Condition(InitCondition::FixedAlgebraic(i)),
                vec![column(algebraic_family[i], 0)],
            );
        }
    }
    for (i, d) in init.discretes.iter().enumerate() {
        if d.fixed {
            push(
                Row::Condition(InitCondition::FixedDiscrete(i)),
                vec![discrete_column(i)],
            );
        }
    }

    let matching = hopcroft_karp(rows.len(), columns.len(), &incidence);
    let dm = dulmage_mendelsohn(&incidence, &matching);
    let extra = dm
        .over_equations
        .len()
        .saturating_sub(dm.over_unknowns.len());
    let missing = dm
        .under_unknowns
        .len()
        .saturating_sub(dm.under_equations.len());
    let mut redundant: Vec<InitCondition> = dm
        .over_equations
        .iter()
        .filter_map(|&r| match rows.get(r) {
            Some(Row::Condition(c)) => Some(*c),
            _ => None,
        })
        .collect();
    redundant.sort_by_key(|c| match *c {
        InitCondition::InitialEquation(i) => (0, i),
        InitCondition::FixedState(i) => (1, i),
        InitCondition::FixedAlgebraic(i) => (2, i),
        InitCondition::FixedDiscrete(i) => (3, i),
    });
    let over_with_dae = dm
        .over_equations
        .iter()
        .any(|&r| matches!(rows.get(r), Some(Row::Dae)));
    let undetermined: Vec<InitUnknown> = dm
        .under_unknowns
        .iter()
        .filter_map(|&c| columns.get(c).copied())
        .collect();
    Some(InitMatching {
        n_equations: rows.len(),
        n_unknowns: columns.len(),
        matching: matching.size,
        extra,
        missing,
        redundant,
        over_with_dae,
        undetermined,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pantelides::analyze_index;
    use crate::testutil::dae;
    use globe_mode_core::TextRange;
    use globe_mode_ir::{EquationOrigin, EquationRow, InitVariable};

    fn variable(name: &str, fixed: bool, has_start: bool) -> InitVariable {
        InitVariable {
            name: name.into(),
            fixed,
            has_start,
            has_binding: false,
            file: Some("M.mo".into()),
            range: Some(TextRange::new(1, 4)),
        }
    }

    fn equation(label: &str) -> EquationRow {
        EquationRow {
            label: label.into(),
            origin: EquationOrigin::Source,
            file: Some("M.mo".into()),
            range: Some(TextRange::new(5, 10)),
        }
    }

    /// 状態と代数変数の名前（`dae` と同じ順）と fixed、初期方程式（ラベル, der(x) と代数の添字, 状態の添字）
    fn init(
        states: &[(&str, bool)],
        algebraics: &[&str],
        initial: &[(&str, &[u32], &[u32])],
    ) -> InitStructure {
        InitStructure {
            states: states.iter().map(|(n, f)| variable(n, *f, true)).collect(),
            algebraics: algebraics
                .iter()
                .map(|n| variable(n, false, false))
                .collect(),
            initial_equations: initial.iter().map(|(l, _, _)| equation(l)).collect(),
            has_incidence: true,
            initial_incidence: initial.iter().map(|(_, i, _)| i.to_vec()).collect(),
            initial_state_refs: initial.iter().map(|(_, _, s)| s.to_vec()).collect(),
            initial_parameter_refs: vec![Vec::new(); initial.len()],
            initial_discrete_refs: vec![Vec::new(); initial.len()],
            ..Default::default()
        }
    }

    #[test]
    fn fixed_state_with_initial_equation_is_a_pure_redundancy() {
        // der(x) = -x; x は fixed = true、initial equation に x = 2 もある
        let s = dae(&["x"], &[], &[("der(x) = -x", &["der(x)", "x"])]);
        let ix = analyze_index(&s);
        let i = init(&[("x", true)], &[], &[("x = 2", &[], &[0])]);
        let m = match_init_system(&s, &ix, &i).expect("組める");
        assert_eq!((m.n_equations, m.n_unknowns, m.matching), (3, 2, 2));
        assert_eq!((m.extra, m.missing), (1, 0));
        assert_eq!(
            m.redundant,
            [
                InitCondition::InitialEquation(0),
                InitCondition::FixedState(0)
            ]
        );
        // DAE の行は過剰な部分に入らない（der(x) で対応づく）: 確かな過剰
        assert!(!m.over_with_dae);
        assert!(m.undetermined.is_empty());
    }

    #[test]
    fn redundant_fixed_on_a_constrained_state_and_a_missing_state() {
        // Init.RedundantAndMissing と同じ形: x は拘束 x = pb で決まる（インデックス 2）が fixed = true、
        // y には初期条件が無い。数は 1 / 1 で合う
        let s = dae(
            &["x", "y"],
            &["u"],
            &[
                ("der(x) = u - y", &["der(x)", "u", "y"]),
                ("x = pb", &["x"]),
                ("der(y) = y - x", &["der(y)", "y", "x"]),
            ],
        );
        let ix = analyze_index(&s);
        assert_eq!((ix.index, ix.dynamic_states), (2, 1), "{ix:?}");
        let i = init(&[("x", true), ("y", false)], &["u"], &[]);
        let m = match_init_system(&s, &ix, &i).expect("組める");
        // 行: DAE 3 + 微分 1 + fixed 1 = 5、列: x 0..=1, y 0..=1, u 0 = 5
        assert_eq!((m.n_equations, m.n_unknowns, m.matching), (5, 5, 4));
        assert_eq!((m.extra, m.missing), (1, 1));
        assert_eq!(m.redundant, [InitCondition::FixedState(0)]);
        assert!(m.over_with_dae, "拘束 x = pb と重なる");
        assert!(
            m.undetermined
                .contains(&InitUnknown::State { index: 1, order: 0 }),
            "{:?}",
            m.undetermined
        );
    }

    #[test]
    fn pendulum_with_two_fixed_states_is_balanced_through_the_extended_system() {
        let s = dae(
            &["x", "y", "vx", "vy"],
            &["lambda"],
            &[
                ("der(x) = vx", &["der(x)", "vx"]),
                ("der(y) = vy", &["der(y)", "vy"]),
                ("der(vx) = lambda * x", &["der(vx)", "lambda", "x"]),
                ("der(vy) = lambda * y - g", &["der(vy)", "lambda", "y"]),
                ("x ^ 2 + y ^ 2 = L ^ 2", &["x", "y"]),
            ],
        );
        let ix = analyze_index(&s);
        assert_eq!((ix.index, ix.dynamic_states), (3, 2), "{ix:?}");
        let i = init(
            &[("x", true), ("y", false), ("vx", true), ("vy", false)],
            &["lambda"],
            &[],
        );
        let m = match_init_system(&s, &ix, &i).expect("組める");
        assert!(m.is_balanced(), "{m:?}");
        assert_eq!(m.n_unknowns - m.n_equations, 0);
        // fixed を x と y にすると、拘束 x^2 + y^2 = L^2 と重なって冗長、vx と vy は決まらない
        let i = init(
            &[("x", true), ("y", true), ("vx", false), ("vy", false)],
            &["lambda"],
            &[],
        );
        let m = match_init_system(&s, &ix, &i).expect("組める");
        assert_eq!((m.extra, m.missing), (1, 1), "{m:?}");
        assert!(m.over_with_dae);
    }

    #[test]
    fn steady_state_initial_equation_on_the_derivative_determines_the_state() {
        // der(x) = -x + u0（u0 は fixed = false のパラメータ）、initial equation der(x) = 0 と x = 1
        let s = dae(&["x"], &[], &[("der(x) = -x + u0", &["der(x)", "x"])]);
        let ix = analyze_index(&s);
        let mut i = init(
            &[("x", false)],
            &[],
            &[("der(x) = 0", &[0], &[]), ("x = 1", &[], &[0])],
        );
        i.parameters = vec![variable("u0", false, true)];
        i.initial_parameter_refs = vec![Vec::new(), Vec::new()];
        // DAE の方程式からパラメータへの参照が無ければ組めない
        assert!(match_init_system(&s, &ix, &i).is_none());
        i.equation_parameter_refs = vec![vec![0]];
        let m = match_init_system(&s, &ix, &i).expect("組める");
        assert!(m.is_balanced(), "{m:?}");
        assert_eq!((m.n_equations, m.n_unknowns), (3, 3));
    }

    #[test]
    fn a_parameter_bound_to_a_free_parameter_is_an_unknown_with_its_binding() {
        // der(x) = -k * x（k は s.k = c の束縛で、c が fixed = false）、initial equation der(x) = -4、x は fixed
        let s = dae(&["x"], &[], &[("der(x) = -k * x", &["der(x)", "x"])]);
        let ix = analyze_index(&s);
        let mut i = init(&[("x", true)], &[], &[("der(x) = -4", &[0], &[])]);
        i.parameters = vec![variable("c", false, true), variable("s.k", true, true)];
        i.parameters[1].has_binding = true;
        i.initial_parameter_refs = vec![Vec::new()];
        i.equation_parameter_refs = vec![vec![1]];
        i.parameter_binding_refs = vec![(1, vec![0])];
        let m = match_init_system(&s, &ix, &i).expect("組める");
        // 行: DAE 1 + 束縛 1 + 初期方程式 1 + fixed 1、列: x 0..=1 + c + s.k
        assert_eq!((m.n_equations, m.n_unknowns), (4, 4));
        assert!(m.is_balanced(), "{m:?}");
        // 束縛が無ければ c は何にも繋がらず、決まらない（InitSpringConstant で起きた形）
        i.parameter_binding_refs.clear();
        i.equation_parameter_refs = vec![Vec::new()];
        let m = match_init_system(&s, &ix, &i).expect("組める");
        assert_eq!((m.extra, m.missing), (1, 1), "{m:?}");
        assert!(
            m.undetermined
                .contains(&InitUnknown::Parameter { index: 0 })
        );
    }

    #[test]
    fn without_incidence_or_with_mismatched_lengths_there_is_no_matching() {
        let s = dae(&["x"], &[], &[("der(x) = -x", &["der(x)", "x"])]);
        let ix = analyze_index(&s);
        let mut i = init(&[("x", true)], &[], &[]);
        i.has_incidence = false;
        assert!(match_init_system(&s, &ix, &i).is_none());
        let mut i = init(&[("x", true)], &[], &[]);
        i.states.push(variable("y", false, false));
        assert!(match_init_system(&s, &ix, &i).is_none());
        let mut failed = ix.clone();
        failed.failed = true;
        assert!(match_init_system(&s, &ix, &init(&[("x", true)], &[], &[])).is_some());
        assert!(match_init_system(&s, &failed, &init(&[("x", true)], &[], &[])).is_none());
    }
}

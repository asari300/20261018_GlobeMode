//! Rumoca の DAE から、初期化系の構造（[`InitStructure`]、S8 の入力）を取り出す（S007。参照は S009）。
//!
//! - 状態: `VariableRole::State` の変数を、変数の順にスカラーごとに 1 つ。`structure.rs` の
//!   `Derivative` の未知数（`der(x)`）と同じ順になる。`fixed` の既定は false（MLS §4.8）。
//! - パラメータ: `VariableRole::Parameter`（`Constant` は除く）。`fixed` の既定は true。束縛の有無も持つ。
//! - 離散変数: `DiscreteReal` / `DiscreteValue`。`fixed` の既定は false。
//! - 代数変数: `Algebraic` / `Output`。`fixed` の既定は false（`fixed = true` なら初期条件が 1 つ増える）。
//! - 初期方程式: `initialization_owners()` を `structure.rs` と同じ要領でスカラーの行に展開する
//!   （Residual は 1 行、構造化された族は定義域の点ごと・本体の式ごとに 1 行）。
//!   Rumoca が初期方程式（と初期アルゴリズム）から取り分ける定義も、対象のスカラーごとに 1 行として足す:
//!   - Real でない `fixed = false` のパラメータを決める式（`initial_parameter_values()`。Rumoca は初期化の
//!     方程式として扱う）
//!   - 離散変数の、初期化の時点の値を決める式や代入（`initial_discrete_values()`）
//! - 初期方程式の参照（S009）: 行ごとに `rumoca_eval_dae::for_each_scalar_coordinate_cached` で依存する
//!   スカラーの座標を列挙し、`structure.rs` と同じ鍵（[`crate::structure::unknowns_and_keys`]）で
//!   [`DaeStructure`](globe_mode_ir::DaeStructure) の未知数の添字に写す。
//!   - `der(x)` と代数（`pre(z)` も）→ `initial_incidence`、状態そのもの `x`（`pre(x)` も）→ `initial_state_refs`
//!   - パラメータ → `initial_parameter_refs`、離散変数 `d` → `initial_discrete_refs`、`pre(d)` →
//!     `initial_pre_discrete_refs`（S010 で分けた）
//!   - 取り分けた定義の行は、対象のスカラーと、値の式の参照。値の式を射影できなければ対象だけ
//!   - `fixed = false` で束縛の無いパラメータ（初期化の未知数）が 1 つでもあれば、束縛がそれに（推移的に）
//!     依存するパラメータも初期化の未知数として `parameter_binding_refs` に残し（`Sub s(k = c)` の `s.k`。
//!     DAE の方程式は `c` ではなく `s.k` を参照する）、DAE の連続の方程式ももう一度歩いて、それらへの参照を
//!     `equation_parameter_refs` に残す（無ければどちらも歩かない）
//!   - 式を 1 つでも射影できなければ `has_incidence = false`（S8 は数と属性だけで判定する）
//! - 離散変数の決まり方（S010、[`discrete_init`]。MLS §8.6）: `has_incidence` のときだけ。
//!   - Boolean・Integer・列挙の離散変数は `discrete_value_owner` の枝（`DiscreteBranchActivation`）で、
//!     離散 Real は `discrete_real_equations()` の `DiscreteRealActivation` で分ける。`Always` の枝は when の外
//!     （[`DiscreteInitKind::Always`]）、`When` の枝は、きっかけの条件が `initial()`（か `{…, initial(), …}` の
//!     `AnyRise`）なら [`DiscreteInitKind::InitialWhen`]、そうでなければ [`DiscreteInitKind::When`]。
//!   - 初期化で働く枝（`Always` と `InitialWhen`）は、決まる離散変数のスカラーごとに 1 行の
//!     [`DiscreteEquation`]（対象と値の式の参照。値の式を射影できなければ対象だけ）。
//!   - 離散 Real の方程式は残差 `d - 値` の形（Rumoca の `generated_residual`）なので、左辺から対象を、
//!     右辺から参照を取る。左辺が離散 Real の座標 1 つに決まらなければ飛ばす（対象は `Unknown` のまま）。
//!   - Rumoca は `initial equation pre(y) = …` も `y` の定義として取り分けるので、本文が `pre(` で始まる定義は
//!     `pre(y)` を決める式とする。`pre(a) = pre(b)` を等号でつないだ k 個の定義（位置が同じ）は k − 1 行にまとめる
//!     （`extract_view` の中の説明）。
//!   - クロックの区間の変数（`clock_ownerships()`。MLS §16）は §8.6 の初期化の外なので `Unknown` にし、行も作らない。
//!   - `pre(d)` の参照（[`DiscreteInit::pre_referenced`]）は、初期方程式・初期化で働く枝・when の条件
//!     （`edge(b)` は `b and not pre(b)`）から集める。DAE の連続の方程式の中の `pre(d)` は見ない（残した近似）。
//!
//! Rumoca v0.10.2 の DAE への変換（`compile_model_strict`）について、テストで確かめたこと:
//!
//! - `fixed = true` の `start` を初期方程式には**変えない**（`initialization_owners()` に現れない。
//!   その変換はシミュレーションの準備（solve）の側にある）。だから S8 は `fixed = true` の状態を
//!   初期方程式とは別に数えればよく、2 重には数えない。
//! - `start()` は、何も書かなくても**ほぼ常に Some**。既定の start（0 など）を補うため:
//!   - 組み込みの型（`Real x;`）は、宣言そのもの（`Real x` の位置）を指す式で補う。
//!   - start の無い型（`type V = Real(unit = "V")`）は、`DaeGeneration::DefaultStart` の式で補う。
//!
//!   そこで「start が書いてある」は、start の式が宣言と同じ位置ではなく、`DefaultStart` でもないこと、とする
//!   （[`explicit_start`]）。型から受け継いだ start（`type V = Real(start = 1)`）、パラメータを使った式
//!   （`start = p`）、外からの修正（`Sub s(x(start = 2))`）は、どれも書いてあるとみなされる。
//! - 束縛のあるパラメータ（`parameter Real p = 2`）は、束縛が start にも写される（MLS §4.4.4）。
//! - 初期アルゴリズムで決める `fixed = false` のパラメータ（`p := 2`）は、束縛のあるパラメータになる。
//! - 初期方程式のうち離散変数を決めるもの（`initial_discrete_values()`）は、`initialization_owners()` に
//!   入らない（連続の変数を読む離散 Real の式は入る）。S8 は離散変数を数えるので、上のとおり足す。
//!
//! 取り出せないときは Err（理由）。panic しても Err にする。

use std::collections::HashMap;

use std::collections::BTreeMap;

use globe_mode_ir::{
    DiscreteEquation, DiscreteInit, DiscreteInitKind, EquationRow, InitStructure, InitVariable,
};
use rumoca_compile::compile::{
    ContinuousOwnerView, CoordinateView, Dae, DaeProvenance, DaeView, DiscreteBranchActivation,
    DiscreteRealActivation, VariableRole, VariableView,
};
use rumoca_eval_dae::{ScalarCoordinateProjectionCache, for_each_scalar_coordinate_cached};
use rumoca_ir_dae::{
    BinaryOperator, ConditionId, ConditionOperation, DaeGeneration, DaeProvenanceOrigin,
    ExpressionOperation, InitializationOwnerView,
};

use crate::structure::{Key, label, location, origin, unknowns_and_keys};

/// 初期化系の構造を取り出す。
pub(crate) fn extract(dae: &Dae) -> Result<InitStructure, String> {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dae.inspect(|view| extract_view(dae, view))
    }));
    match caught {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "初期化系を取り出す途中で panic した: {}",
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        )),
    }
}

/// 1 行の初期方程式の参照（5 つの列の添字。昇順・重複なし）。
#[derive(Debug, Default)]
struct Refs {
    incidence: Vec<u32>,
    state_refs: Vec<u32>,
    parameters: Vec<u32>,
    /// 離散変数そのもの `d`
    discretes: Vec<u32>,
    /// `pre(d)`（S010 で `discretes` から分けた）
    pre_discretes: Vec<u32>,
}

impl Refs {
    /// `other` の参照を足す（同じ離散変数を決める枝をまとめるとき）。
    fn extend(&mut self, other: Refs) {
        self.incidence.extend(other.incidence);
        self.state_refs.extend(other.state_refs);
        self.parameters.extend(other.parameters);
        self.discretes.extend(other.discretes);
        self.pre_discretes.extend(other.pre_discretes);
    }

    fn finish(mut self) -> Self {
        for v in [
            &mut self.incidence,
            &mut self.state_refs,
            &mut self.parameters,
            &mut self.discretes,
            &mut self.pre_discretes,
        ] {
            v.sort_unstable();
            v.dedup();
        }
        self
    }
}

fn extract_view<'dae>(dae: &Dae, view: DaeView<'dae>) -> Result<InitStructure, String> {
    let mut init = InitStructure::default();
    // 未知数の鍵（structure.rs と同じ対応）
    let (_, keys) = unknowns_and_keys(dae, view)?;
    // パラメータと離散変数の鍵（変数の添字、スカラーの添字）→ parameters / discretes の添字
    let mut parameter_keys: HashMap<(u32, u32), u32> = HashMap::new();
    let mut discrete_keys: HashMap<(u32, u32), u32> = HashMap::new();

    // 変数（変数の順、スカラーごと。structure.rs の未知数と同じ順）
    for (id, variable) in view.variables() {
        let role = variable.role();
        let (list, default_fixed) = match role {
            VariableRole::State => (&mut init.states, false),
            VariableRole::Parameter => (&mut init.parameters, true),
            VariableRole::DiscreteReal | VariableRole::DiscreteValue => {
                (&mut init.discretes, false)
            }
            VariableRole::Algebraic | VariableRole::Output => (&mut init.algebraics, false),
            VariableRole::Input | VariableRole::Constant => continue,
        };
        let count = variable.value_type().scalar_count().ok_or_else(|| {
            format!(
                "変数 `{}` の大きさがコンパイル時に決まっていない",
                variable.name()
            )
        })?;
        let (file, range) = location(dae, variable.declaration());
        let has_start = explicit_start(view, variable);
        // 束縛はパラメータだけが意味を持つ（状態・離散・代数は常に false）
        let has_binding = role == VariableRole::Parameter && variable.binding().is_some();
        for scalar in 0..count {
            let scalar_u32 =
                u32::try_from(scalar).map_err(|_| "スカラーの添字が u32 に収まらない")?;
            list.push(InitVariable {
                name: variable
                    .scalar_name(scalar)
                    .unwrap_or_else(|| variable.name().to_string()),
                fixed: variable.fixed_scalar(scalar).unwrap_or(default_fixed),
                has_start,
                has_binding,
                file: file.clone(),
                range,
            });
            let index = (list.len() - 1) as u32;
            match role {
                VariableRole::Parameter => {
                    parameter_keys.insert((id.index(), scalar_u32), index);
                }
                VariableRole::DiscreteReal | VariableRole::DiscreteValue => {
                    discrete_keys.insert((id.index(), scalar_u32), index);
                }
                _ => {}
            }
        }
    }

    // 1 つの式（スカラー）の参照を集める
    let mut cache = ScalarCoordinateProjectionCache::default();
    let mut project =
        |expression, scalar: usize, point: Option<(_, &[i64])>| -> Result<Refs, String> {
            let mut refs = Refs::default();
            for_each_scalar_coordinate_cached(
                view,
                expression,
                scalar,
                point,
                &mut cache,
                |coordinate, scalar| {
                    let Ok(scalar) = u32::try_from(scalar) else {
                        return;
                    };
                    match coordinate {
                        CoordinateView::Derivative(state) => {
                            if let Some(&u) = keys.get(&Key::Derivative(state.index(), scalar)) {
                                refs.incidence.push(u);
                            }
                        }
                        CoordinateView::Algebraic(v) | CoordinateView::PreAlgebraic(v) => {
                            if let Some(&u) = keys.get(&Key::Algebraic(v.index(), scalar)) {
                                refs.incidence.push(u);
                            }
                        }
                        CoordinateView::State(state) | CoordinateView::PreState(state) => {
                            if let Some(&u) = keys.get(&Key::Derivative(state.index(), scalar)) {
                                refs.state_refs.push(u);
                            }
                        }
                        CoordinateView::Parameter(p) => {
                            if let Some(&i) = parameter_keys.get(&(p.index(), scalar)) {
                                refs.parameters.push(i);
                            }
                        }
                        CoordinateView::DiscreteReal(d) => {
                            if let Some(&i) = discrete_keys.get(&(d.index(), scalar)) {
                                refs.discretes.push(i);
                            }
                        }
                        CoordinateView::DiscreteValue(d) => {
                            if let Some(&i) = discrete_keys.get(&(d.index(), scalar)) {
                                refs.discretes.push(i);
                            }
                        }
                        CoordinateView::PreDiscreteReal(d) => {
                            if let Some(&i) = discrete_keys.get(&(d.index(), scalar)) {
                                refs.pre_discretes.push(i);
                            }
                        }
                        CoordinateView::PreDiscreteValue(d) => {
                            if let Some(&i) = discrete_keys.get(&(d.index(), scalar)) {
                                refs.pre_discretes.push(i);
                            }
                        }
                        // 既知の値（入力・time など）や式の内部の束縛
                        _ => {}
                    }
                },
            )
            .map_err(|e| e.to_string())?;
            Ok(refs.finish())
        };

    // 初期方程式（Rumoca の visit_initialization_rows と同じ順）
    let mut equations: Vec<EquationRow> = Vec::new();
    let mut rows: Vec<Refs> = Vec::new();
    let mut has_incidence = true;
    let mut push_row = |provenance: DaeProvenance, suffix: Option<&[i64]>, refs: Option<Refs>| {
        let index = equations.len();
        let (file, range) = location(dae, provenance);
        equations.push(EquationRow {
            label: label(view, provenance, index, suffix),
            origin: origin(provenance),
            file,
            range,
        });
        rows.push(refs.unwrap_or_default());
    };
    for owner in view.initialization_owners() {
        match owner {
            InitializationOwnerView::Residual { equation, .. } => {
                let refs = match project(equation.residual(), 0, None) {
                    Ok(r) => Some(r),
                    Err(_) => {
                        has_incidence = false;
                        None
                    }
                };
                push_row(equation.provenance(), None, refs);
            }
            InitializationOwnerView::Structured { family, .. } => {
                let domain = view
                    .domain(family.domain())
                    .ok_or("構造化された初期方程式の定義域が見つからない")?;
                for point in 0..domain.scalar_count() as usize {
                    let values = domain
                        .structured()
                        .index_tuple_at(point)
                        .map_err(|e| format!("定義域の点を作れない: {e:?}"))?
                        .ok_or("定義域の点が範囲の外")?;
                    let scalar = family
                        .scalar_view()
                        .body_scalar(point, domain.extents())
                        .ok_or("族の本体のスカラーを決められない")?;
                    for expression in family.bodies().iter() {
                        let refs =
                            match project(expression, scalar, Some((family.domain(), &values))) {
                                Ok(r) => Some(r),
                                Err(_) => {
                                    has_incidence = false;
                                    None
                                }
                            };
                        push_row(family.provenance(), Some(&values), refs);
                    }
                }
            }
        }
    }
    // Rumoca が初期化の行から取り分けた定義: Real でない fixed = false のパラメータの値と、
    // 離散変数の初期化の時点の値。どちらも対象のスカラーごとに 1 行（参照は対象 + 値の式）
    let definitions: Vec<_> = view
        .initial_parameter_values()
        .map(|d| (d.target(), d.value(), d.provenance()))
        .chain(
            view.initial_discrete_values()
                .map(|d| (d.target(), d.value(), d.provenance())),
        )
        .collect();
    // `pre(a) = pre(b)` の形の初期方程式を、Rumoca は対象ごとの定義に取り分け、つながった等式をまとめて
    // 定数の値にする（MSL の StateGraph の PartialStep: `pre(newActive) = pre(localActive)` と
    // `pre(oldActive) = pre(localActive)` の 2 本が、localActive・newActive・oldActive の 3 つの定義になり、
    // 位置はどれも 1 本目。localActive の start を使って解いている）。k 個の `pre` を等号でつなぐ式は k − 1 本
    // なので、同じ位置の `pre(` の定義で、対象が同じ部品（名前の最後の `.` より前。部品ごとに同じ式は位置が
    // 同じなので、部品の名前で分ける）のもの k 個を、k 個の参照をまとめた k − 1 行にする（k = 1 ならそのまま）。
    // (由来, スカラー, スカラーの数, 参照, まとめた定義の数)。行は最初の定義の位置に置く
    let mut pending: Vec<(DaeProvenance, usize, usize, Refs, usize)> = Vec::new();
    type MergeKey = (
        Option<String>,
        Option<globe_mode_core::TextRange>,
        usize,
        String,
    );
    let mut merged: HashMap<MergeKey, usize> = HashMap::new();
    for (target, value, provenance) in definitions {
        // Rumoca は `initial equation pre(y) = false` も `y` の定義として取り分ける（`pre` の区別を持たない）。
        // 本文が `pre(` で始まるものは `pre(y)` を決める式とみなす
        let defines_pre = view
            .source_text(provenance)
            .is_some_and(|t| t.trim_start().starts_with("pre("));
        let variable = view
            .variable(target)
            .ok_or("初期化で値を決める変数が見つからない")?;
        let count = variable.value_type().scalar_count().ok_or_else(|| {
            format!(
                "変数 `{}` の大きさがコンパイル時に決まっていない",
                variable.name()
            )
        })?;
        for scalar in 0..count {
            // 値の式を射影できなければ対象だけ（定義は対象を決めるので、構造は保てる）
            let mut refs = project(value, scalar, None).unwrap_or_default();
            let scalar_u32 = u32::try_from(scalar).unwrap_or(u32::MAX);
            if let Some(&i) = parameter_keys.get(&(target.index(), scalar_u32)) {
                refs.parameters.push(i);
            }
            if let Some(&i) = discrete_keys.get(&(target.index(), scalar_u32)) {
                if defines_pre {
                    refs.pre_discretes.push(i);
                } else {
                    refs.discretes.push(i);
                }
            }
            let (file, range) = location(dae, provenance);
            if defines_pre && range.is_some() {
                // 部品（対象の名前の最後の `.` より前）
                let name = variable.name().to_string();
                let instance = name.rsplit_once('.').map_or("", |(p, _)| p).to_string();
                match merged.entry((file, range, scalar, instance)) {
                    std::collections::hash_map::Entry::Occupied(e) => {
                        let group = &mut pending[*e.get()];
                        group.3.extend(refs);
                        group.4 += 1;
                        continue;
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(pending.len());
                    }
                }
            }
            pending.push((provenance, scalar, count, refs, 1));
        }
    }
    for (provenance, scalar, count, refs, k) in pending {
        let point = [scalar as i64 + 1];
        let refs = refs.finish();
        for _ in 0..k.saturating_sub(1).max(1) {
            push_row(
                provenance,
                (count > 1).then_some(&point[..]),
                Some(Refs {
                    incidence: refs.incidence.clone(),
                    state_refs: refs.state_refs.clone(),
                    parameters: refs.parameters.clone(),
                    discretes: refs.discretes.clone(),
                    pre_discretes: refs.pre_discretes.clone(),
                }),
            );
        }
    }
    init.initial_equations = equations;

    // 初期化で決めるパラメータがあれば、束縛を通して依存するパラメータと、DAE の連続の方程式からの参照も要る
    let mut unknown: Vec<bool> = init
        .parameters
        .iter()
        .map(|p| !p.fixed && !p.has_binding)
        .collect();
    let mut equation_parameter_refs: Vec<Vec<u32>> = Vec::new();
    let mut parameter_binding_refs: Vec<(u32, Vec<u32>)> = Vec::new();
    if has_incidence && unknown.iter().any(|&f| f) {
        // 束縛のあるパラメータごとに、束縛が参照するパラメータ（射影できなければ無し）
        let mut bindings: Vec<(u32, Vec<u32>)> = Vec::new();
        for (id, variable) in view.variables() {
            if variable.role() != VariableRole::Parameter {
                continue;
            }
            let Some(binding) = variable.binding() else {
                continue;
            };
            let count = variable.value_type().scalar_count().unwrap_or(0);
            for scalar in 0..count {
                let Some(&i) = parameter_keys.get(&(id.index(), scalar as u32)) else {
                    continue;
                };
                if let Ok(refs) = project(binding, scalar, None) {
                    bindings.push((i, refs.parameters));
                }
            }
        }
        // 固定点: 束縛が初期化の未知数を参照するパラメータも未知数
        loop {
            let mut changed = false;
            for (i, refs) in &bindings {
                let i = *i as usize;
                if !unknown[i] && refs.iter().any(|&r| unknown[r as usize]) {
                    unknown[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for (i, refs) in bindings {
            if unknown[i as usize] {
                let refs: Vec<u32> = refs.into_iter().filter(|&r| unknown[r as usize]).collect();
                parameter_binding_refs.push((i, refs));
            }
        }
        // DAE の連続の方程式（structure.rs と同じ順に歩く）
        let mut push_dae_row = |refs: Result<Refs, String>| match refs {
            Ok(r) => equation_parameter_refs.push(
                r.parameters
                    .into_iter()
                    .filter(|&i| unknown.get(i as usize).copied().unwrap_or(false))
                    .collect(),
            ),
            Err(_) => has_incidence = false,
        };
        for owner in view.continuous_owners() {
            match owner {
                ContinuousOwnerView::Residual { equation, .. } => {
                    push_dae_row(project(equation.residual(), 0, None));
                }
                ContinuousOwnerView::Structured { family, .. } => {
                    let domain = view
                        .domain(family.domain())
                        .ok_or("構造化された方程式の定義域が見つからない")?;
                    for point in 0..domain.scalar_count() as usize {
                        let values = domain
                            .structured()
                            .index_tuple_at(point)
                            .map_err(|e| format!("定義域の点を作れない: {e:?}"))?
                            .ok_or("定義域の点が範囲の外")?;
                        let scalar = family
                            .scalar_view()
                            .body_scalar(point, domain.extents())
                            .ok_or("族の本体のスカラーを決められない")?;
                        for expression in family.bodies().iter() {
                            push_dae_row(project(
                                expression,
                                scalar,
                                Some((family.domain(), &values)),
                            ));
                        }
                    }
                }
            }
        }
    }

    init.has_incidence = has_incidence;
    if has_incidence {
        let initial_pre: Vec<u32> = rows
            .iter()
            .flat_map(|r| r.pre_discretes.iter().copied())
            .collect();
        let (discrete_init, discrete_equations) = discrete_init(
            view,
            init.discretes.len(),
            &discrete_keys,
            &initial_pre,
            &mut project,
        );
        init.discrete_init = discrete_init;
        init.discrete_equations = discrete_equations;
        init.initial_incidence = rows.iter().map(|r| r.incidence.clone()).collect();
        init.initial_state_refs = rows.iter().map(|r| r.state_refs.clone()).collect();
        init.initial_parameter_refs = rows.iter().map(|r| r.parameters.clone()).collect();
        init.initial_discrete_refs = rows.iter().map(|r| r.discretes.clone()).collect();
        init.initial_pre_discrete_refs = rows.iter().map(|r| r.pre_discretes.clone()).collect();
        init.equation_parameter_refs = equation_parameter_refs;
        init.parameter_binding_refs = parameter_binding_refs;
    }
    Ok(init)
}

/// 離散変数ごとの決まり方と、初期化でも成り立つ離散の方程式（S010。モジュールの doc）。
///
/// `initial_pre` は初期方程式が `pre(d)` として参照する離散変数（`discretes` の添字）。
fn discrete_init<'dae>(
    view: DaeView<'dae>,
    n_discretes: usize,
    discrete_keys: &HashMap<(u32, u32), u32>,
    initial_pre: &[u32],
    project: &mut impl FnMut(
        rumoca_ir_dae::ExprId<'dae>,
        usize,
        Option<(rumoca_ir_dae::DomainId<'dae>, &[i64])>,
    ) -> Result<Refs, String>,
) -> (Vec<DiscreteInit>, Vec<DiscreteEquation>) {
    let mut kinds = vec![DiscreteInitKind::Unknown; n_discretes];
    let mut pre_referenced = vec![false; n_discretes];
    // 初期化で働く枝の参照（対象の discretes の添字 → 参照。同じ対象の枝はまとめる）
    let mut active: BTreeMap<u32, Refs> = BTreeMap::new();
    // 同じスカラーに別の決まり方が重なったら、初期化で働くほうを採る（Always > InitialWhen > When）
    let rank = |k: DiscreteInitKind| match k {
        DiscreteInitKind::Unknown => 0,
        DiscreteInitKind::When => 1,
        DiscreteInitKind::InitialWhen => 2,
        DiscreteInitKind::Always => 3,
    };
    let mut define = |target: u32, kind: DiscreteInitKind, refs: Option<Refs>| {
        let t = target as usize;
        if rank(kind) > rank(kinds[t]) {
            kinds[t] = kind;
        }
        if let Some(refs) = refs {
            active.entry(target).or_default().extend(refs);
        }
    };
    let branch_kind = |trigger: ConditionId<'dae>| {
        if initial_trigger(view, trigger) {
            DiscreteInitKind::InitialWhen
        } else {
            DiscreteInitKind::When
        }
    };

    // Boolean・Integer・列挙（owner の枝）
    for i in 0..view.discrete_value_owner_count() {
        let Some(owner) = view
            .discrete_value_owner_id(i)
            .and_then(|id| view.discrete_value_owner(id))
        else {
            continue;
        };
        for (t, target) in owner.targets().iter().enumerate() {
            let count = scalar_count(view, target.index());
            for branch in owner.branches().iter() {
                let kind = match branch.activation() {
                    DiscreteBranchActivation::Always => DiscreteInitKind::Always,
                    DiscreteBranchActivation::When { trigger, .. } => branch_kind(trigger),
                };
                let value = branch.values().get(t).map(|(e, _)| e);
                for scalar in 0..count {
                    let Some(&d) = discrete_keys.get(&(target.index(), scalar as u32)) else {
                        continue;
                    };
                    let refs = (kind != DiscreteInitKind::When).then(|| {
                        // 値の式を射影できなければ対象だけ
                        value
                            .and_then(|e| project(e, scalar, None).ok())
                            .unwrap_or_default()
                    });
                    define(d, kind, refs);
                }
            }
        }
    }
    // 離散 Real（残差 `d - 値`）
    for equation in view.discrete_real_equations() {
        let Some(ExpressionOperation::Binary {
            operator: BinaryOperator::Subtract,
            lhs,
            rhs,
        }) = view.expression(equation.residual()).map(|e| e.operation())
        else {
            continue;
        };
        let kind = match equation.activation() {
            DiscreteRealActivation::Always => DiscreteInitKind::Always,
            DiscreteRealActivation::When { trigger, .. } => branch_kind(trigger),
        };
        // 左辺が座標そのものなら変数の全スカラー、そうでなければ 1 スカラー
        let count = match view.expression(lhs).map(|e| e.operation()) {
            Some(ExpressionOperation::Coordinate(CoordinateView::DiscreteReal(d))) => {
                scalar_count(view, d.index())
            }
            _ => 1,
        };
        for scalar in 0..count {
            let Ok(target) = project(lhs, scalar, None) else {
                continue;
            };
            let [d] = target.discretes[..] else {
                continue;
            };
            let refs = (kind != DiscreteInitKind::When)
                .then(|| project(rhs, scalar, None).unwrap_or_default());
            define(d, kind, refs);
        }
    }

    // クロックの区間の変数（MLS §16）は §8.6 の初期化の外（previous(v) の初期値は start）。決まり方を
    // 分からないものとし（S009 までと同じ扱い）、行も作らない
    for (_, ownership) in view.clock_ownerships() {
        let variable = ownership.variable().index();
        for scalar in 0..scalar_count(view, variable) {
            if let Some(&d) = discrete_keys.get(&(variable, scalar as u32)) {
                kinds[d as usize] = DiscreteInitKind::Unknown;
                active.remove(&d);
            }
        }
    }

    // pre(d) の参照: 初期方程式、初期化で働く枝、when の条件（edge(b) など）
    let mut mark = |d: u32| {
        if let Some(p) = pre_referenced.get_mut(d as usize) {
            *p = true;
        }
    };
    initial_pre.iter().copied().for_each(&mut mark);
    for refs in active.values() {
        refs.pre_discretes.iter().copied().for_each(&mut mark);
    }
    for (_, condition) in view.conditions() {
        let expression = match condition.operation() {
            ConditionOperation::Discrete(e) => e,
            ConditionOperation::Relation(r) => match view.relation(r) {
                Some(r) => r.expression(),
                None => continue,
            },
            _ => continue,
        };
        if let Ok(refs) = project(expression, 0, None) {
            refs.pre_discretes.into_iter().for_each(&mut mark);
        }
    }

    let discrete_init = kinds
        .into_iter()
        .zip(pre_referenced)
        .map(|(kind, pre_referenced)| DiscreteInit {
            kind,
            pre_referenced,
        })
        .collect();
    let equations = active
        .into_iter()
        .map(|(target, refs)| {
            let mut refs = refs;
            refs.discretes.push(target);
            let refs = refs.finish();
            DiscreteEquation {
                target,
                incidence: refs.incidence,
                state_refs: refs.state_refs,
                parameters: refs.parameters,
                discretes: refs.discretes,
                pre_discretes: refs.pre_discretes,
            }
        })
        .collect();
    (discrete_init, equations)
}

/// 変数（添字）のスカラーの数。分からなければ 0。
fn scalar_count(view: DaeView<'_>, variable: u32) -> usize {
    view.variable_id(variable as usize)
        .and_then(|id| view.variable(id))
        .and_then(|v| v.value_type().scalar_count())
        .unwrap_or(0)
}

/// when のきっかけの条件が初期化で働く（MLS §8.6: `when initial() then` か `when {…, initial(), …} then`）。
fn initial_trigger<'dae>(view: DaeView<'dae>, trigger: ConditionId<'dae>) -> bool {
    match view.condition(trigger).map(|c| c.operation()) {
        Some(ConditionOperation::Initial) => true,
        Some(ConditionOperation::AnyRise(a, b)) => {
            initial_trigger(view, a) || initial_trigger(view, b)
        }
        _ => false,
    }
}

/// `start` が書いてある（修飾子か、型の宣言から受け継いだもの）。
///
/// Rumoca は start を書かない変数にも既定の start を補う。補った式は、宣言と同じ位置を指すか
/// （組み込みの型）、`DaeGeneration::DefaultStart` の由来を持つ（start の無い型）ので、それを除く。
fn explicit_start<'dae>(view: DaeView<'dae>, variable: VariableView<'dae>) -> bool {
    let Some(start) = variable.start().and_then(|id| view.expression(id)) else {
        return false;
    };
    let provenance = start.provenance();
    let defaulted = matches!(
        provenance.origin(),
        DaeProvenanceOrigin::Generated(DaeGeneration::DefaultStart)
    ) || provenance.span() == variable.declaration().span();
    !defaulted
}

#[cfg(test)]
mod tests {
    use rumoca_compile::{Session, SessionConfig};

    use super::*;

    fn compile(name: &str, text: &str) -> InitStructure {
        let mut session = Session::new(SessionConfig::default());
        session.add_document("M.mo", text).unwrap();
        let compilation = session
            .compile_model_strict(name)
            .unwrap_or_else(|_| panic!("{name} compiles"));
        extract(&compilation.result().dae).expect("init")
    }

    /// (名前, fixed, has_start, has_binding)
    fn flags(vars: &[InitVariable]) -> Vec<(&str, bool, bool, bool)> {
        vars.iter()
            .map(|v| (v.name.as_str(), v.fixed, v.has_start, v.has_binding))
            .collect()
    }

    #[test]
    fn free_parameter_reached_through_a_binding_is_an_initialization_unknown() {
        // c（fixed = false）は Sub の k = c の束縛を通してしか DAE に現れない。
        // s.k を初期化の未知数にし、束縛 s.k = c を方程式にし、DAE の方程式は s.k を参照する
        let text = "model Q\n  model Sub\n    parameter Real k = 1;\n    Real x(start = 1, fixed = true);\n  equation\n    der(x) = -k * x;\n  end Sub;\n  parameter Real c(fixed = false, start = 2);\n  Sub s(k = c);\ninitial equation\n  der(s.x) = -4;\nend Q;\n";
        let init = compile("Q", text);
        assert!(init.has_incidence);
        let c = init.parameters.iter().position(|p| p.name == "c").unwrap() as u32;
        let k = init
            .parameters
            .iter()
            .position(|p| p.name == "s.k")
            .unwrap() as u32;
        assert_eq!(init.parameter_binding_refs, [(k, vec![c])]);
        assert_eq!(init.equation_parameter_refs, [vec![k]]);
        // initial equation der(s.x) = -4 は der(x)（未知数 0）を参照する
        assert_eq!(init.initial_incidence, [vec![0]]);
    }

    #[test]
    fn fixed_start_is_not_turned_into_an_initial_equation() {
        // Rumoca の DAE への変換は、fixed = true の start を初期方程式にしない（S8 で 2 重に数えない）
        let text =
            "model A\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend A;\n";
        let init = compile("A", text);
        assert_eq!(flags(&init.states), [("x", true, true, false)]);
        assert!(init.initial_equations.is_empty());
        let r = init.states[0].range.expect("宣言の位置");
        assert_eq!(&text[r.start as usize..r.end as usize], "Real x");
    }

    #[test]
    fn default_start_is_not_counted_as_written() {
        // 何も書かなければ、Rumoca が補う既定の start は「書いてある」に数えない
        let text = "model B\n  type U = Real(unit = \"V\");\n  Real x;\n  U y;\n  Real z(start = 0);\nequation\n  der(x) = -x;\n  der(y) = -y;\n  der(z) = -z;\nend B;\n";
        let init = compile("B", text);
        assert_eq!(
            flags(&init.states),
            [
                ("x", false, false, false),
                ("y", false, false, false),
                ("z", false, true, false)
            ]
        );
    }

    #[test]
    fn inherited_and_expression_starts_count_as_written() {
        let text = "model C\n  type V = Real(start = 1);\n  type W = Real(fixed = true, start = 2);\n  model Sub\n    Real a;\n    V b;\n  equation\n    der(a) = -a;\n    der(b) = -b;\n  end Sub;\n  parameter Real p = 2;\n  V x;\n  W w;\n  Real y(start = p);\n  Sub s(a(start = 3));\n  Sub t;\nequation\n  der(x) = -x;\n  der(w) = -w;\n  der(y) = -y;\nend C;\n";
        let init = compile("C", text);
        assert_eq!(
            flags(&init.states),
            [
                ("x", false, true, false),
                ("w", true, true, false),
                ("y", false, true, false),
                ("s.a", false, true, false),
                ("s.b", false, true, false),
                ("t.a", false, false, false),
                ("t.b", false, true, false)
            ]
        );
        assert_eq!(flags(&init.parameters), [("p", true, true, true)]);
    }

    #[test]
    fn parameters_keep_fixed_start_and_binding() {
        let text = "model P\n  parameter Real a;\n  parameter Real b(start = 3);\n  parameter Real c = 1;\n  parameter Real d(fixed = false);\n  parameter Real e(fixed = false, start = 1);\n  constant Real k = 2;\n  Real x(start = 1, fixed = true);\ninitial equation\n  d = 2 * a;\ninitial algorithm\n  e := 2;\nequation\n  der(x) = -(a + b + c + d + e + k) * x;\nend P;\n";
        let init = compile("P", text);
        assert_eq!(
            flags(&init.parameters),
            [
                ("a", true, false, false),
                ("b", true, true, false),
                ("c", true, true, true),
                ("d", false, false, false),
                // 初期アルゴリズムで決めるパラメータは、束縛のあるパラメータになる
                ("e", false, true, true)
            ]
        );
        assert_eq!(init.initial_equations.len(), 1);
        assert_eq!(init.initial_equations[0].label, "d = 2 * a");
    }

    #[test]
    fn non_real_free_parameter_equation_is_an_initial_equation() {
        // Rumoca は Real でない fixed = false のパラメータを決める式を、初期化の行から取り分ける
        let text = "model H\n  parameter Integer n(fixed = false);\n  Real x(start = 1, fixed = true);\ninitial equation\n  n = 3;\nequation\n  der(x) = -n * x;\nend H;\n";
        let init = compile("H", text);
        assert_eq!(flags(&init.parameters), [("n", false, false, false)]);
        assert_eq!(init.initial_equations.len(), 1);
        assert_eq!(init.initial_equations[0].label, "n = 3");
        assert!(init.initial_equations[0].range.is_some());
    }

    #[test]
    fn discretes_and_algebraics_are_listed_apart() {
        let text = "model K\n  discrete Real d(start = 1, fixed = true);\n  Integer k;\n  Real x(start = 1, fixed = true);\n  Real y(start = 2, fixed = true);\n  output Real z;\nequation\n  der(x) = -x;\n  y = 2 * x;\n  z = y + 1;\n  when sample(0, 0.1) then\n    d = pre(d) + 1;\n    k = pre(k) + 1;\n  end when;\nend K;\n";
        let init = compile("K", text);
        assert_eq!(
            flags(&init.discretes),
            [("d", true, true, false), ("k", false, false, false)]
        );
        assert_eq!(
            flags(&init.algebraics),
            [("y", true, true, false), ("z", false, false, false)]
        );
        assert_eq!(flags(&init.states), [("x", true, true, false)]);
        assert!(init.initial_equations.is_empty());
    }

    #[test]
    fn initial_definitions_of_discretes_are_initial_equations() {
        // k = 0（定数で決まる離散値）は Rumoca が initial_discrete_values() に取り分け、
        // d = x（連続の変数を読む離散 Real）は初期化の行に残る。どちらも初期方程式として数える。
        // 初期アルゴリズムの代入（b := true）も、離散変数の初期化の時点の値として数える
        let text = "model D
  discrete Real d;
  Integer k;
  Boolean b;
  Real x(start = 1, fixed = true);
initial equation
  k = 0;
  d = x;
initial algorithm
  b := true;
equation
  der(x) = -x;
  when sample(0, 0.1) then
    d = x;
    k = pre(k) + 1;
    b = not pre(b);
  end when;
end D;
";
        let init = compile("D", text);
        assert_eq!(init.discretes.len(), 3);
        let mut labels: Vec<&str> = init
            .initial_equations
            .iter()
            .map(|e| e.label.as_str())
            .collect();
        labels.sort_unstable();
        // 初期アルゴリズムの代入の位置は、代入先（`b`）を指す
        assert_eq!(labels, ["b", "d = x", "k = 0"]);
        assert!(init.initial_equations.iter().all(|e| e.range.is_some()));
    }

    #[test]
    fn array_initial_equations_expand_per_scalar() {
        let text = "model Arr\n  Real x[3];\ninitial equation\n  for i in 1:3 loop\n    x[i] = i;\n  end for;\nequation\n  der(x) = -x;\nend Arr;\n";
        let init = compile("Arr", text);
        let names: Vec<&str> = init.states.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["x[1]", "x[2]", "x[3]"]);
        assert_eq!(init.initial_equations.len(), 3);
        assert!(init.initial_equations.iter().all(|e| e.range.is_some()));
    }

    #[test]
    fn array_equation_in_initial_section_counts_each_scalar() {
        let text = "model ArrEq\n  Real x[2];\ninitial equation\n  x = {1, 2};\nequation\n  der(x) = -x;\nend ArrEq;\n";
        let init = compile("ArrEq", text);
        assert_eq!(init.states.len(), 2);
        assert_eq!(init.initial_equations.len(), 2);
    }

    #[test]
    fn per_element_fixed_is_read_per_scalar() {
        let text = "model Each\n  Real x[2](start = {1, 2}, fixed = {true, false});\nequation\n  der(x) = -x;\nend Each;\n";
        let init = compile("Each", text);
        assert_eq!(
            flags(&init.states),
            [("x[1]", true, true, false), ("x[2]", false, true, false)]
        );
    }

    #[test]
    fn pre_equalities_grouped_by_the_compiler_become_k_minus_one_rows() {
        // MSL の StateGraph の PartialStep と同じ形。Rumoca は 2 本の `pre(…) = pre(localActive)` を
        // 3 つの定義（位置はどれも 1 本目）にするので、3 つの pre をまとめた 2 行にする
        let text = "model SG
  Boolean localActive;
  Boolean newActive;
  Boolean oldActive;
  Boolean active;
  Boolean suspend = time > 2;
  Boolean inSet = time > 0.5;
initial equation
  pre(newActive) = pre(localActive);
  pre(oldActive) = pre(localActive);
  active = true;
equation
  localActive = pre(newActive);
  newActive = (inSet or localActive) and not suspend;
  when suspend then
    oldActive = localActive;
  end when;
  active = localActive;
end SG;
";
        let init = compile("SG", text);
        let mut labels: Vec<&str> = init
            .initial_equations
            .iter()
            .map(|e| e.label.as_str())
            .collect();
        labels.sort_unstable();
        assert_eq!(
            labels,
            [
                "active = true",
                "pre(newActive) = pre(localActive)",
                "pre(newActive) = pre(localActive)"
            ]
        );
        let pre_rows: Vec<&Vec<u32>> = init
            .initial_pre_discrete_refs
            .iter()
            .filter(|r| !r.is_empty())
            .collect();
        assert_eq!(pre_rows, [&vec![0, 1, 2], &vec![0, 1, 2]]);
        // localActive・newActive・active などは when の外、oldActive は when の中
        let kinds: Vec<DiscreteInitKind> = init.discrete_init.iter().map(|d| d.kind).collect();
        assert_eq!(kinds[2], DiscreteInitKind::When);
        assert_eq!(
            kinds
                .iter()
                .filter(|&&k| k == DiscreteInitKind::Always)
                .count(),
            init.discretes.len() - 1
        );
        // pre(localActive) と pre(newActive) は参照される（初期方程式と localActive = pre(newActive)）
        assert!(init.discrete_init[0].pre_referenced && init.discrete_init[1].pre_referenced);
    }
}

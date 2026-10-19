//! S9（代数ループ）: BLT 分解、tearing、非線形の反復変数の start 値（S010）。
//!
//! # 解く系
//!
//! - S7 で微分が要らない（[`IndexReport::differentiations`] がすべて 0）なら、der の形の
//!   [`DaeStructure::incidence`] そのもの。未知数は `der(x)` と代数・出力で、状態 `x` は既知。
//! - 微分が要る（インデックス 2 以上）なら、Pantelides 法の**最高階の系**。行は方程式 e の
//!   `differentiations[e]`（= c）回目の微分、列は各族の最高階（[`IndexReport::orders`]）。行 e^(c) は、元の参照
//!   （`incidence` の `der(x)` は階数 1、代数・出力は 0、`state_refs` の `x` は 0）のうち、c 回の微分で
//!   `o + c == orders[u]` になるものだけを未知数として参照する（それより低い階数は、積分で決まる既知とみなす）。
//!   c ≥ 1 の行は最高階の項について 1 次（微分は連鎖律で最高階を線形に出す）、c = 0 の行は
//!   [`DaeStructure::nonlinear`] に従う。状態選択（dummy derivative）の前の見込みなので、
//!   [`LoopReport::reduced`] で印を付ける。
//!
//! # BLT 分解
//!
//! 完全マッチング（[`hopcroft_karp`]）を取り、方程式 e が別の方程式 e' に対応づいた未知数を参照するとき
//! 「e は e' に依存する」とした有向グラフの強連結成分（再帰を使わない Tarjan 法）を、解く順（依存の先が先）に
//! 並べる。大きさ 2 以上の成分が代数ループ。成分の分け方は完全マッチングの選び方によらない（Duff 1981 の
//! 「正方で構造正則な行列の既約な block triangular form は行と列の置換を除いて一意」）。
//!
//! # tearing（貪欲法）
//!
//! 最適な tearing（tearing 変数の最小化）は NP 困難なので、発見的に選ぶ。
//!
//! 1. ループの中で、まだ決まっていない未知数が 1 つだけで、それが**線形に**現れる方程式があれば、
//!    その方程式でその未知数を解く（既知にする）。
//! 2. 無ければ tearing 変数（反復変数）を 1 つ選んで既知にする。start 値のある代数・出力を優先し、
//!    次に残りの方程式に多く現れるもの、同点は添字の小さい方。`der(x)` と最高階が 1 以上の列は最後に回す
//!    （start 値を書けないため）。
//! 3. 決まっていない未知数が無くなった方程式で、まだ使っていないものは残差になる。
//!
//! 方程式と未知数は同じ数で、1 で 1 本と 1 個、2 で 1 個、3 で 1 本を使うので、残差の本数は tearing 変数の数と
//! 同じになる。未知数が残っている方程式は使わないので、各未知数が「残りの方程式に現れる回数」は最後の 1 つが
//! 決まるまで変わらず、候補は始めに 1 回並べればよい。ループの参照の数 E と大きさ k に対して O(E + k log k)。
//!
//! # 反復変数の start 値
//!
//! 非線形のループの tearing 変数は、Newton 法の反復変数になる。そのうち階数 0 の代数・出力で
//! [`InitStructure::algebraics`] の `has_start` が false のものは、既定の 0 から反復する（`GM-W9001`）。
//! `der(x)` や高階の列は start 値を書けないので数えない。`init` が無ければ判定しない。
//!
//! Rumoca に依存しない。wasm32 でも動く。

use std::cmp::Reverse;

use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_ir::{DaeStructure, InitStructure, UnknownKind};
use serde::Serialize;

use crate::matching::hopcroft_karp;
use crate::pantelides::IndexReport;
use crate::{list, locate};

/// 代数ループ 1 つ（BLT の大きさ 2 以上のブロック）。添字は [`DaeStructure`] の `equations` / `unknowns` のもの。
///
/// [`LoopReport::reduced`] のとき、未知数 u は族 u の最高階（[`IndexReport::orders`]`[u]`）の列を表す。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct AlgebraicLoop {
    /// ループの方程式（昇順）
    pub equations: Vec<usize>,
    /// ループの未知数（昇順。数は `equations` と同じ）
    pub unknowns: Vec<usize>,
    /// `unknowns[k]` の列の階数（der の形なら `der(x)` が 1、代数・出力が 0。reduced なら S7 の最高階）
    pub orders: Vec<u32>,
    /// どれかの方程式が、ループの未知数を非線形に参照する
    pub nonlinear: bool,
    /// tearing 変数（選んだ順）
    pub tearing: Vec<usize>,
    /// 残差の方程式（決まった順。本数は `tearing` と同じ）
    pub residuals: Vec<usize>,
    /// tearing 変数を既知としたときに、順に解く（方程式, その方程式で解く未知数）
    pub solved: Vec<(usize, usize)>,
    /// 非線形のループの tearing 変数のうち、start 値の無い階数 0 の代数・出力（選んだ順）。
    /// 線形のループと、start 値を判定しなかったとき（[`LoopReport::start_checked`] が false）は空
    pub without_start: Vec<usize>,
}

impl AlgebraicLoop {
    /// 方程式の数。
    pub fn size(&self) -> usize {
        self.equations.len()
    }
}

/// S9 の結果。位置を持たないので、[`DaeStructure::without_ranges`] を渡しても同じ値になる（防火壁 2、ADR-0005）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct LoopReport {
    /// BLT のブロックの数（大きさ 1 のものも含む）
    pub blocks: usize,
    /// Pantelides 法の最高階の系で解析した（インデックス 2 以上。状態選択の前の見込み）
    pub reduced: bool,
    /// 代数ループ（BLT の解く順）
    pub loops: Vec<AlgebraicLoop>,
    /// 反復変数の start 値を判定した（`init` があり、代数・出力の数が合った）
    pub start_checked: bool,
    /// 解析しなかった理由（S7 の打ち切り、長さの不一致、完全マッチングが無い）。あれば他は空
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

impl LoopReport {
    /// ループの大きさ（方程式の数）の一覧（昇順）。
    pub fn loop_sizes(&self) -> Vec<usize> {
        let mut sizes: Vec<usize> = self.loops.iter().map(AlgebraicLoop::size).collect();
        sizes.sort_unstable();
        sizes
    }

    /// ループの数。
    pub fn n_loops(&self) -> usize {
        self.loops.len()
    }

    /// 線形のループの数。
    pub fn n_linear(&self) -> usize {
        self.loops.iter().filter(|l| !l.nonlinear).count()
    }

    /// 非線形のループの数。
    pub fn n_nonlinear(&self) -> usize {
        self.loops.iter().filter(|l| l.nonlinear).count()
    }

    /// 反復変数（非線形のループの tearing 変数）の数。
    pub fn n_iteration_variables(&self) -> usize {
        self.loops
            .iter()
            .filter(|l| l.nonlinear)
            .map(|l| l.tearing.len())
            .sum()
    }

    /// start 値の無い反復変数の数（判定しなかったときは 0）。
    pub fn n_iteration_without_start(&self) -> usize {
        self.loops.iter().map(|l| l.without_start.len()).sum()
    }

    /// 一番大きいループの方程式の数（無ければ 0）。
    pub fn max_loop_size(&self) -> usize {
        self.loops
            .iter()
            .map(AlgebraicLoop::size)
            .max()
            .unwrap_or(0)
    }

    /// 診断（S9）。`structure` と `init` は、この結果を作ったときと同じ（位置のある）ものを渡す。
    ///
    /// - ループがあれば `GM-I9001`（Info）を 1 つ。位置は一番大きい非線形のループ（無ければ一番大きいループ）の
    ///   最初の方程式。補足にそのループの未知数。
    /// - start 値の無い反復変数を持つ非線形のループごとに `GM-W9001`（Warning）。位置は最初のその変数の宣言。
    ///
    /// 解析しなかったときは何も出さない。添字が範囲外なら、名前の代わりに `#添字` と書く。
    pub fn diagnostics(
        &self,
        structure: &DaeStructure,
        _init: Option<&InitStructure>,
    ) -> Vec<Diagnostic> {
        if self.skipped.is_some() || self.loops.is_empty() {
            return Vec::new();
        }
        // ループ l の列 u の表示名（reduced なら高階の列を `der(der(x))` の形で書く）
        let name = |l: &AlgebraicLoop, u: usize| match structure.unknowns.get(u) {
            Some(x) if !x.name.is_empty() => {
                let order = l
                    .unknowns
                    .binary_search(&u)
                    .ok()
                    .and_then(|k| l.orders.get(k).copied());
                column_name(&x.name, x.kind, order)
            }
            _ => format!("#{u}"),
        };
        let at_unknown = |d: Diagnostic, u: usize| {
            let x = structure.unknowns.get(u);
            locate(
                d,
                x.and_then(|x| x.file.as_deref()),
                x.and_then(|x| x.range),
            )
        };

        let mut out = Vec::new();
        // 一番大きい非線形のループ（無ければ一番大きいループ）。同じ大きさなら解く順で先のもの
        let main = self
            .loops
            .iter()
            .enumerate()
            .max_by_key(|(i, l)| (l.nonlinear, l.size(), Reverse(*i)))
            .map(|(_, l)| l)
            .expect("ループがある");
        let mut d = Diagnostic::info(
            codes::ALGEBRAIC_LOOP,
            Stage::S9,
            format!(
                "代数ループが {} 個（線形 {}・非線形 {}、最大 {} 本の方程式）",
                self.n_loops(),
                self.n_linear(),
                self.n_nonlinear(),
                self.max_loop_size()
            ),
        );
        if let Some(&e) = main.equations.first() {
            let row = structure.equations.get(e);
            d = locate(
                d,
                row.and_then(|r| r.file.as_deref()),
                row.and_then(|r| r.range),
            );
        }
        d = d.note(format!(
            "{}のループの未知数（{} 個）: {}",
            if main.nonlinear {
                "一番大きい非線形"
            } else {
                "一番大きい線形"
            },
            main.unknowns.len(),
            list(&main.unknowns, |u| name(main, u))
        ));
        let n_iter = self.n_iteration_variables();
        if n_iter > 0 {
            d = d.note(format!(
                "非線形のループの反復変数（tearing 変数）は計 {n_iter} 個（貪欲法の見積もり）"
            ));
        }
        if self.reduced {
            d = d.note(
                "インデックス低減（Pantelides 法）の最高階の系で数えた。状態選択（dummy derivative）の前の見込み",
            );
        }
        out.push(d);

        for l in self.loops.iter().filter(|l| !l.without_start.is_empty()) {
            let d = Diagnostic::warning(
                codes::LOOP_ITERATION_WITHOUT_START,
                Stage::S9,
                format!(
                    "非線形の代数ループ（方程式 {} 本）の反復変数 {} 個に start 値が無い（既定の 0 から反復する）",
                    l.size(),
                    l.without_start.len()
                ),
            );
            let d = at_unknown(d, l.without_start[0])
                .note(format!(
                    "start 値の無い反復変数（{} 個）: {}",
                    l.without_start.len(),
                    list(&l.without_start, |u| name(l, u))
                ))
                .note(format!(
                    "このループの反復変数（tearing 変数）は {} 個",
                    l.tearing.len()
                ));
            out.push(d);
        }
        out
    }
}

/// 族の名前と階数から、列の表示名を作る（`der(x)` の 2 階は `der(der(x))`）。階数が分からなければ名前のまま。
fn column_name(name: &str, kind: UnknownKind, order: Option<u32>) -> String {
    let Some(order) = order else {
        return name.to_string();
    };
    let (base, base_order) = match kind {
        UnknownKind::Derivative => (
            name.strip_prefix("der(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(name),
            1,
        ),
        _ => (name, 0),
    };
    if order == base_order {
        return name.to_string();
    }
    let mut s = base.to_string();
    for _ in 0..order {
        s = format!("der({s})");
    }
    s
}

/// S9 の解析。`init` は start 値の有無に使う（無ければ start 値は判定しない）。
///
/// `index` は同じ `structure` の S7 の結果。S7 が打ち切った、長さが合わない、解く系に完全マッチングが無い
/// ときは解析しない（[`LoopReport::skipped`] に理由を入れる）。
pub fn analyze_loops(
    structure: &DaeStructure,
    index: &IndexReport,
    init: Option<&InitStructure>,
) -> LoopReport {
    let system = match System::new(structure, index) {
        Ok(s) => s,
        Err(reason) => {
            return LoopReport {
                skipped: Some(reason),
                ..LoopReport::default()
            };
        }
    };
    let n = system.n();
    let m = structure.unknowns.len();
    let matching = hopcroft_karp(n, m, &system.rows);
    if !matching.is_perfect() {
        return LoopReport {
            reduced: system.reduced,
            skipped: Some(format!(
                "解く系に完全マッチングが無い（方程式 {n} 本のうち {} 本しか対応づかない）",
                matching.size
            )),
            ..LoopReport::default()
        };
    }
    let mate_e: Vec<usize> = matching
        .equation_to_unknown
        .iter()
        .map(|u| u.expect("完全マッチング"))
        .collect();
    let mate_u: Vec<usize> = matching
        .unknown_to_equation
        .iter()
        .map(|e| e.expect("完全マッチング"))
        .collect();
    let blt = blt(&system, &mate_u);

    // start 値: 代数・出力の族 → InitStructure::algebraics の添字
    let n_algebraics = structure
        .unknowns
        .iter()
        .filter(|u| u.kind != UnknownKind::Derivative)
        .count();
    let init = init.filter(|i| i.algebraics.len() == n_algebraics);
    let mut algebraic_index = Vec::with_capacity(m);
    let mut k = 0;
    for u in &structure.unknowns {
        if u.kind == UnknownKind::Derivative {
            algebraic_index.push(None);
        } else {
            algebraic_index.push(Some(k));
            k += 1;
        }
    }
    // 列ごとの tearing の優先の組（0: start 値のある代数・出力、1: start 値の無い（分からない）代数・出力、
    // 2: der(x) と最高階が 1 以上の列）
    let class: Vec<u8> = (0..m)
        .map(|u| {
            if system.order[u] != 0 || algebraic_index[u].is_none() {
                2
            } else if init
                .zip(algebraic_index[u])
                .is_some_and(|(i, a)| i.algebraics[a].has_start)
            {
                0
            } else {
                1
            }
        })
        .collect();
    let has_start = |u: usize| {
        init.zip(algebraic_index[u])
            .is_some_and(|(i, a)| i.algebraics[a].has_start)
    };

    let mut tearer = Tearer::new(&system, &blt.block_of, &mate_u, &class);
    let mut loops = Vec::new();
    for b in 0..blt.n_blocks() {
        let eqs = blt.block(b);
        if eqs.len() < 2 {
            continue;
        }
        let mut equations = eqs.to_vec();
        equations.sort_unstable();
        let mut unknowns: Vec<usize> = equations.iter().map(|&e| mate_e[e]).collect();
        unknowns.sort_unstable();
        let nonlinear = equations.iter().any(|&e| {
            system
                .row(e)
                .any(|(u, nl)| nl && blt.block_of[mate_u[u]] == b)
        });
        let torn = tearer.tear(b, &equations, &unknowns);
        let without_start = if nonlinear && init.is_some() {
            torn.tearing
                .iter()
                .copied()
                .filter(|&u| class[u] != 2 && !has_start(u))
                .collect()
        } else {
            Vec::new()
        };
        let orders = unknowns.iter().map(|&u| system.order[u]).collect();
        loops.push(AlgebraicLoop {
            equations,
            unknowns,
            orders,
            nonlinear,
            tearing: torn.tearing,
            residuals: torn.residuals,
            solved: torn.solved,
            without_start,
        });
    }
    LoopReport {
        blocks: blt.n_blocks(),
        reduced: system.reduced,
        loops,
        start_checked: init.is_some(),
        skipped: None,
    }
}

/// 解く系（方程式 × 未知数の列）。行は昇順・重複なしで、参照ごとに非線形かを持つ。
#[derive(Debug)]
struct System {
    /// 行 e の列（HK に渡す形）
    rows: Vec<Vec<u32>>,
    /// `rows[e][k]` が非線形に現れる
    nonlinear: Vec<Vec<bool>>,
    /// 列 u の階数（der の形なら Derivative が 1、代数・出力が 0。reduced なら S7 の最高階）
    order: Vec<u32>,
    reduced: bool,
}

impl System {
    fn new(structure: &DaeStructure, index: &IndexReport) -> Result<Self, String> {
        let n = structure.equations.len();
        let m = structure.unknowns.len();
        if index.failed {
            return Err("S7（インデックス）が打ち切ったので解析しない".to_string());
        }
        if n != m
            || structure.incidence.len() != n
            || structure.state_refs.len() != n
            || !(structure.nonlinear.is_empty() || structure.nonlinear.len() == n)
            || index.differentiations.len() != n
            || index.orders.len() != m
        {
            return Err(format!(
                "長さが合わない（方程式 {n} 本、未知数 {m} 個、incidence {}、state_refs {}、nonlinear {}、S7 の微分 {}・階数 {}）",
                structure.incidence.len(),
                structure.state_refs.len(),
                structure.nonlinear.len(),
                index.differentiations.len(),
                index.orders.len()
            ));
        }
        let base = |u: usize| match structure.unknowns[u].kind {
            UnknownKind::Derivative => 1u32,
            _ => 0,
        };
        let reduced = index.differentiations.iter().any(|&d| d > 0);
        let order: Vec<u32> = if reduced {
            index.orders.clone()
        } else {
            (0..m).map(base).collect()
        };
        let is_nonlinear = |e: usize, u: u32| {
            structure
                .nonlinear
                .get(e)
                .is_none_or(|r| r.binary_search(&u).is_ok())
        };
        let mut rows = Vec::with_capacity(n);
        let mut nonlinear = Vec::with_capacity(n);
        let mut scratch: Vec<u32> = Vec::new();
        for e in 0..n {
            scratch.clear();
            let c = index.differentiations[e];
            for &u in &structure.incidence[e] {
                let ui = u as usize;
                if ui < m
                    && (!reduced || u64::from(base(ui)) + u64::from(c) == u64::from(order[ui]))
                {
                    scratch.push(u);
                }
            }
            if reduced {
                // 状態そのもの x（階数 0）は、c 回の微分で最高階に届くときだけ未知数
                for &u in &structure.state_refs[e] {
                    let ui = u as usize;
                    if ui < m
                        && structure.unknowns[ui].kind == UnknownKind::Derivative
                        && c == order[ui]
                    {
                        scratch.push(u);
                    }
                }
            }
            scratch.sort_unstable();
            scratch.dedup();
            // c ≥ 1 の行は最高階について線形。c = 0 の行は nonlinear に従う
            let flags = scratch
                .iter()
                .map(|&u| c == 0 && is_nonlinear(e, u))
                .collect();
            rows.push(scratch.clone());
            nonlinear.push(flags);
        }
        Ok(Self {
            rows,
            nonlinear,
            order,
            reduced,
        })
    }

    fn n(&self) -> usize {
        self.rows.len()
    }

    /// 行 e の（列, 非線形か）。
    fn row(&self, e: usize) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.rows[e]
            .iter()
            .zip(&self.nonlinear[e])
            .map(|(&u, &nl)| (u as usize, nl))
    }
}

/// BLT 分解の結果: 強連結成分を解く順に並べたもの（CSR）。
#[derive(Debug)]
struct Blt {
    /// 成分 b の方程式は `order[start[b]..start[b + 1]]`
    order: Vec<usize>,
    start: Vec<usize>,
    /// 方程式 e の成分
    block_of: Vec<usize>,
}

impl Blt {
    fn n_blocks(&self) -> usize {
        self.start.len() - 1
    }

    fn block(&self, b: usize) -> &[usize] {
        &self.order[self.start[b]..self.start[b + 1]]
    }
}

/// 依存グラフ（方程式 e → e が参照する未知数に対応づいた方程式）の強連結成分。再帰を使わない Tarjan 法で O(E)。
///
/// Tarjan 法は、成分から届く成分をすべて出してからその成分を出すので、出てきた順がそのまま解く順になる。
fn blt(system: &System, mate_u: &[usize]) -> Blt {
    const UNVISITED: usize = usize::MAX;
    let n = system.n();
    let mut index = vec![UNVISITED; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    // 呼び出しの代わり: (方程式, 次に見る `rows` の位置)
    let mut call: Vec<(usize, usize)> = Vec::new();
    let mut counter = 0;
    let mut out = Blt {
        order: Vec::with_capacity(n),
        start: vec![0],
        block_of: vec![UNVISITED; n],
    };
    for root in 0..n {
        if index[root] != UNVISITED {
            continue;
        }
        index[root] = counter;
        low[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        call.push((root, 0));
        while let Some(top) = call.last_mut() {
            let (v, pos) = *top;
            if let Some(&u) = system.rows[v].get(pos) {
                top.1 += 1;
                let w = mate_u[u as usize];
                if w == v {
                    continue;
                }
                if index[w] == UNVISITED {
                    index[w] = counter;
                    low[w] = counter;
                    counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            call.pop();
            if let Some(&(p, _)) = call.last() {
                low[p] = low[p].min(low[v]);
            }
            if low[v] == index[v] {
                let b = out.n_blocks();
                loop {
                    let w = stack.pop().expect("v はスタックにある");
                    on_stack[w] = false;
                    out.block_of[w] = b;
                    out.order.push(w);
                    if w == v {
                        break;
                    }
                }
                out.start.push(out.order.len());
            }
        }
    }
    out
}

/// 1 つのループの tearing の結果。
#[derive(Debug, Default)]
struct Torn {
    tearing: Vec<usize>,
    residuals: Vec<usize>,
    solved: Vec<(usize, usize)>,
}

/// tearing の作業領域。ループをまたいで使い回す（配列の大きさは方程式・未知数の数）。
struct Tearer<'a> {
    system: &'a System,
    block_of: &'a [usize],
    class: &'a [u8],
    /// 列 u を参照する方程式は `col_rows[col_start[u]..col_start[u + 1]]`（転置の CSR）
    col_start: Vec<usize>,
    col_rows: Vec<usize>,
    /// 列 u の成分（u に対応づいた方程式の成分）
    col_block: Vec<usize>,
    known: Vec<bool>,
    done: Vec<bool>,
    /// 方程式ごとの、まだ決まっていない成分の未知数の数
    remaining: Vec<usize>,
}

impl<'a> Tearer<'a> {
    fn new(system: &'a System, block_of: &'a [usize], mate_u: &[usize], class: &'a [u8]) -> Self {
        let n = system.n();
        let m = mate_u.len();
        let mut col_start = vec![0usize; m + 1];
        for row in &system.rows {
            for &u in row {
                col_start[u as usize + 1] += 1;
            }
        }
        for u in 0..m {
            col_start[u + 1] += col_start[u];
        }
        let mut fill = col_start.clone();
        let mut col_rows = vec![0usize; col_start[m]];
        for (e, row) in system.rows.iter().enumerate() {
            for &u in row {
                col_rows[fill[u as usize]] = e;
                fill[u as usize] += 1;
            }
        }
        Self {
            system,
            block_of,
            class,
            col_start,
            col_rows,
            col_block: mate_u.iter().map(|&e| block_of[e]).collect(),
            known: vec![false; m],
            done: vec![false; n],
            remaining: vec![0; n],
        }
    }

    /// 成分 b（方程式 `equations`、未知数 `unknowns`）を tearing する（貪欲法。モジュールの説明を参照）。
    fn tear(&mut self, b: usize, equations: &[usize], unknowns: &[usize]) -> Torn {
        let mut torn = Torn::default();
        // 解ける見込みの方程式（未知数が 1 つだけ残ったもの）
        let mut queue: Vec<usize> = Vec::new();
        for &e in equations {
            let r = self.rows_in_block(e, b).count();
            self.remaining[e] = r;
            if r == 1 {
                queue.push(e);
            }
        }
        // tearing 変数の候補を優先の順に並べる: 組（start 値のある代数・出力 → 無いもの → der(x) と高階）、
        // 成分の方程式に現れる回数の多い順、添字の小さい順。回数は、未知数が決まらないうちは方程式が
        // 使われない（残差になるのは全部決まってから、解くのは最後の 1 つを決めるとき）ので変わらない
        let mut candidates: Vec<(u8, Reverse<usize>, usize)> = unknowns
            .iter()
            .map(|&u| {
                let occurrences = self.col_rows[self.col_start[u]..self.col_start[u + 1]]
                    .iter()
                    .filter(|&&e| self.block_of[e] == b)
                    .count();
                (self.class[u], Reverse(occurrences), u)
            })
            .collect();
        candidates.sort_unstable();
        let mut next_candidate = 0;
        let mut left = unknowns.len();
        loop {
            while let Some(e) = queue.pop() {
                if self.done[e] || self.remaining[e] != 1 {
                    continue;
                }
                let (u, nonlinear) = self
                    .rows_in_block(e, b)
                    .find(|&(u, _)| !self.known[u])
                    .expect("未知数が 1 つ残っている");
                if nonlinear {
                    continue; // 非線形にしか現れないので解かない（決まれば残差になる）
                }
                self.done[e] = true;
                torn.solved.push((e, u));
                self.settle(u, b, &mut queue, &mut torn);
                left -= 1;
            }
            if left == 0 {
                break;
            }
            while self.known[candidates[next_candidate].2] {
                next_candidate += 1;
            }
            let u = candidates[next_candidate].2;
            torn.tearing.push(u);
            self.settle(u, b, &mut queue, &mut torn);
            left -= 1;
        }
        debug_assert!(equations.iter().all(|&e| self.done[e]));
        debug_assert_eq!(torn.tearing.len(), torn.residuals.len());
        torn
    }

    /// 方程式 e の、成分 b の列（列, 非線形か）。
    fn rows_in_block(&self, e: usize, b: usize) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.system
            .row(e)
            .filter(move |&(u, _)| self.col_block[u] == b)
    }

    /// 列 u を既知にし、u を参照する成分の方程式の残りを減らす。残りが 1 なら解ける見込み、0 なら残差。
    fn settle(&mut self, u: usize, b: usize, queue: &mut Vec<usize>, torn: &mut Torn) {
        self.known[u] = true;
        for &e in &self.col_rows[self.col_start[u]..self.col_start[u + 1]] {
            if self.block_of[e] != b || self.done[e] {
                continue;
            }
            self.remaining[e] -= 1;
            match self.remaining[e] {
                0 => {
                    self.done[e] = true;
                    torn.residuals.push(e);
                }
                1 => queue.push(e),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pantelides::analyze_index;
    use crate::testutil::{FILE, XorShift, dae, structure};
    use globe_mode_core::{Severity, TextRange};
    use globe_mode_ir::InitVariable;

    /// 方程式ごとの非線形の参照を付ける。
    fn with_nonlinear(mut s: DaeStructure, nonlinear: &[&[u32]]) -> DaeStructure {
        s.nonlinear = nonlinear.iter().map(|r| r.to_vec()).collect();
        s
    }

    /// 代数・出力の start 値の有無だけを持つ InitStructure。
    fn init_with_starts(starts: &[bool]) -> InitStructure {
        InitStructure {
            algebraics: starts
                .iter()
                .enumerate()
                .map(|(i, &has_start)| InitVariable {
                    name: format!("z{i}"),
                    fixed: false,
                    has_start,
                    has_binding: false,
                    file: None,
                    range: None,
                })
                .collect(),
            ..InitStructure::default()
        }
    }

    fn run(s: &DaeStructure, init: Option<&InitStructure>) -> LoopReport {
        let index = analyze_index(s);
        let r = analyze_loops(s, &index, init);
        // 位置を落としても同じ（防火壁 2）
        assert_eq!(analyze_loops(&s.without_ranges(), &index, init), r);
        r
    }

    /// tearing が成り立つ: tearing 変数を既知とすると、`solved` の順に、方程式の他の未知数がすべて既知で、
    /// 解く未知数が線形に現れる。最後に残差の方程式の未知数はすべて既知。方程式と未知数はちょうど 1 回ずつ使う。
    fn assert_valid_tearing(s: &DaeStructure, index: &IndexReport, l: &AlgebraicLoop) {
        let system = System::new(s, index).expect("解く系");
        assert_eq!(l.unknowns.len(), l.equations.len());
        assert_eq!(l.tearing.len(), l.residuals.len());
        assert_eq!(l.solved.len() + l.tearing.len(), l.unknowns.len());
        let in_loop = |u: usize| l.unknowns.binary_search(&u).is_ok();
        let mut known: std::collections::HashSet<usize> = l.tearing.iter().copied().collect();
        assert_eq!(known.len(), l.tearing.len(), "tearing 変数が重なる");
        let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for &(e, u) in &l.solved {
            assert!(l.equations.binary_search(&e).is_ok());
            assert!(in_loop(u));
            assert!(used.insert(e), "方程式 {e} を 2 回使った");
            let mut found = false;
            for (v, nl) in system.row(e).filter(|&(v, _)| in_loop(v)) {
                if v == u {
                    assert!(!nl, "方程式 {e} は {u} を非線形に参照する");
                    found = true;
                } else {
                    assert!(
                        known.contains(&v),
                        "方程式 {e} を解くとき {v} が決まっていない"
                    );
                }
            }
            assert!(found, "方程式 {e} は {u} を参照しない");
            assert!(known.insert(u), "未知数 {u} を 2 回決めた");
        }
        for &e in &l.residuals {
            assert!(used.insert(e), "方程式 {e} を 2 回使った");
            assert!(
                system
                    .row(e)
                    .all(|(v, _)| !in_loop(v) || known.contains(&v))
            );
        }
        assert_eq!(used.len(), l.equations.len());
        assert_eq!(known.len(), l.unknowns.len());
    }

    #[test]
    fn chain_has_no_loops() {
        // z0 = 1; z1 = z0; z2 = z1 + z0
        let s = structure(
            &["z0", "z1", "z2"],
            &[
                ("z0 = 1", &[0]),
                ("z1 = z0", &[0, 1]),
                ("z2 = z1 + z0", &[0, 1, 2]),
            ],
        );
        let r = run(&s, None);
        assert_eq!(r.skipped, None);
        assert!(!r.reduced);
        assert_eq!(r.blocks, 3);
        assert!(r.loops.is_empty());
        assert_eq!(r.loop_sizes(), Vec::<usize>::new());
        assert_eq!(r.max_loop_size(), 0);
        assert!(r.diagnostics(&s, None).is_empty());
    }

    #[test]
    fn linear_two_by_two_loop() {
        // a + b = 1; a - b = 0（どちらも線形）。der(x) = a は後に解く
        let s = with_nonlinear(
            structure(
                &["der(x)", "a", "b"],
                &[
                    ("der(x) = a", &[0, 1]),
                    ("a + b = 1", &[1, 2]),
                    ("a - b = 0", &[1, 2]),
                ],
            ),
            &[&[], &[], &[]],
        );
        let r = run(&s, None);
        assert_eq!(r.blocks, 2);
        assert_eq!(r.loop_sizes(), [2]);
        assert_eq!((r.n_loops(), r.n_linear(), r.n_nonlinear()), (1, 1, 0));
        let l = &r.loops[0];
        assert_eq!(l.equations, [1, 2]);
        assert_eq!(l.unknowns, [1, 2]);
        assert_eq!(l.orders, [0, 0]);
        assert!(!l.nonlinear);
        assert_eq!(l.tearing.len(), 1);
        assert_valid_tearing(&s, &analyze_index(&s), l);
        // 線形のループは反復変数を数えない
        assert_eq!(r.n_iteration_variables(), 0);
        assert!(l.without_start.is_empty());

        let ds = r.diagnostics(&s, None);
        assert_eq!(ds.len(), 1, "{ds:#?}");
        let d = &ds[0];
        assert_eq!(d.code, codes::ALGEBRAIC_LOOP);
        assert_eq!((d.severity, d.stage), (Severity::Info, Stage::S9));
        assert_eq!(
            d.message,
            "代数ループが 1 個（線形 1・非線形 0、最大 2 本の方程式）"
        );
        assert_eq!(d.file.as_deref(), Some(FILE));
        assert_eq!(d.range, Some(TextRange::new(10, 18)));
        assert_eq!(
            d.notes,
            ["一番大きい線形のループの未知数（2 個）: `a`, `b`"]
        );
    }

    /// a * b = 1（非線形）; a + b = 3（線形）
    fn nonlinear_loop() -> DaeStructure {
        with_nonlinear(
            structure(
                &["a", "b"],
                &[("a * b = 1", &[0, 1]), ("a + b = 3", &[0, 1])],
            ),
            &[&[0, 1], &[]],
        )
    }

    #[test]
    fn nonlinear_loop_prefers_tearing_variable_with_start() {
        let s = nonlinear_loop();
        // b にだけ start 値がある → b を反復変数にし、a + b = 3 で a を解き、a * b = 1 が残差
        let init = init_with_starts(&[false, true]);
        let r = run(&s, Some(&init));
        assert!(r.start_checked);
        assert_eq!((r.n_linear(), r.n_nonlinear()), (0, 1));
        let l = &r.loops[0];
        assert!(l.nonlinear);
        assert_eq!(l.tearing, [1]);
        assert_eq!(l.solved, [(1, 0)]);
        assert_eq!(l.residuals, [0]);
        assert!(l.without_start.is_empty());
        assert_eq!(
            (r.n_iteration_variables(), r.n_iteration_without_start()),
            (1, 0)
        );
        let ds = r.diagnostics(&s, Some(&init));
        assert_eq!(ds.len(), 1, "{ds:#?}");
        assert_eq!(
            ds[0].message,
            "代数ループが 1 個（線形 0・非線形 1、最大 2 本の方程式）"
        );
        assert_eq!(
            ds[0].notes,
            [
                "一番大きい非線形のループの未知数（2 個）: `a`, `b`",
                "非線形のループの反復変数（tearing 変数）は計 1 個（貪欲法の見積もり）",
            ]
        );
    }

    #[test]
    fn nonlinear_loop_without_start_warns() {
        let s = nonlinear_loop();
        let init = init_with_starts(&[false, false]);
        let r = run(&s, Some(&init));
        let l = &r.loops[0];
        // 同点なので添字の小さい a
        assert_eq!(l.tearing, [0]);
        assert_eq!(l.without_start, [0]);
        assert_eq!(r.n_iteration_without_start(), 1);
        let ds = r.diagnostics(&s, Some(&init));
        assert_eq!(ds.len(), 2, "{ds:#?}");
        let w = &ds[1];
        assert_eq!(w.code, codes::LOOP_ITERATION_WITHOUT_START);
        assert_eq!((w.severity, w.stage), (Severity::Warning, Stage::S9));
        assert_eq!(
            w.message,
            "非線形の代数ループ（方程式 2 本）の反復変数 1 個に start 値が無い（既定の 0 から反復する）"
        );
        // 位置は a の宣言
        assert_eq!(w.file.as_deref(), Some(FILE));
        assert_eq!(w.range, Some(TextRange::new(1000, 1005)));
        assert_eq!(
            w.notes,
            [
                "start 値の無い反復変数（1 個）: `a`",
                "このループの反復変数（tearing 変数）は 1 個",
            ]
        );

        // init が無ければ判定しない
        let r = run(&s, None);
        assert!(!r.start_checked);
        assert!(r.loops[0].without_start.is_empty());
        assert_eq!(r.diagnostics(&s, None).len(), 1);
        // 代数・出力の数が合わない init も判定しない
        let r = run(&s, Some(&init_with_starts(&[false])));
        assert!(!r.start_checked);
    }

    #[test]
    fn empty_nonlinear_means_all_nonlinear() {
        // nonlinear が長さ 0 なら、すべての参照を非線形とみなす（解けないので全部 tearing 変数になる）
        let mut s = nonlinear_loop();
        s.nonlinear = Vec::new();
        let r = run(&s, None);
        let l = &r.loops[0];
        assert!(l.nonlinear);
        assert_eq!(l.tearing, [0, 1]);
        assert!(l.solved.is_empty());
        assert_valid_tearing(&s, &analyze_index(&s), l);
    }

    #[test]
    fn two_loops_in_series() {
        // ループ 1: a, b（線形）。ループ 2: c, d（非線形。a を参照する）。方程式はループ 2 を先に書く
        let s = with_nonlinear(
            structure(
                &["a", "b", "c", "d"],
                &[
                    ("c * d = a", &[0, 2, 3]),
                    ("c + d = 1", &[2, 3]),
                    ("a + b = 1", &[0, 1]),
                    ("a - b = 0", &[0, 1]),
                ],
            ),
            &[&[2, 3], &[], &[], &[]],
        );
        let r = run(&s, Some(&init_with_starts(&[true, true, true, false])));
        assert_eq!(r.blocks, 2);
        assert_eq!(r.loop_sizes(), [2, 2]);
        // 解く順: a, b のループが先
        assert_eq!(r.loops[0].equations, [2, 3]);
        assert!(!r.loops[0].nonlinear);
        assert_eq!(r.loops[1].equations, [0, 1]);
        assert!(r.loops[1].nonlinear);
        // c に start 値があるので c を選ぶ
        assert_eq!(r.loops[1].tearing, [2]);
        assert!(r.loops[1].without_start.is_empty());
        let ds = r.diagnostics(&s, None);
        assert_eq!(ds.len(), 1);
        assert_eq!(
            ds[0].message,
            "代数ループが 2 個（線形 1・非線形 1、最大 2 本の方程式）"
        );
        // 位置は非線形のループの最初の方程式
        assert_eq!(ds[0].range, Some(TextRange::new(0, 8)));
    }

    /// pantelides.rs のテストと同じ振り子（インデックス 3）
    fn pendulum() -> DaeStructure {
        dae(
            &["x", "y", "vx", "vy"],
            &["lambda"],
            &[
                ("der(x) = vx", &["der(x)", "vx"]),
                ("der(y) = vy", &["der(y)", "vy"]),
                ("der(vx) = lambda * x", &["der(vx)", "lambda", "x"]),
                ("der(vy) = lambda * y - g", &["der(vy)", "lambda", "y"]),
                ("x^2 + y^2 = L^2", &["x", "y"]),
            ],
        )
    }

    #[test]
    fn pendulum_uses_highest_order_system() {
        let s = pendulum();
        let index = analyze_index(&s);
        assert_eq!(index.index, 3);
        let system = System::new(&s, &index).unwrap();
        assert!(system.reduced);
        // 列: 0 = der(der(x))、1 = der(der(y))、2 = der(vx)、3 = der(vy)、4 = lambda
        assert_eq!(
            system.rows,
            [vec![0, 2], vec![1, 3], vec![2, 4], vec![3, 4], vec![0, 1]]
        );
        // 微分した行は線形、微分していない行は nonlinear（ここでは空 = すべて非線形）に従う
        assert_eq!(
            system.nonlinear,
            [
                vec![false, false],
                vec![false, false],
                vec![true, true],
                vec![true, true],
                vec![false, false]
            ]
        );
        let r = run(&s, Some(&init_with_starts(&[false])));
        assert!(r.reduced);
        assert_eq!(r.blocks, 1);
        assert_eq!(r.loop_sizes(), [5]);
        let l = &r.loops[0];
        assert_eq!(l.orders, [2, 2, 1, 1, 0]);
        assert!(l.nonlinear);
        assert_valid_tearing(&s, &index, l);
        // lambda（階数 0 の代数）を先に選ぶ。der(vx) = lambda * x が非線形とされるので、もう 1 つ要る
        assert_eq!(l.tearing[0], 4);
        assert_eq!(l.tearing.len(), 2);
        // start 値の警告は lambda だけ（高階の列は start 値を書けない）
        assert_eq!(l.without_start, [4]);

        let ds = r.diagnostics(&s, None);
        assert_eq!(ds.len(), 2, "{ds:#?}");
        assert_eq!(
            ds[0].notes[0],
            "一番大きい非線形のループの未知数（5 個）: `der(der(x))`, `der(der(y))`, `der(vx)`, `der(vy)`, `lambda`"
        );
        assert_eq!(
            ds[0].notes[2],
            "インデックス低減（Pantelides 法）の最高階の系で数えた。状態選択（dummy derivative）の前の見込み"
        );

        // lambda * x を線形とすれば線形のループで、lambda を選べば残りは順に解ける
        let s = with_nonlinear(pendulum(), &[&[], &[], &[], &[], &[]]);
        let r = run(&s, None);
        let l = &r.loops[0];
        assert!(!l.nonlinear);
        assert_eq!(l.tearing, [4]);
        assert_valid_tearing(&s, &index, l);
    }

    #[test]
    fn skips_when_not_analyzable() {
        let s = nonlinear_loop();
        let mut index = analyze_index(&s);
        index.failed = true;
        let r = analyze_loops(&s, &index, None);
        assert!(r.skipped.is_some());
        assert!(r.diagnostics(&s, None).is_empty());

        let index = IndexReport {
            differentiations: vec![0],
            ..analyze_index(&s)
        };
        let r = analyze_loops(&s, &index, None);
        assert!(r.skipped.as_deref().unwrap().starts_with("長さが合わない"));

        // der の形に完全マッチングが無いのに、S7 は微分が要らないと言っている（食い違った入力）
        let s = structure(&["a", "b"], &[("a = 1", &[0]), ("a = 2", &[0])]);
        let index = IndexReport {
            index: 1,
            differentiations: vec![0, 0],
            orders: vec![0, 0],
            ..IndexReport::default()
        };
        let r = analyze_loops(&s, &index, None);
        assert!(
            r.skipped
                .as_deref()
                .unwrap()
                .starts_with("解く系に完全マッチングが無い")
        );
        assert!(r.diagnostics(&s, None).is_empty());
    }

    /// 方程式 n 本。`cycle` なら e_i は u_i と u_{i+1 mod n}（1 つの大きなループ）、でなければ
    /// e_i は u_i と u_{i+1}（e_0 が e_1 に、e_1 が e_2 に…と依存する鎖。Tarjan の深さが n になる）
    fn long(n: usize, cycle: bool) -> DaeStructure {
        let names: Vec<String> = (0..n).map(|i| format!("z{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let rows: Vec<Vec<u32>> = (0..n)
            .map(|i| {
                let mut r = vec![i as u32];
                if i + 1 < n {
                    r.push(i as u32 + 1);
                } else if cycle {
                    r.insert(0, 0);
                }
                r
            })
            .collect();
        let eqs: Vec<(&str, &[u32])> = rows.iter().map(|r| ("e", r.as_slice())).collect();
        with_nonlinear(structure(&names, &eqs), &vec![&[][..]; n])
    }

    #[test]
    fn long_chain_does_not_overflow_the_stack() {
        let n = 100_000;
        let s = long(n, false);
        let index = analyze_index(&s);
        let r = analyze_loops(&s, &index, None);
        assert_eq!(r.blocks, n);
        assert!(r.loops.is_empty());
    }

    #[test]
    fn one_large_loop_is_torn_by_one_variable() {
        let n = 100_000;
        let s = long(n, true);
        let index = analyze_index(&s);
        let r = analyze_loops(&s, &index, None);
        assert_eq!(r.blocks, 1);
        assert_eq!(r.loop_sizes(), [n]);
        let l = &r.loops[0];
        assert!(!l.nonlinear);
        assert_eq!(l.tearing, [0]);
        assert_eq!(l.residuals.len(), 1);
        assert_valid_tearing(&s, &index, l);
    }

    /// 乱択の正方で構造正則な構造（行の並べ替えた対角を必ず含む）。非線形の参照も乱択。
    fn random_structure(rng: &mut XorShift, n: usize) -> DaeStructure {
        let mut perm: Vec<u32> = (0..n as u32).collect();
        for i in (1..n).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            perm.swap(i, j);
        }
        let density = [5, 15, 30, 60][rng.below(4) as usize];
        let mut rows = Vec::with_capacity(n);
        let mut nonlinear = Vec::with_capacity(n);
        for &p in &perm {
            let mut row: Vec<u32> = (0..n as u32)
                .filter(|&u| u == p || rng.below(100) < density)
                .collect();
            row.dedup();
            let nl: Vec<u32> = row.iter().copied().filter(|_| rng.below(3) == 0).collect();
            rows.push(row);
            nonlinear.push(nl);
        }
        let names: Vec<String> = (0..n).map(|i| format!("z{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let eqs: Vec<(&str, &[u32])> = rows.iter().map(|r| ("e", r.as_slice())).collect();
        let mut s = structure(&names, &eqs);
        s.nonlinear = nonlinear;
        s
    }

    #[test]
    fn random_structures_satisfy_blt_and_tearing_properties() {
        let mut rng = XorShift::new(0x5eed_0009);
        for _ in 0..300 {
            let n = 1 + rng.below(14) as usize;
            let s = random_structure(&mut rng, n);
            let index = analyze_index(&s);
            let starts: Vec<bool> = (0..n).map(|_| rng.below(2) == 0).collect();
            let init = init_with_starts(&starts);
            let r = analyze_loops(&s, &index, Some(&init));
            assert_eq!(r.skipped, None);

            // BLT: 依存の先は同じか前のブロック
            let system = System::new(&s, &index).unwrap();
            let mt = hopcroft_karp(n, n, &system.rows);
            let mate_u: Vec<usize> = mt.unknown_to_equation.iter().map(|e| e.unwrap()).collect();
            let b = blt(&system, &mate_u);
            assert_eq!(b.n_blocks(), r.blocks);
            assert_eq!(b.order.len(), n);
            for e in 0..n {
                for &u in &system.rows[e] {
                    assert!(b.block_of[mate_u[u as usize]] <= b.block_of[e]);
                }
            }
            // 同じブロック ⇔ 互いに届く（推移閉包と比べる。n ≤ 14 なのでビット列で持つ）
            let mut reach: Vec<u32> = (0..n)
                .map(|e| {
                    system.rows[e]
                        .iter()
                        .fold(1 << e, |acc, &u| acc | 1 << mate_u[u as usize])
                })
                .collect();
            for k in 0..n {
                let via = reach[k];
                for r in reach.iter_mut().filter(|r| **r >> k & 1 == 1) {
                    *r |= via;
                }
            }
            for (i, &ri) in reach.iter().enumerate() {
                for (j, &rj) in reach.iter().enumerate() {
                    let mutual = ri >> j & 1 == 1 && rj >> i & 1 == 1;
                    assert_eq!(b.block_of[i] == b.block_of[j], mutual);
                }
            }

            for l in &r.loops {
                assert!(l.size() >= 2);
                assert_valid_tearing(&s, &index, l);
                let nonlinear = l.equations.iter().any(|&e| {
                    system
                        .row(e)
                        .any(|(u, nl)| nl && l.unknowns.binary_search(&u).is_ok())
                });
                assert_eq!(l.nonlinear, nonlinear);
                // start 値の無い反復変数は、非線形のループの tearing 変数で start 値の無いもの
                let expected: Vec<usize> = if l.nonlinear {
                    l.tearing.iter().copied().filter(|&u| !starts[u]).collect()
                } else {
                    Vec::new()
                };
                assert_eq!(l.without_start, expected);
            }

            // ブロックの大きさは方程式と未知数の並べ替え（= マッチングの選び方）によらない
            let mut pe: Vec<usize> = (0..n).collect();
            let mut pu: Vec<usize> = (0..n).collect();
            for p in [&mut pe, &mut pu] {
                for i in (1..n).rev() {
                    let j = rng.below(i as u64 + 1) as usize;
                    p.swap(i, j);
                }
            }
            let mut t = s.clone();
            for (e, &pe) in pe.iter().enumerate() {
                let map = |r: &Vec<u32>| {
                    let mut r: Vec<u32> = r.iter().map(|&u| pu[u as usize] as u32).collect();
                    r.sort_unstable();
                    r
                };
                t.incidence[pe] = map(&s.incidence[e]);
                t.nonlinear[pe] = map(&s.nonlinear[e]);
            }
            let rt = analyze_loops(&t, &analyze_index(&t), None);
            assert_eq!(rt.loop_sizes(), r.loop_sizes());
            assert_eq!(rt.blocks, r.blocks);
            assert_eq!(rt.n_nonlinear(), r.n_nonlinear());
        }
    }

    #[test]
    fn out_of_range_names_use_index() {
        let s = nonlinear_loop();
        let r = run(&s, Some(&init_with_starts(&[false, false])));
        // 位置の無い（未知数の無い）構造を渡しても panic しない
        let empty = DaeStructure::default();
        let ds = r.diagnostics(&empty, None);
        assert_eq!(ds.len(), 2);
        assert_eq!(
            ds[0].notes[0],
            "一番大きい非線形のループの未知数（2 個）: `#0`, `#1`"
        );
        assert_eq!(ds[0].file, None);
        assert_eq!(ds[1].range, None);
    }
}

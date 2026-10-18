//! S7: Pantelides 法によるインデックスの見積もりと、状態選択の候補（S006）。
//!
//! 入力は [`DaeStructure`]（der の形の `incidence` と、状態そのものの参照 `state_refs`）。
//! S6 を通った（`x` と `der(x)` をまとめたグラフに完全マッチングがある）構造に対して、
//! 方程式ごとに何回微分すれば最高階の微分が完全にマッチするかを求める。
//!
//! # グラフの持ち方
//!
//! 変数は「族」ごとに階数を持つ。族は [`DaeStructure::unknowns`] の 1 項目に対応する。
//!
//! - 状態 `x_i` の族（`Derivative` の未知数 `der(x_i)`）は、階数 0（`x_i`）と 1（`der(x_i)`）から始まる。
//! - 代数 `z_j` の族（`Algebraic` / `Output`）は、階数 0 から始まる。
//! - 方程式 e は、`state_refs[e]` の `x_i` を階数 0 で、`incidence[e]` の `der(x_i)` を階数 1 で、`z_j` を階数 0 で参照する。
//! - 方程式 e を d 回微分したものは、元の参照（族 f, 階数 o）ごとに階数 o..=o+d を参照する。
//!
//! マッチングの対象は「最高階」のノード（族 f の階数 `top[f]`）だけ。微分した方程式のうち
//! 最高階のノードに届くのは階数 o+d の参照だけなので、方程式 e（の最後に微分したもの）と族 f は、
//! `o + d[e] == top[f]` のとき辺で結ばれる。Pantelides の「微分した方程式と上げた変数の対応を引き継ぐ」
//! （`A(der(j)) = B(A(j))`）は、この持ち方では「族と方程式の対応をそのまま残す」になる。
//!
//! # アルゴリズム（Pantelides 1988、Mattsson–Söderlind 1993 の説明）
//!
//! 1. der の形（`incidence`）の最大マッチング（Hopcroft–Karp）から始める。der の形は、初めの最高階のグラフそのもの。
//! 2. 対応のない方程式 e ごとに、彩色つきの交互路探索 `augmentpath(e)` を回す。
//! 3. 失敗したら、彩色した方程式をすべて 1 回微分し、彩色した族の最高階を 1 つ上げて、e で再試行する。
//!    失敗 1 回ごとに、彩色した族が c 個なら方程式は c + 1 本微分するので、`Σ top − Σ d` は 1 ずつ減る。
//!
//! 結果の `(d, top)` は Pryce の Σ 法のオフセット（`c_i = d[e]`、`d_j = top[f]`）の条件を満たし、
//! `Σ top − Σ d` は Σ 法の自由度（最大の横断の値）になる（[`IndexReport::dynamic_states`]）。
//! どの状態の `der(x)` もどこかの方程式に現れる構造では、微分の回数は Σ 法の最小のオフセットと
//! 一致する（乱択のテスト `differentiations_are_pryce_canonical_offsets` で確かめている。証明はしていない）。
//!
//! Rumoca に依存しない。wasm32 でも動く。

use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_ir::{DaeStructure, UnknownKind};
use serde::Serialize;

use crate::bipartite::Bipartite;
use crate::{list, list_by, locate, matching};

/// S7 の結果。添字は [`DaeStructure`] の `equations` / `unknowns` のもの。
///
/// 位置を持たないので、[`DaeStructure::without_ranges`] を渡しても同じ値になる（防火壁 2、ADR-0005）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct IndexReport {
    /// 構造インデックスの見込み（方程式の微分の最大回数 + 1。微分が要らなければ 1。
    /// 未知数が無ければ 0。打ち切ったときも 0）
    pub index: usize,
    /// 方程式ごとの微分の回数（長さは `equations.len()`）
    pub differentiations: Vec<u32>,
    /// 未知数ごとの、最終的に現れる最高階の微分の階数（長さは `unknowns.len()`）。
    /// `Derivative` の未知数は状態 `x` から数えた階数（元は 1）、Algebraic / Output は元は 0
    pub orders: Vec<u32>,
    /// 状態選択（dummy derivative）の候補の状態（`Derivative` の未知数の添字、昇順）。
    ///
    /// 1 回以上微分した方程式が参照する状態（`state_refs` の `x`、または `incidence` の `der(x)` で届くもの）。
    /// このうち `状態の数 − dynamic_states` 個が、状態選択で代数変数に回る見込み
    pub dummy_candidates: Vec<usize>,
    /// 1 回以上微分した方程式の数
    pub n_differentiated: usize,
    /// 動的な状態の数の見込み（自由度。積分器の数）:
    /// `状態の数 + Σ_u（orders[u] − 初期の階数）− Σ_e differentiations[e]`（初期の階数は Derivative が 1、
    /// Algebraic / Output が 0）。拡張した系の未知数 − 方程式で、Pryce の Σ 法の自由度と同じ。
    /// = 状態の数 − 微分で行き詰まった回数
    pub dynamic_states: usize,
    /// 打ち切った（方程式数と未知数が違う、構造特異、または微分の回数の上限）。S6 を通った構造では起きないはず
    pub failed: bool,
}

impl IndexReport {
    /// 微分の最大回数。
    pub fn max_differentiations(&self) -> u32 {
        self.differentiations.iter().copied().max().unwrap_or(0)
    }

    /// 診断（S7）。
    ///
    /// - 打ち切ったら `GM-E7001`（Error）だけ。
    /// - インデックスが 2 以上なら `GM-I7001`（Info。位置は一番多く微分した方程式。補足に微分した方程式と回数）。
    /// - 動的な状態が状態の数より少なければ `GM-I7002`（Info。位置は最初の候補の宣言。補足に候補の状態）。
    ///
    /// `structure` は、この結果を作ったときと同じ（位置のある）`DaeStructure` を渡す。
    /// 添字が範囲外なら、名前の代わりに `#添字` と書く。
    pub fn diagnostics(&self, structure: &DaeStructure) -> Vec<Diagnostic> {
        let label = |e: usize| {
            structure
                .equations
                .get(e)
                .map(|r| r.label.as_str())
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("#{e}"), str::to_string)
        };
        let at_equation = |d: Diagnostic, e: usize| {
            let row = structure.equations.get(e);
            locate(
                d,
                row.and_then(|r| r.file.as_deref()),
                row.and_then(|r| r.range),
            )
        };
        let state = |u: usize| {
            structure
                .unknowns
                .get(u)
                .map(|x| state_name(&x.name))
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("#{u}"), str::to_string)
        };
        let max = self.max_differentiations();
        // 微分した方程式（回数の多い順、同じ回数なら添字の順）
        let mut differentiated: Vec<usize> = (0..self.differentiations.len())
            .filter(|&e| self.differentiations[e] > 0)
            .collect();
        differentiated.sort_by_key(|&e| (std::cmp::Reverse(self.differentiations[e]), e));
        let differentiated_note = || {
            format!(
                "微分した方程式（{} 本）: {}",
                differentiated.len(),
                list_by(&differentiated, |e| format!(
                    "`{}`（{} 回）",
                    label(e),
                    self.differentiations[e]
                ))
            )
        };

        if self.failed {
            let (n, m) = (structure.equations.len(), structure.unknowns.len());
            let d = if n != m {
                Diagnostic::error(
                    codes::INDEX_REDUCTION_FAILED,
                    Stage::S7,
                    format!("インデックスを求められない: 方程式 {n} / 未知数 {m} で数が合わない"),
                )
            } else if let Some(&worst) = differentiated.first() {
                // 構造特異なら微分する前に打ち切るので、微分した方程式があるのは回数の上限のとき
                at_equation(
                    Diagnostic::error(
                        codes::INDEX_REDUCTION_FAILED,
                        Stage::S7,
                        format!(
                            "インデックスの低減が終わらない: 方程式 `{}` を {max} 回微分しても、最高階の微分に対応づけられない（上限は方程式数 + 1 = {} 回）",
                            label(worst),
                            differentiation_limit(n)
                        ),
                    ),
                    worst,
                )
                .note(differentiated_note())
            } else {
                Diagnostic::error(
                    codes::INDEX_REDUCTION_FAILED,
                    Stage::S7,
                    "インデックスを求められない: 構造特異（x と der(x) をまとめた二部グラフに完全マッチングが無い）",
                )
                .note(format!("S6 の {} を参照", codes::SINGULAR))
            };
            return vec![d];
        }

        let mut out = Vec::new();
        if self.index >= 2 {
            let d = Diagnostic::info(
                codes::HIGH_INDEX,
                Stage::S7,
                format!(
                    "構造インデックス {} の見込み（Pantelides 法: 方程式 {} 本を微分し、最大 {max} 回）",
                    self.index, self.n_differentiated
                ),
            );
            let d = match differentiated.first() {
                Some(&e) => at_equation(d, e),
                None => d,
            };
            out.push(d.note(differentiated_note()));
        }
        let n_states = structure
            .unknowns
            .iter()
            .filter(|u| u.kind == UnknownKind::Derivative)
            .count();
        if self.dynamic_states < n_states {
            let first = self
                .dummy_candidates
                .first()
                .and_then(|&u| structure.unknowns.get(u));
            let mut d = locate(
                Diagnostic::info(
                    codes::STATE_SELECTION,
                    Stage::S7,
                    format!(
                        "状態 {n_states} 個のうち動的な状態は {} 個の見込み。状態選択（dummy derivative）が要る",
                        self.dynamic_states
                    ),
                ),
                first.and_then(|u| u.file.as_deref()),
                first.and_then(|u| u.range),
            )
            .note(format!(
                "候補の状態（{} 個）: {}",
                self.dummy_candidates.len(),
                list(&self.dummy_candidates, state)
            ));
            let raised: Vec<usize> = self
                .dummy_candidates
                .iter()
                .copied()
                .filter(|&u| self.orders.get(u).is_some_and(|&o| o > 1))
                .collect();
            if !raised.is_empty() {
                d = d.note(format!(
                    "2 階以上の微分が現れる状態（{} 個）: {}",
                    raised.len(),
                    list_by(&raised, |u| format!(
                        "`{}`（{} 階）",
                        state(u),
                        self.orders[u]
                    ))
                ));
            }
            out.push(d);
        }
        out
    }
}

/// `der(x[2])` → `x[2]`（状態の名前）。`der(` で始まらなければそのまま。
fn state_name(name: &str) -> &str {
    name.strip_prefix("der(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(name)
}

/// 1 本の方程式を微分してよい回数の上限（方程式数 + 1）。
fn differentiation_limit(n_equations: usize) -> u64 {
    n_equations as u64 + 1
}

/// Pantelides 法を回す。
///
/// 方程式数と未知数が違うか、S6 の意味で構造特異なら、すぐに打ち切る（`failed`、`index = 0`）。
/// incidence と state_refs の範囲外の添字や、足りない行は捨てる（[`crate::analyze`] と同じ）。
pub fn analyze_index(structure: &DaeStructure) -> IndexReport {
    run(structure, differentiation_limit(structure.equations.len())).report
}

/// [`analyze_index`] の中身。テストのために、最後のマッチングも返す。
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) report: IndexReport,
    /// 方程式ごとの、対応づけた族（打ち切ったときは途中のもの）。読むのはテストだけ
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) mate: Vec<Option<usize>>,
}

const NONE: usize = usize::MAX;

/// Pantelides 法の作業場。
#[derive(Debug)]
struct Pantelides {
    /// 方程式ごとの（族, 元の階数）。族の昇順で、族ごとに一番高い階数だけ
    refs: Vec<Vec<(usize, u32)>>,
    /// 方程式ごとの微分の回数
    diff: Vec<u32>,
    /// 族ごとの最高階
    top: Vec<u32>,
    /// 方程式 → 族 / 族 → 方程式（無ければ NONE）
    mate_e: Vec<usize>,
    mate_f: Vec<usize>,
    /// 彩色（直前の augment で塗ったもの）
    colored_e: Vec<bool>,
    colored_f: Vec<bool>,
    painted_e: Vec<usize>,
    painted_f: Vec<usize>,
    stack: Vec<(usize, usize)>,
}

impl Pantelides {
    fn new(structure: &DaeStructure) -> Self {
        let n = structure.equations.len();
        let m = structure.unknowns.len();
        let base = |u: usize| u32::from(structure.unknowns[u].kind == UnknownKind::Derivative);
        let mut refs = Vec::with_capacity(n);
        for e in 0..n {
            let mut row: Vec<(usize, u32)> = Vec::new();
            if let Some(inc) = structure.incidence.get(e) {
                // der(x) は階数 1、代数は階数 0（どちらも族の元の最高階）
                row.extend(
                    inc.iter()
                        .map(|&u| u as usize)
                        .filter(|&u| u < m)
                        .map(|u| (u, base(u))),
                );
            }
            if let Some(st) = structure.state_refs.get(e) {
                // 状態そのもの x は階数 0（代数を指していたら、その代数そのもの = 階数 0）
                row.extend(
                    st.iter()
                        .map(|&u| u as usize)
                        .filter(|&u| u < m)
                        .map(|u| (u, 0)),
                );
            }
            // 族ごとに一番高い階数だけを残す（低い方は最高階に届かない）
            row.sort_unstable_by_key(|&(f, o)| (f, std::cmp::Reverse(o)));
            row.dedup_by_key(|&mut (f, _)| f);
            refs.push(row);
        }
        Self {
            refs,
            diff: vec![0; n],
            top: (0..m).map(base).collect(),
            mate_e: vec![NONE; n],
            mate_f: vec![NONE; m],
            colored_e: vec![false; n],
            colored_f: vec![false; m],
            painted_e: Vec::new(),
            painted_f: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// 方程式 e と族 f（元の階数 o で参照）が、最高階どうしで結ばれているか。
    fn is_highest(&self, e: usize, f: usize, o: u32) -> bool {
        o + self.diff[e] == self.top[f]
    }

    /// 方程式 e が参照する、まだ対応のない最高階の族。
    fn free_highest(&self, e: usize) -> Option<usize> {
        self.refs[e]
            .iter()
            .find(|&&(f, o)| self.is_highest(e, f, o) && self.mate_f[f] == NONE)
            .map(|&(f, _)| f)
    }

    fn paint_e(&mut self, e: usize) {
        self.colored_e[e] = true;
        self.painted_e.push(e);
    }

    /// 彩色つきの交互路探索（Pantelides の AUGMENTPATH）。見つかれば対応を付け替えて true。
    ///
    /// 失敗したとき、`painted_e` / `painted_f` に塗った方程式と族が残る。再帰を使わずスタックで歩く。
    fn augment(&mut self, start: usize) -> bool {
        for &e in &self.painted_e {
            self.colored_e[e] = false;
        }
        for &f in &self.painted_f {
            self.colored_f[f] = false;
        }
        self.painted_e.clear();
        self.painted_f.clear();
        self.stack.clear();

        self.paint_e(start);
        if let Some(g) = self.free_highest(start) {
            self.mate_e[start] = g;
            self.mate_f[g] = start;
            return true;
        }
        self.stack.push((start, 0));
        while let Some(&(e, mut pos)) = self.stack.last() {
            // まだ塗っていない最高階の族を探す（対応のない族は無い: 積むときに確かめた）
            let mut next = None;
            while let Some(&(f, o)) = self.refs[e].get(pos) {
                pos += 1;
                if self.is_highest(e, f, o) && !self.colored_f[f] {
                    next = Some(f);
                    break;
                }
            }
            if let Some(top) = self.stack.last_mut() {
                top.1 = pos;
            }
            let Some(f) = next else {
                self.stack.pop();
                continue;
            };
            self.colored_f[f] = true;
            self.painted_f.push(f);
            let w = self.mate_f[f];
            debug_assert_ne!(w, NONE, "対応のない族は free_highest で拾っている");
            self.paint_e(w);
            if let Some(g) = self.free_highest(w) {
                // 増加路: g → w、f → e、e の元の族 → その下の方程式 …
                self.mate_e[w] = g;
                self.mate_f[g] = w;
                let mut via = f;
                for k in (0..self.stack.len()).rev() {
                    let ek = self.stack[k].0;
                    let prev = self.mate_e[ek];
                    self.mate_e[ek] = via;
                    self.mate_f[via] = ek;
                    via = prev;
                }
                debug_assert_eq!(via, NONE, "探索の根は対応のない方程式");
                return true;
            }
            self.stack.push((w, 0));
        }
        false
    }

    /// 直前の augment で塗った方程式を 1 回微分し、塗った族の最高階を 1 つ上げる。
    fn differentiate_painted(&mut self) {
        for &e in &self.painted_e {
            self.diff[e] += 1;
        }
        for &f in &self.painted_f {
            self.top[f] += 1;
        }
    }
}

/// Pantelides 法の本体。1 本の方程式の微分の回数が `limit` を超えたら打ち切る。
pub(crate) fn run(structure: &DaeStructure, limit: u64) -> Run {
    let n = structure.equations.len();
    let m = structure.unknowns.len();
    let base_orders: Vec<u32> = structure
        .unknowns
        .iter()
        .map(|u| u32::from(u.kind == UnknownKind::Derivative))
        .collect();
    let give_up = |differentiations: Vec<u32>, orders: Vec<u32>, mate: Vec<Option<usize>>| Run {
        report: IndexReport {
            index: 0,
            n_differentiated: differentiations.iter().filter(|&&d| d > 0).count(),
            differentiations,
            orders,
            dummy_candidates: Vec::new(),
            dynamic_states: 0,
            failed: true,
        },
        mate,
    };
    if n != m {
        return give_up(vec![0; n], base_orders, vec![None; n]);
    }
    if n == 0 {
        return Run {
            report: IndexReport::default(),
            mate: Vec::new(),
        };
    }
    // S6 と同じ判定: まとめた形に完全マッチングが無ければ、Pantelides 法は終わらない
    let merged = Bipartite::union(n, m, &[&structure.incidence, &structure.state_refs]);
    if matching::maximum_matching(&merged).size != n {
        return give_up(vec![0; n], base_orders, vec![None; n]);
    }

    let mut p = Pantelides::new(structure);
    // der の形（初めの最高階のグラフ）の最大マッチングから始める
    let der_form = matching::maximum_matching(&Bipartite::new(n, m, &structure.incidence));
    for (e, u) in der_form.equation_to_unknown.iter().enumerate() {
        if let Some(u) = *u {
            p.mate_e[e] = u;
            p.mate_f[u] = e;
        }
    }
    let mut stuck = 0usize;
    for k in 0..n {
        if p.mate_e[k] != NONE {
            continue;
        }
        while !p.augment(k) {
            p.differentiate_painted();
            stuck += 1;
            // 塗った方程式には必ず k が入るので、k の回数を見れば足りる
            if u64::from(p.diff[k]) > limit {
                let mate = p.mate_e.iter().map(|&f| (f != NONE).then_some(f)).collect();
                return give_up(p.diff, p.top, mate);
            }
        }
    }

    let max = p.diff.iter().copied().max().unwrap_or(0);
    // 候補: 1 回以上微分した方程式が参照する状態（x でも der(x) でも）
    let mut candidate = vec![false; m];
    for (row, _) in p.refs.iter().zip(&p.diff).filter(|(_, d)| **d > 0) {
        for &(f, _) in row {
            if structure.unknowns[f].kind == UnknownKind::Derivative {
                candidate[f] = true;
            }
        }
    }
    let n_states = base_orders.iter().filter(|&&o| o == 1).count();
    debug_assert_eq!(
        p.top.iter().map(|&t| i64::from(t)).sum::<i64>()
            - p.diff.iter().map(|&d| i64::from(d)).sum::<i64>(),
        n_states as i64 - stuck as i64,
        "失敗 1 回ごとに Σ top − Σ d は 1 減る"
    );
    let report = IndexReport {
        index: max as usize + 1,
        n_differentiated: p.diff.iter().filter(|&&d| d > 0).count(),
        dummy_candidates: (0..m).filter(|&f| candidate[f]).collect(),
        dynamic_states: n_states.saturating_sub(stuck),
        differentiations: p.diff,
        orders: p.top,
        failed: false,
    };
    let mate = p.mate_e.iter().map(|&f| (f != NONE).then_some(f)).collect();
    Run { report, mate }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{XorShift, dae, equation, structure, unknown, with_state_refs};
    use crate::{NOTE_LIST_LIMIT, analyze};
    use globe_mode_core::Severity;
    use globe_mode_ir::Unknown;

    /// 直交座標の平面の振り子（Index/Pendulum.mo と同じ形）。未知数は der(x), der(y), der(vx), der(vy), lambda
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

    /// Invalid/HighIndexDuplicate.mo と同じ形。未知数は der(vC), vR, i
    fn high_index_duplicate() -> DaeStructure {
        dae(
            &["vC"],
            &["vR", "i"],
            &[
                ("C * der(vC) = i", &["der(vC)", "i"]),
                ("V = vR + vC", &["vR", "vC"]),
                ("vR = V - vC", &["vR", "vC"]),
            ],
        )
    }

    /// Index/ParallelCapacitors.mo と同じ形。未知数は der(v1), der(v2), i1, i2, i
    fn parallel_capacitors() -> DaeStructure {
        dae(
            &["v1", "v2"],
            &["i1", "i2", "i"],
            &[
                ("C1 * der(v1) = i1", &["der(v1)", "i1"]),
                ("C2 * der(v2) = i2", &["der(v2)", "i2"]),
                ("v1 = v2", &["v1", "v2"]),
                ("i1 + i2 = i", &["i1", "i2", "i"]),
                ("i = I", &["i"]),
            ],
        )
    }

    /// 最後のマッチングが、最高階どうしの完全マッチングで、どの参照も最高階を超えないこと。
    fn assert_pantelides_invariants(s: &DaeStructure, run: &Run) {
        let r = &run.report;
        let p = Pantelides::new(s);
        let n = s.equations.len();
        let mut used = vec![false; s.unknowns.len()];
        for e in 0..n {
            let d = r.differentiations[e];
            for &(f, o) in &p.refs[e] {
                assert!(
                    o + d <= r.orders[f],
                    "方程式 {e} の参照 ({f}, {o}) が最高階 {} を超える",
                    r.orders[f]
                );
            }
            let f = run.mate[e].unwrap_or_else(|| panic!("方程式 {e} に対応が無い"));
            assert!(!used[f], "族 {f} が 2 回使われている");
            used[f] = true;
            let o = p.refs[e]
                .iter()
                .find(|&&(g, _)| g == f)
                .map(|&(_, o)| o)
                .unwrap_or_else(|| panic!("方程式 {e} は族 {f} を参照しない"));
            assert_eq!(
                o + d,
                r.orders[f],
                "方程式 {e} と族 {f} が最高階で結ばれていない"
            );
        }
        // 階数は元より下がらない
        for (u, x) in s.unknowns.iter().enumerate() {
            let base = u32::from(x.kind == UnknownKind::Derivative);
            assert!(r.orders[u] >= base);
        }
    }

    #[test]
    fn pendulum_is_index_three() {
        let s = pendulum();
        let r = analyze_index(&s);
        assert!(!r.failed);
        // 拘束を 2 回、der(x) = vx と der(y) = vy を 1 回ずつ
        assert_eq!(r.differentiations, [1, 1, 0, 0, 2]);
        assert_eq!(r.index, 3);
        assert_eq!(r.max_differentiations(), 2);
        assert_eq!(r.n_differentiated, 3);
        // x と y は 2 階まで、vx と vy は 1 階、lambda は 0 階
        assert_eq!(r.orders, [2, 2, 1, 1, 0]);
        // 微分した方程式（拘束、der(x) = vx、der(y) = vy）が参照する状態
        assert_eq!(r.dummy_candidates, [0, 1, 2, 3]);
        // 自由度 = 状態 4 + 上がった階数 2（x, y）− 微分 4 = 2（位置 1 つと速度 1 つ）
        assert_eq!(r.dynamic_states, 2);
        assert_pantelides_invariants(&s, &run(&s, 100));
        // S6 の判定と食い違わない
        assert!(analyze(&s).index_reduction_needed);

        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 2, "{ds:#?}");
        assert!(
            ds.iter()
                .all(|d| d.stage == Stage::S7 && d.severity == Severity::Info)
        );
        assert_eq!(ds[0].code, codes::HIGH_INDEX);
        assert_eq!(
            ds[0].message,
            "構造インデックス 3 の見込み（Pantelides 法: 方程式 3 本を微分し、最大 2 回）"
        );
        // 位置は一番多く微分した方程式（拘束）
        assert_eq!(ds[0].range, s.equations[4].range);
        assert_eq!(ds[0].file.as_deref(), Some(crate::testutil::FILE));
        assert_eq!(
            ds[0].notes,
            [
                "微分した方程式（3 本）: `x^2 + y^2 = L^2`（2 回）, `der(x) = vx`（1 回）, `der(y) = vy`（1 回）"
            ]
        );
        assert_eq!(ds[1].code, codes::STATE_SELECTION);
        assert_eq!(
            ds[1].message,
            "状態 4 個のうち動的な状態は 2 個の見込み。状態選択（dummy derivative）が要る"
        );
        assert_eq!(ds[1].range, s.unknowns[0].range);
        assert_eq!(
            ds[1].notes,
            [
                "候補の状態（4 個）: `x`, `y`, `vx`, `vy`",
                "2 階以上の微分が現れる状態（2 個）: `x`（2 階）, `y`（2 階）"
            ]
        );
    }

    #[test]
    fn high_index_duplicate_is_index_two() {
        let s = high_index_duplicate();
        let r = analyze_index(&s);
        assert!(!r.failed);
        // 電圧則を 2 本とも 1 回ずつ微分する
        assert_eq!(r.differentiations, [0, 1, 1]);
        assert_eq!((r.index, r.n_differentiated), (2, 2));
        assert_eq!(r.orders, [1, 1, 0]);
        // 状態の階数は上がらないが、微分した電圧則が vC を参照する
        assert_eq!(r.dummy_candidates, [0]);
        // 2 本の電圧則が vR と vC を代数的に決めるので、構造上の自由度は 0
        // （状態 1 + 上がった階数 1（vR が 1 階に）− 微分 2）
        assert_eq!(r.dynamic_states, 0);
        assert_pantelides_invariants(&s, &run(&s, 100));

        let ds = r.diagnostics(&s);
        let codes_: Vec<&str> = ds.iter().map(|d| d.code).collect();
        assert_eq!(codes_, [codes::HIGH_INDEX, codes::STATE_SELECTION]);
        assert_eq!(
            ds[0].message,
            "構造インデックス 2 の見込み（Pantelides 法: 方程式 2 本を微分し、最大 1 回）"
        );
        assert_eq!(ds[0].range, s.equations[1].range);
        assert_eq!(
            ds[1].message,
            "状態 1 個のうち動的な状態は 0 個の見込み。状態選択（dummy derivative）が要る"
        );
        // 2 階以上の状態は無いので、補足は候補だけ
        assert_eq!(ds[1].notes, ["候補の状態（1 個）: `vC`"]);
    }

    #[test]
    fn parallel_capacitors_are_index_two() {
        let s = parallel_capacitors();
        let r = analyze_index(&s);
        assert!(!r.failed);
        // v1 = v2 だけを 1 回微分する
        assert_eq!(r.differentiations, [0, 0, 1, 0, 0]);
        assert_eq!((r.index, r.n_differentiated), (2, 1));
        assert_eq!(r.orders, [1, 1, 0, 0, 0]);
        assert_eq!(r.dummy_candidates, [0, 1]);
        // v1 = v2 なので、動的な状態は 1 個（状態 2 + 上がった階数 0 − 微分 1）。
        // 「状態 − Σ(状態の階数 − 1)」では 2 になって誤る
        assert_eq!(r.dynamic_states, 1);
        assert_pantelides_invariants(&s, &run(&s, 100));
        let ds = r.diagnostics(&s);
        let codes_: Vec<&str> = ds.iter().map(|d| d.code).collect();
        assert_eq!(codes_, [codes::HIGH_INDEX, codes::STATE_SELECTION]);
        assert_eq!(
            ds[1].message,
            "状態 2 個のうち動的な状態は 1 個の見込み。状態選択（dummy derivative）が要る"
        );
        assert_eq!(ds[1].notes, ["候補の状態（2 個）: `v1`, `v2`"]);
    }

    #[test]
    fn index_one_dae_needs_no_differentiation() {
        // der(x) = -z; z = 2 * x;（der の形で完全マッチングがある）
        let s = dae(
            &["x"],
            &["z"],
            &[
                ("der(x) = -z", &["der(x)", "z"]),
                ("z = 2 * x", &["z", "x"]),
            ],
        );
        assert!(!analyze(&s).index_reduction_needed);
        let r = analyze_index(&s);
        assert_eq!(r.index, 1);
        assert_eq!(r.differentiations, [0, 0]);
        assert_eq!(r.orders, [1, 0]);
        assert_eq!(r.n_differentiated, 0);
        assert!(r.dummy_candidates.is_empty());
        assert_eq!(r.dynamic_states, 1);
        assert!(!r.failed);
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn pure_ode_is_index_one() {
        let s = dae(
            &["x", "y", "z"],
            &[],
            &[
                ("der(x) = -x", &["der(x)", "x"]),
                ("der(y) = x - y", &["der(y)", "x", "y"]),
                ("der(z) = y", &["der(z)", "y"]),
            ],
        );
        let r = analyze_index(&s);
        assert_eq!(r.index, 1);
        assert_eq!(r.dynamic_states, 3);
        assert_eq!(r.orders, [1, 1, 1]);
        assert!(r.dummy_candidates.is_empty() && r.n_differentiated == 0 && !r.failed);
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn empty_structure() {
        let s = DaeStructure::default();
        let r = analyze_index(&s);
        assert_eq!(r, IndexReport::default());
        assert_eq!(r.index, 0);
        assert!(!r.failed);
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn structurally_singular_gives_up() {
        // x = 1; x = 2;（未知数 x, y）: y を決める方程式が無い
        let s = structure(&["x", "y"], &[("x = 1", &[0]), ("x = 2", &[0])]);
        assert!(!analyze(&s).is_regular());
        let r = analyze_index(&s);
        assert!(r.failed);
        assert_eq!(r.index, 0);
        assert_eq!(r.differentiations, [0, 0]);
        assert!(r.dummy_candidates.is_empty());
        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 1);
        assert_eq!(
            (ds[0].code, ds[0].stage, ds[0].severity),
            (codes::INDEX_REDUCTION_FAILED, Stage::S7, Severity::Error)
        );
        assert!(ds[0].message.contains("構造特異"), "{}", ds[0].message);
        assert_eq!(ds[0].notes, [format!("S6 の {} を参照", codes::SINGULAR)]);
    }

    #[test]
    fn non_square_gives_up() {
        let s = structure(&["x", "y"], &[("x + y = 1", &[0, 1])]);
        let r = analyze_index(&s);
        assert!(r.failed);
        assert_eq!(
            (r.index, r.differentiations.len(), r.orders.len()),
            (0, 1, 2)
        );
        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].code, codes::INDEX_REDUCTION_FAILED);
        assert_eq!(
            ds[0].message,
            "インデックスを求められない: 方程式 1 / 未知数 2 で数が合わない"
        );
    }

    #[test]
    fn differentiation_limit_stops_the_loop() {
        // 振り子は拘束を 2 回微分する。上限 1 回なら、2 回目の微分で打ち切る
        let s = pendulum();
        let r = run(&s, 1).report;
        assert!(r.failed);
        assert_eq!(r.index, 0);
        assert_eq!(r.differentiations[4], 2);
        assert!(r.dummy_candidates.is_empty() && r.dynamic_states == 0);
        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 1);
        assert_eq!(
            (ds[0].code, ds[0].severity),
            (codes::INDEX_REDUCTION_FAILED, Severity::Error)
        );
        // 微分した方程式があるので、回数の上限として書く。位置は一番多く微分した方程式
        assert_eq!(
            ds[0].message,
            "インデックスの低減が終わらない: 方程式 `x^2 + y^2 = L^2` を 2 回微分しても、最高階の微分に対応づけられない（上限は方程式数 + 1 = 6 回）"
        );
        assert_eq!(ds[0].range, s.equations[4].range);
        assert!(ds[0].notes[0].starts_with("微分した方程式（3 本）"));
        // 上限 2 回なら通る
        assert!(!run(&s, 2).report.failed);
    }

    #[test]
    fn array_elements_are_separate_families() {
        // x[1] は 2 回微分した拘束に現れ、x[2] は独立した ODE
        let s = dae(
            &["x[1]", "x[2]", "v"],
            &["z"],
            &[
                ("der(x[1]) = v", &["der(x[1])", "v"]),
                ("der(v) = z", &["der(v)", "z"]),
                ("x[1] = sin(time)", &["x[1]"]),
                ("der(x[2]) = -x[2]", &["der(x[2])", "x[2]"]),
            ],
        );
        let r = analyze_index(&s);
        assert_eq!(r.index, 3);
        assert_eq!(r.differentiations, [1, 0, 2, 0]);
        // der(x[1]) の族だけが 2 階に上がる。der(x[2]) の族は 1 階のまま
        assert_eq!(r.orders, [2, 1, 1, 0]);
        assert_eq!(r.dummy_candidates, [0, 2]);
        assert_eq!(r.dynamic_states, 1);
        let ds = r.diagnostics(&s);
        let selection = ds
            .iter()
            .find(|d| d.code == codes::STATE_SELECTION)
            .expect("GM-I7002");
        assert_eq!(
            selection.notes,
            [
                "候補の状態（2 個）: `x[1]`, `v`",
                "2 階以上の微分が現れる状態（1 個）: `x[1]`（2 階）"
            ]
        );
    }

    #[test]
    fn differentiated_list_is_capped() {
        // x_j = sin(time)（j = 0..12）: どの方程式も状態 x_j だけを含み、1 回ずつ微分する
        let names: Vec<String> = (0..12).map(|j| format!("der(x{j})")).collect();
        let labels: Vec<String> = (0..12).map(|j| format!("x{j} = sin(time)")).collect();
        let s = DaeStructure {
            unknowns: names
                .iter()
                .enumerate()
                .map(|(i, n)| unknown(i, n))
                .collect(),
            equations: labels
                .iter()
                .enumerate()
                .map(|(i, l)| equation(i, l))
                .collect(),
            incidence: vec![Vec::new(); 12],
            state_refs: (0..12u32).map(|j| vec![j]).collect(),
        };
        let r = analyze_index(&s);
        assert_eq!((r.index, r.n_differentiated, r.dynamic_states), (2, 12, 0));
        let ds = r.diagnostics(&s);
        let note = &ds[0].notes[0];
        assert!(
            note.starts_with("微分した方程式（12 本）: `x0 = sin(time)`（1 回）, "),
            "{note}"
        );
        assert!(
            note.ends_with("`x9 = sin(time)`（1 回） ほか 2 件"),
            "{note}"
        );
        assert_eq!(note.matches('`').count(), 2 * NOTE_LIST_LIMIT);
        let candidates = &ds[1].notes[0];
        assert!(candidates.ends_with("`x9` ほか 2 件"), "{candidates}");
    }

    #[test]
    fn report_does_not_depend_on_positions() {
        for s in [pendulum(), high_index_duplicate(), parallel_capacitors()] {
            let stripped = s.without_ranges();
            assert_ne!(s, stripped);
            let r = analyze_index(&s);
            assert_eq!(r, analyze_index(&stripped));
            let with = r.diagnostics(&s);
            let without = r.diagnostics(&stripped);
            assert_eq!(with.len(), without.len());
            for (a, b) in with.iter().zip(&without) {
                assert_eq!(
                    (&a.message, &a.notes, &a.file),
                    (&b.message, &b.notes, &b.file)
                );
                assert!(a.range.is_some() && b.range.is_none());
            }
        }
    }

    #[test]
    fn malformed_structure_does_not_panic() {
        // 範囲外の添字、足りない行、余った行、代数を指す state_refs、空のラベル
        let base = with_state_refs(
            structure(
                &["der(x)", "y"],
                &[("der(x) = y", &[0, 1, 7]), ("", &[u32::MAX])],
            ),
            &[&[], &[0, 0, 99]],
        );
        for (incidence, state_refs) in [
            (base.incidence.clone(), base.state_refs.clone()),
            (vec![vec![0, 1]], vec![]),
            (vec![], vec![vec![1], vec![0]]),
            (
                vec![vec![0, 1], vec![], vec![5]],
                vec![vec![], vec![0], vec![1]],
            ),
        ] {
            let mut s = base.clone();
            s.incidence = incidence;
            s.state_refs = state_refs;
            let r = analyze_index(&s);
            let _ = r.diagnostics(&s);
            let _ = r.diagnostics(&DaeStructure::default());
            assert_eq!(analyze_index(&s.without_ranges()), r);
            assert_eq!(r.differentiations.len(), 2);
            assert_eq!(r.orders.len(), 2);
        }
        // 空のラベルは添字で書く: x = sin(time) と同じ形（状態だけの拘束）
        let r = analyze_index(&base);
        assert_eq!(r.index, 2);
        let ds = r.diagnostics(&base);
        assert!(ds[0].notes[0].contains("`#1`（1 回）"), "{:?}", ds[0].notes);
    }

    /// 乱択の正方な構造（状態と代数を混ぜる。state_refs は状態だけを指す）。
    fn random_structure(rng: &mut XorShift, max: usize) -> DaeStructure {
        let n = rng.below(max as u64 + 1) as usize;
        let unknowns: Vec<Unknown> = (0..n)
            .map(|i| {
                if rng.below(2) == 0 {
                    unknown(i, &format!("der(x{i})"))
                } else {
                    unknown(i, &format!("z{i}"))
                }
            })
            .collect();
        let states: Vec<u32> = (0..n as u32)
            .filter(|&u| unknowns[u as usize].kind == UnknownKind::Derivative)
            .collect();
        let (_, _, incidence) = rng.bipartite_sized(n, n);
        let mut state_refs = Vec::with_capacity(n);
        for _ in 0..n {
            let row: Vec<u32> = states
                .iter()
                .copied()
                .filter(|_| rng.below(100) < 30)
                .collect();
            state_refs.push(row);
        }
        DaeStructure {
            unknowns,
            equations: (0..n).map(|i| equation(i, &format!("e{i}"))).collect(),
            incidence,
            state_refs,
        }
    }

    /// 総当たりの最大の横断: まとめた形の完全マッチングのうち、Σ（族の元の階数）が最大のもの。
    /// Pryce の Σ 法の自由度（Σ d_j − Σ c_i）はこれに等しい。完全マッチングが無ければ None。
    fn max_transversal(p: &Pantelides) -> Option<u32> {
        fn go(p: &Pantelides, e: usize, used: &mut [bool], sum: u32, best: &mut Option<u32>) {
            if e == p.refs.len() {
                *best = Some(best.map_or(sum, |b| b.max(sum)));
                return;
            }
            for &(f, o) in &p.refs[e] {
                if !used[f] {
                    used[f] = true;
                    go(p, e + 1, used, sum + o, best);
                    used[f] = false;
                }
            }
        }
        let mut best = None;
        go(p, 0, &mut vec![false; p.top.len()], 0, &mut best);
        best
    }

    #[test]
    fn agrees_with_structural_report_on_random_structures() {
        let mut rng = XorShift::new(0x9A47_E11D);
        let (mut regular, mut index2, mut index3) = (0, 0, 0);
        for _ in 0..3000 {
            let s = random_structure(&mut rng, 6);
            let structural = analyze(&s);
            let out = run(&s, differentiation_limit(s.equations.len()));
            let r = &out.report;
            assert_eq!(*r, analyze_index(&s.without_ranges()));
            // S6 の der の形のフラグと、Pantelides 法のインデックスは同値
            assert_eq!(
                structural.index_reduction_needed,
                r.index >= 2,
                "{s:?}\n{r:?}"
            );
            let ds = r.diagnostics(&s);
            if structural.is_regular() {
                regular += 1;
                assert!(!r.failed, "S6 を通ったのに打ち切った: {s:?}");
                assert_pantelides_invariants(&s, &out);
                // 自由度は Σ 法の最大の横断の値（Pantelides の選び方によらない）
                let p = Pantelides::new(&s);
                assert_eq!(
                    Some(r.dynamic_states as u32),
                    max_transversal(&p),
                    "{s:?}\n{r:?}"
                );
                let sum_orders: u32 = r.orders.iter().sum();
                let sum_diff: u32 = r.differentiations.iter().sum();
                assert_eq!(sum_orders - sum_diff, r.dynamic_states as u32);
                assert_eq!(
                    r.index,
                    if s.unknowns.is_empty() {
                        0
                    } else {
                        r.max_differentiations() as usize + 1
                    }
                );
                assert_eq!(
                    r.n_differentiated,
                    r.differentiations.iter().filter(|&&d| d > 0).count()
                );
                // 診断は Info だけ。GM-I7001 はインデックスが 2 以上のときだけ
                assert!(ds.iter().all(|d| d.severity == Severity::Info));
                assert_eq!(ds.iter().any(|d| d.code == codes::HIGH_INDEX), r.index >= 2);
                let n_states = s
                    .unknowns
                    .iter()
                    .filter(|u| u.kind == UnknownKind::Derivative)
                    .count();
                assert_eq!(
                    ds.iter().any(|d| d.code == codes::STATE_SELECTION),
                    r.dynamic_states < n_states
                );
                // 動的な状態が減るなら、候補の状態がある
                if r.dynamic_states < n_states {
                    assert!(!r.dummy_candidates.is_empty(), "{s:?}\n{r:?}");
                }
                // 候補は状態だけ。微分が無ければ候補も無い
                assert!(
                    r.dummy_candidates
                        .iter()
                        .all(|&u| s.unknowns[u].kind == UnknownKind::Derivative)
                );
                if r.index < 2 {
                    assert!(r.dummy_candidates.is_empty());
                }
                index2 += usize::from(r.index == 2);
                index3 += usize::from(r.index >= 3);
            } else {
                assert!(r.failed && r.index == 0);
                assert_eq!(ds.len(), 1);
                assert_eq!(ds[0].code, codes::INDEX_REDUCTION_FAILED);
            }
        }
        assert!(regular > 300, "正則な例が {regular} 件しかない");
        assert!(index2 > 50, "インデックス 2 の例が {index2} 件しかない");
        assert!(index3 > 5, "インデックス 3 以上の例が {index3} 件しかない");
    }

    /// Pryce の Σ 法の正準なオフセット c（最小のもの）。`transversal` は最大の横断（方程式 → 族）。
    ///
    /// c = 0 から、d_j = max_i (σ_ij + c_i)、c_i = d_{T(i)} − σ_{i,T(i)} を繰り返す（Pryce 2001）。
    fn pryce_offsets(p: &Pantelides, transversal: &[usize]) -> Vec<u32> {
        let n = p.refs.len();
        let sigma = |e: usize, f: usize| {
            p.refs[e]
                .iter()
                .find(|&&(g, _)| g == f)
                .map(|&(_, o)| o)
                .expect("横断の辺")
        };
        let mut c = vec![0u32; n];
        loop {
            let mut d = vec![0u32; p.top.len()];
            for (row, &ce) in p.refs.iter().zip(&c) {
                for &(f, o) in row {
                    d[f] = d[f].max(o + ce);
                }
            }
            let next: Vec<u32> = (0..n)
                .map(|e| d[transversal[e]] - sigma(e, transversal[e]))
                .collect();
            if next == c {
                return c;
            }
            c = next;
        }
    }

    #[test]
    fn differentiations_are_pryce_canonical_offsets() {
        // どの状態の der(x) もどこかの方程式に現れる（Modelica の状態の定義）構造では、
        // Pantelides 法の微分の回数は Σ 法の最小のオフセットと一致する。
        // der(x) がどこにも現れないと、Σ 法は x を代数変数とみなすので一致しない（ここでは作らない）
        let mut rng = XorShift::new(0x0FF5_E750);
        let mut compared = 0;
        let mut high = 0;
        for _ in 0..3000 {
            let mut s = random_structure(&mut rng, 6);
            let n = s.equations.len();
            for u in 0..n {
                let used = s.incidence.iter().any(|r| r.contains(&(u as u32)));
                if s.unknowns[u].kind == UnknownKind::Derivative && !used {
                    let e = rng.below(n as u64) as usize;
                    s.incidence[e].push(u as u32);
                }
            }
            let out = run(&s, differentiation_limit(n));
            if out.report.failed {
                continue;
            }
            let p = Pantelides::new(&s);
            let transversal: Vec<usize> = out.mate.iter().map(|f| f.expect("完全")).collect();
            assert_eq!(
                out.report.differentiations,
                pryce_offsets(&p, &transversal),
                "{s:?}"
            );
            compared += 1;
            high += usize::from(out.report.index >= 2);
        }
        assert!(compared > 1000, "比べた例が {compared} 件しかない");
        assert!(high > 100, "インデックス 2 以上の例が {high} 件しかない");
    }

    #[test]
    fn long_augmenting_path_does_not_overflow_the_stack() {
        // A: der(x) = z_0、B_k: z_k + z_{k+1} = 0（k < n）、C: x = sin(time)。
        // der の形の最大マッチングは A → der(x)、B_k → z_k で、z_n が残る。C を 1 回微分すると、
        // 増加路 C' → der(x) → A → z_0 → B_0 → … → B_{n-1} → z_n（長さ n）が 1 本だけある
        let n = 200_000usize;
        let m = n + 2;
        let mut unknowns = vec![unknown(0, "der(x)")];
        unknowns.extend((0..=n).map(|k| unknown(k + 1, &format!("z{k}"))));
        let mut incidence = vec![vec![0u32, 1]];
        incidence.extend((0..n as u32).map(|k| vec![k + 1, k + 2]));
        incidence.push(Vec::new());
        let mut state_refs = vec![Vec::new(); n + 1];
        state_refs.push(vec![0u32]);
        let s = DaeStructure {
            unknowns,
            equations: (0..m).map(|i| equation(i, &format!("e{i}"))).collect(),
            incidence,
            state_refs,
        };
        let out = run(&s, differentiation_limit(m));
        let r = &out.report;
        assert!(!r.failed);
        assert_eq!((r.index, r.n_differentiated, r.dynamic_states), (2, 1, 0));
        assert_eq!(r.differentiations[m - 1], 1);
        assert_eq!(r.dummy_candidates, [0]);
        // 増加路で全部付け替わる: C → der(x)、A → z_0、B_k → z_{k+1}
        assert_eq!(out.mate[m - 1], Some(0));
        assert_eq!(out.mate[0], Some(1));
        assert_eq!(out.mate[n], Some(n + 1));
    }

    #[test]
    fn report_serializes() {
        fn is_serialize<T: Serialize>(_: &T) {}
        is_serialize(&analyze_index(&pendulum()));
    }
}

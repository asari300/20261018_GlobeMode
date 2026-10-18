//! 粗い Dulmage–Mendelsohn 分解: 最大マッチングから、過剰な部分・不足している部分・適定な部分に分ける。
//!
//! - 過剰（over-determined）: 対応のない方程式から、交互路（方程式 → その方程式が含む未知数 →
//!   その未知数の相手の方程式）で届く方程式と、それらが含む未知数。方程式の数が未知数より多い。
//! - 不足（under-determined）: 対応のない未知数から、交互路（未知数 → それを含む方程式 →
//!   その方程式の相手の未知数）で届く未知数と、それらを含む方程式。未知数の数が方程式より多い。
//! - 適定（well-determined）: 残り。方程式と未知数が同じ数で、その中で完全マッチングがある。
//!
//! 最大マッチングを渡せば、分け方はどの最大マッチングを選んでも同じになる（DM の定理）。

use serde::Serialize;

use crate::bipartite::Bipartite;
use crate::matching::Matching;

/// 粗い DM 分解。添字はすべて昇順。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct Decomposition {
    /// 過剰な部分の方程式
    pub over_equations: Vec<usize>,
    /// 過剰な部分の未知数（過剰な方程式が含む未知数のすべて）
    pub over_unknowns: Vec<usize>,
    /// 不足している部分の方程式（不足している未知数を含む方程式のすべて）
    pub under_equations: Vec<usize>,
    /// 不足している部分の未知数
    pub under_unknowns: Vec<usize>,
    /// 適定な部分の方程式
    pub well_equations: Vec<usize>,
    /// 適定な部分の未知数
    pub well_unknowns: Vec<usize>,
}

/// incidence と、その最大マッチング（[`crate::hopcroft_karp`]）から粗い DM 分解を作る。
///
/// 方程式と未知数の数は `matching` の長さで決める。incidence の範囲外の添字は捨てる。
/// 最大でないマッチングを渡すと、過剰と不足が重なることがある（そのときは両方に入れ、適定には入れない）。
pub fn dulmage_mendelsohn(incidence: &[Vec<u32>], matching: &Matching) -> Decomposition {
    let g = Bipartite::new(matching.n_equations(), matching.n_unknowns(), incidence);
    decompose(&g, matching)
}

pub(crate) fn decompose(g: &Bipartite, matching: &Matching) -> Decomposition {
    let (n, m) = (g.n_rows(), g.n_cols());
    // マッチングの添字も信用しない（範囲外は対応なしとみなす）
    let mate_e = |e: usize| {
        matching
            .equation_to_unknown
            .get(e)
            .copied()
            .flatten()
            .filter(|&u| u < m)
    };
    let mate_u = |u: usize| {
        matching
            .unknown_to_equation
            .get(u)
            .copied()
            .flatten()
            .filter(|&e| e < n)
    };

    // 過剰: 対応のない方程式から
    let mut over_e = vec![false; n];
    let mut over_u = vec![false; m];
    let mut stack: Vec<usize> = (0..n).filter(|&e| mate_e(e).is_none()).collect();
    for &e in &stack {
        over_e[e] = true;
    }
    while let Some(e) = stack.pop() {
        for &u in g.row(e) {
            if over_u[u] {
                continue;
            }
            over_u[u] = true;
            if let Some(f) = mate_u(u)
                && !over_e[f]
            {
                over_e[f] = true;
                stack.push(f);
            }
        }
    }

    // 不足: 対応のない未知数から（未知数ごとの方程式の一覧 = 転置を歩く）
    let t = g.transpose();
    let mut under_e = vec![false; n];
    let mut under_u = vec![false; m];
    let mut stack: Vec<usize> = (0..m).filter(|&u| mate_u(u).is_none()).collect();
    for &u in &stack {
        under_u[u] = true;
    }
    while let Some(u) = stack.pop() {
        for &e in t.row(u) {
            if under_e[e] {
                continue;
            }
            under_e[e] = true;
            if let Some(v) = mate_e(e)
                && !under_u[v]
            {
                under_u[v] = true;
                stack.push(v);
            }
        }
    }

    let pick = |flags: &[bool]| -> Vec<usize> {
        flags
            .iter()
            .enumerate()
            .filter(|(_, f)| **f)
            .map(|(i, _)| i)
            .collect()
    };
    let rest = |a: &[bool], b: &[bool]| -> Vec<usize> {
        (0..a.len()).filter(|&i| !a[i] && !b[i]).collect()
    };
    Decomposition {
        over_equations: pick(&over_e),
        over_unknowns: pick(&over_u),
        under_equations: pick(&under_e),
        under_unknowns: pick(&under_u),
        well_equations: rest(&over_e, &under_e),
        well_unknowns: rest(&over_u, &under_u),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hopcroft_karp;
    use crate::testutil::{XorShift, brute_force_max_matchings, cleaned};

    fn dm(n: usize, m: usize, inc: &[Vec<u32>]) -> Decomposition {
        dulmage_mendelsohn(inc, &hopcroft_karp(n, m, inc))
    }

    #[test]
    fn perfect_matching_is_all_well_determined() {
        let d = dm(3, 3, &[vec![0, 1], vec![1, 2], vec![2]]);
        assert!(d.over_equations.is_empty() && d.over_unknowns.is_empty());
        assert!(d.under_equations.is_empty() && d.under_unknowns.is_empty());
        assert_eq!(d.well_equations, [0, 1, 2]);
        assert_eq!(d.well_unknowns, [0, 1, 2]);
    }

    #[test]
    fn two_equations_on_one_unknown() {
        // x = 1; x = 2;（未知数 x, y）
        let d = dm(2, 2, &[vec![0], vec![0]]);
        assert_eq!(d.over_equations, [0, 1]);
        assert_eq!(d.over_unknowns, [0]);
        assert!(d.under_equations.is_empty());
        assert_eq!(d.under_unknowns, [1]);
        assert!(d.well_equations.is_empty() && d.well_unknowns.is_empty());
    }

    #[test]
    fn three_parts_at_once() {
        // e0: {u0}, e1: {u0}           過剰（u0 に 2 本）
        // e2: {u1, u2}                 不足（1 本で 2 個）
        // e3: {u3, u4}, e4: {u4}       適定。e3 は過剰の u0 も含んでよい（適定 → 過剰の向きの辺）
        let inc = [vec![0], vec![0], vec![1, 2], vec![0, 3, 4], vec![4]];
        let d = dm(5, 5, &inc);
        assert_eq!(d.over_equations, [0, 1]);
        assert_eq!(d.over_unknowns, [0]);
        assert_eq!(d.under_equations, [2]);
        assert_eq!(d.under_unknowns, [1, 2]);
        assert_eq!(d.well_equations, [3, 4]);
        assert_eq!(d.well_unknowns, [3, 4]);
    }

    #[test]
    fn non_maximum_matching_is_tolerated() {
        // 空のマッチング（最大でない）を渡しても panic せず、重なりは適定から外す
        let inc = [vec![0]];
        let empty = Matching {
            equation_to_unknown: vec![None],
            unknown_to_equation: vec![None],
            size: 0,
        };
        let d = dulmage_mendelsohn(&inc, &empty);
        assert_eq!(d.over_equations, [0]);
        assert_eq!(d.under_unknowns, [0]);
        assert!(d.well_equations.is_empty() && d.well_unknowns.is_empty());
        // 範囲外の相手を指すマッチングも対応なしとみなす
        let broken = Matching {
            equation_to_unknown: vec![Some(9)],
            unknown_to_equation: vec![Some(9)],
            size: 1,
        };
        assert_eq!(dulmage_mendelsohn(&inc, &broken), d);
    }

    /// DM 分解の性質をまとめて確かめる。
    fn assert_dm_properties(
        n: usize,
        m: usize,
        inc: &[Vec<u32>],
        mt: &Matching,
        d: &Decomposition,
    ) {
        let rows = cleaned(n, m, inc);
        let mut eq_part = vec![0u8; n];
        let mut unk_part = vec![0u8; m];
        // 3 つの部分は互いに交わらず、全体を覆う
        for (part, es, us) in [
            (1u8, &d.over_equations, &d.over_unknowns),
            (2, &d.under_equations, &d.under_unknowns),
            (3, &d.well_equations, &d.well_unknowns),
        ] {
            assert!(es.windows(2).all(|w| w[0] < w[1]) && us.windows(2).all(|w| w[0] < w[1]));
            for &e in es {
                assert_eq!(eq_part[e], 0, "方程式 {e} が 2 つの部分に入っている");
                eq_part[e] = part;
            }
            for &u in us {
                assert_eq!(unk_part[u], 0, "未知数 {u} が 2 つの部分に入っている");
                unk_part[u] = part;
            }
        }
        assert!(eq_part.iter().all(|&p| p != 0) && unk_part.iter().all(|&p| p != 0));
        // 過剰な方程式は過剰な未知数しか含まない。不足している未知数は不足している方程式にしか現れない
        for (e, row) in rows.iter().enumerate() {
            for &u in row {
                if eq_part[e] == 1 {
                    assert_eq!(
                        unk_part[u], 1,
                        "過剰な方程式 {e} が部分の外の未知数 {u} を含む"
                    );
                }
                if unk_part[u] == 2 {
                    assert_eq!(
                        eq_part[e], 2,
                        "不足している未知数 {u} が部分の外の方程式 {e} に現れる"
                    );
                }
            }
        }
        // 数: 過剰は方程式が対応のない分だけ多く、不足は未知数が多い。適定は同じ数
        let free_e = mt.unmatched_equations().len();
        let free_u = mt.unmatched_unknowns().len();
        assert_eq!(d.over_equations.len(), d.over_unknowns.len() + free_e);
        assert_eq!(d.under_unknowns.len(), d.under_equations.len() + free_u);
        assert_eq!(d.well_equations.len(), d.well_unknowns.len());
        // 適定の部分の中では、マッチングが閉じている（完全マッチング）
        for &e in &d.well_equations {
            let u = mt.equation_to_unknown[e].expect("適定な方程式は対応を持つ");
            assert_eq!(unk_part[u], 3);
        }
    }

    #[test]
    fn decomposition_is_independent_of_the_maximum_matching() {
        let mut rng = XorShift::new(0xDA1A_6E00);
        // 乱択のグラフが、特異で最大マッチングが複数ある場合を十分に含むことも確かめる
        let mut interesting = 0;
        for _ in 0..1500 {
            let (n, m, inc) = rng.bipartite(6, 6);
            let hk = hopcroft_karp(n, m, &inc);
            let expected = dulmage_mendelsohn(&inc, &hk);
            assert_dm_properties(n, m, &inc, &hk, &expected);
            // 別の最大マッチングから作っても同じになる
            let (_, all) = brute_force_max_matchings(n, m, &inc, 32);
            if all.len() > 1
                && !expected.over_equations.is_empty()
                && !expected.under_unknowns.is_empty()
            {
                interesting += 1;
            }
            for other in &all {
                assert_eq!(
                    dulmage_mendelsohn(&inc, other),
                    expected,
                    "n={n} m={m} inc={inc:?} matching={other:?}"
                );
            }
        }
        assert!(
            interesting > 100,
            "過剰と不足が両方ある例が {interesting} 件しかない"
        );
    }

    #[test]
    fn properties_hold_on_larger_random_graphs() {
        let mut rng = XorShift::new(0x0B16_0DA7);
        for _ in 0..200 {
            let (n, m, inc) = rng.bipartite(60, 60);
            let hk = hopcroft_karp(n, m, &inc);
            assert_dm_properties(n, m, &inc, &hk, &dulmage_mendelsohn(&inc, &hk));
        }
    }
}

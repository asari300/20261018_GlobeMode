//! 二部グラフ（方程式 × 未知数）の最大マッチング。Hopcroft–Karp 法で O(E√V)。

use serde::Serialize;

use crate::bipartite::Bipartite;

/// 最大マッチング。方程式と未知数の 1 対 1 の対応づけ。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct Matching {
    /// 方程式ごとの、対応づけた未知数（無ければ None）。長さは方程式の数
    pub equation_to_unknown: Vec<Option<usize>>,
    /// 未知数ごとの、対応づけた方程式（無ければ None）。長さは未知数の数
    pub unknown_to_equation: Vec<Option<usize>>,
    /// 対応づけた組の数
    pub size: usize,
}

impl Matching {
    pub fn n_equations(&self) -> usize {
        self.equation_to_unknown.len()
    }

    pub fn n_unknowns(&self) -> usize {
        self.unknown_to_equation.len()
    }

    /// すべての方程式とすべての未知数が対応づいている（方程式数 = 未知数で、構造特異でない）。
    pub fn is_perfect(&self) -> bool {
        self.size == self.n_equations() && self.size == self.n_unknowns()
    }

    /// 対応づかなかった方程式（昇順）。
    pub fn unmatched_equations(&self) -> Vec<usize> {
        unmatched(&self.equation_to_unknown)
    }

    /// 対応づかなかった未知数（昇順）。
    pub fn unmatched_unknowns(&self) -> Vec<usize> {
        unmatched(&self.unknown_to_equation)
    }
}

fn unmatched(mate: &[Option<usize>]) -> Vec<usize> {
    mate.iter()
        .enumerate()
        .filter(|(_, m)| m.is_none())
        .map(|(i, _)| i)
        .collect()
}

/// 方程式 `n_equations` 本 × 未知数 `n_unknowns` 個の二部グラフの最大マッチング（Hopcroft–Karp）。
///
/// `incidence[e]` は方程式 `e` が含む未知数の添字。並びと重複は問わない。
/// 範囲外の添字は捨てる。`incidence` の行が足りなければ、その方程式は未知数を含まないものとして扱う。
pub fn hopcroft_karp(n_equations: usize, n_unknowns: usize, incidence: &[Vec<u32>]) -> Matching {
    maximum_matching(&Bipartite::new(n_equations, n_unknowns, incidence))
}

/// 対応が無いことを表す印（内部の配列用）。
const NONE: usize = usize::MAX;
/// BFS で届かなかった層。
const INF: usize = usize::MAX;

pub(crate) fn maximum_matching(g: &Bipartite) -> Matching {
    let (n, m) = (g.n_rows(), g.n_cols());
    let mut mate_e = vec![NONE; n];
    let mut mate_u = vec![NONE; m];
    let mut size = 0;

    // 貪欲法で初期のマッチングを作る（多くの DAE はこれだけでほぼ埋まる）
    for (e, mate) in mate_e.iter_mut().enumerate() {
        if let Some(&u) = g.row(e).iter().find(|&&u| mate_u[u] == NONE) {
            *mate = u;
            mate_u[u] = e;
            size += 1;
        }
    }

    let mut dist = vec![INF; n];
    let mut next = vec![0usize; n];
    let mut queue = Vec::with_capacity(n);
    let mut stack: Vec<usize> = Vec::new();
    loop {
        // BFS: 対応のない方程式を層 0 とし、交互路（方程式 → 未知数 → その相手の方程式）で層を付ける。
        // 対応のない未知数に最初に届いた層（limit）より先は付けない（最短の増加路だけを使う）
        queue.clear();
        for e in 0..n {
            if mate_e[e] == NONE {
                dist[e] = 0;
                queue.push(e);
            } else {
                dist[e] = INF;
            }
        }
        let mut limit = INF;
        let mut head = 0;
        while head < queue.len() {
            let e = queue[head];
            head += 1;
            if dist[e] >= limit {
                break; // queue は層の順なので、ここから先はすべて limit 以上
            }
            for &u in g.row(e) {
                let w = mate_u[u];
                if w == NONE {
                    limit = limit.min(dist[e]);
                } else if dist[w] == INF {
                    dist[w] = dist[e] + 1;
                    queue.push(w);
                }
            }
        }
        if limit == INF {
            break; // 増加路が無い: 最大マッチング
        }

        // DFS: 層を 1 つずつ下る増加路を、互いに頂点を共有しないように探して付け替える。
        // 増加路は長くなりうる（数万本の方程式の連鎖）ので、再帰を使わずスタックで歩く
        next.fill(0);
        for root in 0..n {
            if mate_e[root] != NONE || dist[root] != 0 {
                continue;
            }
            stack.clear();
            stack.push(root);
            while let Some(&e) = stack.last() {
                let row = g.row(e);
                let Some(&u) = row.get(next[e]) else {
                    // 行き止まり: このフェーズではもう通らない
                    dist[e] = INF;
                    stack.pop();
                    if let Some(&parent) = stack.last() {
                        next[parent] += 1;
                    }
                    continue;
                };
                let w = mate_u[u];
                if w == NONE {
                    if dist[e] == limit {
                        // 増加路が見つかった: スタックの各方程式を、いま見ている未知数に付け替える
                        for &x in &stack {
                            let y = g.row(x)[next[x]];
                            mate_e[x] = y;
                            mate_u[y] = x;
                            dist[x] = INF; // このフェーズでは同じ頂点を 2 度使わない
                        }
                        size += 1;
                        break;
                    }
                    next[e] += 1;
                } else if dist[e] < limit && dist[w] == dist[e] + 1 {
                    stack.push(w);
                } else {
                    next[e] += 1;
                }
            }
        }
    }

    let to_option = |x: usize| (x != NONE).then_some(x);
    Matching {
        equation_to_unknown: mate_e.into_iter().map(to_option).collect(),
        unknown_to_equation: mate_u.into_iter().map(to_option).collect(),
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{XorShift, assert_valid_matching, brute_force_max_matchings, kuhn_size};

    #[test]
    fn empty_graphs() {
        let m = hopcroft_karp(0, 0, &[]);
        assert_eq!(m, Matching::default());
        assert!(m.is_perfect());

        let m = hopcroft_karp(2, 0, &[vec![], vec![]]);
        assert_eq!(m.size, 0);
        assert_eq!(m.unmatched_equations(), [0, 1]);
        assert!(!m.is_perfect());

        let m = hopcroft_karp(0, 3, &[]);
        assert_eq!(m.unmatched_unknowns(), [0, 1, 2]);
    }

    #[test]
    fn perfect_matching_needs_augmentation() {
        // 貪欲法だと e0 → u0 で e1 が詰まる。増加路 e1 → u0 → e0 → u1 で直す
        let inc = [vec![0, 1], vec![0]];
        let m = hopcroft_karp(2, 2, &inc);
        assert!(m.is_perfect());
        assert_eq!(m.equation_to_unknown, [Some(1), Some(0)]);
        assert_eq!(m.unknown_to_equation, [Some(1), Some(0)]);
        assert_valid_matching(2, 2, &inc, &m);
    }

    #[test]
    fn non_square_graphs() {
        // 方程式 3 本 × 未知数 2 個
        let inc = [vec![0], vec![0, 1], vec![1]];
        let m = hopcroft_karp(3, 2, &inc);
        assert_eq!(m.size, 2);
        assert_eq!(m.unmatched_equations().len(), 1);
        assert!(m.unmatched_unknowns().is_empty());
        assert_valid_matching(3, 2, &inc, &m);

        // 方程式 1 本 × 未知数 3 個
        let inc = [vec![2, 1]];
        let m = hopcroft_karp(1, 3, &inc);
        assert_eq!(m.size, 1);
        assert_eq!(m.unmatched_unknowns().len(), 2);
        assert_valid_matching(1, 3, &inc, &m);
    }

    #[test]
    fn bad_rows_are_ignored() {
        // 範囲外の添字（5、u32::MAX）と重複は捨てる。行が足りない方程式は未知数を含まない
        let inc = [vec![5, 0, 0, u32::MAX]];
        let m = hopcroft_karp(3, 2, &inc);
        assert_eq!(m.size, 1);
        assert_eq!(m.equation_to_unknown, [Some(0), None, None]);
        assert_valid_matching(3, 2, &inc, &m);
        // 行が多すぎても、余った行は見ない
        let m = hopcroft_karp(1, 1, &[vec![0], vec![0]]);
        assert_eq!(m.size, 1);
        assert_eq!(m.equation_to_unknown.len(), 1);
    }

    #[test]
    fn long_augmenting_path_does_not_overflow_the_stack() {
        // e_i = {u_i, u_{i+1}}（i < n-1）、e_{n-1} = {u_0}。貪欲法は e_i → u_i で e_{n-1} が詰まり、
        // 長さ n の増加路 e_{n-1} → u_0 → e_0 → u_1 → … → u_{n-1} が 1 本だけ残る
        let n = 200_000usize;
        let mut inc: Vec<Vec<u32>> = (0..n - 1).map(|i| vec![i as u32, i as u32 + 1]).collect();
        inc.push(vec![0]);
        let m = hopcroft_karp(n, n, &inc);
        assert!(m.is_perfect());
        assert_eq!(m.equation_to_unknown[n - 1], Some(0));
        assert_eq!(m.equation_to_unknown[0], Some(1));
    }

    #[test]
    fn agrees_with_brute_force_on_small_random_graphs() {
        let mut rng = XorShift::new(0x5EED_0005);
        for _ in 0..2000 {
            let (n, m, inc) = rng.bipartite(6, 6);
            let got = hopcroft_karp(n, m, &inc);
            assert_valid_matching(n, m, &inc, &got);
            let (best, _) = brute_force_max_matchings(n, m, &inc, 1);
            assert_eq!(got.size, best, "n={n} m={m} inc={inc:?}");
        }
    }

    #[test]
    fn agrees_with_kuhn_on_larger_random_graphs() {
        let mut rng = XorShift::new(0xD0E5_0006);
        for _ in 0..200 {
            let (n, m, inc) = rng.bipartite(80, 80);
            let got = hopcroft_karp(n, m, &inc);
            assert_valid_matching(n, m, &inc, &got);
            assert_eq!(got.size, kuhn_size(n, m, &inc), "n={n} m={m}");
        }
    }
}

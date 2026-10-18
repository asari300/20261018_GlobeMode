//! テスト用の道具: 種を固定した乱数、総当たりの最大マッチング、Kuhn 法、マッチングの検算。

use crate::bipartite::Bipartite;
use crate::matching::Matching;

/// xorshift64（種を固定して、毎回同じグラフを作る）。
pub(crate) struct XorShift(u64);

impl XorShift {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub(crate) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// 方程式 0..=max_n 本 × 未知数 0..=max_m 個の乱択の二部グラフ。
    /// 行の並びはばらばらで、ときどき重複と範囲外の添字を混ぜる（掃除されることを確かめるため）。
    pub(crate) fn bipartite(
        &mut self,
        max_n: usize,
        max_m: usize,
    ) -> (usize, usize, Vec<Vec<u32>>) {
        let n = self.below(max_n as u64 + 1) as usize;
        let m = self.below(max_m as u64 + 1) as usize;
        self.bipartite_sized(n, m)
    }

    /// 方程式 n 本 × 未知数 m 個の乱択の二部グラフ（[`Self::bipartite`] の大きさを決めた版）。
    pub(crate) fn bipartite_sized(&mut self, n: usize, m: usize) -> (usize, usize, Vec<Vec<u32>>) {
        // 辺の密度（%）。疎なグラフほど特異になりやすい
        let density = [8, 20, 35, 60, 90][self.below(5) as usize];
        let mut inc = Vec::with_capacity(n);
        for _ in 0..n {
            let mut row: Vec<u32> = (0..m as u32)
                .filter(|_| self.below(100) < density)
                .collect();
            if !row.is_empty() && self.below(4) == 0 {
                let dup = row[self.below(row.len() as u64) as usize];
                row.push(dup);
            }
            if self.below(8) == 0 {
                row.push(m as u32 + self.below(3) as u32);
            }
            // 並びを崩す
            for i in (1..row.len()).rev() {
                let j = self.below(i as u64 + 1) as usize;
                row.swap(i, j);
            }
            inc.push(row);
        }
        (n, m, inc)
    }
}

/// 掃除した行（範囲外を捨て、昇順・重複なし）。
pub(crate) fn cleaned(n: usize, m: usize, inc: &[Vec<u32>]) -> Vec<Vec<usize>> {
    let g = Bipartite::new(n, m, inc);
    (0..n).map(|e| g.row(e).to_vec()).collect()
}

/// 総当たりで最大マッチングの大きさと、最大マッチングを `keep` 個まで返す。小さなグラフ専用。
pub(crate) fn brute_force_max_matchings(
    n: usize,
    m: usize,
    inc: &[Vec<u32>],
    keep: usize,
) -> (usize, Vec<Matching>) {
    struct Search<'a> {
        rows: &'a [Vec<usize>],
        mate_e: Vec<Option<usize>>,
        mate_u: Vec<Option<usize>>,
        best: usize,
        found: Vec<Matching>,
        keep: usize,
    }
    impl Search<'_> {
        fn go(&mut self, e: usize, size: usize) {
            if size + (self.rows.len() - e) < self.best {
                return; // 残りを全部対応づけても届かない
            }
            if e == self.rows.len() {
                if size > self.best {
                    self.best = size;
                    self.found.clear();
                }
                if self.found.len() < self.keep {
                    self.found.push(Matching {
                        equation_to_unknown: self.mate_e.clone(),
                        unknown_to_equation: self.mate_u.clone(),
                        size,
                    });
                }
                return;
            }
            // e を対応づけない
            self.go(e + 1, size);
            // e を空いている未知数のどれかに対応づける
            for k in 0..self.rows[e].len() {
                let u = self.rows[e][k];
                if self.mate_u[u].is_none() {
                    self.mate_e[e] = Some(u);
                    self.mate_u[u] = Some(e);
                    self.go(e + 1, size + 1);
                    self.mate_e[e] = None;
                    self.mate_u[u] = None;
                }
            }
        }
    }
    let rows = cleaned(n, m, inc);
    let mut s = Search {
        rows: &rows,
        mate_e: vec![None; n],
        mate_u: vec![None; m],
        best: 0,
        found: Vec::new(),
        keep,
    };
    s.go(0, 0);
    (s.best, s.found)
}

/// Kuhn 法（増加路を 1 本ずつ探す O(VE)）での最大マッチングの大きさ。
pub(crate) fn kuhn_size(n: usize, m: usize, inc: &[Vec<u32>]) -> usize {
    fn augment(
        e: usize,
        rows: &[Vec<usize>],
        seen: &mut [bool],
        mate_u: &mut [Option<usize>],
    ) -> bool {
        for &u in &rows[e] {
            if seen[u] {
                continue;
            }
            seen[u] = true;
            if mate_u[u].is_none_or(|f| augment(f, rows, seen, mate_u)) {
                mate_u[u] = Some(e);
                return true;
            }
        }
        false
    }
    let rows = cleaned(n, m, inc);
    let mut mate_u = vec![None; m];
    (0..n)
        .filter(|&e| augment(e, &rows, &mut vec![false; m], &mut mate_u))
        .count()
}

/// マッチングが incidence の辺だけを使い、両向きの対応が食い違っていないことを確かめる。
pub(crate) fn assert_valid_matching(n: usize, m: usize, inc: &[Vec<u32>], mt: &Matching) {
    let rows = cleaned(n, m, inc);
    assert_eq!(mt.equation_to_unknown.len(), n);
    assert_eq!(mt.unknown_to_equation.len(), m);
    let mut size = 0;
    for (e, mate) in mt.equation_to_unknown.iter().enumerate() {
        if let Some(u) = *mate {
            assert!(rows[e].contains(&u), "方程式 {e} は未知数 {u} を含まない");
            assert_eq!(mt.unknown_to_equation[u], Some(e));
            size += 1;
        }
    }
    for (u, mate) in mt.unknown_to_equation.iter().enumerate() {
        if let Some(e) = *mate {
            assert_eq!(mt.equation_to_unknown[e], Some(u));
        }
    }
    assert_eq!(mt.size, size);
}

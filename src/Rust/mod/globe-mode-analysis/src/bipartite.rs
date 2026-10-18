//! 二部グラフ（方程式 × 未知数）の隣接リスト。マッチングと DM 分解が共有する。

/// 行（方程式）ごとの列（未知数）の添字を、CSR（行の開始位置 + 列の並び）で持つ。
///
/// 入力の incidence は信用しない。範囲外の添字は捨て、行ごとに昇順・重複なしにそろえる。
/// 行が足りなければ空の行として扱い、余った行は捨てる。
#[derive(Debug, Clone)]
pub(crate) struct Bipartite {
    n_rows: usize,
    n_cols: usize,
    /// 行 `r` の列は `cols[row_start[r]..row_start[r + 1]]`。長さは `n_rows + 1`
    row_start: Vec<usize>,
    cols: Vec<usize>,
}

impl Bipartite {
    pub(crate) fn new(n_rows: usize, n_cols: usize, incidence: &[Vec<u32>]) -> Self {
        Self::union(n_rows, n_cols, &[incidence])
    }

    /// 複数の incidence を行ごとに合わせたもの（例: `incidence ∪ state_refs`）。
    /// どの入力も、行が足りなければ空の行として扱う。
    pub(crate) fn union(n_rows: usize, n_cols: usize, sources: &[&[Vec<u32>]]) -> Self {
        let mut row_start = Vec::with_capacity(n_rows + 1);
        let mut cols = Vec::new();
        let mut scratch = Vec::new();
        row_start.push(0);
        for r in 0..n_rows {
            scratch.clear();
            for source in sources {
                if let Some(row) = source.get(r) {
                    scratch.extend(row.iter().map(|&c| c as usize).filter(|&c| c < n_cols));
                }
            }
            scratch.sort_unstable();
            scratch.dedup();
            cols.extend_from_slice(&scratch);
            row_start.push(cols.len());
        }
        Self {
            n_rows,
            n_cols,
            row_start,
            cols,
        }
    }

    pub(crate) fn n_rows(&self) -> usize {
        self.n_rows
    }

    pub(crate) fn n_cols(&self) -> usize {
        self.n_cols
    }

    /// 行 `r` の列（昇順、重複なし）。
    pub(crate) fn row(&self, r: usize) -> &[usize] {
        &self.cols[self.row_start[r]..self.row_start[r + 1]]
    }

    /// 行と列を入れ替えたもの（未知数ごとの、それを含む方程式）。
    pub(crate) fn transpose(&self) -> Bipartite {
        let mut count = vec![0usize; self.n_cols + 1];
        for &c in &self.cols {
            count[c + 1] += 1;
        }
        for c in 0..self.n_cols {
            count[c + 1] += count[c];
        }
        let row_start = count.clone();
        let mut fill = count;
        let mut cols = vec![0usize; self.cols.len()];
        // 行の順に詰めるので、転置後の各行も昇順になる
        for r in 0..self.n_rows {
            for &c in self.row(r) {
                cols[fill[c]] = r;
                fill[c] += 1;
            }
        }
        Bipartite {
            n_rows: self.n_cols,
            n_cols: self.n_rows,
            row_start,
            cols,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_cleaned() {
        // 範囲外（7）を捨て、重複を除き、昇順にする。足りない行は空
        let g = Bipartite::new(3, 3, &[vec![2, 0, 2, 7, 0], vec![]]);
        assert_eq!(g.row(0), [0, 2]);
        assert_eq!(g.row(1), [] as [usize; 0]);
        assert_eq!(g.row(2), [] as [usize; 0]);
        // 余った行は捨てる
        let g = Bipartite::new(1, 2, &[vec![1], vec![0]]);
        assert_eq!(g.n_rows(), 1);
        assert_eq!(g.row(0), [1]);
    }

    #[test]
    fn union_merges_rows_of_different_lengths() {
        // 2 つ目の入力だけに行がある方程式も拾う
        let a = vec![vec![1]];
        let b = vec![vec![0, 1], vec![2, 9]];
        let g = Bipartite::union(3, 3, &[&a, &b]);
        assert_eq!(g.row(0), [0, 1]);
        assert_eq!(g.row(1), [2]);
        assert_eq!(g.row(2), [] as [usize; 0]);
    }

    #[test]
    fn transpose_lists_rows_per_column() {
        let g = Bipartite::new(3, 4, &[vec![0, 1], vec![1, 3], vec![1]]);
        let t = g.transpose();
        assert_eq!((t.n_rows(), t.n_cols()), (4, 3));
        assert_eq!(t.row(0), [0]);
        assert_eq!(t.row(1), [0, 1, 2]);
        assert_eq!(t.row(2), [] as [usize; 0]);
        assert_eq!(t.row(3), [1]);
    }
}

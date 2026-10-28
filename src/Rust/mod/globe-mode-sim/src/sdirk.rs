//! 刻み幅の制御のある SDIRK・ESDIRK の係数表（S027）。diffsol の SDIRK の方法（`Tableau::new`）に差す。
//!
//! Rumoca にも diffsol にも依存しない純粋なデータ。実行系（`globe-mode-exec` の `plugins::rk`）が diffsol の
//! `Tableau` に写す。次数の条件は下のテストで数値的に確かめる（係数の写し間違いを見つけるため）。
//!
//! diffsol の SDIRK が係数表に求める形（`runge_kutta.rs` の `check_sdirk_rk`。等号は厳密）:
//! `a` は下三角、`a[i][i]`（i >= 1）はすべて同じ γ、`a[0][0]` は 0（ESDIRK）か γ（SDIRK）、最後の行は `b`
//! （stiffly accurate）、最後の `c` は 1、ESDIRK は最初の `c` が 0。そのため `b` は `a` の最後の行から取り、
//! 別に持たない。
//!
//! 係数の出典（確認日 2026-10-29。【】は確認できなかったこと）:
//!
//! - `esdirk23`: J. B. Jørgensen, M. R. Kristensen, P. G. Thomsen, "A family of ESDIRK integration methods",
//!   arXiv:1803.01613 (2018) の §3.2（<https://arxiv.org/abs/1803.01613>）。閉じた式で与えられている。
//!   diffsol の `esdirk34` と同じ論文。
//! - `esdirk45`: ESDIRK5(4)7L[2]SA2。C. A. Kennedy, M. H. Carpenter, "Diagonally implicit Runge–Kutta methods for
//!   stiff ODEs", Applied Numerical Mathematics 146 (2019) 221–244 の係数を、OrdinaryDiffEq.jl の
//!   `ESDIRK547L2SA2Tableau`（`lib/OrdinaryDiffEqSDIRK/src/sdirk_tableaus.jl`、MIT、master を 2026-10-29 に取得）の
//!   有理数のまま写した。【論文本体は未確認】
//!   c4 は 49/353 とした（同ファイルは 49/453 と書くが、`a` の第 4 行の和は 1e-26 で 49/353 に一致し、
//!   次数の条件も `a` で通るので、c4 の誤記と判断した。他の c は行の和と一致する）。
//!   NASA/TM-2016-219173 の Table 25（ESDIRK5(4)7L[2]SA。別の版）は c6 = 26/25 > 1（段の時刻が刻みの外に出る）
//!   のため使わない。
//! - `sdirk34hw`: E. Hairer, G. Wanner, "Solving Ordinary Differential Equations II", 2nd ed., Springer (1996),
//!   §IV.6 の SDIRK4（γ = 1/4）。係数は OrdinaryDiffEq.jl の `Hairer4Tableau`（同じファイル）で照合した。
//!   【書籍の表は未確認】。

use globe_mode_ir::SolverId;

/// 刻み幅の制御のある SDIRK・ESDIRK の係数表。
#[derive(Debug, Clone, PartialEq)]
pub struct SdirkTableau {
    /// GlobeMode のソルバの名前
    pub name: &'static str,
    /// 刻み幅の制御に使う次数（主解と埋め込みの低い方。diffsol の `Tableau::order`）
    pub order: usize,
    /// `a[i][j]`（`j <= i`。長さは段の数の正方形で、上三角は 0）
    pub a: Vec<Vec<f64>>,
    /// 段の時刻の位置
    pub c: Vec<f64>,
    /// 誤差の見積もりの重み `d = b - b̂`（b̂ は埋め込みの重み）
    pub d: Vec<f64>,
    /// 刻みの間の補間の係数 `beta[i][j]`（段 `i`、`θ^(j+1)` の係数）。`y(θ) = y0 + Σ_i (Σ_j beta[i][j] θ^(j+1)) h f_i`。
    /// 無ければ diffsol が 3 次の Hermite 補間にする（これは最初の段が `f(t0, y0)` の ESDIRK だけで正しい）
    pub beta: Option<Vec<Vec<f64>>>,
}

impl SdirkTableau {
    /// 段の数。
    pub fn stages(&self) -> usize {
        self.c.len()
    }

    /// 主解の重み（`a` の最後の行。stiffly accurate）。
    pub fn b(&self) -> &[f64] {
        &self.a[self.stages() - 1]
    }
}

/// `n / d`（係数表の有理数を、そのまま写して 1 回だけ丸める）。
fn q(n: i128, d: i128) -> f64 {
    n as f64 / d as f64
}

/// 下三角の `rows`（各行は `a[i][0..i]`。対角は `gamma`）から正方の `a` を作る。
fn square(rows: &[&[f64]], gamma: f64, first_diagonal: f64) -> Vec<Vec<f64>> {
    let s = rows.len();
    (0..s)
        .map(|i| {
            let mut row = vec![0.0; s];
            row[..i].copy_from_slice(rows[i]);
            row[i] = if i == 0 { first_diagonal } else { gamma };
            row
        })
        .collect()
}

/// `id` の係数表（SDIRK・ESDIRK の自前のソルバでなければ None）。
pub fn sdirk_tableau(id: SolverId) -> Option<SdirkTableau> {
    match id {
        SolverId::Esdirk23 => Some(esdirk23()),
        SolverId::Esdirk45 => Some(esdirk45()),
        SolverId::Sdirk34hw => Some(sdirk34hw()),
        _ => None,
    }
}

/// ESDIRK 2(3)（Jørgensen–Kristensen–Thomsen 2018 §3.2、式 (3.11)–(3.15)）。
/// γ = (2 − √2)/2、c2 = 2γ、a21 = γ、b1 = b2 = √2/4、b3 = γ。主解は 2 次の L 安定、埋め込みは 3 次。
fn esdirk23() -> SdirkTableau {
    let g = (2.0 - 2f64.sqrt()) / 2.0;
    let b = 2f64.sqrt() / 4.0;
    // 埋め込み b̂（式 (3.15)）
    let bh = [
        (6.0 * g - 1.0) / (12.0 * g),
        1.0 / (12.0 * g * (1.0 - 2.0 * g)),
        (1.0 - 3.0 * g) / (3.0 * (1.0 - 2.0 * g)),
    ];
    let main = [b, b, g];
    SdirkTableau {
        name: "esdirk23",
        order: 2,
        a: square(&[&[], &[g], &[b, b]], g, 0.0),
        c: vec![0.0, 2.0 * g, 1.0],
        d: (0..3).map(|i| main[i] - bh[i]).collect(),
        beta: None,
    }
}

/// ESDIRK 5(4)、7 段（Kennedy–Carpenter 2019 の ESDIRK5(4)7L[2]SA2。出典はモジュールの説明）。
/// γ = 23/125、c2 = 2γ、a_i1 = a_i2（stage order 2）。主解は 5 次、埋め込みは 4 次。
fn esdirk45() -> SdirkTableau {
    let g = q(23, 125);
    let a31 = q(791020047304, 3561426431547);
    let a41 = q(-158159076358, 11257294102345);
    let a51 = q(-1653327111580, 4048416487981);
    let a61 = q(-4540011970825, 8418487046959);
    let a71 = q(-188593204321, 4778616380481);
    let rows: [&[f64]; 7] = [
        &[],
        &[g],
        &[a31, a31],
        &[a41, a41, q(-85517644447, 5003708988389)],
        &[
            a51,
            a51,
            q(1514767744496, 9099671765375),
            q(14283835447591, 12247432691556),
        ],
        &[
            a61,
            a61,
            q(-1790937573418, 7393406387169),
            q(10819093665085, 7266595846747),
            q(4109463131231, 7386972500302),
        ],
        &[
            a71,
            a71,
            q(2809310203510, 10304234040467),
            q(1021729336898, 2364210264653),
            q(870612361811, 2470410392208),
            q(-1307970675534, 8059683598661),
        ],
    ];
    // OrdinaryDiffEq.jl の btilde（7 個。b̂ − b。符号は誤差の大きさに効かない）。diffsol の d は b − b̂
    let btilde = [
        q(1421105133983177175480607, 34473265709570096426728110),
        q(1421105133983177175480607, 34473265709570096426728110),
        q(3109316704168648469186161, 34649291136493354414563315),
        q(-1238536018553705945740781, 14596994419085808131818547),
        q(-2644586096629232150947109, 35649179736979632279017232),
        q(-4528233948360460577182037, 78128560886623254313643924),
        q(77036761781598, 1719282803550125),
    ];
    SdirkTableau {
        name: "esdirk45",
        order: 4,
        a: square(&rows, g, 0.0),
        c: vec![
            0.0,
            2.0 * g,
            q(7121331996143, 11335814405378),
            // OrdinaryDiffEq.jl は 49/453 と書くが、第 4 段の行の和は厳密に 49/353（誤記と判断した）
            q(49, 353),
            q(3706679970760, 5295570149437),
            q(347, 382),
            1.0,
        ],
        d: btilde.iter().map(|t| -t).collect(),
        beta: None,
    }
}

/// Hairer–Wanner の SDIRK4（5 段、4 次、L 安定。Hairer–Wanner II の §IV.6）。γ = 1/4、埋め込みは 3 次。
fn sdirk34hw() -> SdirkTableau {
    let g = 0.25;
    let rows: [&[f64]; 5] = [
        &[],
        &[0.5],
        &[q(17, 50), q(-1, 25)],
        &[q(371, 1360), q(-137, 2720), q(15, 544)],
        &[q(25, 24), q(-49, 48), q(125, 16), q(-85, 12)],
    ];
    // b̂ = (59/48, −17/96, 225/32, −85/12, 0)。d = b − b̂
    let bh = [q(59, 48), q(-17, 96), q(225, 32), q(-85, 12), 0.0];
    let a = square(&rows, g, g);
    let d = (0..5).map(|i| a[4][i] - bh[i]).collect();
    // 補間（3 次）。OrdinaryDiffEq.jl の `Hairer4Tableau` の r11..r54（Hairer–Wanner II の d_ij に A を掛けて
    // f の形にしたもの）。下のテストが、この係数が 3 次の補間の条件を厳密に満たすことを確かめる
    let beta = vec![
        vec![q(11, 3), q(-463, 72), q(217, 36), q(-20, 9)],
        vec![q(11, 2), q(-385, 16), q(661, 24), q(-10, 1)],
        vec![q(-125, 18), q(20125, 432), q(-8875, 216), q(250, 27)],
        vec![0.0, q(-85, 4), q(85, 6), 0.0],
        vec![q(-11, 9), q(557, 108), q(-359, 54), q(80, 27)],
    ];
    SdirkTableau {
        name: "sdirk34hw",
        order: 3,
        a,
        c: vec![0.25, 0.75, q(11, 20), 0.5, 1.0],
        d,
        beta: Some(beta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [SolverId; 3] = [SolverId::Esdirk23, SolverId::Esdirk45, SolverId::Sdirk34hw];
    /// 係数の有理数を f64 に丸めた誤差と、条件の和の丸めの誤差に十分な余裕
    const TOL: f64 = 1e-12;

    /// 根付き木（子の木の並び。節点 1 個は `Tree(vec![])`）。
    #[derive(Clone)]
    struct Tree(Vec<Tree>);

    impl Tree {
        fn order(&self) -> usize {
            1 + self.0.iter().map(Tree::order).sum::<usize>()
        }

        /// 密度 γ(t) = |t| · Π γ(子)
        fn density(&self) -> f64 {
            self.order() as f64 * self.0.iter().map(Tree::density).product::<f64>()
        }

        /// 基本重みの段ごとの値 Ψ(t)_i = Π_子 (A Ψ(子))_i（子が無ければ 1）
        fn psi(&self, a: &[Vec<f64>]) -> Vec<f64> {
            let mut out = vec![1.0; a.len()];
            for child in &self.0 {
                let p = child.psi(a);
                for (i, o) in out.iter_mut().enumerate() {
                    *o *= a[i].iter().zip(&p).map(|(x, y)| x * y).sum::<f64>();
                }
            }
            out
        }
    }

    /// 次数 1..=max の根付き木すべて（次数ごと。子の並びは添字の昇順で重複を除く）。
    fn trees_up_to(max: usize) -> Vec<Vec<Tree>> {
        let mut by_order: Vec<Vec<Tree>> = vec![vec![], vec![Tree(vec![])]];
        for n in 2..=max {
            let lower: Vec<Tree> = by_order.iter().flatten().cloned().collect();
            let mut found = Vec::new();
            // 次数の合計が n - 1 になる、添字の昇順の子の組
            fn extend(
                lower: &[Tree],
                start: usize,
                left: usize,
                children: &mut Vec<Tree>,
                out: &mut Vec<Tree>,
            ) {
                if left == 0 {
                    out.push(Tree(children.clone()));
                    return;
                }
                for k in start..lower.len() {
                    let o = lower[k].order();
                    if o <= left {
                        children.push(lower[k].clone());
                        extend(lower, k, left - o, children, out);
                        children.pop();
                    }
                }
            }
            extend(&lower, 0, n - 1, &mut Vec::new(), &mut found);
            by_order.push(found);
        }
        by_order
    }

    /// 重み `w` が次数 `order` までの条件 Σ w·Ψ(t) = 1/γ(t) を満たすときの、条件ごとの残差の最大値（次数 n の木だけ）。
    fn residual(w: &[f64], a: &[Vec<f64>], order: usize) -> f64 {
        trees_up_to(order)[order]
            .iter()
            .map(|t| {
                let lhs: f64 = w.iter().zip(t.psi(a)).map(|(x, y)| x * y).sum();
                (lhs - 1.0 / t.density()).abs()
            })
            .fold(0.0, f64::max)
    }

    /// 次数 `order` まで条件を満たす（満たさない最小の次数を返すための道具）。
    fn attained_order(w: &[f64], a: &[Vec<f64>], max: usize) -> usize {
        (1..=max)
            .take_while(|&n| residual(w, a, n) < TOL)
            .last()
            .unwrap_or(0)
    }

    fn embedded(t: &SdirkTableau) -> Vec<f64> {
        t.b().iter().zip(&t.d).map(|(b, d)| b - d).collect()
    }

    #[test]
    fn rooted_tree_enumeration_has_the_known_counts() {
        // 次数 1..=6 の根付き木は 1, 1, 2, 4, 9, 20 本（OEIS A000081）
        let counts: Vec<usize> = trees_up_to(6).iter().skip(1).map(Vec::len).collect();
        assert_eq!(counts, [1, 1, 2, 4, 9, 20]);
    }

    #[test]
    fn order_check_detects_a_wrong_method() {
        // 陽的 Euler の表（a = 0、b = 1）は 1 次だけ
        assert_eq!(attained_order(&[1.0], &[vec![0.0]], 4), 1);
        // 古典的な RK4 は 4 次ちょうど
        let a = vec![
            vec![0.0; 4],
            vec![0.5, 0.0, 0.0, 0.0],
            vec![0.0, 0.5, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
        ];
        let b = [1.0 / 6.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 6.0];
        assert_eq!(attained_order(&b, &a, 6), 4);
    }

    #[test]
    fn only_the_three_sdirk_solvers_have_tableaus() {
        for id in ALL {
            let t = sdirk_tableau(id).expect("係数表がある");
            assert_eq!(t.name, id.name());
        }
        assert!(sdirk_tableau(SolverId::Esdirk34).is_none());
        assert!(sdirk_tableau(SolverId::Bdf).is_none());
    }

    #[test]
    fn shape_matches_what_diffsol_requires() {
        for id in ALL {
            let t = sdirk_tableau(id).unwrap();
            let s = t.stages();
            assert_eq!(t.d.len(), s);
            assert!(t.a.iter().all(|r| r.len() == s));
            let gamma = t.a[1][1];
            for i in 0..s {
                assert!(
                    t.a[i][i + 1..].iter().all(|&x| x == 0.0),
                    "{} 上三角",
                    t.name
                );
                if i >= 1 {
                    assert_eq!(t.a[i][i], gamma, "{} 対角", t.name);
                }
            }
            assert!(t.a[0][0] == 0.0 || t.a[0][0] == gamma);
            assert_eq!(t.c[s - 1], 1.0);
            if t.a[0][0] == 0.0 {
                assert_eq!(t.c[0], 0.0, "{} ESDIRK の最初の c", t.name);
            }
        }
    }

    #[test]
    fn row_sums_equal_c() {
        for id in ALL {
            let t = sdirk_tableau(id).unwrap();
            for (i, row) in t.a.iter().enumerate() {
                let sum: f64 = row.iter().sum();
                assert!(
                    (sum - t.c[i]).abs() < TOL,
                    "{} 段 {i}: {sum} != {}",
                    t.name,
                    t.c[i]
                );
            }
            // 時刻はどれも刻みの中（イベントの検出で刻みの外を評価しない）
            assert!(t.c.iter().all(|&c| (0.0..=1.0).contains(&c)), "{}", t.name);
        }
    }

    #[test]
    fn main_method_attains_its_order_and_no_more() {
        // (名前, 主解の次数, 埋め込みの次数)
        for (id, p, p_hat) in [
            (SolverId::Esdirk23, 2, 3),
            (SolverId::Esdirk45, 5, 4),
            (SolverId::Sdirk34hw, 4, 3),
        ] {
            let t = sdirk_tableau(id).unwrap();
            assert_eq!(attained_order(t.b(), &t.a, 6), p, "{} 主解", t.name);
            assert_eq!(
                attained_order(&embedded(&t), &t.a, 6),
                p_hat,
                "{} 埋め込み",
                t.name
            );
            // diffsol に渡す次数は主解と埋め込みの低い方
            assert_eq!(t.order, p.min(p_hat), "{}", t.name);
        }
    }

    #[test]
    fn dense_output_of_sdirk34hw_is_third_order() {
        let t = sdirk_tableau(SolverId::Sdirk34hw).unwrap();
        let beta = t.beta.as_ref().expect("SDIRK は補間の係数が要る");
        // θ = 1 で主解に一致する: Σ_j beta[i][j] = b[i]
        for (i, row) in beta.iter().enumerate() {
            assert!((row.iter().sum::<f64>() - t.b()[i]).abs() < TOL, "段 {i}");
        }
        // 次数 ρ の木で Σ_i beta[i][j] Ψ_i = δ(j, ρ) / γ(木)（θ の各べきごと）
        let worst = |max: usize| {
            let mut worst = 0.0f64;
            for rho in 1..=max {
                for tree in &trees_up_to(max)[rho] {
                    let psi = tree.psi(&t.a);
                    for j in 0..beta[0].len() {
                        let lhs: f64 = beta.iter().zip(&psi).map(|(row, p)| row[j] * p).sum();
                        let rhs = if j + 1 == rho {
                            1.0 / tree.density()
                        } else {
                            0.0
                        };
                        worst = worst.max((lhs - rhs).abs());
                    }
                }
            }
            worst
        };
        assert!(worst(3) < TOL, "3 次の条件の残差 {}", worst(3));
        assert!(worst(4) > 1e-3, "4 次は満たさない（満たすなら係数が変）");
        // 補間の係数が無いのは Hermite で足りる ESDIRK
        assert!(sdirk_tableau(SolverId::Esdirk23).unwrap().beta.is_none());
        assert!(sdirk_tableau(SolverId::Esdirk45).unwrap().beta.is_none());
    }

    #[test]
    fn error_weights_sum_to_zero() {
        // Σ b = Σ b̂ = 1 なので Σ d = 0（定数の解で誤差が 0）
        for id in ALL {
            let t = sdirk_tableau(id).unwrap();
            assert!(t.d.iter().sum::<f64>().abs() < TOL, "{}", t.name);
        }
    }
}

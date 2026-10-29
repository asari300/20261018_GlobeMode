//! 刻み幅の制御のある SDIRK・ESDIRK の係数表（S027）と、陽的 RK の `dopri853`（S028）・`cerk23`・`cerk34`・`cerk45`（S029）の係数表。diffsol の
//! `Tableau::new` に差す（SDIRK は `sdirk_solver`、陽的 RK は `explicit_rk_solver`）。
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
//! - `dopri853`（S028）: E. Hairer, S. P. Nørsett, G. Wanner, "Solving Ordinary Differential Equations I", 2nd ed.,
//!   Springer (1993), §II.5 の DOP853。係数は SciPy の `scipy/integrate/_ivp/dop853_coefficients.py`（BSD-3-Clause。
//!   Hairer の `dop853.f`、<http://www.unige.ch/~hairer/software.html> から起こしたもの）の `C`・`A`・`E5` を、
//!   <https://raw.githubusercontent.com/scipy/scipy/main/scipy/integrate/_ivp/dop853_coefficients.py> から
//!   2026-10-29 に取得して桁を変えずに写した（`dop853.f` 本体は未確認）。詳しい扱いは [`dopri853`] を見る。
//! - `cerk23`・`cerk34`・`cerk45`（S029）: B. Owren, M. Zennaro, "Derivation of efficient, continuous, explicit
//!   Runge–Kutta methods", SIAM J. Sci. Stat. Comput. 13 (1992) 1488–1501 の 3(2)・4(3)・5(4)。係数と補間の多項式は
//!   OrdinaryDiffEq.jl の `OwrenZen3`・`OwrenZen4`・`OwrenZen5`（MIT。
//!   `lib/OrdinaryDiffEqLowOrderRK/src/low_order_rk_tableaus.jl` と `interpolants.jl`、master の
//!   45ff46afe9919acf32e9af8058705073e0995503 = 2026-10-25 を 2026-10-29 に
//!   <https://raw.githubusercontent.com/SciML/OrdinaryDiffEq.jl/45ff46afe9919acf32e9af8058705073e0995503/lib/OrdinaryDiffEqLowOrderRK/src/low_order_rk_tableaus.jl>
//!   から取得）の有理数の版を、スクリプトで Rust に起こした（手で写していない）。【論文本体は未確認】
//!   どれも FSAL（最後の行 = `b`、`c = 1`）で、補間は主解と同じ次数（3・4・5 次）。

use globe_mode_ir::SolverId;

/// 刻み幅の制御のある SDIRK・ESDIRK、陽的 RK の係数表。
#[derive(Debug, Clone, PartialEq)]
pub struct RkTableau {
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
    /// 無ければ diffsol が 3 次の Hermite 補間にする（左の傾きは最初の段、右の傾きは最後の段。これは最初の段が
    /// `f(t0, y0)` の ESDIRK と陽的 RK で、最後の段が `f(t0 + h, y1)` のとき正しい）
    pub beta: Option<Vec<Vec<f64>>>,
}

impl RkTableau {
    /// 段の数。
    pub fn stages(&self) -> usize {
        self.c.len()
    }

    /// 主解の重み（`a` の最後の行。SDIRK は stiffly accurate、陽的 RK は FSAL。diffsol がどちらも `a` の最後の行 = `b` を求める）。
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
pub fn sdirk_tableau(id: SolverId) -> Option<RkTableau> {
    match id {
        SolverId::Esdirk23 => Some(esdirk23()),
        SolverId::Esdirk45 => Some(esdirk45()),
        SolverId::Sdirk34hw => Some(sdirk34hw()),
        _ => None,
    }
}

/// ESDIRK 2(3)（Jørgensen–Kristensen–Thomsen 2018 §3.2、式 (3.11)–(3.15)）。
/// γ = (2 − √2)/2、c2 = 2γ、a21 = γ、b1 = b2 = √2/4、b3 = γ。主解は 2 次の L 安定、埋め込みは 3 次。
fn esdirk23() -> RkTableau {
    let g = (2.0 - 2f64.sqrt()) / 2.0;
    let b = 2f64.sqrt() / 4.0;
    // 埋め込み b̂（式 (3.15)）
    let bh = [
        (6.0 * g - 1.0) / (12.0 * g),
        1.0 / (12.0 * g * (1.0 - 2.0 * g)),
        (1.0 - 3.0 * g) / (3.0 * (1.0 - 2.0 * g)),
    ];
    let main = [b, b, g];
    RkTableau {
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
fn esdirk45() -> RkTableau {
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
    RkTableau {
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
fn sdirk34hw() -> RkTableau {
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
    RkTableau {
        name: "sdirk34hw",
        order: 3,
        a,
        c: vec![0.25, 0.75, q(11, 20), 0.5, 1.0],
        d,
        beta: Some(beta),
    }
}

/// `id` の陽的 RK の係数表（`dopri853`・`cerk23`・`cerk34`・`cerk45` でなければ None）。
pub fn explicit_tableau(id: SolverId) -> Option<RkTableau> {
    match id {
        SolverId::Dopri853 => Some(dopri853()),
        SolverId::Cerk23 => Some(cerk23()),
        SolverId::Cerk34 => Some(cerk34()),
        SolverId::Cerk45 => Some(cerk45()),
        _ => None,
    }
}

/// 連続の陽的 RK（FSAL。最後の行 = `b`）の係数表を、下三角の `rows`（`rows[i]` は `a[i][0..i]`）、時刻 `c`、
/// 埋め込みとの差 `btilde = b̂ − b`（出典の符号。`d = b − b̂ = −btilde`）、補間の `beta` から組む。
fn continuous_explicit(
    name: &'static str,
    order: usize,
    rows: &[&[f64]],
    c: Vec<f64>,
    btilde: &[f64],
    beta: Vec<Vec<f64>>,
) -> RkTableau {
    let s = rows.len();
    let a = (0..s)
        .map(|i| {
            let mut row = vec![0.0; s];
            row[..i].copy_from_slice(rows[i]);
            row
        })
        .collect();
    RkTableau {
        name,
        order,
        a,
        c,
        d: btilde.iter().map(|t| -t).collect(),
        beta: Some(beta),
    }
}

/// `cerk23`（Owren–Zennaro 1992 の連続の陽的 RK。3(2)、4 段の FSAL、主解は 3 次、埋め込みは 2 次、
/// 刻みの間は 3 次の補間）。係数は OrdinaryDiffEq.jl の `OwrenZen3ConstantCache`（有理数の版）から生成した。
fn cerk23() -> RkTableau {
    let rows: [&[f64]; 4] = [
        &[],
        &[q(12, 23)],
        &[q(-68, 375), q(368, 375)],
        &[q(31, 144), q(529, 1152), q(125, 384)],
    ];
    // c（段 2..=4。最後の段は 1）
    let c = vec![0.0, q(12, 23), q(4, 5), 1.0];
    // btilde = b̂ − b（出典の符号。diffsol の d = b − b̂ = −btilde）
    let btilde = [q(-25, 144), q(575, 1152), q(-125, 384), 0.0];
    // 補間の係数 beta[段][θ^1..θ^3]（出典の r<段><べき> を並べ替えた）
    let beta = vec![
        vec![1.0, q(-65, 48), q(41, 72)],
        vec![0.0, q(529, 384), q(-529, 576)],
        vec![0.0, q(125, 128), q(-125, 192)],
        vec![0.0, -1.0, 1.0],
    ];
    continuous_explicit("cerk23", 2, &rows, c, &btilde, beta)
}

/// `cerk34`（Owren–Zennaro 1992 の連続の陽的 RK。4(3)、6 段の FSAL、主解は 4 次、埋め込みは 3 次、
/// 刻みの間は 4 次の補間）。係数は OrdinaryDiffEq.jl の `OwrenZen4ConstantCache`（有理数の版）から生成した。
fn cerk34() -> RkTableau {
    let rows: [&[f64]; 6] = [
        &[],
        &[q(1, 6)],
        &[q(44, 1369), q(363, 1369)],
        &[q(3388, 4913), q(-8349, 4913), q(8140, 4913)],
        &[
            q(-36764, 408375),
            q(767, 1125),
            q(-32708, 136125),
            q(210392, 408375),
        ],
        &[
            q(1697, 18876),
            0.0,
            q(50653, 116160),
            q(299693, 1626240),
            q(3375, 11648),
        ],
    ];
    // c（段 2..=6。最後の段は 1）
    let c = vec![0.0, q(1, 6), q(11, 37), q(11, 17), q(13, 15), 1.0];
    // btilde = b̂ − b（出典の符号。diffsol の d = b − b̂ = −btilde）
    let btilde = [
        q(1185, 6292),
        0.0,
        q(-4107, 7744),
        q(68493, 108416),
        q(-3375, 11648),
        0.0,
    ];
    // 補間の係数 beta[段][θ^1..θ^4]（出典の r<段><べき> を並べ替えた）
    let beta = vec![
        vec![
            1.0,
            q(-104217, 37466),
            q(1806901, 618189),
            q(-866577, 824252),
        ],
        vec![0.0, 0.0, 0.0, 0.0],
        vec![
            0.0,
            q(861101, 230560),
            q(-2178079, 380424),
            q(12308679, 5072320),
        ],
        vec![
            0.0,
            q(-63869, 293440),
            q(6244423, 5325936),
            q(-7816583, 10144640),
        ],
        vec![
            0.0,
            q(-1522125, 762944),
            q(982125, 190736),
            q(-624375, 217984),
        ],
        vec![0.0, q(165, 131), q(-461, 131), q(296, 131)],
    ];
    continuous_explicit("cerk34", 3, &rows, c, &btilde, beta)
}

/// `cerk45`（Owren–Zennaro 1992 の連続の陽的 RK。5(4)、8 段の FSAL、主解は 5 次、埋め込みは 4 次、
/// 刻みの間は 5 次の補間）。係数は OrdinaryDiffEq.jl の `OwrenZen5ConstantCache`（有理数の版）から生成した。
fn cerk45() -> RkTableau {
    let rows: [&[f64]; 8] = [
        &[],
        &[q(1, 6)],
        &[q(1, 16), q(3, 16)],
        &[q(1, 4), q(-3, 4), 1.0],
        &[q(-3, 4), q(15, 4), -3.0, q(1, 2)],
        &[
            q(369, 1372),
            q(-243, 343),
            q(297, 343),
            q(1485, 9604),
            q(297, 4802),
        ],
        &[
            q(-133, 4512),
            q(1113, 6016),
            q(7945, 16544),
            q(-12845, 24064),
            q(-315, 24064),
            q(156065, 198528),
        ],
        &[
            q(83, 945),
            0.0,
            q(248, 825),
            q(41, 180),
            q(1, 36),
            q(2401, 38610),
            q(6016, 20475),
        ],
    ];
    // c（段 2..=8。最後の段は 1）
    let c = vec![
        0.0,
        q(1, 6),
        q(1, 4),
        q(1, 2),
        q(1, 2),
        q(9, 14),
        q(7, 8),
        1.0,
    ];
    // btilde = b̂ − b（出典の符号。diffsol の d = b − b̂ = −btilde）
    let btilde = [
        q(-188, 945),
        0.0,
        q(752, 825),
        q(-89, 45),
        q(-1, 9),
        q(32242, 19305),
        q(-6016, 20475),
        0.0,
    ];
    // 補間の係数 beta[段][θ^1..θ^5]（出典の r<段><べき> を並べ替えた）
    let beta = vec![
        vec![
            1.0,
            q(-3292, 819),
            q(17893, 2457),
            q(-4969, 819),
            q(596, 315),
        ],
        vec![0.0, 0.0, 0.0, 0.0, 0.0],
        vec![
            0.0,
            q(5112, 715),
            q(-43568, 2145),
            q(1344, 65),
            q(-1984, 275),
        ],
        vec![0.0, q(-123, 52), q(3161, 234), q(-1465, 78), q(118, 15)],
        vec![0.0, q(-63, 52), q(1061, 234), q(-413, 78), 2.0],
        vec![
            0.0,
            q(-40817, 33462),
            q(60025, 50193),
            q(2401, 1521),
            q(-9604, 6435),
        ],
        vec![
            0.0,
            q(18048, 5915),
            q(-637696, 53235),
            q(96256, 5915),
            q(-48128, 6825),
        ],
        vec![0.0, q(-18, 13), q(75, 13), q(-109, 13), 4.0],
    ];
    continuous_explicit("cerk45", 4, &rows, c, &btilde, beta)
}

/// DOP853（Hairer–Nørsett–Wanner。8 次の主解、5 次の埋め込み）。
///
/// - 段: `dop853.f` の 12 段に、FSAL の 13 段目（`c = 1`、`a` の行 = `b`、`b13 = 0`）を足した 13 段。diffsol の陽的 RK は
///   `a` の最後の行 = `b`・`c_last = 1` を求めるため。13 段目の `f(t0 + h, y1)` は次の刻みの最初の段に使われるので、
///   刻みあたりの `f` の評価は 12 回のまま。
/// - 誤差: diffsol の誤差は `Σ d_i h k_i` の 1 本だけ。`d` には 5 次の埋め込みの `E5`（`dop853.f` の `er1..er12`）を使う。
///   Hairer の符号と同じく `d = b − b̂`。Hairer の刻み幅の制御は 5 次と 3 次の見積もりを
///   `err5² / sqrt(err5² + 0.01 err3²)` で混ぜるが、diffsol にはそれを表せないので **刻み幅の制御は `dop853.f` と違い**
///   （刻みは 5 次の見積もりで決まる。主解は 8 次なので実際の誤差は許容誤差よりかなり小さい）、結果は `dop853.f` と一致しない。
/// - 補間: Hairer の 7 次の連続の拡張は追加の 3 段を要し、diffsol の `beta` では表せないので `beta` は None。
///   刻みの間は diffsol の 3 次の Hermite 補間（端の値と傾きは主解の `f` で正しい）。
#[allow(clippy::excessive_precision)] // 出典の桁をそのまま写す
fn dopri853() -> RkTableau {
    // 段 2..=13 の `a` の非零の要素 `(列, 値)`。最後の行は `b`
    const ROWS: [&[(usize, f64)]; 12] = [
        // 段 2
        &[(0, 5.26001519587677318785587544488e-2)],
        // 段 3
        &[
            (0, 1.97250569845378994544595329183e-2),
            (1, 5.91751709536136983633785987549e-2),
        ],
        // 段 4
        &[
            (0, 2.95875854768068491816892993775e-2),
            (2, 8.87627564304205475450678981324e-2),
        ],
        // 段 5
        &[
            (0, 2.41365134159266685502369798665e-1),
            (2, -8.84549479328286085344864962717e-1),
            (3, 9.24834003261792003115737966543e-1),
        ],
        // 段 6
        &[
            (0, 3.7037037037037037037037037037e-2),
            (3, 1.70828608729473871279604482173e-1),
            (4, 1.25467687566822425016691814123e-1),
        ],
        // 段 7
        &[
            (0, 3.7109375e-2),
            (3, 1.70252211019544039314978060272e-1),
            (4, 6.02165389804559606850219397283e-2),
            (5, -1.7578125e-2),
        ],
        // 段 8
        &[
            (0, 3.70920001185047927108779319836e-2),
            (3, 1.70383925712239993810214054705e-1),
            (4, 1.07262030446373284651809199168e-1),
            (5, -1.53194377486244017527936158236e-2),
            (6, 8.27378916381402288758473766002e-3),
        ],
        // 段 9
        &[
            (0, 6.24110958716075717114429577812e-1),
            (3, -3.36089262944694129406857109825),
            (4, -8.68219346841726006818189891453e-1),
            (5, 2.75920996994467083049415600797e1),
            (6, 2.01540675504778934086186788979e1),
            (7, -4.34898841810699588477366255144e1),
        ],
        // 段 10
        &[
            (0, 4.77662536438264365890433908527e-1),
            (3, -2.48811461997166764192642586468),
            (4, -5.90290826836842996371446475743e-1),
            (5, 2.12300514481811942347288949897e1),
            (6, 1.52792336328824235832596922938e1),
            (7, -3.32882109689848629194453265587e1),
            (8, -2.03312017085086261358222928593e-2),
        ],
        // 段 11
        &[
            (0, -9.3714243008598732571704021658e-1),
            (3, 5.18637242884406370830023853209),
            (4, 1.09143734899672957818500254654),
            (5, -8.14978701074692612513997267357),
            (6, -1.85200656599969598641566180701e1),
            (7, 2.27394870993505042818970056734e1),
            (8, 2.49360555267965238987089396762),
            (9, -3.0467644718982195003823669022),
        ],
        // 段 12
        &[
            (0, 2.27331014751653820792359768449),
            (3, -1.05344954667372501984066689879e1),
            (4, -2.00087205822486249909675718444),
            (5, -1.79589318631187989172765950534e1),
            (6, 2.79488845294199600508499808837e1),
            (7, -2.85899827713502369474065508674),
            (8, -8.87285693353062954433549289258),
            (9, 1.23605671757943030647266201528e1),
            (10, 6.43392746015763530355970484046e-1),
        ],
        // 段 13
        &[
            (0, 5.42937341165687622380535766363e-2),
            (5, 4.45031289275240888144113950566),
            (6, 1.89151789931450038304281599044),
            (7, -5.8012039600105847814672114227),
            (8, 3.1116436695781989440891606237e-1),
            (9, -1.52160949662516078556178806805e-1),
            (10, 2.01365400804030348374776537501e-1),
            (11, 4.47106157277725905176885569043e-2),
        ],
    ];
    let c = vec![
        0.0,
        0.526001519587677318785587544488e-01,
        0.789002279381515978178381316732e-01,
        0.118350341907227396726757197510,
        0.281649658092772603273242802490,
        0.333333333333333333333333333333,
        0.25,
        0.307692307692307692307692307692,
        0.651282051282051282051282051282,
        0.6,
        0.857142857142857142857142857142,
        1.0,
        1.0,
    ];
    // E5（`dop853.f` の er1..er12。段の番号 0 から。13 段目は 0）
    const E5: [(usize, f64); 8] = [
        (0, 0.1312004499419488073250102996e-1),
        (5, -0.1225156446376204440720569753e+1),
        (6, -0.4957589496572501915214079952),
        (7, 0.1664377182454986536961530415e+1),
        (8, -0.3503288487499736816886487290),
        (9, 0.3341791187130174790297318841),
        (10, 0.8192320648511571246570742613e-1),
        (11, -0.2235530786388629525884427845e-1),
    ];
    let mut a = vec![vec![0.0; 13]; 13];
    for (i, row) in ROWS.iter().enumerate() {
        for &(j, v) in *row {
            a[i + 1][j] = v;
        }
    }
    let mut d = vec![0.0; 13];
    for (j, v) in E5 {
        d[j] = v;
    }
    RkTableau {
        name: "dopri853",
        order: 5,
        a,
        c,
        d,
        beta: None,
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

    fn embedded(t: &RkTableau) -> Vec<f64> {
        t.b().iter().zip(&t.d).map(|(b, d)| b - d).collect()
    }

    #[test]
    fn rooted_tree_enumeration_has_the_known_counts() {
        // 次数 1..=8 の根付き木は 1, 1, 2, 4, 9, 20, 48, 115 本（OEIS A000081）
        let counts: Vec<usize> = trees_up_to(8).iter().skip(1).map(Vec::len).collect();
        assert_eq!(counts, [1, 1, 2, 4, 9, 20, 48, 115]);
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

    /// 補間の係数 `beta` が、次数 1..=max の木で Σ_i beta[i][j] Ψ_i = δ(j+1, ρ) / γ(木) を満たすときの残差の最大値
    /// （θ の各べきごと。`beta` の列が θ^1.. に当たる）。
    fn dense_residual(t: &RkTableau, max: usize) -> f64 {
        let beta = t.beta.as_ref().expect("補間の係数がある");
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
    }

    #[test]
    fn dense_output_of_sdirk34hw_is_third_order() {
        let t = sdirk_tableau(SolverId::Sdirk34hw).unwrap();
        let beta = t.beta.as_ref().expect("SDIRK は補間の係数が要る");
        // θ = 1 で主解に一致する: Σ_j beta[i][j] = b[i]
        for (i, row) in beta.iter().enumerate() {
            assert!((row.iter().sum::<f64>() - t.b()[i]).abs() < TOL, "段 {i}");
        }
        assert!(dense_residual(&t, 3) < TOL, "3 次の条件の残差");
        assert!(
            dense_residual(&t, 4) > 1e-3,
            "4 次は満たさない（満たすなら係数が変）"
        );
        // 補間の係数が無いのは Hermite で足りる ESDIRK
        assert!(sdirk_tableau(SolverId::Esdirk23).unwrap().beta.is_none());
        assert!(sdirk_tableau(SolverId::Esdirk45).unwrap().beta.is_none());
    }

    const CERK: [(SolverId, usize, usize, usize); 3] = [
        // (名前, 主解の次数, 埋め込みの次数, 段の数)
        (SolverId::Cerk23, 3, 2, 4),
        (SolverId::Cerk34, 4, 3, 6),
        (SolverId::Cerk45, 5, 4, 8),
    ];

    #[test]
    fn cerk_tableaus_are_fsal_explicit_in_the_form_diffsol_requires() {
        for (id, _, p_hat, s) in CERK {
            let t = explicit_tableau(id).expect("係数表がある");
            assert_eq!(t.name, id.name());
            assert!(sdirk_tableau(id).is_none());
            assert_eq!((t.stages(), t.d.len(), t.a.len()), (s, s, s), "{}", t.name);
            for (i, row) in t.a.iter().enumerate() {
                assert_eq!(row.len(), s);
                assert!(row[i..].iter().all(|&x| x == 0.0), "{} 段 {i}", t.name);
                // 行の和 = c
                let sum: f64 = row.iter().sum();
                assert!((sum - t.c[i]).abs() < TOL, "{} 段 {i}", t.name);
            }
            // diffsol の陽的 RK の条件: 最初の c = 0、最後の c = 1（最後の行 = b は b() が保証）。b の最後は 0（FSAL）
            assert_eq!((t.c[0], t.c[s - 1]), (0.0, 1.0), "{}", t.name);
            assert_eq!(t.b()[s - 1], 0.0, "{}", t.name);
            assert_eq!(t.d[s - 1], 0.0, "{}", t.name);
            assert!(t.c.iter().all(|&c| (0.0..=1.0).contains(&c)), "{}", t.name);
            assert!(t.d.iter().sum::<f64>().abs() < TOL, "{}", t.name);
            assert_eq!(t.order, p_hat, "{} diffsol に渡す次数は埋め込み", t.name);
            let beta = t.beta.as_ref().expect("連続の方法は補間の係数を持つ");
            assert_eq!(beta.len(), s);
        }
    }

    #[test]
    fn cerk_main_and_embedded_attain_their_orders() {
        for (id, p, p_hat, _) in CERK {
            let t = explicit_tableau(id).unwrap();
            assert_eq!(attained_order(t.b(), &t.a, p + 1), p, "{} 主解", t.name);
            assert_eq!(
                attained_order(&embedded(&t), &t.a, p + 1),
                p_hat,
                "{} 埋め込み",
                t.name
            );
        }
    }

    #[test]
    fn cerk_dense_output_has_the_order_of_the_main_method() {
        for (id, p, _, _) in CERK {
            let t = explicit_tableau(id).unwrap();
            let beta = t.beta.as_ref().unwrap();
            assert_eq!(beta[0].len(), p, "{} 補間の多項式の次数", t.name);
            // θ = 1 で主解に一致し、θ = 0 で 0（べきは θ^1 から）
            for (i, row) in beta.iter().enumerate() {
                assert!(
                    (row.iter().sum::<f64>() - t.b()[i]).abs() < TOL,
                    "{} 段 {i}",
                    t.name
                );
            }
            // 次数 p までの条件を、θ の各べきで満たす（連続の拡張の次数 p）
            assert!(dense_residual(&t, p) < TOL, "{} {p} 次の条件", t.name);
        }
    }

    #[test]
    fn dopri853_is_an_fsal_explicit_tableau_in_the_form_diffsol_requires() {
        let t = explicit_tableau(SolverId::Dopri853).expect("係数表がある");
        assert_eq!(t.name, "dopri853");
        assert!(explicit_tableau(SolverId::Tsit45).is_none());
        assert!(sdirk_tableau(SolverId::Dopri853).is_none());
        let s = t.stages();
        assert_eq!((s, t.d.len(), t.a.len()), (13, 13, 13));
        for (i, row) in t.a.iter().enumerate() {
            assert_eq!(row.len(), s);
            // 厳密な下三角（対角も 0）
            assert!(row[i..].iter().all(|&x| x == 0.0), "段 {i}");
        }
        assert_eq!((t.c[0], t.c[s - 1]), (0.0, 1.0));
        // FSAL: 最後の行 = b、b の最後は 0（13 段目は次の刻みの最初の段）
        assert_eq!(t.b()[s - 1], 0.0);
        // 12 段目（c = 1）と 13 段目（b の行）は同じ時刻
        assert_eq!(t.c[s - 2], 1.0);
        // 時刻はどれも刻みの中
        assert!(t.c.iter().all(|&c| (0.0..=1.0).contains(&c)));
        // 行の和 = c
        for (i, row) in t.a.iter().enumerate() {
            let sum: f64 = row.iter().sum();
            assert!((sum - t.c[i]).abs() < TOL, "段 {i}: {sum} != {}", t.c[i]);
        }
        assert!(t.beta.is_none());
    }

    #[test]
    fn dopri853_main_is_eighth_order_and_embedded_is_fifth() {
        let t = explicit_tableau(SolverId::Dopri853).unwrap();
        // 8 次までの木 1+1+2+4+9+20+48+115 本の条件をすべて通り、9 次は通らない
        assert_eq!(attained_order(t.b(), &t.a, 9), 8, "主解");
        // 埋め込み b̂ = b − d は 5 次ちょうど
        assert_eq!(attained_order(&embedded(&t), &t.a, 6), 5, "埋め込み");
        // diffsol の刻み幅の制御に渡す次数は埋め込み（5 次の見積もり）に合わせる
        assert_eq!(t.order, 5);
        // 主解と埋め込みは Σ = 1 なので Σ d = 0
        assert!(t.d.iter().sum::<f64>().abs() < TOL);
        // 13 段目の重みは d でも 0
        assert_eq!(t.d[12], 0.0);
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

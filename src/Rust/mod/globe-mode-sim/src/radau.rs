//! Radau IIA（3 段、5 次）の刻み幅の制御つき積分（`radau5`。S030）。Rumoca に依存しない純粋な数値部で、
//! 実行系（`globe-mode-exec` の `plugins::radau`）が `MeIntegratorBackend` に包む。
//!
//! 出典: E. Hairer, G. Wanner, "Solving Ordinary Differential Equations II", 2nd ed., Springer (1996)。
//! Radau IIA の定義（§IV.5）と、刻み幅の制御・誤差の見積もり（§IV.8）。【書籍の本文は未確認】
//! Fortran の `radau5.f` は読んでおらず写していない。係数と誤差の見積もりの式は下のとおり自分で導いて、
//! 試験（`coefficients_*`・`embedded_*`）で数値的に確かめている。
//!
//! ## 係数
//!
//! 段の時刻は `c = ((4-√6)/10, (4+√6)/10, 1)`（右端が 1 の Gauss 型求積の節点）。`a` は配置法
//! `a_ij = ∫₀^{c_i} ℓ_j(s) ds`（ℓ は c の Lagrange 基底）の値で、閉じた式を使う。`b` は `a` の最後の行
//! （stiffly accurate なので `y₁ = y₀ + Z₃`）。条件 B(5)・C(3)・D(1) を試験で確かめる（これで 5 次）。
//!
//! ## 段の方程式
//!
//! 段の増分 `Z_i = Y_i - y₀` は `Z = h (A⊗I) F(Z)`、`F_i = f(t₀ + c_i h, y₀ + Z_i)` を満たす。
//! 簡略 Newton 法で解く: ヤコビ行列 `J = ∂f/∂y(t₀, y₀)` を歩の始めに 1 回だけ取り、
//! `(I - h A⊗J) ΔZ = Z - h (A⊗I) F(Z)` を反復する（3n×3n の行列を 1 回だけ LU 分解して使い回す）。
//!
//! ## 誤差の見積もり（導出）
//!
//! 埋め込みの 3 次の公式を `ŷ₁ = y₀ + h (γ₀ f₀ + Σᵢ b̂ᵢ fᵢ)` とする（`f₀ = f(t₀, y₀)`、`γ₀` は `A` の実の
//! 固有値 `1/(3 + ∛9 - ∛3) = 0.2749`。`60λ³ - 36λ² + 9λ - 1 = 0` の実根）。3 次の条件
//! `γ₀ + Σ b̂ᵢ = 1`、`Σ b̂ᵢ cᵢ = 1/2`、`Σ b̂ᵢ cᵢ² = 1/3` で `b̂` が決まる。本当の `b` は
//! `Σ bᵢ cᵢ^(k-1) = 1/k` を満たすので、`b̂ = b - γ₀ w`（`w` は `Σ wᵢ cᵢ^k = δ_{k0}`、k = 0, 1, 2 の解）。
//! 段の方程式から `h fᵢ = Σⱼ (A⁻¹)ᵢⱼ Zⱼ` なので、`b = Aᵀe₃`（`e₃` は第 3 単位ベクトル）を使って
//!
//! ```text
//! y₁ - ŷ₁ = Z₃ - γ₀ h f₀ - (b̂ᵀ A⁻¹) Z = -γ₀ ( h f₀ + Σⱼ ddⱼ Zⱼ ),   dd = -A⁻ᵀ w
//! ```
//!
//! `dd` は `√6` だけで書けて `(-(13+7√6)/3, (-13+7√6)/3, -1/3)`（試験で `w = -Aᵀdd` が
//! `Σ wᵢ cᵢ^k = δ_{k0}` を満たすことを確かめる）。この差は `O(h⁴)` だが、硬い成分では減衰しないので、
//! HW の式どおり `(I - hγ₀J)⁻¹` を掛けて見積もりにする: `err = (I - hγ₀J)⁻¹ γ₀ (h f₀ + Σⱼ ddⱼ Zⱼ)`。
//! shortcut: HW は最初の歩と棄却の直後に `f₀` を `f(y₀ + err)` に替えて硬い成分の見積もりを直すが、
//! ここでは省いた（硬い問題で刻みを不必要に縮めることがあれば足す）。
//!
//! ## 刻み幅の制御
//!
//! 誤差のノルムは重み `atol_i + rtol·max(|y₀ᵢ|, |y₁ᵢ|)` の RMS で、`h_new = h·fac·err^(-1/4)`
//! （`fac` は 0.9 に Newton 法の反復数で補正、`[0.2, 8]` に制限）。誤差が 1 を超えるか Newton 法が収束しなければ
//! 刻みを縮めてやり直す（Newton 法の失敗は 1/2）。
//!
//! shortcut: 3n×3n の密な LU を歩ごとに作る。状態の数 n が大きいと O(n³) で遅い。そのときは HW の
//! 変換（`A⁻¹` の固有値分解で n×n の実 1 本と複素 1 本に分ける）と、ヤコビ行列・LU の使い回しにする。
//! shortcut: 外挿の初期値は直前の歩が連続のときだけ（それ以外は 0。反復が増える）。

// 行列の添字を p・q・i・j で回す式が多く、イテレータにすると平行な配列の対応が読みにくくなる
#![allow(clippy::needless_range_loop)]

use crate::integrators::Knot;

/// 段の数。
const STAGES: usize = 3;
/// 1 歩の Newton 法の反復の上限。
const NEWTON_MAX_ITERATIONS: usize = 10;
/// 1 回の `advance` で刻みを縮めてやり直す回数の上限。
const MAX_ATTEMPTS: usize = 50;
/// 刻みを増やす倍率の上限と、縮める倍率の下限。
const MAX_GROWTH: f64 = 8.0;
const MIN_SHRINK: f64 = 0.2;
/// 誤差の見積もりの次数 + 1 の逆数（埋め込みは 3 次なので局所誤差は h⁴）。
const ERROR_EXPONENT: f64 = -0.25;

/// 許容誤差（重みは `atol_i + rtol·|y_i|`）。
#[derive(Debug, Clone, Copy)]
pub struct Tolerances<'a> {
    pub rtol: f64,
    /// 成分ごとの絶対許容誤差
    pub atol: &'a [f64],
}

/// Radau IIA 3 段の係数と、誤差の見積もりの係数。
#[derive(Debug, Clone, PartialEq)]
pub struct Coefficients {
    /// 段の時刻の位置
    pub c: [f64; STAGES],
    /// 段の係数。`b` は最後の行
    pub a: [[f64; STAGES]; STAGES],
    /// 誤差の見積もりの係数（上の導出の `dd`）
    pub dd: [f64; STAGES],
    /// `A` の実の固有値
    pub gamma0: f64,
}

impl Coefficients {
    pub fn new() -> Self {
        let s = 6.0_f64.sqrt();
        Self {
            c: [(4.0 - s) / 10.0, (4.0 + s) / 10.0, 1.0],
            a: [
                [
                    (88.0 - 7.0 * s) / 360.0,
                    (296.0 - 169.0 * s) / 1800.0,
                    (-2.0 + 3.0 * s) / 225.0,
                ],
                [
                    (296.0 + 169.0 * s) / 1800.0,
                    (88.0 + 7.0 * s) / 360.0,
                    (-2.0 - 3.0 * s) / 225.0,
                ],
                [(16.0 - s) / 36.0, (16.0 + s) / 36.0, 1.0 / 9.0],
            ],
            dd: [-(13.0 + 7.0 * s) / 3.0, (-13.0 + 7.0 * s) / 3.0, -1.0 / 3.0],
            gamma0: 1.0 / (3.0 + 9.0_f64.cbrt() - 3.0_f64.cbrt()),
        }
    }

    /// 主解の重み（`a` の最後の行）。
    pub fn b(&self) -> [f64; STAGES] {
        self.a[STAGES - 1]
    }
}

impl Default for Coefficients {
    fn default() -> Self {
        Self::new()
    }
}

/// 部分ピボットつきの LU 分解（`a` を L（単位下三角の乗数）と U で置き換える）。特異か非有限なら false。
fn lu_factor(a: &mut [f64], n: usize, piv: &mut [usize]) -> bool {
    for k in 0..n {
        let p = (k..n)
            .max_by(|&i, &j| a[i * n + k].abs().total_cmp(&a[j * n + k].abs()))
            .unwrap_or(k);
        let pivot = a[p * n + k];
        if !(pivot.abs() > 0.0 && pivot.is_finite()) {
            return false;
        }
        piv[k] = p;
        if p != k {
            for c in 0..n {
                a.swap(k * n + c, p * n + c);
            }
        }
        for i in k + 1..n {
            let m = a[i * n + k] / pivot;
            a[i * n + k] = m;
            if m != 0.0 {
                for c in k + 1..n {
                    a[i * n + c] -= m * a[k * n + c];
                }
            }
        }
    }
    true
}

/// [`lu_factor`] の結果で `A x = b` を解き、`b` を `x` で置き換える。
fn lu_solve(lu: &[f64], n: usize, piv: &[usize], b: &mut [f64]) {
    for k in 0..n {
        b.swap(k, piv[k]);
    }
    for i in 1..n {
        let s: f64 = (0..i).map(|j| lu[i * n + j] * b[j]).sum();
        b[i] -= s;
    }
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|j| lu[i * n + j] * b[j]).sum();
        b[i] = (b[i] - s) / lu[i * n + i];
    }
}

/// Radau IIA 3 段の積分器。作業用の領域と、制御が選ぶ次の刻みを持つ。
#[derive(Debug, Clone)]
pub struct Radau {
    coef: Coefficients,
    n: usize,
    /// 次の歩の刻みの候補
    h: f64,
    /// 受理した歩の数、棄却した試行の数（誤差・Newton 法の失敗・ホストの拒否）
    pub accepted: usize,
    pub rejected: usize,
    jac: Vec<f64>,
    /// `I - h A⊗J`（3n×3n、LU 分解後は L と U）
    big: Vec<f64>,
    big_piv: Vec<usize>,
    /// `I - hγ₀J`（n×n）
    small: Vec<f64>,
    small_piv: Vec<usize>,
    /// 段の増分 Z（3n）
    z: Vec<f64>,
    /// 残差、続いて Newton の修正量（3n）
    g: Vec<f64>,
    /// 段の導関数（3n）
    stage_f: Vec<f64>,
    stage_y: Vec<f64>,
    y1: Vec<f64>,
    err: Vec<f64>,
    scale: Vec<f64>,
    seed: Vec<f64>,
    column: Vec<f64>,
    /// 直近の試行の Newton 法の反復数
    newton_iterations: usize,
    /// 直前に受理した歩の終点（時刻、値）、刻み、段の増分。次の歩の Newton 法の初期値の外挿に使う
    prev: Option<(f64, f64)>,
    prev_y1: Vec<f64>,
    prev_z: Vec<f64>,
}

impl Radau {
    /// 状態の数 `width` の系に使う。`h0` は最初の刻みの候補。
    pub fn new(width: usize, h0: f64) -> Self {
        let n = width;
        Self {
            coef: Coefficients::new(),
            n,
            h: h0,
            accepted: 0,
            rejected: 0,
            jac: vec![0.0; n * n],
            big: vec![0.0; 9 * n * n],
            big_piv: vec![0; STAGES * n],
            small: vec![0.0; n * n],
            small_piv: vec![0; n],
            z: vec![0.0; STAGES * n],
            g: vec![0.0; STAGES * n],
            stage_f: vec![0.0; STAGES * n],
            stage_y: vec![0.0; n],
            y1: vec![0.0; n],
            err: vec![0.0; n],
            scale: vec![0.0; n],
            seed: vec![0.0; n],
            column: vec![0.0; n],
            newton_iterations: 0,
            prev: None,
            prev_y1: vec![0.0; n],
            prev_z: vec![0.0; STAGES * n],
        }
    }

    pub fn coefficients(&self) -> &Coefficients {
        &self.coef
    }

    /// 次の歩の刻みの候補。
    pub fn step_size(&self) -> f64 {
        self.h
    }

    /// 直近の試行の終点 `y₁`（Newton 法が収束したときだけ意味がある）。
    pub fn solution(&self) -> &[f64] {
        &self.y1
    }

    /// 直近の試行の誤差の見積もりのベクトル（上の `err`）。
    pub fn error(&self) -> &[f64] {
        &self.err
    }

    /// 刻み `h` を 1 回試す（棄却しても状態は進まない）。`f(t, y, dy)` は導関数、`jv(t, y, v, dv)` は
    /// `dv = (∂f/∂y)·v`。成功したら重み付き RMS の誤差のノルム（1 以下なら許容誤差の内）を返し、
    /// 終点は [`Self::solution`]、誤差のベクトルは [`Self::error`] に残る。
    /// Newton 法が収束しない・行列が特異・値が有限でないときは Err（呼び手が刻みを縮める）。
    pub fn attempt(
        &mut self,
        start: Knot<'_>,
        h: f64,
        tol: Tolerances<'_>,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
        mut jv: impl FnMut(f64, &[f64], &[f64], &mut [f64]),
    ) -> Result<f64, String> {
        let Knot { t, y, f: f0 } = start;
        let (n, m) = (self.n, STAGES * self.n);
        if !(h > 0.0 && h.is_finite()) {
            return Err(format!("刻み {h} は正の有限の値でなければならない"));
        }
        for i in 0..n {
            self.scale[i] = tol.atol[i] + tol.rtol * y[i].abs();
        }
        // ヤコビ行列は始点で 1 回だけ（方向微分を単位ベクトルごとに）
        for j in 0..n {
            self.seed.fill(0.0);
            self.seed[j] = 1.0;
            jv(t, y, &self.seed, &mut self.column);
            for i in 0..n {
                self.jac[i * n + j] = self.column[i];
            }
        }
        if !self.jac.iter().all(|v| v.is_finite()) {
            return Err(format!("t = {t} のヤコビ行列が有限でない"));
        }
        for bi in 0..STAGES {
            for bj in 0..STAGES {
                let ha = h * self.coef.a[bi][bj];
                for p in 0..n {
                    for q in 0..n {
                        let identity = if bi == bj && p == q { 1.0 } else { 0.0 };
                        self.big[(bi * n + p) * m + bj * n + q] =
                            identity - ha * self.jac[p * n + q];
                    }
                }
            }
        }
        if !lu_factor(&mut self.big, m, &mut self.big_piv) {
            return Err(format!(
                "t = {t} の段の行列 I - hA⊗J が特異か、値が有限でない（h = {h}）"
            ));
        }
        self.newton(t, y, h, tol.rtol, &mut f)?;
        for i in 0..n {
            self.y1[i] = y[i] + self.z[(STAGES - 1) * n + i];
        }
        self.estimate_error(f0, h)?;
        let mut sum = 0.0;
        for i in 0..n {
            let sc = tol.atol[i] + tol.rtol * y[i].abs().max(self.y1[i].abs());
            sum += (self.err[i] / sc).powi(2);
        }
        let norm = (sum / n.max(1) as f64).sqrt();
        if !norm.is_finite() {
            return Err(format!("t = {t} の誤差の見積もりが有限でない（h = {h}）"));
        }
        Ok(norm)
    }

    /// Newton 法の初期値 `z`。直前の歩（終点が今の始点）の配置多項式（0, c₁, c₂, 1 を通る 3 次）を今の段の時刻まで
    /// 外挿する（HW と同じ）。直前の歩が無い・始点が違う・刻みの比が 2 を超えるときは 0。
    fn predict(&mut self, t: f64, y: &[f64], h: f64) {
        let n = self.n;
        self.z.fill(0.0);
        let Some((t1, prev_h)) = self.prev else {
            return;
        };
        if t1 != t || self.prev_y1 != y || h > 2.0 * prev_h {
            return;
        }
        let x = [0.0, self.coef.c[0], self.coef.c[1], 1.0];
        for i in 0..STAGES {
            let s = 1.0 + self.coef.c[i] * h / prev_h;
            // 節点 k（k = 1..3）の Lagrange 基底。節点 0 の値は 0（増分）なので足さない
            for k in 1..=STAGES {
                let mut l = 1.0;
                for (m, xm) in x.iter().enumerate() {
                    if m != k {
                        l *= (s - xm) / (x[k] - xm);
                    }
                }
                for p in 0..n {
                    self.z[i * n + p] += l * self.prev_z[(k - 1) * n + p];
                }
            }
            for p in 0..n {
                self.z[i * n + p] -= self.prev_z[(STAGES - 1) * n + p];
            }
        }
    }

    /// 簡略 Newton 法で段の増分 `z` を解く。
    fn newton(
        &mut self,
        t: f64,
        y: &[f64],
        h: f64,
        rtol: f64,
        f: &mut impl FnMut(f64, &[f64], &mut [f64]),
    ) -> Result<(), String> {
        let (n, m) = (self.n, STAGES * self.n);
        self.predict(t, y, h);
        // 修正量（許容誤差を 1 とした重み付き RMS）が許容誤差の fnewt 倍より小さくなれば収束（HW と同じ式）
        let fnewt = f64::max(10.0 * f64::EPSILON / rtol, 0.03_f64.min(rtol.sqrt()));
        let mut old_norm = 0.0;
        for k in 0..NEWTON_MAX_ITERATIONS {
            for i in 0..STAGES {
                for p in 0..n {
                    self.stage_y[p] = y[p] + self.z[i * n + p];
                }
                f(
                    t + self.coef.c[i] * h,
                    &self.stage_y,
                    &mut self.stage_f[i * n..(i + 1) * n],
                );
            }
            for i in 0..STAGES {
                for p in 0..n {
                    let af: f64 = (0..STAGES)
                        .map(|j| self.coef.a[i][j] * self.stage_f[j * n + p])
                        .sum();
                    self.g[i * n + p] = self.z[i * n + p] - h * af;
                }
            }
            if !self.g.iter().all(|v| v.is_finite()) {
                return Err(format!(
                    "t = {t} の Newton 法が有限でない値に達した（h = {h}）"
                ));
            }
            lu_solve(&self.big, m, &self.big_piv, &mut self.g);
            let mut sum = 0.0;
            for i in 0..STAGES {
                for p in 0..n {
                    let d = self.g[i * n + p];
                    self.z[i * n + p] -= d;
                    sum += (d / self.scale[p]).powi(2);
                }
            }
            let norm = (sum / m.max(1) as f64).sqrt();
            if !norm.is_finite() {
                return Err(format!(
                    "t = {t} の Newton 法が有限でない値に達した（h = {h}）"
                ));
            }
            if norm <= 0.01 * fnewt {
                self.newton_iterations = k + 1;
                return Ok(());
            }
            if k > 0 {
                let theta = norm / old_norm;
                if theta >= 0.99 {
                    return Err(format!(
                        "t = {t} の Newton 法が発散した（h = {h}、収束率 {theta:.3}）"
                    ));
                }
                let remaining = (NEWTON_MAX_ITERATIONS - 1 - k) as i32;
                // 残りの反復でも fnewt に届かない見込みなら早めに諦める
                if norm * theta.powi(remaining) / (1.0 - theta) > fnewt {
                    return Err(format!(
                        "t = {t} の Newton 法が収束しそうにない（h = {h}、収束率 {theta:.3}）"
                    ));
                }
                if theta / (1.0 - theta) * norm <= fnewt {
                    self.newton_iterations = k + 1;
                    return Ok(());
                }
            }
            old_norm = norm;
        }
        Err(format!(
            "t = {t} の Newton 法が {NEWTON_MAX_ITERATIONS} 回で収束しなかった（h = {h}）"
        ))
    }

    /// `err = (I - hγ₀J)⁻¹ γ₀ (h f₀ + Σⱼ ddⱼ Zⱼ)`（上の導出）。
    fn estimate_error(&mut self, f0: &[f64], h: f64) -> Result<(), String> {
        let n = self.n;
        let g0 = self.coef.gamma0;
        for p in 0..n {
            let mixed: f64 = (0..STAGES)
                .map(|j| self.coef.dd[j] * self.z[j * n + p])
                .sum();
            self.err[p] = g0 * (h * f0[p] + mixed);
            for q in 0..n {
                let identity = if p == q { 1.0 } else { 0.0 };
                self.small[p * n + q] = identity - h * g0 * self.jac[p * n + q];
            }
        }
        if !lu_factor(&mut self.small, n, &mut self.small_piv) {
            return Err(format!("行列 I - hγ₀J が特異か、値が有限でない（h = {h}）"));
        }
        lu_solve(&self.small, n, &self.small_piv, &mut self.err);
        Ok(())
    }

    /// 始点 `start` から、時刻 `t_limit` を超えない範囲で 1 歩を受理するまで進める（棄却したら刻みを縮めて
    /// やり直す）。終点の値と導関数を `out_y`・`out_f` に書き、終点の時刻を返す。
    /// `veto()` は 1 回の試行のたびに呼ばれ、true ならその試行を棄てて刻みを縮める（ホストが試しの点を断ったとき）。
    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        start: Knot<'_>,
        t_limit: f64,
        tol: Tolerances<'_>,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
        mut jv: impl FnMut(f64, &[f64], &[f64], &mut [f64]),
        mut veto: impl FnMut() -> bool,
        out_y: &mut [f64],
        out_f: &mut [f64],
    ) -> Result<f64, String> {
        let t0 = start.t;
        let remaining = t_limit - t0;
        if !(remaining > 0.0 && remaining.is_finite()) {
            return Err(format!("上限 {t_limit} が始点 {t0} より先にない"));
        }
        let mut rejected = false;
        let mut last_failure = String::new();
        for _ in 0..MAX_ATTEMPTS {
            if t0 + self.h == t0 || self.h < 100.0 * f64::EPSILON * t0.abs() {
                break;
            }
            // 上限の少し手前（1%）なら最後の歩を上限ちょうどにする（小さな端数の歩を作らない）
            let clipped = t0 + 1.01 * self.h >= t_limit;
            let h = if clipped { remaining } else { self.h };
            let outcome = self.attempt(start, h, tol, &mut f, &mut jv);
            let vetoed = veto();
            match outcome {
                Ok(err) if !vetoed && err <= 1.0 => {
                    let t1 = if clipped { t_limit } else { t0 + h };
                    out_y.copy_from_slice(&self.y1);
                    f(t1, out_y, out_f);
                    self.prev = Some((t1, h));
                    self.prev_y1.copy_from_slice(&self.y1);
                    self.prev_z.copy_from_slice(&self.z);
                    // Newton 法の反復が多かった歩は増やし方を控える（HW と同じ補正）
                    let safety = 0.9 * (1.0 + 2.0 * NEWTON_MAX_ITERATIONS as f64)
                        / (self.newton_iterations as f64 + 2.0 * NEWTON_MAX_ITERATIONS as f64);
                    let mut fac = (safety * err.max(1e-10).powf(ERROR_EXPONENT))
                        .clamp(MIN_SHRINK, MAX_GROWTH);
                    if rejected {
                        fac = fac.min(1.0);
                    }
                    let proposed = h * fac;
                    // 上限で切った歩は、元の刻みの候補より小さい見積もりにならない限り候補を下げない
                    self.h = if clipped {
                        proposed.max(self.h)
                    } else {
                        proposed
                    };
                    self.accepted += 1;
                    return Ok(t1);
                }
                Ok(err) if !vetoed => {
                    let fac = (0.9 * err.powf(ERROR_EXPONENT)).clamp(MIN_SHRINK, 1.0);
                    self.h = h * fac;
                    last_failure =
                        format!("誤差の見積もり {err:.3e} が許容誤差を超えた（h = {h}）");
                }
                Ok(_) => {
                    self.h = h * 0.5;
                    last_failure = format!("ホストが試しの点を断った（h = {h}）");
                }
                Err(message) => {
                    self.h = h * 0.5;
                    last_failure = message;
                }
            }
            rejected = true;
            self.rejected += 1;
        }
        Err(format!(
            "刻みが小さくなりすぎた（t = {t0}、h = {}）。最後の失敗: {last_failure}",
            self.h
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 条件 B(5)・C(3)・D(1)（Butcher）が成り立てば 5 次。c = A·1、b は a の最後の行。
    #[test]
    fn coefficients_satisfy_the_order_five_conditions() {
        let Coefficients { c, a, .. } = Coefficients::new();
        let b = a[2];
        for i in 0..3 {
            assert!(
                (a[i].iter().sum::<f64>() - c[i]).abs() < 1e-15,
                "行 {i} の和"
            );
        }
        for k in 1..=5 {
            let s: f64 = (0..3).map(|i| b[i] * c[i].powi(k - 1)).sum();
            assert!((s - 1.0 / f64::from(k)).abs() < 1e-15, "B({k}): {s}");
        }
        // 6 次は成り立たない（ちょうど 5 次）
        let b6: f64 = (0..3).map(|i| b[i] * c[i].powi(5)).sum();
        assert!((b6 - 1.0 / 6.0).abs() > 1e-3, "B(6): {b6}");
        for k in 1..=3 {
            for i in 0..3 {
                let s: f64 = (0..3).map(|j| a[i][j] * c[j].powi(k - 1)).sum();
                assert!(
                    (s - c[i].powi(k) / f64::from(k)).abs() < 1e-15,
                    "C({k}) 行 {i}"
                );
            }
        }
        for j in 0..3 {
            let s: f64 = (0..3).map(|i| b[i] * a[i][j]).sum();
            assert!((s - b[j] * (1.0 - c[j])).abs() < 1e-15, "D(1) 列 {j}");
        }
    }

    /// γ₀ は A の固有値（特性方程式 60λ³ - 36λ² + 9λ - 1 = 0 の実根）で、dd は導出どおり
    /// `w = -Aᵀdd` が `Σ wᵢ cᵢ^k = δ_{k0}`（k = 0, 1, 2）を満たす。
    #[test]
    fn coefficients_of_the_error_estimate_follow_the_derivation() {
        let Coefficients { c, a, dd, gamma0 } = Coefficients::new();
        let l = gamma0;
        let poly = 60.0 * l * l * l - 36.0 * l * l + 9.0 * l - 1.0;
        assert!(poly.abs() < 1e-14, "特性方程式の残り {poly}");
        assert!((l - 0.274_888_829_595_677).abs() < 1e-14, "{l}");
        let w: Vec<f64> = (0..3)
            .map(|j| -(0..3).map(|i| a[i][j] * dd[i]).sum::<f64>())
            .collect();
        for k in 0..3 {
            let s: f64 = (0..3).map(|i| w[i] * c[i].powi(k)).sum();
            let want = if k == 0 { 1.0 } else { 0.0 };
            assert!((s - want).abs() < 1e-13, "Σ w c^{k} = {s}");
        }
    }

    /// 1 次元の系で 1 歩を試す。
    fn try_step(
        radau: &mut Radau,
        t: f64,
        y: f64,
        h: f64,
        f: impl Fn(f64, f64) -> f64,
        dfdy: impl Fn(f64, f64) -> f64,
    ) -> f64 {
        let f0 = [f(t, y)];
        let tol = Tolerances {
            rtol: 1e-9,
            atol: &[1e-11],
        };
        radau
            .attempt(
                Knot { t, y: &[y], f: &f0 },
                h,
                tol,
                |t, y, dy| dy[0] = f(t, y[0]),
                |t, y, v, dv| dv[0] = dfdy(t, y[0]) * v[0],
            )
            .unwrap()
    }

    /// 埋め込みの公式は 3 次: y' = t^m（m = 0, 1, 2）は誤差の見積もりが 0、m = 3 は 0 でない。
    #[test]
    fn embedded_estimate_is_exact_for_quadratic_right_hand_sides() {
        for m in 0..4 {
            let mut radau = Radau::new(1, 0.1);
            try_step(&mut radau, 0.0, 0.0, 0.5, |t, _| t.powi(m), |_, _| 0.0);
            let e = radau.error()[0].abs();
            if m < 3 {
                assert!(e < 1e-15, "m = {m}: {e}");
            } else {
                assert!(e > 1e-4, "m = {m}: {e}");
            }
            // 主解は 4 次の多項式まで正確
            let exact = 0.5_f64.powi(m + 1) / f64::from(m + 1);
            assert!((radau.solution()[0] - exact).abs() < 1e-15, "m = {m}");
        }
    }

    /// 誤差の見積もりは h⁴ で減り、1 歩の本当の誤差は h⁶ で減る（y' = -y）。
    #[test]
    fn error_estimate_scales_as_h_fourth_and_local_error_as_h_sixth() {
        let mut radau = Radau::new(1, 0.1);
        let mut estimate = |h: f64| {
            try_step(&mut radau, 0.0, 1.0, h, |_, y| -y, |_, _| -1.0);
            (
                radau.error()[0].abs(),
                (radau.solution()[0] - (-h).exp()).abs(),
            )
        };
        let ((e1, d1), (e2, d2)) = (estimate(0.2), estimate(0.1));
        let (est_order, true_order) = ((e1 / e2).log2(), (d1 / d2).log2());
        eprintln!(
            "見積もり {e1:.3e} → {e2:.3e}（次数 {est_order:.3}）、本当の誤差 {d1:.3e} → {d2:.3e}（次数 {true_order:.3}）"
        );
        assert!((est_order - 4.0).abs() < 0.3, "{est_order}");
        assert!((true_order - 6.0).abs() < 0.3, "{true_order}");
        let ((e3, _), (e4, _)) = (estimate(0.05), estimate(0.025));
        assert!(((e3 / e4).log2() - 4.0).abs() < 0.1, "{}", (e3 / e4).log2());
    }

    /// 固定刻みで回した大域誤差の次数は 5（非線形 y' = -y³、解 1/√(1+2t)）。
    #[test]
    fn global_order_is_five() {
        let error = |steps: usize| {
            let h = 1.0 / steps as f64;
            let mut radau = Radau::new(1, h);
            let mut y = 1.0;
            for k in 0..steps {
                let t = k as f64 * h;
                try_step(&mut radau, t, y, h, |_, y| -y.powi(3), |_, y| -3.0 * y * y);
                y = radau.solution()[0];
            }
            (y - 3.0_f64.sqrt().recip()).abs()
        };
        let (e1, e2, e3) = (error(8), error(16), error(32));
        let (o1, o2) = ((e1 / e2).log2(), (e2 / e3).log2());
        eprintln!("大域誤差 {e1:.3e} {e2:.3e} {e3:.3e}、次数 {o1:.3} {o2:.3}");
        assert!((o2 - 5.0).abs() < 0.25, "次数 {o1} {o2}");
    }

    /// 制御つきで t_end まで回す。(終点, 受理した歩の数, 棄却した試行の数, 導関数の呼び出し回数)
    fn integrate(
        y0: Vec<f64>,
        t_end: f64,
        rtol: f64,
        atol: f64,
        h0: f64,
        f: impl Fn(f64, &[f64], &mut [f64]),
        jv: impl Fn(f64, &[f64], &[f64], &mut [f64]),
    ) -> (Vec<f64>, usize, usize, usize) {
        let n = y0.len();
        let atols = vec![atol; n];
        let tol = Tolerances { rtol, atol: &atols };
        let calls = std::cell::Cell::new(0_usize);
        let counted = |t: f64, y: &[f64], dy: &mut [f64]| {
            calls.set(calls.get() + 1);
            f(t, y, dy);
        };
        let mut radau = Radau::new(n, h0);
        let (mut t, mut y, mut f0) = (0.0, y0, vec![0.0; n]);
        f(t, &y, &mut f0);
        let (mut out_y, mut out_f) = (vec![0.0; n], vec![0.0; n]);
        while t < t_end {
            let start = Knot { t, y: &y, f: &f0 };
            t = radau
                .advance(
                    start,
                    t_end,
                    tol,
                    counted,
                    &jv,
                    || false,
                    &mut out_y,
                    &mut out_f,
                )
                .unwrap();
            y.copy_from_slice(&out_y);
            f0.copy_from_slice(&out_f);
        }
        assert_eq!(t, t_end);
        (y, radau.accepted, radau.rejected, calls.get())
    }

    /// 硬い問題 y' = -1000(y - cos t)、y(0) = 0 を少ない歩で安定に解く。
    #[test]
    fn stiff_problem_is_solved_in_few_steps() {
        let k = 1000.0_f64;
        let exact = |t: f64| {
            let particular = k * (k * t.cos() + t.sin()) / (k * k + 1.0);
            particular - k * k / (k * k + 1.0) * (-k * t).exp()
        };
        let (y, steps, rejected, calls) = integrate(
            vec![0.0],
            1.0,
            1e-6,
            1e-8,
            1e-3,
            |t, y, dy| dy[0] = -k * (y[0] - t.cos()),
            |_, _, v, dv| dv[0] = -k * v[0],
        );
        let error = (y[0] - exact(1.0)).abs();
        eprintln!("硬い問題: {steps} 歩、棄却 {rejected}、f {calls} 回、誤差 {error:.3e}");
        assert!(error < 1e-5, "誤差 {error}");
        // 陽的な方法は安定の限界 h ≲ 3/1000 で 330 歩以上かかる
        assert!(steps < 60, "歩の数 {steps}");
    }

    /// 刻みが大きすぎる出発（誤差の棄却）からも、許容誤差の内の答えに着く（y' = -y³）。
    #[test]
    fn rejects_a_too_large_first_step_and_recovers() {
        let (y, steps, rejected, _) = integrate(
            vec![1.0],
            10.0,
            1e-8,
            1e-10,
            10.0,
            |_, y, dy| dy[0] = -y[0].powi(3),
            |_, y, v, dv| dv[0] = -3.0 * y[0] * y[0] * v[0],
        );
        let exact = 21.0_f64.sqrt().recip();
        eprintln!(
            "棄却の復帰: {steps} 歩、棄却 {rejected}、誤差 {:.3e}",
            (y[0] - exact).abs()
        );
        assert!(rejected > 0);
        assert!((y[0] - exact).abs() < 1e-7);
    }

    /// 2 次元の線形の系（調和振動子）の終点が解析解に合う。
    #[test]
    fn oscillator_matches_the_analytic_solution() {
        let (y, steps, _, _) = integrate(
            vec![1.0, 0.0],
            2.0,
            1e-8,
            1e-10,
            1e-2,
            |_, y, dy| {
                dy[0] = y[1];
                dy[1] = -y[0];
            },
            |_, _, v, dv| {
                dv[0] = v[1];
                dv[1] = -v[0];
            },
        );
        let (ex, ev) = (2.0_f64.cos(), -2.0_f64.sin());
        eprintln!(
            "振動子: {steps} 歩、誤差 {:.3e} {:.3e}",
            (y[0] - ex).abs(),
            (y[1] - ev).abs()
        );
        assert!((y[0] - ex).abs() < 1e-6 && (y[1] - ev).abs() < 1e-6);
    }

    /// 導関数が有限でない（ホストが断った）試行は棄てて刻みを縮め、`veto` が true の試行も棄てる。
    #[test]
    fn vetoed_and_non_finite_trials_shrink_the_step() {
        let mut radau = Radau::new(1, 1.0);
        let f0 = [-1.0];
        let tol = Tolerances {
            rtol: 1e-8,
            atol: &[1e-10],
        };
        let mut vetoes = 2;
        let (mut out_y, mut out_f) = ([0.0], [0.0]);
        let t1 = radau
            .advance(
                Knot {
                    t: 0.0,
                    y: &[1.0],
                    f: &f0,
                },
                10.0,
                tol,
                // 刻みが 0.3 を超える試行では NaN を返す
                |t, y, dy| dy[0] = if t > 0.3 { f64::NAN } else { -y[0] },
                |_, _, v, dv| dv[0] = -v[0],
                || {
                    let v = vetoes > 0;
                    vetoes -= 1;
                    v
                },
                &mut out_y,
                &mut out_f,
            )
            .unwrap();
        assert!(t1 <= 0.3 && t1 > 0.0, "{t1}");
        assert!(radau.rejected >= 3);
        assert!((out_y[0] - (-t1).exp()).abs() < 1e-7);
    }
}

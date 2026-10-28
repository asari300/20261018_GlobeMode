//! 固定刻みの積分の係数と、刻みの間の補間（S012）。Rumoca に依存しない純粋な関数で、実行系の積分器
//! （`globe-mode-exec` の `plugins`）と、後で作る自前の driver（FMU と Track B）で使い回す。
//!
//! - [`Tableau`] … 陽的な Runge–Kutta 法の係数（Butcher 表）。[`EULER`]・[`HEUN`]・[`RK4`]。
//! - [`ExplicitStepper`] … 1 歩を進める（段の値の置き場を使い回す）。
//! - [`ThetaMethod`]・[`ThetaStepper`] … 陰的 Euler 法と台形公式（θ 法）。1 歩ごとに Newton 法で解く。
//! - [`hermite`] … 刻みの両端の値と微分（[`Knot`]）から作る 3 次の Hermite 補間。
//! - [`next_grid_time`] … 固定刻みの格子 t0 + k·h の、今より先の最初の点。

use globe_mode_ir::SolverId;

/// 陽的な Runge–Kutta 法の係数（Butcher 表）。`a` は狭義の下三角（`a[i]` の長さは `i`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tableau {
    /// 名前（GlobeMode のソルバの名前と同じ）
    pub name: &'static str,
    /// 段の係数。`a[i][j]`（`j < i`）
    pub a: &'static [&'static [f64]],
    /// 重み
    pub b: &'static [f64],
    /// 段の時刻の位置
    pub c: &'static [f64],
    /// 大域誤差の次数
    pub order: u32,
}

impl Tableau {
    /// 段の数。
    pub const fn stages(&self) -> usize {
        self.b.len()
    }
}

/// 陽的 Euler 法（1 段、1 次）。
pub const EULER: Tableau = Tableau {
    name: "euler",
    a: &[&[]],
    b: &[1.0],
    c: &[0.0],
    order: 1,
};

/// Heun 法（台形則の予測子修正子。2 段、2 次）。
pub const HEUN: Tableau = Tableau {
    name: "heun",
    a: &[&[], &[1.0]],
    b: &[0.5, 0.5],
    c: &[0.0, 1.0],
    order: 2,
};

/// 古典的な 4 段の Runge–Kutta 法（4 次）。
pub const RK4: Tableau = Tableau {
    name: "rk4",
    a: &[&[], &[0.5], &[0.0, 0.5], &[0.0, 0.0, 1.0]],
    b: &[1.0 / 6.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 6.0],
    c: &[0.0, 0.5, 0.5, 1.0],
    order: 4,
};

/// 固定刻みのソルバの係数（陽的なもの）。陽的な固定刻みでなければ None。
pub const fn fixed_step_tableau(id: SolverId) -> Option<&'static Tableau> {
    match id {
        SolverId::Euler => Some(&EULER),
        SolverId::Heun => Some(&HEUN),
        SolverId::Rk4 => Some(&RK4),
        _ => None,
    }
}

/// θ 法 `y₁ = y₀ + h((1-θ) f(t₀, y₀) + θ f(t₁, y₁))`（陰的。θ = 1 が陰的 Euler 法、θ = ½ が台形公式）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThetaMethod {
    /// 名前（GlobeMode のソルバの名前と同じ）
    pub name: &'static str,
    pub theta: f64,
    /// 大域誤差の次数
    pub order: u32,
}

/// 陰的 Euler 法（1 次、L 安定）。
pub const IMPEULER: ThetaMethod = ThetaMethod {
    name: "impeuler",
    theta: 1.0,
    order: 1,
};

/// 台形公式（2 次、A 安定）。
pub const TRAPEZOID: ThetaMethod = ThetaMethod {
    name: "trapezoid",
    theta: 0.5,
    order: 2,
};

/// 固定刻みの陰的なソルバの係数。θ 法でなければ None。
pub const fn theta_method(id: SolverId) -> Option<&'static ThetaMethod> {
    match id {
        SolverId::Impeuler => Some(&IMPEULER),
        SolverId::Trapezoid => Some(&TRAPEZOID),
        _ => None,
    }
}

/// Newton 法の反復の上限。
const NEWTON_MAX_ITERATIONS: usize = 20;
/// Newton 法の収束判定: 修正量が `NEWTON_TOLERANCE·(1 + |y|)` 以下（固定刻みは許容誤差を使わないので定数）。
const NEWTON_TOLERANCE: f64 = 1e-12;

/// θ 法で 1 歩を進める道具。段の方程式 `y - y₀ - h((1-θ) f₀ + θ f(t₁, y)) = 0` を Newton 法で解く。
/// 行列 `I - hθJ` は密で、反復ごとに方向微分（単位ベクトル n 本）から作り直す。
/// shortcut: 密な行列と毎回のヤコビ行列は状態の数 n に O(n²) の領域と O(n³) の時間がかかる。
/// 状態が数百を超える系を回す必要が出たら、疎の形（`state_jacobian_columns`）と LU の使い回しにする。
#[derive(Debug, Clone)]
pub struct ThetaStepper {
    method: &'static ThetaMethod,
    f1: Vec<f64>,
    /// 残差、続いて Newton の修正量
    g: Vec<f64>,
    /// `I - hθJ`（行優先）
    matrix: Vec<f64>,
    seed: Vec<f64>,
    column: Vec<f64>,
}

impl ThetaStepper {
    /// 状態の数 `width` の系に使う。
    pub fn new(method: &'static ThetaMethod, width: usize) -> Self {
        Self {
            method,
            f1: vec![0.0; width],
            g: vec![0.0; width],
            matrix: vec![0.0; width * width],
            seed: vec![0.0; width],
            column: vec![0.0; width],
        }
    }

    /// 始点 `start`（時刻、値、f(t, y)）から刻み `h` で 1 歩進め、`out` に書く。`f(t, y, dy)` は導関数、
    /// `jv(t, y, v, dv)` は `dv = (∂f/∂y)·v`。Newton 法が収束しなければ（特異な行列、非有限の値を含む）Err。
    pub fn step(
        &mut self,
        start: Knot<'_>,
        h: f64,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
        mut jv: impl FnMut(f64, &[f64], &[f64], &mut [f64]),
        out: &mut [f64],
    ) -> Result<(), String> {
        let Knot { t, y, f: f0 } = start;
        let n = y.len();
        let (theta, t1) = (self.method.theta, t + h);
        // 初期値は陽的 Euler 法の予測
        for i in 0..n {
            out[i] = y[i] + h * f0[i];
        }
        for _ in 0..NEWTON_MAX_ITERATIONS {
            f(t1, out, &mut self.f1);
            for i in 0..n {
                self.g[i] = out[i] - y[i] - h * ((1.0 - theta) * f0[i] + theta * self.f1[i]);
            }
            for j in 0..n {
                self.seed.fill(0.0);
                self.seed[j] = 1.0;
                jv(t1, out, &self.seed, &mut self.column);
                for i in 0..n {
                    let identity = if i == j { 1.0 } else { 0.0 };
                    self.matrix[i * n + j] = identity - h * theta * self.column[i];
                }
            }
            if !solve_dense(&mut self.matrix, &mut self.g, n) {
                return Err(format!(
                    "t = {t1} の段の行列 I - hθJ が特異か、値が有限でない"
                ));
            }
            let mut converged = true;
            for (o, g) in out.iter_mut().zip(&self.g) {
                *o -= g;
                if !o.is_finite() {
                    return Err(format!("t = {t1} の Newton 法が有限でない値に達した"));
                }
                converged &= g.abs() <= NEWTON_TOLERANCE * (1.0 + o.abs());
            }
            if converged {
                return Ok(());
            }
        }
        Err(format!(
            "t = {t1} の Newton 法が {NEWTON_MAX_ITERATIONS} 回で収束しなかった（刻み h = {h} が大きすぎる）"
        ))
    }
}

/// 密な連立方程式 `A x = b` を部分ピボットつきの LU 分解で解き、`b` を `x` で置き換える（`a` は壊れる）。
/// 行列が特異（ピボットが 0 か非有限）なら false。
fn solve_dense(a: &mut [f64], b: &mut [f64], n: usize) -> bool {
    for k in 0..n {
        let pivot_row = (k..n)
            .max_by(|&i, &j| a[i * n + k].abs().total_cmp(&a[j * n + k].abs()))
            .unwrap_or(k);
        let pivot = a[pivot_row * n + k];
        if !(pivot.abs() > 0.0 && pivot.is_finite()) {
            return false;
        }
        if pivot_row != k {
            for c in 0..n {
                a.swap(k * n + c, pivot_row * n + c);
            }
            b.swap(k, pivot_row);
        }
        for i in k + 1..n {
            let m = a[i * n + k] / pivot;
            if m != 0.0 {
                for c in k + 1..n {
                    a[i * n + c] -= m * a[k * n + c];
                }
                b[i] -= m * b[k];
            }
        }
    }
    for k in (0..n).rev() {
        let tail: f64 = (k + 1..n).map(|c| a[k * n + c] * b[c]).sum();
        b[k] = (b[k] - tail) / a[k * n + k];
    }
    b.iter().all(|v| v.is_finite())
}

/// 陽的な Runge–Kutta 法で 1 歩を進める道具。段の値の置き場を持ち、歩ごとに確保しない。
#[derive(Debug, Clone)]
pub struct ExplicitStepper {
    tableau: &'static Tableau,
    /// 段の導関数 k_i（`k[0]` は呼び手が渡す f(t, y)）
    k: Vec<Vec<f64>>,
    /// 段の状態 y + h Σ a_ij k_j
    stage: Vec<f64>,
}

impl ExplicitStepper {
    /// 状態の数 `width` の系に使う。
    pub fn new(tableau: &'static Tableau, width: usize) -> Self {
        Self {
            tableau,
            k: vec![vec![0.0; width]; tableau.stages()],
            stage: vec![0.0; width],
        }
    }

    pub fn tableau(&self) -> &'static Tableau {
        self.tableau
    }

    /// `(t, y)` から刻み `h` で 1 歩進め、`out` に書く。`f0` は f(t, y)（前の歩の終点の導関数を使い回せる）。
    /// `f(t, y, dy)` は導関数を `dy` に書く。導関数を呼ぶのは段の数 − 1 回。
    pub fn step(
        &mut self,
        t: f64,
        y: &[f64],
        f0: &[f64],
        h: f64,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
        out: &mut [f64],
    ) {
        let tableau = self.tableau;
        self.k[0].copy_from_slice(f0);
        for i in 1..tableau.stages() {
            self.stage.copy_from_slice(y);
            for (j, &a) in tableau.a[i].iter().enumerate() {
                if a != 0.0 {
                    for (s, k) in self.stage.iter_mut().zip(&self.k[j]) {
                        *s += h * a * k;
                    }
                }
            }
            f(t + tableau.c[i] * h, &self.stage, &mut self.k[i]);
        }
        out.copy_from_slice(y);
        for (k, &b) in self.k.iter().zip(tableau.b) {
            for (o, k) in out.iter_mut().zip(k) {
                *o += h * b * k;
            }
        }
    }
}

/// 補間の節点（時刻、値、微分）。
#[derive(Debug, Clone, Copy)]
pub struct Knot<'a> {
    pub t: f64,
    pub y: &'a [f64],
    /// dy/dt
    pub f: &'a [f64],
}

/// 3 次の Hermite 補間。区間の両端の節点 `left`・`right` の値と微分から、時刻 `t` の値を `out` に書く。
/// 両端では値をそのまま返す（`t == left.t` なら `left.y`、`t == right.t` なら `right.y` と一致する）。
pub fn hermite(left: Knot<'_>, right: Knot<'_>, t: f64, out: &mut [f64]) {
    if t <= left.t {
        out.copy_from_slice(left.y);
        return;
    }
    if t >= right.t {
        out.copy_from_slice(right.y);
        return;
    }
    let h = right.t - left.t;
    let s = (t - left.t) / h;
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    for (i, o) in out.iter_mut().enumerate() {
        *o = h00 * left.y[i] + h10 * h * left.f[i] + h01 * right.y[i] + h11 * h * right.f[i];
    }
}

/// 固定刻みの格子 `origin + k·h`（k は整数）のうち、`now` より `min_gap` を超えて先にある最初の点。
/// 格子の点は掛け算で作る（足し合わせない）ので、イベントで格子の外に止まった後も同じ格子に戻る。
pub fn next_grid_time(origin: f64, h: f64, now: f64, min_gap: f64) -> f64 {
    let mut k = ((now - origin) / h).floor() + 1.0;
    // 割り算の丸めで 1 つ手前や先になることがあるので、両側から合わせる
    while k > 1.0 && origin + (k - 1.0) * h - now > min_gap {
        k -= 1.0;
    }
    while origin + k * h - now <= min_gap {
        k += 1.0;
    }
    origin + k * h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// y' = -y を (0, 1) から 1 歩進めたときの誤差。
    fn one_step_error(tableau: &'static Tableau, h: f64) -> f64 {
        let mut stepper = ExplicitStepper::new(tableau, 1);
        let mut out = [0.0];
        stepper.step(0.0, &[1.0], &[-1.0], h, |_, y, dy| dy[0] = -y[0], &mut out);
        (out[0] - (-h).exp()).abs()
    }

    /// 1 歩の誤差は h^(p+1) で減る（h を半分にすると 2^(p+1) 分の 1）。
    #[test]
    fn local_error_has_the_order_of_the_method() {
        for tableau in [&EULER, &HEUN, &RK4] {
            let (h1, h2) = (0.1, 0.05);
            let ratio = one_step_error(tableau, h1) / one_step_error(tableau, h2);
            let observed = ratio.log2();
            let expected = f64::from(tableau.order + 1);
            assert!(
                (observed - expected).abs() < 0.15,
                "{}: 1 歩の誤差の次数 {observed}（期待 {expected}）",
                tableau.name
            );
        }
    }

    /// 係数の整合: Σ b = 1、c_i = Σ_j a_ij、a は狭義の下三角。
    #[test]
    fn tableaus_are_consistent() {
        for tableau in [&EULER, &HEUN, &RK4] {
            assert_eq!(tableau.a.len(), tableau.stages());
            assert_eq!(tableau.c.len(), tableau.stages());
            assert!((tableau.b.iter().sum::<f64>() - 1.0).abs() < 1e-15);
            for (i, row) in tableau.a.iter().enumerate() {
                assert_eq!(row.len(), i, "{} の a[{i}]", tableau.name);
                assert!((row.iter().sum::<f64>() - tableau.c[i]).abs() < 1e-15);
            }
        }
        assert_eq!(fixed_step_tableau(SolverId::Rk4), Some(&RK4));
        assert_eq!(fixed_step_tableau(SolverId::Bdf), None);
    }

    /// Euler 法で y' = -y を h = 0.1 で 10 歩進めると 0.9^10。
    #[test]
    fn euler_reproduces_the_geometric_sequence() {
        let mut stepper = ExplicitStepper::new(&EULER, 1);
        let mut y = [1.0];
        for _ in 0..10 {
            let mut out = [0.0];
            stepper.step(0.0, &y, &[-y[0]], 0.1, |_, y, dy| dy[0] = -y[0], &mut out);
            y = out;
        }
        assert!((y[0] - 0.9f64.powi(10)).abs() < 1e-15, "{}", y[0]);
    }

    /// Hermite 補間は両端の値と微分を再現し、3 次の多項式はそのまま再現する。
    #[test]
    fn hermite_reproduces_endpoints_derivatives_and_cubics() {
        let p = |t: f64| 1.0 - 2.0 * t + 0.5 * t * t - 0.25 * t * t * t;
        let dp = |t: f64| -2.0 + t - 0.75 * t * t;
        let (t0, t1) = (0.3, 1.1);
        let at = |t: f64| {
            let mut out = [0.0];
            let (y0, f0, y1, f1) = ([p(t0)], [dp(t0)], [p(t1)], [dp(t1)]);
            let left = Knot {
                t: t0,
                y: &y0,
                f: &f0,
            };
            let right = Knot {
                t: t1,
                y: &y1,
                f: &f1,
            };
            hermite(left, right, t, &mut out);
            out[0]
        };
        assert_eq!(at(t0), p(t0));
        assert_eq!(at(t1), p(t1));
        for i in 0..=20 {
            let t = t0 + (t1 - t0) * f64::from(i) / 20.0;
            assert!((at(t) - p(t)).abs() < 1e-14, "t = {t}");
        }
        // 端の微分（中心差分）
        let e = 1e-6;
        assert!(((at(t0 + e) - at(t0)) / e - dp(t0)).abs() < 1e-5);
        assert!(((at(t1) - at(t1 - e)) / e - dp(t1)).abs() < 1e-5);
    }

    /// 格子は t0 + k·h。格子の上からは次の点へ、イベントで外れた後も同じ格子へ戻る。
    #[test]
    fn grid_returns_to_origin_plus_multiples_of_h() {
        let gap = 1e-12;
        assert_eq!(next_grid_time(0.0, 0.1, 0.0, gap), 0.1);
        assert_eq!(next_grid_time(0.0, 0.1, 0.1, gap), 0.2);
        assert_eq!(next_grid_time(0.0, 0.1, 0.2, gap), 3.0 * 0.1);
        assert_eq!(next_grid_time(0.0, 0.1, 0.45, gap), 5.0 * 0.1);
        assert_eq!(next_grid_time(0.0, 0.1, 9.0 * 0.1, gap), 1.0);
        // 格子の点の丸めのすぐ手前（gap の内側）は次の点へ
        assert_eq!(next_grid_time(0.0, 0.1, 0.3 - 1e-15, gap), 4.0 * 0.1);
        assert_eq!(next_grid_time(1.0, 0.25, 1.6, gap), 1.0 + 3.0 * 0.25);
        for k in 0..1000 {
            let now = f64::from(k) * 0.001;
            let next = next_grid_time(0.0, 0.001, now, gap);
            assert_eq!(next, f64::from(k + 1) * 0.001, "k = {k}");
        }
    }

    /// y' = λy の θ 法の 1 歩は解析的に y₁ = y₀(1 + (1-θ)hλ)/(1 - θhλ)（線形なので Newton 法は 1 回で解く）。
    #[test]
    fn theta_step_on_a_linear_system_is_exact() {
        for method in [&IMPEULER, &TRAPEZOID] {
            let (lambda, h) = (-3.0, 0.2);
            let mut stepper = ThetaStepper::new(method, 2);
            let mut out = [0.0; 2];
            let f = |_: f64, y: &[f64], dy: &mut [f64]| {
                dy[0] = lambda * y[0];
                dy[1] = y[0] - y[1];
            };
            let jv = |_: f64, _: &[f64], v: &[f64], dv: &mut [f64]| {
                dv[0] = lambda * v[0];
                dv[1] = v[0] - v[1];
            };
            let (y0, f0) = ([1.0, 2.0], [lambda, -1.0]);
            let start = Knot {
                t: 0.0,
                y: &y0,
                f: &f0,
            };
            stepper.step(start, h, f, jv, &mut out).unwrap();
            let th = method.theta;
            let x = (1.0 + (1.0 - th) * h * lambda) / (1.0 - th * h * lambda);
            assert!((out[0] - x).abs() < 1e-14, "{}: {}", method.name, out[0]);
            // 第 2 成分 y₂' = y₁ - y₂: (1 + θh) y₂₁ = y₂₀ + h((1-θ)(y₁₀ - y₂₀) + θ y₁₁)
            let y2 = (2.0 + h * ((1.0 - th) * (1.0 - 2.0) + th * x)) / (1.0 + th * h);
            assert!((out[1] - y2).abs() < 1e-14, "{}: {}", method.name, out[1]);
        }
    }

    /// 非線形 y' = -y³（解 1/√(1+2t)）で、h を半分にすると誤差が 2^p 分の 1 になる。
    #[test]
    fn theta_methods_converge_with_their_order_on_a_nonlinear_system() {
        for method in [&IMPEULER, &TRAPEZOID] {
            let error = |steps: usize| {
                let h = 1.0 / steps as f64;
                let mut stepper = ThetaStepper::new(method, 1);
                let (mut y, mut out) = ([1.0_f64], [0.0_f64]);
                for k in 0..steps {
                    let t = k as f64 * h;
                    let f0 = [-y[0].powi(3)];
                    let start = Knot { t, y: &y, f: &f0 };
                    stepper
                        .step(
                            start,
                            h,
                            |_, y, dy| dy[0] = -y[0].powi(3),
                            |_, y, v, dv| dv[0] = -3.0 * y[0] * y[0] * v[0],
                            &mut out,
                        )
                        .unwrap();
                    y = out;
                }
                (y[0] - 3.0_f64.sqrt().recip()).abs()
            };
            let observed = (error(20) / error(40)).log2();
            assert!(
                (observed - f64::from(method.order)).abs() < 0.2,
                "{}: 次数 {observed}",
                method.name
            );
        }
    }

    /// 段の方程式に実数の解が無ければ（y' = y²、y₀ = 1、h = 2 は 1 = y₁ - 1 - ... で解なし）Newton 法の失敗を返す。
    #[test]
    fn newton_failure_is_an_error() {
        let mut stepper = ThetaStepper::new(&IMPEULER, 1);
        let mut out = [0.0];
        let start = Knot {
            t: 0.0,
            y: &[1.0],
            f: &[1.0],
        };
        let result = stepper.step(
            start,
            2.0,
            |_, y, dy| dy[0] = y[0] * y[0],
            |_, y, v, dv| dv[0] = 2.0 * y[0] * v[0],
            &mut out,
        );
        assert!(result.is_err(), "{result:?} {out:?}");
    }
}

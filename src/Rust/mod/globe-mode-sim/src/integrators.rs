//! 固定刻みの積分の係数と、刻みの間の補間（S012）。Rumoca に依存しない純粋な関数で、実行系の積分器
//! （`globe-mode-exec` の `plugins`）と、後で作る自前の driver（FMU と Track B）で使い回す。
//!
//! - [`Tableau`] … 陽的な Runge–Kutta 法の係数（Butcher 表）。[`EULER`]・[`HEUN`]・[`RK4`]。
//! - [`ExplicitStepper`] … 1 歩を進める（段の値の置き場を使い回す）。
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

/// 固定刻みのソルバの係数。固定刻みでなければ None。
pub const fn fixed_step_tableau(id: SolverId) -> Option<&'static Tableau> {
    match id {
        SolverId::Euler => Some(&EULER),
        SolverId::Heun => Some(&HEUN),
        SolverId::Rk4 => Some(&RK4),
        _ => None,
    }
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
}

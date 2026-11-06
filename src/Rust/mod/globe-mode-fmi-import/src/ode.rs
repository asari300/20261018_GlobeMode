//! 依存の無い陽的な Runge–Kutta（S205、ADR-0026 (l)）。`globe-mode-sim`・`globe-mode-exec`（Rumoca を引く）には依存しない
//! ＝Import の利用者に Rumoca を背負わせない。固定刻みの古典 RK4 と、誤差制御つきの Dormand–Prince 5(4)。
//!
//! shortcut: 陰的な積分は無い（硬い系は刻みが小さくなりすぎて [`Error::Fmu`] で止まる）。硬い系は次の WP。

use crate::{Error, Result};

/// 右辺 `f(t, x)`。
pub(crate) type Rhs<'a> = dyn FnMut(f64, &[f64]) -> Result<Vec<f64>> + 'a;

fn axpy(x: &[f64], h: f64, ks: &[(&[f64], f64)]) -> Vec<f64> {
    (0..x.len())
        .map(|i| x[i] + h * ks.iter().map(|(k, c)| c * k[i]).sum::<f64>())
        .collect()
}

/// 古典 RK4 の 1 歩。
pub(crate) fn rk4_step(f: &mut Rhs<'_>, t: f64, x: &[f64], h: f64) -> Result<Vec<f64>> {
    let k1 = f(t, x)?;
    let k2 = f(t + h / 2.0, &axpy(x, h, &[(&k1, 0.5)]))?;
    let k3 = f(t + h / 2.0, &axpy(x, h, &[(&k2, 0.5)]))?;
    let k4 = f(t + h, &axpy(x, h, &[(&k3, 1.0)]))?;
    Ok(axpy(
        x,
        h,
        &[
            (&k1, 1.0 / 6.0),
            (&k2, 1.0 / 3.0),
            (&k3, 1.0 / 3.0),
            (&k4, 1.0 / 6.0),
        ],
    ))
}

/// Dormand–Prince 5(4) の 1 歩。`(5 次の解, 5 次と 4 次の差)`。係数は Dormand・Prince, J. Comput. Appl. Math. 6 (1980) の表。
pub(crate) fn dopri5_step(
    f: &mut Rhs<'_>,
    t: f64,
    x: &[f64],
    h: f64,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let k1 = f(t, x)?;
    let k2 = f(t + h / 5.0, &axpy(x, h, &[(&k1, 1.0 / 5.0)]))?;
    let k3 = f(
        t + 3.0 * h / 10.0,
        &axpy(x, h, &[(&k1, 3.0 / 40.0), (&k2, 9.0 / 40.0)]),
    )?;
    let k4 = f(
        t + 4.0 * h / 5.0,
        &axpy(
            x,
            h,
            &[(&k1, 44.0 / 45.0), (&k2, -56.0 / 15.0), (&k3, 32.0 / 9.0)],
        ),
    )?;
    let k5 = f(
        t + 8.0 * h / 9.0,
        &axpy(
            x,
            h,
            &[
                (&k1, 19372.0 / 6561.0),
                (&k2, -25360.0 / 2187.0),
                (&k3, 64448.0 / 6561.0),
                (&k4, -212.0 / 729.0),
            ],
        ),
    )?;
    let k6 = f(
        t + h,
        &axpy(
            x,
            h,
            &[
                (&k1, 9017.0 / 3168.0),
                (&k2, -355.0 / 33.0),
                (&k3, 46732.0 / 5247.0),
                (&k4, 49.0 / 176.0),
                (&k5, -5103.0 / 18656.0),
            ],
        ),
    )?;
    let b5 = [
        (&k1[..], 35.0 / 384.0),
        (&k3[..], 500.0 / 1113.0),
        (&k4[..], 125.0 / 192.0),
        (&k5[..], -2187.0 / 6784.0),
        (&k6[..], 11.0 / 84.0),
    ];
    let x5 = axpy(x, h, &b5);
    let k7 = f(t + h, &x5)?;
    // 5 次と 4 次の係数の差（FSAL の k7 を含む）
    let e = axpy(
        &vec![0.0; x.len()],
        h,
        &[
            (&k1, 71.0 / 57600.0),
            (&k3, -71.0 / 16695.0),
            (&k4, 71.0 / 1920.0),
            (&k5, -17253.0 / 339200.0),
            (&k6, 22.0 / 525.0),
            (&k7, -1.0 / 40.0),
        ],
    );
    Ok((x5, e))
}

/// 許容誤差（相対と、状態ごとの絶対）。
pub(crate) struct Tol<'a> {
    pub rtol: f64,
    pub atol: &'a [f64],
}

/// 受理した 1 歩。
pub(crate) struct Accepted {
    pub t: f64,
    pub x: Vec<f64>,
    /// 次に試す刻み。
    pub h_next: f64,
}

/// `t_end` を超えない刻みで、誤差が許容内に収まる 1 歩を探す。`t_end` に届く歩は `t` をちょうど `t_end` にする。
pub(crate) fn dopri5_accepted(
    f: &mut Rhs<'_>,
    t: f64,
    x: &[f64],
    h_try: f64,
    t_end: f64,
    tol: &Tol<'_>,
) -> Result<Accepted> {
    let mut h = h_try.min(t_end - t);
    loop {
        let reaches_end = h >= t_end - t;
        if reaches_end {
            h = t_end - t;
        }
        let (x5, e) = dopri5_step(f, t, x, h)?;
        // 重み付きの最大ノルム
        let err = (0..x.len())
            .map(|i| e[i].abs() / (tol.atol[i] + tol.rtol * x[i].abs().max(x5[i].abs())))
            .fold(0.0f64, f64::max);
        if !err.is_finite() {
            return Err(Error::Fmu(format!("t = {t} の積分が有限でない値になった")));
        }
        let factor = (0.9 * err.max(1e-10).powf(-0.2)).clamp(0.2, 5.0);
        if err <= 1.0 {
            return Ok(Accepted {
                t: if reaches_end { t_end } else { t + h },
                x: x5,
                h_next: h * factor,
            });
        }
        h *= factor.min(0.9);
        if h < 16.0 * f64::EPSILON * t.abs().max(1.0) {
            return Err(Error::Fmu(format!(
                "t = {t} で刻みが小さくなりすぎた（硬い系かもしれない。陰的な積分は未対応）"
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// x' = -x、x(0) = 1。解析解 exp(-t)。
    fn decay() -> impl FnMut(f64, &[f64]) -> Result<Vec<f64>> {
        |_, x| Ok(vec![-x[0]])
    }

    fn rk4_error(n: usize) -> f64 {
        let (mut t, mut x) = (0.0, vec![1.0]);
        let h = 1.0 / n as f64;
        for _ in 0..n {
            x = rk4_step(&mut decay(), t, &x, h).unwrap();
            t += h;
        }
        (x[0] - (-1.0f64).exp()).abs()
    }

    #[test]
    fn rk4_is_fourth_order() {
        let ratio = rk4_error(10) / rk4_error(20);
        assert!((ratio - 16.0).abs() < 1.0, "{ratio}");
    }

    #[test]
    fn dopri5_step_is_fifth_order_and_error_estimate_is_fourth() {
        let one = |h: f64| {
            let (x5, e) = dopri5_step(&mut decay(), 0.0, &[1.0], h).unwrap();
            ((x5[0] - (-h).exp()).abs(), e[0].abs())
        };
        let (a1, e1) = one(0.05);
        let (a2, e2) = one(0.025);
        // 局所誤差は h^6、見積もり（4 次の解の誤差）は h^5
        assert!((a1 / a2 - 64.0).abs() < 4.0, "{}", a1 / a2);
        assert!((e1 / e2 - 32.0).abs() < 3.0, "{}", e1 / e2);
    }

    #[test]
    fn dopri5_meets_the_tolerance_and_lands_on_t_end() {
        let tol = Tol {
            rtol: 1e-9,
            atol: &[1e-9],
        };
        let (mut t, mut x, mut h, mut steps) = (0.0, vec![1.0], 0.5, 0);
        while t < 3.0 {
            let a = dopri5_accepted(&mut decay(), t, &x, h, 3.0, &tol).unwrap();
            (t, x, h) = (a.t, a.x, a.h_next);
            steps += 1;
        }
        assert_eq!(t, 3.0);
        assert!((x[0] - (-3.0f64).exp()).abs() < 1e-7, "{}", x[0]);
        assert!(steps < 200, "{steps}");
    }

    #[test]
    fn a_blowing_up_system_fails_instead_of_looping() {
        // x' = x^2 は t = 1 で発散する
        let tol = Tol {
            rtol: 1e-6,
            atol: &[1e-6],
        };
        let mut f = |_: f64, x: &[f64]| Ok(vec![x[0] * x[0]]);
        let (mut t, mut x, mut h) = (0.0, vec![1.0], 0.1);
        let r = loop {
            match dopri5_accepted(&mut f, t, &x, h, 2.0, &tol) {
                Ok(a) => (t, x, h) = (a.t, a.x, a.h_next),
                Err(e) => break e,
            }
        };
        assert!(
            r.to_string().contains("刻み") || r.to_string().contains("有限"),
            "{r}"
        );
    }
}

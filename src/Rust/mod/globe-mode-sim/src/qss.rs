//! 量子化状態法 QSS2（2 次。`qss`。S031）。Rumoca に依存しない純粋な数値部で、実行系（`globe-mode-exec` の
//! `plugins::qss`）が `MeIntegratorBackend` に包む。
//!
//! 出典: E. Kofman, S. Junco, "Quantized-state systems: a DEVS approach for continuous system simulation",
//! Trans. SCS 18(3), 2001 と、F. Cellier, E. Kofman, "Continuous System Simulation", Springer (2006) の QSS の章の
//! 手法の説明（式だけ）。【書籍・論文の本文は未確認】qss-solver・PowerDEVS・OpenModelica のコードは読んでおらず写していない。
//!
//! ## 手法
//!
//! 状態 `x_i` の軌道は 2 次の多項式 `x_i + dx_i τ + ddx_i τ²/2`（τ = t - 基準の時刻）、量子化した状態 `q_i` は
//! 1 次式 `q_i + mq_i τ`。微分方程式 `x' = f(t, x)` の右辺には `x` でなく `q` を入れる:
//! `dx_i = f_i(t, q)`、`ddx_i = d/dt f_i(t, q(t))`。`|x_i(t) - q_i(t)|` が量子 `ΔQ_i` に達した時刻に
//! 状態 `i` を再量子化する（`q_i ← x_i(t)`、`mq_i ← x_i'(t)`）。時刻は刻みでなく事象で決まるので、
//! 精度（`ΔQ`）を上げると事象の数は `ΔQ^(-1/2)` で増える（試験で確かめる）。誤差は `ΔQ` に比例する。
//!
//! - 量子は `ΔQ_i = max(atol_i, rtol·|x_i|)`（`x_i` は再量子化の時点の値）。
//! - 次の再量子化の時刻は `d(τ) = (x_i - q_i) + (dx_i - mq_i) τ + ddx_i τ²/2` が `±ΔQ_i` に達する最小の `τ > 0`
//!   （2 次方程式。`ddx_i ≈ 0` は 1 次式として解く。桁落ちを避ける形で解く）。
//! - `ddx` は `f` を `q` の軌道に沿って時刻 δ だけ進めた点（`t + δ`、`q + mq δ`）で評価して前進差分で取る
//!   （`δ = √ε·max(1, |t|)`、事象 1 回につき `f` を 2 回呼ぶ）。差分の誤差は相対 1e-8 ほどで、量子に比べて無視できる
//!   （事象ごとの誤差が量子の 1e-7 倍）。
//!
//! shortcut: 状態ごとの依存（どの `f_j` がどの `q_i` に依存するか）を持たないので、事象のたびに全状態の
//! 導関数の多項式を `f` 全体で作り直す（本来の QSS は依存する状態だけ更新し、事象 1 回の費用が状態数に依らない）。
//! 状態が多い系で遅くなったら、ヤコビ行列の構造（疎パターン）から依存を取って、その状態だけ更新する。
//! shortcut: 硬い系は苦手（陽的な方法と同じ安定の限界があり、`ΔQ` の幅を行き来して事象が増え続ける）。
//! 失敗は実行系の壁時計の上限で止める。直すなら LIQSS（`f` の `q_i` への依存を陰的に解く線形の陰的 QSS）にする。
//! shortcut: 前進差分の δ は時間の目盛りを知らずに決める（`√ε·max(1, |t|)`）。固有の時間が 1e-6 s より短い系では
//! δ が大きすぎる。そのときは直前の事象の間隔から決める。

// 平行な配列を同じ添字で回す式が多く、イテレータにすると対応が読みにくくなる
#![allow(clippy::needless_range_loop)]

use crate::radau::Tolerances;

/// 状態の軌道 `x + dx τ + ddx τ²/2`（τ = 時刻 - `t`）。
#[derive(Debug, Clone, Default)]
pub struct Poly {
    pub t: f64,
    pub x: Vec<f64>,
    pub dx: Vec<f64>,
    pub ddx: Vec<f64>,
}

impl Poly {
    /// 時刻 `t` の値を `out` に書く。
    pub fn eval(&self, t: f64, out: &mut [f64]) {
        let tau = t - self.t;
        for (i, o) in out.iter_mut().enumerate() {
            *o = self.x[i] + tau * (self.dx[i] + 0.5 * tau * self.ddx[i]);
        }
    }
}

/// `a τ² + b τ + c = 0` の最小の非負の根（無ければ無限大）。桁落ちを避ける形（`q = -(b + sign(b)√D)/2`、
/// 根は `q/a` と `c/q`）。
fn smallest_root(a: f64, b: f64, c: f64) -> f64 {
    let d = b * b - 4.0 * a * c;
    if d < 0.0 {
        return f64::INFINITY;
    }
    let q = -0.5 * (b + b.signum() * d.sqrt());
    if q == 0.0 {
        // b = 0 で D = 0: 二重根 0（a·c = 0 かつ b = 0）。a ≠ 0 なら c も 0
        return if a != 0.0 || c == 0.0 {
            0.0
        } else {
            f64::INFINITY
        };
    }
    [if a != 0.0 { q / a } else { f64::NAN }, c / q]
        .into_iter()
        .filter(|r| *r >= 0.0)
        .fold(f64::INFINITY, f64::min)
}

/// `d(τ) = d0 + d1 τ + d2 τ²` が `±quantum` に達する最小の `τ ≥ 0`。`|d0| ≥ quantum` なら 0。
fn first_crossing(d0: f64, d1: f64, d2: f64, quantum: f64) -> f64 {
    if d0.abs() >= quantum {
        return 0.0;
    }
    smallest_root(d2, d1, d0 - quantum).min(smallest_root(d2, d1, d0 + quantum))
}

/// QSS2 の積分器。作業用の領域と、軌道の多項式を持つ。基準の時刻は全状態で共通（事象のたびに全部を進める）。
#[derive(Debug, Clone)]
pub struct Qss2 {
    poly: Poly,
    /// 量子化した状態 `q_i(poly.t)` とその傾き、量子 `ΔQ_i`
    q: Vec<f64>,
    mq: Vec<f64>,
    quantum: Vec<f64>,
    shifted: Vec<f64>,
    shifted_f: Vec<f64>,
    /// 再量子化の数
    pub events: usize,
}

impl Qss2 {
    pub fn new(width: usize) -> Self {
        let zeros = vec![0.0; width];
        Self {
            poly: Poly {
                t: 0.0,
                x: zeros.clone(),
                dx: zeros.clone(),
                ddx: zeros.clone(),
            },
            q: zeros.clone(),
            mq: zeros.clone(),
            quantum: zeros.clone(),
            shifted: zeros.clone(),
            shifted_f: zeros,
            events: 0,
        }
    }

    /// 今の軌道（基準の時刻での値と、そこからの多項式）。
    pub fn poly(&self) -> &Poly {
        &self.poly
    }

    /// 時刻 `t`・値 `x` から始める（全状態を量子化し直す）。
    pub fn start(
        &mut self,
        t: f64,
        x: &[f64],
        tol: Tolerances<'_>,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
    ) -> Result<(), String> {
        self.poly.t = t;
        self.poly.x.copy_from_slice(x);
        self.q.copy_from_slice(x);
        for i in 0..x.len() {
            self.quantum[i] = tol.atol[i].max(tol.rtol * x[i].abs());
        }
        f(t, &self.q, &mut self.poly.dx);
        self.mq.copy_from_slice(&self.poly.dx);
        self.second_derivatives(&mut f)
    }

    /// `dx` が決まっているとして、`ddx = d/dt f(t, q(t))` を前進差分で作る。
    fn second_derivatives(
        &mut self,
        f: &mut impl FnMut(f64, &[f64], &mut [f64]),
    ) -> Result<(), String> {
        let t = self.poly.t;
        let delta = f64::EPSILON.sqrt() * t.abs().max(1.0);
        for i in 0..self.q.len() {
            self.shifted[i] = self.q[i] + self.mq[i] * delta;
        }
        f(t + delta, &self.shifted, &mut self.shifted_f);
        for i in 0..self.q.len() {
            self.poly.ddx[i] = (self.shifted_f[i] - self.poly.dx[i]) / delta;
        }
        if self
            .poly
            .dx
            .iter()
            .chain(&self.poly.ddx)
            .all(|v| v.is_finite())
        {
            Ok(())
        } else {
            Err(format!("t = {t} で導関数が有限でない"))
        }
    }

    /// 基準の時刻を `tau` だけ進める（`x`・`dx` を多項式で進め、`q` は直線で進める）。
    fn shift(&mut self, tau: f64) {
        for i in 0..self.q.len() {
            let (x, dx, ddx) = (self.poly.x[i], self.poly.dx[i], self.poly.ddx[i]);
            self.poly.x[i] = x + tau * (dx + 0.5 * tau * ddx);
            self.poly.dx[i] = dx + tau * ddx;
            self.q[i] += tau * self.mq[i];
        }
        self.poly.t += tau;
    }

    /// 次の再量子化までの時間と、その状態。
    fn next_event(&self) -> (f64, usize) {
        let mut best = (f64::INFINITY, 0);
        for i in 0..self.q.len() {
            let d0 = self.poly.x[i] - self.q[i];
            let d1 = self.poly.dx[i] - self.mq[i];
            let tau = first_crossing(d0, d1, 0.5 * self.poly.ddx[i], self.quantum[i]);
            if tau < best.0 {
                best = (tau, i);
            }
        }
        best
    }

    /// 時刻 `t_limit` を超えない範囲で、次の再量子化の時刻まで進める（それが `t_limit` より先なら `t_limit` まで進めて
    /// 再量子化しない）。進んだ先の時刻を返す。値は [`Qss2::poly`] の `x`。
    pub fn advance(
        &mut self,
        t_limit: f64,
        tol: Tolerances<'_>,
        mut f: impl FnMut(f64, &[f64], &mut [f64]),
    ) -> Result<f64, String> {
        let t0 = self.poly.t;
        let remaining = t_limit - t0;
        if !(remaining > 0.0 && remaining.is_finite()) {
            return Err(format!("上限 {t_limit} が始点 {t0} より先にない"));
        }
        let (tau, k) = self.next_event();
        // 同時の事象（量子の境界にちょうど着いた状態）は 1 つずつ処理する。時刻が進まない歩は作らない
        let tau = tau
            .max(4.0 * f64::EPSILON * t0.abs())
            .max(f64::MIN_POSITIVE);
        if tau >= remaining {
            self.shift(remaining);
            self.poly.t = t_limit;
            return Ok(t_limit);
        }
        self.shift(tau);
        // 状態 k を再量子化: 値は x、傾きは x の導関数（多項式の今の値）
        let t = self.poly.t;
        self.q[k] = self.poly.x[k];
        self.mq[k] = self.poly.dx[k];
        self.quantum[k] = tol.atol[k].max(tol.rtol * self.q[k].abs());
        self.events += 1;
        f(t, &self.q, &mut self.poly.dx);
        self.second_derivatives(&mut f)?;
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 制御つきで t_end まで回す（`t_limit` を t_end にして、事象ごとに進める）。(終点, 事象の数, f の呼び出し回数)
    fn integrate(
        x0: Vec<f64>,
        t_end: f64,
        rtol: f64,
        atol: f64,
        f: impl Fn(f64, &[f64], &mut [f64]),
    ) -> (Vec<f64>, usize, usize) {
        let atols = vec![atol; x0.len()];
        let tol = Tolerances { rtol, atol: &atols };
        let calls = std::cell::Cell::new(0_usize);
        let counted = |t: f64, y: &[f64], dy: &mut [f64]| {
            calls.set(calls.get() + 1);
            f(t, y, dy);
        };
        let mut qss = Qss2::new(x0.len());
        qss.start(0.0, &x0, tol, counted).unwrap();
        while qss.poly().t < t_end {
            qss.advance(t_end, tol, counted).unwrap();
        }
        assert_eq!(qss.poly().t, t_end);
        (qss.poly().x.clone(), qss.events, calls.get())
    }

    fn decay(_: f64, x: &[f64], dx: &mut [f64]) {
        dx[0] = -x[0];
    }

    fn oscillator(_: f64, x: &[f64], dx: &mut [f64]) {
        dx[0] = x[1];
        dx[1] = -x[0];
    }

    #[test]
    fn crossing_time_solves_the_quadratic() {
        // d(τ) = τ² - 1 が ±4 に達するのは τ = √5（+4 側。-4 側は根が無い）
        assert!((first_crossing(-1.0, 0.0, 1.0, 4.0) - 5.0_f64.sqrt()).abs() < 1e-14);
        // 2 次の項が無い: d = 2τ が 1 に達する
        assert_eq!(first_crossing(0.0, 2.0, 0.0, 1.0), 0.5);
        // 下向き: d = -τ
        assert_eq!(first_crossing(0.0, -1.0, 0.0, 3.0), 3.0);
        // 動かない
        assert_eq!(first_crossing(0.0, 0.0, 0.0, 1.0), f64::INFINITY);
        // 境界にいるなら 0
        assert_eq!(first_crossing(1.0, 5.0, 0.0, 1.0), 0.0);
        // a が極小でも桁落ちしない（根は 1e-3 / 1 の近く）
        let tau = first_crossing(0.0, 1.0, 1e-20, 1e-3);
        assert!((tau - 1e-3).abs() < 1e-15, "{tau}");
    }

    /// x' = -x、x(0) = 1 の x(1) が解析解に合う。許容誤差の数倍に収まる。
    #[test]
    fn decay_matches_the_analytic_solution() {
        for tol in [1e-4, 1e-6, 1e-8] {
            let (x, events, calls) = integrate(vec![1.0], 1.0, tol, tol, decay);
            let error = (x[0] - (-1.0_f64).exp()).abs();
            eprintln!("減衰 tol {tol:e}: 事象 {events}、f {calls} 回、誤差 {error:.3e}");
            assert!(error < 3.0 * tol, "tol {tol}: 誤差 {error}");
            assert_eq!(calls, 2 * (events + 1));
        }
    }

    /// 調和振動子の x(2)・v(2) が解析解に合う。
    #[test]
    fn oscillator_matches_the_analytic_solution() {
        let tol = 1e-6;
        let (x, events, _) = integrate(vec![1.0, 0.0], 2.0, tol, tol, oscillator);
        let (ex, ev) = (2.0_f64.cos(), -2.0_f64.sin());
        eprintln!(
            "振動子 tol {tol:e}: 事象 {events}、誤差 {:.3e} {:.3e}",
            (x[0] - ex).abs(),
            (x[1] - ev).abs()
        );
        assert!((x[0] - ex).abs() < 10.0 * tol && (x[1] - ev).abs() < 10.0 * tol);
    }

    /// 誤差は量子に比例して減り（半分にすればほぼ半分以下）、事象の数は ΔQ^(-1/2) で増える。
    #[test]
    fn error_follows_the_quantum_and_events_scale_as_its_inverse_square_root() {
        let run = |tol: f64| {
            let (x, events, _) = integrate(vec![1.0, 0.0], 3.0, tol, tol, oscillator);
            let error = (x[0] - 3.0_f64.cos()).hypot(x[1] + 3.0_f64.sin());
            (error, events as f64)
        };
        let (e1, n1) = run(1e-5);
        let (e2, n2) = run(1e-7);
        let (e3, n3) = run(1e-9);
        // 100 倍細かくしたときの傾き（理想は誤差 1、事象 -1/2）
        let error_exponent = (e1 / e3).log10() / 4.0;
        let event_exponent = (n3 / n1).log10() / 4.0;
        eprintln!(
            "傾き: 誤差 ΔQ^{error_exponent:.2}、事象 ΔQ^-{event_exponent:.2}（誤差 {e1:.2e} {e2:.2e} {e3:.2e}、事象 {n1} {n2} {n3}）"
        );
        assert!(e2 < e1 / 2.0 && e3 < e2 / 2.0);
        assert!(
            (0.8..1.3).contains(&error_exponent),
            "誤差の傾き {error_exponent}"
        );
        assert!(
            (0.4..0.6).contains(&event_exponent),
            "事象の傾き {event_exponent}"
        );
    }

    /// 右辺が q に依らない x' = a + b t は、2 次の軌道なので量子化の誤差が無い（差分の誤差だけ）。
    #[test]
    fn quadratic_trajectory_is_reproduced() {
        let (a, b) = (1.5, -0.75);
        let (x, events, _) = integrate(vec![2.0], 4.0, 1e-6, 1e-6, |t, _, dx| dx[0] = a + b * t);
        let exact = 2.0 + a * 4.0 + b * 8.0;
        eprintln!(
            "2 次の軌道: 事象 {events}、誤差 {:.3e}",
            (x[0] - exact).abs()
        );
        assert!((x[0] - exact).abs() < 1e-7 * exact.abs());
    }

    /// 上限で止めて、同じ点から続けても（`start` しなおさなくても）軌道は同じ。
    #[test]
    fn stopping_at_a_limit_does_not_change_the_trajectory() {
        let tol = Tolerances {
            rtol: 1e-6,
            atol: &[1e-6, 1e-6],
        };
        let mut whole = Qss2::new(2);
        whole.start(0.0, &[1.0, 0.0], tol, oscillator).unwrap();
        while whole.poly().t < 2.0 {
            whole.advance(2.0, tol, oscillator).unwrap();
        }
        let mut split = Qss2::new(2);
        split.start(0.0, &[1.0, 0.0], tol, oscillator).unwrap();
        for limit in [0.5, 1.0, 2.0] {
            while split.poly().t < limit {
                split.advance(limit, tol, oscillator).unwrap();
            }
        }
        for i in 0..2 {
            assert!((whole.poly().x[i] - split.poly().x[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn non_finite_derivatives_are_an_error() {
        let tol = Tolerances {
            rtol: 1e-6,
            atol: &[1e-6],
        };
        let mut qss = Qss2::new(1);
        qss.start(0.0, &[1.0], tol, decay).unwrap();
        let bad = |_: f64, _: &[f64], dx: &mut [f64]| dx[0] = f64::NAN;
        assert!(qss.advance(10.0, tol, bad).is_err());
        assert!(qss.start(0.0, &[1.0], tol, bad).is_err());
    }
}

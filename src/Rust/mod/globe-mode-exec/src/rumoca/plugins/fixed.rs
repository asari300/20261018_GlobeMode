//! 固定刻みの積分器（`euler`・`heun`・`rk4`、陰的な `impeuler`・`trapezoid`）。係数・1 歩・補間・格子は
//! `globe_mode_sim::integrators`。陰的なものは 1 歩ごとに Newton 法で解き、解けなければ失敗にする。
//!
//! - 格子は t0 + k·h（t0 は開始時刻）。1 歩は次の格子の点まで進む。ホストの上限（終了時刻・時間イベント・
//!   遅延の上限。`latest_accepted_time`）が手前にあればそこで止まり、次の歩で格子に戻る。状態イベントで
//!   ホストが区間を切り詰めた後（`truncate_reset`・`initialize`）も同じ格子に戻る。
//! - 刻みの間は 3 次の Hermite 補間（両端の値と微分）。`sample` の間は導関数を呼べないので、終点の導関数は
//!   `advance` の中で計算して取っておき、次の歩の始点がその終点と同じなら最初の段に使い回す（FSAL と同じ）。
//! - 刻み幅は変えない。導関数が試しの点を断った（discard）ときは、刻みを縮めずに失敗にする。

use globe_mode_sim::integrators::{
    ExplicitStepper, Knot, Tableau, ThetaMethod, ThetaStepper, hermite, next_grid_time,
};
use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeIntegrationError,
    MeIntegratorBackend, MeNumericalFailure, MeStepCandidate, accepted_interval_contains,
    accepted_step_roundoff,
};

/// 固定刻みの方法。
#[derive(Clone, Copy)]
pub(super) enum Method {
    Explicit(&'static Tableau),
    Theta(&'static ThetaMethod),
}

enum Stepper {
    Explicit(ExplicitStepper),
    Theta(ThetaStepper),
}

/// 固定刻みの積分器。
pub(super) struct FixedStep {
    method: Method,
    /// 刻み幅
    h: f64,
    /// 格子の原点（開始時刻）
    origin: f64,
    derivatives: Option<MeDerivativeHandle>,
    stepper: Option<Stepper>,
    /// 直近に受理した区間の始点（時刻、値、微分）
    start: Segment,
    /// 直近に受理した区間の終点。`valid` が false なら区間は無い
    end: Segment,
    valid: bool,
    /// 終点の導関数を次の歩の始点に使い回してよいか（区間の後に再初期化も切り詰めも無い）
    reusable: bool,
}

#[derive(Debug, Default)]
pub(super) struct Segment {
    pub(super) t: f64,
    pub(super) y: Vec<f64>,
    pub(super) f: Vec<f64>,
}

impl Segment {
    pub(super) fn knot(&self) -> Knot<'_> {
        Knot {
            t: self.t,
            y: &self.y,
            f: &self.f,
        }
    }
}

impl FixedStep {
    pub(super) fn new(method: Method, h: f64, origin: f64) -> Self {
        Self {
            method,
            h,
            origin,
            derivatives: None,
            stepper: None,
            start: Segment::default(),
            end: Segment::default(),
            valid: false,
            reusable: false,
        }
    }

    fn method(&self) -> &'static str {
        match self.method {
            Method::Explicit(tableau) => tableau.name,
            Method::Theta(theta) => theta.name,
        }
    }

    /// 大域誤差の次数
    fn order(&self) -> u32 {
        match self.method {
            Method::Explicit(tableau) => tableau.order,
            Method::Theta(theta) => theta.order,
        }
    }

    fn fail(&self, category: MeNumericalFailure, detail: impl Into<String>) -> MeIntegrationError {
        MeIntegrationError::numerical(self.method(), category, detail)
    }

    /// 導関数が断られていたら DerivativeRefused、試しの点を断られていたら（刻みを縮められないので）失敗。
    fn check_handle(&self, derivatives: &MeDerivativeHandle) -> Result<(), MeIntegrationError> {
        if derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        if derivatives.take_discard() {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!(
                    "the component discarded a trial point and a fixed step of {} cannot shrink",
                    self.h
                ),
            ));
        }
        Ok(())
    }

    fn restart(&mut self, width: usize) {
        self.valid = false;
        self.reusable = false;
        if self.stepper.is_none() {
            self.stepper = Some(match self.method {
                Method::Explicit(tableau) => {
                    Stepper::Explicit(ExplicitStepper::new(tableau, width))
                }
                Method::Theta(theta) => Stepper::Theta(ThetaStepper::new(theta, width)),
            });
        }
    }
}

impl MeIntegratorBackend for FixedStep {
    fn initialize(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: MeDerivativeHandle,
    ) -> Result<(), MeIntegrationError> {
        if !(self.h.is_finite() && self.h > 0.0) {
            return Err(self.fail(
                MeNumericalFailure::Construction,
                format!("the fixed step {} must be finite and positive", self.h),
            ));
        }
        self.derivatives = Some(derivatives);
        self.restart(point.width());
        Ok(())
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        let derivatives = self.derivatives.as_ref().ok_or_else(|| {
            self.fail(
                MeNumericalFailure::Construction,
                "the host has not initialized the fixed-step integrator",
            )
        })?;
        let current = request.current();
        let now = current.time();
        let latest = request.latest_accepted_time();
        // 次の格子の点。ホストの丸めの幅より近い点は飛ばす（進んだと見なされないため）
        let gap = 2.0 * accepted_step_roundoff(now, self.h);
        let grid = next_grid_time(self.origin, self.h, now, gap);
        let end_time = if grid > latest { latest } else { grid };
        let dt = end_time - now;
        // 格子の上から次の格子の点までは h そのもので進める（引き算の丸めを刻みに入れない）
        let dt = if (dt - self.h).abs() <= gap {
            self.h
        } else {
            dt
        };

        // 始点の導関数。前の歩の終点と同じ点なら使い回す
        let reuse = self.reusable
            && self.valid
            && self.end.t.to_bits() == now.to_bits()
            && self.end.y.as_slice() == current.states();
        let mut start = std::mem::take(&mut self.start);
        if reuse {
            std::mem::swap(&mut start, &mut self.end);
        } else {
            start.t = now;
            start.y.clear();
            start.y.extend_from_slice(current.states());
            start.f.resize(current.width(), 0.0);
            derivatives.derivatives_into(now, current.states(), &mut start.f);
        }
        self.valid = false;
        self.reusable = false;

        let mut end = std::mem::take(&mut self.end);
        end.t = end_time;
        end.y.resize(current.width(), 0.0);
        end.f.resize(current.width(), 0.0);
        let method = self.method();
        let stepper = self.stepper.as_mut().ok_or_else(|| {
            MeIntegrationError::numerical(
                method,
                MeNumericalFailure::Construction,
                "the host has not initialized the fixed-step integrator",
            )
        })?;
        let f = |t: f64, y: &[f64], dy: &mut [f64]| derivatives.derivatives_into(t, y, dy);
        let stepped = match stepper {
            Stepper::Explicit(stepper) => {
                stepper.step(now, &start.y, &start.f, dt, f, &mut end.y);
                Ok(())
            }
            // Newton 法のヤコビ行列は部品の方向微分（単位ベクトルごと）から作る
            Stepper::Theta(stepper) => stepper.step(
                start.knot(),
                dt,
                f,
                |t, y, v, dv| derivatives.directional_derivative_into(t, y, v, dv),
                &mut end.y,
            ),
        };
        // 補間と次の歩のための終点の導関数
        derivatives.derivatives_into(end_time, &end.y, &mut end.f);
        self.start = start;
        self.end = end;
        self.check_handle(derivatives)?;
        // 導関数の拒否が先（NaN で Newton 法も失敗するため）。Newton 法の失敗は黙って進まず失敗にする
        stepped.map_err(|detail| self.fail(MeNumericalFailure::AdvanceExhausted, detail))?;
        if !self.end.y.iter().chain(&self.end.f).all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("a fixed step from t={now} to t={end_time} reached a non-finite state"),
            ));
        }
        self.valid = true;
        self.reusable = true;
        Ok(MeStepCandidate::new(
            end_time,
            self.end.y.clone(),
            // 刻みの間の補間は 3 次の Hermite（局所 3 次）。方法の次数がそれより低ければそちら
            self.order().min(3),
        ))
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        if !self.valid {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "no accepted fixed step is available",
            ));
        }
        if states.len() != self.end.y.len() {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "the sampler output width differs from the initialized component width",
            ));
        }
        if !accepted_interval_contains(self.start.t, self.end.t, time) {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                format!(
                    "sample time {time} lies outside [{}, {}]",
                    self.start.t, self.end.t
                ),
            ));
        }
        hermite(self.start.knot(), self.end.knot(), time, states);
        Ok(())
    }

    fn truncate_reset(&mut self, point: &MeContinuousPoint) -> Result<(), MeIntegrationError> {
        self.restart(point.width());
        Ok(())
    }
}

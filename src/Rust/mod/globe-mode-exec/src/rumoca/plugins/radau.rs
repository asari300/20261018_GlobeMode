//! `radau5`: Radau IIA 3 段 5 次の積分器（S030）。係数・1 歩（簡略 Newton 法）・誤差の見積もり・刻み幅の制御は
//! `globe_mode_sim::radau`（Rumoca に依存しない）。ここはそれを `MeIntegratorBackend` に包むだけ。
//!
//! - 1 回の `advance` で 1 歩を受理するまで進める。ホストの上限（終了時刻・時間イベント・遅延の上限。
//!   `latest_accepted_time`）を超えない。刻みの候補は歩をまたいで持ち、イベントの後（`truncate_reset`）も保つ。
//! - 刻みの間は 3 次の Hermite 補間（両端の値と微分。`fixed` と同じ）。終点の導関数は `advance` の中で
//!   計算して取っておき、次の歩の始点がその終点と同じなら使い回す。
//! - 導関数がホストに断られた（discard）試行は棄てて刻みを縮める（`veto`）。断られたままなら失敗。
//! - ヤコビ行列は部品の方向微分（単位ベクトルごと）から密に作る（[`ThetaStepper`] と同じ道）。
//!
//! [`ThetaStepper`]: globe_mode_sim::integrators::ThetaStepper

use globe_mode_sim::{
    integrators::hermite,
    radau::{Radau, Tolerances},
};
use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeIntegrationError,
    MeIntegratorBackend, MeNumericalFailure, MeNumericalSetup, MeStepCandidate,
    accepted_interval_contains,
};

use super::{fixed::Segment, rk::scaled_absolute_tolerances};

const NAME: &str = "radau5";

pub(super) struct Radau5 {
    rtol: f64,
    atol: Vec<f64>,
    /// 最初の刻みの候補
    h0: f64,
    derivatives: Option<MeDerivativeHandle>,
    radau: Option<Radau>,
    /// 直近に受理した区間の始点・終点（時刻、値、微分）。`valid` が false なら区間は無い
    start: Segment,
    end: Segment,
    valid: bool,
    /// 終点の導関数を次の歩の始点に使い回してよいか（区間の後に再初期化も切り詰めも無い）
    reusable: bool,
}

impl Radau5 {
    pub(super) fn new(setup: MeNumericalSetup) -> Self {
        Self {
            rtol: setup.relative_tolerance(),
            atol: scaled_absolute_tolerances(&setup),
            h0: setup.initial_step_hint().unwrap_or(1.0e-3),
            derivatives: None,
            radau: None,
            start: Segment::default(),
            end: Segment::default(),
            valid: false,
            reusable: false,
        }
    }

    fn fail(&self, category: MeNumericalFailure, detail: impl Into<String>) -> MeIntegrationError {
        MeIntegrationError::numerical(NAME, category, detail)
    }
}

impl MeIntegratorBackend for Radau5 {
    fn initialize(
        &mut self,
        point: &MeContinuousPoint,
        derivatives: MeDerivativeHandle,
    ) -> Result<(), MeIntegrationError> {
        if point.width() != self.atol.len() || derivatives.state_count() != self.atol.len() {
            return Err(self.fail(
                MeNumericalFailure::Construction,
                format!(
                    "numerical setup has width {}, point has width {}, and derivative source has width {}",
                    self.atol.len(),
                    point.width(),
                    derivatives.state_count()
                ),
            ));
        }
        self.derivatives = Some(derivatives);
        self.radau = Some(Radau::new(point.width(), self.h0));
        self.valid = false;
        self.reusable = false;
        Ok(())
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        let (Some(derivatives), Some(_)) = (self.derivatives.as_ref(), self.radau.as_ref()) else {
            return Err(self.fail(
                MeNumericalFailure::Construction,
                "the host has not initialized the Radau integrator",
            ));
        };
        let current = request.current();
        let now = current.time();
        let width = current.width();

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
            start.f.resize(width, 0.0);
            derivatives.derivatives_into(now, current.states(), &mut start.f);
        }
        self.valid = false;
        self.reusable = false;

        let mut end = std::mem::take(&mut self.end);
        end.y.resize(width, 0.0);
        end.f.resize(width, 0.0);
        let radau = self.radau.as_mut().expect("initialize で作った");
        let tol = Tolerances {
            rtol: self.rtol,
            atol: &self.atol,
        };
        let stepped = radau.advance(
            start.knot(),
            request.latest_accepted_time(),
            tol,
            |t, y, dy| derivatives.derivatives_into(t, y, dy),
            |t, y, v, dv| derivatives.directional_derivative_into(t, y, v, dv),
            || derivatives.take_discard(),
            &mut end.y,
            &mut end.f,
        );
        self.start = start;
        self.end = end;
        // 導関数の拒否が先（NaN で Newton 法も失敗するため）
        if derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        let end_time =
            stepped.map_err(|detail| self.fail(MeNumericalFailure::AdvanceExhausted, detail))?;
        self.end.t = end_time;
        if !self.end.y.iter().chain(&self.end.f).all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("a Radau step from t={now} to t={end_time} reached a non-finite state"),
            ));
        }
        self.valid = true;
        self.reusable = true;
        // 刻みの間の補間は 3 次の Hermite（局所 3 次）
        Ok(MeStepCandidate::new(end_time, self.end.y.clone(), 3))
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        if !self.valid {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "no accepted Radau step is available",
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
        if point.width() != self.atol.len() {
            return Err(self.fail(
                MeNumericalFailure::Reset,
                "the reset point has a different width than the initialized component",
            ));
        }
        self.valid = false;
        self.reusable = false;
        Ok(())
    }
}

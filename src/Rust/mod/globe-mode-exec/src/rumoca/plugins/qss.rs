//! `qss`: 量子化状態法 QSS2 の積分器（S031）。事象の探索・状態の更新は `globe_mode_sim::qss`（Rumoca に依存しない）。
//! ここはそれを `MeIntegratorBackend` に包むだけ。
//!
//! - 1 回の `advance` で再量子化の事象を 1 つ進める（刻みでなく事象の時刻が歩の幅）。ホストの上限（終了時刻・時間イベント・
//!   遅延の上限。`latest_accepted_time`）を超えない。事象が上限より先なら上限まで進めて再量子化しない。
//! - 刻みの間は 2 次の多項式そのもの（`x + dx τ + ddx τ²/2`）で、補間に誤差が無い。多項式は歩の始めの分を取っておく
//!   （`Poly`）。歩の終わりの点が次の `advance` の始点と同じなら量子化の状態を続け、違えば（イベント・リセットの後）
//!   その点から量子化し直す。
//! - 導関数がホストに断られた（discard）試行は、刻みを縮められないので失敗にする（`fixed` と同じ）。
//!
//! 壊れ方: 硬い系では事象の間隔が極端に小さくなって進まない。歩の数の上限は無く、ホストの壁時計の上限
//! （`max_wall_seconds`、時間切れは `GM-E11005`）で止まる。

use globe_mode_sim::{
    qss::{Poly, Qss2},
    radau::Tolerances,
};
use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeIntegrationError,
    MeIntegratorBackend, MeNumericalFailure, MeNumericalSetup, MeStepCandidate,
    accepted_interval_contains,
};

use super::rk::scaled_absolute_tolerances;

const NAME: &str = "qss";

pub(super) struct Qss {
    rtol: f64,
    atol: Vec<f64>,
    derivatives: Option<MeDerivativeHandle>,
    qss: Option<Qss2>,
    /// 直近に受理した歩の始めの多項式と終点の時刻。`valid` が false なら区間は無い
    start: Poly,
    end_time: f64,
    valid: bool,
    /// 量子化の状態が今の点（歩の終点）のものか（初期化も切り詰めも無い）
    running: bool,
}

impl Qss {
    pub(super) fn new(setup: MeNumericalSetup) -> Self {
        Self {
            rtol: setup.relative_tolerance(),
            atol: scaled_absolute_tolerances(&setup),
            derivatives: None,
            qss: None,
            start: Poly::default(),
            end_time: 0.0,
            valid: false,
            running: false,
        }
    }

    fn fail(&self, category: MeNumericalFailure, detail: impl Into<String>) -> MeIntegrationError {
        MeIntegrationError::numerical(NAME, category, detail)
    }
}

impl MeIntegratorBackend for Qss {
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
        self.qss = Some(Qss2::new(point.width()));
        self.valid = false;
        self.running = false;
        Ok(())
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        let (Some(derivatives), Some(qss)) = (self.derivatives.as_ref(), self.qss.as_mut()) else {
            return Err(self.fail(
                MeNumericalFailure::Construction,
                "the host has not initialized the QSS integrator",
            ));
        };
        let current = request.current();
        let now = current.time();
        let tol = Tolerances {
            rtol: self.rtol,
            atol: &self.atol,
        };
        let mut f = |t: f64, y: &[f64], dy: &mut [f64]| derivatives.derivatives_into(t, y, dy);

        // 続きかどうか: 前の歩の終点と同じ点なら量子化の状態を使う。違えば（イベント・リセットの後）量子化し直す
        let continues = self.running
            && qss.poly().t.to_bits() == now.to_bits()
            && qss.poly().x.as_slice() == current.states();
        self.valid = false;
        self.running = false;
        let result = if continues {
            Ok(())
        } else {
            qss.start(now, current.states(), tol, &mut f)
        }
        .and_then(|()| {
            self.start.clone_from(qss.poly());
            qss.advance(request.latest_accepted_time(), tol, &mut f)
        });
        let end = qss.poly().x.clone();
        // 導関数の拒否が先（NaN で他の失敗も起きるため）
        if derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        if derivatives.take_discard() {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("the component discarded a trial point near t={now} and QSS cannot shrink a step"),
            ));
        }
        let end_time =
            result.map_err(|detail| self.fail(MeNumericalFailure::AdvanceExhausted, detail))?;
        if !end.iter().all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("a QSS step from t={now} to t={end_time} reached a non-finite state"),
            ));
        }
        self.end_time = end_time;
        self.valid = true;
        self.running = true;
        // 刻みの間は 2 次の多項式そのもの
        Ok(MeStepCandidate::new(end_time, end, 2))
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        if !self.valid {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "no accepted QSS step is available",
            ));
        }
        if states.len() != self.start.x.len() {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "the sampler output width differs from the initialized component width",
            ));
        }
        if !accepted_interval_contains(self.start.t, self.end_time, time) {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                format!(
                    "sample time {time} lies outside [{}, {}]",
                    self.start.t, self.end_time
                ),
            ));
        }
        self.start.eval(time, states);
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
        self.running = false;
        Ok(())
    }
}

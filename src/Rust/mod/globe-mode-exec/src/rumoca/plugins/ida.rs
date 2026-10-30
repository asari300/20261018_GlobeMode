//! `ida`: SUNDIALS の IDA（可変係数の BDF、可変次数 1〜5・可変刻み。DAE 用）を `MeIntegratorBackend` に包む（S034）。
//! feature `sundials` のときだけ組み込む。`cvode.rs` と同じ作りで、SUNDIALS の呼び出しもこの 2 ファイルだけ（unsafe）。
//!
//! - ホストが渡すのは ODE x' = f(t, x)。IDA には陰的な残差 F(t, y, y') = y' − f(t, y) として渡す（全成分が微分変数）。
//!   無矛盾な初期値の計算（`IDACalcIC`）はしない。y'0 = f(t0, y0) を導関数 1 回で直接求めて渡す。
//! - 1 回の `advance` で IDA の内部の 1 歩（`IDA_ONE_STEP`）を進める。ホストの上限（`latest_accepted_time`）は
//!   `IDASetStopTime` で渡し、超えない。
//! - 刻みの間は IDA の密な出力（`IDAGetDky`、区間は [tn − hu, tn]）。次数は歩の次数 q で、
//!   `MeStepCandidate` の order にはその q を入れる。
//! - 前の歩の終点と同じ点なら IDA の履歴を続け、違えば（イベント・リセットの後）`IDAReInit` で
//!   その点から（y0, y'0 = f(t0, y0)）始め直す。
//! - 導関数がホストに断られた（discard）試行は、残差の関数が正の値を返して IDA に刻みを縮めさせる。
//!   ホストの失敗（`has_failed`）は負の値を返して止め、`DerivativeRefused` にする。
//!
//! shortcut: 線形ソルバは密な行列だけ（`SUNLinSol_Dense`）でヤコビ行列は IDA の差分商。部品の方向微分や疎な
//! 構造は使わない（状態数が大きいと遅い）。根探索（`IDARootInit`）も使わず、事象はホストが受け持つ。
//! ホストは ODE（x' = f）しか渡さないので、IDA の本領の陰的な DAE（代数変数・`IDACalcIC`）は使わない。
//! 疎な線形ソルバ（KLU は LGPL なので入れない）や、陰的な残差をホストが渡せるようになったときに見直す。

// FFI のために unsafe を許す（Cargo.toml の [lints.rust]。ほかのファイルでは deny のまま）
#![allow(unsafe_code)]

use std::{
    ffi::{c_int, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
};

use rumoca_solver::fmi_me::{
    MeAdvanceRequest, MeContinuousPoint, MeDerivativeHandle, MeIntegrationError,
    MeIntegratorBackend, MeNumericalFailure, MeNumericalSetup, MeStepCandidate,
    accepted_interval_contains,
};
use sundials_sys::{
    IDA_CONV_FAIL, IDA_ERR_FAIL, IDA_FIRST_RES_FAIL, IDA_LINIT_FAIL, IDA_LSETUP_FAIL,
    IDA_LSOLVE_FAIL, IDA_ONE_STEP, IDA_REP_RES_ERR, IDA_RES_FAIL, IDA_SUCCESS, IDA_TOO_MUCH_ACC,
    IDA_TOO_MUCH_WORK, IDA_TSTOP_RETURN, IDACreate, IDAFree, IDAGetCurrentTime, IDAGetDky,
    IDAGetLastOrder, IDAGetLastStep, IDAInit, IDALS_SUCCESS, IDAReInit, IDASVtolerances,
    IDASetLinearSolver, IDASetStopTime, IDASetUserData, IDASolve, N_VDestroy, N_VGetArrayPointer,
    N_VNew_Serial, N_Vector, SUNContext, SUNContext_ClearErrHandlers, SUNContext_Create,
    SUNContext_Free, SUNDenseMatrix, SUNLinSol_Dense, SUNLinSolFree, SUNLinearSolver,
    SUNMatDestroy, SUNMatrix, comm_no_mpi,
};

use super::rk::scaled_absolute_tolerances;

const NAME: &str = "ida";

/// 残差の関数に渡す user_data。`Box` に入れて住所を固定する（IDA が覚えている）
struct User {
    derivatives: MeDerivativeHandle,
    n: usize,
}

/// SUNDIALS の持ち物をまとめたもの。`Drop` で全部解放する
struct Raw {
    ctx: SUNContext,
    mem: *mut c_void,
    y: N_Vector,
    yp: N_Vector,
    abstol: N_Vector,
    matrix: SUNMatrix,
    solver: SUNLinearSolver,
}

impl Drop for Raw {
    fn drop(&mut self) {
        // SAFETY: どれも `Ida::build` が作ったもので、ここでしか解放しない（null は飛ばす）
        unsafe {
            if !self.mem.is_null() {
                IDAFree(&mut self.mem);
            }
            if !self.solver.is_null() {
                SUNLinSolFree(self.solver);
            }
            if !self.matrix.is_null() {
                SUNMatDestroy(self.matrix);
            }
            if !self.yp.is_null() {
                N_VDestroy(self.yp);
            }
            if !self.abstol.is_null() {
                N_VDestroy(self.abstol);
            }
            if !self.y.is_null() {
                N_VDestroy(self.y);
            }
            if !self.ctx.is_null() {
                SUNContext_Free(&mut self.ctx);
            }
        }
    }
}

pub(super) struct Ida {
    rtol: f64,
    atol: Vec<f64>,
    /// `raw` が参照する user_data。`raw` より後に落とす（宣言の順）
    raw: Option<Raw>,
    user: Option<Box<User>>,
    /// 直近に受理した歩の区間 [start, end]。`valid` が false なら区間は無い
    start: f64,
    end: f64,
    valid: bool,
    /// IDA の履歴が今の点（歩の終点）のものか（初期化も切り詰めも無い）
    running: bool,
}

impl Ida {
    pub(super) fn new(setup: MeNumericalSetup) -> Self {
        Self {
            rtol: setup.relative_tolerance(),
            atol: scaled_absolute_tolerances(&setup),
            raw: None,
            user: None,
            start: 0.0,
            end: 0.0,
            valid: false,
            running: false,
        }
    }

    fn fail(&self, category: MeNumericalFailure, detail: impl Into<String>) -> MeIntegrationError {
        MeIntegrationError::numerical(NAME, category, detail)
    }

    fn construction(&self, what: &str, flag: c_int) -> MeIntegrationError {
        self.fail(
            MeNumericalFailure::Construction,
            format!("SUNDIALS {what} failed (flag {flag})"),
        )
    }

    /// IDA を作る（始点 `t0`、`y0`、y'0 = f(t0, y0)）。失敗したら作りかけは `Raw` の `Drop` が解放する
    fn build(&mut self, t0: f64, y0: &[f64]) -> Result<(), MeIntegrationError> {
        let n = y0.len();
        let Ok(len) = i64::try_from(n) else {
            return Err(self.construction("vector length", -1));
        };
        let mut raw = Raw {
            ctx: ptr::null_mut(),
            mem: ptr::null_mut(),
            y: ptr::null_mut(),
            yp: ptr::null_mut(),
            abstol: ptr::null_mut(),
            matrix: ptr::null_mut(),
            solver: ptr::null_mut(),
        };
        // SAFETY: 戻り値の null・負のフラグを見て進む。生ポインタは `raw` が持ち、`Drop` で解放する
        unsafe {
            let flag = SUNContext_Create(comm_no_mpi(), &mut raw.ctx);
            if flag < 0 {
                return Err(self.construction("SUNContext_Create", flag));
            }
            // 既定の誤差ハンドラは標準エラーに書く。失敗は戻り値のフラグで伝える
            SUNContext_ClearErrHandlers(raw.ctx);
            raw.y = N_VNew_Serial(len, raw.ctx);
            raw.yp = N_VNew_Serial(len, raw.ctx);
            raw.abstol = N_VNew_Serial(len, raw.ctx);
            if raw.y.is_null() || raw.yp.is_null() || raw.abstol.is_null() {
                return Err(self.construction("N_VNew_Serial", -1));
            }
            slice::from_raw_parts_mut(N_VGetArrayPointer(raw.y), n).copy_from_slice(y0);
            slice::from_raw_parts_mut(N_VGetArrayPointer(raw.abstol), n)
                .copy_from_slice(&self.atol);
            let yp0 = slice::from_raw_parts_mut(N_VGetArrayPointer(raw.yp), n);
            self.initial_slope(t0, y0, yp0)?;
            raw.mem = IDACreate(raw.ctx);
            if raw.mem.is_null() {
                return Err(self.construction("IDACreate", -1));
            }
            let flag = IDAInit(raw.mem, Some(residual), t0, raw.y, raw.yp);
            if flag != IDA_SUCCESS {
                return Err(self.construction("IDAInit", flag));
            }
            let user: *mut User = &mut **self.user.as_mut().expect("initialize で入れた");
            let flag = IDASetUserData(raw.mem, user.cast());
            if flag != IDA_SUCCESS {
                return Err(self.construction("IDASetUserData", flag));
            }
            let flag = IDASVtolerances(raw.mem, self.rtol, raw.abstol);
            if flag != IDA_SUCCESS {
                return Err(self.construction("IDASVtolerances", flag));
            }
            raw.matrix = SUNDenseMatrix(len, len, raw.ctx);
            if raw.matrix.is_null() {
                return Err(self.construction("SUNDenseMatrix", -1));
            }
            raw.solver = SUNLinSol_Dense(raw.y, raw.matrix, raw.ctx);
            if raw.solver.is_null() {
                return Err(self.construction("SUNLinSol_Dense", -1));
            }
            let flag = IDASetLinearSolver(raw.mem, raw.solver, raw.matrix);
            if flag != IDALS_SUCCESS as c_int {
                return Err(self.construction("IDASetLinearSolver", flag));
            }
        }
        self.raw = Some(raw);
        Ok(())
    }

    /// 始点の傾き y'0 = f(t, y) を導関数 1 回で求める（`IDACalcIC` の代わり）。断られた・有限でない値なら失敗
    fn initial_slope(&self, t: f64, y: &[f64], yp: &mut [f64]) -> Result<(), MeIntegrationError> {
        let user = self.user.as_ref().expect("initialize で入れた");
        user.derivatives.derivatives_into(t, y, yp);
        if user.derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        if user.derivatives.take_discard() || !yp.iter().all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("the derivative at the start point t={t} was discarded or not finite"),
            ));
        }
        Ok(())
    }
}

/// 残差 F(t, y, y') = y' − f(t, y)。0 = 成功、正 = 回復できる失敗（IDA が刻みを縮める）、負 = 回復できない失敗
extern "C" fn residual(
    t: f64,
    y: N_Vector,
    yp: N_Vector,
    rr: N_Vector,
    user_data: *mut c_void,
) -> c_int {
    // SAFETY: user_data は `build` が渡した `Box<User>` の住所で、IDA の生きている間は有効。
    // y・yp・rr は長さ n の serial ベクトル。パニックは FFI の外へ出さない
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let user = &*user_data.cast::<User>();
        let y = slice::from_raw_parts(N_VGetArrayPointer(y), user.n);
        let yp = slice::from_raw_parts(N_VGetArrayPointer(yp), user.n);
        let rr = slice::from_raw_parts_mut(N_VGetArrayPointer(rr), user.n);
        user.derivatives.derivatives_into(t, y, rr);
        if user.derivatives.has_failed() {
            -1
        } else if user.derivatives.take_discard() || !rr.iter().all(|v| v.is_finite()) {
            1
        } else {
            for (r, p) in rr.iter_mut().zip(yp) {
                *r = p - *r;
            }
            0
        }
    }))
    .unwrap_or(-1)
}

/// IDA の戻り値のフラグの名前と、分類
fn flag_info(flag: c_int) -> (&'static str, MeNumericalFailure) {
    use MeNumericalFailure::{AdvanceExhausted, LinearAlgebra};
    match flag {
        f if f == IDA_TOO_MUCH_WORK => ("IDA_TOO_MUCH_WORK", AdvanceExhausted),
        f if f == IDA_TOO_MUCH_ACC => ("IDA_TOO_MUCH_ACC", AdvanceExhausted),
        f if f == IDA_ERR_FAIL => ("IDA_ERR_FAIL", AdvanceExhausted),
        f if f == IDA_CONV_FAIL => ("IDA_CONV_FAIL", AdvanceExhausted),
        f if f == IDA_LINIT_FAIL => ("IDA_LINIT_FAIL", LinearAlgebra),
        f if f == IDA_LSETUP_FAIL => ("IDA_LSETUP_FAIL", LinearAlgebra),
        f if f == IDA_LSOLVE_FAIL => ("IDA_LSOLVE_FAIL", LinearAlgebra),
        f if f == IDA_RES_FAIL => ("IDA_RES_FAIL", AdvanceExhausted),
        f if f == IDA_FIRST_RES_FAIL => ("IDA_FIRST_RES_FAIL", AdvanceExhausted),
        f if f == IDA_REP_RES_ERR => ("IDA_REP_RES_ERR", AdvanceExhausted),
        _ => ("(その他のフラグ)", AdvanceExhausted),
    }
}

impl MeIntegratorBackend for Ida {
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
        self.raw = None;
        self.valid = false;
        self.running = false;
        self.user = Some(Box::new(User {
            derivatives,
            n: point.width(),
        }));
        self.build(point.time(), point.states())
    }

    fn advance(
        &mut self,
        request: &MeAdvanceRequest,
    ) -> Result<MeStepCandidate, MeIntegrationError> {
        let (Some(raw), Some(user)) = (self.raw.as_ref(), self.user.as_ref()) else {
            return Err(self.fail(
                MeNumericalFailure::Construction,
                "the host has not initialized the IDA integrator",
            ));
        };
        let current = request.current();
        let now = current.time();
        let stop = request.latest_accepted_time();
        let n = user.n;
        let (mem, y, yp) = (raw.mem, raw.y, raw.yp);
        // SAFETY: mem・y・yp は `build` で作った有効なもの。y・yp は長さ n の serial ベクトル
        let y_slice = unsafe { slice::from_raw_parts_mut(N_VGetArrayPointer(y), n) };

        // 続きかどうか: 前の歩の終点と同じ点なら IDA の履歴を使う。違えば（イベント・リセットの後）始め直す
        let continues =
            self.running && self.end.to_bits() == now.to_bits() && y_slice == current.states();
        self.valid = false;
        self.running = false;
        y_slice.copy_from_slice(current.states());
        let mut t = now;
        // SAFETY: 同上。返り値のフラグはすべて見る
        let flag = unsafe {
            let yp_slice = slice::from_raw_parts_mut(N_VGetArrayPointer(yp), n);
            let reinit = if continues {
                IDA_SUCCESS
            } else {
                // 始め直す点の傾きは y' = f(now, y)
                self.initial_slope(now, current.states(), yp_slice)?;
                IDAReInit(mem, now, y, yp)
            };
            if reinit != IDA_SUCCESS {
                return Err(self.construction("IDAReInit", reinit));
            }
            let stop_flag = IDASetStopTime(mem, stop);
            if stop_flag != IDA_SUCCESS {
                return Err(self.fail(
                    MeNumericalFailure::AdvanceExhausted,
                    format!("IDASetStopTime({stop}) failed at t={now} (flag {stop_flag})"),
                ));
            }
            IDASolve(mem, stop, &mut t, y, yp, IDA_ONE_STEP)
        };
        // 導関数の拒否が先（NaN で IDA の他の失敗も起きるため）
        if user.derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        if flag != IDA_SUCCESS && flag != IDA_TSTOP_RETURN {
            let (name, category) = flag_info(flag);
            return Err(self.fail(
                category,
                format!("IDASolve failed with {name} (flag {flag}) in a step from t={now}"),
            ));
        }
        let end = y_slice.to_vec();
        if !end.iter().all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("an IDA step from t={now} to t={t} reached a non-finite state"),
            ));
        }
        // 密な出力の区間は [tn - hu, tn]。ストップ時刻に着いたときは t が tn より丸め誤差だけ先
        let (mut tn, mut hu, mut order) = (0.0, 0.0, 0);
        // SAFETY: 同上
        unsafe {
            IDAGetCurrentTime(mem, &mut tn);
            IDAGetLastStep(mem, &mut hu);
            IDAGetLastOrder(mem, &mut order);
        }
        self.start = tn - hu;
        self.end = t;
        self.valid = true;
        self.running = true;
        Ok(MeStepCandidate::new(t, end, order.max(1).unsigned_abs()))
    }

    fn sample(&self, time: f64, states: &mut [f64]) -> Result<(), MeIntegrationError> {
        let (Some(raw), Some(user)) = (self.raw.as_ref(), self.user.as_ref()) else {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "the host has not initialized the IDA integrator",
            ));
        };
        if !self.valid {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "no accepted IDA step is available",
            ));
        }
        if states.len() != user.n {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "the sampler output width differs from the initialized component width",
            ));
        }
        if !accepted_interval_contains(self.start, self.end, time) {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                format!(
                    "sample time {time} lies outside [{}, {}]",
                    self.start, self.end
                ),
            ));
        }
        // IDAGetDky は [tn - hu, tn] の外（丸め誤差の余裕を除く）を断る。範囲の端に寄せる
        let tn = self.end.max(self.start);
        let time = time.clamp(self.start, tn);
        // SAFETY: dky に使うベクトルは y（長さ n）。IDA の状態は歩の後から変わっていない（valid）。
        // y には歩の終点が入っているので、使い終わったら書き戻す
        let flag = unsafe {
            let y = slice::from_raw_parts_mut(N_VGetArrayPointer(raw.y), user.n);
            let saved = y.to_vec();
            let flag = IDAGetDky(raw.mem, time, 0, raw.y);
            states.copy_from_slice(y);
            y.copy_from_slice(&saved);
            flag
        };
        if flag != IDA_SUCCESS {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                format!("IDAGetDky failed at t={time} (flag {flag})"),
            ));
        }
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

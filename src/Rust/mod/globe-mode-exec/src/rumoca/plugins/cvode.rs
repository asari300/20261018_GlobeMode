//! `cvode`: SUNDIALS の CVODE（BDF、可変次数 1〜5・可変刻み）を `MeIntegratorBackend` に包む（S032）。
//! feature `sundials` のときだけ組み込む。SUNDIALS は `sundials-sys` が同梱のソース（BSD-3-Clause）から cmake で
//! ビルドして静的にリンクする。この 1 ファイルだけが C の API（unsafe）に触れる。
//!
//! - 1 回の `advance` で CVODE の内部の 1 歩（`CV_ONE_STEP`）を進める。ホストの上限（`latest_accepted_time`）は
//!   `CVodeSetStopTime` で渡し、超えない。
//! - 刻みの間は CVODE の密な出力（Nordsieck 配列の多項式、`CVodeGetDky`）。多項式の次数は歩の次数 q で、
//!   `MeStepCandidate` の order にはその q を入れる。
//! - 前の歩の終点と同じ点なら CVODE の履歴（次数・刻み）を続け、違えば（イベント・リセットの後）`CVodeReInit` で
//!   その点から次数 1 で始め直す。
//! - 導関数がホストに断られた（discard）試行は、右辺の関数が正の値を返して CVODE に刻みを縮めさせる。
//!   ホストの失敗（`has_failed`）は負の値を返して止め、`DerivativeRefused` にする。
//!
//! shortcut: 線形ソルバは密な行列だけ（`SUNLinSol_Dense`）でヤコビ行列は CVODE の差分商。部品の方向微分や疎な
//! 構造は使わない（状態数が大きいと遅い）。根探索（`CVodeRootInit`）も使わず、事象はホストが受け持つ。
//! 疎な線形ソルバ（KLU は LGPL なので入れない）が要るときに見直す。

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
    CV_BDF, CV_CONV_FAILURE, CV_ERR_FAILURE, CV_FIRST_RHSFUNC_ERR, CV_LINIT_FAIL, CV_LSETUP_FAIL,
    CV_LSOLVE_FAIL, CV_ONE_STEP, CV_REPTD_RHSFUNC_ERR, CV_RHSFUNC_FAIL, CV_SUCCESS,
    CV_TOO_MUCH_ACC, CV_TOO_MUCH_WORK, CV_TSTOP_RETURN, CV_UNREC_RHSFUNC_ERR, CVLS_SUCCESS, CVode,
    CVodeCreate, CVodeFree, CVodeGetCurrentTime, CVodeGetDky, CVodeGetLastOrder, CVodeGetLastStep,
    CVodeInit, CVodeReInit, CVodeSVtolerances, CVodeSetLinearSolver, CVodeSetStopTime,
    CVodeSetUserData, N_VDestroy, N_VGetArrayPointer, N_VNew_Serial, N_Vector, SUNContext,
    SUNContext_ClearErrHandlers, SUNContext_Create, SUNContext_Free, SUNDenseMatrix,
    SUNLinSol_Dense, SUNLinSolFree, SUNLinearSolver, SUNMatDestroy, SUNMatrix, comm_no_mpi,
};

use super::rk::scaled_absolute_tolerances;

const NAME: &str = "cvode";

/// 右辺の関数に渡す user_data。`Box` に入れて住所を固定する（CVODE が覚えている）
struct User {
    derivatives: MeDerivativeHandle,
    n: usize,
}

/// SUNDIALS の持ち物をまとめたもの。`Drop` で全部解放する
struct Raw {
    ctx: SUNContext,
    mem: *mut c_void,
    y: N_Vector,
    abstol: N_Vector,
    matrix: SUNMatrix,
    solver: SUNLinearSolver,
}

impl Drop for Raw {
    fn drop(&mut self) {
        // SAFETY: どれも `Cvode::build` が作ったもので、ここでしか解放しない（null は飛ばす）
        unsafe {
            if !self.mem.is_null() {
                CVodeFree(&mut self.mem);
            }
            if !self.solver.is_null() {
                SUNLinSolFree(self.solver);
            }
            if !self.matrix.is_null() {
                SUNMatDestroy(self.matrix);
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

pub(super) struct Cvode {
    rtol: f64,
    atol: Vec<f64>,
    /// `raw` が参照する user_data。`raw` より後に落とす（宣言の順）
    raw: Option<Raw>,
    user: Option<Box<User>>,
    /// 直近に受理した歩の区間 [start, end]。`valid` が false なら区間は無い
    start: f64,
    end: f64,
    valid: bool,
    /// CVODE の履歴が今の点（歩の終点）のものか（初期化も切り詰めも無い）
    running: bool,
}

impl Cvode {
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

    /// CVODE を作る（始点 `t0`、`y0`）。失敗したら作りかけは `Raw` の `Drop` が解放する
    fn build(&mut self, t0: f64, y0: &[f64]) -> Result<(), MeIntegrationError> {
        let n = y0.len();
        let Ok(len) = i64::try_from(n) else {
            return Err(self.construction("vector length", -1));
        };
        let mut raw = Raw {
            ctx: ptr::null_mut(),
            mem: ptr::null_mut(),
            y: ptr::null_mut(),
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
            raw.abstol = N_VNew_Serial(len, raw.ctx);
            if raw.y.is_null() || raw.abstol.is_null() {
                return Err(self.construction("N_VNew_Serial", -1));
            }
            slice::from_raw_parts_mut(N_VGetArrayPointer(raw.y), n).copy_from_slice(y0);
            slice::from_raw_parts_mut(N_VGetArrayPointer(raw.abstol), n)
                .copy_from_slice(&self.atol);
            raw.mem = CVodeCreate(CV_BDF, raw.ctx);
            if raw.mem.is_null() {
                return Err(self.construction("CVodeCreate", -1));
            }
            let flag = CVodeInit(raw.mem, Some(rhs), t0, raw.y);
            if flag != CV_SUCCESS {
                return Err(self.construction("CVodeInit", flag));
            }
            let user: *mut User = &mut **self.user.as_mut().expect("initialize で入れた");
            let flag = CVodeSetUserData(raw.mem, user.cast());
            if flag != CV_SUCCESS {
                return Err(self.construction("CVodeSetUserData", flag));
            }
            let flag = CVodeSVtolerances(raw.mem, self.rtol, raw.abstol);
            if flag != CV_SUCCESS {
                return Err(self.construction("CVodeSVtolerances", flag));
            }
            raw.matrix = SUNDenseMatrix(len, len, raw.ctx);
            if raw.matrix.is_null() {
                return Err(self.construction("SUNDenseMatrix", -1));
            }
            raw.solver = SUNLinSol_Dense(raw.y, raw.matrix, raw.ctx);
            if raw.solver.is_null() {
                return Err(self.construction("SUNLinSol_Dense", -1));
            }
            let flag = CVodeSetLinearSolver(raw.mem, raw.solver, raw.matrix);
            if flag != CVLS_SUCCESS as c_int {
                return Err(self.construction("CVodeSetLinearSolver", flag));
            }
        }
        self.raw = Some(raw);
        Ok(())
    }
}

/// 右辺 f(t, y)。0 = 成功、正 = 回復できる失敗（CVODE が刻みを縮める）、負 = 回復できない失敗
extern "C" fn rhs(t: f64, y: N_Vector, dy: N_Vector, user_data: *mut c_void) -> c_int {
    // SAFETY: user_data は `build` が渡した `Box<User>` の住所で、CVODE の生きている間は有効。
    // y・dy は長さ n の serial ベクトル。パニックは FFI の外へ出さない
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let user = &*user_data.cast::<User>();
        let y = slice::from_raw_parts(N_VGetArrayPointer(y), user.n);
        let dy = slice::from_raw_parts_mut(N_VGetArrayPointer(dy), user.n);
        user.derivatives.derivatives_into(t, y, dy);
        if user.derivatives.has_failed() {
            -1
        } else if user.derivatives.take_discard() || !dy.iter().all(|v| v.is_finite()) {
            1
        } else {
            0
        }
    }))
    .unwrap_or(-1)
}

/// CVODE の戻り値のフラグの名前と、分類
fn flag_info(flag: c_int) -> (&'static str, MeNumericalFailure) {
    use MeNumericalFailure::{AdvanceExhausted, LinearAlgebra};
    match flag {
        f if f == CV_TOO_MUCH_WORK => ("CV_TOO_MUCH_WORK", AdvanceExhausted),
        f if f == CV_TOO_MUCH_ACC => ("CV_TOO_MUCH_ACC", AdvanceExhausted),
        f if f == CV_ERR_FAILURE => ("CV_ERR_FAILURE", AdvanceExhausted),
        f if f == CV_CONV_FAILURE => ("CV_CONV_FAILURE", AdvanceExhausted),
        f if f == CV_LINIT_FAIL => ("CV_LINIT_FAIL", LinearAlgebra),
        f if f == CV_LSETUP_FAIL => ("CV_LSETUP_FAIL", LinearAlgebra),
        f if f == CV_LSOLVE_FAIL => ("CV_LSOLVE_FAIL", LinearAlgebra),
        f if f == CV_RHSFUNC_FAIL => ("CV_RHSFUNC_FAIL", AdvanceExhausted),
        f if f == CV_FIRST_RHSFUNC_ERR => ("CV_FIRST_RHSFUNC_ERR", AdvanceExhausted),
        f if f == CV_REPTD_RHSFUNC_ERR => ("CV_REPTD_RHSFUNC_ERR", AdvanceExhausted),
        f if f == CV_UNREC_RHSFUNC_ERR => ("CV_UNREC_RHSFUNC_ERR", AdvanceExhausted),
        _ => ("(その他のフラグ)", AdvanceExhausted),
    }
}

impl MeIntegratorBackend for Cvode {
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
                "the host has not initialized the CVODE integrator",
            ));
        };
        let current = request.current();
        let now = current.time();
        let stop = request.latest_accepted_time();
        let n = user.n;
        let (mem, y) = (raw.mem, raw.y);
        // SAFETY: mem・y は `build` で作った有効なもの。y は長さ n の serial ベクトル
        let y_slice = unsafe { slice::from_raw_parts_mut(N_VGetArrayPointer(y), n) };

        // 続きかどうか: 前の歩の終点と同じ点なら CVODE の履歴を使う。違えば（イベント・リセットの後）始め直す
        let continues =
            self.running && self.end.to_bits() == now.to_bits() && y_slice == current.states();
        self.valid = false;
        self.running = false;
        y_slice.copy_from_slice(current.states());
        let mut t = now;
        // SAFETY: 同上。返り値のフラグはすべて見る
        let flag = unsafe {
            let reinit = if continues {
                CV_SUCCESS
            } else {
                CVodeReInit(mem, now, y)
            };
            if reinit != CV_SUCCESS {
                return Err(self.construction("CVodeReInit", reinit));
            }
            let stop_flag = CVodeSetStopTime(mem, stop);
            if stop_flag != CV_SUCCESS {
                return Err(self.fail(
                    MeNumericalFailure::AdvanceExhausted,
                    format!("CVodeSetStopTime({stop}) failed at t={now} (flag {stop_flag})"),
                ));
            }
            CVode(mem, stop, y, &mut t, CV_ONE_STEP)
        };
        // 導関数の拒否が先（NaN で CVODE の他の失敗も起きるため）
        if user.derivatives.has_failed() {
            return Err(MeIntegrationError::DerivativeRefused);
        }
        if flag != CV_SUCCESS && flag != CV_TSTOP_RETURN {
            let (name, category) = flag_info(flag);
            return Err(self.fail(
                category,
                format!("CVode failed with {name} (flag {flag}) in a step from t={now}"),
            ));
        }
        let end = y_slice.to_vec();
        if !end.iter().all(|v| v.is_finite()) {
            return Err(self.fail(
                MeNumericalFailure::AdvanceExhausted,
                format!("a CVODE step from t={now} to t={t} reached a non-finite state"),
            ));
        }
        // 密な出力の区間は [tn - hu, tn]。ストップ時刻に着いたときは t が tn より丸め誤差だけ先
        let (mut tn, mut hu, mut order) = (0.0, 0.0, 0);
        // SAFETY: 同上
        unsafe {
            CVodeGetCurrentTime(mem, &mut tn);
            CVodeGetLastStep(mem, &mut hu);
            CVodeGetLastOrder(mem, &mut order);
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
                "the host has not initialized the CVODE integrator",
            ));
        };
        if !self.valid {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                "no accepted CVODE step is available",
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
        // CVodeGetDky は [tn - hu, tn] の外（丸め誤差の余裕を除く）を断る。範囲の端に寄せる
        let tn = self.end.max(self.start);
        let time = time.clamp(self.start, tn);
        // SAFETY: dky に使うベクトルは y（長さ n）。CVODE の状態は歩の後から変わっていない（valid）。
        // y には歩の終点が入っているので、使い終わったら書き戻す
        let flag = unsafe {
            let y = slice::from_raw_parts_mut(N_VGetArrayPointer(raw.y), user.n);
            let saved = y.to_vec();
            let flag = CVodeGetDky(raw.mem, time, 0, raw.y);
            states.copy_from_slice(y);
            y.copy_from_slice(&saved);
            flag
        };
        if flag != CV_SUCCESS {
            return Err(self.fail(
                MeNumericalFailure::Interpolation,
                format!("CVodeGetDky failed at t={time} (flag {flag})"),
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

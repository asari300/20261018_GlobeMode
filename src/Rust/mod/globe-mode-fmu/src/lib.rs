//! GlobeMode の自前の FMI 2.0 Model Exchange のランタイム（P5b の 1 回目、S095、ADR-0032）。
//!
//! モデルは `GlobeModeValidation.Simulation.Decay`（`der(x) = -k*x`、`x(0) = 1`、`k = 1`）を手で
//! この crate に固定する。`modelDescription.xml`（手書き）と組で FMU に詰める（`tests/fmpy.rs`）。
//! 任意のモデルを載せ替える道は ADR-0032 に書いて持ち越す。
//!
//! - 値参照: 0 = `x`、1 = `der(x)`、2 = `k`。
//! - 状態の保存（FMUstate）・方向微分・Integer/Boolean/String の値は無いので `fmi2Error` を返す（`nvr = 0` は OK）。
//! - ログの callback は呼ばない。panic は起きない作りだが、起きても `extern "C"` の境界で abort する（Rust 2024）。
// 安全の約束は FMI 2.0 の規格（呼び手は fmi2Instantiate が返した c と、nvr 個の有効な配列を渡す）
#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

/// `modelDescription.xml` の `guid` と同じ。
pub const GUID: &str = "{6f1d6a0e-95a1-4c1e-9b8e-5095f0d0decb}";

type Component = *mut c_void;
type Status = c_int;
const OK: Status = 0;
const ERROR: Status = 3;
/// `fmi2ModelExchange`
const MODEL_EXCHANGE: c_int = 0;

/// 1 つの FMU のインスタンス（Decay の状態と係数。時間に陽に依らないので時刻は持たない）。
#[derive(Debug)]
struct Decay {
    x: f64,
    k: f64,
}

impl Decay {
    const START: Decay = Decay { x: 1.0, k: 1.0 };
}

/// FMI 2.0 の `fmi2EventInfo`。
#[repr(C)]
#[derive(Debug)]
pub struct EventInfo {
    pub new_discrete_states_needed: c_int,
    pub terminate_simulation: c_int,
    pub nominals_of_continuous_states_changed: c_int,
    pub values_of_continuous_states_changed: c_int,
    pub next_event_time_defined: c_int,
    pub next_event_time: f64,
}

unsafe fn inst<'a>(c: Component) -> Option<&'a mut Decay> {
    // SAFETY: c は fmi2Instantiate が返し、まだ放していないもの（規格の約束）
    unsafe { c.cast::<Decay>().as_mut() }
}

/// `nvr` 個の値参照と値の配列を組にして `f` に渡す。値参照が範囲外なら `fmi2Error`。
unsafe fn each<T>(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut T,
    mut f: impl FnMut(&mut Decay, c_uint, &mut T) -> bool,
) -> Status {
    if nvr == 0 {
        return OK;
    }
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if vr.is_null() || value.is_null() {
        return ERROR;
    }
    // SAFETY: 呼び手は nvr 個の有効な配列を渡す
    let (vr, value) = unsafe {
        (
            std::slice::from_raw_parts(vr, nvr),
            std::slice::from_raw_parts_mut(value, nvr),
        )
    };
    if vr.iter().zip(value).all(|(&r, v)| f(m, r, v)) {
        OK
    } else {
        ERROR
    }
}

/// 長さ `n` の配列の先頭に `f` で書く（Decay の状態は 1 つなので `n == 1` だけ）。
unsafe fn one(c: Component, p: *mut f64, n: usize, f: impl FnOnce(&mut Decay, &mut f64)) -> Status {
    match (unsafe { inst(c) }, unsafe { p.as_mut() }) {
        (Some(m), Some(v)) if n == 1 => {
            f(m, v);
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
    c"default".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fmi2GetVersion() -> *const c_char {
    c"2.0".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetDebugLogging(
    _c: Component,
    _on: c_int,
    _n: usize,
    _cats: *const *const c_char,
) -> Status {
    OK
}

/// ME だけを受け、`guid` が違えば NULL を返す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Instantiate(
    _instance_name: *const c_char,
    fmu_type: c_int,
    guid: *const c_char,
    _resource_location: *const c_char,
    _functions: *const c_void,
    _visible: c_int,
    _logging_on: c_int,
) -> Component {
    // SAFETY: guid は NUL 終端の文字列（規格の約束）
    let ok = !guid.is_null() && unsafe { CStr::from_ptr(guid) }.to_bytes() == GUID.as_bytes();
    if fmu_type != MODEL_EXCHANGE || !ok {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(Decay::START)).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2FreeInstance(c: Component) {
    if !c.is_null() {
        // SAFETY: fmi2Instantiate の Box::into_raw を 1 度だけ戻す
        drop(unsafe { Box::from_raw(c.cast::<Decay>()) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetupExperiment(
    c: Component,
    _tol_defined: c_int,
    _tol: f64,
    _start: f64,
    _stop_defined: c_int,
    _stop: f64,
) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterInitializationMode(c: Component) -> Status {
    if unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2ExitInitializationMode(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Terminate(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Reset(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        *m = Decay::START;
        OK
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetReal(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut f64,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            *v = match r {
                0 => m.x,
                1 => -m.k * m.x,
                2 => m.k,
                _ => return false,
            };
            true
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetReal(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *const f64,
) -> Status {
    unsafe {
        each(c, vr, nvr, value.cast_mut(), |m, r, v| {
            match r {
                0 => m.x = *v,
                2 => m.k = *v,
                _ => return false,
            }
            true
        })
    }
}

/// Integer・Boolean・String の変数は無い（`nvr = 0` だけ OK）。
fn no_variables(nvr: usize) -> Status {
    if nvr == 0 { OK } else { ERROR }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetInteger(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *mut c_int,
) -> Status {
    no_variables(nvr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetBoolean(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *mut c_int,
) -> Status {
    no_variables(nvr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetString(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *mut *const c_char,
) -> Status {
    no_variables(nvr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetInteger(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *const c_int,
) -> Status {
    no_variables(nvr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetBoolean(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *const c_int,
) -> Status {
    no_variables(nvr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetString(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *const *const c_char,
) -> Status {
    no_variables(nvr)
}

// canGetAndSetFMUstate・canSerializeFMUstate・providesDirectionalDerivative は false（modelDescription.xml の既定）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetFMUstate(_c: Component, _s: *mut *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetFMUstate(_c: Component, _s: *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2FreeFMUstate(_c: Component, _s: *mut *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SerializedFMUstateSize(
    _c: Component,
    _s: *mut c_void,
    _size: *mut usize,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SerializeFMUstate(
    _c: Component,
    _s: *mut c_void,
    _bytes: *mut c_char,
    _size: usize,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2DeSerializeFMUstate(
    _c: Component,
    _bytes: *const c_char,
    _size: usize,
    _s: *mut *mut c_void,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetDirectionalDerivative(
    _c: Component,
    _unknown: *const c_uint,
    _n_unknown: usize,
    _known: *const c_uint,
    _n_known: usize,
    _dv_known: *const f64,
    _dv_unknown: *mut f64,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterEventMode(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

/// Decay にイベントは無いので、すべて偽で返す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2NewDiscreteStates(c: Component, info: *mut EventInfo) -> Status {
    match (unsafe { inst(c) }, unsafe { info.as_mut() }) {
        (Some(_), Some(info)) => {
            *info = EventInfo {
                new_discrete_states_needed: 0,
                terminate_simulation: 0,
                nominals_of_continuous_states_changed: 0,
                values_of_continuous_states_changed: 0,
                next_event_time_defined: 0,
                next_event_time: 0.0,
            };
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterContinuousTimeMode(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2CompletedIntegratorStep(
    c: Component,
    _no_set_prior: c_int,
    enter_event_mode: *mut c_int,
    terminate: *mut c_int,
) -> Status {
    match (
        unsafe { inst(c) },
        unsafe { enter_event_mode.as_mut() },
        unsafe { terminate.as_mut() },
    ) {
        (Some(_), Some(e), Some(t)) => {
            (*e, *t) = (0, 0);
            OK
        }
        _ => ERROR,
    }
}

/// Decay は時間に陽に依らないので時刻は持たない。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetTime(c: Component, _time: f64) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetContinuousStates(c: Component, x: *const f64, nx: usize) -> Status {
    unsafe { one(c, x.cast_mut(), nx, |m, v| m.x = *v) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetContinuousStates(c: Component, x: *mut f64, nx: usize) -> Status {
    unsafe { one(c, x, nx, |m, v| *v = m.x) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetDerivatives(c: Component, dx: *mut f64, nx: usize) -> Status {
    unsafe { one(c, dx, nx, |m, v| *v = -m.k * m.x) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetNominalsOfContinuousStates(
    c: Component,
    x: *mut f64,
    nx: usize,
) -> Status {
    unsafe { one(c, x, nx, |_, v| *v = 1.0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetEventIndicators(c: Component, _z: *mut f64, ni: usize) -> Status {
    if ni == 0 && unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

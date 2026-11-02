//! GlobeMode の自前の FMI 2.0 Model Exchange のランタイム（P5b、S095・S099、ADR-0032）。
//!
//! S099: モデルは FMU の `resources/<完全修飾名>.mo`（1 ファイル）から `fmi2Instantiate` で compile し、
//! `globe_mode_exec::ModelInstance` で微分を求める。`modelDescription.xml` は [`model_description`] が同じ道で作る
//! （手書きをやめた）。事象のあるモデルは `ModelInstance::new` が断るので、`fmi2Instantiate` が NULL を返す。
//!
//! - 値参照: `0..n` = 状態、`n..2n` = その微分（`der(…)`）、`2n..` = tunable なパラメータ。
//! - パラメータの変更は `fmi2ExitInitializationMode` で lowering からやり直して反映する（依存するパラメータにも届く）。
//! - 状態の保存（FMUstate）・方向微分・Integer/Boolean/String の値は無いので `fmi2Error` を返す（`nvr = 0` は OK）。
//! - ログの callback は呼ばない（失敗の理由は標準エラーに出す）。panic は `extern "C"` の境界で abort する（Rust 2024）。
// 安全の約束は FMI 2.0 の規格（呼び手は fmi2Instantiate が返した c と、nvr 個の有効な配列を渡す）
#![allow(clippy::missing_safety_doc)]

use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use globe_mode_exec::{CompiledModel, ModelInstance};
use globe_mode_frontend::{Checker, Source};

type Component = *mut c_void;
type Status = c_int;
const OK: Status = 0;
const ERROR: Status = 3;
/// `fmi2ModelExchange`
const MODEL_EXCHANGE: c_int = 0;

/// `modelDescription.xml` の `guid`。モデルの名前と文面の FNV-1a（64 ビット）なので、`fmi2Instantiate` が
/// resources の文面から同じ値を作って突き合わせられる。
pub fn guid(model: &str, source: &str) -> String {
    let hash = [model.as_bytes(), &[0], source.as_bytes()]
        .concat()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("{{globe-mode-{hash:016x}}}")
}

/// 1 ファイルの Modelica の文面から `model` を DAE まで変換する（MSL は読まない）。
pub fn compile(model: &str, source: &str) -> Result<CompiledModel, String> {
    let checker = Checker::new(&[]).map_err(|d| format!("{d:?}"))?;
    let sources = [Source {
        name: "model.mo",
        text: source,
    }];
    checker
        .compile_retaining(&sources, model)
        .map_err(|d| format!("{d:?}"))
}

/// FMU の `modelDescription.xml`（FMI 2.0 ME）。変数表は [`ModelInstance`] の状態とパラメータから作る。
pub fn model_description(model: &str, source: &str) -> Result<String, String> {
    let compiled = compile(model, source)?;
    let inst =
        ModelInstance::new(&compiled, &BTreeMap::new(), 0.0).map_err(|d| format!("{d:?}"))?;
    let e = compiled.experiment.clone().unwrap_or_default();
    let start = e.start_time.map_or(0.0, |b| b.get());
    let n = inst.n_states();
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;")
    };
    let mut v = String::new();
    for (i, (name, x)) in inst.state_names().iter().zip(inst.states()).enumerate() {
        let _ = write!(
            v,
            "    <ScalarVariable name=\"{}\" valueReference=\"{i}\" causality=\"local\" variability=\"continuous\" initial=\"exact\">\n      <Real start=\"{x:?}\"/>\n    </ScalarVariable>\n",
            esc(name)
        );
    }
    for (i, name) in inst.state_names().iter().enumerate() {
        let _ = write!(
            v,
            "    <ScalarVariable name=\"der({})\" valueReference=\"{}\" causality=\"local\" variability=\"continuous\" initial=\"calculated\">\n      <Real derivative=\"{}\"/>\n    </ScalarVariable>\n",
            esc(name),
            n + i,
            i + 1
        );
    }
    for (i, (name, p)) in inst.parameters().iter().enumerate() {
        let _ = write!(
            v,
            "    <ScalarVariable name=\"{}\" valueReference=\"{}\" causality=\"parameter\" variability=\"fixed\" initial=\"exact\">\n      <Real start=\"{p:?}\"/>\n    </ScalarVariable>\n",
            esc(name),
            2 * n + i
        );
    }
    let unknowns: String = (n + 1..=2 * n)
        .map(|i| format!("      <Unknown index=\"{i}\"/>\n"))
        .collect();
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription
  fmiVersion="2.0"
  modelName="{name}"
  guid="{guid}"
  generationTool="GlobeMode globe-mode-fmu"
  variableNamingConvention="structured"
  numberOfEventIndicators="0">
  <ModelExchange modelIdentifier="globe_mode_fmu"/>
  <DefaultExperiment startTime="{start:?}" stopTime="{stop:?}" tolerance="{tol:?}"/>
  <ModelVariables>
{v}  </ModelVariables>
  <ModelStructure>
    <Derivatives>
{unknowns}    </Derivatives>
    <InitialUnknowns>
{unknowns}    </InitialUnknowns>
  </ModelStructure>
</fmiModelDescription>
"#,
        name = esc(model),
        guid = guid(model, source),
        stop = e.stop_time.map_or(start + 1.0, |b| b.get()),
        tol = e.tolerance.map_or(1e-6, |b| b.get()),
    ))
}

/// 1 つの FMU のインスタンス。
struct Instance {
    model: CompiledModel,
    inst: ModelInstance,
    start: f64,
    /// `fmi2SetReal` で変えたパラメータ（`fmi2ExitInitializationMode` で lowering からやり直して反映する）
    overrides: BTreeMap<String, f64>,
    dirty: bool,
}

impl Instance {
    /// パラメータか開始時刻が変わっていれば、作り直す。
    fn rebuild(&mut self) -> bool {
        if self.dirty {
            match ModelInstance::new(&self.model, &self.overrides, self.start) {
                Ok(inst) => self.inst = inst,
                Err(d) => {
                    eprintln!("globe-mode-fmu: {d:?}");
                    return false;
                }
            }
            self.dirty = false;
        }
        true
    }
}

/// `resources` の場所（`file:///…` の URI）から、ただ 1 つの `.mo` の（完全修飾名、文面）を読む。
// shortcut: %xx のエスケープを戻さない。空白などを含む置き場で使うときに戻す
fn read_resource(location: &str) -> Option<(String, String)> {
    let path = location.strip_prefix("file:")?;
    let dir = PathBuf::from(path.strip_prefix("//").unwrap_or(path));
    let mo = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|x| x == "mo"))?;
    let name = Path::new(mo.file_stem()?).to_str()?.to_string();
    Some((name, std::fs::read_to_string(&mo).ok()?))
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

unsafe fn inst<'a>(c: Component) -> Option<&'a mut Instance> {
    // SAFETY: c は fmi2Instantiate が返し、まだ放していないもの（規格の約束）
    unsafe { c.cast::<Instance>().as_mut() }
}

/// `nvr` 個の値参照と値の配列を組にして `f` に渡す。値参照が範囲外なら `fmi2Error`。
unsafe fn each<T>(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut T,
    mut f: impl FnMut(&mut Instance, c_uint, &mut T) -> bool,
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

/// 長さ `n`（状態の数）の配列を `f` に渡す。`f` が失敗したら `fmi2Error`。
unsafe fn states(
    c: Component,
    p: *mut f64,
    n: usize,
    f: impl FnOnce(&mut Instance, &mut [f64]) -> Result<(), String>,
) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if n != m.inst.n_states() || (n > 0 && p.is_null()) {
        return ERROR;
    }
    let v: &mut [f64] = if n == 0 {
        &mut []
    } else {
        // SAFETY: 呼び手は n 個の有効な配列を渡す
        unsafe { std::slice::from_raw_parts_mut(p, n) }
    };
    match f(m, v) {
        Ok(()) => OK,
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
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

/// ME だけを受け、resources の `.mo` を compile する。`guid` が文面と合わないか、載せられないモデルなら NULL を返す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Instantiate(
    _instance_name: *const c_char,
    fmu_type: c_int,
    guid: *const c_char,
    resource_location: *const c_char,
    _functions: *const c_void,
    _visible: c_int,
    _logging_on: c_int,
) -> Component {
    if fmu_type != MODEL_EXCHANGE || guid.is_null() || resource_location.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: guid と resource_location は NUL 終端の文字列（規格の約束）
    let (guid_given, location) = unsafe {
        (
            CStr::from_ptr(guid).to_string_lossy(),
            CStr::from_ptr(resource_location).to_string_lossy(),
        )
    };
    let Some((name, source)) = read_resource(&location) else {
        eprintln!("globe-mode-fmu: {location} に .mo が無い");
        return std::ptr::null_mut();
    };
    if self::guid(&name, &source) != guid_given {
        return std::ptr::null_mut();
    }
    let built = compile(&name, &source).and_then(|model| {
        ModelInstance::new(&model, &BTreeMap::new(), 0.0)
            .map(|inst| (model, inst))
            .map_err(|d| format!("{d:?}"))
    });
    match built {
        Ok((model, inst)) => Box::into_raw(Box::new(Instance {
            model,
            inst,
            start: 0.0,
            overrides: BTreeMap::new(),
            dirty: false,
        }))
        .cast(),
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2FreeInstance(c: Component) {
    if !c.is_null() {
        // SAFETY: fmi2Instantiate の Box::into_raw を 1 度だけ戻す
        drop(unsafe { Box::from_raw(c.cast::<Instance>()) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetupExperiment(
    c: Component,
    _tol_defined: c_int,
    _tol: f64,
    start: f64,
    _stop_defined: c_int,
    _stop: f64,
) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        if m.start != start {
            (m.start, m.dirty) = (start, true);
        }
        OK
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterInitializationMode(c: Component) -> Status {
    if unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

/// パラメータか開始時刻が変わっていれば、lowering からやり直して初期化し直す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2ExitInitializationMode(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| if m.rebuild() { OK } else { ERROR })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Terminate(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Reset(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        m.overrides.clear();
        (m.start, m.dirty) = (0.0, true);
        if m.rebuild() { OK } else { ERROR }
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
            let (n, r) = (m.inst.n_states(), r as usize);
            let got = if r < n {
                Some(m.inst.states()[r])
            } else if r < 2 * n {
                m.inst.derivatives().ok().map(|d| d[r - n])
            } else {
                m.inst
                    .parameters()
                    .get(r - 2 * n)
                    .map(|(name, p)| *m.overrides.get(name).unwrap_or(p))
            };
            got.map(|x| *v = x).is_some()
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
            let (n, r) = (m.inst.n_states(), r as usize);
            if r < n {
                let mut x = m.inst.states().to_vec();
                x[r] = *v;
                return m.inst.set_states(&x).is_ok();
            }
            let Some((name, _)) = r
                .checked_sub(2 * n)
                .and_then(|i| m.inst.parameters().get(i))
            else {
                return false;
            };
            m.overrides.insert(name.clone(), *v);
            m.dirty = true;
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

/// 事象のあるモデルは載せないので、すべて偽で返す。
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetTime(c: Component, time: f64) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        m.inst.set_time(time);
        OK
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetContinuousStates(c: Component, x: *const f64, nx: usize) -> Status {
    unsafe { states(c, x.cast_mut(), nx, |m, v| m.inst.set_states(v)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetContinuousStates(c: Component, x: *mut f64, nx: usize) -> Status {
    unsafe {
        states(c, x, nx, |m, v| {
            v.copy_from_slice(m.inst.states());
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetDerivatives(c: Component, dx: *mut f64, nx: usize) -> Status {
    unsafe {
        states(c, dx, nx, |m, v| {
            v.copy_from_slice(&m.inst.derivatives()?);
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetNominalsOfContinuousStates(
    c: Component,
    x: *mut f64,
    nx: usize,
) -> Status {
    unsafe {
        states(c, x, nx, |_, v| {
            v.fill(1.0);
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetEventIndicators(c: Component, _z: *mut f64, ni: usize) -> Status {
    if ni == 0 && unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

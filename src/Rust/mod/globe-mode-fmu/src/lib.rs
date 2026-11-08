//! GlobeMode の自前の FMI 2.0 Model Exchange のランタイム（P5b、S095・S099、ADR-0032）。
//!
//! S099: モデルは FMU の `resources/<完全修飾名>.mo`（1 ファイル）から `fmi2Instantiate` で compile し、
//! `globe_mode_exec::ModelInstance` で微分を求める。`modelDescription.xml` は [`model_description`] が同じ道で作る
//! （手書きをやめた）。
//!
//! S109: 事象。`fmi2GetEventIndicators` は root の条件の値（`numberOfEventIndicators` 本）、`fmi2EnterEventMode` →
//! `fmi2NewDiscreteStates` で `when` と `reinit` を回し、次の時刻事象（`sample`）を `nextEventTime` で返す。
//!
//! S112: 代数・離散の変数（`ModelInstance::variables`）を `fmi2GetReal`・`fmi2GetInteger`・`fmi2GetBoolean` で読む。
//!
//! S114: 入力（`causality="input"`）を `fmi2SetReal` で書く（`ModelInstance::set_input`）。
//!
//! S137: 同じ cdylib で FMI 3.0 の ME も出す（`fmi3*`。2.0 の関数を形だけ変えて呼ぶ。`modelDescription.xml` は
//! [`model_description_3`]）。3.0 の EventIndicator は値参照 `2n+p+q..`（q は代数・離散・入力の変数の数）の
//! Float64 の変数として載せ、その次に independent の `time` を置く。
//!
//! - 値参照: `0..n` = 状態、`n..2n` = その微分（`der(…)`）、`2n..2n+p` = tunable なパラメータ、
//!   `2n+p..` = 代数・出力・離散・入力の変数（S112・S114。書けるのは Real の入力だけ）。
//! - パラメータの変更は `fmi2ExitInitializationMode` で lowering からやり直して反映する（依存するパラメータにも届く）。
//! - 状態の保存（FMUstate）・方向微分・String の値と、Integer/Boolean の書き込みは無いので `fmi2Error` を返す（`nvr = 0` は OK）。
//! - ログの callback は呼ばない（失敗の理由は標準エラーに出す）。panic は `extern "C"` の境界で abort する（Rust 2024）。
// 安全の約束は FMI 2.0 の規格（呼び手は fmi2Instantiate が返した c と、nvr 個の有効な配列を渡す）
#![allow(clippy::missing_safety_doc)]

use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};

use globe_mode_exec::{CompiledModel, ModelInstance, ValueType, VariableRole};
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

/// FMU の `modelDescription.xml`（FMI 2.0 ME）。変数表は [`ModelInstance`] の状態とパラメータから作り、
/// XML は `globe_mode_fmi::write_model_description_2` が書く（S117、P5a と型を共有）。
pub fn model_description(model: &str, source: &str) -> Result<String, String> {
    describe(model, source, false)
}

/// FMU の `modelDescription.xml`（FMI 3.0 ME、S137）。変数表は 2.0 と同じで、EventIndicator の変数を足す。
pub fn model_description_3(model: &str, source: &str) -> Result<String, String> {
    describe(model, source, true)
}

fn describe(model: &str, source: &str, v3: bool) -> Result<String, String> {
    use globe_mode_fmi::{
        Causality, DefaultExperiment, Initial, Interface, ModelDescription, ModelStructure,
        ScalarVariable, VarType, Variability,
    };
    let compiled = compile(model, source)?;
    let inst =
        ModelInstance::new(&compiled, &BTreeMap::new(), 0.0).map_err(|d| format!("{d:?}"))?;
    let e = compiled.experiment.clone().unwrap_or_default();
    let start = e.start_time.map_or(0.0, |b| b.get());
    let n = inst.n_states();
    // ModelStructure と derivative の参照: 2.0 は変数の 1 始まりの索引、3.0 は valueReference
    let refer = |r: usize| r as u32 + u32::from(!v3);
    let var = |name: String, r: usize, causality, variability, initial, ty, start| ScalarVariable {
        name,
        value_reference: r as u32,
        causality,
        variability,
        ty,
        initial,
        start,
        derivative: None,
        dimensions: false,
    };
    let mut variables = Vec::new();
    for (i, (name, x)) in inst.state_names().iter().zip(inst.states()).enumerate() {
        variables.push(var(
            name.clone(),
            i,
            Causality::Local,
            Variability::Continuous,
            Some(Initial::Exact),
            VarType::Float64,
            Some(format!("{x:?}")),
        ));
    }
    for (i, name) in inst.state_names().iter().enumerate() {
        variables.push(ScalarVariable {
            derivative: Some(refer(i)),
            ..var(
                format!("der({name})"),
                n + i,
                Causality::Local,
                Variability::Continuous,
                Some(Initial::Calculated),
                VarType::Float64,
                None,
            )
        });
    }
    for (i, (name, p)) in inst.parameters().iter().enumerate() {
        variables.push(var(
            name.clone(),
            2 * n + i,
            Causality::Parameter,
            Variability::Fixed,
            Some(Initial::Exact),
            VarType::Float64,
            Some(format!("{p:?}")),
        ));
    }
    let first = 2 * n + inst.parameters().len();
    let values = inst.variable_values()?;
    let mut outputs = Vec::new();
    for (i, v) in inst.variables().enumerate() {
        let r = first + i;
        let (ty, discrete) = match v.value_type {
            ValueType::Real => (VarType::Float64, v.role == VariableRole::Discrete),
            ValueType::Integer => (VarType::Int32, true),
            ValueType::Boolean => (VarType::Boolean, true),
        };
        let (causality, initial, start) = match v.role {
            VariableRole::Output => {
                outputs.push(refer(r));
                (Causality::Output, Some(Initial::Calculated), None)
            }
            // 入力は initial を書かず start を必須とする（FMI 2.0 の表）
            VariableRole::Input => {
                // start は変数の型で書く（Integer・Boolean に f64 の `2.0` を書くと XSD に反する。#51）
                let start = match ty {
                    VarType::Int32 => format!("{}", values[i] as i32),
                    VarType::Boolean => format!("{}", values[i] != 0.0),
                    _ => format!("{:?}", values[i]),
                };
                (Causality::Input, None, Some(start))
            }
            _ => (Causality::Local, Some(Initial::Calculated), None),
        };
        let variability = if discrete {
            Variability::Discrete
        } else {
            Variability::Continuous
        };
        variables.push(var(
            v.name.clone(),
            r,
            causality,
            variability,
            initial,
            ty,
            start,
        ));
    }
    let mut event_indicators = Vec::new();
    if v3 {
        for i in 0..inst.n_event_indicators() {
            let r = first + values.len() + i;
            event_indicators.push(r as u32);
            variables.push(var(
                format!("_eventIndicator{}", i + 1),
                r,
                Causality::Local,
                Variability::Continuous,
                Some(Initial::Calculated),
                VarType::Float64,
                None,
            ));
        }
        // 3.0 は independent の変数がちょうど 1 つ要る（FMPy の validate）
        variables.push(var(
            "time".into(),
            first + values.len() + inst.n_event_indicators(),
            Causality::Independent,
            Variability::Continuous,
            None,
            VarType::Float64,
            None,
        ));
    }
    let derivatives: Vec<u32> = (n..2 * n).map(refer).collect();
    // 出力（initial="calculated"）は InitialUnknowns にも並べる（FMI 2.0。索引は微分より後なので昇順のまま）
    let initial_unknowns = [derivatives.clone(), outputs.clone()].concat();
    let md = ModelDescription {
        fmi_version: if v3 { "3.0" } else { "2.0" }.into(),
        model_name: model.into(),
        instantiation_token: guid(model, source),
        model_exchange: Some(Interface {
            model_identifier: "globe_mode_fmu".into(),
        }),
        co_simulation: None,
        scheduled_execution: None,
        variables,
        generation_tool: Some("GlobeMode globe-mode-fmu".into()),
        variable_naming_convention: Some("structured".into()),
        number_of_event_indicators: inst.n_event_indicators() as u32,
        default_experiment: Some(DefaultExperiment {
            start_time: Some(format!("{start:?}")),
            stop_time: Some(format!(
                "{:?}",
                e.stop_time.map_or(start + 1.0, |b| b.get())
            )),
            tolerance: Some(format!("{:?}", e.tolerance.map_or(1e-6, |b| b.get()))),
            step_size: None,
        }),
        model_structure: ModelStructure {
            outputs,
            derivatives,
            initial_unknowns,
            event_indicators,
        },
    };
    if v3 {
        globe_mode_fmi::write_model_description_3(&md)
    } else {
        globe_mode_fmi::write_model_description_2(&md)
    }
    .map_err(|e| e.to_string())
}

/// 1 つの FMU のインスタンス。
struct Instance {
    model: CompiledModel,
    inst: ModelInstance,
    start: f64,
    /// `fmi2SetReal` で変えたパラメータ（`fmi2ExitInitializationMode` で lowering からやり直して反映する）
    overrides: BTreeMap<String, f64>,
    /// `fmi2SetReal` で与えた入力（作り直したときに置き直す）
    inputs: BTreeMap<String, f64>,
    /// `fmi2SetReal` で与えた状態の値（添字 → 値。作り直したときに置き直す。start 値と同じ）
    states: BTreeMap<usize, f64>,
    dirty: bool,
}

impl Instance {
    /// パラメータか開始時刻が変わっていれば、作り直す。
    fn rebuild(&mut self) -> bool {
        if self.dirty {
            // shortcut: 初期化は入力の start で解き、作り直した後に与えた入力を置く。初期値が入力に依るモデルを載せるときに
            // `ModelInstance::new` へ入力を渡す
            match ModelInstance::new(&self.model, &self.overrides, self.start) {
                Ok(mut inst) => {
                    for (name, v) in &self.inputs {
                        let _ = inst.set_input(name, *v);
                    }
                    let mut x = inst.states().to_vec();
                    for (&i, &v) in &self.states {
                        x[i] = v;
                    }
                    if let Err(e) = inst.set_states(&x) {
                        eprintln!("globe-mode-fmu: {e}");
                        return false;
                    }
                    self.inst = inst
                }
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

/// `resources` の場所の URI（またはそのままのパス）をパスの文字列にする。
fn resource_path(location: &str) -> &str {
    let p = location
        .strip_prefix("file:")
        .map_or(location, |p| p.strip_prefix("//").unwrap_or(p));
    // Windows の `/C:/x` は `C:/x`
    match p.as_bytes() {
        [b'/', d, b':', ..] if d.is_ascii_alphabetic() => &p[1..],
        _ => p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #51: Integer・Boolean の入力の start は XSD の型（xs:int・xs:boolean）で書く（`2.0` ではなく `2`、`true`）。
    #[test]
    fn integer_and_boolean_inputs_start_in_their_own_types() {
        let src = "model M\n  input Integer n(start = 2);\n  input Boolean b(start = true);\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend M;\n";
        for xml in [
            model_description("M", src).unwrap(),
            model_description_3("M", src).unwrap(),
        ] {
            assert!(
                xml.contains("start=\"2\"") && xml.contains("start=\"true\""),
                "{xml}"
            );
        }
    }

    #[test]
    fn resource_path_from_uri() {
        assert_eq!(resource_path("file:///home/u/r"), "/home/u/r");
        assert_eq!(resource_path("/home/u/r"), "/home/u/r");
        // Windows: ドライブ文字の前の / を落とす（FMPy は file:///C:/… を渡す）
        assert_eq!(resource_path("file:///C:/Users/u/r"), "C:/Users/u/r");
        assert_eq!(resource_path("file:///c:/r"), "c:/r");
        // 3.0 の resourcePath はそのままのパス
        assert_eq!(resource_path(r"C:\Users\u\r"), r"C:\Users\u\r");
    }
}

/// `resources` の場所（2.0 は `file:///…` の URI、3.0 はそのままのパス）から、ただ 1 つの `.mo` の
/// （完全修飾名、文面）を読む。
// shortcut: %xx のエスケープを戻さない。空白などを含む置き場で使うときに戻す
fn read_resource(location: &str) -> Option<(String, String)> {
    let dir = PathBuf::from(resource_path(location));
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
    if fmu_type != MODEL_EXCHANGE {
        return std::ptr::null_mut();
    }
    unsafe { instantiate(guid, resource_location) }
}

/// `fmi2Instantiate`・`fmi3InstantiateModelExchange` の中身。
unsafe fn instantiate(guid: *const c_char, resource_location: *const c_char) -> Component {
    if guid.is_null() || resource_location.is_null() {
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
            inputs: BTreeMap::new(),
            states: BTreeMap::new(),
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
        m.inputs.clear();
        m.states.clear();
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
            } else if let Some((name, p)) = m.inst.parameters().get(r - 2 * n) {
                Some(*m.overrides.get(name).unwrap_or(p))
            } else {
                variable(m, r, ValueType::Real)
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
                let ok = m.inst.set_states(&x).is_ok();
                if ok {
                    m.states.insert(r, *v);
                }
                return ok;
            }
            let p = m.inst.parameters().len();
            if let Some(var) = (r.checked_sub(2 * n + p)).and_then(|i| m.inst.variables().nth(i)) {
                let name = var.name.clone();
                let ok = var.role == VariableRole::Input && m.inst.set_input(&name, *v).is_ok();
                if ok {
                    m.inputs.insert(name, *v);
                }
                return ok;
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

/// 値参照 `r`（`2n+p..`）の代数・離散の変数の今の値。型が `ty` でなければ None。
// shortcut: 値参照ごとに解き直す（nvr 個なら nvr 回）。変数の多いモデルで遅ければ 1 回の呼び出しの中で使い回す
fn variable(m: &Instance, r: usize, ty: ValueType) -> Option<f64> {
    let i = r.checked_sub(2 * m.inst.n_states() + m.inst.parameters().len())?;
    // その後ろは 3.0 の EventIndicator の変数と time（S137）
    if let Some(k) = i.checked_sub(m.inst.variables().count()) {
        if ty != ValueType::Real {
            return None;
        }
        if k == m.inst.n_event_indicators() {
            return Some(m.inst.time());
        }
        return m.inst.event_indicators().ok()?.get(k).copied();
    }
    if m.inst.variables().nth(i)?.value_type != ty {
        return None;
    }
    match m.inst.variable_values() {
        Ok(v) => Some(v[i]),
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            None
        }
    }
}

/// String の変数と、Integer・Boolean の書き込みは無い（`nvr = 0` だけ OK）。
fn no_variables(nvr: usize) -> Status {
    if nvr == 0 { OK } else { ERROR }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetInteger(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut c_int,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            // 値は整数の f64（Rumoca は離散の値も f64 で持つ）
            variable(m, r as usize, ValueType::Integer)
                .map(|x| *v = x.round() as c_int)
                .is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetBoolean(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut c_int,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            variable(m, r as usize, ValueType::Boolean)
                .map(|x| *v = c_int::from(x != 0.0))
                .is_some()
        })
    }
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

/// 今の時刻と状態を `pre` の値として控える。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterEventMode(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| match m.inst.enter_event_mode() {
        Ok(()) => OK,
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    })
}

/// `when` と `reinit` を回す。Rumoca は 1 回の呼び出しで事象の反復を収束させるので、
/// `newDiscreteStatesNeeded` はいつも偽。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2NewDiscreteStates(c: Component, info: *mut EventInfo) -> Status {
    let (Some(m), Some(info)) = (unsafe { inst(c) }, unsafe { info.as_mut() }) else {
        return ERROR;
    };
    match m.inst.update_discrete_states() {
        Ok(u) => {
            *info = EventInfo {
                new_discrete_states_needed: 0,
                terminate_simulation: c_int::from(u.terminate),
                nominals_of_continuous_states_changed: 0,
                values_of_continuous_states_changed: c_int::from(u.states_changed),
                next_event_time_defined: c_int::from(u.next_event_time.is_some()),
                next_event_time: u.next_event_time.unwrap_or(0.0),
            };
            OK
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
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
pub unsafe extern "C" fn fmi2GetEventIndicators(c: Component, z: *mut f64, ni: usize) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if ni != m.inst.n_event_indicators() || (ni > 0 && z.is_null()) {
        return ERROR;
    }
    if ni == 0 {
        return OK;
    }
    match m.inst.event_indicators() {
        Ok(v) => {
            // SAFETY: 呼び手は ni 個の有効な配列を渡す
            unsafe { std::slice::from_raw_parts_mut(z, ni) }.copy_from_slice(&v);
            OK
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

// ---- FMI 3.0 の ME（S137）。2.0 の関数を形だけ変えて呼ぶ（fmi3Boolean は C の bool） ----

#[unsafe(no_mangle)]
pub extern "C" fn fmi3GetVersion() -> *const c_char {
    c"3.0".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetDebugLogging(
    _c: Component,
    _on: bool,
    _n: usize,
    _cats: *const *const c_char,
) -> Status {
    OK
}

/// resources の `.mo` を compile する（2.0 の `fmi2Instantiate` と同じ。`resourcePath` はそのままのパス）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3InstantiateModelExchange(
    _instance_name: *const c_char,
    instantiation_token: *const c_char,
    resource_path: *const c_char,
    _visible: bool,
    _logging_on: bool,
    _environment: *mut c_void,
    _log_message: *const c_void,
) -> Component {
    unsafe { instantiate(instantiation_token, resource_path) }
}

// CS・SE は無い。引数を読まずに NULL を返す（呼び手が引数を片付ける C の呼び出し規約なので、引数を宣言しなくてよい）
#[unsafe(no_mangle)]
pub extern "C" fn fmi3InstantiateCoSimulation() -> Component {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn fmi3InstantiateScheduledExecution() -> Component {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3FreeInstance(c: Component) {
    unsafe { fmi2FreeInstance(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterInitializationMode(
    c: Component,
    tol_defined: bool,
    tol: f64,
    start: f64,
    stop_defined: bool,
    stop: f64,
) -> Status {
    unsafe { fmi2SetupExperiment(c, tol_defined.into(), tol, start, stop_defined.into(), stop) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3ExitInitializationMode(c: Component) -> Status {
    unsafe { fmi2ExitInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterEventMode(c: Component) -> Status {
    unsafe { fmi2EnterEventMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3Terminate(c: Component) -> Status {
    unsafe { fmi2Terminate(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3Reset(c: Component) -> Status {
    unsafe { fmi2Reset(c) }
}

/// スカラーだけなので `nValues` は `nvr` と同じでなければ `fmi3Error`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetFloat64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut f64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2GetReal(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetFloat64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *const f64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2SetReal(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetInt32(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut i32,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2GetInteger(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetBoolean(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut bool,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe {
        each(c, vr, nvr, values, |m, r, v| {
            variable(m, r as usize, ValueType::Boolean)
                .map(|x| *v = x != 0.0)
                .is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3UpdateDiscreteStates(
    c: Component,
    discrete_states_need_update: *mut bool,
    terminate_simulation: *mut bool,
    nominals_changed: *mut bool,
    values_changed: *mut bool,
    next_event_time_defined: *mut bool,
    next_event_time: *mut f64,
) -> Status {
    let mut info = EventInfo {
        new_discrete_states_needed: 0,
        terminate_simulation: 0,
        nominals_of_continuous_states_changed: 0,
        values_of_continuous_states_changed: 0,
        next_event_time_defined: 0,
        next_event_time: 0.0,
    };
    let status = unsafe { fmi2NewDiscreteStates(c, &mut info) };
    // SAFETY: 呼び手は有効な（または NULL の）ポインタを渡す
    let outs = unsafe {
        [
            (
                discrete_states_need_update.as_mut(),
                info.new_discrete_states_needed,
            ),
            (terminate_simulation.as_mut(), info.terminate_simulation),
            (
                nominals_changed.as_mut(),
                info.nominals_of_continuous_states_changed,
            ),
            (
                values_changed.as_mut(),
                info.values_of_continuous_states_changed,
            ),
            (
                next_event_time_defined.as_mut(),
                info.next_event_time_defined,
            ),
        ]
    };
    for (p, v) in outs {
        if let Some(p) = p {
            *p = v != 0;
        }
    }
    if let Some(t) = unsafe { next_event_time.as_mut() } {
        *t = info.next_event_time;
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterContinuousTimeMode(c: Component) -> Status {
    unsafe { fmi2EnterContinuousTimeMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3CompletedIntegratorStep(
    c: Component,
    _no_set_prior: bool,
    enter_event_mode: *mut bool,
    terminate: *mut bool,
) -> Status {
    let (mut e, mut t) = (0, 0);
    let status = unsafe { fmi2CompletedIntegratorStep(c, 0, &mut e, &mut t) };
    match unsafe { (enter_event_mode.as_mut(), terminate.as_mut()) } {
        (Some(pe), Some(pt)) if status == OK => {
            (*pe, *pt) = (e != 0, t != 0);
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetTime(c: Component, time: f64) -> Status {
    unsafe { fmi2SetTime(c, time) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetContinuousStates(c: Component, x: *const f64, nx: usize) -> Status {
    unsafe { fmi2SetContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetContinuousStates(c: Component, x: *mut f64, nx: usize) -> Status {
    unsafe { fmi2GetContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetContinuousStateDerivatives(
    c: Component,
    dx: *mut f64,
    nx: usize,
) -> Status {
    unsafe { fmi2GetDerivatives(c, dx, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNominalsOfContinuousStates(
    c: Component,
    x: *mut f64,
    nx: usize,
) -> Status {
    unsafe { fmi2GetNominalsOfContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetEventIndicators(c: Component, z: *mut f64, ni: usize) -> Status {
    unsafe { fmi2GetEventIndicators(c, z, ni) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNumberOfEventIndicators(c: Component, n: *mut usize) -> Status {
    match unsafe { (inst(c), n.as_mut()) } {
        (Some(m), Some(n)) => {
            *n = m.inst.n_event_indicators();
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNumberOfContinuousStates(c: Component, n: *mut usize) -> Status {
    match unsafe { (inst(c), n.as_mut()) } {
        (Some(m), Some(n)) => {
            *n = m.inst.n_states();
            OK
        }
        _ => ERROR,
    }
}

/// 無い関数（ほかの型の値・FMUstate・方向微分・Clock・CS・SE）。引数を読まずに `fmi3Error` を返す
/// （呼び手が引数を片付ける C の呼び出し規約なので、引数を宣言しなくてよい）。
macro_rules! unsupported {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> Status {
            ERROR
        }
    )*};
}

unsupported!(
    fmi3GetFloat32,
    fmi3GetInt8,
    fmi3GetUInt8,
    fmi3GetInt16,
    fmi3GetUInt16,
    fmi3GetUInt32,
    fmi3GetInt64,
    fmi3GetUInt64,
    fmi3GetString,
    fmi3GetBinary,
    fmi3GetClock,
    fmi3SetFloat32,
    fmi3SetInt8,
    fmi3SetUInt8,
    fmi3SetInt16,
    fmi3SetUInt16,
    fmi3SetInt32,
    fmi3SetUInt32,
    fmi3SetInt64,
    fmi3SetUInt64,
    fmi3SetBoolean,
    fmi3SetString,
    fmi3SetBinary,
    fmi3SetClock,
    fmi3GetNumberOfVariableDependencies,
    fmi3GetVariableDependencies,
    fmi3GetFMUState,
    fmi3SetFMUState,
    fmi3FreeFMUState,
    fmi3SerializedFMUStateSize,
    fmi3SerializeFMUState,
    fmi3DeserializeFMUState,
    fmi3GetDirectionalDerivative,
    fmi3GetAdjointDerivative,
    fmi3EnterConfigurationMode,
    fmi3ExitConfigurationMode,
    fmi3GetIntervalDecimal,
    fmi3GetIntervalFraction,
    fmi3GetShiftDecimal,
    fmi3GetShiftFraction,
    fmi3SetIntervalDecimal,
    fmi3SetIntervalFraction,
    fmi3SetShiftDecimal,
    fmi3SetShiftFraction,
    fmi3EvaluateDiscreteStates,
    fmi3EnterStepMode,
    fmi3GetOutputDerivatives,
    fmi3DoStep,
    fmi3ActivateModelPartition,
);

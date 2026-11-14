//! FMI 2.0 Co-Simulation・Model Exchange の C ABI（案 A、自前）。型と関数の並びは `fmi2FunctionTypes.h` /
//! `fmi2TypesPlatform.h`（v2.0.5、2-Clause BSD。ヘッダは複製せず、型を手で書いた）を見て書いた。
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::marker::PhantomData;
use std::path::Path;

use libloading::Library;

use crate::{Error, EventInfo, Fmu, FmuCs, FmuMe, Result, StepInfo, check, file_uri};

type Component = *mut c_void;
type Status = c_int;
/// `fmi2Type`: `fmi2ModelExchange` = 0、`fmi2CoSimulation` = 1
const MODEL_EXCHANGE: c_int = 0;
const CO_SIMULATION: c_int = 1;

/// `fmi2CallbackFunctions`
#[repr(C)]
struct Callbacks {
    logger: unsafe extern "C" fn(*mut c_void, *const c_char, Status, *const c_char, *const c_char),
    allocate_memory: unsafe extern "C" fn(usize, usize) -> *mut c_void,
    free_memory: unsafe extern "C" fn(*mut c_void),
    step_finished: Option<unsafe extern "C" fn(*mut c_void, Status)>,
    component_environment: *mut c_void,
}

unsafe extern "C" {
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// 本物の `fmi2CallbackLogger` は可変長引数（`...`）だが、Rust の安定版では C の可変長の関数を定義できない。
/// 名前のついた 5 つの引数だけを受けて残りは無視する（x86-64 の System V・Win64・AArch64 のどれでも、名前のついた引数は
/// 通常の関数と同じ場所で渡るので、受け手が可変長引数を読まなければ安全。FMPy の ctypes も同じやり方）。
/// 書式 `message` は展開せず、Error / Fatal のときだけそのまま標準エラーに出す。
unsafe extern "C" fn logger(
    _env: *mut c_void,
    name: *const c_char,
    status: Status,
    category: *const c_char,
    message: *const c_char,
) {
    if status < 3 {
        return;
    }
    let text = |p: *const c_char| {
        if p.is_null() {
            String::new()
        } else {
            // SAFETY: FMI 2.0 は logger の文字列引数を、呼び出しの間だけ有効な NUL 終端の C 文字列と定める。NULL は上で外した
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };
    eprintln!("[fmu {}] {}: {}", text(name), text(category), text(message));
}
/// CS・ME に共通の関数（`fmi2FunctionTypes.h`）。
struct Api {
    instantiate: unsafe extern "C" fn(
        *const c_char,
        c_int,
        *const c_char,
        *const c_char,
        *const Callbacks,
        c_int,
        c_int,
    ) -> Component,
    free_instance: unsafe extern "C" fn(Component),
    setup_experiment: unsafe extern "C" fn(Component, c_int, f64, f64, c_int, f64) -> Status,
    enter_init: unsafe extern "C" fn(Component) -> Status,
    exit_init: unsafe extern "C" fn(Component) -> Status,
    terminate: unsafe extern "C" fn(Component) -> Status,
    get_real: unsafe extern "C" fn(Component, *const c_uint, usize, *mut f64) -> Status,
    set_real: unsafe extern "C" fn(Component, *const c_uint, usize, *const f64) -> Status,
}

/// `fmi2EventInfo`（`fmi2Boolean` は int）
#[repr(C)]
#[derive(Default)]
struct EventInfo2 {
    new_discrete_states_needed: c_int,
    terminate_simulation: c_int,
    nominals_of_continuous_states_changed: c_int,
    values_of_continuous_states_changed: c_int,
    next_event_time_defined: c_int,
    next_event_time: f64,
}

/// Model Exchange だけの関数。
struct MeApi {
    set_time: unsafe extern "C" fn(Component, f64) -> Status,
    set_continuous_states: unsafe extern "C" fn(Component, *const f64, usize) -> Status,
    get_continuous_states: unsafe extern "C" fn(Component, *mut f64, usize) -> Status,
    get_derivatives: unsafe extern "C" fn(Component, *mut f64, usize) -> Status,
    get_event_indicators: unsafe extern "C" fn(Component, *mut f64, usize) -> Status,
    get_nominals: unsafe extern "C" fn(Component, *mut f64, usize) -> Status,
    enter_event_mode: unsafe extern "C" fn(Component) -> Status,
    new_discrete_states: unsafe extern "C" fn(Component, *mut EventInfo2) -> Status,
    enter_continuous_time_mode: unsafe extern "C" fn(Component) -> Status,
    completed_integrator_step:
        unsafe extern "C" fn(Component, c_int, *mut c_int, *mut c_int) -> Status,
}

/// 共有ライブラリから関数を 1 つ引く。
///
/// # Safety
/// `T` は `name` の関数の実際の型（呼び出し規約と引数）と一致する関数ポインタ型であること。
/// 返したポインタは `lib` が生きている間だけ有効。
unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T> {
    // SAFETY: 呼び手が T の一致を約束する（上の Safety）
    let s = unsafe { lib.get::<T>(name.as_bytes()) }
        .map_err(|e| Error::Library(format!("{name}: {e}")))?;
    Ok(*s)
}

/// CS と ME が共有するインスタンス。drop で `Terminate` と `FreeInstance` を呼ぶ。
struct Common {
    api: Api,
    c: Component,
    // FMU が instantiate のあとも指すかもしれないので、インスタンスと同じ寿命で持つ
    _callbacks: Box<Callbacks>,
    // 関数ポインタが指すライブラリ。最後に drop される（Drop::drop のあとにフィールドが落ちる）
    lib: Library,
    /// ExitInitializationMode が成功したか。Terminate は初期化のあとにしか呼べない（#53）
    initialized: bool,
}

impl Common {
    fn new(fmu: &Fmu, lib_path: &Path, instance_name: &str, fmu_type: c_int) -> Result<Self> {
        // SAFETY: FMU の共有ライブラリを読み込む。初期化関数（コンストラクタ）が走るので、FMU は信頼されたものでなければならない
        // （呼び手の `Fmu::instantiate_cs` / `instantiate_me` の注意書きと、CLI の事前の警告）
        let lib = unsafe { Library::new(lib_path) }
            .map_err(|e| Error::Library(format!("{}: {e}", lib_path.display())))?;
        // SAFETY: 型は fmi2FunctionTypes.h の fmi2*TYPE と同じ並び（c_int は fmi2Boolean / fmi2Type / enum、c_uint は fmi2ValueReference）
        let api = unsafe {
            Api {
                instantiate: sym(&lib, "fmi2Instantiate")?,
                free_instance: sym(&lib, "fmi2FreeInstance")?,
                setup_experiment: sym(&lib, "fmi2SetupExperiment")?,
                enter_init: sym(&lib, "fmi2EnterInitializationMode")?,
                exit_init: sym(&lib, "fmi2ExitInitializationMode")?,
                terminate: sym(&lib, "fmi2Terminate")?,
                get_real: sym(&lib, "fmi2GetReal")?,
                set_real: sym(&lib, "fmi2SetReal")?,
            }
        };
        let callbacks = Box::new(Callbacks {
            logger,
            allocate_memory: calloc,
            free_memory: free,
            step_finished: None,
            component_environment: std::ptr::null_mut(),
        });
        let cstr =
            |s: &str| CString::new(s).map_err(|_| Error::Fmu(format!("NUL を含む文字列: {s:?}")));
        let name = cstr(instance_name)?;
        let guid = cstr(&fmu.description.instantiation_token)?;
        let resources = cstr(&file_uri(&fmu.root().join("resources")))?;
        // SAFETY: 3 つの C 文字列と callbacks は呼び出しの間生きている。callbacks はインスタンスと同じ寿命で持つ。
        // 失敗は NULL で返る
        let c = unsafe {
            (api.instantiate)(
                name.as_ptr(),
                fmu_type,
                guid.as_ptr(),
                resources.as_ptr(),
                &*callbacks,
                0,
                0,
            )
        };
        if c.is_null() {
            return Err(Error::Fmu("fmi2Instantiate が NULL を返した".into()));
        }
        Ok(Common {
            api,
            c,
            _callbacks: callbacks,
            lib,
            initialized: false,
        })
    }

    /// `SetupExperiment` → `EnterInitializationMode` → `ExitInitializationMode`。
    fn initialize(
        &mut self,
        start_time: f64,
        stop_time: Option<f64>,
        tolerance: Option<f64>,
    ) -> Result<()> {
        // SAFETY: self.c は fmi2Instantiate が返した有効なインスタンス（Drop まで解放しない）
        unsafe {
            check(
                "fmi2SetupExperiment",
                (self.api.setup_experiment)(
                    self.c,
                    tolerance.is_some().into(),
                    tolerance.unwrap_or(0.0),
                    start_time,
                    stop_time.is_some().into(),
                    stop_time.unwrap_or(0.0),
                ),
            )?;
            check("fmi2EnterInitializationMode", (self.api.enter_init)(self.c))?;
            check("fmi2ExitInitializationMode", (self.api.exit_init)(self.c))?;
        }
        self.initialized = true;
        Ok(())
    }

    fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
        let mut out = vec![0.0; vrs.len()];
        // SAFETY: vrs と out は同じ長さ nvr の有効な配列
        check("fmi2GetReal", unsafe {
            (self.api.get_real)(self.c, vrs.as_ptr(), vrs.len(), out.as_mut_ptr())
        })?;
        Ok(out)
    }

    fn set_real(&mut self, vrs: &[u32], values: &[f64]) -> Result<()> {
        if vrs.len() != values.len() {
            return Err(Error::Fmu("値参照と値の数が違う".into()));
        }
        // SAFETY: vrs と values は同じ長さ nvr の有効な配列
        check("fmi2SetReal", unsafe {
            (self.api.set_real)(self.c, vrs.as_ptr(), vrs.len(), values.as_ptr())
        })
    }

    /// `fmi2GetInteger` / `fmi2GetBoolean`（どちらも `fmi2Integer`・`fmi2Boolean` が int）。必要なときに引く
    fn get_vals(&mut self, name: &str, vrs: &[u32]) -> Result<Vec<c_int>> {
        // SAFETY: 型は fmi2FunctionTypes.h の fmi2GetIntegerTYPE / fmi2GetBooleanTYPE
        let f: unsafe extern "C" fn(Component, *const c_uint, usize, *mut c_int) -> Status =
            unsafe { sym(&self.lib, name)? };
        let mut out = vec![0; vrs.len()];
        // SAFETY: vrs と out は同じ長さ nvr の有効な配列
        check(name, unsafe {
            f(self.c, vrs.as_ptr(), vrs.len(), out.as_mut_ptr())
        })?;
        Ok(out)
    }

    fn set_vals(&mut self, name: &str, vrs: &[u32], values: &[c_int]) -> Result<()> {
        if vrs.len() != values.len() {
            return Err(Error::Fmu("値参照と値の数が違う".into()));
        }
        // SAFETY: 型は fmi2SetIntegerTYPE / fmi2SetBooleanTYPE
        let f: unsafe extern "C" fn(Component, *const c_uint, usize, *const c_int) -> Status =
            unsafe { sym(&self.lib, name)? };
        // SAFETY: vrs と values は同じ長さ nvr の有効な配列
        check(name, unsafe {
            f(self.c, vrs.as_ptr(), vrs.len(), values.as_ptr())
        })
    }
}

impl Drop for Common {
    fn drop(&mut self) {
        // SAFETY: self.c は有効で、ここで最後に使う。Terminate の失敗は後片付けの途中なので無視する。
        // 初期化の前（失敗を含む）は Terminate を呼ばない（状態の守りが弱い FMU が落ちるため）
        unsafe {
            if self.initialized {
                (self.api.terminate)(self.c);
            }
            (self.api.free_instance)(self.c);
        }
    }
}

pub(crate) struct Cs2<'a> {
    inner: Common,
    do_step: unsafe extern "C" fn(Component, f64, f64, c_int) -> Status,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Cs2<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        let inner = Common::new(fmu, lib_path, instance_name, CO_SIMULATION)?;
        // SAFETY: fmi2DoStepTYPE（fmi2Component, fmi2Real, fmi2Real, fmi2Boolean）-> fmi2Status
        let do_step = unsafe { sym(&inner.lib, "fmi2DoStep")? };
        Ok(Cs2 {
            inner,
            do_step,
            _fmu: PhantomData,
        })
    }
}

impl FmuCs for Cs2<'_> {
    fn initialize(&mut self, start_time: f64, stop_time: Option<f64>) -> Result<()> {
        self.inner.initialize(start_time, stop_time, None)
    }

    fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
        self.inner.get_real(vrs)
    }

    fn set_real(&mut self, vrs: &[u32], values: &[f64]) -> Result<()> {
        self.inner.set_real(vrs, values)
    }

    fn get_int(&mut self, vrs: &[u32]) -> Result<Vec<i32>> {
        self.inner.get_vals("fmi2GetInteger", vrs)
    }

    fn set_int(&mut self, vrs: &[u32], values: &[i32]) -> Result<()> {
        self.inner.set_vals("fmi2SetInteger", vrs, values)
    }

    fn get_bool(&mut self, vrs: &[u32]) -> Result<Vec<bool>> {
        Ok(self
            .inner
            .get_vals("fmi2GetBoolean", vrs)?
            .into_iter()
            .map(|v| v != 0)
            .collect())
    }

    fn set_bool(&mut self, vrs: &[u32], values: &[bool]) -> Result<()> {
        let v: Vec<c_int> = values.iter().map(|&b| c_int::from(b)).collect();
        self.inner.set_vals("fmi2SetBoolean", vrs, &v)
    }

    fn do_step(&mut self, time: f64, step: f64) -> Result<bool> {
        // SAFETY: 有効なインスタンス。noSetFMUStatePriorToCurrentPoint = true（状態を巻き戻さない）
        check("fmi2DoStep", unsafe {
            (self.do_step)(self.inner.c, time, step, 1)
        })?;
        Ok(false) // 2.0 の DoStep は打ち切りを返さない（fmi2GetStatus(Terminated) は未対応）
    }
}

pub(crate) struct Me2<'a> {
    inner: Common,
    me: MeApi,
    nx: usize,
    nz: usize,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Me2<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        let inner = Common::new(fmu, lib_path, instance_name, MODEL_EXCHANGE)?;
        let lib = &inner.lib;
        // SAFETY: 型は fmi2FunctionTypes.h の fmi2*TYPE と同じ並び（fmi2Real は f64、size_t は usize、fmi2Boolean は int）
        let me = unsafe {
            MeApi {
                set_time: sym(lib, "fmi2SetTime")?,
                set_continuous_states: sym(lib, "fmi2SetContinuousStates")?,
                get_continuous_states: sym(lib, "fmi2GetContinuousStates")?,
                get_derivatives: sym(lib, "fmi2GetDerivatives")?,
                get_event_indicators: sym(lib, "fmi2GetEventIndicators")?,
                get_nominals: sym(lib, "fmi2GetNominalsOfContinuousStates")?,
                enter_event_mode: sym(lib, "fmi2EnterEventMode")?,
                new_discrete_states: sym(lib, "fmi2NewDiscreteStates")?,
                enter_continuous_time_mode: sym(lib, "fmi2EnterContinuousTimeMode")?,
                completed_integrator_step: sym(lib, "fmi2CompletedIntegratorStep")?,
            }
        };
        Ok(Me2 {
            inner,
            me,
            // 2.0 の状態の数は `ModelStructure/Derivatives` の数、event indicator の数は `numberOfEventIndicators`
            nx: fmu.description.model_structure.derivatives.len(),
            nz: fmu.description.number_of_event_indicators as usize,
            _fmu: PhantomData,
        })
    }

    fn vec(
        &mut self,
        what: &str,
        n: usize,
        f: unsafe extern "C" fn(Component, *mut f64, usize) -> Status,
    ) -> Result<Vec<f64>> {
        let mut out = vec![0.0; n];
        // SAFETY: out は長さ n の有効な配列で、f の第 3 引数も n（nx / ni）
        check(what, unsafe { f(self.inner.c, out.as_mut_ptr(), n) })?;
        Ok(out)
    }
}

impl FmuMe for Me2<'_> {
    fn n_states(&self) -> usize {
        self.nx
    }

    fn n_event_indicators(&self) -> usize {
        self.nz
    }

    fn initialize(
        &mut self,
        start_time: f64,
        stop_time: Option<f64>,
        tolerance: Option<f64>,
    ) -> Result<()> {
        // ExitInitializationMode でそのまま Event Mode に入る（仕様 2.0 の ME の状態遷移）
        self.inner.initialize(start_time, stop_time, tolerance)
    }

    fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
        self.inner.get_real(vrs)
    }

    fn set_real(&mut self, vrs: &[u32], values: &[f64]) -> Result<()> {
        self.inner.set_real(vrs, values)
    }

    fn set_time(&mut self, time: f64) -> Result<()> {
        // SAFETY: 有効なインスタンス
        check("fmi2SetTime", unsafe {
            (self.me.set_time)(self.inner.c, time)
        })
    }

    fn get_continuous_states(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi2GetContinuousStates",
            self.nx,
            self.me.get_continuous_states,
        )
    }

    fn set_continuous_states(&mut self, x: &[f64]) -> Result<()> {
        if x.len() != self.nx {
            return Err(Error::Fmu("連続状態の数が違う".into()));
        }
        // SAFETY: x は長さ nx の有効な配列
        check("fmi2SetContinuousStates", unsafe {
            (self.me.set_continuous_states)(self.inner.c, x.as_ptr(), x.len())
        })
    }

    fn get_nominals(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi2GetNominalsOfContinuousStates",
            self.nx,
            self.me.get_nominals,
        )
    }

    fn get_derivatives(&mut self) -> Result<Vec<f64>> {
        self.vec("fmi2GetDerivatives", self.nx, self.me.get_derivatives)
    }

    fn get_event_indicators(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi2GetEventIndicators",
            self.nz,
            self.me.get_event_indicators,
        )
    }

    fn enter_event_mode(&mut self) -> Result<()> {
        // SAFETY: 有効なインスタンス
        check("fmi2EnterEventMode", unsafe {
            (self.me.enter_event_mode)(self.inner.c)
        })
    }

    fn update_discrete_states(&mut self) -> Result<EventInfo> {
        let mut info = EventInfo2::default();
        // SAFETY: info は fmi2EventInfo と同じ配置（repr(C)）の有効な出力先
        check("fmi2NewDiscreteStates", unsafe {
            (self.me.new_discrete_states)(self.inner.c, &mut info)
        })?;
        Ok(EventInfo {
            discrete_states_need_update: info.new_discrete_states_needed != 0,
            terminate: info.terminate_simulation != 0,
            nominals_changed: info.nominals_of_continuous_states_changed != 0,
            values_changed: info.values_of_continuous_states_changed != 0,
            next_event_time: (info.next_event_time_defined != 0).then_some(info.next_event_time),
        })
    }

    fn enter_continuous_time_mode(&mut self) -> Result<()> {
        // SAFETY: 有効なインスタンス
        check("fmi2EnterContinuousTimeMode", unsafe {
            (self.me.enter_continuous_time_mode)(self.inner.c)
        })
    }

    fn completed_integrator_step(&mut self) -> Result<StepInfo> {
        let (mut enter, mut terminate) = (0, 0);
        // SAFETY: 有効なインスタンスと 2 つの有効な出力先。noSetFMUStatePriorToCurrentPoint = true（FMU の状態を巻き戻さない）
        check("fmi2CompletedIntegratorStep", unsafe {
            (self.me.completed_integrator_step)(self.inner.c, 1, &mut enter, &mut terminate)
        })?;
        Ok(StepInfo {
            enter_event_mode: enter != 0,
            terminate: terminate != 0,
        })
    }
}

impl std::fmt::Debug for Cs2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cs2")
    }
}

impl std::fmt::Debug for Me2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Me2")
    }
}

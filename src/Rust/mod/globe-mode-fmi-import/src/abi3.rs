//! FMI 3.0 Co-Simulation・Model Exchange の C ABI（案 A、自前）。型は `fmi3FunctionTypes.h` / `fmi3PlatformTypes.h`
//! （v3.0.2、2-Clause BSD。ヘッダは複製せず、型を手で書いた）を見て書いた。
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::marker::PhantomData;
use std::path::Path;

use libloading::Library;

use crate::{Error, EventInfo, Fmu, FmuCs, FmuMe, Result, StepInfo, check};

type Instance = *mut c_void;
type Status = c_int;

/// `fmi3LogMessageCallback`（3.0 は可変長引数ではない）。Error / Fatal だけ標準エラーに出す。
unsafe extern "C" fn log_message(
    _env: *mut c_void,
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
            // SAFETY: FMI 3.0 は logMessage の文字列引数を、呼び出しの間だけ有効な NUL 終端の C 文字列と定める。NULL は上で外した
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };
    eprintln!("[fmu] {}: {}", text(category), text(message));
}

/// CS・ME に共通の関数（`fmi3FunctionTypes.h`）。インスタンスを作る関数だけが違うので、ここには含めない。
struct Api {
    free_instance: unsafe extern "C" fn(Instance),
    enter_init: unsafe extern "C" fn(Instance, bool, f64, f64, bool, f64) -> Status,
    exit_init: unsafe extern "C" fn(Instance) -> Status,
    terminate: unsafe extern "C" fn(Instance) -> Status,
    get_float64: unsafe extern "C" fn(Instance, *const u32, usize, *mut f64, usize) -> Status,
    set_float64: unsafe extern "C" fn(Instance, *const u32, usize, *const f64, usize) -> Status,
}

/// Model Exchange だけの関数。
struct MeApi {
    set_time: unsafe extern "C" fn(Instance, f64) -> Status,
    set_continuous_states: unsafe extern "C" fn(Instance, *const f64, usize) -> Status,
    get_continuous_states: unsafe extern "C" fn(Instance, *mut f64, usize) -> Status,
    get_derivatives: unsafe extern "C" fn(Instance, *mut f64, usize) -> Status,
    get_event_indicators: unsafe extern "C" fn(Instance, *mut f64, usize) -> Status,
    get_nominals: unsafe extern "C" fn(Instance, *mut f64, usize) -> Status,
    enter_event_mode: unsafe extern "C" fn(Instance) -> Status,
    #[allow(clippy::type_complexity)]
    update_discrete_states: unsafe extern "C" fn(
        Instance,
        *mut bool, // discreteStatesNeedUpdate
        *mut bool, // terminateSimulation
        *mut bool, // nominalsOfContinuousStatesChanged
        *mut bool, // valuesOfContinuousStatesChanged
        *mut bool, // nextEventTimeDefined
        *mut f64,  // nextEventTime
    ) -> Status,
    enter_continuous_time_mode: unsafe extern "C" fn(Instance) -> Status,
    completed_integrator_step: unsafe extern "C" fn(Instance, bool, *mut bool, *mut bool) -> Status,
}

/// # Safety
/// `T` は `name` の関数の実際の型と一致する関数ポインタ型であること。返したポインタは `lib` が生きている間だけ有効。
unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T> {
    // SAFETY: 呼び手が T の一致を約束する
    let s = unsafe { lib.get::<T>(name.as_bytes()) }
        .map_err(|e| Error::Library(format!("{name}: {e}")))?;
    Ok(*s)
}

/// CS と ME が共有するインスタンス。drop で `Terminate` と `FreeInstance` を呼ぶ。
struct Common {
    api: Api,
    c: Instance,
    // 関数ポインタが指すライブラリ。最後に drop される
    lib: Library,
}

impl Common {
    /// ライブラリを読み、共通の関数を引く。インスタンスはまだ作らない（`finish` で作る）。
    fn load(lib_path: &Path) -> Result<(Library, Api)> {
        // SAFETY: FMU の共有ライブラリを読み込む。初期化関数（コンストラクタ）が走るので、FMU は信頼されたものでなければならない
        // （呼び手の `Fmu::instantiate_cs` / `instantiate_me` の注意書きと、CLI の事前の警告）
        let lib = unsafe { Library::new(lib_path) }
            .map_err(|e| Error::Library(format!("{}: {e}", lib_path.display())))?;
        // SAFETY: 型は fmi3FunctionTypes.h の fmi3*TYPE と同じ並び（bool は fmi3Boolean＝C の bool、u32 は fmi3ValueReference）
        let api = unsafe {
            Api {
                free_instance: sym(&lib, "fmi3FreeInstance")?,
                enter_init: sym(&lib, "fmi3EnterInitializationMode")?,
                exit_init: sym(&lib, "fmi3ExitInitializationMode")?,
                terminate: sym(&lib, "fmi3Terminate")?,
                get_float64: sym(&lib, "fmi3GetFloat64")?,
                set_float64: sym(&lib, "fmi3SetFloat64")?,
            }
        };
        Ok((lib, api))
    }

    fn initialize(
        &mut self,
        start_time: f64,
        stop_time: Option<f64>,
        tolerance: Option<f64>,
    ) -> Result<()> {
        // SAFETY: self.c は有効なインスタンス（Drop まで解放しない）
        unsafe {
            check(
                "fmi3EnterInitializationMode",
                (self.api.enter_init)(
                    self.c,
                    tolerance.is_some(),
                    tolerance.unwrap_or(0.0),
                    start_time,
                    stop_time.is_some(),
                    stop_time.unwrap_or(0.0),
                ),
            )?;
            check("fmi3ExitInitializationMode", (self.api.exit_init)(self.c))
        }
    }

    fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
        let mut out = vec![0.0; vrs.len()];
        // SAFETY: vrs と out は同じ長さ（nValueReferences = nValues = len）の有効な配列
        check("fmi3GetFloat64", unsafe {
            (self.api.get_float64)(self.c, vrs.as_ptr(), vrs.len(), out.as_mut_ptr(), out.len())
        })?;
        Ok(out)
    }

    fn set_real(&mut self, vrs: &[u32], values: &[f64]) -> Result<()> {
        if vrs.len() != values.len() {
            return Err(Error::Fmu("値参照と値の数が違う".into()));
        }
        // SAFETY: vrs と values は同じ長さの有効な配列（スカラの変数だけ。配列の変数は未対応）
        check("fmi3SetFloat64", unsafe {
            (self.api.set_float64)(
                self.c,
                vrs.as_ptr(),
                vrs.len(),
                values.as_ptr(),
                values.len(),
            )
        })
    }
}

impl Drop for Common {
    fn drop(&mut self) {
        // SAFETY: self.c は有効で、ここで最後に使う。Terminate の失敗は後片付けの途中なので無視する
        unsafe {
            (self.api.terminate)(self.c);
            (self.api.free_instance)(self.c);
        }
    }
}

/// 3.0 の `resourcePath` は URI でなく、`resources` ディレクトリの絶対パス（末尾に区切り）。
fn resource_path(fmu: &Fmu) -> String {
    let mut res = fmu.root().join("resources").to_string_lossy().into_owned();
    res.push(std::path::MAIN_SEPARATOR);
    res
}

fn cstr(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::Fmu(format!("NUL を含む文字列: {s:?}")))
}

pub(crate) struct Cs3<'a> {
    inner: Common,
    do_step: unsafe extern "C" fn(
        Instance,
        f64,
        f64,
        bool,
        *mut bool,
        *mut bool,
        *mut bool,
        *mut f64,
    ) -> Status,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Cs3<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        let (lib, api) = Common::load(lib_path)?;
        #[allow(clippy::type_complexity)]
        // SAFETY: 型は fmi3InstantiateCoSimulationTYPE / fmi3DoStepTYPE と同じ並び
        let (instantiate, do_step): (
            unsafe extern "C" fn(
                *const c_char, // instanceName
                *const c_char, // instantiationToken
                *const c_char, // resourcePath
                bool,          // visible
                bool,          // loggingOn
                bool,          // eventModeUsed
                bool,          // earlyReturnAllowed
                *const u32,    // requiredIntermediateVariables
                usize,         // nRequiredIntermediateVariables
                *mut c_void,   // instanceEnvironment
                Option<unsafe extern "C" fn(*mut c_void, Status, *const c_char, *const c_char)>, // logMessage
                *const c_void, // intermediateUpdate（NULL）
            ) -> Instance,
            _,
        ) = unsafe {
            (
                sym(&lib, "fmi3InstantiateCoSimulation")?,
                sym(&lib, "fmi3DoStep")?,
            )
        };
        let name = cstr(instance_name)?;
        let token = cstr(&fmu.description.instantiation_token)?;
        let resources = cstr(&resource_path(fmu))?;
        // SAFETY: 3 つの C 文字列は呼び出しの間生きている。必要な中間変数は 0 個（NULL, 0）、intermediateUpdate は NULL
        // （earlyReturnAllowed = false なので呼ばれない）、eventModeUsed = false（事象は FMU の中で処理される）。失敗は NULL
        let c = unsafe {
            instantiate(
                name.as_ptr(),
                token.as_ptr(),
                resources.as_ptr(),
                false,
                false,
                false,
                false,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                Some(log_message),
                std::ptr::null(),
            )
        };
        if c.is_null() {
            return Err(Error::Fmu(
                "fmi3InstantiateCoSimulation が NULL を返した".into(),
            ));
        }
        Ok(Cs3 {
            inner: Common { api, c, lib },
            do_step,
            _fmu: PhantomData,
        })
    }
}

impl FmuCs for Cs3<'_> {
    fn initialize(&mut self, start_time: f64, stop_time: Option<f64>) -> Result<()> {
        self.inner.initialize(start_time, stop_time, None)
    }

    fn get_real(&mut self, vrs: &[u32]) -> Result<Vec<f64>> {
        self.inner.get_real(vrs)
    }

    fn set_real(&mut self, vrs: &[u32], values: &[f64]) -> Result<()> {
        self.inner.set_real(vrs, values)
    }

    fn do_step(&mut self, time: f64, step: f64) -> Result<bool> {
        let (mut event, mut terminate, mut early, mut last) = (false, false, false, 0.0);
        // SAFETY: 有効なインスタンスと、有効な 4 つの出力先（ローカル変数）
        check("fmi3DoStep", unsafe {
            (self.do_step)(
                self.inner.c,
                time,
                step,
                true,
                &mut event,
                &mut terminate,
                &mut early,
                &mut last,
            )
        })?;
        Ok(terminate)
    }
}

pub(crate) struct Me3<'a> {
    inner: Common,
    me: MeApi,
    nx: usize,
    nz: usize,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Me3<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        let (lib, api) = Common::load(lib_path)?;
        #[allow(clippy::type_complexity)]
        // SAFETY: 型は fmi3FunctionTypes.h の fmi3*TYPE と同じ並び（fmi3Float64 は f64、size_t は usize、fmi3Boolean は bool）
        let (instantiate, me): (
            unsafe extern "C" fn(
                *const c_char, // instanceName
                *const c_char, // instantiationToken
                *const c_char, // resourcePath
                bool,          // visible
                bool,          // loggingOn
                *mut c_void,   // instanceEnvironment
                Option<unsafe extern "C" fn(*mut c_void, Status, *const c_char, *const c_char)>, // logMessage
            ) -> Instance,
            MeApi,
        ) = unsafe {
            (
                sym(&lib, "fmi3InstantiateModelExchange")?,
                MeApi {
                    set_time: sym(&lib, "fmi3SetTime")?,
                    set_continuous_states: sym(&lib, "fmi3SetContinuousStates")?,
                    get_continuous_states: sym(&lib, "fmi3GetContinuousStates")?,
                    get_derivatives: sym(&lib, "fmi3GetContinuousStateDerivatives")?,
                    get_event_indicators: sym(&lib, "fmi3GetEventIndicators")?,
                    get_nominals: sym(&lib, "fmi3GetNominalsOfContinuousStates")?,
                    enter_event_mode: sym(&lib, "fmi3EnterEventMode")?,
                    update_discrete_states: sym(&lib, "fmi3UpdateDiscreteStates")?,
                    enter_continuous_time_mode: sym(&lib, "fmi3EnterContinuousTimeMode")?,
                    completed_integrator_step: sym(&lib, "fmi3CompletedIntegratorStep")?,
                },
            )
        };
        let name = cstr(instance_name)?;
        let token = cstr(&fmu.description.instantiation_token)?;
        let resources = cstr(&resource_path(fmu))?;
        // SAFETY: 3 つの C 文字列は呼び出しの間生きている。instanceEnvironment は NULL（logMessage は使わない）。失敗は NULL
        let c = unsafe {
            instantiate(
                name.as_ptr(),
                token.as_ptr(),
                resources.as_ptr(),
                false,
                false,
                std::ptr::null_mut(),
                Some(log_message),
            )
        };
        if c.is_null() {
            return Err(Error::Fmu(
                "fmi3InstantiateModelExchange が NULL を返した".into(),
            ));
        }
        let mut inst = Me3 {
            inner: Common { api, c, lib },
            me,
            nx: 0,
            nz: 0,
            _fmu: PhantomData,
        };
        // 数は FMU に問う（3.0 には `fmi3GetNumberOfContinuousStates` / `…EventIndicators` がある）
        // SAFETY: 型は fmi3GetNumberOf*TYPE（fmi3Instance, size_t*）-> fmi3Status。有効な出力先
        unsafe {
            let n_states: unsafe extern "C" fn(Instance, *mut usize) -> Status =
                sym(&inst.inner.lib, "fmi3GetNumberOfContinuousStates")?;
            let n_events: unsafe extern "C" fn(Instance, *mut usize) -> Status =
                sym(&inst.inner.lib, "fmi3GetNumberOfEventIndicators")?;
            check("fmi3GetNumberOfContinuousStates", n_states(c, &mut inst.nx))?;
            check("fmi3GetNumberOfEventIndicators", n_events(c, &mut inst.nz))?;
        }
        Ok(inst)
    }

    fn vec(
        &mut self,
        what: &str,
        n: usize,
        f: unsafe extern "C" fn(Instance, *mut f64, usize) -> Status,
    ) -> Result<Vec<f64>> {
        let mut out = vec![0.0; n];
        // SAFETY: out は長さ n の有効な配列で、f の第 3 引数も n
        check(what, unsafe { f(self.inner.c, out.as_mut_ptr(), n) })?;
        Ok(out)
    }
}

impl FmuMe for Me3<'_> {
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
        // ExitInitializationMode でそのまま Event Mode に入る（仕様 3.0 の ME）
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
        check("fmi3SetTime", unsafe {
            (self.me.set_time)(self.inner.c, time)
        })
    }

    fn get_continuous_states(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi3GetContinuousStates",
            self.nx,
            self.me.get_continuous_states,
        )
    }

    fn set_continuous_states(&mut self, x: &[f64]) -> Result<()> {
        if x.len() != self.nx {
            return Err(Error::Fmu("連続状態の数が違う".into()));
        }
        // SAFETY: x は長さ nx の有効な配列
        check("fmi3SetContinuousStates", unsafe {
            (self.me.set_continuous_states)(self.inner.c, x.as_ptr(), x.len())
        })
    }

    fn get_nominals(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi3GetNominalsOfContinuousStates",
            self.nx,
            self.me.get_nominals,
        )
    }

    fn get_derivatives(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi3GetContinuousStateDerivatives",
            self.nx,
            self.me.get_derivatives,
        )
    }

    fn get_event_indicators(&mut self) -> Result<Vec<f64>> {
        self.vec(
            "fmi3GetEventIndicators",
            self.nz,
            self.me.get_event_indicators,
        )
    }

    fn enter_event_mode(&mut self) -> Result<()> {
        // SAFETY: 有効なインスタンス
        check("fmi3EnterEventMode", unsafe {
            (self.me.enter_event_mode)(self.inner.c)
        })
    }

    fn update_discrete_states(&mut self) -> Result<EventInfo> {
        let (mut need, mut terminate, mut nominals, mut values, mut defined) =
            (false, false, false, false, false);
        let mut next = 0.0;
        // SAFETY: 有効なインスタンスと、有効な 6 つの出力先（ローカル変数）
        check("fmi3UpdateDiscreteStates", unsafe {
            (self.me.update_discrete_states)(
                self.inner.c,
                &mut need,
                &mut terminate,
                &mut nominals,
                &mut values,
                &mut defined,
                &mut next,
            )
        })?;
        Ok(EventInfo {
            discrete_states_need_update: need,
            terminate,
            nominals_changed: nominals,
            values_changed: values,
            next_event_time: defined.then_some(next),
        })
    }

    fn enter_continuous_time_mode(&mut self) -> Result<()> {
        // SAFETY: 有効なインスタンス
        check("fmi3EnterContinuousTimeMode", unsafe {
            (self.me.enter_continuous_time_mode)(self.inner.c)
        })
    }

    fn completed_integrator_step(&mut self) -> Result<StepInfo> {
        let (mut enter, mut terminate) = (false, false);
        // SAFETY: 有効なインスタンスと 2 つの有効な出力先。noSetFMUStatePriorToCurrentPoint = true（FMU の状態を巻き戻さない）
        check("fmi3CompletedIntegratorStep", unsafe {
            (self.me.completed_integrator_step)(self.inner.c, true, &mut enter, &mut terminate)
        })?;
        Ok(StepInfo {
            enter_event_mode: enter,
            terminate,
        })
    }
}

impl std::fmt::Debug for Cs3<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cs3")
    }
}

impl std::fmt::Debug for Me3<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Me3")
    }
}

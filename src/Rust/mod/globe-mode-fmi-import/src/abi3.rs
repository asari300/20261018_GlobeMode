//! FMI 3.0 Co-Simulation の C ABI（案 A、自前）。型は `fmi3FunctionTypes.h` / `fmi3PlatformTypes.h`
//! （v3.0.2、2-Clause BSD。ヘッダは複製せず、型を手で書いた）を見て書いた。
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::marker::PhantomData;
use std::path::Path;

use libloading::Library;

use crate::{Error, Fmu, FmuCs, Result, check};

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

struct Api {
    #[allow(clippy::type_complexity)]
    instantiate: unsafe extern "C" fn(
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
    free_instance: unsafe extern "C" fn(Instance),
    enter_init: unsafe extern "C" fn(Instance, bool, f64, f64, bool, f64) -> Status,
    exit_init: unsafe extern "C" fn(Instance) -> Status,
    terminate: unsafe extern "C" fn(Instance) -> Status,
    get_float64: unsafe extern "C" fn(Instance, *const u32, usize, *mut f64, usize) -> Status,
    set_float64: unsafe extern "C" fn(Instance, *const u32, usize, *const f64, usize) -> Status,
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
}

/// # Safety
/// `T` は `name` の関数の実際の型と一致する関数ポインタ型であること。返したポインタは `lib` が生きている間だけ有効。
unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T> {
    // SAFETY: 呼び手が T の一致を約束する
    let s = unsafe { lib.get::<T>(name.as_bytes()) }
        .map_err(|e| Error::Library(format!("{name}: {e}")))?;
    Ok(*s)
}

pub(crate) struct Cs3<'a> {
    api: Api,
    c: Instance,
    _lib: Library,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Cs3<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        // SAFETY: FMU の共有ライブラリを読み込む（abi2 と同じ。FMU は信頼されたものでなければならない）
        let lib = unsafe { Library::new(lib_path) }
            .map_err(|e| Error::Library(format!("{}: {e}", lib_path.display())))?;
        // SAFETY: 型は fmi3FunctionTypes.h の fmi3*TYPE と同じ並び（bool は fmi3Boolean＝C の bool、u32 は fmi3ValueReference）
        let api = unsafe {
            Api {
                instantiate: sym(&lib, "fmi3InstantiateCoSimulation")?,
                free_instance: sym(&lib, "fmi3FreeInstance")?,
                enter_init: sym(&lib, "fmi3EnterInitializationMode")?,
                exit_init: sym(&lib, "fmi3ExitInitializationMode")?,
                terminate: sym(&lib, "fmi3Terminate")?,
                get_float64: sym(&lib, "fmi3GetFloat64")?,
                set_float64: sym(&lib, "fmi3SetFloat64")?,
                do_step: sym(&lib, "fmi3DoStep")?,
            }
        };
        let cstr =
            |s: &str| CString::new(s).map_err(|_| Error::Fmu(format!("NUL を含む文字列: {s:?}")));
        let name = cstr(instance_name)?;
        let token = cstr(&fmu.description.instantiation_token)?;
        // 3.0 の resourcePath は URI でなく、`resources` ディレクトリの絶対パス（末尾に区切り）
        let mut res = fmu.root().join("resources").to_string_lossy().into_owned();
        res.push(std::path::MAIN_SEPARATOR);
        let resources = cstr(&res)?;
        // SAFETY: 3 つの C 文字列は呼び出しの間生きている。必要な中間変数は 0 個（NULL, 0）、intermediateUpdate は NULL
        // （earlyReturnAllowed = false なので呼ばれない）、eventModeUsed = false（事象は FMU の中で処理される）。失敗は NULL
        let c = unsafe {
            (api.instantiate)(
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
            api,
            c,
            _lib: lib,
            _fmu: PhantomData,
        })
    }
}

impl FmuCs for Cs3<'_> {
    fn initialize(&mut self, start_time: f64, stop_time: Option<f64>) -> Result<()> {
        // SAFETY: self.c は有効なインスタンス（Drop まで解放しない）
        unsafe {
            check(
                "fmi3EnterInitializationMode",
                (self.api.enter_init)(
                    self.c,
                    false,
                    0.0,
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

    fn do_step(&mut self, time: f64, step: f64) -> Result<bool> {
        let (mut event, mut terminate, mut early, mut last) = (false, false, false, 0.0);
        // SAFETY: 有効なインスタンスと、有効な 4 つの出力先（ローカル変数）
        check("fmi3DoStep", unsafe {
            (self.api.do_step)(
                self.c,
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

impl Drop for Cs3<'_> {
    fn drop(&mut self) {
        // SAFETY: self.c は有効で、ここで最後に使う。Terminate の失敗は後片付けの途中なので無視する
        unsafe {
            (self.api.terminate)(self.c);
            (self.api.free_instance)(self.c);
        }
    }
}

impl std::fmt::Debug for Cs3<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cs3")
    }
}

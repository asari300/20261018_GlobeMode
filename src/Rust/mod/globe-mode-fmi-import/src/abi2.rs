//! FMI 2.0 Co-Simulation の C ABI（案 A、自前）。型と関数の並びは `fmi2FunctionTypes.h` / `fmi2TypesPlatform.h`
//! （v2.0.5、2-Clause BSD。ヘッダは複製せず、型を手で書いた）を見て書いた。
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::marker::PhantomData;
use std::path::Path;

use libloading::Library;

use crate::{Error, Fmu, FmuCs, Result, check, file_uri};

type Component = *mut c_void;
type Status = c_int;
/// `fmi2CoSimulation`（`fmi2Type` の 2 番目）
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
    do_step: unsafe extern "C" fn(Component, f64, f64, c_int) -> Status,
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

pub(crate) struct Cs2<'a> {
    api: Api,
    c: Component,
    // FMU が instantiate のあとも指すかもしれないので、インスタンスと同じ寿命で持つ
    _callbacks: Box<Callbacks>,
    // 関数ポインタが指すライブラリ。最後に drop される（Drop::drop のあとにフィールドが落ちる）
    _lib: Library,
    _fmu: PhantomData<&'a Fmu>,
}

impl<'a> Cs2<'a> {
    pub(crate) fn new(fmu: &'a Fmu, lib_path: &Path, instance_name: &str) -> Result<Self> {
        // SAFETY: FMU の共有ライブラリを読み込む。初期化関数（コンストラクタ）が走るので、FMU は信頼されたものでなければならない
        // （呼び手の `Fmu::instantiate_cs` の注意書きと、CLI の事前の警告）
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
                do_step: sym(&lib, "fmi2DoStep")?,
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
                CO_SIMULATION,
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
        Ok(Cs2 {
            api,
            c,
            _callbacks: callbacks,
            _lib: lib,
            _fmu: PhantomData,
        })
    }
}

impl FmuCs for Cs2<'_> {
    fn initialize(&mut self, start_time: f64, stop_time: Option<f64>) -> Result<()> {
        // SAFETY: self.c は fmi2Instantiate が返した有効なインスタンス（Drop まで解放しない）
        unsafe {
            check(
                "fmi2SetupExperiment",
                (self.api.setup_experiment)(
                    self.c,
                    0,
                    0.0,
                    start_time,
                    stop_time.is_some().into(),
                    stop_time.unwrap_or(0.0),
                ),
            )?;
            check("fmi2EnterInitializationMode", (self.api.enter_init)(self.c))?;
            check("fmi2ExitInitializationMode", (self.api.exit_init)(self.c))
        }
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

    fn do_step(&mut self, time: f64, step: f64) -> Result<bool> {
        // SAFETY: 有効なインスタンス。noSetFMUStatePriorToCurrentPoint = true（状態を巻き戻さない）
        check("fmi2DoStep", unsafe {
            (self.api.do_step)(self.c, time, step, 1)
        })?;
        Ok(false) // 2.0 の DoStep は打ち切りを返さない（fmi2GetStatus(Terminated) は未対応）
    }
}

impl Drop for Cs2<'_> {
    fn drop(&mut self) {
        // SAFETY: self.c は有効で、ここで最後に使う。Terminate の失敗は後片付けの途中なので無視する
        unsafe {
            (self.api.terminate)(self.c);
            (self.api.free_instance)(self.c);
        }
    }
}

impl std::fmt::Debug for Cs2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cs2")
    }
}

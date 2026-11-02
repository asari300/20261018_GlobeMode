//! GlobeMode の C ABI（S075）。facade `globe-mode` の `Workspace` を C の関数で包む（Zig・Julia などから使う）。
//!
//! 入出力は `globe-mode-py`・`globe-mode-wasm` と同じく JSON の文字列。ヘッダは `include/globe_mode.h`
//! （手書き。`tests/header.rs` がこのファイルの署名と突き合わせる）。
//!
//! - 返す `char*` は必ず [`gm_string_free`] で放す。誤りは `{"error": "..."}` の JSON で返す（NULL は返さない）。
//! - panic は境界で `catch_unwind` で止め、`{"error": "panic: ..."}` にする（unwind を C 側へ出さない）。
//! - `GmWorkspace` は作ったスレッドからだけ使う（salsa のデータベースは Sync でない）。

use std::ffi::{CStr, CString, c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

/// 不透明なワークスペース（C からは `GmWorkspace *` だけを持つ）。
#[derive(Debug)]
pub struct GmWorkspace(globe_mode::Workspace);

fn str_arg<'a>(p: *const c_char, name: &str) -> Result<&'a str, String> {
    if p.is_null() {
        return Err(format!("{name} が NULL"));
    }
    // SAFETY: 呼び手は NUL 終端の文字列を渡す（ヘッダの約束）
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|e| format!("{name} が UTF-8 でない: {e}"))
}

fn ws_arg<'a>(ws: *mut GmWorkspace) -> Result<&'a mut globe_mode::Workspace, String> {
    // SAFETY: 呼び手は gm_workspace_new が返して、まだ放していないポインタを渡す
    unsafe { ws.as_mut() }
        .map(|w| &mut w.0)
        .ok_or_else(|| "ws が NULL".to_string())
}

fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON: {e}"))
}

fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "不明".into())
}

/// `f` を panic を止めて回し、結果（誤りは `{"error": ...}`）を C の文字列にして返す。
fn guard(f: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    let text = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => serde_json::json!({ "error": e }).to_string(),
        Err(p) => {
            serde_json::json!({ "error": format!("panic: {}", panic_message(&*p)) }).to_string()
        }
    };
    // 本文に NUL が混じることは JSON では無い（\u0000 にエスケープされる）
    CString::new(text).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// `settings_json`（`{"stop_time": 2, "solver": "bdf"}` など。NULL 可）から実行の依頼を作って回す。
fn simulate_json(
    ws: &mut globe_mode::Workspace,
    model: &str,
    settings_json: Option<&str>,
) -> Result<String, String> {
    let settings: serde_json::Value = match settings_json {
        Some(s) => {
            serde_json::from_str(s).map_err(|e| format!("settings の JSON が読めない: {e}"))?
        }
        None => serde_json::json!({}),
    };
    let request =
        serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
            .map_err(|e| format!("settings が読めない: {e}"))?;
    json(&ws.simulate(&request))
}

/// ワークスペースを作る。`libraries_json` はライブラリのディレクトリの JSON の配列（`["data/msl"]`。NULL か
/// `[]` ならライブラリ無し）。読めなければ NULL（理由は stderr に出す）。
///
/// # Safety
///
/// `libraries_json` は NULL か NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_workspace_new(libraries_json: *const c_char) -> *mut GmWorkspace {
    let made = catch_unwind(|| -> Result<GmWorkspace, String> {
        let libs: Vec<PathBuf> = if libraries_json.is_null() {
            Vec::new()
        } else {
            serde_json::from_str(str_arg(libraries_json, "libraries_json")?)
                .map_err(|e| format!("libraries_json が読めない: {e}"))?
        };
        if libs.is_empty() {
            return Ok(GmWorkspace(globe_mode::Workspace::new()));
        }
        globe_mode::Workspace::with_libraries(&libs)
            .map(GmWorkspace)
            .map_err(|ds| json(&ds).unwrap_or_default())
    });
    match made {
        Ok(Ok(ws)) => Box::into_raw(Box::new(ws)),
        Ok(Err(e)) => {
            eprintln!("gm_workspace_new: {e}");
            std::ptr::null_mut()
        }
        Err(p) => {
            eprintln!("gm_workspace_new: panic: {}", panic_message(&*p));
            std::ptr::null_mut()
        }
    }
}

/// ワークスペースを放す。NULL なら何もしない。
///
/// # Safety
///
/// `ws` は NULL か、`gm_workspace_new` が返してまだ放していないポインタ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_workspace_free(ws: *mut GmWorkspace) {
    if !ws.is_null() {
        // SAFETY: 上の約束どおり Box::into_raw で作ったポインタ
        drop(unsafe { Box::from_raw(ws) });
    }
}

/// ファイルを足すか、本文を差し替える。0 で成功、-1 で引数の誤りか panic。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`path`・`text` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_set_file(
    ws: *mut GmWorkspace,
    path: *const c_char,
    text: *const c_char,
) -> c_int {
    let done = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let ws = ws_arg(ws)?;
        ws.set_file(str_arg(path, "path")?, str_arg(text, "text")?);
        Ok(())
    }));
    if matches!(done, Ok(Ok(()))) { 0 } else { -1 }
}

/// ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。-1 で誤りか panic。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`dir` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_load_dir(ws: *mut GmWorkspace, dir: *const c_char) -> i64 {
    let done = catch_unwind(AssertUnwindSafe(|| -> Result<usize, String> {
        let dir = str_arg(dir, "dir")?;
        ws_arg(ws)?
            .load_dir(Path::new(dir))
            .map_err(|e| e.to_string())
    }));
    match done {
        Ok(Ok(n)) => i64::try_from(n).unwrap_or(i64::MAX),
        _ => -1,
    }
}

/// 検査（S0〜S10）の結果（`CheckResponse` の JSON）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_check(ws: *mut GmWorkspace, model: *const c_char) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        json(&ws_arg(ws)?.check(model))
    })
}

/// connect グラフ（`ConnectGraph` の JSON）。モデルが索引に無ければ `{"error": ...}`。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_graph(
    ws: *mut GmWorkspace,
    model: *const c_char,
    depth: u32,
) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        json(&ws_arg(ws)?.graph(model, depth).map_err(|e| e.to_string())?)
    })
}

/// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。モデルの誤りは `ok` と `diagnostics` で返す。
/// `settings_json` は NULL 可（`{"stop_time": 2, "solver": "bdf"}` など）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model` は NUL 終端の UTF-8 の文字列、`settings_json` は NULL か同じ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_simulate(
    ws: *mut GmWorkspace,
    model: *const c_char,
    settings_json: *const c_char,
) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        let settings = if settings_json.is_null() {
            None
        } else {
            Some(str_arg(settings_json, "settings_json")?)
        };
        simulate_json(ws_arg(ws)?, model, settings)
    })
}

/// ソルバの登録表（JSON の配列）。
#[unsafe(no_mangle)]
pub extern "C" fn gm_solvers() -> *mut c_char {
    guard(|| json(globe_mode::solvers()))
}

/// このライブラリが返した文字列を放す。NULL なら何もしない。
///
/// # Safety
///
/// `s` は NULL か、このライブラリの関数が返してまだ放していないポインタ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_string_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: 上の約束どおり CString::into_raw で作ったポインタ
        drop(unsafe { CString::from_raw(s) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C の文字列を読んで放す。
    fn take(s: *mut c_char) -> serde_json::Value {
        assert!(!s.is_null());
        let v = serde_json::from_str(unsafe { CStr::from_ptr(s) }.to_str().unwrap()).unwrap();
        unsafe { gm_string_free(s) };
        v
    }

    #[test]
    fn check_and_simulate_round_trip_through_the_c_abi() {
        unsafe {
            let ws = gm_workspace_new(c"[]".as_ptr());
            assert_eq!(
                gm_set_file(
                    ws,
                    c"Decay.mo".as_ptr(),
                    c"model Decay Real x(start = 1, fixed = true); equation der(x) = -x; end Decay;"
                        .as_ptr(),
                ),
                0
            );
            assert_eq!(take(gm_check(ws, c"Decay".as_ptr()))["passed"], true);
            let res = take(gm_simulate(
                ws,
                c"Decay".as_ptr(),
                cr#"{"stop_time": 1}"#.as_ptr(),
            ));
            assert_eq!(res["ok"], true, "{}", res["diagnostics"]);
            assert!(res["csv"].as_str().unwrap().starts_with("\"time\""));
            // 誤りは NULL でなく JSON で返る
            assert!(take(gm_simulate(ws, c"Decay".as_ptr(), c"{".as_ptr()))["error"].is_string());
            assert!(take(gm_graph(ws, c"NoSuchModel".as_ptr(), 1))["error"].is_string());
            assert!(take(gm_check(std::ptr::null_mut(), c"Decay".as_ptr()))["error"].is_string());
            assert_eq!(gm_set_file(ws, std::ptr::null(), c"".as_ptr()), -1);
            assert!(take(gm_solvers()).is_array());
            gm_workspace_free(ws);
            assert!(gm_workspace_new(c"{".as_ptr()).is_null());
        }
    }

    #[test]
    fn panic_becomes_a_json_error() {
        let v = take(guard(|| panic!("boom")));
        assert_eq!(v["error"], "panic: boom");
    }
}

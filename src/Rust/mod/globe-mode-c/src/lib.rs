//! GlobeMode の C ABI（S075）。facade `globe-mode` の `Workspace` を C の関数で包む（Zig・Julia などから使う）。
//!
//! 入出力は `globe-mode-py`・`globe-mode-wasm` と同じく JSON の文字列。ヘッダは `include/globe_mode.h`
//! （手書き。`tests/header.rs` がこのファイルの署名と突き合わせる）。
//!
//! - 返す `char*` は必ず [`gm_string_free`] で放す。誤りは `{"error": "..."}` の JSON で返す（NULL は返さない）。
//! - panic は境界で `catch_unwind` で止め、`{"error": "panic: ..."}` にする（unwind を C 側へ出さない）。
//! - `GmWorkspace` は作ったスレッドからだけ使う（salsa のデータベースは Sync でない）。
//! - 進捗のコールバックの中から、実行中の同じ `GmWorkspace` を使う `gm_*` を呼ぶと誤りを返す（使用中の印。S202）。
//!   `gm_workspace_free` は使用中なら放さない（stderr に 1 行）。コールバックから抜けたあとに放し直す。
//! - 進捗と中断（S131、ADR-0033）: `*_observed` は進捗のたびに `GmProgressCallback` を呼び、非 0 が返ったら次の区切りで止める。

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 実行の進捗（ADR-0033。facade の `Progress` を写す）。`stage` は `"lowering"`・`"integration"` などの小文字で、
/// コールバックの間だけ有効。`time` は積分の段だけ（`has_time` が 0 なら NaN）。
#[repr(C)]
#[derive(Debug)]
pub struct GmProgress {
    pub stage: *const c_char,
    pub time: f64,
    pub has_time: c_int,
    pub start_time: f64,
    pub stop_time: f64,
}

/// 進捗のコールバック。非 0 を返すと中断する（`termination.kind = "cancelled"` と `GM-I11004`）。
pub type GmProgressCallback = Option<unsafe extern "C" fn(*const GmProgress, *mut c_void) -> c_int>;

/// `callback` を facade の `SimulateHandle` に写して `run` を回す（NULL なら進捗も中断も無い道）。
fn observed<T>(
    callback: GmProgressCallback,
    user_data: *mut c_void,
    run: impl FnOnce(globe_mode::SimulateHandle<'_>) -> T,
) -> T {
    let Some(cb) = callback else {
        return run(globe_mode::SimulateHandle::default());
    };
    let token = globe_mode::CancelToken::new();
    let mut notify = |p: &globe_mode::Progress| {
        let stage = CString::new(p.stage.as_str()).unwrap_or_default();
        let progress = GmProgress {
            stage: stage.as_ptr(),
            time: p.time.unwrap_or(f64::NAN),
            has_time: c_int::from(p.time.is_some()),
            start_time: p.start_time,
            stop_time: p.stop_time,
        };
        // SAFETY: 呼び手は (const GmProgress*, void*) を受ける関数と、その関数が受け付ける user_data を渡す
        if unsafe { cb(&progress, user_data) } != 0 {
            token.cancel();
        }
    };
    run(globe_mode::SimulateHandle {
        on_progress: Some(&mut notify),
        cancel: Some(token.clone()),
    })
}

/// 不透明なワークスペース（C からは `GmWorkspace *` だけを持つ）。
///
/// `busy` は「いま `&mut Workspace` が貸し出されている」印（S202）。進捗のコールバックの中から同じ `ws` で
/// `gm_*` が呼ばれても、`ws_arg` が印を見て誤りを返す。印は **生のポインタから `busy` のフィールドだけ**を読む
/// （`&mut GmWorkspace` も、印を見る前の `&mut Workspace` も作らない）ので、貸し出し中の `&mut Workspace` と
/// 別名にならない（`busy` と `ws` は別のフィールド）。
#[derive(Debug)]
pub struct GmWorkspace {
    busy: AtomicBool,
    ws: globe_mode::Workspace,
}

/// 印を立てて借りた `&mut Workspace`。落とすと（panic の unwind でも）印を戻す。
struct Lent<'a> {
    busy: &'a AtomicBool,
    ws: &'a mut globe_mode::Workspace,
}

impl std::ops::Deref for Lent<'_> {
    type Target = globe_mode::Workspace;
    fn deref(&self) -> &Self::Target {
        self.ws
    }
}

impl std::ops::DerefMut for Lent<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ws
    }
}

impl Drop for Lent<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

fn str_arg<'a>(p: *const c_char, name: &str) -> Result<&'a str, String> {
    if p.is_null() {
        return Err(format!("{name} が NULL"));
    }
    // SAFETY: 呼び手は NUL 終端の文字列を渡す（ヘッダの約束）
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|e| format!("{name} が UTF-8 でない: {e}"))
}

fn ws_arg<'a>(ws: *mut GmWorkspace) -> Result<Lent<'a>, String> {
    if ws.is_null() {
        return Err("ws が NULL".to_string());
    }
    // SAFETY: 呼び手は gm_workspace_new が返して、まだ放していないポインタを渡す。ここでは `&mut GmWorkspace` を
    // 作らず、フィールド `busy` への共有参照だけを作る。`busy` は貸し出し中の `&mut Workspace`（別のフィールド）と重ならない
    let busy = unsafe { &(*ws).busy };
    if busy.swap(true, Ordering::Acquire) {
        return Err(IN_USE.to_string());
    }
    // SAFETY: 上の swap が false → true にしたので、`ws` フィールドの `&mut` はこの 1 つだけ（落とすまで印は立ったまま）
    let ws = unsafe { &mut (*ws).ws };
    Ok(Lent { busy, ws })
}

const IN_USE: &str = "ワークスペースは使用中（進捗のコールバックの中から同じ ws は使えない）";

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

fn opt_str_arg<'a>(p: *const c_char, name: &str) -> Result<Option<&'a str>, String> {
    if p.is_null() {
        Ok(None)
    } else {
        str_arg(p, name).map(Some)
    }
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
        let ws = if libs.is_empty() {
            globe_mode::Workspace::new()
        } else {
            globe_mode::Workspace::with_libraries(&libs)
                .map_err(|ds| json(&ds).unwrap_or_default())?
        };
        Ok(GmWorkspace {
            busy: AtomicBool::new(false),
            ws,
        })
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

/// ワークスペースを放す。NULL なら何もしない。使用中（進捗のコールバックの中など）なら放さずに stderr に 1 行
/// 出して戻る（コールバックから抜けたあとに放し直す。S202）。
///
/// # Safety
///
/// `ws` は NULL か、`gm_workspace_new` が返してまだ放していないポインタ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_workspace_free(ws: *mut GmWorkspace) {
    if ws.is_null() {
        return;
    }
    // SAFETY: 約束どおり生きているポインタ。`&mut GmWorkspace` は作らず印だけを読む。
    // 立てたまま Box を落とす（使用中なら swap が true を返し、何もしない）
    if unsafe { &(*ws).busy }.swap(true, Ordering::Acquire) {
        eprintln!("gm_workspace_free: 放さない（{IN_USE}）");
        return;
    }
    // SAFETY: 上の約束どおり Box::into_raw で作ったポインタ。印を立てたので、貸し出し中の `&mut` は無い。
    // Drop の panic は C へ出さない（unwind は abort になる）
    if let Err(p) = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(ws) }))) {
        eprintln!("gm_workspace_free: panic: {}", panic_message(&*p));
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
        let mut ws = ws_arg(ws)?;
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

/// ファイル（`gm_set_file` に渡したパス）のアウトライン（`globe outline --json` と同じ `Outline` の JSON。S191）。
/// ファイルが無いか構文の誤りがあれば `{"error": ...}`（誤りは最初の診断の `コード: 文`）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`path` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_outline(ws: *mut GmWorkspace, path: *const c_char) -> *mut c_char {
    guard(|| ws_arg(ws)?.outline_json(str_arg(path, "path")?))
}

/// `gm_check` が読むファイル（モデルが使うものの閉包。パスの昇順）の JSON の配列。モデルが無ければ `[]`（S191）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_deps(ws: *mut GmWorkspace, model: *const c_char) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        json(&ws_arg(ws)?.deps(model))
    })
}

/// flat モデルの要約（`FlatSummary` の JSON）。flatten まで通らなければ `null`（S191）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_flat(ws: *mut GmWorkspace, model: *const c_char) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        json(&ws_arg(ws)?.flat(model))
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
        let settings = opt_str_arg(settings_json, "settings_json")?;
        json(&ws_arg(ws)?.simulate(&globe_mode::simulate_request(model, settings)?))
    })
}

/// [`gm_simulate`] と同じだが、進捗のたびに `callback(progress, user_data)` を呼び、非 0 が返ったら次の区切りで
/// 止める（S131、ADR-0033）。積分の途中なら `termination.kind = "cancelled"` と途中までの CSV、前なら `ok: false`。
/// どちらも `GM-I11004`。`callback` が NULL なら [`gm_simulate`] と同じ。
///
/// # Safety
///
/// [`gm_simulate`] と同じ。`callback` は NULL か、`user_data` を受け付ける関数。
///
/// 実行の間 `ws` は「使用中」で、`callback` の中から同じ `ws` を使う `gm_*` は誤りを返し、`gm_workspace_free(ws)` は
/// 何もしない（S202）。実行中の `&mut Workspace` と別名になる参照を作らないための守りで、`callback` から
/// `longjmp` や例外で抜けないこと（Rust のフレームを越える）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_simulate_observed(
    ws: *mut GmWorkspace,
    model: *const c_char,
    settings_json: *const c_char,
    callback: GmProgressCallback,
    user_data: *mut c_void,
) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        let request =
            globe_mode::simulate_request(model, opt_str_arg(settings_json, "settings_json")?)?;
        let mut ws = ws_arg(ws)?;
        let simulated = observed(callback, user_data, |h| {
            ws.simulate_observed(&request, &globe_mode::now_ms, h)
        });
        json(&simulated.response)
    })
}

/// 列の道（S116。Python の `simulate_columns`・wasm の `SimColumns` と同じ並び）。戻り値は
/// `{"response": SimulateResponse（csv 無し）, "names": ["time", ...], "points": n, "defaults": ["x", ...]}` の JSON
/// （`defaults` は既定で描く信号の名前。最大 3 本。S190・S191）。
/// `*data` には列を順に並べた `names.len() * points` 個の f64（列 i は `data[i * points ..]`）を、`*len` にその個数を入れる。
/// `*data` は [`gm_doubles_free`] で放す。誤りや実行できなかったときは `*data` が NULL で `*len` が 0。
///
/// # Safety
///
/// `ws`・`model`・`settings_json` は [`gm_simulate`] と同じ。`data`・`len` は書ける場所を指す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_simulate_columns(
    ws: *mut GmWorkspace,
    model: *const c_char,
    settings_json: *const c_char,
    data: *mut *mut f64,
    len: *mut usize,
) -> *mut c_char {
    // SAFETY: 引数の約束は同じ
    unsafe {
        gm_simulate_columns_observed(
            ws,
            model,
            settings_json,
            data,
            len,
            None,
            std::ptr::null_mut(),
        )
    }
}

/// [`gm_simulate_columns`] に [`gm_simulate_observed`] と同じ `callback` を足したもの（S131）。中断したら
/// 止めた時刻までの列を返す。
///
/// # Safety
///
/// [`gm_simulate_columns`] と [`gm_simulate_observed`] と同じ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_simulate_columns_observed(
    ws: *mut GmWorkspace,
    model: *const c_char,
    settings_json: *const c_char,
    data: *mut *mut f64,
    len: *mut usize,
    callback: GmProgressCallback,
    user_data: *mut c_void,
) -> *mut c_char {
    if data.is_null() || len.is_null() {
        return guard(|| Err("data か len が NULL".to_string()));
    }
    // SAFETY: 上で NULL でないことを確かめた。呼び手は書ける場所を渡す
    unsafe {
        *data = std::ptr::null_mut();
        *len = 0;
    }
    guard(|| {
        let model = str_arg(model, "model")?;
        let mut request =
            globe_mode::simulate_request(model, opt_str_arg(settings_json, "settings_json")?)?;
        request.no_csv = true;
        let mut ws = ws_arg(ws)?;
        let simulated = observed(callback, user_data, |h| {
            ws.simulate_observed(&request, &globe_mode::now_ms, h)
        });
        let (layout, columns) = simulated.columns();
        let flat: Box<[f64]> = columns.concat().into_boxed_slice();
        let text = simulated.layout_json(&layout)?;
        if !flat.is_empty() {
            // SAFETY: 上と同じ
            unsafe {
                *len = flat.len();
                *data = Box::into_raw(flat).cast::<f64>();
            }
        }
        Ok(text)
    })
}

/// [`gm_simulate_columns`] が返した列を放す。NULL なら何もしない。
///
/// # Safety
///
/// `data`・`len` は [`gm_simulate_columns`] が返した組で、まだ放していないもの。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_doubles_free(data: *mut f64, len: usize) {
    if !data.is_null() {
        // SAFETY: 上の約束どおり Box<[f64]> を Box::into_raw で作ったポインタと長さ
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, len)) });
    }
}

/// FMU を書き出す（S118。`globe fmu export` と同じ）。`{"path": ".fmu のパス"}`、誤りは `{"error": ...}`。
/// `options_json` は NULL 可（`{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}`）。
///
/// # Safety
///
/// `ws` は生きているワークスペース、`model`・`out_dir` は NUL 終端の UTF-8 の文字列、`options_json` は NULL か同じ。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_export_fmu(
    ws: *mut GmWorkspace,
    model: *const c_char,
    out_dir: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    guard(|| {
        let model = str_arg(model, "model")?;
        let out_dir = str_arg(out_dir, "out_dir")?;
        let options: globe_mode::FmuOptions = match opt_str_arg(options_json, "options_json")? {
            Some(s) => serde_json::from_str(s).map_err(|e| format!("options が読めない: {e}"))?,
            None => Default::default(),
        };
        let path = ws_arg(ws)?.export_fmu(model, Path::new(out_dir), &options)?;
        Ok(serde_json::json!({ "path": path }).to_string())
    })
}

/// ソルバの登録表（JSON の配列）。
#[unsafe(no_mangle)]
pub extern "C" fn gm_solvers() -> *mut c_char {
    guard(|| json(globe_mode::solvers()))
}

/// 本文を整形する（S111）。`{"text": "..."}`、文法が壊れていれば `{"error": "..."}`。`insert_origin` は 0 か 1。
///
/// # Safety
///
/// `text` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_format(
    text: *const c_char,
    indent: u32,
    insert_origin: c_int,
) -> *mut c_char {
    guard(|| {
        let text = str_arg(text, "text")?;
        let formatted = globe_mode::format(text, indent as usize, insert_origin != 0)?;
        Ok(serde_json::json!({ "text": formatted }).to_string())
    })
}

/// lint の結果（`[{rule, line, column, message}]` の JSON。S111）。
///
/// # Safety
///
/// `text` は NUL 終端の UTF-8 の文字列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gm_lint(text: *const c_char) -> *mut c_char {
    guard(|| Ok(globe_mode::lint(str_arg(text, "text")?)))
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
            // S111: 参照シナリオの最後（format）と lint
            let x = res["final"]["x"].as_f64().unwrap();
            assert!((x - (-1f64).exp()).abs() < 1e-4, "{x}");
            let f = take(gm_format(c"model Decay Real x; end Decay;".as_ptr(), 2, 0));
            assert!(
                f["text"].as_str().unwrap().starts_with("model Decay\n"),
                "{f}"
            );
            assert!(take(gm_format(c"model Decay".as_ptr(), 2, 0))["error"].is_string());
            assert!(take(gm_lint(c"model Decay Real x; end Decay;".as_ptr())).is_array());
            // S191: 読み取りの口と summary
            assert_eq!(
                take(gm_outline(ws, c"Decay.mo".as_ptr()))["classes"][0]["name"],
                "Decay"
            );
            assert!(take(gm_outline(ws, c"Nope.mo".as_ptr()))["error"].is_string());
            assert_eq!(
                take(gm_deps(ws, c"Decay".as_ptr())),
                serde_json::json!(["Decay.mo"])
            );
            assert_eq!(take(gm_deps(ws, c"NoSuch".as_ptr())), serde_json::json!([]));
            assert_eq!(take(gm_flat(ws, c"Decay".as_ptr()))["variables"], 1);
            assert!(take(gm_flat(ws, c"NoSuch".as_ptr())).is_null());
            assert_eq!(
                take(gm_check(ws, c"Decay".as_ptr()))["summary"],
                "Decay: OK（S0〜S10 を通過）"
            );
            // S116: 列の道。x(1) が e⁻¹、点の数が CSV の行数 - 1 と同じ
            let (mut data, mut len) = (std::ptr::null_mut(), 0usize);
            let lay = take(gm_simulate_columns(
                ws,
                c"Decay".as_ptr(),
                cr#"{"stop_time": 1}"#.as_ptr(),
                &mut data,
                &mut len,
            ));
            assert!(lay["response"]["csv"].is_null(), "{lay}");
            assert_eq!(lay["names"], serde_json::json!(["time", "x"]));
            assert_eq!(lay["defaults"], serde_json::json!(["x"]), "{lay}"); // S191
            let points = lay["points"].as_u64().unwrap() as usize;
            assert_eq!(points, res["csv"].as_str().unwrap().lines().count() - 1);
            assert_eq!(len, 2 * points);
            let cols = std::slice::from_raw_parts(data, len);
            assert_eq!(cols[2 * points - 1], x);
            gm_doubles_free(data, len);
            let err = take(gm_simulate_columns(
                ws,
                c"Decay".as_ptr(),
                c"{".as_ptr(),
                &mut data,
                &mut len,
            ));
            assert!(err["error"].is_string() && data.is_null() && len == 0);
            // S118: FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある
            let out = std::env::temp_dir().join(format!("gm-c-fmu-{}", std::process::id()));
            let out_c = CString::new(out.to_str().unwrap()).unwrap();
            let fmu = take(gm_export_fmu(
                ws,
                c"Decay".as_ptr(),
                out_c.as_ptr(),
                std::ptr::null(),
            ));
            assert!(
                fmu["path"].as_str().unwrap().ends_with("Decay-fmi2.fmu"),
                "{fmu}"
            );
            assert!(out.join("Decay/modelDescription.xml").is_file());
            std::fs::remove_dir_all(&out).unwrap();
            let bad = take(gm_export_fmu(
                ws,
                c"Decay".as_ptr(),
                out_c.as_ptr(),
                cr#"{"fmi_version": 4}"#.as_ptr(),
            ));
            assert!(bad["error"].is_string(), "{bad}");
            gm_workspace_free(ws);
            assert!(gm_workspace_new(c"{".as_ptr()).is_null());
        }
    }

    /// 最初の t の進捗で 1 を返す（S131）。`user_data` は呼ばれた回数。
    /// 最初の「t が 1 つの出力の間隔を越えた」進捗で止める。t = 0 の進捗で止めると途中の列が空になり、
    /// 負荷の高いランナーで落ちる（S148 の全体検査）。S136 の LSP の試験と同じ待ち方
    unsafe extern "C" fn stop_at_first_time(p: *const GmProgress, user: *mut c_void) -> c_int {
        unsafe {
            *user.cast::<u32>() += 1;
            c_int::from((*p).has_time != 0 && (*p).time > 1.0)
        }
    }

    #[test]
    fn a_nonzero_callback_cancels_with_the_partial_trajectory() {
        unsafe {
            let ws = gm_workspace_new(std::ptr::null());
            gm_set_file(
                ws,
                c"Wiggle.mo".as_ptr(),
                c"model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;"
                    .as_ptr(),
            );
            let settings = cr#"{"solver": "esdirk34", "stop_time": 1000, "interval": 1}"#;
            let mut calls = 0u32;
            let user = (&raw mut calls).cast::<c_void>();
            let res = take(gm_simulate_observed(
                ws,
                c"Wiggle".as_ptr(),
                settings.as_ptr(),
                Some(stop_at_first_time),
                user,
            ));
            assert_eq!(res["termination"]["kind"], "cancelled", "{res}");
            assert!(calls >= 2, "{calls}");
            let codes = res["diagnostics"].to_string();
            assert!(codes.contains("GM-I11004"), "{codes}");
            let (mut data, mut len) = (std::ptr::null_mut(), 0usize);
            let lay = take(gm_simulate_columns_observed(
                ws,
                c"Wiggle".as_ptr(),
                settings.as_ptr(),
                &mut data,
                &mut len,
                Some(stop_at_first_time),
                user,
            ));
            let time = lay["response"]["termination"]["time"].as_f64().unwrap();
            let points = lay["points"].as_u64().unwrap() as usize;
            assert!(0.0 < time && time < 1000.0, "{lay}");
            assert_eq!(std::slice::from_raw_parts(data, len)[points - 1], time);
            gm_doubles_free(data, len);
            gm_workspace_free(ws);
        }
    }

    /// 進捗のコールバックの中から同じ `ws` を使う（S202）。結果を `Reenter` に溜める。
    struct Reenter {
        ws: *mut GmWorkspace,
        calls: u32,
        errors: Vec<serde_json::Value>,
        set_file: c_int,
        load_dir: i64,
    }

    unsafe extern "C" fn reenter(_p: *const GmProgress, user: *mut c_void) -> c_int {
        unsafe {
            let r = &mut *user.cast::<Reenter>();
            r.calls += 1;
            if r.calls == 1 {
                let (mut data, mut len) = (std::ptr::null_mut(), 0usize);
                r.errors.push(take(gm_check(r.ws, c"Decay".as_ptr())));
                r.errors
                    .push(take(gm_simulate(r.ws, c"Decay".as_ptr(), std::ptr::null())));
                r.errors.push(take(gm_simulate_columns(
                    r.ws,
                    c"Decay".as_ptr(),
                    std::ptr::null(),
                    &mut data,
                    &mut len,
                )));
                assert!(data.is_null() && len == 0);
                r.set_file = gm_set_file(r.ws, c"B.mo".as_ptr(), c"model B end B;".as_ptr());
                r.load_dir = gm_load_dir(r.ws, c".".as_ptr());
                gm_workspace_free(r.ws); // 使用中なので放されない（放されたら、このあとの実行が use-after-free）
            }
            0
        }
    }

    #[test]
    fn a_callback_cannot_reuse_or_free_the_running_workspace() {
        unsafe {
            let ws = gm_workspace_new(std::ptr::null());
            gm_set_file(
                ws,
                c"Decay.mo".as_ptr(),
                c"model Decay Real x(start = 1, fixed = true); equation der(x) = -x; end Decay;"
                    .as_ptr(),
            );
            let mut r = Reenter {
                ws,
                calls: 0,
                errors: Vec::new(),
                set_file: 0,
                load_dir: 0,
            };
            let user = (&raw mut r).cast::<c_void>();
            let res = take(gm_simulate_observed(
                ws,
                c"Decay".as_ptr(),
                cr#"{"stop_time": 1}"#.as_ptr(),
                Some(reenter),
                user,
            ));
            assert_eq!(res["ok"], true, "{}", res["diagnostics"]);
            assert_eq!(r.errors.len(), 3);
            for e in &r.errors {
                assert!(e["error"].as_str().unwrap().contains("使用中"), "{e}");
            }
            assert_eq!((r.set_file, r.load_dir), (-1, -1));
            // 印は戻っていて、ws はまだ生きている
            assert_eq!(take(gm_check(ws, c"Decay".as_ptr()))["passed"], true);
            assert_eq!(
                gm_set_file(ws, c"B.mo".as_ptr(), c"model B end B;".as_ptr()),
                0
            );
            // 列の道も同じ
            let (mut data, mut len) = (std::ptr::null_mut(), 0usize);
            r.calls = 0;
            r.errors.clear();
            take(gm_simulate_columns_observed(
                ws,
                c"Decay".as_ptr(),
                cr#"{"stop_time": 1}"#.as_ptr(),
                &mut data,
                &mut len,
                Some(reenter),
                user,
            ));
            assert_eq!(r.errors.len(), 3);
            gm_doubles_free(data, len);
            gm_workspace_free(ws);
        }
    }

    /// 実行が panic しても印は戻る（`Lent` の Drop）
    #[test]
    fn the_busy_mark_is_cleared_after_a_panic() {
        unsafe {
            let ws = gm_workspace_new(std::ptr::null());
            let v = take(guard(|| {
                let _lent = ws_arg(ws)?;
                panic!("boom")
            }));
            assert_eq!(v["error"], "panic: boom");
            assert_eq!(take(gm_check(ws, c"X".as_ptr()))["passed"], false);
            gm_workspace_free(ws);
        }
    }

    #[cfg(target_pointer_width = "64")]
    /// `GmProgress` の並びと大きさは `include/globe_mode.h` の typedef と同じ（`tests/header.rs` が C 側の規則で突き合わせる）
    #[test]
    fn gm_progress_layout() {
        use std::mem::{offset_of, size_of};
        assert_eq!(size_of::<GmProgress>(), 40);
        assert_eq!(offset_of!(GmProgress, stage), 0);
        assert_eq!(offset_of!(GmProgress, time), 8);
        assert_eq!(offset_of!(GmProgress, has_time), 16);
        assert_eq!(offset_of!(GmProgress, start_time), 24);
        assert_eq!(offset_of!(GmProgress, stop_time), 32);
    }

    #[test]
    fn panic_becomes_a_json_error() {
        let v = take(guard(|| panic!("boom")));
        assert_eq!(v["error"], "panic: boom");
    }
}

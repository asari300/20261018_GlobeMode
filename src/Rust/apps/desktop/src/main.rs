//! GlobeMode Desktop（S049、ADR-0023）。Tauri のシェル。検査も実行もここではしない。
//! 同梱の `globe`（sidecar）を `globe lsp` として立ち上げ、UI（`src/Svelte`）からの要求を stdio の JSON-RPC に中継する。
//! facade `globe-mode` はリンクしない。UI へは 2 つの command（`rpc`・`notify`）とイベント `globe-notification`
//! （サーバからの通知。`publishDiagnostics` など）を出す。
//!
//! 引数・環境変数: `--library <DIR>`（何度でも）と `GLOBE_LIBRARY`（パスの区切りで複数。MSL を渡す）、
//! `GLOBE_SIDECAR`（`globe` のパス。既定は実行ファイルと同じ場所）。
//! `GLOBE_DESKTOP_SMOKE=1` なら、起動して sidecar に 1 モデルの検査を頼み、結果を表示して終わる（CI の smoke）。
//! さらに `GLOBE_DESKTOP_SMOKE_MODEL=<ライブラリのモデル名>` なら、そのモデルを `globe/check`・`globe/simulate` させ、
//! 各段の後にアプリと sidecar の最大の常駐メモリ（VmHWM）を表示する（S056。MSL を読んだ後の RSS の基準値）。

// Windows のリリースでコンソールを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod worker;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::{Value, json};
use tauri::{Emitter, Manager, State};
use worker::{Sink, Worker};

type Shared = Arc<Mutex<Worker>>;

fn lock(w: &Shared) -> MutexGuard<'_, Worker> {
    w.lock().unwrap_or_else(|e| e.into_inner())
}

/// 要求を 1 つ頼み、`result` を返す（JSON-RPC のエラーは Err の文字列）。
#[tauri::command]
async fn rpc(state: State<'_, Shared>, method: String, params: Value) -> Result<Value, String> {
    let w = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || lock(&w).request(&method, params))
        .await
        .map_err(|e| e.to_string())?
}

/// 通知を 1 つ送る（`textDocument/didOpen` など）。
#[tauri::command]
async fn notify(state: State<'_, Shared>, method: String, params: Value) -> Result<(), String> {
    let w = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || lock(&w).notify(&method, params))
        .await
        .map_err(|e| e.to_string())?
}

/// 起動時に渡したライブラリのパス（UI の `TauriBackend.library` が「sidecar が既に読んでいるか」を知るため）。
struct Libraries(Vec<PathBuf>);

#[tauri::command]
fn libraries(state: State<'_, Libraries>) -> Vec<String> {
    state.0.iter().map(|p| p.display().to_string()).collect()
}

fn libraries_from_env() -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = std::env::var_os("GLOBE_LIBRARY")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    let args: Vec<String> = std::env::args().collect();
    for pair in args.windows(2) {
        if pair[0] == "--library" {
            libs.push(PathBuf::from(&pair[1]));
        }
    }
    libs
}

fn sidecar() -> PathBuf {
    if let Some(p) = std::env::var_os("GLOBE_SIDECAR") {
        return p.into();
    }
    let exe = std::env::current_exe().expect("current_exe");
    exe.with_file_name(if cfg!(windows) { "globe.exe" } else { "globe" })
}

/// `/proc/<pid>/status` の VmHWM の行（Linux だけ）。
fn hwm(pid: &str) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "VmHWM: n/a".into())
}

/// アプリ自身と sidecar の VmHWM を `label` つきで表示する。
fn report_hwm(label: &str, w: &Worker) {
    let side = w
        .pid()
        .map_or_else(|| "n/a".into(), |p| hwm(&p.to_string()));
    println!("smoke: {label} app {} | sidecar {side}", hwm("self"));
}

/// smoke: 起動 → 検査（→ 実行）→ 最大の常駐メモリ（Linux）を表示 → 終了。失敗は終了コード 1。
/// モデル名が `GLOBE_DESKTOP_SMOKE_MODEL` にあればライブラリのモデルを検査・実行し、無ければ埋め込みの小さい `D`。
fn smoke(w: &Shared) -> Result<(), String> {
    let text = "model D\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend D;\n";
    let lib_model = std::env::var("GLOBE_DESKTOP_SMOKE_MODEL").ok();
    let model = lib_model.as_deref().unwrap_or("D");
    let mut w = lock(w);
    if lib_model.is_none() {
        w.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": "D.mo", "languageId": "modelica", "version": 1, "text": text}}),
        )?;
    }
    let r = w.request("globe/check", json!({"model": model}))?;
    if r["passed"] != true {
        return Err(format!("{model} の検査が通らない: {r}"));
    }
    println!("smoke: ok passed=true failed_stage={}", r["failed_stage"]);
    report_hwm("after check", &w);
    if lib_model.is_some() {
        let r = w.request(
            "globe/simulate",
            json!({"request": {"model": model, "no_csv": true}}),
        )?;
        if r["ok"] != true {
            return Err(format!("{model} の実行が通らない: {r}"));
        }
        println!("smoke: simulate ok=true");
        report_hwm("after simulate", &w);
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![rpc, notify, libraries])
        .setup(|app| {
            let handle = app.handle().clone();
            let sink: Sink = Arc::new({
                let handle = handle.clone();
                move |m: Value| {
                    let _ = handle.emit("globe-notification", m);
                }
            });
            let libs = libraries_from_env();
            app.manage(Libraries(libs.clone()));
            let w: Shared = Arc::new(Mutex::new(Worker::new(sidecar(), libs, sink)));
            app.manage(w.clone());
            // 起動と同時にライブラリを読み始める（待たされた要求はこのロックの後に走る）
            std::thread::spawn(move || {
                let smoke_run = std::env::var_os("GLOBE_DESKTOP_SMOKE").is_some();
                let r = if smoke_run {
                    smoke(&w)
                } else {
                    lock(&w).ensure_live()
                };
                if let Err(e) = &r {
                    eprintln!("globe sidecar: {e}");
                }
                if smoke_run {
                    handle.exit(i32::from(r.is_err()));
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri の起動に失敗");
}

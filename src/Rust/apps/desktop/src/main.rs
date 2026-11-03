//! GlobeMode Desktop（S049、ADR-0023）。Tauri のシェル。検査も実行もここではしない。
//! 同梱の `globe`（sidecar）を `globe lsp` として立ち上げ、UI（`src/Svelte`）からの要求を stdio の JSON-RPC に中継する。
//! facade `globe-mode` はリンクしない。UI へは command（`rpc`・`notify`、実行を止める `cancel_run`。S136）とイベント `globe-notification`
//! （サーバからの通知。`publishDiagnostics` など）を出す。
//!
//! 引数・環境変数: `--library <DIR>`（何度でも）と `GLOBE_LIBRARY`（パスの区切りで複数。MSL を渡す）、
//! `GLOBE_SIDECAR`（`globe` のパス。既定は実行ファイルと同じ場所）。
//! `GLOBE_DESKTOP_SMOKE=1` なら、起動して sidecar に 1 モデルの検査を頼み、結果を表示して終わる（CI の smoke）。
//! さらに `GLOBE_DESKTOP_SMOKE_MODEL=<ライブラリのモデル名>` なら、そのモデルを `globe/check`・`globe/simulate` させ、
//! 各段の後にアプリと sidecar の最大の常駐メモリ（VmHWM）を表示する（S056。MSL を読んだ後の RSS の基準値）。
//!
//! MSL（S069、ADR-0030）: ライブラリが渡されなければ、app data dir の `msl/<version>/` に展開済み（目印あり）のときそれを渡す
//! （4.1.0 を先に探す）。無ければ UI のボタンが command `fetch_msl { version }`（S084。"4.1.0" | "4.0.0"）を呼び、
//! 取得して展開し、sidecar を新しい `--library` で作り直す（版を替えるときも同じ）。取得中はイベント `msl-progress`
//! （`{received, total}`。total は Content-Length、無ければ null）を出し、command `cancel_msl` で取り消せる。
//! 自動更新（S135）: `plugins.updater.pubkey` が空でなければ updater プラグインを入れる（空なら UI の「更新を確認」は「未設定」）。
//! `GLOBE_DESKTOP_SMOKE_FETCH=1` を足すと、smoke は検査の前にこの取得を 1 回行う（CI で取得の道を確かめる）。
//!
//! Linux で locale が `C.UTF-8` などなら、起動時に `en_US.UTF-8` に替える（S134。`fix_c_locale`）。
//!
//! `GLOBE_DESKTOP_BENCH=<ファイル>`（S125、ADR-0034）なら、UI が起動時に `?smoke=1` と同じ手順を測り、
//! command `bench { line }` で `GM-SMOKE … ready_ms=…` の行をそのファイルに書いて終わる。ブラウザと揃えるため、
//! 取得済みの MSL を起動時には渡さない（MSL を読むのは UI の手順の中）。

// Windows のリリースでコンソールを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod msl;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

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

/// 実行中の `globe/simulate*` を止める（S136、ADR-0033）。`Worker` の `Mutex` を待たずに `$/cancelRequest` を送る。
#[tauri::command]
fn cancel_run(c: State<'_, worker::Canceller>) {
    c.cancel();
}

/// 通知を 1 つ送る（`textDocument/didOpen` など）。
#[tauri::command]
async fn notify(state: State<'_, Shared>, method: String, params: Value) -> Result<(), String> {
    let w = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || lock(&w).notify(&method, params))
        .await
        .map_err(|e| e.to_string())?
}

/// S125: `GLOBE_DESKTOP_BENCH` のとき、line なしは true（測る）、line ありはその行をファイルに書いてアプリを終える。
/// async にして主スレッドの外で `exit` を呼ぶ（smoke と同じ）。
#[tauri::command]
async fn bench(app: tauri::AppHandle, line: Option<String>) -> Result<bool, String> {
    let Some(path) = std::env::var_os("GLOBE_DESKTOP_BENCH") else {
        return Ok(false);
    };
    if let Some(line) = line {
        std::fs::write(path, line + "\n").map_err(|e| e.to_string())?;
        app.exit(0);
    }
    Ok(true)
}

/// sidecar に渡しているライブラリのパス（UI の `TauriBackend.library` が「sidecar が既に読んでいるか」を知るため）。
/// 取得した MSL で増える。
#[derive(Clone)]
struct Libraries(Arc<Mutex<Vec<PathBuf>>>);

/// MSL の置き場の根（app data dir の `msl`。版ごとに `msl/<version>`）と、取得の取り消しの旗。
#[derive(Clone)]
struct MslDir(PathBuf, Arc<AtomicBool>);

fn names(libs: &Libraries) -> Vec<String> {
    let libs = libs.0.lock().unwrap_or_else(|e| e.into_inner());
    libs.iter().map(|p| p.display().to_string()).collect()
}

#[tauri::command]
fn libraries(state: State<'_, Libraries>) -> Vec<String> {
    names(&state)
}

/// その版の MSL を取得して展開し、sidecar をそれだけで作り直す。読んだパスを返す。
/// 外から渡されたライブラリ（`--library`・`GLOBE_LIBRARY`）があれば何もしない。その版を読んでいても何もしない。
fn install_msl(
    dir: &MslDir,
    libs: &Libraries,
    w: &Shared,
    version: &str,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<String>, String> {
    let target = msl::dest(&dir.0, version);
    let current = libs.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if matches!(current.as_slice(), [p] if *p == target) {
        return Ok(names(libs));
    }
    if current.iter().any(|p| !p.starts_with(&dir.0)) {
        return Err(
            "外から渡したライブラリ（--library・GLOBE_LIBRARY）を使っているので MSL を切り替えない"
                .into(),
        );
    }
    dir.1.store(false, Ordering::Relaxed);
    msl::fetch(&target, version, progress, &dir.1)?;
    *libs.0.lock().unwrap_or_else(|e| e.into_inner()) = vec![target.clone()];
    let mut w = lock(w);
    w.set_libraries(vec![target]);
    w.ensure_live()?; // 読めない MSL はここで UI に返す
    Ok(names(libs))
}

/// MSL を取得して使えるようにする（UI の「MSL を取得する」と版の選択）。進捗はイベント `msl-progress`。
#[tauri::command]
async fn fetch_msl(
    app: tauri::AppHandle,
    version: String,
    dir: State<'_, MslDir>,
    libs: State<'_, Libraries>,
    state: State<'_, Shared>,
) -> Result<Vec<String>, String> {
    let (dir, libs, w) = (
        dir.inner().clone(),
        libs.inner().clone(),
        state.inner().clone(),
    );
    tauri::async_runtime::spawn_blocking(move || {
        install_msl(&dir, &libs, &w, &version, |received, total| {
            let _ = app.emit(
                "msl-progress",
                json!({"received": received, "total": total}),
            );
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 取得中の `fetch_msl` を止める（途中のファイルは消える。`fetch_msl` は Err を返す）。
#[tauri::command]
fn cancel_msl(dir: State<'_, MslDir>) {
    dir.1.store(true, Ordering::Relaxed);
}

/// updater の公開鍵（`plugins.updater.pubkey`）が入ったビルドか（S135）。UI はこれが偽なら「未設定」と出す。
#[tauri::command]
fn updater_configured(app: tauri::AppHandle) -> bool {
    let updater = app.config().plugins.0.get("updater");
    updater
        .and_then(|u| u["pubkey"].as_str())
        .is_some_and(|k| !k.is_empty())
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
fn smoke(w: &Shared, dir: &MslDir, libs: &Libraries) -> Result<(), String> {
    if std::env::var_os("GLOBE_DESKTOP_SMOKE_FETCH").is_some() {
        let t = Instant::now();
        let got = install_msl(dir, libs, w, msl::VERSIONS[0], |_, _| {})?;
        println!(
            "smoke: fetched msl in {:.1} s -> {got:?}",
            t.elapsed().as_secs_f64()
        );
    }
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

/// S134（ADR-0023）: WebKitGTK は LC_CTYPE の `C.UTF-8` から `.` 以降を落として既定の言語を `C` にし、
/// JS の `Intl`（CodeMirror の `Intl.Segmenter`、uPlot の `Intl.NumberFormat`）が RangeError を投げて UI が描かれない。
/// `C`・`POSIX` そのものは WebKit が `en-US` にするので、`C.`・`POSIX.` で始まるときだけ `en_US.UTF-8` に替える
/// （その locale が無い環境では setlocale が失敗して `C` のままになり、WebKit はやはり `en-US` を使う）。
#[cfg(target_os = "linux")]
fn fix_c_locale() {
    let var = ["LC_ALL", "LC_CTYPE", "LANG"]
        .into_iter()
        .find(|v| std::env::var_os(v).is_some_and(|s| !s.is_empty()));
    if let Some(var) = var
        && std::env::var(var).is_ok_and(|s| s.starts_with("C.") || s.starts_with("POSIX."))
    {
        // SAFETY: main の最初、ほかのスレッドが立つ前
        unsafe { std::env::set_var(var, "en_US.UTF-8") };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    fix_c_locale();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            rpc,
            notify,
            cancel_run,
            libraries,
            fetch_msl,
            cancel_msl,
            bench,
            updater_configured
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            // 公開鍵は tauri.conf.json に書く（鍵を作ったユーザーが。ADR-0023 の S135）。空なら入れない
            if updater_configured(handle.clone()) {
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            let sink: Sink = Arc::new({
                let handle = handle.clone();
                move |m: Value| {
                    let _ = handle.emit("globe-notification", m);
                }
            });
            let mut libs = libraries_from_env();
            let dir = MslDir(
                app.path()
                    .app_data_dir()
                    .map_err(|e| e.to_string())?
                    .join("msl"),
                Arc::new(AtomicBool::new(false)),
            );
            // 渡されたライブラリが優先。無くて、取得済みの MSL があればそれを渡す（4.1.0 を先に）
            if libs.is_empty() && std::env::var_os("GLOBE_DESKTOP_BENCH").is_none() {
                let found = msl::VERSIONS
                    .iter()
                    .map(|v| (msl::dest(&dir.0, v), *v))
                    .find(|(d, v)| msl::ready(d, v));
                libs.extend(found.map(|(d, _)| d));
            }
            let libs_state = Libraries(Arc::new(Mutex::new(libs.clone())));
            app.manage(libs_state.clone());
            app.manage(dir.clone());
            let worker = Worker::new(sidecar(), libs, sink);
            app.manage(worker.canceller());
            let w: Shared = Arc::new(Mutex::new(worker));
            app.manage(w.clone());
            // 起動と同時にライブラリを読み始める（待たされた要求はこのロックの後に走る）
            std::thread::spawn(move || {
                let smoke_run = std::env::var_os("GLOBE_DESKTOP_SMOKE").is_some();
                let r = if smoke_run {
                    smoke(&w, &dir, &libs_state)
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

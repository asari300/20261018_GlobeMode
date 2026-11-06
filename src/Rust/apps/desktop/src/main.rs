//! GlobeMode Desktop（S049、ADR-0023）。Tauri のシェル。検査も実行もここではしない。
//! 同梱の `globe`（sidecar）を `globe lsp` として立ち上げ、UI（`src/Svelte`）からの要求を stdio の JSON-RPC に中継する。
//! facade `globe-mode` はリンクしない。UI へは command（`rpc`・`notify`、実行を止める `cancel_run`。S136）とイベント `globe-notification`
//! （サーバからの通知。`publishDiagnostics` など）を出す。
//!
//! 引数・環境変数: `--library <DIR>`（何度でも）と `GLOBE_LIBRARY`（パスの区切りで複数。MSL を渡す）、
//! `GLOBE_SIDECAR`（`globe` のパス。既定は実行ファイルと同じ場所）。
//! 設定ファイル（S143）: `--root <DIR>`、無ければ `GLOBE_ROOT`、無ければカレントディレクトリを sidecar の `initialize` に
//! `rootPath` で渡す。sidecar は根から上へ `globe-mode.toml` を探し、整形と lint に使う。command `reload_config` で読み直す。
//! `GLOBE_DESKTOP_SMOKE=1` なら、起動して sidecar に 1 モデルの検査を頼み、結果を表示して終わる（CI の smoke）。
//! さらに `GLOBE_DESKTOP_SMOKE_MODEL=<ライブラリのモデル名>` なら、そのモデルを `globe/check`・`globe/simulate` させ、
//! 各段の後にアプリと sidecar の最大の常駐メモリ（VmHWM）を表示する（S056。MSL を読んだ後の RSS の基準値）。
//!
//! MSL（S069、ADR-0030）: ライブラリが渡されなければ、app data dir の `msl/<version>/` に展開済み（目印あり）のときそれを渡す
//! （4.1.0 を先に探す）。無ければ UI のボタンが command `fetch_msl { version }`（S084。"4.1.0" | "4.0.0"）を呼び、
//! 取得して展開し、sidecar を新しい `--library` で作り直す（版を替えるときも同じ）。取得中はイベント `msl-progress`
//! （`{received, total}`。total は Content-Length、無ければ null）を出し、command `cancel_msl` で取り消せる。
//! ファイル（S141）: dialog / fs プラグインを入れる（UI の「開く」「保存」。権限は capabilities/desktop.json）。
//! 最近使ったファイル（S154）: dialog を通さないパスは fs の許可の外なので、command `read_recent { path }` が読む（`.mo` の通常のファイルだけ）。
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
mod shell;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{Value, json};
use shell::{
    Libraries, MslDir, NOTIFY_METHODS, RPC_METHODS, Shared, allow, install_msl, lock, names,
};
use tauri::{Emitter, Manager, State};
use worker::{Sink, Worker};

/// 要求を 1 つ頼み、`result` を返す（JSON-RPC のエラーは Err の文字列）。
#[tauri::command]
async fn rpc(state: State<'_, Shared>, method: String, params: Value) -> Result<Value, String> {
    allow(&method, &RPC_METHODS)?;
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
    allow(&method, &NOTIFY_METHODS)?;
    let w = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || lock(&w).notify(&method, params))
        .await
        .map_err(|e| e.to_string())?
}

/// `globe-mode.toml` を読み直す（S143。LSP の `workspace/didChangeConfiguration`）。誤りは `window/showMessage` の通知で UI に届く。
#[tauri::command]
async fn reload_config(state: State<'_, Shared>) -> Result<(), String> {
    let w = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        lock(&w).notify("workspace/didChangeConfiguration", json!({"settings": {}}))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 最近使ったファイルを開く（S154、ADR-0023）。dialog を通していないパスは fs プラグインの許可の外なので、Rust が読む。
/// パスは UI から来るので、拡張子が `.mo` の存在する通常のファイル（ディレクトリ・デバイス・無いパスは Err）だけを読み、
/// 任意のファイルを UI の JS に渡さない。相対パスは作業ディレクトリから。
fn read_mo(path: &str) -> Result<String, String> {
    let p = std::path::Path::new(path);
    if p.extension().is_none_or(|e| e != "mo") {
        return Err(format!("{path}: 拡張子が .mo でない"));
    }
    if !std::fs::metadata(p).is_ok_and(|m| m.is_file()) {
        return Err(format!("{path}: 通常のファイルでない（または無い）"));
    }
    std::fs::read_to_string(p).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
fn read_recent(path: String) -> Result<String, String> {
    read_mo(&path)
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

#[tauri::command]
fn libraries(state: State<'_, Libraries>) -> Vec<String> {
    names(&state)
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

/// ワークスペースの根（S143）: `--root <DIR>`、無ければ `GLOBE_ROOT`、無ければカレントディレクトリ。
/// sidecar はここから上へ `globe-mode.toml` を探す。
fn root_from_env() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let arg = args
        .windows(2)
        .find(|p| p[0] == "--root")
        .map(|p| p[1].clone());
    arg.map(PathBuf::from)
        .or_else(|| std::env::var_os("GLOBE_ROOT").map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())
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
        .plugin(tauri_plugin_dialog::init()) // S141: 開く・保存（権限は capabilities/desktop.json）
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            rpc,
            notify,
            cancel_run,
            reload_config,
            libraries,
            fetch_msl,
            cancel_msl,
            bench,
            updater_configured,
            read_recent
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
            let dir = MslDir::new(
                app.path()
                    .app_data_dir()
                    .map_err(|e| e.to_string())?
                    .join("msl"),
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
            let mut worker = Worker::new(sidecar(), libs, sink);
            if let Some(root) = root_from_env() {
                worker = worker.with_root(root);
            }
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

#[cfg(test)]
mod tests {
    use super::read_mo;

    #[test]
    fn read_mo_reads_only_existing_mo_files() {
        let d = std::env::temp_dir().join(format!("globe-read-mo-{}", std::process::id()));
        std::fs::create_dir_all(d.join("dir.mo")).unwrap();
        std::fs::write(d.join("A.mo"), "model A end A;").unwrap();
        std::fs::write(d.join("secret.txt"), "x").unwrap();
        assert_eq!(
            read_mo(d.join("A.mo").to_str().unwrap()).unwrap(),
            "model A end A;"
        );
        assert!(read_mo(d.join("secret.txt").to_str().unwrap()).is_err()); // 拡張子
        assert!(read_mo(d.join("dir.mo").to_str().unwrap()).is_err()); // ディレクトリ
        assert!(read_mo(d.join("none.mo").to_str().unwrap()).is_err()); // 無い
        std::fs::remove_dir_all(d).ok();
    }
}

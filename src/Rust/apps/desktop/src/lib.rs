//! GlobeMode の Android の入口（S092、ADR-0023 の「Android」）。
//! Android では sidecar（別プロセスの `globe`）を起動できないので、command を出さず UI を表示するだけ。
//! 検査と実行は UI がブラウザと同じ `WorkerBackend`（wasm）で行う。Desktop の入口は main.rs（ここは空）。

#[cfg(mobile)]
#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("tauri の起動に失敗");
}

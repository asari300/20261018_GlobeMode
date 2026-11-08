//! main.rs の tauri に依存しない部分（S197）。MSL の切り替えと、UI から中継する method の許可。ここだけ手元で試験できる。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::msl;
use crate::worker::Worker;

pub type Shared = Arc<Mutex<Worker>>;

pub fn lock(w: &Shared) -> MutexGuard<'_, Worker> {
    w.lock().unwrap_or_else(|e| e.into_inner())
}

/// UI が `rpc` で頼む method（`src/Svelte/src/lib/tauri-backend.ts`）。試験が UI の呼び出しと突き合わせる。
pub const RPC_METHODS: [&str; 20] = [
    "globe/check",
    "globe/completion",
    "globe/definition",
    "globe/exportFmu",
    "globe/format",
    "globe/graph",
    "globe/hover",
    "globe/librarySearch",
    "globe/librarySource",
    "globe/libraryTree",
    "globe/lint",
    "globe/models",
    "globe/readResults",
    "globe/references",
    "globe/rename",
    "globe/simulate",
    "globe/simulateColumns",
    "globe/solvers",
    "globe/tunableParameters",
    "textDocument/foldingRange",
];
/// UI が `notify` で送る method
pub const NOTIFY_METHODS: [&str; 1] = ["textDocument/didOpen"];

/// UI の `invoke` は任意の method を言えるので、UI が使うものだけを sidecar に中継する（多層防御。ADR-0023）。
pub fn allow(method: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&method) {
        Ok(())
    } else {
        Err(format!("{method}: 中継を許していない method"))
    }
}

/// sidecar に渡しているライブラリのパス（UI の `TauriBackend.library` が「sidecar が既に読んでいるか」を知るため）。
/// 取得した MSL で増える。
#[derive(Clone)]
pub struct Libraries(pub Arc<Mutex<Vec<PathBuf>>>);

/// MSL の置き場の根（app data dir の `msl`。版ごとに `msl/<version>`）、取得の取り消しの旗、取得中の旗（同時に 1 つだけ）。
#[derive(Clone)]
pub struct MslDir(pub PathBuf, pub Arc<AtomicBool>, pub Arc<AtomicBool>);

impl MslDir {
    pub fn new(root: PathBuf) -> Self {
        MslDir(root, Arc::default(), Arc::default())
    }
}

/// 取得中の旗を、抜けるときに必ず下ろす。
struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn names(libs: &Libraries) -> Vec<String> {
    let libs = libs.0.lock().unwrap_or_else(|e| e.into_inner());
    libs.iter().map(|p| p.display().to_string()).collect()
}

/// その版の MSL を取得して展開し、sidecar をそれだけで作り直す。読んだパスを返す。
/// 外から渡されたライブラリ（`--library`・`GLOBE_LIBRARY`）があれば何もしない。その版を読んでいても何もしない。
/// 取得は同時に 1 つだけ。sidecar を作り直せなかったら、ライブラリの設定を元に戻す。
pub fn install_msl(
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
    if dir.2.swap(true, Ordering::AcqRel) {
        return Err("MSL を取得中（終わってからやり直す）".into());
    }
    let _busy = Busy(&dir.2);
    dir.1.store(false, Ordering::Relaxed);
    msl::fetch(&target, version, progress, &dir.1)?;
    let mut w = lock(w);
    w.set_libraries(vec![target.clone()]);
    if let Err(e) = w.ensure_live() {
        w.set_libraries(current); // 読めない MSL は UI に返し、元のライブラリに戻す（次の要求で作り直す）
        return Err(e);
    }
    *libs.0.lock().unwrap_or_else(|e| e.into_inner()) = vec![target];
    Ok(names(libs))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI の `method: "…"` を拾う。
    fn ui_methods() -> Vec<String> {
        let ts = include_str!("../../../../Svelte/src/lib/tauri-backend.ts");
        ts.split("method: \"")
            .skip(1)
            .map(|r| r[..r.find('"').unwrap()].to_string())
            .collect()
    }

    #[test]
    fn relayed_methods_are_exactly_what_the_ui_sends() {
        let mut ui = ui_methods();
        ui.sort();
        ui.dedup();
        let mut allowed: Vec<String> = RPC_METHODS
            .iter()
            .chain(&NOTIFY_METHODS)
            .map(|s| s.to_string())
            .collect();
        allowed.sort();
        assert_eq!(ui, allowed, "tauri-backend.ts と shell.rs の一覧が食い違う");
        assert!(allow("globe/check", &RPC_METHODS).is_ok());
        assert!(allow("shutdown", &RPC_METHODS).is_err());
        assert!(allow("textDocument/didOpen", &RPC_METHODS).is_err());
        assert!(allow("$/cancelRequest", &NOTIFY_METHODS).is_err());
    }

    /// Tauri の CSP は index.html の meta と同じに、IPC の `connect-src` を足したもの。
    #[test]
    fn tauri_csp_is_the_index_html_csp_plus_ipc() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let html = include_str!("../../../../Svelte/index.html");
        let meta = html
            .split("http-equiv=\"Content-Security-Policy\" content=\"")
            .nth(1)
            .unwrap();
        let meta = &meta[..meta.find('"').unwrap()];
        let expect = meta.replace(
            "connect-src 'self'",
            "connect-src 'self' ipc: http://ipc.localhost",
        );
        assert_eq!(conf["app"]["security"]["csp"], expect);
    }

    fn state(root: &std::path::Path, program: &str) -> (MslDir, Libraries, Shared) {
        let w = Worker::new(program.into(), vec![], Arc::new(|_| {}));
        (
            MslDir::new(root.to_path_buf()),
            Libraries(Arc::default()),
            Arc::new(Mutex::new(w)),
        )
    }

    #[test]
    fn failed_restart_restores_libraries_and_releases_the_flag() {
        let root = std::env::temp_dir().join(format!("globe-shell-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = msl::dest(&root, "4.1.0");
        std::fs::create_dir_all(&dest).unwrap();
        msl::mark_ready(&dest, "4.1.0"); // 取得は済んでいる。sidecar だけが立ち上がらない
        let (dir, libs, w) = state(&root, "/nonexistent/globe");
        assert!(install_msl(&dir, &libs, &w, "4.1.0", |_, _| {}).is_err());
        assert!(names(&libs).is_empty());
        assert!(lock(&w).libraries().is_empty());
        assert!(!dir.2.load(Ordering::Acquire), "取得中の旗が残った");
        // 取得中なら 2 つ目は入らない
        dir.2.store(true, Ordering::Release);
        let e = install_msl(&dir, &libs, &w, "4.1.0", |_, _| {}).unwrap_err();
        assert!(e.contains("取得中"), "{e}");
        let _ = std::fs::remove_dir_all(root);
    }
}

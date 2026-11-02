//! GlobeMode の Erlang NIF（S082、Rustler）。facade `globe-mode` の `Workspace` を Elixir・Gleam から使う。
//!
//! 入出力は `globe-mode-py`・`globe-mode-c` と同じく JSON の文字列で、Elixir 側（`src/Elixir/globe_mode`）が
//! 標準の `JSON` でデコードする。誤りは `{:error, 理由}`。Erlang のモジュール名は `globe_mode_nif`。

use std::path::PathBuf;
use std::sync::Mutex;

use rustler::{Resource, ResourceArc};

/// ワークスペース（BEAM の resource）。salsa のデータベースは Sync でないので Mutex で 1 つずつ使う。
#[derive(Debug)]
struct Ws(Mutex<globe_mode::Workspace>);

#[rustler::resource_impl]
impl Resource for Ws {}

fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON: {e}"))
}

fn with<T>(ws: &Ws, f: impl FnOnce(&mut globe_mode::Workspace) -> T) -> Result<T, String> {
    let mut guard =
        ws.0.lock()
            .map_err(|_| "ワークスペースが panic で壊れた".to_string())?;
    Ok(f(&mut guard))
}

/// `libraries` はライブラリのディレクトリ（MSL など。`[]` でライブラリ無し）。読めなければ診断の JSON。
#[rustler::nif(schedule = "DirtyIo")]
fn new(libraries: Vec<String>) -> Result<ResourceArc<Ws>, String> {
    let libs: Vec<PathBuf> = libraries.into_iter().map(PathBuf::from).collect();
    let inner = if libs.is_empty() {
        globe_mode::Workspace::new()
    } else {
        globe_mode::Workspace::with_libraries(&libs).map_err(|ds| json(&ds).unwrap_or_default())?
    };
    Ok(ResourceArc::new(Ws(Mutex::new(inner))))
}

#[rustler::nif]
fn set_file(ws: ResourceArc<Ws>, path: String, text: String) -> Result<(), String> {
    with(&ws, |w| w.set_file(&path, text))
}

/// ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。
#[rustler::nif(schedule = "DirtyIo")]
fn load_dir(ws: ResourceArc<Ws>, dir: String) -> Result<usize, String> {
    with(&ws, |w| w.load_dir(std::path::Path::new(&dir)))?.map_err(|e| format!("{dir}: {e}"))
}

/// 検査（S0〜S10）の結果（`CheckResponse` の JSON）。
#[rustler::nif(schedule = "DirtyCpu")]
fn check(ws: ResourceArc<Ws>, model: String) -> Result<String, String> {
    with(&ws, |w| json(&w.check(&model)))?
}

/// connect グラフ（`ConnectGraph` の JSON）。モデルが索引に無ければ `{:error, 理由}`。
#[rustler::nif(schedule = "DirtyCpu")]
fn graph(ws: ResourceArc<Ws>, model: String, depth: u32) -> Result<String, String> {
    with(&ws, |w| {
        json(&w.graph(&model, depth).map_err(|e| e.to_string())?)
    })?
}

/// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。`settings_json` は `{"stop_time": 2}` など。
#[rustler::nif(schedule = "DirtyCpu")]
fn simulate(ws: ResourceArc<Ws>, model: String, settings_json: String) -> Result<String, String> {
    let settings: serde_json::Value = serde_json::from_str(&settings_json)
        .map_err(|e| format!("settings の JSON が読めない: {e}"))?;
    let request =
        serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
            .map_err(|e| format!("settings が読めない: {e}"))?;
    with(&ws, |w| json(&w.simulate(&request)))?
}

/// ソルバの登録表（JSON の配列）。
#[rustler::nif]
fn solvers() -> Result<String, String> {
    json(globe_mode::solvers())
}

rustler::init!("globe_mode_nif");

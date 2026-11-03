//! GlobeMode の Erlang NIF（S082、Rustler）。facade `globe-mode` の `Workspace` を Elixir・Gleam から使う。
//!
//! 入出力は `globe-mode-py`・`globe-mode-c` と同じく JSON の文字列で、Elixir 側（`src/Elixir/globe_mode`）が
//! 標準の `JSON` でデコードする。誤りは `{:error, 理由}`。Erlang のモジュール名は `globe_mode_nif`。
//! 進捗と中断（S131、ADR-0033）は `simulate_observed`・`cancel_token`・`cancel`。

use std::path::PathBuf;
use std::sync::Mutex;

use rustler::{Binary, Env, LocalPid, OwnedBinary, Resource, ResourceArc};

/// ワークスペース（BEAM の resource）。salsa のデータベースは Sync でないので Mutex で 1 つずつ使う。
#[derive(Debug)]
struct Ws(Mutex<globe_mode::Workspace>);

#[rustler::resource_impl]
impl Resource for Ws {}

/// 中断の合図（ADR-0033）。`simulate_observed` を回している間に別のプロセスから `cancel` で立てる。
#[derive(Debug)]
struct Token(globe_mode::CancelToken);

#[rustler::resource_impl]
impl Resource for Token {}

rustler::atoms! { globe_progress }

/// 進捗のメッセージの本体（`%{stage: "integration", time: t | nil, start_time: ..., stop_time: ...}`）。
#[derive(rustler::NifMap)]
struct ProgressMsg {
    stage: String,
    time: Option<f64>,
    start_time: f64,
    stop_time: f64,
}

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

fn request(model: String, settings_json: &str) -> Result<globe_mode::SimulateRequest, String> {
    let settings: serde_json::Value = serde_json::from_str(settings_json)
        .map_err(|e| format!("settings の JSON が読めない: {e}"))?;
    serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
        .map_err(|e| format!("settings が読めない: {e}"))
}

/// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。`settings_json` は `{"stop_time": 2}` など。
#[rustler::nif(schedule = "DirtyCpu")]
fn simulate(ws: ResourceArc<Ws>, model: String, settings_json: String) -> Result<String, String> {
    let request = request(model, &settings_json)?;
    with(&ws, |w| json(&w.simulate(&request)))?
}

/// 新しい中断の合図を作る（S131）。
#[rustler::nif]
fn cancel_token() -> ResourceArc<Token> {
    ResourceArc::new(Token(globe_mode::CancelToken::new()))
}

/// 合図を立てる。`simulate_observed` は次の区切りで止まる（S131）。
#[rustler::nif]
fn cancel(token: ResourceArc<Token>) -> rustler::Atom {
    token.0.cancel();
    rustler::types::atom::ok()
}

/// [`simulate`] と同じだが、進捗のたびに `pid` へ `{:globe_progress, map}` を送り、`token` が立てられたら次の区切りで
/// 止める（S131、ADR-0033）。積分の途中なら `termination` が `cancelled` で途中までの CSV、前なら `ok: false`。どちらも
/// `GM-I11004`。トークンを先に作って渡すのは、DirtyCpu の中で回っている間に返せる値が無いため。
#[rustler::nif(schedule = "DirtyCpu")]
fn simulate_observed(
    env: Env,
    ws: ResourceArc<Ws>,
    model: String,
    settings_json: String,
    pid: LocalPid,
    token: ResourceArc<Token>,
) -> Result<String, String> {
    let request = request(model, &settings_json)?;
    let mut notify = |p: &globe_mode::Progress| {
        let msg = ProgressMsg {
            stage: format!("{:?}", p.stage).to_lowercase(),
            time: p.time,
            start_time: p.start_time,
            stop_time: p.stop_time,
        };
        // 受け手が死んでいても実行は続ける（止めるのはトークン）
        let _ = env.send(&pid, (globe_progress(), msg));
    };
    let handle = globe_mode::SimulateHandle {
        on_progress: Some(&mut notify),
        cancel: Some(token.0.clone()),
    };
    with(&ws, |w| {
        json(
            &w.simulate_observed(&request, &globe_mode::now_ms, handle)
                .response,
        )
    })?
}

/// 列の道（S116。Python の `simulate_columns` と同じ並び）。`{layout_json, [binary]}` で、layout は
/// `{"response": SimulateResponse（csv 無し）, "names": ["time", ...], "points": n}`、binary は列ごとの f64（little-endian）。
#[rustler::nif(schedule = "DirtyCpu")]
fn simulate_columns<'a>(
    env: Env<'a>,
    ws: ResourceArc<Ws>,
    model: String,
    settings_json: String,
) -> Result<(String, Vec<Binary<'a>>), String> {
    let mut request = request(model, &settings_json)?;
    request.no_csv = true;
    let simulated = with(&ws, |w| w.simulate_with(&request, &globe_mode::now_ms))?;
    let (layout, columns) = simulated.columns();
    let mut binaries = Vec::with_capacity(columns.len());
    for c in columns {
        let mut b = OwnedBinary::new(c.len() * 8);
        for (dst, x) in b.as_mut_slice().as_chunks_mut::<8>().0.iter_mut().zip(c) {
            *dst = x.to_le_bytes();
        }
        binaries.push(b.release(env));
    }
    let text = json(&serde_json::json!({
        "response": simulated.response,
        "names": layout.names,
        "points": layout.points,
    }))?;
    Ok((text, binaries))
}

/// FMU を書き出し（S118。`globe fmu export` と同じ）、`.fmu` のパスを返す。`options_json` は
/// `{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}`（省略した鍵は既定）。
#[rustler::nif(schedule = "DirtyIo")]
fn export_fmu(
    ws: ResourceArc<Ws>,
    model: String,
    out_dir: String,
    options_json: String,
) -> Result<String, String> {
    let options: globe_mode::FmuOptions =
        serde_json::from_str(&options_json).map_err(|e| format!("options が読めない: {e}"))?;
    let path = with(&ws, |w| {
        w.export_fmu(&model, std::path::Path::new(&out_dir), &options)
    })??;
    Ok(path.display().to_string())
}

/// ソルバの登録表（JSON の配列）。
#[rustler::nif]
fn solvers() -> Result<String, String> {
    json(globe_mode::solvers())
}

/// 本文を整形する（S111）。文法が壊れていれば `{:error, 理由}`。
#[rustler::nif]
fn format(text: String, indent: usize, insert_origin: bool) -> Result<String, String> {
    globe_mode::format(&text, indent, insert_origin)
}

/// lint の結果（`[{rule, line, column, message}]` の JSON。S111）。
#[rustler::nif]
fn lint(text: String) -> String {
    globe_mode::lint(&text)
}

rustler::init!("globe_mode_nif");

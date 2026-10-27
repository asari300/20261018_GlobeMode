//! GlobeMode の wasm の束ね（S012）。ブラウザと Node で、検査（S0〜S10）→ DAE まで変換 → 実行を回す。
//!
//! ネイティブの `globe simulate` と同じ道（`globe-mode-db` の `check_model` → `compiled_model` →
//! `globe_mode_exec::run`）を通る。exec は wasm32 では Rumoca の session の道を使う（std の
//! `Instant::now` が panic するため。`globe-mode-exec/src/rumoca/session.rs`）。
//!
//! JS から使う形:
//!
//! ```js
//! const ws = new Workspace(JSON.stringify(mslFiles), "MSL");   // ライブラリは 1 回だけ読む（省略可）
//! ws.set_file("Decay.mo", text);
//! const res = JSON.parse(ws.simulate(JSON.stringify({ model: "Decay", settings: { solver: "bdf" } })));
//! ```
//!
//! wasm32-unknown-unknown には ファイルシステムが無いので、ライブラリ（MSL など）はファイルの組
//! （名前 → 本文）で渡す。[`Workspace::with_library`] は Rumoca の in-memory の組（DurableExternal）として
//! 読むので、ライブラリの文書を検査のたびに渡し直さない。ライブラリを文書として `files` に入れることも
//! できる（遅い。`globe-mode-db` が `deps_closure` で使うファイルだけを渡す）。
//!
//! 入出力は JSON の文字列（[`Request`] と [`Response`]）。Rust からは同じ処理を [`Workspace::run`] と
//! [`run`] で呼べる（ネイティブの試験のため。JS 向けの関数は `JsValue` を作るので wasm32 でしか呼べない）。
//!
//! wasm32-unknown-unknown は panic=abort なので、panic は JS では `RuntimeError: unreachable` になる。
//! panic の文言は [`last_panic`] で読める（モジュールを読み込んだときに panic の hook を差す）。
//! panic の後の `Workspace` は使えない（作り直す）。

use std::collections::BTreeMap;
use std::sync::Mutex;

use globe_mode_core::Diagnostic;
use globe_mode_db::GlobeDb;
use globe_mode_frontend::Source;
use globe_mode_ir::{ResolvedSettings, SimSettings, SimStats, Termination};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// [`Workspace::run`] の入力。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// 実行の前にワークスペースに足す（差し替える）ファイル（名前 → 本文）。ライブラリを文書として
    /// 入れてもよい
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// モデルの完全修飾名
    pub model: String,
    /// 実行の設定（None の項目は experiment annotation か既定値）
    #[serde(default)]
    pub settings: SimSettings,
    /// S0〜S10 に Error があっても実行する
    #[serde(default)]
    pub force: bool,
    /// 結果の CSV を返さない（大きな結果で JSON を小さくするため。最後の値は `final` に入る）
    #[serde(default)]
    pub no_csv: bool,
}

/// [`Workspace::run`] の出力。
#[derive(Debug, Serialize)]
pub struct Response {
    pub model: String,
    /// 結果があり、Error の診断が無い
    pub ok: bool,
    /// 実行しなかった・失敗した理由（成功なら None）
    pub refused: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub settings: Option<ResolvedSettings>,
    pub termination: Option<Termination>,
    pub stats: Option<SimStats>,
    /// 最後の時刻の値（変数名 → 値。`globe simulate --json` の `final` と同じ）
    #[serde(rename = "final")]
    pub final_values: Option<BTreeMap<String, f64>>,
    /// 結果の CSV（`globe_mode_sim::store::write_csv` と同じ形。`globe simulate -o` と同じ）
    pub csv: Option<String>,
    /// 段階ごとの壁時計（ミリ秒）: load / check / compile / run
    pub timings_ms: BTreeMap<&'static str, f64>,
    /// 返す時点の wasm の線形メモリ（バイト。ネイティブでは 0）
    pub memory_bytes: f64,
}

/// 壁時計（ミリ秒）。wasm32-unknown-unknown では std の `Instant::now` が panic するので JS の
/// `performance.now()`（Node・ブラウザ・Worker にある。`Date.now()` は 1 ms 刻みなので使わない）を使う。
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        performance_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1e3)
    }
}

/// ライブラリを 1 回だけ読み、作業中のファイルを出し入れしながら何度も実行する。
#[wasm_bindgen]
pub struct Workspace {
    db: GlobeDb,
    library_files: usize,
    library_load_ms: f64,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("files", &self.db.paths().count())
            .field("library_files", &self.library_files)
            .finish()
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Rust から呼ぶ形。
impl Workspace {
    /// ライブラリ無しのワークスペース。
    pub fn new() -> Self {
        Workspace {
            db: GlobeDb::new(),
            library_files: 0,
            library_load_ms: 0.0,
        }
    }

    /// ライブラリ（名前 → 本文）を一度だけ読み込んだワークスペース。読めなければ S1 の診断（`GM-E1003`）。
    pub fn with_library(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Workspace, Vec<Diagnostic>> {
        let t = now_ms();
        let sources: Vec<Source<'_>> = files
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let db = GlobeDb::with_library_sources(name, &sources)?;
        Ok(Workspace {
            db,
            library_files: files.len(),
            library_load_ms: now_ms() - t,
        })
    }

    /// 検査 → コンパイル → 実行。`request.files` は先にワークスペースに足す（残る）。
    pub fn run(&mut self, request: &Request) -> Response {
        let mut timings = BTreeMap::new();
        let mut t = now_ms();
        let mut lap = |name: &'static str, timings: &mut BTreeMap<&'static str, f64>| {
            let n = now_ms();
            timings.insert(name, n - t);
            t = n;
        };
        let model = request.model.as_str();
        let mut response = Response {
            model: model.to_string(),
            ok: false,
            refused: None,
            diagnostics: Vec::new(),
            settings: None,
            termination: None,
            stats: None,
            final_values: None,
            csv: None,
            timings_ms: BTreeMap::new(),
            memory_bytes: 0.0,
        };
        let db = &mut self.db;
        for (name, text) in &request.files {
            db.set_file(name, text.clone());
        }
        lap("load", &mut timings);
        let checked = db.check_model(model);
        lap("check", &mut timings);
        response.diagnostics.extend(checked.diagnostics.clone());
        if let Some(stage) = checked.failed_stage()
            && !request.force
        {
            response.refused = Some(format!(
                "実行しない（{stage:?} {} で Error。force で強行できる）",
                stage.title()
            ));
            return finish(response, timings);
        }
        let compiled = match db.compiled_model_with_overrides(model, &request.settings.parameters) {
            Ok(c) => c,
            Err(ds) => {
                response.diagnostics.extend(ds);
                response.refused = Some("DAE まで変換できないので実行できない".to_string());
                return finish(response, timings);
            }
        };
        lap("compile", &mut timings);
        let report = globe_mode_exec::run(&compiled, &request.settings);
        lap("run", &mut timings);
        response.ok = report.succeeded();
        response.diagnostics.extend(report.diagnostics);
        response.settings = report.settings;
        if let Some(run) = report.run {
            let traj = &run.trajectory;
            response.final_values = Some(
                traj.variables
                    .iter()
                    .filter_map(|v| v.values.last().map(|x| (v.name.clone(), *x)))
                    .collect(),
            );
            if !request.no_csv {
                let mut buf = Vec::new();
                if globe_mode_sim::store::write_csv(traj, &mut buf).is_ok() {
                    response.csv = String::from_utf8(buf).ok();
                }
            }
            response.termination = Some(run.termination);
            response.stats = Some(run.stats);
        } else {
            response.refused = Some("実行に失敗した".to_string());
        }
        finish(response, timings)
    }

    /// ファイルを足すか、本文を差し替える。
    pub fn put_file(&mut self, name: &str, text: impl Into<String>) {
        self.db.set_file(name, text);
    }

    /// ファイルを除く。無ければ false。
    pub fn delete_file(&mut self, name: &str) -> bool {
        self.db.remove_file(name)
    }

    /// 検査器（DB）。
    pub fn db(&mut self) -> &mut GlobeDb {
        &mut self.db
    }
}

fn finish(mut response: Response, timings: BTreeMap<&'static str, f64>) -> Response {
    response.timings_ms = timings;
    response.memory_bytes = memory_bytes();
    response
}

/// 一度きりの実行（Rust から呼ぶ形）。ライブラリ無しのワークスペースを作り、`request.files` を足して回す。
pub fn run(request: &Request) -> Response {
    Workspace::new().run(request)
}

fn js_error(message: String) -> JsValue {
    JsValue::from_str(&message)
}

fn parse_request(request_json: &str) -> Result<Request, JsValue> {
    serde_json::from_str(request_json).map_err(|e| js_error(format!("入力の JSON が読めない: {e}")))
}

fn to_json<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|e| js_error(format!("JSON: {e}")))
}

/// JS から呼ぶ形（入出力は JSON の文字列）。
#[wasm_bindgen]
impl Workspace {
    /// `library_json` はライブラリのファイルの組（`{"名前": "本文", …}` の JSON。省略するとライブラリ無し）、
    /// `name` はその組の名前（診断に出す。既定は `library`）。読めなければ診断の配列の JSON を例外で投げる。
    #[wasm_bindgen(constructor)]
    pub fn js_new(
        library_json: Option<String>,
        name: Option<String>,
    ) -> Result<Workspace, JsValue> {
        let Some(json) = library_json else {
            return Ok(Workspace::new());
        };
        let files: BTreeMap<String, String> = serde_json::from_str(&json)
            .map_err(|e| js_error(format!("ライブラリの JSON が読めない: {e}")))?;
        Workspace::with_library(name.as_deref().unwrap_or("library"), &files)
            .map_err(|ds| js_error(serde_json::to_string(&ds).unwrap_or_default()))
    }

    /// ファイルを足すか、本文を差し替える。
    pub fn set_file(&mut self, name: &str, text: String) {
        self.put_file(name, text);
    }

    /// ファイルを除く。無ければ false。
    pub fn remove_file(&mut self, name: &str) -> bool {
        self.delete_file(name)
    }

    /// ワークスペースのファイルの名前（JSON の配列。区切りは `/`、昇順）。
    pub fn files(&self) -> String {
        serde_json::to_string(&self.db.paths().collect::<Vec<_>>()).unwrap_or_default()
    }

    /// 読み込んだライブラリのファイルの数。
    #[wasm_bindgen(getter)]
    pub fn library_files(&self) -> usize {
        self.library_files
    }

    /// ライブラリを読み込むのにかかった時間（ミリ秒）。
    #[wasm_bindgen(getter)]
    pub fn library_load_ms(&self) -> f64 {
        self.library_load_ms
    }

    /// 検査 → コンパイル → 実行（[`Request`] の JSON → [`Response`] の JSON）。入力の JSON が読めなければ
    /// 例外（文字列）を投げる。モデルの誤りや実行の失敗は例外にせず、出力の `ok` と `diagnostics` で返す。
    pub fn simulate(&mut self, request_json: &str) -> Result<String, JsValue> {
        let request = parse_request(request_json)?;
        to_json(&self.run(&request))
    }
}

/// 一度きりの実行（JS から呼ぶ形）。ライブラリ無しで、`files` にすべてを入れる。
#[wasm_bindgen]
pub fn simulate(request_json: &str) -> Result<String, JsValue> {
    let request = parse_request(request_json)?;
    to_json(&run(&request))
}

/// ソルバの登録表（`globe_mode_sim::solvers::SOLVERS` の JSON の配列）。
#[wasm_bindgen]
pub fn solvers() -> String {
    solvers_json()
}

/// [`solvers`] の中身（Rust から呼ぶ形）。
pub fn solvers_json() -> String {
    serde_json::to_string(globe_mode_sim::solvers::SOLVERS).unwrap_or_default()
}

/// 版（Cargo の版）。
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// wasm の線形メモリの大きさ（バイト）。線形メモリは縮まないので、呼んだ時点までの最大と同じ。
#[wasm_bindgen]
pub fn memory_bytes() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        (core::arch::wasm32::memory_size(0) * 65536) as f64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

/// 最後の panic の文言（場所を含む）。panic していなければ undefined。
#[wasm_bindgen]
pub fn last_panic() -> Option<String> {
    LAST_PANIC.lock().ok().and_then(|g| g.clone())
}

/// panic の hook を試すために、わざと panic する（試験用）。
#[doc(hidden)]
#[wasm_bindgen]
pub fn panic_for_test(message: &str) {
    panic!("{message}");
}

static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

/// panic の hook を差す（文言を残し、JS の console.error にも出す）。依存を足さずに
/// console_error_panic_hook と同じことをする。モジュールを読み込んだときに一度だけ呼ばれる。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = info.to_string();
        if let Ok(mut g) = LAST_PANIC.lock() {
            *g = Some(message.clone());
        }
        console_error(&message);
    }));
}

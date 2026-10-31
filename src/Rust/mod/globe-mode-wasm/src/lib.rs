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
//! 入出力は JSON の文字列（[`Request`] と [`Response`]。S035 から `globe-mode-protocol` の型で、中身は facade
//! `globe-mode` の `Workspace::simulate_with`）。Rust からは同じ処理を [`Workspace::run`] と
//! [`run`] で呼べる（ネイティブの試験のため。JS 向けの関数は `JsValue` を作るので wasm32 でしか呼べない）。
//!
//! wasm32-unknown-unknown は panic=abort なので、panic は JS では `RuntimeError: unreachable` になる。
//! panic の文言は [`last_panic`] で読める（モジュールを読み込んだときに panic の hook を差す）。
//! panic の後の `Workspace` は使えない（作り直す）。

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use globe_mode::{Diagnostic, Simulated};
pub use globe_mode::{SimulateRequest as Request, SimulateResponse as Response};

/// 壁時計（ミリ秒）。wasm32-unknown-unknown では std の `Instant::now` が panic するので JS の
/// `performance.now()`（Node・ブラウザ・Worker にある。`Date.now()` は 1 ms 刻みなので使わない）を使う。
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        performance_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        globe_mode::now_ms()
    }
}

/// ライブラリを 1 回だけ読み、作業中のファイルを出し入れしながら何度も実行する（facade の
/// [`globe_mode::Workspace`] に JS の時計とメモリの大きさを足したもの）。
#[wasm_bindgen]
#[derive(Debug)]
pub struct Workspace {
    inner: globe_mode::Workspace,
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
            inner: globe_mode::Workspace::new(),
        }
    }

    /// ライブラリ（名前 → 本文）を一度だけ読み込んだワークスペース。読めなければ S1 の診断（`GM-E1003`）。
    pub fn with_library(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Workspace, Vec<Diagnostic>> {
        // 読み込みの時間は facade の時計（wasm32 では 0）でなく、ここで測る
        let t = now_ms();
        let mut inner = globe_mode::Workspace::with_library_sources(name, files)?;
        inner.set_library_load_ms(now_ms() - t);
        Ok(Workspace { inner })
    }

    /// 検査 → コンパイル → 実行。`request.files` は先にワークスペースに足す（残る）。
    pub fn run(&mut self, request: &Request) -> Response {
        let Simulated { mut response, .. } = self.inner.simulate_with(request, &now_ms);
        response.memory_bytes = memory_bytes();
        response
    }

    /// ファイルを足すか、本文を差し替える。
    pub fn put_file(&mut self, name: &str, text: impl Into<String>) {
        self.inner.set_file(name, text);
    }

    /// ファイルを除く。無ければ false。
    pub fn delete_file(&mut self, name: &str) -> bool {
        self.inner.remove_file(name)
    }

    /// facade のワークスペース（検査やグラフも呼べる）。
    pub fn inner(&mut self) -> &mut globe_mode::Workspace {
        &mut self.inner
    }
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
        serde_json::to_string(&self.inner.files().collect::<Vec<_>>()).unwrap_or_default()
    }

    /// 読み込んだライブラリのファイルの数。
    #[wasm_bindgen(getter)]
    pub fn library_files(&self) -> usize {
        self.inner.library_files()
    }

    /// ライブラリを読み込むのにかかった時間（ミリ秒）。
    #[wasm_bindgen(getter)]
    pub fn library_load_ms(&self) -> f64 {
        self.inner.library_load_ms()
    }

    /// 検査 → コンパイル → 実行（[`Request`] の JSON → [`Response`] の JSON）。入力の JSON が読めなければ
    /// 例外（文字列）を投げる。モデルの誤りや実行の失敗は例外にせず、出力の `ok` と `diagnostics` で返す。
    pub fn simulate(&mut self, request_json: &str) -> Result<String, JsValue> {
        let request = parse_request(request_json)?;
        to_json(&self.run(&request))
    }

    /// モデルの検査だけ（S0〜S10。実行しない）。`CheckResponse` の JSON を返す（S041。ブラウザ UI の診断パネル用）。
    pub fn check(&mut self, model: &str) -> Result<String, JsValue> {
        to_json(&self.inner.check(model))
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
    serde_json::to_string(globe_mode::solvers()).unwrap_or_default()
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

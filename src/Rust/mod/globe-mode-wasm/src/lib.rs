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

mod fmt;

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

/// 木の項目の JSON（ライブラリが無ければ `[]`）。
fn entries_json(entries: Option<Vec<globe_mode_fmt::TreeEntry<'_>>>) -> String {
    let json = globe_mode_lsp::tree_entries_json(&entries.unwrap_or_default());
    serde_json::to_string(&json).unwrap_or_default()
}

/// ライブラリを 1 回だけ読み、作業中のファイルを出し入れしながら何度も実行する（facade の
/// [`globe_mode::Workspace`] に JS の時計とメモリの大きさを足したもの）。
#[wasm_bindgen]
#[derive(Debug)]
pub struct Workspace {
    inner: globe_mode::Workspace,
    /// ライブラリの木と本文（S127。`library_tree`・`library_source`）。
    /// shortcut: 本文を分割読み込みの写しと別に持つ（MSL で約 15 MiB）。facade が本文を返せるようになったらそちらを引く
    tree: Option<(globe_mode_fmt::LibraryTree, BTreeMap<String, String>)>,
    /// 定義・補完が読んだライブラリのファイルのアウトライン（S171。呼び出しをまたいで持つ。MSL の補完で毎回 260 ms かかっていた）
    scratch: std::cell::RefCell<globe_mode::Workspace>,
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
            tree: None,
            scratch: Default::default(),
        }
    }

    /// `definition`・`hover`・`references`・`rename` の共通: ライブラリの木とファイル組で Resolver を作って `f` を回す。
    fn with_resolver<T>(
        &self,
        f: impl FnOnce(&mut globe_mode_lsp::definition::Resolver) -> T,
    ) -> T {
        let empty;
        let (tree, files) = match &self.tree {
            Some((t, f)) => (t, Some(f)),
            None => {
                empty = globe_mode_fmt::LibraryTree::new(std::iter::empty());
                (&empty, None)
            }
        };
        let mut scratch = self.scratch.borrow_mut();
        f(&mut globe_mode_lsp::definition::Resolver {
            ws: &self.inner,
            tree,
            scratch: &mut scratch,
            overlay: None,
            read: &|f| files?.get(f).cloned(),
        })
    }

    /// `definition`・`hover` の共通: `f` の結果を JSON にする。
    fn resolve(
        &self,
        f: fn(
            &mut globe_mode_lsp::definition::Resolver,
            &str,
            &str,
            u32,
            u32,
        ) -> Option<serde_json::Value>,
        file: &str,
        text: &str,
        line: u32,
        character: u32,
    ) -> String {
        let found = self.with_resolver(|r| f(r, file, text, line, character));
        serde_json::to_string(&found).unwrap_or_default()
    }

    /// ライブラリ（名前 → 本文）を一度だけ全部読み込んだワークスペース。読めなければ S1 の診断（`GM-E1003`）。
    pub fn with_library(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Workspace, Vec<Diagnostic>> {
        Self::timed(|| globe_mode::Workspace::with_library_sources(name, files))
    }

    /// [`Workspace::with_library`] と同じ入力で、モデルが使うファイルだけを検査のときに読む（S037）。
    /// MSL を全部読んだときの線形メモリ 262 MiB を 62 MiB にする（ブラウザ向け。JS の既定）。
    pub fn with_library_split(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Workspace, Vec<Diagnostic>> {
        Self::timed(|| globe_mode::Workspace::with_library_sources_split(name, files))
    }

    fn timed(
        load: impl FnOnce() -> Result<globe_mode::Workspace, Vec<Diagnostic>>,
    ) -> Result<Workspace, Vec<Diagnostic>> {
        // 読み込みの時間は facade の時計（wasm32 では 0）でなく、ここで測る
        let t = now_ms();
        let mut inner = load()?;
        inner.set_library_load_ms(now_ms() - t);
        Ok(Workspace {
            inner,
            tree: None,
            scratch: Default::default(),
        })
    }

    /// ライブラリの木を引けるようにする（S127。`files` は読み込みに渡したのと同じ組）。
    pub fn with_tree(mut self, files: BTreeMap<String, String>) -> Self {
        let tree =
            globe_mode_fmt::LibraryTree::new(files.iter().map(|(f, t)| (f.as_str(), t.as_str())));
        self.tree = Some((tree, files));
        self
    }

    /// `prefix`（空ならトップレベル）の直下の子（`[{name, path, kind, description, file, hasChildren}]` の JSON）。ライブラリが無ければ `[]`。
    pub fn tree_children(&self, prefix: &str) -> String {
        entries_json(self.tree.as_ref().map(|(t, _)| t.children(prefix)))
    }

    /// 名前の部分一致で最大 `limit` 件（S167。[`globe_mode_fmt::LibraryTree::search`]。形は [`Workspace::tree_children`] と同じ）。
    pub fn tree_search(&self, query: &str, limit: usize) -> String {
        entries_json(self.tree.as_ref().map(|(t, _)| t.search(query, limit)))
    }

    /// `path`（完全修飾名）を定義するライブラリのファイルと本文。
    pub fn tree_source(&self, path: &str) -> Option<(&str, &str)> {
        let (t, files) = self.tree.as_ref()?;
        let file = t.file(path)?;
        Some((file, files.get(file)?.as_str()))
    }

    /// 検査 → コンパイル → 実行。`request.files` は先にワークスペースに足す（残る）。
    pub fn run(&mut self, request: &Request) -> Response {
        let Simulated { mut response, .. } = self.inner.simulate_with(request, &now_ms);
        response.memory_bytes = memory_bytes();
        response
    }

    /// [`Workspace::run`] と同じだが、時系列を CSV にせず列ごとに持つ（S051。`request.no_csv` は無視して CSV を作らない）。
    pub fn run_columns(&mut self, request: Request) -> SimColumns {
        self.run_columns_observed(request, globe_mode::SimulateHandle::default())
    }

    /// [`Workspace::run_columns`] と同じだが、進捗を `handle` に知らせる（ADR-0033、S133）。wasm の Worker は
    /// 1 本のスレッドなので `handle.cancel` は外から立てられない（`simulate_columns` は進捗の関数の戻り値で立てる。S157）。
    pub fn run_columns_observed(
        &mut self,
        mut request: Request,
        handle: globe_mode::SimulateHandle<'_>,
    ) -> SimColumns {
        request.no_csv = true;
        let mut simulated = self.inner.simulate_observed(&request, &now_ms, handle);
        simulated.response.memory_bytes = memory_bytes();
        // 列だけを持つ（コンパイル済みのモデルは放す）
        simulated.compiled = None;
        SimColumns {
            response: serde_json::to_string(&simulated.response).unwrap_or_default(),
            simulated,
        }
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

/// 列ごとの実行の結果（S051）。時系列は wasm の中に残し、JS は列を 1 本ずつ `Float64Array` で受け取る（CSV の文字列を作らず、
/// 解析もしない。コピーは wasm の線形メモリから JS への 1 回）。使い終わったら `free()`。
#[wasm_bindgen]
#[derive(Debug)]
pub struct SimColumns {
    response: String,
    simulated: Simulated,
}

#[wasm_bindgen]
impl SimColumns {
    /// [`Response`] の JSON（`csv` は無い）。
    #[wasm_bindgen(getter)]
    pub fn response(&self) -> String {
        self.response.clone()
    }

    /// [`globe_mode::ColumnsLayout`] の JSON。実行できなかったときは空の列と 0 点。
    pub fn layout(&self) -> String {
        serde_json::to_string(&self.simulated.columns().0).unwrap_or_default()
    }

    /// 列 `i`（0 が時刻、1 から変数）。範囲外は空。
    pub fn column(&self, i: usize) -> Vec<f64> {
        self.simulated
            .columns()
            .1
            .get(i)
            .map_or_else(Vec::new, |c| c.to_vec())
    }
}

/// 開いた結果のファイルの列（S222）。[`SimColumns`] の `layout`・`column` と同じ形。使い終わったら `free()`。
#[wasm_bindgen]
#[derive(Debug)]
pub struct ResultColumns(globe_mode::OpenedResults);

#[wasm_bindgen]
impl ResultColumns {
    /// [`globe_mode::ColumnsLayout`] の JSON。
    pub fn layout(&self) -> String {
        serde_json::to_string(&self.0.columns().0).unwrap_or_default()
    }

    /// 列 `i`（0 が時刻、1 から変数）。範囲外は空。
    pub fn column(&self, i: usize) -> Vec<f64> {
        self.0
            .columns()
            .1
            .get(i)
            .map_or_else(Vec::new, |c| c.to_vec())
    }
}

/// 結果のファイル（`.mat` か `.csv`。`name` の拡張子で読み分ける）を列にする（S222）。読めなければ理由を投げる。
#[wasm_bindgen]
pub fn read_results(name: &str, bytes: &[u8]) -> Result<ResultColumns, JsValue> {
    globe_mode::open_results(name, bytes)
        .map(ResultColumns)
        .map_err(js_error)
}

/// 一度きりの実行（Rust から呼ぶ形）。ライブラリ無しのワークスペースを作り、`request.files` を足して回す。
pub fn run(request: &Request) -> Response {
    Workspace::new().run(request)
}

fn js_error(message: String) -> JsValue {
    JsValue::from_str(&message)
}

/// 実行の入口（`simulate`・`simulate_columns`・自由関数 `simulate`）が最初に呼ぶ。前の panic の文言を消す（#12。消さないと、
/// 後の panic でない例外を `last_panic()` を見るホストが panic と誤る）。
fn parse_request(request_json: &str) -> Result<Request, JsValue> {
    clear_last_panic();
    serde_json::from_str(request_json).map_err(|e| js_error(format!("入力の JSON が読めない: {e}")))
}

fn to_json<T: Serialize>(value: &T) -> Result<String, JsValue> {
    globe_mode::to_json(value).map_err(js_error)
}

/// JS から呼ぶ形（入出力は JSON の文字列）。
#[wasm_bindgen]
impl Workspace {
    /// `library_json` はライブラリのファイルの組（`{"名前": "本文", …}` の JSON。省略するとライブラリ無し）、
    /// `name` はその組の名前（診断に出す。既定は `library`）。読めなければ診断の配列の JSON を例外で投げる。
    ///
    /// ライブラリは分割して読む（S037）: 検査のたびにモデルと文書が使うファイルだけを足す。構文の誤りはそのファイルを
    /// 使うときに分かる。1 ファイルに複数のトップレベルのクラスがあるライブラリは `eager = true` で全部を読む。
    #[wasm_bindgen(constructor)]
    pub fn js_new(
        library_json: Option<String>,
        name: Option<String>,
        eager: Option<bool>,
    ) -> Result<Workspace, JsValue> {
        clear_last_panic();
        let Some(json) = library_json else {
            return Ok(Workspace::new());
        };
        let files: BTreeMap<String, String> = serde_json::from_str(&json)
            .map_err(|e| js_error(format!("ライブラリの JSON が読めない: {e}")))?;
        let name = name.as_deref().unwrap_or("library");
        let loaded = if eager.unwrap_or(false) {
            Workspace::with_library(name, &files)
        } else {
            Workspace::with_library_split(name, &files)
        };
        loaded
            .map(|ws| ws.with_tree(files))
            .map_err(|ds| js_error(serde_json::to_string(&ds).unwrap_or_default()))
    }

    /// ライブラリの木の `prefix` の直下の子（S127。[`Workspace::tree_children`]）。
    pub fn library_tree(&self, prefix: &str) -> String {
        self.tree_children(prefix)
    }

    /// ライブラリのクラスを名前の部分一致で探す（S167。木の項目と同じ形の JSON、最大 `limit` 件、近い順）。
    pub fn library_search(&self, query: &str, limit: usize) -> String {
        self.tree_search(query, limit)
    }

    /// ライブラリのクラス `path` を定義するファイルの `{file, text, range}` の JSON（S127。S186 から、`range` はそのクラスの宣言の範囲で LSP の Range。引けなければ無い）。無ければ例外（文字列）。
    pub fn library_source(&self, path: &str) -> Result<String, JsValue> {
        let (file, text) = self
            .tree_source(path)
            .ok_or_else(|| js_error(format!("ライブラリに {path} が無い")))?;
        to_json(
            &self
                .with_resolver(|r| globe_mode_lsp::definition::library_source(r, path, file, text)),
        )
    }

    /// 文書 `file`（本文 `text`。先に `set_file` で足しておく）の行・列（LSP と同じ UTF-16）にある名前の定義を
    /// `{file, text, range}` の JSON で返す（S138。LSP の `globe/definition` と同じ形。`text` は定義のあるファイルの本文）。無ければ `null`。
    pub fn definition(&self, file: &str, text: &str, line: u32, character: u32) -> String {
        self.resolve(
            globe_mode_lsp::definition::definition_at,
            file,
            text,
            line,
            character,
        )
    }

    /// `definition` と同じ解決で、名前の宣言の 1 行を `{contents, range}` の JSON で返す（S149。LSP の `globe/hover` と同じ形）。無ければ `null`。
    pub fn hover(&self, file: &str, text: &str, line: u32, character: u32) -> String {
        self.resolve(
            globe_mode_lsp::definition::hover_at,
            file,
            text,
            line,
            character,
        )
    }

    /// 文書 `file`（本文 `text`。先に `set_file` で足しておく）の行・列（UTF-16）の補完候補を LSP の `CompletionItem` の**配列**の JSON で返す（S171。LSP の `globe/completion` は S198 から `{isIncomplete, items}` で、`items` が同じ形。ここは配列のまま）。
    pub fn completion(&self, file: &str, text: &str, line: u32, character: u32) -> String {
        self.with_resolver(|r| {
            globe_mode_lsp::completion::complete_at(r, file, text, line, character)
        })
        .to_string()
    }

    /// 文書 `file`（先に `set_file` で足しておく）の行・列にある名前の参照を LSP の Range の配列の JSON で返す（S161。LSP の `globe/references` と同じ形）。
    pub fn references(&self, file: &str, text: &str, line: u32, character: u32) -> String {
        self.with_resolver(|r| {
            globe_mode_lsp::definition::references_at(r, file, text, line, character)
        })
        .to_string()
    }

    /// 文書 `file`（先に `set_file` で足しておく）の実行できるクラスの修飾名を文字列の配列の JSON で返す（S170。LSP の `globe/models` と同じ形）。
    pub fn models(&self, file: &str) -> String {
        let o = self.inner.outline(file).and_then(|r| r.as_ref().ok());
        globe_mode_lsp::definition::models(o).to_string()
    }

    /// 文書 `file`（先に `set_file` で足しておく。本文 `text`）の折りたたみの範囲を `{startLine, endLine, kind}` の配列の JSON で返す（S225。LSP の `textDocument/foldingRange` と同じ関数。構文が壊れていればクラスと節は出ない）。
    pub fn folding_ranges(&self, file: &str, text: &str) -> String {
        let o = self.inner.outline(file).and_then(|r| r.as_ref().ok());
        let classes = o.map(|o| o.classes.as_slice());
        serde_json::Value::Array(globe_mode_lsp::folding::folding_ranges(text, classes)).to_string()
    }

    /// 名前を `new_name` に変える TextEdit（`{range, newText}`）の配列の JSON（S161。LSP の `globe/rename` と同じ形）。
    /// 断る場合（部品でもクラスでもない・別のファイルのクラス・識別子でない・予約語・同じ階層に既にある）は理由を例外（文字列）で返す。
    pub fn rename(
        &self,
        file: &str,
        text: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<String, JsValue> {
        self.with_resolver(|r| {
            globe_mode_lsp::definition::rename_at(r, file, text, line, character, new_name)
        })
        .map(|v| v.to_string())
        .map_err(js_error)
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

    /// [`Workspace::simulate`] と同じだが、時系列を CSV でなく列ごとに返す（S051。[`SimColumns`]）。
    ///
    /// `on_progress` を渡すと、進捗のたびに `on_progress(stage, time, start_time, stop_time)` を同期で呼ぶ
    /// （ADR-0033、S133。`stage` は `"lowering"`・`"integration"` などの小文字、`time` は積分の t で、無ければ
    /// `undefined`）。関数が投げた例外は無視する。**関数が `true`（厳密に）を返すと、次の 1 歩の前に止めて、
    /// そこまでの列と `GM-I11004` を返す**（S157。1 本のスレッドの Worker は外から立てられないので、
    /// 呼び手が SharedArrayBuffer のフラグを読んで返す）。`push` の戻り値のような真の値では止まらない。
    pub fn simulate_columns(
        &mut self,
        request_json: &str,
        on_progress: Option<ProgressFn>,
    ) -> Result<SimColumns, JsValue> {
        let request = parse_request(request_json)?;
        let Some(f) = on_progress else {
            return Ok(self.run_columns(request));
        };
        let token = globe_mode::CancelToken::default();
        let mut notify = |p: &globe_mode::Progress| {
            let stage = p.stage.as_str();
            let stop = f.call4(&JsValue::NULL, stage, p.time, p.start_time, p.stop_time);
            if stop.is_ok_and(|v| v.as_bool() == Some(true)) {
                token.cancel();
            }
        };
        let handle = globe_mode::SimulateHandle {
            on_progress: Some(&mut notify),
            cancel: Some(token.clone()),
        };
        Ok(self.run_columns_observed(request, handle))
    }

    /// ソースの FMU（FMI `fmi_version`、2 か 3。build 無し）を zip のバイト列（`Uint8Array`）で返す（S144。UI は Blob にしてダウンロードする）。
    /// DAE まで変換できないなど、作れなければ例外（文字列）。
    pub fn export_fmu(&mut self, model: &str, fmi_version: u8) -> Result<Vec<u8>, JsValue> {
        self.inner
            .export_fmu_bytes(model, fmi_version)
            .map_err(js_error)
    }

    /// 上書きできるパラメータの一覧（S150）。`ParametersResponse` の JSON。DAE まで変換できなければ例外（診断の文）。
    pub fn tunable_parameters(&mut self, model: &str) -> Result<String, JsValue> {
        let r = self.inner.tunable_parameters(model).map_err(|ds| {
            js_error(
                ds.iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        to_json(&r)
    }

    /// モデルの検査だけ（S0〜S10。実行しない）。`CheckResponse` の JSON を返す（S041。ブラウザ UI の診断パネル用）。
    pub fn check(&mut self, model: &str) -> Result<String, JsValue> {
        to_json(&self.inner.check(model))
    }

    /// connect グラフ（graphology の JSON。`graph.import(json)` で読める形。S046）。ライブラリとして読んだモデルも引ける（S079）。モデルが無ければ例外（文字列）。
    pub fn graph(&mut self, model: &str, depth: u32) -> Result<String, JsValue> {
        let g = self
            .inner
            .graph(model, depth)
            .map_err(|e| js_error(e.to_string()))?;
        to_json(&g.to_graphology())
    }

    /// ファイル（`set_file` に渡した名前）のアウトライン（`globe outline --json` と同じ `Outline` の JSON。S191）。
    /// ファイルが無いか、構文の誤りがあれば例外（文字列。誤りは最初の診断の `コード: 文`）。
    pub fn outline(&self, path: &str) -> Result<String, JsValue> {
        self.inner.outline_json(path).map_err(js_error)
    }

    /// `check` に渡すファイルの閉包（パスの昇順）の JSON の配列。モデルが無ければ `[]`（S191）。
    pub fn deps(&mut self, model: &str) -> Result<String, JsValue> {
        to_json(&self.inner.deps(model))
    }

    /// flat モデルの要約（`FlatSummary` の JSON）。flatten まで通らなければ `null`（S191）。
    pub fn flat(&mut self, model: &str) -> Result<String, JsValue> {
        to_json(&self.inner.flat(model))
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

fn clear_last_panic() {
    if let Ok(mut g) = LAST_PANIC.lock() {
        *g = None;
    }
}

/// panic の hook を試すために、わざと panic する（試験用）。
#[doc(hidden)]
#[wasm_bindgen]
pub fn panic_for_test(message: &str) {
    panic!("{message}");
}

#[wasm_bindgen]
extern "C" {
    /// JS の関数（`simulate_columns` の進捗の受け手。js-sys を足さないための最小の宣言）
    #[wasm_bindgen(js_name = Function)]
    pub type ProgressFn;
    #[wasm_bindgen(method, catch, js_name = call)]
    fn call4(
        this: &ProgressFn,
        context: &JsValue,
        stage: &str,
        time: Option<f64>,
        start_time: f64,
        stop_time: f64,
    ) -> Result<JsValue, JsValue>;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// #12: 読んだあとも残る panic の文言が、次の実行・作り直しで別の失敗の原因と誤られない
    #[test]
    fn a_new_call_clears_the_last_panic() {
        let stale = || *LAST_PANIC.lock().unwrap() = Some("古い panic".to_string());
        stale();
        assert_eq!(last_panic().as_deref(), Some("古い panic"));
        Workspace::js_new(None, None, None).unwrap();
        assert_eq!(last_panic(), None, "js_new");
        stale();
        simulate(r#"{"model":"X"}"#).unwrap();
        assert_eq!(last_panic(), None, "simulate");
        let mut ws = Workspace::new();
        stale();
        ws.simulate(r#"{"model":"X"}"#).unwrap();
        assert_eq!(last_panic(), None, "Workspace::simulate");
        stale();
        ws.simulate_columns(r#"{"model":"X"}"#, None).unwrap();
        assert_eq!(last_panic(), None, "simulate_columns");
    }
}

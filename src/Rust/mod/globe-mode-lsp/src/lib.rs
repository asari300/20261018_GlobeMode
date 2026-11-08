//! GlobeMode の LSP サーバ（S038）。stdio の JSON-RPC を自前で回し、型だけ `lsp-types` を使う。
//!
//! 機能: 診断（`publishDiagnostics`、検査 S0〜S10 と lint）、`documentSymbol`（アウトライン）、
//! `hover`（クラスの種類）、`formatting`（S053、`globe-mode-fmt`）、`completion`（S053、[`completion`]）。
//! `definition`・`references`（S132、[`definition`]）、`foldingRange`（S220、[`folding`]）。同期は Full で、要求は 1 本のスレッドで順に処理する
//! （S136: 入力の読みは別のスレッド。`$/cancelRequest` で `globe/simulate`・`globe/simulateColumns` を止め、
//! params に `workDoneToken` があれば `$/progress` で進捗を出す。ADR-0033）。
//!
//! 独自の要求（S049、ADR-0023。Desktop の UI が sidecar の `globe lsp` に同じ接続で頼む）:
//!
//! | 要求 | params | result |
//! | --- | --- | --- |
//! | `globe/check` | `{model}` | `CheckResponse` の JSON |
//! | `globe/simulate` | `{request}`（`SimulateRequest` の JSON。S214: `max_wall_seconds` が無ければ 180 s の予算を置く） | `SimulateResponse` の JSON |
//! | `globe/simulateColumns` | `{request}` | `{response, layout, columns}`（S076。wasm の `simulate_columns` と同じ形。`response` に CSV は無く、`layout` は `ColumnsLayout`（`names`・`points`・既定で描く `defaults`。S190）、`columns[i]` は列 `i` の数の配列で、有限でない値は null） |
//! | `globe/graph` | `{model, depth}` | graphology の JSON（`ConnectGraph::to_graphology`） |
//! | `globe/format` | `{text, indent?, insertOrigin?, blankAroundSections?, formatHtml?}` | 整形後の本文（S053。壊れていれば -32000。S214: `globe-mode.toml` があれば indent は設定が勝つ。無いときだけ params の indent） |
//! | `globe/lint` | `{text}` | `[{rule, line, column, message}]`（S053） |
//! | `globe/libraryTree` | `{prefix}`（空ならトップレベル） | `[{name, path, kind, description, file, hasChildren}]`（S127。直下の子だけ。wasm の `library_tree` と同じ形） |
//! | `globe/definition` | `{file, line, character}`（開いている文書。行は 0 始まり、列は UTF-16） | `{file, text, range}` か null（S138。`textDocument/definition` と同じ解決で、定義のあるファイルの本文も返す。wasm の `definition` と同じ形） |
//! | `globe/hover` | `{file, line, character}`（`globe/definition` と同じ） | `{contents, range}` か null（S149。名前の宣言の 1 行（クラスは見出し、部品は `;` まで。説明文字列を含む）と、カーソルの名前の Range。wasm の `hover` と同じ形） |
//! | `globe/references` | `{file, line, character}`（`globe/definition` と同じ） | LSP の Range の配列（S161。カーソルの名前と字句が同じ位置。部品はカーソルのあるいちばん内側のクラスの中だけ。S166: クラス名の上なら、同じ文書の中で定義ジャンプと同じ解決で同じクラスに行き着く識別子（宣言・`end`・型・`extends`・`import`）。wasm の `references` と同じ形） |
//! | `globe/models` | `{file}` | 文字列の配列（S170。文書の model・block・class の修飾名。partial は除く。package の中まで入り、`within` は頭に付く。構文が壊れていれば空。wasm の `models` と同じ形） |
//! | `globe/rename` | `{file, line, character, newName}` | LSP の TextEdit（`{range, newText}`）の配列（S161。いちばん内側のクラスで宣言した部品と、S166 からこの文書で定義したクラス。部品でもクラスでもない・別のファイルのクラス・識別子でない・予約語・同じ階層に既にある名前・ほかの開いた文書に参照があるときは -32803 に理由。`textDocument/rename` は同じ計算の WorkspaceEdit（S198: クラスのリネームは、最上位のクラスがファイル名 `M.mo`・`package.mo` に対応しているとき、ワークスペースの根の下の開いていない `.mo` に同じ名前の識別子があるときも断る。`prepareRename` も同じ。開いていない文書への `globe/rename`・`textDocument/rename` は -32803、`globe/references` は空の配列）、`textDocument/prepareRename` はカーソルの識別子の Range か同じ理由） |
//! | `globe/solvers` | なし | ソルバの登録表（`[{name, status, description, instead?}]`。S142。wasm の `solvers` と同じ） |
//! | `globe/readResults` | `{path}`（`.mat` は MAT v4、それ以外は CSV） | `{layout, columns}`（S222。`globe/simulateColumns` の `layout`・`columns` と同じ形。`defaults` は先頭の 3 本。読めなければ -32000） |
//! | `globe/fmuInfo` | `{path}`（`.fmu`） | `FmuInfo` の JSON に `warning`（実行の前に見せる警告。S229。`globe fmu info` と同じ情報。ネイティブコードは実行しない。読めなければ -32000。feature `fmu-import`） |
//! | `globe/fmuRun` | `{path, request?, workDoneToken?}`（`request` は `FmuRunRequest`: `kind`（`"cs"`/`"me"`）・`start_time`・`stop_time`・`step`・`outputs`・`set`・`timeout`・`memory_limit_mib`） | `{layout, columns, me}`（S229。`globe/readResults` と同じ `layout`・`columns` に、ME だけ事象の要約 `me`。**FMU のネイティブコードを別プロセス（自分と同じ実行ファイルの `fmu __worker`）で実行する**。`$/progress` は `{kind: "report", stage: "integration", time}`、`$/cancelRequest` でワーカーを kill して -32800。失敗は -32000。feature `fmu-import`） |
//! | `globe/exportFmu` | `{model, fmiVersion?, outDir}`（既定は FMI 2。`build` はしない） | `{path}`（S144。`outDir/<名前>.fmu` と展開した `outDir/<名前>/` を書く。DAE まで変換できなければ -32000） |
//! | `globe/tunableParameters` | `{model}` | `ParametersResponse` の JSON（S150。上書きできるパラメータの一覧。wasm の `tunable_parameters` と同じ。DAE まで変換できなければ -32000） |
//! | `globe/completion` | `{file, line, character}`（`globe/definition` と同じ） | LSP の `CompletionList` `{isIncomplete, items}`（S171。`textDocument/completion` と同じ計算。予約語、文書の名前、点つきの名前の途中はクラスの子か部品の型の部品名。最大 200 件で、200 件に達したら `isIncomplete: true`（S198。打ちかけの語で絞ってから切る）。文字列・コメントの中は空。wasm の `completion` は `items` と同じ配列だけを返す） |
//! | `globe/librarySearch` | `{query}`（名前の部分一致。大文字小文字を区別しない。`.` を含めると完全修飾名） | 同じ形の項目の配列（S167。最大 50 件、近い順。ファイルの中の入れ子のクラスも出る） |
//! | `globe/librarySource` | `{path}`（完全修飾名） | `{file, text, range}`（S127。そのクラスを定義するライブラリのファイル。S186 から `range` はそのクラスの宣言の範囲（LSP の Range。引けなければ無い）。無ければ -32000） |
//!
//! 設定ファイル（S130）: `initialize` の `rootUri`（無ければ `rootPath`）から上へたどって最初の `globe-mode.toml` を
//! 読み、lint の診断と `textDocument/formatting` に使う（`workspace/didChangeConfiguration` で読み直す）。
//! `globe/format`・`globe/lint` は wasm と同じく params だけで決める。
//!
//! params が読めなければ -32602、モデルが無いなど計算の失敗は -32000（`message` に理由）。
//! facade（`globe-mode`）の `Workspace` だけを呼び、新しい計算は入れない。

pub mod completion;
pub mod definition;
pub mod folding;

use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;

use globe_mode::db::normalize_path;
use globe_mode::{
    CancelToken, Diagnostic, Progress, SimulateHandle, SimulateRequest, Simulated, Workspace,
};
use globe_mode_core::Severity;
use globe_mode_ir::{ClassKind, ClassOutline};
use lsp_types::{
    DiagnosticSeverity, DocumentSymbol, Hover, HoverContents, MarkupContent, MarkupKind,
    NumberOrString, Position, Range, SymbolKind, Uri,
};
use serde_json::{Value, json};

/// `globe/simulate` の既定の時間の予算（秒。S214）。`msl-sim` の予算と同じ。要求の `max_wall_seconds` が優先する。
const DEFAULT_MAX_WALL_SECONDS: f64 = 180.0;

/// 1 つのクライアントとの会話の状態。
#[derive(Debug)]
pub struct Server {
    ws: Workspace,
    /// 開いているファイルの本文（キーは URI の文字列）。位置の換算に使う
    texts: HashMap<String, String>,
    libraries: Vec<PathBuf>,
    /// ライブラリの木（S127。最初の `globe/libraryTree` で全ファイルの見出しを読んで作る）
    tree: Option<globe_mode_fmt::LibraryTree>,
    /// ワークスペースの根（`initialize` の `rootUri`）
    root: Option<PathBuf>,
    /// 根の `globe-mode.toml`（S130）。無ければ None
    config: Option<globe_mode_fmt::LintOptions>,
    /// 定義ジャンプで読んだライブラリのファイルのアウトラインだけを持つ（S132。検査には使わない）
    scratch: Workspace,
    /// `scratch` に読んだライブラリのファイルの更新時刻（S198。ディスクで変わったら次の要求で読み直す）
    scratch_mtime: HashMap<String, Option<std::time::SystemTime>>,
    /// 最後に出した診断（URI → 診断）。ほかの文書の編集で診断が変わったものだけを出し直すための比べる相手（#64）
    published: HashMap<String, Vec<lsp_types::Diagnostic>>,
}

impl Server {
    /// `libraries` を一度だけ読む（読めなければ S1 の診断）。
    pub fn new(libraries: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        Ok(Server {
            ws: Workspace::with_libraries(libraries)?,
            texts: HashMap::new(),
            libraries: libraries.to_vec(),
            tree: None,
            root: None,
            config: None,
            scratch: Workspace::new(),
            scratch_mtime: HashMap::new(),
            published: HashMap::new(),
        })
    }

    /// 整形の indent。設定ファイルがあればその値（`base`）、無いときだけ要求の値（S214）。
    fn indent(&self, requested: Option<u64>, base: usize) -> usize {
        match (&self.config, requested) {
            (None, Some(n)) => n as usize,
            _ => base,
        }
    }

    fn tree(&mut self) -> &globe_mode_fmt::LibraryTree {
        self.tree.get_or_insert_with(|| {
            let mut files = Vec::new();
            for lib in &self.libraries {
                collect_mo(lib, &mut files);
            }
            // 名前は Workspace::with_libraries がライブラリのファイルに付けるのと同じ（display）。開くと同じ文書を差し替える
            let texts: Vec<(String, String)> = files
                .iter()
                .filter_map(|p| Some((p.display().to_string(), std::fs::read_to_string(p).ok()?)))
                .collect();
            globe_mode_fmt::LibraryTree::new(texts.iter().map(|(f, t)| (f.as_str(), t.as_str())))
        })
    }

    /// `uri` 以外の開いている文書（S166。クラス名の参照を、ほかの文書にも探す）。
    fn others(&self, uri: &str) -> Vec<(String, String)> {
        self.texts
            .iter()
            .filter(|(u, _)| u.as_str() != uri)
            .map(|(u, t)| (u.clone(), t.clone()))
            .collect()
    }

    /// ライブラリの木と開いている文書で名前を引く道具を作って `f` を回す（S166。参照・リネームが使う）。
    fn with_resolver<T>(&mut self, f: impl FnOnce(&mut definition::Resolver) -> T) -> T {
        let mtime = |p: &str| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        // ディスクで更新時刻が変わったライブラリのファイルは scratch から除く（読み直される）
        let stale: Vec<String> = self
            .scratch
            .files()
            .filter(|p| self.scratch_mtime.get(*p).is_some_and(|t| *t != mtime(p)))
            .map(str::to_string)
            .collect();
        for p in &stale {
            self.scratch.remove_file(p);
            self.scratch_mtime.remove(p);
        }
        self.tree();
        let tree = self.tree.as_ref().expect("tree() で作った");
        let open = &self.texts;
        let out = f(&mut definition::Resolver {
            ws: &self.ws,
            tree,
            scratch: &mut self.scratch,
            overlay: None,
            read: &|p| {
                open.iter()
                    .find(|(u, _)| uri_path(u) == p)
                    .map(|(_, t)| t.clone())
                    .or_else(|| std::fs::read_to_string(p).ok())
            },
        });
        for p in self.scratch.files() {
            self.scratch_mtime
                .entry(p.to_string())
                .or_insert_with(|| mtime(p));
        }
        out
    }

    /// 根から上へたどって `globe-mode.toml` を読み直す（S130）。誤りは `window/showMessage` で知らせ、既定で続ける。
    fn load_config(&mut self) -> Vec<Value> {
        let found = self
            .root
            .as_deref()
            .map(globe_mode_fmt::LintOptions::find)
            .transpose()
            .map(Option::flatten);
        match found {
            Ok(c) => {
                self.config = c;
                vec![]
            }
            Err(e) => {
                self.config = None;
                vec![json!({"jsonrpc": "2.0", "method": "window/showMessage",
                    "params": {"type": 1, "message": format!("globe-mode.toml: {e}")}})]
            }
        }
    }

    /// 受けたメッセージ 1 つに対して、送り返すメッセージ（応答と通知）を返す。
    pub fn handle(&mut self, msg: Value) -> Vec<Value> {
        let mut notes = Vec::new();
        let replies = self.handle_with(msg, None, &mut |m| notes.push(m));
        notes.extend(replies);
        notes
    }

    /// [`Server::handle`] と同じだが、実行の要求は `cancel` で止められ、進捗（`$/progress`）は返り値を待たずに
    /// `notify` へ渡す（S136、ADR-0033）。
    pub fn handle_with(
        &mut self,
        msg: Value,
        cancel: Option<CancelToken>,
        notify: &mut dyn FnMut(Value),
    ) -> Vec<Value> {
        let id = msg.get("id").cloned();
        let params = &msg["params"];
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match msg["method"].as_str().unwrap_or("") {
            "initialize" => {
                self.root = params["rootUri"]
                    .as_str()
                    .map(uri_path)
                    .or_else(|| params["rootPath"].as_str().map(str::to_string))
                    .map(PathBuf::from);
                let mut out = reply(
                    id,
                    json!({
                        // 1 = Full
                        "capabilities": {"textDocumentSync": 1, "documentSymbolProvider": true, "hoverProvider": true,
                            "documentFormattingProvider": true, "completionProvider": {"triggerCharacters": ["."]},
                            "definitionProvider": true, "referencesProvider": true, "foldingRangeProvider": true,
                            "renameProvider": {"prepareProvider": true}},
                        "serverInfo": {"name": "globe-mode", "version": env!("CARGO_PKG_VERSION")},
                    }),
                );
                out.extend(self.load_config());
                out
            }
            // S130: 設定ファイルを読み直し、開いている文書の診断を出し直す
            "workspace/didChangeConfiguration" => {
                let mut out = self.load_config();
                let open: Vec<_> = self.texts.clone().into_iter().collect();
                for (uri, text) in open {
                    out.extend(self.update(uri, text));
                }
                out
            }
            "shutdown" => reply(id, Value::Null),
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or("");
                let mut out = self.update(uri.clone(), text.to_string());
                out.extend(self.republish_others(&uri));
                out
            }
            "textDocument/didChange" => {
                // Full 同期: 最後の変更が本文全体
                let text = params["contentChanges"]
                    .as_array()
                    .and_then(|a| a.last())
                    .and_then(|c| c["text"].as_str())
                    .unwrap_or("");
                let mut out = self.update(uri.clone(), text.to_string());
                out.extend(self.republish_others(&uri));
                out
            }
            "textDocument/didClose" => {
                self.texts.remove(&uri);
                self.published.remove(&uri);
                self.ws.remove_file(&uri_path(&uri));
                let mut out = vec![publish(&uri, &[])];
                out.extend(self.republish_others(&uri));
                out
            }
            "textDocument/documentSymbol" => {
                let symbols = match (self.ws.outline(&uri_path(&uri)), self.texts.get(&uri)) {
                    (Some(Ok(o)), Some(text)) => Some(
                        o.classes
                            .iter()
                            .map(|c| symbol(c, text))
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                };
                reply(id, json!(symbols))
            }
            // S220: 範囲は行だけ。構文が壊れていてアウトラインが引けなくても、コメントと annotation は出る
            "textDocument/foldingRange" => {
                let ranges = self.texts.get(&uri).map(|text| {
                    let outline = self.ws.outline(&uri_path(&uri));
                    let classes = outline.as_ref().and_then(|o| o.as_ref().ok()).map(|o| o.classes.as_slice());
                    folding::folding_ranges(text, classes)
                });
                reply(id, json!(ranges))
            }
            "textDocument/hover" => {
                let pos = &params["position"];
                let pos = Position::new(
                    pos["line"].as_u64().unwrap_or(0) as u32,
                    pos["character"].as_u64().unwrap_or(0) as u32,
                );
                let hover = match (self.ws.outline(&uri_path(&uri)), self.texts.get(&uri)) {
                    (Some(Ok(o)), Some(text)) => {
                        innermost(&o.classes, offset(text, pos) as u32).map(hover)
                    }
                    _ => None,
                };
                reply(id, json!(hover))
            }
            "textDocument/formatting" => {
                // 文書全体を 1 つの TextEdit で置き換える。整形できない（文法が壊れている）ときは null
                // FormattingOptions の追加のキー: insertOrigin・blankAroundSections（S061。VS Code の拡張が設定から渡す）
                // S130: globe-mode.toml があればそれを土台にし、indent は tabSize より設定ファイルを採る（ruff と同じ）
                let opts = &params["options"];
                let base = self.config.clone().unwrap_or_default().format;
                let indent = self.indent(opts["tabSize"].as_u64(), base.indent);
                let options = globe_mode_fmt::Options {
                    indent,
                    insert_origin: opts["insertOrigin"].as_bool().unwrap_or(base.insert_origin),
                    blank_around_sections: opts["blankAroundSections"]
                        .as_bool()
                        .unwrap_or(base.blank_around_sections),
                    format_html: opts["formatHtml"].as_bool().unwrap_or(base.format_html),
                    ..base
                };
                let edit = self.texts.get(&uri).and_then(|text| {
                    let out = globe_mode_fmt::format(text, &options).ok()?;
                    (out != *text).then(|| {
                        json!([{"range": range(text, globe_mode_core::TextRange { start: 0, end: text.len() as u32 }), "newText": out}])
                    })
                });
                reply(id, edit.unwrap_or(Value::Null))
            }
            // S171: UI 向けの `globe/completion` は `file`・`line`・`character`（`globe/definition` と同じ）。計算は同じ
            "textDocument/completion" | "globe/completion" => {
                let (key, pos) = if msg["method"] == "globe/completion" {
                    (params["file"].as_str().unwrap_or("").to_string(), lsp_position(params))
                } else {
                    (uri.clone(), lsp_position(&params["position"]))
                };
                let list = match self.texts.get(&key).cloned() {
                    Some(text) => {
                        let (path, at) = (uri_path(&key), offset(&text, pos));
                        self.with_resolver(|r| completion::complete_list(r, &path, &text, at))
                    }
                    None => json!({"isIncomplete": false, "items": []}),
                };
                reply(id, list)
            }
            "textDocument/definition" => {
                let Some(text) = self.texts.get(&uri).cloned() else {
                    return reply(id, Value::Null);
                };
                let at = offset(&text, lsp_position(&params["position"]));
                let path = uri_path(&uri);
                let target = self.with_resolver(|r| r.definition(&path, &text, at));
                let location = target.and_then(|t| {
                    // 開いている文書ならその本文と URI、無ければディスクから読む
                    let open = self.texts.iter().find(|(u, _)| uri_path(u) == t.path);
                    let (u, body) = match open {
                        Some((u, body)) => (u.clone(), body.clone()),
                        None => (file_uri(&t.path), std::fs::read_to_string(&t.path).ok()?),
                    };
                    Some(json!({"uri": u, "range": range(&body, t.range)}))
                });
                reply(id, location.unwrap_or(Value::Null))
            }
            // UI 向け（S138）: 開いている文書 `file` の行・列の定義を `{file, text, range}` で返す（無ければ null）
            "globe/definition" | "globe/hover" => {
                let Some(file) = params["file"].as_str() else {
                    return error(id, -32602, "params.file が文字列でない");
                };
                let Some(text) = self.texts.get(file).cloned() else {
                    return reply(id, Value::Null);
                };
                // globe/hover（S149）は同じ解決で、宣言の 1 行を `{contents, range}` で返す
                let hover = msg["method"] == "globe/hover";
                let (line, character) = (
                    params["line"].as_u64().unwrap_or(0) as u32,
                    params["character"].as_u64().unwrap_or(0) as u32,
                );
                let path = uri_path(file);
                let found = self.with_resolver(|r| {
                    let f = if hover { definition::hover_at } else { definition::definition_at };
                    f(r, &path, &text, line, character)
                });
                reply(id, found.unwrap_or(Value::Null))
            }
            // S166: クラス名の上なら、開いているほかの文書の同じクラスへの参照も含める（部品の参照は同じ文書の中だけ）
            "textDocument/references" => {
                let Some(text) = self.texts.get(&uri).cloned() else {
                    return reply(id, json!([]));
                };
                let at = offset(&text, lsp_position(&params["position"]));
                let others = self.others(&uri);
                let path = uri_path(&uri);
                let refs = self.with_resolver(|r| {
                    let mut refs: Vec<Value> = definition::references(r, &path, &text, at)
                        .into_iter()
                        .map(|x| json!({"uri": uri, "range": range(&text, x)}))
                        .collect();
                    if let Some((name, t)) = definition::class_name_at(r, &path, &text, at) {
                        for (u, body) in &others {
                            for x in definition::class_hits(r, &uri_path(u), body, &name, &t) {
                                refs.push(json!({"uri": u, "range": range(body, x)}));
                            }
                        }
                    }
                    refs
                });
                reply(id, json!(refs))
            }
            // S161: 参照の一覧とリネーム。UI 向けの `globe/*` は `file`・`line`・`character`（rename は `newName` も）、
            // `textDocument/rename` は VS Code 向け。計算は `definition::references_at` / `rename_at`（wasm と同じ）。
            // S166: クラス名も同じ口。リネームは 1 文書の中だけ、開いているほかの文書に同じクラスへの参照があれば断る。
            // `textDocument/prepareRename` は断る理由（`rename` と同じ）かカーソルの識別子の範囲
            "textDocument/rename" | "textDocument/prepareRename" | "globe/references" | "globe/rename" => {
                let method = msg["method"].as_str().unwrap_or("");
                let (key, pos) = if method.starts_with("textDocument/") {
                    (uri.clone(), lsp_position(&params["position"]))
                } else {
                    let (l, c) = (&params["line"], &params["character"]);
                    (
                        params["file"].as_str().unwrap_or("").to_string(),
                        Position::new(l.as_u64().unwrap_or(0) as u32, c.as_u64().unwrap_or(0) as u32),
                    )
                };
                // 開いていない文書: references は空の配列、リネームは断る（S198。`textDocument/references` と同じ形）
                let Some(text) = self.texts.get(&key).cloned() else {
                    return match method {
                        "globe/references" => reply(id, json!([])),
                        _ => error(id, -32803, "文書が開かれていない"),
                    };
                };
                let (path, others) = (uri_path(&key), self.others(&key));
                let new_name = params["newName"].as_str().unwrap_or("");
                // ディスクの上のほかのファイルの検査は VS Code など `textDocument/*` だけ（UI の `globe/*` はメモリ上の文書）
                let disk = method.starts_with("textDocument/").then(|| {
                    (self.root.clone(), self.texts.keys().map(|u| uri_path(u)).collect::<Vec<_>>())
                });
                let result: Result<Value, String> = self.with_resolver(|r| {
                    let blocked = |r: &mut definition::Resolver| {
                        let (root, open) = disk.as_ref()?;
                        class_rename_blocked(r, root.as_deref(), open, &path, &text, offset(&text, pos))
                    };
                    match method {
                        "globe/references" => Ok(definition::references_at(r, &path, &text, pos.line, pos.character)),
                        "textDocument/prepareRename" => {
                            let x = definition::prepare_rename(r, &path, &text, offset(&text, pos))?;
                            blocked(r).map_or_else(|| Ok(json!(range(&text, x))), Err)
                        }
                        _ => {
                            let edits = definition::rename_at(r, &path, &text, pos.line, pos.character, new_name)?;
                            if !others.is_empty()
                                && let Some((name, t)) = definition::class_name_at(r, &path, &text, offset(&text, pos))
                            {
                                let n: usize = others
                                    .iter()
                                    .map(|(u, body)| definition::class_hits(r, &uri_path(u), body, &name, &t).len())
                                    .sum();
                                if n > 0 {
                                    return Err(format!("ほかの文書に {n} 箇所の参照がある（リネームは 1 文書の中だけ）"));
                                }
                            }
                            if let Some(why) = blocked(r) {
                                return Err(why);
                            }
                            Ok(if method == "globe/rename" { edits } else { json!({"changes": {key.clone(): edits}}) })
                        }
                    }
                });
                match result {
                    Ok(v) => reply(id, v),
                    Err(why) => error(id, -32803, &why), // RequestFailed
                }
            }
            // S170: 文書の中の実行できるクラスの修飾名（UI のモデル名の候補。wasm の `models` と同じ形）
            "globe/models" => {
                let file = params["file"].as_str().unwrap_or("");
                let o = self.ws.outline(&uri_path(file)).and_then(|r| r.as_ref().ok());
                reply(id, definition::models(o))
            }
            "globe/check" => match params["model"].as_str() {
                Some(model) => reply(id, json!(self.ws.check(model))),
                None => error(id, -32602, "params.model が文字列でない"),
            },
            "globe/tunableParameters" => match params["model"].as_str() {
                Some(model) => match self.ws.tunable_parameters(model) {
                    Ok(r) => reply(id, json!(r)),
                    Err(ds) => {
                        let m: Vec<&str> = ds.iter().map(|d| d.message.as_str()).collect();
                        error(id, -32000, &m.join("; "))
                    }
                },
                None => error(id, -32602, "params.model が文字列でない"),
            },
            "globe/simulate" => {
                match serde_json::from_value::<SimulateRequest>(params["request"].clone()) {
                    Ok(mut request) => {
                        let s = self.simulate(&mut request, params.get("workDoneToken"), cancel, notify);
                        reply(id, json!(s.response))
                    }
                    Err(e) => error(id, -32602, &format!("params.request: {e}")),
                }
            }
            "globe/simulateColumns" => {
                match serde_json::from_value::<SimulateRequest>(params["request"].clone()) {
                    Ok(mut request) => {
                        request.no_csv = true;
                        let s = self.simulate(&mut request, params.get("workDoneToken"), cancel, notify);
                        let (layout, columns) = s.columns();
                        // serde_json は NaN / Inf を null にする（UI の欠損と同じ）
                        reply(id, json!({"response": s.response, "layout": layout, "columns": columns}))
                    }
                    Err(e) => error(id, -32602, &format!("params.request: {e}")),
                }
            }
            // S222: 結果のファイル（.mat か .csv）を開く。UI の dialog で選んだパスをここで読む（Desktop には wasm が無い）
            "globe/readResults" => match params["path"].as_str() {
                Some(path) => match std::fs::read(path)
                    .map_err(|e| format!("{path}: {e}"))
                    .and_then(|b| globe_mode::open_results(path, &b))
                {
                    Ok(r) => {
                        let (layout, columns) = r.columns();
                        reply(id, json!({"layout": layout, "columns": columns}))
                    }
                    Err(e) => error(id, -32000, &e),
                },
                None => error(id, -32602, "params.path が文字列でない"),
            },
            // S229: FMI Import（feature fmu-import。wasm には付けない）
            #[cfg(feature = "fmu-import")]
            "globe/fmuInfo" => match params["path"].as_str() {
                Some(path) => match globe_mode::import_fmu_info(std::path::Path::new(path)) {
                    // warning は UI が実行の前の確認に出す
                    Ok(info) => {
                        let mut v = json!(info);
                        v["warning"] = json!(globe_mode::FMU_TRUST_WARNING);
                        reply(id, v)
                    }
                    Err(e) => error(id, -32000, &e),
                },
                None => error(id, -32602, "params.path が文字列でない"),
            },
            #[cfg(feature = "fmu-import")]
            "globe/fmuRun" => fmu_run(id, params, cancel, notify),
            "globe/graph" => match params["model"].as_str() {
                Some(model) => {
                    let depth = params["depth"].as_u64().unwrap_or(1) as u32;
                    match self.ws.graph(model, depth) {
                        Ok(g) => reply(id, g.to_graphology()),
                        Err(e) => error(id, -32000, &e.to_string()),
                    }
                }
                None => error(id, -32602, "params.model が文字列でない"),
            },
            // 文書に依らない整形と lint（S053。Desktop の UI が wasm と同じ口で頼む）
            // S143: 設定ファイル（globe-mode.toml）があればそれを土台にする。S214: indent は textDocument/formatting と同じく
            // 設定ファイルが勝つ（VS Code は常に tabSize を送るので、勝たせると保存時の整形が CI の `fmt --check` と食い違う）。
            // 設定ファイルが無いとき（ブラウザ版の UI）だけ params の indent を採る
            "globe/format" => match params["text"].as_str() {
                Some(text) => {
                    let base = self.config.clone().unwrap_or_default().format;
                    let options = globe_mode_fmt::Options {
                        indent: self.indent(params["indent"].as_u64(), base.indent),
                        insert_origin: params["insertOrigin"].as_bool().unwrap_or(base.insert_origin),
                        blank_around_sections: params["blankAroundSections"]
                            .as_bool()
                            .unwrap_or(base.blank_around_sections),
                        format_html: params["formatHtml"].as_bool().unwrap_or(base.format_html),
                        ..base
                    };
                    match globe_mode_fmt::format(text, &options) {
                        Ok(out) => reply(id, json!(out)),
                        Err(e) => error(id, -32000, &e.message),
                    }
                }
                None => error(id, -32602, "params.text が文字列でない"),
            },
            // ソルバの登録表（S142。wasm の `solvers` と同じ JSON の配列）
            "globe/solvers" => reply(id, json!(globe_mode::solvers())),
            // ソースの FMU の書き出し（S144。build 無し。`outDir/<名前>-fmi<版>.fmu` と展開した `outDir/<名前>/` を書く）
            "globe/exportFmu" => match (params["model"].as_str(), params["outDir"].as_str()) {
                (Some(model), Some(out_dir)) => {
                    let options = globe_mode::FmuOptions {
                        fmi_version: params["fmiVersion"].as_u64().unwrap_or(2) as u8,
                        ..Default::default()
                    };
                    match self.ws.export_fmu(model, std::path::Path::new(out_dir), &options) {
                        Ok(path) => reply(id, json!({"path": path.to_string_lossy()})),
                        Err(e) => error(id, -32000, &e),
                    }
                }
                _ => error(id, -32602, "params.model と params.outDir が文字列でない"),
            },
            "globe/lint" => match params["text"].as_str() {
                Some(text) => reply(
                    id,
                    json!(globe_mode_fmt::lint(text, &self.config.clone().unwrap_or_default())
                        .iter()
                        .map(|m| json!({"rule": m.rule, "line": m.line, "column": m.column, "message": m.message}))
                        .collect::<Vec<_>>()),
                ),
                None => error(id, -32602, "params.text が文字列でない"),
            },
            "globe/libraryTree" => {
                let prefix = params["prefix"].as_str().unwrap_or("");
                reply(id, json!(tree_entries_json(&self.tree().children(prefix))))
            }
            "globe/librarySearch" => match params["query"].as_str() {
                Some(q) => reply(id, json!(tree_entries_json(&self.tree().search(q, 50)))),
                None => error(id, -32602, "params.query が文字列でない"),
            },
            "globe/librarySource" => match params["path"].as_str() {
                Some(path) => match self.tree().file(path).map(|f| (f.to_string(), std::fs::read_to_string(f))) {
                    Some((file, Ok(text))) => {
                        let v = self.with_resolver(|r| definition::library_source(r, path, &file, &text));
                        reply(id, v)
                    }
                    Some((file, Err(e))) => error(id, -32000, &format!("{file}: {e}")),
                    None => error(id, -32000, &format!("ライブラリに {path} が無い")),
                },
                None => error(id, -32602, "params.path が文字列でない"),
            },
            // 要求（id あり）は断る。知らない通知は無視する
            _ => match id {
                Some(id) => vec![
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}}),
                ],
                None => vec![],
            },
        }
    }

    /// 本文を差し替えて、診断を出し直す。
    /// 実行（ADR-0033）。`token`（params の `workDoneToken`）があれば `$/progress` の begin・report・end を出す。
    /// report の value には LSP の `message`（段）と `percentage`（t のあるときだけ）に、facade の `Progress` の
    /// `stage`・`time`・`start_time`・`stop_time` を添える（Desktop の UI がそのまま使う）。
    fn simulate(
        &mut self,
        request: &mut SimulateRequest,
        token: Option<&Value>,
        cancel: Option<CancelToken>,
        notify: &mut dyn FnMut(Value),
    ) -> Simulated {
        // 要求に無ければ既定の予算を置く（S214。既定のネイティブの道は段の間でしか中断を見ないので、硬いモデルが他の要求を塞ぐ）
        with_default_budget(request);
        let mut progress = |value: Value| {
            if let Some(token) = token {
                notify(json!({"jsonrpc": "2.0", "method": "$/progress",
                    "params": {"token": token, "value": value}}));
            }
        };
        progress(
            json!({"kind": "begin", "title": format!("simulate {}", request.model), "cancellable": true}),
        );
        let mut on_progress = |p: &Progress| {
            let stage = p.stage.as_str();
            let mut v = json!({"kind": "report", "message": stage, "stage": stage,
                "start_time": p.start_time, "stop_time": p.stop_time});
            if let Some(t) = p.time {
                let pct = (t - p.start_time) / (p.stop_time - p.start_time) * 100.0;
                v["time"] = json!(t);
                // 幅が 0 なら NaN で、`as` は 0 にする
                v["percentage"] = json!(pct.clamp(0.0, 100.0) as u32);
            }
            progress(v);
        };
        let handle = SimulateHandle {
            on_progress: Some(&mut on_progress),
            cancel,
        };
        let s = self
            .ws
            .simulate_observed(request, &globe_mode::now_ms, handle);
        progress(json!({"kind": "end"}));
        s
    }

    fn update(&mut self, uri: String, text: String) -> Vec<Value> {
        let path = uri_path(&uri);
        self.ws.set_file(&path, text.clone());
        // 検査が panic しても本文は最新にしておく（S214。古い本文から整形やリネームの編集を作らない）
        self.texts.insert(uri.clone(), text.clone());
        let diags = self.diagnostics(&path, &text);
        let out = vec![publish(&uri, &diags)];
        self.published.insert(uri, diags);
        out
    }

    /// 文書 `except` の編集・閉じ・開きのあと、ほかの開いている文書の診断を取り直し、変わったものだけ出し直す（#64。
    /// 依存先の変更で、波線が出ない・消えないのを避ける）。
    /// shortcut: 開いている全文書を検査し直す（salsa が変わっていない分は使い回す）。多すぎて重いなら `ws.deps` で絞る。
    fn republish_others(&mut self, except: &str) -> Vec<Value> {
        let others: Vec<(String, String)> = self
            .texts
            .iter()
            .filter(|(u, _)| u.as_str() != except)
            .map(|(u, t)| (u.clone(), t.clone()))
            .collect();
        let mut out = Vec::new();
        for (uri, text) in others {
            let diags = self.diagnostics(&uri_path(&uri), &text);
            if self.published.get(&uri) != Some(&diags) {
                out.push(publish(&uri, &diags));
                self.published.insert(uri, diags);
            }
        }
        out
    }

    /// 構文の誤り（S0）はそのまま、通れば最上位の model / block / class を 1 つずつ検査する。
    /// shortcut: パッケージの中のモデルは検査しない。必要になったら再帰する。
    fn diagnostics(&mut self, path: &str, text: &str) -> Vec<lsp_types::Diagnostic> {
        let models: Vec<String> = match self.ws.outline(path) {
            Some(Err(diags)) => return diags.iter().map(|d| lsp_diagnostic(d, text)).collect(),
            Some(Ok(o)) => o
                .classes
                .iter()
                .filter(|c| {
                    matches!(
                        c.kind,
                        ClassKind::Model | ClassKind::Block | ClassKind::Class
                    )
                })
                .map(|c| match &o.within {
                    Some(w) => format!("{w}.{}", c.name),
                    None => c.name.clone(),
                })
                .collect(),
            None => return vec![],
        };
        let key = normalize_path(path);
        let lint = self.config.clone().unwrap_or_default();
        let mut out: Vec<lsp_types::Diagnostic> = lint_diagnostics(text, &lint);
        // S062: 型の食い違う connect は、ワークスペースの型で解決できたものだけ warning にする
        let mismatches = if lint.disabled.iter().any(|d| d == "connect-type-mismatch") {
            vec![]
        } else {
            self.ws.connect_mismatches(path)
        };
        for m in mismatches {
            out.push(lsp_types::Diagnostic {
                range: range(text, m.connect.range),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("GM-L0005".to_string())),
                source: Some("globe-mode".to_string()),
                message: format!(
                    "{} は {}、{} は {} で、型が合わない (connect-type-mismatch)",
                    m.connect.lhs, m.lhs_type, m.connect.rhs, m.rhs_type
                ),
                ..Default::default()
            });
        }
        for model in models {
            for d in self.ws.check(&model).diagnostics {
                if d.file.as_deref().is_none_or(|f| f == key) {
                    let d = lsp_diagnostic(&d, text);
                    if !out.contains(&d) {
                        out.push(d);
                    }
                }
            }
        }
        out
    }
}

/// lint（S053）を LSP の診断（warning、GM-L0001〜）にする。
fn lint_diagnostics(
    text: &str,
    options: &globe_mode_fmt::LintOptions,
) -> Vec<lsp_types::Diagnostic> {
    globe_mode_fmt::lint(text, options)
        .into_iter()
        .map(|m| {
            let (code, severity) = match m.rule {
                "line-too-long" => ("GM-L0001", DiagnosticSeverity::WARNING),
                "class-name-pascal-case" => ("GM-L0002", DiagnosticSeverity::WARNING),
                "missing-description" => ("GM-L0003", DiagnosticSeverity::WARNING),
                // S061: 整形で直る並びの指摘は通知の重さ
                _ => ("GM-L0004", DiagnosticSeverity::INFORMATION),
            };
            let start = Position::new(
                m.line.saturating_sub(1) as u32,
                m.column.saturating_sub(1) as u32,
            );
            lsp_types::Diagnostic {
                range: Range::new(start, start),
                severity: Some(severity),
                code: Some(NumberOrString::String(code.to_string())),
                source: Some("globe-mode".to_string()),
                message: format!("{} ({})", m.message, m.rule),
                ..Default::default()
            }
        })
        .collect()
}

/// 標準入出力で LSP を回す。`exit` か入力の終わりで返る。
pub fn run_stdio(libraries: &[PathBuf]) -> io::Result<()> {
    let server = Server::new(libraries).map_err(|ds| {
        let msgs: Vec<_> = ds.iter().map(|d| d.message.as_str()).collect();
        io::Error::other(msgs.join("; "))
    })?;
    serve(server, io::BufReader::new(io::stdin()), io::stdout().lock())
}

/// 実行の要求（`$/cancelRequest` で止められるもの。ADR-0033）。
fn cancellable(method: &str) -> bool {
    matches!(
        method,
        "globe/simulate" | "globe/simulateColumns" | "globe/fmuRun"
    )
}

/// `globe/fmuRun`（S229）。ワーカーは自分と同じ実行ファイル（`globe lsp` の `globe`）の `fmu __worker`。
#[cfg(feature = "fmu-import")]
fn fmu_run(
    id: Option<Value>,
    params: &Value,
    cancel: Option<CancelToken>,
    notify: &mut dyn FnMut(Value),
) -> Vec<Value> {
    let Some(path) = params["path"].as_str() else {
        return error(id, -32602, "params.path が文字列でない");
    };
    let request = match serde_json::from_value(params.get("request").cloned().unwrap_or(json!({})))
    {
        Ok(r) => r,
        Err(e) => return error(id, -32602, &format!("params.request が読めない: {e}")),
    };
    let worker = match std::env::current_exe() {
        Ok(w) => w,
        Err(e) => {
            return error(
                id,
                -32000,
                &format!("ワーカーの実行ファイルが分からない: {e}"),
            );
        }
    };
    let token = params.get("workDoneToken");
    let mut progress = |value: Value| {
        if let Some(token) = token {
            notify(json!({"jsonrpc": "2.0", "method": "$/progress",
                "params": {"token": token, "value": value}}));
        }
    };
    progress(json!({"kind": "begin", "title": format!("fmu {path}"), "cancellable": true}));
    let run = globe_mode::run_fmu(
        &worker,
        std::path::Path::new(path),
        &request,
        &mut |t| {
            progress(
                json!({"kind": "report", "message": "integration", "stage": "integration", "time": t}),
            )
        },
        cancel.as_ref(),
    );
    progress(json!({"kind": "end"}));
    match run {
        Ok(r) => {
            let (layout, columns) = r.results.columns();
            reply(
                id,
                json!({"layout": layout, "columns": columns, "me": r.me}),
            )
        }
        Err(e) if cancel.is_some_and(|c| c.is_cancelled()) => error(id, -32800, &e),
        Err(e) => error(id, -32000, &e),
    }
}

/// 読みと計算を分けて回す（S136、ADR-0033）。読みのスレッドは枠を読んでキューに積み、実行の要求には
/// トークンを作って表に置く。`$/cancelRequest` はキューに積まず、表のトークンを立てる（待っている要求にも効く）。
/// 計算はこのスレッドで 1 度に 1 つ。書くのもこのスレッドだけ。
fn serve(
    mut server: Server,
    mut input: impl BufRead + Send + 'static,
    mut out: impl Write,
) -> io::Result<()> {
    let tokens: Arc<Mutex<HashMap<String, CancelToken>>> = Arc::default();
    let (tx, rx) = mpsc::channel::<io::Result<(Value, Option<CancelToken>)>>();
    let table = tokens.clone();
    let reader = thread::spawn(move || {
        loop {
            let msg = match read_message(&mut input) {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            };
            let method = msg["method"].as_str().unwrap_or("");
            if method == "exit" {
                break;
            }
            if method == "$/cancelRequest" {
                if let Some(t) = lock(&table).get(&msg["params"]["id"].to_string()) {
                    t.cancel();
                }
                continue;
            }
            let token = (cancellable(method) && msg.get("id").is_some()).then(|| {
                let t = CancelToken::new();
                lock(&table).insert(msg["id"].to_string(), t.clone());
                t
            });
            if tx.send(Ok((msg, token))).is_err() {
                break;
            }
        }
    });
    let mut write = |m: Value| -> io::Result<()> {
        let body = m.to_string();
        write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
        out.flush()
    };
    // S214: 積まれた編集をまとめる。読み切ったあと、次も同じ文書の didChange なら今のは古い版なので検査せずに捨てる（全文の同期）
    let mut queue = VecDeque::new();
    loop {
        if queue.is_empty() {
            match rx.recv() {
                Ok(got) => queue.push_back(got),
                Err(_) => break,
            }
        }
        queue.extend(rx.try_iter());
        let Some(got) = queue.pop_front() else {
            continue;
        };
        let (msg, token) = got?;
        if let (Some(uri), Some(Ok((next, _)))) = (change_uri(&msg), queue.front())
            && change_uri(next) == Some(uri)
        {
            continue;
        }
        let id = msg["id"].to_string();
        let (mut failed, rid) = (Ok(()), msg.get("id").cloned());
        // 1 つの要求の panic（解析の想定外）でプロセスごと落とさない（S198）。panic の内容は標準エラーに出る（既定のフック）。
        let replies = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            server.handle_with(msg, token, &mut |m| {
                if failed.is_ok() {
                    failed = write(m);
                }
            })
        }))
        .unwrap_or_else(|_| error(rid, -32603, "内部エラー（panic）。標準エラーを見る"));
        lock(&tokens).remove(&id);
        failed?;
        for m in replies {
            write(m)?;
        }
    }
    // 読みのスレッドは入力の終わりか `exit` で抜けている（キューが閉じたので）
    let _ = reader.join();
    Ok(())
}

/// 要求に `max_wall_seconds` が無ければ既定の予算を置く（S214）。
fn with_default_budget(request: &mut SimulateRequest) {
    request
        .settings
        .max_wall_seconds
        .get_or_insert(DEFAULT_MAX_WALL_SECONDS);
}

/// didChange の文書の URI。
fn change_uri(msg: &Value) -> Option<&str> {
    (msg["method"] == "textDocument/didChange")
        .then(|| msg["params"]["textDocument"]["uri"].as_str())
        .flatten()
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// `Content-Length` の枠 1 つを読む。入力が終わっていれば None。
fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.eq_ignore_ascii_case("content-length")
        {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let len =
        len.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Content-Length が無い"))?;
    let mut buf = vec![0; len];
    input.read_exact(&mut buf)?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// 要求（id あり）への応答。通知なら何も返さない。
fn reply(id: Option<Value>, result: Value) -> Vec<Value> {
    id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": result}))
        .into_iter()
        .collect()
}

/// 要求（id あり）へのエラー応答。通知なら何も返さない。
fn error(id: Option<Value>, code: i32, message: &str) -> Vec<Value> {
    id.map(|id| json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
        .into_iter()
        .collect()
}

fn publish(uri: &str, diagnostics: &[lsp_types::Diagnostic]) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
           "params": {"uri": uri, "diagnostics": diagnostics}})
}

/// ライブラリの木の項目の JSON（`globe/libraryTree`・`globe/librarySearch`。wasm の `library_tree`・`library_search` も使う）。
pub fn tree_entries_json(entries: &[globe_mode_fmt::TreeEntry<'_>]) -> Vec<Value> {
    entries
        .iter()
        .map(|e| {
            json!({"name": e.name, "path": e.path, "kind": e.kind,
                "description": e.description, "file": e.file, "hasChildren": e.has_children})
        })
        .collect()
}

/// URI をワークスペースのパスにする。`file:` はパーセントを戻した path、それ以外は URI の文字列のまま。
/// `path` の下の `.mo`（`path` が `.mo` ならそれ自身）。読めないディレクトリは飛ばす。
fn collect_mo(path: &std::path::Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
            collect_mo(&entry.path(), out);
        }
    } else if path.extension().is_some_and(|e| e == "mo") {
        out.push(path.to_path_buf());
    }
}

/// クラスのリネームが、ディスクの上のほかのファイルと食い違うか（S198。`textDocument/rename` と `prepareRename` だけ）。理由を返す。
/// ファイルの最上位のクラスがファイル名（`M.mo` の `model M`、`package.mo`）に対応しているときと、
/// ワークスペースの根の下の開いていない `.mo` に同じ名前の識別子があるとき（字句の一致だけ。部品の同名でも断る）。
/// shortcut: 根の下の `.mo` を毎回すべて読む（文字列に `old` を含むものだけ字句に分ける。MSL 全体で数百 ms）。大きな根で遅いなら索引を持つ。
fn class_rename_blocked(
    r: &mut definition::Resolver,
    root: Option<&std::path::Path>,
    open: &[String],
    path: &str,
    text: &str,
    at: usize,
) -> Option<String> {
    let (old, t) = definition::class_name_at(r, path, text, at)?;
    if t.path != path {
        return None; // 別のファイルのクラスは `rename` が断る
    }
    let o = r.ws.outline(path)?.as_ref().ok()?;
    let name = std::path::Path::new(path).file_name()?.to_string_lossy();
    if o.classes.iter().any(|c| c.range == t.range)
        && (name == "package.mo" || name.strip_suffix(".mo") == Some(old.as_str()))
    {
        return Some(format!(
            "{old} はファイル {name} に対応する最上位のクラスで、リネームするとファイル名とほかのファイルの参照が食い違う"
        ));
    }
    let mut files = Vec::new();
    collect_mo(root?, &mut files);
    // Windows: URI 由来のパスは `/`、走査したパスは `\` が混ざるので、比べる前にそろえる
    let path = normalize_path(path);
    let open: Vec<String> = open.iter().map(|p| normalize_path(p)).collect();
    files
        .into_iter()
        .map(|f| normalize_path(&f.display().to_string()))
        .find_map(|f| {
            if f == path || open.contains(&f) {
                return None;
            }
            let body = std::fs::read_to_string(&f)
                .ok()
                .filter(|b| b.contains(old.as_str()))?;
            let hit = globe_mode_fmt::lex(&body)
                .ok()?
                .iter()
                .any(|k| k.text == old);
            hit.then(|| {
                format!(
                    "開いていないファイル {f} にも {old} がある（リネームは開いている文書の中だけ）"
                )
            })
        })
}

/// LSP の `Position` の JSON（読めない欄は 0）。
fn lsp_position(p: &Value) -> Position {
    Position::new(
        p["line"].as_u64().unwrap_or(0) as u32,
        p["character"].as_u64().unwrap_or(0) as u32,
    )
}

/// パス → `file:` の URI（[`uri_path`] の逆。英数字と `/-._~:` 以外はパーセントにする）。
fn file_uri(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut out = String::from(if path.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn uri_path(uri: &str) -> String {
    match uri.parse::<Uri>() {
        Ok(u) if u.scheme().is_some_and(|s| s.as_str() == "file") => {
            let p = u.path().as_estr().decode().into_string_lossy().into_owned();
            // Windows: `/C:/x` → `C:/x`
            match p.as_bytes() {
                [b'/', d, b':', ..] if d.is_ascii_alphabetic() => p[1..].to_string(),
                _ => p,
            }
        }
        _ => uri.to_string(),
    }
}

/// バイト位置 → LSP の位置（行は 0 始まり、列は UTF-16 の単位）。範囲外は末尾、文字の途中は手前に寄せる。
fn position(text: &str, offset: u32) -> Position {
    let mut end = (offset as usize).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line = before.matches('\n').count() as u32;
    let tail = before.rsplit('\n').next().unwrap_or("");
    Position::new(line, tail.encode_utf16().count() as u32)
}

/// LSP の位置 → バイト位置。行の終わりを越える列は行末、行が足りなければ本文の末尾。
fn offset(text: &str, pos: Position) -> usize {
    let mut start = 0;
    for _ in 0..pos.line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let line = &text[start..];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    let mut units = 0;
    for (i, c) in line.char_indices() {
        if units >= pos.character {
            return start + i;
        }
        units += c.len_utf16() as u32;
    }
    start + line.len()
}

fn range(text: &str, r: globe_mode_core::TextRange) -> Range {
    Range::new(position(text, r.start), position(text, r.end))
}

fn lsp_diagnostic(d: &Diagnostic, text: &str) -> lsp_types::Diagnostic {
    let mut message = d.message.clone();
    for n in &d.notes {
        message.push('\n');
        message.push_str(n);
    }
    lsp_types::Diagnostic {
        range: d.range.map_or_else(Range::default, |r| range(text, r)),
        severity: Some(match d.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(d.code.to_string())),
        source: Some("globe-mode".to_string()),
        message,
        ..Default::default()
    }
}

fn keyword(kind: ClassKind) -> &'static str {
    match kind {
        ClassKind::Model => "model",
        ClassKind::Class => "class",
        ClassKind::Block => "block",
        ClassKind::Connector => "connector",
        ClassKind::Record => "record",
        ClassKind::Type => "type",
        ClassKind::Package => "package",
        ClassKind::Function => "function",
        ClassKind::Operator => "operator",
    }
}

/// クラス → シンボル。子は入れ子のクラスとコンポーネント。
#[allow(deprecated)] // DocumentSymbol::deprecated は構造体の初期化で必ず書く
fn symbol(c: &ClassOutline, text: &str) -> DocumentSymbol {
    let r = range(text, c.range);
    let kind = match c.kind {
        ClassKind::Package => SymbolKind::MODULE,
        ClassKind::Record => SymbolKind::STRUCT,
        ClassKind::Connector => SymbolKind::INTERFACE,
        ClassKind::Function | ClassKind::Operator => SymbolKind::FUNCTION,
        _ => SymbolKind::CLASS,
    };
    let mut children: Vec<_> = c.classes.iter().map(|k| symbol(k, text)).collect();
    children.extend(c.components.iter().map(|v| {
        let r = range(text, v.range);
        DocumentSymbol {
            name: v.name.clone(),
            detail: Some(v.type_name.clone()),
            kind: SymbolKind::FIELD,
            tags: None,
            deprecated: None,
            range: r,
            selection_range: r,
            children: None,
        }
    }));
    DocumentSymbol {
        name: c.name.clone(),
        detail: Some(keyword(c.kind).to_string()),
        kind,
        tags: None,
        deprecated: None,
        range: r,
        selection_range: r,
        children: Some(children),
    }
}

/// 位置（バイト）を含む一番内側のクラス。
fn innermost(classes: &[ClassOutline], at: u32) -> Option<&ClassOutline> {
    let c = classes
        .iter()
        .find(|c| c.range.start <= at && at < c.range.end)?;
    Some(innermost(&c.classes, at).unwrap_or(c))
}

fn hover(c: &ClassOutline) -> Hover {
    let partial = if c.partial { "partial " } else { "" };
    let mut value = format!("**{partial}{} {}**", keyword(c.kind), c.name);
    if !c.extends.is_empty() {
        value.push_str(&format!("\n\nextends {}", c.extends.join(", ")));
    }
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #64: 別の文書を直す・閉じると、それに依存する開いている文書の診断も出し直される（変わらない文書には出さない）
    #[test]
    fn editing_or_closing_a_document_republishes_the_ones_that_depend_on_it() {
        let mut s = Server::new(&[]).unwrap();
        let open = |s: &mut Server, uri: &str, text: &str| {
            s.handle(
                json!({"method": "textDocument/didOpen", "params": {"textDocument": {
                "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
            )
        };
        // 出し直されたかと、その誤り（severity 1）の数。lint の warning は文書に依らず付くので数えない
        let diags_of = |out: &[Value], uri: &str| -> Option<usize> {
            out.iter().find(|m| m["params"]["uri"] == uri).map(|m| {
                let d = m["params"]["diagnostics"].as_array().unwrap();
                d.iter().filter(|d| d["severity"] == 1).count()
            })
        };
        open(&mut s, "file:///P.mo", "model P\n  Real R;\nend P;\n");
        let out = open(
            &mut s,
            "file:///T.mo",
            "model T\n  extends P;\nequation\n  R = 1;\nend T;\n",
        );
        assert_eq!(diags_of(&out, "file:///T.mo"), Some(0), "{out:?}");
        // R を Q に変える。T.mo は変えていないが、T の R が未定義になる
        let out = s.handle(json!({"method": "textDocument/didChange", "params": {
            "textDocument": {"uri": "file:///P.mo"},
            "contentChanges": [{"text": "model P\n  Real Q;\nend P;\n"}]}}));
        assert!(
            diags_of(&out, "file:///T.mo").is_some_and(|n| n > 0),
            "{out:?}"
        );
        // もう一度同じ内容を送っても、T.mo の診断は変わらないので出し直さない
        let out = s.handle(json!({"method": "textDocument/didChange", "params": {
            "textDocument": {"uri": "file:///P.mo"},
            "contentChanges": [{"text": "model P\n  Real Q;\nend P;\n"}]}}));
        assert_eq!(diags_of(&out, "file:///T.mo"), None, "{out:?}");
        // P を直すと消える
        let out = s.handle(json!({"method": "textDocument/didChange", "params": {
            "textDocument": {"uri": "file:///P.mo"},
            "contentChanges": [{"text": "model P\n  Real R;\nend P;\n"}]}}));
        assert_eq!(diags_of(&out, "file:///T.mo"), Some(0), "{out:?}");
        // P を閉じると、T の extends P が解決できなくなる
        let out = s.handle(json!({"method": "textDocument/didClose", "params": {
            "textDocument": {"uri": "file:///P.mo"}}}));
        assert!(
            diags_of(&out, "file:///T.mo").is_some_and(|n| n > 0),
            "{out:?}"
        );
    }

    #[test]
    fn annotation_order_is_information_and_blank_sections_can_be_turned_off() {
        let src = "model A \"d\"\n  Real x;\nequation\n  x = 1;\n  annotation(Documentation(info = \"i\"), experiment(StopTime = 1));\nend A;\n";
        let d = lint_diagnostics(src, &globe_mode_fmt::LintOptions::default());
        let order: Vec<_> = d
            .iter()
            .filter(|d| d.code == Some(NumberOrString::String("GM-L0004".into())))
            .collect();
        assert_eq!(order.len(), 1, "{d:?}");
        assert_eq!(order[0].severity, Some(DiagnosticSeverity::INFORMATION));

        let mut s = Server::new(&[]).unwrap();
        // S062: formatHtml の切り替え
        let html = "model H \"d\"\n  annotation(Documentation(info = \"<html><body><p>x</p></body></html>\"));\nend H;\n";
        for (on, want) in [(true, "info = \"\n"), (false, "info = \"<html><body>")] {
            let r = s.handle(json!({"id": 1, "method": "globe/format", "params": {"text": html, "formatHtml": on}}));
            assert!(r[0]["result"].as_str().unwrap().contains(want), "{}", r[0]);
        }
        let r = s.handle(json!({"id": 1, "method": "globe/format", "params": {"text": src, "blankAroundSections": false}}));
        let out = r[0]["result"].as_str().unwrap();
        assert!(out.contains("  Real x;\nequation\n  x = 1;"), "{out}");
        assert!(
            out.contains("annotation(experiment(StopTime = 1), Documentation(info = \"i\"));"),
            "{out}"
        );
    }

    #[test]
    fn position_counts_utf16_units() {
        // 「日本語」は UTF-8 で 3 バイト、UTF-16 で 1 単位。絵文字は UTF-16 で 2 単位
        let text = "// 日本語😀x\nmodel A end A;";
        let x = text.find('x').unwrap() as u32;
        assert_eq!(position(text, x), Position::new(0, 3 + 3 + 2));
        assert_eq!(
            position(text, text.find("model").unwrap() as u32),
            Position::new(1, 0)
        );
        // 文字の途中は手前に寄せ、範囲外は末尾
        assert_eq!(position(text, 4), Position::new(0, 3));
        assert_eq!(position(text, 9999).line, 1);
    }

    #[test]
    fn offset_is_the_inverse_of_position() {
        let text = "// 日本語😀x\nmodel A end A;";
        for (i, _) in text.char_indices() {
            assert_eq!(offset(text, position(text, i as u32)), i);
        }
        assert_eq!(
            offset(text, Position::new(0, 999)),
            text.find('\n').unwrap()
        );
        assert_eq!(offset(text, Position::new(9, 0)), text.len());
    }

    #[test]
    fn uri_to_path() {
        assert_eq!(uri_path("file:///home/a%20b/X.mo"), "/home/a b/X.mo");
        assert_eq!(uri_path("file:///C:/x/X.mo"), "C:/x/X.mo");
        assert_eq!(uri_path("untitled:Untitled-1"), "untitled:Untitled-1");
    }

    #[test]
    fn diagnostic_mapping_keeps_code_notes_and_range() {
        let text = "// 日本語\nmodel A end A;";
        let mut d = Diagnostic::error("GM-E6001", globe_mode::Stage::S6, "壊れている");
        d.range = Some(globe_mode_core::TextRange { start: 0, end: 2 });
        d.notes = vec!["補足".into()];
        let l = lsp_diagnostic(&d, text);
        assert_eq!(l.code, Some(NumberOrString::String("GM-E6001".into())));
        assert_eq!(l.message, "壊れている\n補足");
        assert_eq!(l.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(
            l.range,
            Range::new(Position::new(0, 0), Position::new(0, 2))
        );
        d.range = None;
        assert_eq!(lsp_diagnostic(&d, text).range, Range::default());
    }

    #[test]
    fn server_answers_open_symbol_hover_and_unknown() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/A.mo";
        let out = s.handle(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "modelica", "version": 1,
            "text": "model A\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend A;\n"}}}),
        );
        assert_eq!(out[0]["method"], "textDocument/publishDiagnostics");
        // 警告と情報は出るが、誤り（severity 1）は無い
        let diags = out[0]["params"]["diagnostics"].as_array().unwrap();
        assert!(!diags.is_empty() && diags.iter().all(|d| d["severity"] != 1));
        let sym = s.handle(json!({"id": 1, "method": "textDocument/documentSymbol", "params": {"textDocument": {"uri": uri}}}));
        assert_eq!(sym[0]["result"][0]["name"], "A");
        assert_eq!(sym[0]["result"][0]["children"][0]["name"], "x");
        let h = s.handle(json!({"id": 2, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": 0, "character": 7}}}));
        assert_eq!(h[0]["result"]["contents"]["value"], "**model A**");
        let h = s.handle(json!({"id": 3, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": 9, "character": 0}}}));
        assert_eq!(h[0]["result"], Value::Null);
        assert_eq!(
            s.handle(json!({"id": 4, "method": "foo/bar"}))[0]["error"]["code"],
            -32601
        );
        assert!(s.handle(json!({"method": "foo/bar"})).is_empty());
    }

    #[test]
    fn connect_with_different_types_is_a_warning() {
        let mut s = Server::new(&[]).unwrap();
        let src = "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n\
                   connector Port\n  Real T;\n  flow Real Q;\nend Port;\n\
                   model R\n  Pin p;\n  Port h;\nend R;\n\
                   model C\n  R a;\n  R b;\nequation\n  connect(a.p, b.p);\n  connect(a.p, b.h);\nend C;\n";
        let out = s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "A.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let d: Vec<_> = out[0]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == "GM-L0005")
            .collect();
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0]["severity"], 2);
        assert_eq!(d[0]["range"]["start"]["line"], 17); // 2 本目の connect
        assert!(d[0]["message"].as_str().unwrap().contains("Port"));
    }

    /// S173: 検査の応答の `can_force`、`force: true` の実行（RPC で通る）
    #[test]
    fn force_runs_an_over_determined_initialization() {
        let mut s = Server::new(&[]).unwrap();
        let src = "model Over\n  Real x(start = 1, fixed = true);\ninitial equation\n  x = 1;\nequation\n  der(x) = -x;\nend Over;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "A.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let r = s.handle(json!({"id": 1, "method": "globe/check", "params": {"model": "Over"}}));
        assert_eq!(r[0]["result"]["can_force"], true, "{}", r[0]);
        // S184: 結果の 1 行は応答の `summary`
        assert!(
            r[0]["result"]["summary"]
                .as_str()
                .unwrap()
                .starts_with("Over: NG（S8 "),
            "{}",
            r[0]
        );
        let mut run = |force: bool| {
            s.handle(
                json!({"id": 2, "method": "globe/simulate", "params": {"request": {
            "model": "Over", "force": force, "no_csv": true}}}),
            )
        };
        let refused = run(false);
        assert_eq!(refused[0]["result"]["can_force"], true, "{}", refused[0]);
        let forced = run(true);
        assert!(
            forced[0]["result"]["final"]["x"].is_number(),
            "{}",
            forced[0]
        );
        assert!(forced[0]["result"].get("can_force").is_none());
        assert!(
            refused[0]["result"]["summary"]
                .as_str()
                .unwrap()
                .contains("force で強行できる）。最初の error: GM-E8001 "),
            "{}",
            refused[0]
        );
    }

    #[test]
    fn custom_requests_check_simulate_and_graph() {
        let mut s = Server::new(&[]).unwrap();
        let src = "connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n\
                   model R\n  Pin p;\n  Pin n;\nequation\n  p.v - n.v = p.i;\n  p.i + n.i = 0;\nend R;\n\
                   model C\n  R a;\n  R b;\nequation\n  connect(a.n, b.p);\nend C;\n\
                   model D\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend D;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "A.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let r = s.handle(json!({"id": 1, "method": "globe/check", "params": {"model": "D"}}));
        assert_eq!(r[0]["result"]["model"], "D");
        assert_eq!(r[0]["result"]["passed"], true);
        let r = s.handle(
            json!({"id": 2, "method": "globe/simulate", "params": {"request": {
            "model": "D", "settings": {"stop_time": 1.0}, "no_csv": true}}}),
        );
        assert_eq!(r[0]["result"]["ok"], true, "{}", r[0]);
        let x = r[0]["result"]["final"]["x"].as_f64().unwrap();
        assert!((x - (-1f64).exp()).abs() < 1e-2, "x = {x}");
        let r = s.handle(
            json!({"id": 4, "method": "globe/simulateColumns", "params": {"request": {
            "model": "D", "settings": {"stop_time": 1.0}}}}),
        );
        let res = &r[0]["result"];
        assert_eq!(res["response"]["ok"], true, "{}", r[0]);
        assert!(res["response"]["csv"].is_null());
        assert_eq!(res["layout"]["names"], json!(["time", "x"]));
        assert_eq!(res["layout"]["defaults"], json!(["x"])); // S190
        let cols = res["columns"].as_array().unwrap();
        let n = res["layout"]["points"].as_u64().unwrap() as usize;
        assert!(n > 1 && cols.iter().all(|c| c.as_array().unwrap().len() == n));
        assert_eq!(cols[1][n - 1].as_f64().unwrap(), x);
        let r = s.handle(
            json!({"id": 3, "method": "globe/graph", "params": {"model": "C", "depth": 1}}),
        );
        assert_eq!(
            r[0]["result"]["edges"].as_array().unwrap().len(),
            1,
            "{}",
            r[0]
        );
        // 読めない params は -32602、モデルが無ければ -32000、通知（id 無し）には何も返さない
        for (method, params, code) in [
            ("globe/check", json!({}), -32602),
            ("globe/simulate", json!({"request": {"nope": 1}}), -32602),
            (
                "globe/simulateColumns",
                json!({"request": {"nope": 1}}),
                -32602,
            ),
            ("globe/graph", json!({"model": "Nope"}), -32000),
        ] {
            let r = s.handle(json!({"id": 9, "method": method, "params": params}));
            assert_eq!(r[0]["error"]["code"], code, "{method}: {}", r[0]);
        }
        assert!(
            s.handle(json!({"method": "globe/check", "params": {}}))
                .is_empty()
        );
    }

    #[test]
    fn export_fmu_writes_the_source_fmu() {
        let mut s = Server::new(&[]).unwrap();
        let src = "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "Decay.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let dir = std::env::temp_dir().join(format!("globe-lsp-fmu-{}", std::process::id()));
        let out = dir.to_string_lossy().to_string();
        std::fs::create_dir_all(&dir).unwrap();
        let r = s.handle(json!({"id": 1, "method": "globe/exportFmu",
            "params": {"model": "Decay", "outDir": out}}));
        let path = PathBuf::from(r[0]["result"]["path"].as_str().unwrap());
        assert_eq!(path, dir.join("Decay-fmi2.fmu"));
        assert!(path.is_file());
        assert!(dir.join("Decay/modelDescription.xml").is_file());
        let r = s.handle(json!({"id": 2, "method": "globe/exportFmu",
            "params": {"model": "Nope", "outDir": out}}));
        assert_eq!(r[0]["error"]["code"], -32000);
        let r =
            s.handle(json!({"id": 3, "method": "globe/exportFmu", "params": {"model": "Decay"}}));
        assert_eq!(r[0]["error"]["code"], -32602);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tunable_parameters_are_listed() {
        let mut s = Server::new(&[]).unwrap();
        let src = "model D\n  parameter Real k(unit = \"1/s\") = 1 \"rate\";\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -k * x;\nend D;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "D.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let r = s.handle(
            json!({"id": 1, "method": "globe/tunableParameters", "params": {"model": "D"}}),
        );
        let p = &r[0]["result"]["parameters"][0];
        assert_eq!(
            (p["name"].as_str(), p["default"].as_f64()),
            (Some("k"), Some(1.0)),
            "{}",
            r[0]
        );
        assert_eq!(p["unit"], "1/s");
        assert_eq!(p["description"], "rate");
        let r = s.handle(
            json!({"id": 2, "method": "globe/tunableParameters", "params": {"model": "Nope"}}),
        );
        assert_eq!(r[0]["error"]["code"], -32000);
    }

    #[test]
    fn solvers_and_experiment() {
        let mut s = Server::new(&[]).unwrap();
        let r = s.handle(json!({"id": 1, "method": "globe/solvers"}));
        let all = r[0]["result"].as_array().unwrap();
        assert!(
            all.iter()
                .any(|e| e["name"] == "dopri45" && e["status"]["kind"] == "available")
        );
        let src = "model E\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\n  annotation(experiment(StopTime = 3, Tolerance = 1e-5));\nend E;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "E.mo", "languageId": "modelica", "version": 1, "text": src}}}),
        );
        let r = s.handle(json!({"id": 2, "method": "globe/check", "params": {"model": "E"}}));
        let e = &r[0]["result"]["experiment"];
        assert_eq!(
            (e["stop_time"].as_f64(), e["tolerance"].as_f64()),
            (Some(3.0), Some(1e-5)),
            "{}",
            r[0]
        );
        assert!(e["interval"].is_null());
    }

    #[test]
    fn library_tree_and_source() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let r = s.handle(json!({"id": 1, "method": "globe/libraryTree", "params": {"prefix": ""}}));
        assert_eq!(r[0]["result"][0]["path"], "GlobeModeValidation", "{}", r[0]);
        let r = s.handle(json!({"id": 2, "method": "globe/libraryTree", "params": {"prefix": "GlobeModeValidation.Simulation"}}));
        assert!(
            r[0]["result"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["name"] == "Decay" && e["kind"] == "model"),
            "{}",
            r[0]
        );
        let model = "GlobeModeValidation.Simulation.Decay";
        let r =
            s.handle(json!({"id": 3, "method": "globe/librarySource", "params": {"path": model}}));
        let (file, text) = (
            r[0]["result"]["file"].as_str().unwrap(),
            r[0]["result"]["text"].as_str().unwrap(),
        );
        assert!(
            file.ends_with("Decay.mo") && text.contains("model Decay"),
            "{}",
            r[0]
        );
        // 開くとライブラリの同じファイルを差し替える（検査が通る）
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": file, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let r = s.handle(json!({"id": 4, "method": "globe/check", "params": {"model": model}}));
        assert_eq!(r[0]["result"]["passed"], true, "{}", r[0]);
        let r =
            s.handle(json!({"id": 5, "method": "globe/librarySource", "params": {"path": "Nope"}}));
        assert_eq!(r[0]["error"]["code"], -32000);
        // S167: ファイルの中の入れ子のクラスも木の子になり、検索で見つかる。開くのは定義するファイル
        let r = s.handle(json!({"id": 6, "method": "globe/libraryTree", "params": {"prefix": "GlobeModeValidation.Components"}}));
        let names: Vec<_> = r[0]["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert!(
            names.contains(&"Resistor") && names.contains(&"Switch"),
            "{names:?}"
        );
        let r = s.handle(
            json!({"id": 7, "method": "globe/librarySearch", "params": {"query": "resis"}}),
        );
        let hit = &r[0]["result"][0];
        assert_eq!(
            hit["path"], "GlobeModeValidation.Components.Resistor",
            "{}",
            r[0]
        );
        assert!(
            hit["file"].as_str().unwrap().ends_with("Components.mo"),
            "{hit}"
        );
        let r = s.handle(
            json!({"id": 8, "method": "globe/librarySource", "params": {"path": hit["path"]}}),
        );
        assert!(
            r[0]["result"]["text"]
                .as_str()
                .unwrap()
                .contains("model Resistor"),
            "{}",
            r[0]
        );
        // S186: 入れ子のクラスを開くとき、宣言の行（0 始まり）が range で分かる。1 行目ではない
        let text = r[0]["result"]["text"].as_str().unwrap();
        let line = r[0]["result"]["range"]["start"]["line"].as_u64().unwrap() as usize;
        assert!(
            line > 0 && text.lines().nth(line).unwrap().contains("model Resistor"),
            "{}",
            r[0]
        );
        let r = s.handle(json!({"id": 9, "method": "globe/librarySearch", "params": {"query": 1}}));
        assert_eq!(r[0]["error"]["code"], -32602);
    }

    #[test]
    fn config_file_at_the_root_drives_lint_and_formatting() {
        let dir = std::env::temp_dir().join(format!("globe-lsp-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let toml = dir.join("globe-mode.toml");
        std::fs::write(
            &toml,
            "[fmt]\nindent = 4\n[lint]\ndisable = [\"GM-L0003\", \"class-name-pascal-case\"]\n",
        )
        .unwrap();
        let mut s = Server::new(&[]).unwrap();
        let init = s.handle(json!({"id": 0, "method": "initialize", "params": {"rootPath": dir}}));
        assert_eq!(init.len(), 1, "{init:?}");
        let uri = "file:///tmp/C.mo";
        let text = "model C\nReal x;\nend C;\n";
        let out = s.handle(json!({"method": "textDocument/didOpen", "params": {"textDocument": {"uri": uri, "text": text}}}));
        assert!(!out[0].to_string().contains("GM-L0003"), "{}", out[0]);
        // エディタの tabSize 2 より設定ファイルの indent 4
        let f = s.handle(json!({"id": 1, "method": "textDocument/formatting", "params": {"textDocument": {"uri": uri}, "options": {"tabSize": 2}}}));
        assert!(
            f[0]["result"][0]["newText"]
                .as_str()
                .unwrap()
                .contains("\n    Real x;"),
            "{}",
            f[0]
        );
        // S143: UI 向けの globe/format・globe/lint も、params に indent が無ければ設定ファイル、あればそれを採る
        let f = s.handle(json!({"id": 2, "method": "globe/format", "params": {"text": text}}));
        assert!(
            f[0]["result"].as_str().unwrap().contains("\n    Real x;"),
            "{}",
            f[0]
        );
        // S214: 設定ファイルがあれば params の indent（VS Code は常に tabSize を送る）より設定が勝つ
        let f = s.handle(
            json!({"id": 3, "method": "globe/format", "params": {"text": text, "indent": 2}}),
        );
        assert!(
            f[0]["result"].as_str().unwrap().contains("\n    Real x;"),
            "{}",
            f[0]
        );
        // 設定ファイルが無ければ params の indent（ブラウザ版の UI）
        let mut bare = Server::new(&[]).unwrap();
        let f = bare.handle(
            json!({"id": 3, "method": "globe/format", "params": {"text": text, "indent": 3}}),
        );
        assert!(
            f[0]["result"].as_str().unwrap().contains("\n   Real x;"),
            "{}",
            f[0]
        );
        let l = s.handle(
            json!({"id": 4, "method": "globe/lint", "params": {"text": "model c\nend c;\n"}}),
        );
        assert_eq!(l[0]["result"], json!([]), "{}", l[0]);
        // 誤りを書いて読み直すと showMessage で知らせ、既定に戻る
        std::fs::write(&toml, "[fmt]\nindnt = 4\n").unwrap();
        let out = s.handle(
            json!({"method": "workspace/didChangeConfiguration", "params": {"settings": {}}}),
        );
        assert_eq!(out[0]["method"], "window/showMessage");
        assert!(out[1].to_string().contains("GM-L0003"), "{}", out[1]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 文書を開き、`needle` の `k` 文字目に置いたカーソルで `method` を頼む。
    fn ask_at(
        s: &mut Server,
        uri: &str,
        text: &str,
        method: &str,
        needle: &str,
        k: usize,
    ) -> Value {
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let pos = position(text, (text.find(needle).unwrap() + k) as u32);
        s.handle(
            json!({"id": 1, "method": method, "params": {"textDocument": {"uri": uri},
            "position": pos, "context": {"includeDeclaration": true}}}),
        )
        .remove(0)
    }

    #[test]
    fn definition_of_a_type_and_a_component_in_the_same_document() {
        let mut s = Server::new(&[]).unwrap();
        let init = s.handle(json!({"id": 0, "method": "initialize", "params": {}}));
        assert_eq!(
            init[0]["result"]["capabilities"]["definitionProvider"],
            true
        );
        assert_eq!(
            init[0]["result"]["capabilities"]["referencesProvider"],
            true
        );
        let uri = "file:///tmp/D.mo";
        let text = "package D\n  connector Pin\n    Real v;\n    flow Real i;\n  end Pin;\n  model M\n    Pin p;\n  equation\n    p.v = 0;\n  end M;\nend D;\n";
        // 型 Pin の上 → connector Pin の範囲
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "Pin p;", 1);
        assert_eq!(r["result"]["uri"], uri, "{r}");
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 1, "character": 12}),
            "{r}"
        );
        // p.v の v → Pin の部品 v
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "p.v =", 2);
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 2, "character": 9}),
            "{r}"
        );
        // 予約語や数は null
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "= 0", 2);
        assert_eq!(r["result"], Value::Null, "{r}");
    }

    #[test]
    fn definition_into_another_file_of_the_library() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let uri = "file:///tmp/T.mo";
        let text = "model T\n  import GlobeModeValidation.Components;\n  Components.Resistor r1;\nequation\n  r1.p.v = 0;\nend T;\n";
        let r = ask_at(
            &mut s,
            uri,
            text,
            "textDocument/definition",
            "Resistor r1",
            1,
        );
        let u = r["result"]["uri"].as_str().unwrap_or_default();
        assert!(
            u.starts_with("file://") && u.ends_with("/Components.mo"),
            "{r}"
        );
        assert_eq!(r["result"]["range"]["start"]["line"], 19, "{r}");
        // r1.p の p は extends TwoPin の先の部品
        let r = ask_at(&mut s, uri, text, "textDocument/definition", "r1.p.v", 3);
        assert_eq!(
            r["result"]["range"]["start"],
            json!({"line": 6, "character": 19}),
            "{r}"
        );
    }

    #[test]
    fn globe_definition_returns_the_target_file_text() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let text = "model T\n  import GlobeModeValidation.Components;\n  Components.Resistor r1;\nend T;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "T.mo", "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let ask = |s: &mut Server, line: u32, character: u32| {
            s.handle(json!({"id": 1, "method": "globe/definition",
                "params": {"file": "T.mo", "line": line, "character": character}}))
                .remove(0)["result"]
                .clone()
        };
        let r = ask(&mut s, 2, 15); // Resistor
        assert!(
            r["file"].as_str().unwrap().ends_with("Components.mo"),
            "{r}"
        );
        assert!(r["text"].as_str().unwrap().contains("Resistor"), "{r}");
        assert_eq!(r["range"]["start"]["line"], 19, "{r}");
        assert_eq!(ask(&mut s, 0, 0), Value::Null); // 予約語 model
    }

    #[test]
    fn globe_hover_returns_the_declaration_and_description() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let text = "model T\n  import GlobeModeValidation.Components;\n  Components.Resistor r1;\nend T;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "T.mo", "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let r = s
            .handle(json!({"id": 1, "method": "globe/hover",
                "params": {"file": "T.mo", "line": 2, "character": 15}}))
            .remove(0)["result"]
            .clone();
        assert!(r["contents"].as_str().unwrap().contains("Resistor"), "{r}");
        assert_eq!(r["range"]["start"]["line"], 2, "{r}");
    }

    #[test]
    fn completion_lists_library_children_over_rpc() {
        let lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
        let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
        let text = "model T\n  GlobeModeValidation.Components.\nend T;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "T.mo", "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let labels = |r: Value| -> Vec<String> {
            r["result"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i["label"].as_str().unwrap().to_string())
                .collect()
        };
        let ui = s.handle(json!({"id": 1, "method": "globe/completion",
            "params": {"file": "T.mo", "line": 1, "character": 33}}));
        let ui = labels(ui[0].clone());
        assert!(ui.contains(&"Resistor".to_string()), "{ui:?}");
        let lsp = s.handle(json!({"id": 2, "method": "textDocument/completion",
            "params": {"textDocument": {"uri": "T.mo"}, "position": {"line": 1, "character": 33}}}));
        assert_eq!(labels(lsp[0].clone()), ui);
    }

    #[test]
    fn completion_is_incomplete_only_when_the_limit_cuts() {
        // S198: 200 件で切ったら `isIncomplete: true`、打ち足して絞れば false（配列ではなく CompletionList）
        let mut s = Server::new(&[]).unwrap();
        let comps: String = (0..250).map(|i| format!("  Real x{i:03};\n")).collect();
        let text = format!("model T\n{comps}  Real \nend T;\n");
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "T.mo", "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let ask = |s: &mut Server, method: &str, word: &str| {
            let line = 251; // `  Real ` の行
            let mut text = text.clone();
            text.insert_str(text.find("  Real \n").unwrap() + 7, word);
            s.handle(json!({"method": "textDocument/didChange", "params": {"textDocument": {"uri": "T.mo"},
                "contentChanges": [{"text": text}]}}));
            s.handle(json!({"id": 1, "method": method, "params": {"file": "T.mo", "textDocument": {"uri": "T.mo"},
                "line": line, "character": 7 + word.len(), "position": {"line": line, "character": 7 + word.len()}}}))
                .remove(0)["result"]
                .clone()
        };
        for m in ["globe/completion", "textDocument/completion"] {
            let r = ask(&mut s, m, "");
            assert_eq!(r["isIncomplete"], true, "{m}");
            assert_eq!(r["items"].as_array().unwrap().len(), 200);
            let r = ask(&mut s, m, "x24");
            assert_eq!(r["isIncomplete"], false, "{m}");
            assert!(
                r["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|i| i["label"] == "x249")
            );
        }
        // 開いていない文書も同じ形
        let r = s.handle(json!({"id": 2, "method": "textDocument/completion", "params": {"textDocument": {"uri": "none.mo"}, "position": {"line": 0, "character": 0}}}));
        assert_eq!(r[0]["result"], json!({"isIncomplete": false, "items": []}));
    }

    /// 一時ディレクトリ（テストごとに別の名前）。
    fn scratch_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("globe-lsp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn library_file_changed_on_disk_is_reread() {
        // S198: scratch に読んだライブラリのファイルがディスクで変わったら、次の要求で読み直す（古い範囲で本文を切らない）
        let dir = scratch_dir("stale");
        let lib = dir.join("Lib");
        std::fs::create_dir_all(&lib).unwrap();
        let file = lib.join("package.mo");
        std::fs::write(&file, "package Lib\n  model R\n  end R;\nend Lib;\n").unwrap();
        let mut s = Server::new(std::slice::from_ref(&dir)).unwrap();
        let text = "model T\n  Lib.R r;\nend T;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": "T.mo", "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let hover = |s: &mut Server| {
            s.handle(json!({"id": 1, "method": "globe/hover", "params": {"file": "T.mo", "line": 1, "character": 7}}))
                .remove(0)["result"]["contents"]
                .clone()
        };
        assert_eq!(hover(&mut s), "model R");
        let more = format!(
            "{}package Lib\n  model R\n  end R;\nend Lib;\n",
            "// padding\n".repeat(5)
        );
        std::fs::write(&file, more).unwrap();
        let t = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(t)
            .unwrap();
        assert_eq!(hover(&mut s), "model R");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn class_rename_refuses_what_the_disk_would_contradict() {
        // S198: M.mo の model M はファイル名に対応する。ほかのファイルが参照していれば別名のファイルでも断る
        let dir = scratch_dir("rename");
        let (m, foo, other) = (dir.join("M.mo"), dir.join("Foo.mo"), dir.join("Other.mo"));
        std::fs::write(&m, "model M\nend M;\n").unwrap();
        std::fs::write(&foo, "model Q\nend Q;\n").unwrap();
        std::fs::write(&other, "model Other\n  Q q;\nend Other;\n").unwrap();
        let mut s = Server::new(&[]).unwrap();
        s.handle(json!({"id": 0, "method": "initialize", "params": {"rootUri": file_uri(&dir.display().to_string())}}));
        let open = |s: &mut Server, p: &PathBuf, text: &str| {
            s.handle(json!({"method": "textDocument/didOpen", "params": {"textDocument": {
                "uri": file_uri(&p.display().to_string()), "languageId": "modelica", "version": 1, "text": text}}}))
        };
        open(&mut s, &m, "model M\nend M;\n");
        open(&mut s, &foo, "model Q\nend Q;\n");
        let ask = |s: &mut Server, method: &str, p: &PathBuf| {
            let uri = file_uri(&p.display().to_string());
            s.handle(json!({"id": 1, "method": method, "params": {"textDocument": {"uri": uri}, "file": uri,
                "position": {"line": 0, "character": 7}, "line": 0, "character": 7, "newName": "N"}}))
                .remove(0)
        };
        for method in ["textDocument/rename", "textDocument/prepareRename"] {
            let r = ask(&mut s, method, &m);
            assert_eq!(r["error"]["code"], -32803, "{method}: {r}");
            assert!(
                r["error"]["message"].as_str().unwrap().contains("M.mo"),
                "{r}"
            );
            let r = ask(&mut s, method, &foo);
            assert!(
                r["error"]["message"].as_str().unwrap().contains("Other.mo"),
                "{method}: {r}"
            );
        }
        // 参照するファイルが無ければ通る。UI の `globe/rename` はメモリ上の文書なので、ディスクを見ない
        std::fs::remove_file(&other).unwrap();
        assert!(ask(&mut s, "textDocument/rename", &foo)["result"].is_object());
        assert!(ask(&mut s, "globe/rename", &m)["result"].is_array());
        // 開いていない文書: references は空、リネームは断る
        let r = s.handle(json!({"id": 5, "method": "globe/references", "params": {"file": "none.mo", "line": 0, "character": 0}}));
        assert_eq!(r[0]["result"], json!([]));
        let r = s.handle(json!({"id": 6, "method": "globe/rename", "params": {"file": "none.mo", "line": 0, "character": 0, "newName": "x"}}));
        assert_eq!(r[0]["error"]["code"], -32803);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn references_in_the_same_class() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/R.mo";
        let text = "model R\n  Real x;\n  Real xx;\nequation\n  der(x) = -x + xx;\nend R;\n";
        let r = ask_at(&mut s, uri, text, "textDocument/references", "x;", 0);
        let lines: Vec<_> = r["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["range"]["start"]["line"].clone())
            .collect();
        assert_eq!(lines, [json!(1), json!(4), json!(4)], "{r}");
    }

    #[test]
    fn rename_edits_and_refusals_over_rpc() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/R.mo";
        let text = "model R\n  Real x;\n  Real xx;\nequation\n  der(x) = -x + xx;\nend R;\n";
        let init = s.handle(json!({"id": 0, "method": "initialize", "params": {}}));
        assert_eq!(
            init[0]["result"]["capabilities"]["renameProvider"],
            json!({"prepareProvider": true})
        );
        let r = ask_at(&mut s, uri, text, "textDocument/rename", "x;", 0);
        assert!(
            r["error"].is_object(),
            "newName が無いので識別子でない: {r}"
        );
        let mut ask = |method: &str, new: &str| {
            s.handle(json!({"id": 9, "method": method, "params": {"file": uri, "textDocument": {"uri": uri},
                "position": {"line": 1, "character": 7}, "line": 1, "character": 7, "newName": new}}))
                .remove(0)
        };
        assert_eq!(
            ask("globe/references", "")["result"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let edits = ask("globe/rename", "pos")["result"].clone();
        assert_eq!(edits.as_array().unwrap().len(), 3, "{edits}");
        let w = ask("textDocument/rename", "pos")["result"].clone();
        assert_eq!(w["changes"][uri], edits, "{w}");
        for (new, why) in [("1a", "識別子"), ("end", "予約語"), ("xx", "既に")] {
            let e = ask("globe/rename", new);
            assert_eq!(e["error"]["code"], -32803, "{e}");
            assert!(e["error"]["message"].as_str().unwrap().contains(why), "{e}");
        }
    }

    /// S170: UI のモデル名の候補。開いた文書の model・block（partial は除く）の修飾名
    #[test]
    fn models_over_rpc() {
        let mut s = Server::new(&[]).unwrap();
        let uri = "file:///tmp/M.mo";
        let text = "model A\nend A;\npartial model P\nend P;\nblock B\nend B;\n";
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        let r = s
            .handle(json!({"id": 3, "method": "globe/models", "params": {"file": uri}}))
            .remove(0);
        assert_eq!(r["result"], json!(["A", "B"]), "{r}");
    }

    /// S222: 結果のファイル（MAT v4 と CSV）を、`globe/simulateColumns` と同じ形の列で返す。
    #[test]
    fn read_results_over_rpc() {
        let mut s = Server::new(&[]).unwrap();
        let mat = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../globe-mode-sim/tests/data/omc_decay_res.mat"
        );
        let r = s.handle(json!({"id": 1, "method": "globe/readResults", "params": {"path": mat}}));
        let res = &r[0]["result"];
        assert_eq!(res["layout"]["names"][0], "time", "{}", r[0]);
        let n = res["layout"]["points"].as_u64().unwrap() as usize;
        assert!(
            n > 1
                && res["columns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|c| c.as_array().unwrap().len() == n)
        );
        let csv = std::env::temp_dir().join(format!("gm-lsp-read-{}.csv", std::process::id()));
        std::fs::write(&csv, "time,x\n0,1\n1,0.5\n").unwrap();
        let r = s.handle(json!({"id": 2, "method": "globe/readResults", "params": {"path": csv}}));
        assert_eq!(
            r[0]["result"]["layout"]["names"],
            json!(["time", "x"]),
            "{}",
            r[0]
        );
        assert_eq!(r[0]["result"]["layout"]["defaults"], json!(["x"]));
        assert_eq!(r[0]["result"]["columns"][1], json!([1.0, 0.5]));
        std::fs::remove_file(&csv).unwrap();
        for (params, code) in [
            (json!({}), -32602),
            (json!({"path": "/nonexistent/a.csv"}), -32000),
        ] {
            let r = s.handle(json!({"id": 3, "method": "globe/readResults", "params": params}));
            assert_eq!(r[0]["error"]["code"], code, "{}", r[0]);
        }
    }

    /// S229: `globe/fmuInfo`・`globe/fmuRun` の params の誤り（-32602）と、読めない FMU（fmuInfo の -32000）。
    /// **成功と fmuRun の -32000 は書かない**: ワーカーは `current_exe()` で、この試験ではテストのバイナリ（`fmu __worker` を
    /// 引数にしたテストの絞り込みで自分を再び起動する）。それらは `globe` を起動する globe-mode-cli の `tests/lsp.rs`。
    #[cfg(feature = "fmu-import")]
    #[test]
    fn fmu_requests_reject_bad_params_over_rpc() {
        let mut s = Server::new(&[]).unwrap();
        let bad = |s: &mut Server, method: &str, params: Value, code: i64| {
            let r = s.handle(json!({"id": 1, "method": method, "params": params}));
            assert_eq!(r[0]["error"]["code"], code, "{method} {params}: {}", r[0]);
        };
        for method in ["globe/fmuInfo", "globe/fmuRun"] {
            bad(&mut s, method, json!({}), -32602);
            bad(&mut s, method, json!({"path": 1}), -32602);
        }
        let path = json!("/nonexistent/a.fmu");
        bad(&mut s, "globe/fmuInfo", json!({"path": path}), -32000);
        bad(
            &mut s,
            "globe/fmuRun",
            json!({"path": path, "request": {"nope": 1}}),
            -32602,
        );
        bad(
            &mut s,
            "globe/fmuRun",
            json!({"path": path, "request": {"kind": "xx"}}),
            -32602,
        );
    }

    /// S166: クラス名の参照・リネーム・prepareRename。ほかの開いた文書に参照があるとリネームは断り、参照の一覧には含める。
    #[test]
    fn class_references_rename_and_prepare_over_rpc() {
        let mut s = Server::new(&[]).unwrap();
        let (a, b) = ("file:///tmp/A.mo", "file:///tmp/B.mo");
        let ta = "model Base
  Real x;
end Base;
model A
  extends Base;
  Base b;
end A;
";
        let tb = "model B
  Base c;
end B;
";
        for (u, t) in [(a, ta), (b, tb)] {
            s.handle(
                json!({"method": "textDocument/didOpen", "params": {"textDocument": {
                "uri": u, "languageId": "modelica", "version": 1, "text": t}}}),
            );
        }
        let mut ask = |m: &str, line: u32, ch: u32, new: &str| {
            s.handle(json!({"id": 3, "method": m, "params": {"file": a, "textDocument": {"uri": a},
                "position": {"line": line, "character": ch}, "line": line, "character": ch, "newName": new}}))
                .remove(0)
        };
        // Base の宣言の上: textDocument/references は A の 3 箇所 + B の 1 箇所
        let r = ask("textDocument/references", 4, 11, "");
        let uris: Vec<_> = r["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["uri"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(uris.iter().filter(|u| *u == a).count(), 4, "{r}"); // 宣言・end・extends・型
        assert_eq!(uris.iter().filter(|u| *u == b).count(), 1, "{r}");
        assert_eq!(
            ask("globe/references", 4, 11, "")["result"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        let p = ask("textDocument/prepareRename", 4, 11, "");
        assert_eq!(
            p["result"]["start"],
            json!({"line": 4, "character": 10}),
            "{p}"
        );
        // B が Base を使っているので断る
        let e = ask("globe/rename", 4, 11, "Root");
        assert_eq!(e["error"]["code"], -32803, "{e}");
        assert!(
            e["error"]["message"]
                .as_str()
                .unwrap()
                .contains("ほかの文書に 1 箇所"),
            "{e}"
        );
        // B を閉じる代わりに本文を差し替える（Base を使わない）と通る
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": b, "languageId": "modelica", "version": 2, "text": "model B\nend B;\n"}}}),
        );
        let mut ask = |m: &str, line: u32, ch: u32, new: &str| {
            s.handle(json!({"id": 3, "method": m, "params": {"file": a, "textDocument": {"uri": a},
                "position": {"line": line, "character": ch}, "line": line, "character": ch, "newName": new}}))
                .remove(0)
        };
        assert_eq!(
            ask("globe/rename", 4, 11, "Root")["result"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        // 部品でもクラスでもない所（予約語 model）は prepareRename が断る
        let e = ask("textDocument/prepareRename", 3, 2, "");
        assert_eq!(e["error"]["code"], -32803, "{e}");
    }

    /// S214: 打鍵ごとの didChange が積まれたら、古い版は検査せずに捨て、最後の診断は最新の版のものになる。
    #[test]
    fn queued_did_changes_are_coalesced() {
        fn frame(m: Value) -> Vec<u8> {
            let b = m.to_string();
            format!("Content-Length: {}\r\n\r\n{b}", b.len()).into_bytes()
        }
        let uri = "file:///tmp/Typing.mo";
        let mut input =
            frame(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
        input.extend(frame(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "text": "model Typing\nend Typing;\n"}}}),
        ));
        // n 回目の版は n 行目に構文の誤り（診断の行で版が分かる）
        const N: usize = 50;
        for n in 1..=N {
            let text = format!("model Typing\n{}x y z\nend Typing;\n", "\n".repeat(n));
            input.extend(frame(
                json!({"jsonrpc": "2.0", "method": "textDocument/didChange",
                "params": {"textDocument": {"uri": uri}, "contentChanges": [{"text": text}]}}),
            ));
        }
        input.extend(frame(json!({"jsonrpc": "2.0", "method": "exit"})));
        let mut out = Vec::new();
        serve(Server::new(&[]).unwrap(), io::Cursor::new(input), &mut out).unwrap();
        let mut rd = io::Cursor::new(out);
        let mut published = Vec::new();
        while let Some(m) = read_message(&mut rd).unwrap() {
            if m["method"] == "textDocument/publishDiagnostics" {
                published.push(m);
            }
        }
        let last = published.last().unwrap();
        assert_eq!(
            last["params"]["diagnostics"][0]["range"]["start"]["line"],
            N + 1,
            "{last}"
        );
        // didOpen の 1 回 + まとめた分。まとめなければ 1 + 50
        assert!(published.len() < N / 2, "{} 回検査した", published.len());
    }

    #[test]
    fn simulate_gets_a_default_wall_budget() {
        let mut r = SimulateRequest::default();
        with_default_budget(&mut r);
        assert_eq!(r.settings.max_wall_seconds, Some(DEFAULT_MAX_WALL_SECONDS));
        r.settings.max_wall_seconds = Some(5.0);
        with_default_budget(&mut r);
        assert_eq!(r.settings.max_wall_seconds, Some(5.0));
    }

    /// S136（ADR-0033）: 読みを別のスレッドにした `serve` を pipe で回す。長い実行の進捗が `$/progress` で届き、
    /// `$/cancelRequest` で止めると、普通の応答に GM-I11004 と途中までの列が載る。
    #[test]
    fn cancel_request_stops_a_long_run_with_partial_columns() {
        const WIGGLE: &str = "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;";
        fn send(w: &mut impl Write, m: Value) {
            let b = m.to_string();
            write!(w, "Content-Length: {}\r\n\r\n{b}", b.len()).unwrap();
        }
        let (in_r, mut in_w) = io::pipe().unwrap();
        let (out_r, out_w) = io::pipe().unwrap();
        let server = Server::new(&[]).unwrap();
        let t = thread::spawn(move || serve(server, io::BufReader::new(in_r), out_w));
        let mut out = io::BufReader::new(out_r);
        send(
            &mut in_w,
            json!({"jsonrpc": "2.0", "id": 7, "method": "globe/simulateColumns", "params": {
                "workDoneToken": "run", "request": {"model": "Wiggle", "files": {"Wiggle.mo": WIGGLE},
                "settings": {"solver": "esdirk34", "stop_time": 100000.0, "interval": 1.0}}}}),
        );
        let mut kinds = Vec::new();
        loop {
            let m = read_message(&mut out).unwrap().unwrap();
            assert_eq!(m["method"], "$/progress", "{m}");
            kinds.push(m["params"]["value"]["kind"].clone());
            // 列は出力の点（間隔 1）までなので、t > 1 の report を待ってから止める（それより前だと列は t = 0 の 1 点だけ）
            if m["params"]["value"]["time"]
                .as_f64()
                .is_some_and(|t| t > 1.0)
            {
                assert!(m["params"]["value"]["percentage"].is_number(), "{m}");
                break;
            }
        }
        assert_eq!(kinds[0], "begin");
        send(
            &mut in_w,
            json!({"jsonrpc": "2.0", "method": "$/cancelRequest", "params": {"id": 7}}),
        );
        let r = loop {
            let m = read_message(&mut out).unwrap().unwrap();
            if m["id"] == 7 {
                break m;
            }
            kinds.push(m["params"]["value"]["kind"].clone());
        };
        assert_eq!(kinds.last().unwrap(), "end");
        let res = &r["result"];
        assert_eq!(
            res["response"]["termination"]["kind"], "cancelled",
            "{}",
            res["response"]
        );
        let codes: Vec<_> = res["response"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["code"].clone())
            .collect();
        assert!(codes.contains(&json!("GM-I11004")), "{codes:?}");
        let time = res["columns"][0].as_array().unwrap();
        let last = time.last().unwrap().as_f64().unwrap();
        assert!((1.0..100000.0).contains(&last), "{last}");
        // exit で読みのスレッドが抜け、serve が返る
        send(&mut in_w, json!({"jsonrpc": "2.0", "method": "exit"}));
        t.join().unwrap().unwrap();
    }

    #[test]
    fn frames_are_read_by_content_length() {
        let body = r#"{"id":1,"method":"x"}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut input = raw.as_bytes();
        assert_eq!(read_message(&mut input).unwrap().unwrap()["method"], "x");
        assert!(read_message(&mut input).unwrap().is_none());
    }
}

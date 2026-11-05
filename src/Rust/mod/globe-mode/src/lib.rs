//! GlobeMode の公開 API（facade、S035）。UI・LSP・wasm・各言語の束縛は、この crate と
//! [`globe_mode_protocol`] だけに依存する。
//!
//! 中身は `globe-mode-db` の [`GlobeDb`] と `globe-mode-exec` を呼ぶだけで、新しい計算は入れない。
//! `globe simulate` と wasm の `Workspace::simulate` が S034 まで別々に持っていた「検査 → コンパイル → 実行」の
//! 流れを [`Workspace::simulate_with`] に 1 本にまとめた。
//!
//! ```no_run
//! use globe_mode::{SimulateRequest, Workspace};
//!
//! let mut ws = Workspace::new();
//! ws.set_file("Decay.mo", "model Decay Real x(start = 1); equation der(x) = -x; end Decay;");
//! let check = ws.check("Decay");
//! assert!(check.passed);
//! let res = ws.simulate(&SimulateRequest { model: "Decay".into(), ..Default::default() });
//! assert!(res.ok);
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode_db::GlobeDb;
use globe_mode_frontend::Source;
use globe_mode_ir::UnknownKind;

pub use globe_mode_core::{Diagnostic, Stage};
pub use globe_mode_frontend::CompiledModel;
pub use globe_mode_graph::{ConnectGraph, ConnectMismatch, GraphError};
pub use globe_mode_ir::{FlatSummary, Outline, SimRun};
pub use globe_mode_protocol::{
    CheckResponse, ColumnsLayout, ExperimentValues, ParametersResponse, SimulateRequest,
    SimulateResponse, StructureCounts, force_helps, unsupported_summary,
};
pub use globe_mode_sim::solvers::{SOLVERS, SolverEntry};
// facade に無いものを呼ぶとき（CLI の文字の表示が `FullReport` を読む、結果を CSV / MAT に書く）
pub use globe_mode_db as db;
pub use globe_mode_exec as exec;
pub use globe_mode_exec::{CancelToken, ExecStage, Progress};
pub use globe_mode_sim as sim;

/// ソルバの登録表（`globe simulate --list-solvers` と wasm の `solvers()` の中身）。
pub fn solvers() -> &'static [SolverEntry] {
    SOLVERS
}

/// 本文を整形する（S111。wasm の `format` と同じ形）。文法が壊れていれば `Err`（理由）で、本文は変えない。
pub fn format(text: &str, indent: usize, insert_origin: bool) -> Result<String, String> {
    let options = globe_mode_fmt::Options {
        indent,
        insert_origin,
        ..globe_mode_fmt::Options::default()
    };
    globe_mode_fmt::format(text, &options).map_err(|e| e.to_string())
}

/// lint の結果（`[{rule, line, column, message}]` の JSON。S111。wasm の `lint` と同じ形）。
pub fn lint(text: &str) -> String {
    let msgs: Vec<serde_json::Value> =
        globe_mode_fmt::lint(text, &globe_mode_fmt::LintOptions::default())
            .into_iter()
            .map(|m| {
                serde_json::json!({"rule": m.rule, "line": m.line, "column": m.column, "message": m.message})
            })
            .collect();
    serde_json::Value::from(msgs).to_string()
}

/// FMU の書き出しの設定（S118。`globe fmu export` の `--fmi-version`・`--build`・`--target`・`--fmi-headers`）。
/// C ABI・NIF・Python は同じ形の JSON（`{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}`。省略可）で渡す。
#[cfg(feature = "fmu")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FmuOptions {
    /// FMI の版（2 か 3）
    pub fmi_version: u8,
    /// 共有ライブラリも作る（`targets` が空なら host を `cc` で、あれば `zig cc -target`）。`fmi_headers` が要る
    pub build: bool,
    pub targets: Vec<String>,
    /// `fmi2Functions.h`（3.0 は `fmi3Functions.h`）などのあるディレクトリ
    pub fmi_headers: Option<PathBuf>,
}

#[cfg(feature = "fmu")]
impl Default for FmuOptions {
    fn default() -> Self {
        Self {
            fmi_version: 2,
            build: false,
            targets: Vec::new(),
            fmi_headers: None,
        }
    }
}

/// DAE まで変換したモデルを `out_dir/<名前>/`（展開した FMU）と `out_dir/<名前>.fmu` に書き、`.fmu` のパスを返す（S118）。
#[cfg(feature = "fmu")]
pub fn export_fmu(
    model: &CompiledModel,
    out_dir: &Path,
    options: &FmuOptions,
) -> Result<PathBuf, String> {
    let headers = match (options.build, &options.fmi_headers) {
        (false, _) => None,
        (true, Some(h)) => Some(h.as_path()),
        (true, None) => return Err("build には fmi_headers が要る".into()),
    };
    let src = globe_mode_fmi::export::export_me_sources(model, options.fmi_version)?;
    globe_mode_fmi::export::write_fmu(&src, out_dir, headers, &options.targets)
}

/// 壁時計（ミリ秒、UNIX epoch から）。wasm32 では std の時計が無いので 0（wasm の束ねは JS の
/// `performance.now()` を [`Workspace::simulate_with`] に渡す）。
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1e3)
    }
}

/// [`Workspace::simulate_observed`] に渡す進捗の受け手と中断のトークン（ADR-0033）。
#[derive(Default)]
pub struct SimulateHandle<'a> {
    pub on_progress: Option<&'a mut dyn FnMut(&Progress)>,
    pub cancel: Option<CancelToken>,
}

impl std::fmt::Debug for SimulateHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimulateHandle")
            .field("on_progress", &self.on_progress.is_some())
            .field("cancel", &self.cancel)
            .finish()
    }
}

/// [`Workspace::simulate_with`] の結果。JSON にする部分（`response`）に、Rust からだけ使う結果（時系列と
/// コンパイル済みのモデル。CLI が MAT を書くために使う）を添えたもの。
#[derive(Debug)]
pub struct Simulated {
    pub response: SimulateResponse,
    /// 実行できたときの結果（時系列を含む）
    pub run: Option<SimRun>,
    /// DAE まで変換したモデル（コンパイルまで進んだときだけ）
    pub compiled: Option<CompiledModel>,
}

impl Simulated {
    /// 列の道（S116。wasm の `SimColumns`・Python の `simulate_columns`・C の `gm_simulate_columns`・NIF と共通）。
    /// 列の並び（`names[0]` は `time`）と、列ごとの値（`points` 個ずつ）。実行できなかったときは空の列と 0 点。
    pub fn columns(&self) -> (ColumnsLayout, Vec<&[f64]>) {
        let Some(run) = &self.run else {
            return (
                ColumnsLayout {
                    names: Vec::new(),
                    points: 0,
                },
                Vec::new(),
            );
        };
        let traj = &run.trajectory;
        let names = std::iter::once("time".to_string())
            .chain(traj.variables.iter().map(|v| v.name.clone()))
            .collect();
        let columns = std::iter::once(traj.time.as_slice())
            .chain(traj.variables.iter().map(|v| v.values.as_slice()))
            .collect();
        (
            ColumnsLayout {
                names,
                points: traj.time.len(),
            },
            columns,
        )
    }
}

/// ライブラリを 1 回だけ読み、作業中のファイルを出し入れしながら、検査・グラフ・実行を繰り返す。
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

impl Workspace {
    /// ライブラリ無しのワークスペース。
    pub fn new() -> Self {
        Self::from_db(GlobeDb::new(), 0, 0.0)
    }

    /// ライブラリのディレクトリ（MSL など）の `.mo` を本文のまま持ち、検査のたびにモデルが使うファイルだけを
    /// Rumoca の Session に読むワークスペース（S054。wasm の分割読み込みと同じ。[`Workspace::with_library_sources_split`]）。
    /// MSL の `ChuaCircuit` で最大 RSS を約 1.5 GiB から減らす。結果は [`Workspace::with_libraries_full`] と同じ。
    /// 読めなければ S1 の診断（`GM-E1003`）。ディレクトリでないパスが混じるときは全部読む道に戻る。
    pub fn with_libraries(paths: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        if paths.is_empty() || !paths.iter().all(|p| p.is_dir()) {
            return Self::with_libraries_full(paths);
        }
        let t = now_ms();
        let mut files = Vec::new();
        for dir in paths {
            collect_mo(dir, &mut files).map_err(|e| vec![library_error(dir, &e.to_string())])?;
        }
        files.sort();
        let mut texts = Vec::with_capacity(files.len());
        for f in &files {
            let text =
                std::fs::read_to_string(f).map_err(|e| vec![library_error(f, &e.to_string())])?;
            texts.push((f.display().to_string(), text));
        }
        let sources: Vec<Source<'_>> = texts
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let name = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let db = GlobeDb::with_library_sources_split(&name, &sources)?;
        Ok(Self::from_db(db, paths.len(), now_ms() - t))
    }

    /// [`Workspace::with_libraries`] と同じ入力で、ライブラリのファイルを全部 Session に読む（S054 より前の道。
    /// 比べるために残す。MSL で最大 RSS 約 1.5 GiB）。
    pub fn with_libraries_full(paths: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        let t = now_ms();
        let db = GlobeDb::with_libraries(paths)?;
        Ok(Self::from_db(db, paths.len(), now_ms() - t))
    }

    /// ライブラリをファイルの組（名前 → 本文）で読み込んだワークスペース（ファイルシステムの無い wasm32 用）。
    pub fn with_library_sources(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::from_sources(name, files, GlobeDb::with_library_sources)
    }

    /// [`Workspace::with_library_sources`] と同じ入力で、使うファイルだけを検査のときに読む（S037。メモリを減らす）。
    pub fn with_library_sources_split(
        name: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::from_sources(name, files, GlobeDb::with_library_sources_split)
    }

    fn from_sources(
        name: &str,
        files: &BTreeMap<String, String>,
        load: fn(&str, &[Source<'_>]) -> Result<GlobeDb, Vec<Diagnostic>>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let t = now_ms();
        let sources: Vec<Source<'_>> = files
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let db = load(name, &sources)?;
        Ok(Self::from_db(db, files.len(), now_ms() - t))
    }

    fn from_db(db: GlobeDb, library_files: usize, library_load_ms: f64) -> Self {
        Workspace {
            db,
            library_files,
            library_load_ms,
        }
    }

    /// 読み込んだライブラリのファイル（`with_libraries` ではディレクトリ）の数。
    pub fn library_files(&self) -> usize {
        self.library_files
    }

    /// ライブラリを読み込むのにかかった時間（ミリ秒。wasm32 では 0）。
    pub fn library_load_ms(&self) -> f64 {
        self.library_load_ms
    }

    /// ライブラリの読み込みの時間を外の時計で測り直したときに入れる（wasm の束ねが JS の `performance.now()` で）。
    pub fn set_library_load_ms(&mut self, ms: f64) {
        self.library_load_ms = ms;
    }

    /// ファイルを足すか、本文を差し替える。
    pub fn set_file(&mut self, path: &str, text: impl Into<String>) {
        self.db.set_file(path, text);
    }

    /// ファイルを除く。無ければ false。
    pub fn remove_file(&mut self, path: &str) -> bool {
        self.db.remove_file(path)
    }

    /// ディレクトリの下の `.mo` をすべて読む。読んだ数を返す。
    pub fn load_dir(&mut self, dir: &Path) -> std::io::Result<usize> {
        self.db.load_dir(dir)
    }

    /// ワークスペースのファイルのパス（区切りは `/`、昇順）。
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.db.paths()
    }

    /// ファイルのアウトライン（位置を含む）。ワークスペースに無ければ None、構文の誤りは S0 の診断。
    pub fn outline(&self, path: &str) -> Option<&Result<Outline, Vec<Diagnostic>>> {
        self.db.outline(path)
    }

    /// モデルの検査（S0〜S10）。診断の位置は元の本文の中。
    pub fn check(&mut self, model: &str) -> CheckResponse {
        let report = self.db.check_model(model);
        let check = report.check;
        CheckResponse {
            model: report.model,
            passed: !report.diagnostics.iter().any(Diagnostic::is_error),
            failed_stage: report
                .diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.stage)
                .min(),
            balance: check.balance,
            dae: check.dae,
            structure: check.structure.as_ref().map(|s| StructureCounts {
                unknowns: s.unknowns.len(),
                equations: s.equations.len(),
                derivatives: s
                    .unknowns
                    .iter()
                    .filter(|u| u.kind == UnknownKind::Derivative)
                    .count(),
            }),
            s6: report.structural,
            s7: report.index,
            s8: report.init,
            s9: report.loops,
            s10: report.preflight,
            experiment: check
                .preflight
                .as_ref()
                .and_then(|p| p.experiment.as_ref())
                .map(ExperimentValues::from),
            init: check.init.as_ref().map(|i| self.db.init_in_original(i)),
            flat: check.flat,
            unsupported: unsupported_summary(&report.diagnostics),
            can_force: force_helps(&report.diagnostics),
            diagnostics: report.diagnostics,
            notes: check.notes,
        }
    }

    /// `check` に渡すファイル（モデルが使うものの閉包。パスの昇順）。モデルが無ければ空。
    pub fn deps(&mut self, model: &str) -> Vec<String> {
        self.db.deps_closure(model)
    }

    /// flat モデルの要約。flatten まで通らなければ None。
    pub fn flat(&mut self, model: &str) -> Option<FlatSummary> {
        self.db.flat_summary(model)
    }

    /// connect グラフ（flatten しない）。`depth` は展開する深さ（1 は直下の部品だけ）。
    pub fn graph(&mut self, model: &str, depth: u32) -> Result<ConnectGraph, GraphError> {
        self.db.connect_graph(model, depth)
    }

    /// ファイルの connect のうち、両端のコネクタの型が合わないもの（S062）。ワークスペースのファイルの型だけを見る。
    pub fn connect_mismatches(&mut self, path: &str) -> Vec<ConnectMismatch> {
        let Some(Ok(outline)) = self.db.outline(path).cloned() else {
            return Vec::new();
        };
        globe_mode_graph::connect_mismatches(self.db.class_index(), &outline)
    }

    /// DAE まで変換したモデル（`overrides` はパラメータ名 → 値）。失敗の診断の位置は元の本文の中。
    pub fn compile(
        &mut self,
        model: &str,
        overrides: &BTreeMap<String, f64>,
    ) -> Result<CompiledModel, Vec<Diagnostic>> {
        self.db.compiled_model_with_overrides(model, overrides)
    }

    /// コンパイルして [`export_fmu`]（S118）。DAE まで変換できなければ診断の文を並べた `Err`。
    #[cfg(feature = "fmu")]
    pub fn export_fmu(
        &mut self,
        model: &str,
        out_dir: &Path,
        options: &FmuOptions,
    ) -> Result<PathBuf, String> {
        let compiled = self.compile_for_fmu(model)?;
        export_fmu(&compiled, out_dir, options)
    }

    /// [`Workspace::export_fmu`] の `build` 無しを、ファイルに書かず zip のバイト列で返す（S144。wasm32 でも呼べる。ブラウザの UI のダウンロード用）。
    #[cfg(feature = "fmu")]
    pub fn export_fmu_bytes(&mut self, model: &str, fmi_version: u8) -> Result<Vec<u8>, String> {
        let compiled = self.compile_for_fmu(model)?;
        let src = globe_mode_fmi::export::export_me_sources(&compiled, fmi_version)?;
        globe_mode_fmi::export::fmu_bytes(&src)
    }

    #[cfg(feature = "fmu")]
    fn compile_for_fmu(&mut self, model: &str) -> Result<CompiledModel, String> {
        self.compile(model, &BTreeMap::new()).map_err(|ds| {
            let msgs: Vec<String> = ds.iter().map(|d| d.message.clone()).collect();
            format!(
                "{model}: DAE まで変換できないので FMU を作れない: {}",
                msgs.join("; ")
            )
        })
    }

    /// 再コンパイル無しに変えられる（tunable な）パラメータの一覧（検査の Error では止めない）。
    /// DAE まで変換できなければ `Err`（その診断）。lowering に失敗したら `parameters` は None。
    pub fn tunable_parameters(
        &mut self,
        model: &str,
    ) -> Result<ParametersResponse, Vec<Diagnostic>> {
        let compiled = self.compile(model, &BTreeMap::new())?;
        let (parameters, diagnostics) = match globe_mode_exec::tunable_parameters(&compiled) {
            Ok(p) => (Some(p), Vec::new()),
            Err(d) => (None, d),
        };
        Ok(ParametersResponse {
            model: model.to_string(),
            parameters,
            diagnostics,
        })
    }

    /// 検査 → コンパイル → 実行。`request.files` は先にワークスペースに足す（残る）。
    pub fn simulate(&mut self, request: &SimulateRequest) -> SimulateResponse {
        self.simulate_with(request, &now_ms).response
    }

    /// [`Workspace::simulate`] と同じ。`now` は壁時計（ミリ秒。`timings_ms` に使う）で、結果には時系列と
    /// コンパイル済みのモデルも添える。
    pub fn simulate_with(&mut self, request: &SimulateRequest, now: &dyn Fn() -> f64) -> Simulated {
        self.simulate_observed(request, now, SimulateHandle::default())
    }

    /// [`Workspace::simulate_with`] と同じだが、実行系の進捗を `handle.on_progress` に知らせ、`handle.cancel` が
    /// 立てられたら次の区切りで止まる（ADR-0033。止めたら情報 GM-I11004。積分の途中なら `termination` が
    /// `cancelled` で途中までの列、それより前なら `ok: false`）。`now` は t の間引きにも使う。
    pub fn simulate_observed(
        &mut self,
        request: &SimulateRequest,
        now: &dyn Fn() -> f64,
        handle: SimulateHandle<'_>,
    ) -> Simulated {
        let mut s = self.simulate_unmarked(request, now, handle);
        s.response.unsupported = unsupported_summary(&s.response.diagnostics);
        // 検査で断った（強行していない）ときだけ「強行すれば進める」。強行した結果には付けない
        s.response.can_force =
            !request.force && s.response.refused.is_some() && force_helps(&s.response.diagnostics);
        s
    }

    fn simulate_unmarked(
        &mut self,
        request: &SimulateRequest,
        now: &dyn Fn() -> f64,
        handle: SimulateHandle<'_>,
    ) -> Simulated {
        let model = request.model.as_str();
        let mut response = SimulateResponse {
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
            unsupported: None,
            can_force: false,
        };
        let mut t = now();
        let mut lap = |name: &str, timings: &mut BTreeMap<String, f64>| {
            let n = now();
            timings.insert(name.to_string(), n - t);
            t = n;
        };
        for (name, text) in &request.files {
            self.db.set_file(name, text.clone());
        }
        lap("load", &mut response.timings_ms);
        let checked = self.db.check_model(model);
        lap("check", &mut response.timings_ms);
        response.diagnostics.extend(checked.diagnostics);
        if let Some(stage) = checked_failed_stage(&response.diagnostics)
            && !request.force
        {
            let hint = if force_helps(&response.diagnostics) {
                "force で強行できる"
            } else {
                "強行しても実行できない"
            };
            response.refused = Some(format!(
                "実行しない（{stage:?} {} で Error。{hint}）",
                stage.title()
            ));
            return Simulated {
                response,
                run: None,
                compiled: None,
            };
        }
        let compiled = match self
            .db
            .compiled_model_with_overrides(model, &request.settings.parameters)
        {
            Ok(c) => c,
            Err(ds) => {
                response.diagnostics.extend(ds);
                response.refused = Some("DAE まで変換できないので実行できない".to_string());
                return Simulated {
                    response,
                    run: None,
                    compiled: None,
                };
            }
        };
        lap("compile", &mut response.timings_ms);
        let mut quiet = |_: &Progress| {};
        let report = globe_mode_exec::run_observed(
            &compiled,
            &request.settings,
            &mut globe_mode_exec::Observer {
                on_progress: handle.on_progress.unwrap_or(&mut quiet),
                cancel: handle.cancel,
                now,
            },
        );
        lap("run", &mut response.timings_ms);
        response.ok = report.succeeded();
        response.diagnostics.extend(report.diagnostics);
        response.settings = report.settings;
        if let Some(run) = &report.run {
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
            response.termination = Some(run.termination.clone());
            response.stats = Some(run.stats.clone());
        } else {
            response.refused = Some("実行に失敗した".to_string());
        }
        Simulated {
            response,
            run: report.run,
            compiled: Some(compiled),
        }
    }

    /// 検査器（salsa の DB）。facade に無いクエリを呼ぶとき。
    pub fn db(&mut self) -> &mut GlobeDb {
        &mut self.db
    }
}

/// Error が出た最初の段階。
fn checked_failed_stage(diagnostics: &[Diagnostic]) -> Option<Stage> {
    diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.stage)
        .min()
}

/// ディレクトリの下の `.mo` を集める。
fn collect_mo(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_mo(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "mo") {
            out.push(path);
        }
    }
    Ok(())
}

fn library_error(path: &Path, why: &str) -> Diagnostic {
    Diagnostic::error(
        globe_mode_core::codes::LIBRARY,
        Stage::S1,
        format!("ライブラリ {} を読み込めない", path.display()),
    )
    .note(why)
}

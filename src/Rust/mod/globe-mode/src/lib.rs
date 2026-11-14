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
pub use globe_mode_frontend::{CompiledModel, collect_mo_files};
pub use globe_mode_graph::{ConnectGraph, ConnectMismatch, GraphError};
pub use globe_mode_ir::{FlatSummary, Outline, SimRun, SimSettings};
pub use globe_mode_protocol::{
    CheckResponse, ColumnsLayout, ExperimentValues, ModelParametersResponse, ParametersResponse,
    SimulateRequest, SimulateResponse, StructureCounts, force_helps, refused_summary,
    unsupported_summary,
};
pub use globe_mode_sim::solvers::{SOLVERS, SolverEntry};
// facade に無いものを呼ぶとき（CLI の文字の表示が `FullReport` を読む、結果を CSV / MAT に書く）
#[cfg(feature = "fmu-import")]
mod fmu_import;
#[cfg(feature = "fmu-import")]
pub use fmu_import::{
    CosimSpec, FMU_TRUST_WARNING, FmuExperiment, FmuInfo, FmuInterface, FmuKind, FmuRun,
    FmuRunRequest, FmuVariable, MeSummary, Ssp, import_fmu_info, run_cosim, run_fmu,
};

pub use globe_mode_db as db;
pub use globe_mode_exec as exec;
pub use globe_mode_exec::{CancelToken, ExecStage, Linearization, Progress};
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

#[cfg(feature = "fmu-p5b")]
mod fmu_p5b;
#[cfg(feature = "fmu-p5b")]
pub use fmu_p5b::{P5bError, export_fmu_p5b};

/// FMU の実行時（S239。`globe fmu export --runtime`）。JSON では `"p5a"` / `"p5b"`。
#[cfg(feature = "fmu")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FmuRuntime {
    /// Rumoca のテンプレートが描く C のソースの FMU（既定。`build` で共有ライブラリも作る）
    #[default]
    P5a,
    /// `globe-mode-fmu`（自前の実行時）の共有ライブラリを入れた FMU。イベント・MSL のモデルも載る（feature `fmu-p5b`）
    P5b,
}

/// FMU のインターフェース（S251。`globe fmu export --kind`）。JSON では `"me_cs"` / `"se"`。
#[cfg(feature = "fmu")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FmuExportKind {
    /// Model Exchange と Co-Simulation（既定）
    #[default]
    MeCs,
    /// FMI 3.0 の Scheduled Execution（周期クロック 1 つ。`runtime` は p5b）
    Se,
}

/// FMU の書き出しの設定（S118。`globe fmu export` の `--fmi-version`・`--build`・`--target`・`--fmi-headers`・`--runtime`・`--runtime-lib`）。
/// C ABI・NIF・Python は同じ形の JSON（`{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null, "runtime": "p5a", "runtime_lib": null}`。省略可）で渡す。
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
    /// 実行時（S239）。`P5b` は `build`・`targets`・`fmi_headers` と一緒に使えない
    pub runtime: FmuRuntime,
    /// `P5b` の共有ライブラリ（`libglobe_mode_fmu_cdylib.so` など）。省略すると `cargo build --release -p globe-mode-fmu-cdylib` で作る（S239）
    pub runtime_lib: Option<PathBuf>,
    /// インターフェース（S251）。`Se` は `fmi_version` 3 と `runtime` p5b だけ
    pub kind: FmuExportKind,
}

#[cfg(feature = "fmu")]
impl FmuOptions {
    /// JSON（省略した鍵は既定。`None` は全部既定）から読む（S208。C の ABI・NIF・Python が共有する）。
    pub fn from_json(options_json: Option<&str>) -> Result<Self, String> {
        options_json.map_or_else(
            || Ok(Self::default()),
            |s| serde_json::from_str(s).map_err(|e| format!("options が読めない: {e}")),
        )
    }
}

#[cfg(feature = "fmu")]
impl Default for FmuOptions {
    fn default() -> Self {
        Self {
            fmi_version: 2,
            build: false,
            targets: Vec::new(),
            fmi_headers: None,
            runtime: FmuRuntime::P5a,
            runtime_lib: None,
            kind: FmuExportKind::MeCs,
        }
    }
}

/// DAE まで変換したモデルを `out_dir/<名前>/`（展開した FMU）と `out_dir/<名前>-fmi<版>.fmu`（S189。版は 2 か 3）に書き、`.fmu` のパスを返す（S118）。
#[cfg(feature = "fmu")]
pub fn export_fmu(
    model: &CompiledModel,
    out_dir: &Path,
    options: &FmuOptions,
) -> Result<PathBuf, String> {
    if options.runtime != FmuRuntime::P5a || options.runtime_lib.is_some() {
        return Err("runtime p5b・runtime_lib はモデルの本文が要る（Workspace::export_fmu か export_fmu_p5b から）".into());
    }
    if options.kind == FmuExportKind::Se {
        return Err("kind se は runtime p5b だけ".into());
    }
    let headers = match (options.build, &options.fmi_headers) {
        (false, _) => None,
        (true, Some(h)) => Some(h.as_path()),
        (true, None) => return Err("build には fmi_headers が要る".into()),
    };
    let src = globe_mode_fmi::export::export_me_sources(model, options.fmi_version)?;
    globe_mode_fmi::export::write_fmu(&src, out_dir, headers, &options.targets)
}

/// 値を JSON の文字列にする（S208。C の ABI・NIF・Python・wasm が共有する。誤りの文面は `JSON: …`）。
pub fn to_json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON: {e}"))
}

/// `settings_json`（`{"stop_time": 2, "solver": "bdf"}` など。`None` は設定なし）から実行の依頼を作る（S202。C の ABI・NIF・Python が共有する）。
pub fn simulate_request(
    model: &str,
    settings_json: Option<&str>,
) -> Result<SimulateRequest, String> {
    let settings: serde_json::Value = match settings_json {
        Some(s) => {
            serde_json::from_str(s).map_err(|e| format!("settings の JSON が読めない: {e}"))?
        }
        None => serde_json::json!({}),
    };
    serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
        .map_err(|e| format!("settings が読めない: {e}"))
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
    /// プロットに既定で描く信号（`time` を除く。S190。[`globe_mode_ir::Trajectory::default_plot`]。`compiled` を放しても残る）
    pub default_plot: Vec<String>,
}

impl Simulated {
    /// 列の道の見出しの JSON（S202。C の ABI・NIF が共有する）。
    /// `{"response": SimulateResponse（csv 無し）, "names": [...], "points": n, "defaults": [...]}`。
    pub fn layout_json(&self, layout: &ColumnsLayout) -> Result<String, String> {
        to_json(&serde_json::json!({
            "response": self.response,
            "names": layout.names,
            "points": layout.points,
            "defaults": layout.defaults,
        }))
    }

    /// 列の道（S116。wasm の `SimColumns`・Python の `simulate_columns`・C の `gm_simulate_columns`・NIF と共通）。
    /// 列の並び（`names[0]` は `time`）と、列ごとの値（`points` 個ずつ）。実行できなかったときは空の列と 0 点。
    pub fn columns(&self) -> (ColumnsLayout, Vec<&[f64]>) {
        match &self.run {
            Some(run) => columns_of(&run.trajectory, self.default_plot.clone()),
            None => (
                ColumnsLayout {
                    names: Vec::new(),
                    points: 0,
                    defaults: Vec::new(),
                },
                Vec::new(),
            ),
        }
    }
}

fn columns_of(
    traj: &globe_mode_ir::Trajectory,
    defaults: Vec<String>,
) -> (ColumnsLayout, Vec<&[f64]>) {
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
            defaults,
        },
        columns,
    )
}

/// 開いた結果のファイル（`.mat` か `.csv`）の時系列。UI の「結果を開く」（S222）が実行の結果と同じ列の道で描く。
#[derive(Debug)]
pub struct OpenedResults(
    globe_mode_ir::Trajectory,
    Vec<globe_mode_ir::ResultParameter>,
);

impl OpenedResults {
    /// [`Simulated::columns`] と同じ形（既定で描く信号は先頭の 3 本。読んだ結果に役割は無い）。
    pub fn columns(&self) -> (ColumnsLayout, Vec<&[f64]>) {
        columns_of(&self.0, self.0.default_plot(&[]))
    }

    /// MAT の `data_1` のパラメータ（S236。CSV と FMU の結果は空）。
    pub fn parameters(&self) -> &[globe_mode_ir::ResultParameter] {
        &self.1
    }
}

/// 結果のファイルの中身を `name` の拡張子で読む（[`sim::mat::read_results_bytes`]）。読めなければ理由。
pub fn open_results(name: &str, bytes: &[u8]) -> Result<OpenedResults, String> {
    sim::mat::read_results_bytes(name, bytes)
        .map(|m| OpenedResults(m.trajectory, m.parameters))
        .map_err(|e| format!("{name}: {e}"))
}

/// ライブラリを 1 回だけ読み、作業中のファイルを出し入れしながら、検査・グラフ・実行を繰り返す。
pub struct Workspace {
    db: GlobeDb,
    library_files: usize,
    library_load_ms: f64,
    /// `with_libraries` に渡したディレクトリ（S239。P5b の FMU の `resources/lib` に使うファイルを選ぶ）
    library_paths: Vec<PathBuf>,
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

    /// `libraries` が空ならライブラリ無し（[`Workspace::new`]）、あれば [`Workspace::with_libraries`]（S208。C の ABI・NIF・Python が共有する）。
    pub fn open(libraries: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        if libraries.is_empty() {
            Ok(Self::new())
        } else {
            Self::with_libraries(libraries)
        }
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
            globe_mode_frontend::collect_mo_files(dir, &mut files)
                .map_err(|e| vec![library_error(dir, &e.to_string())])?;
        }
        files.sort();
        let mut texts = Vec::with_capacity(files.len());
        for f in &files {
            // UTF-8 でないファイル（Latin-1 の `°` など）が 1 つあるだけで、無関係なモデルまで読めなくならないよう、
            // 悪いバイトは U+FFFD にして読む（#85）。コメントと文字列の中の字が変わるだけ
            let bytes = std::fs::read(f).map_err(|e| vec![library_error(f, &e.to_string())])?;
            texts.push((
                f.display().to_string(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
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
        Ok(Self::from_db(db, paths.len(), now_ms() - t).with_library_paths(paths))
    }

    /// [`Workspace::with_libraries`] と同じ入力で、ライブラリのファイルを全部 Session に読む（S054 より前の道。
    /// 比べるために残す。MSL で最大 RSS 約 1.5 GiB）。
    pub fn with_libraries_full(paths: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        let t = now_ms();
        let db = GlobeDb::with_libraries(paths)?;
        Ok(Self::from_db(db, paths.len(), now_ms() - t).with_library_paths(paths))
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
            library_paths: Vec::new(),
        }
    }

    fn with_library_paths(mut self, paths: &[PathBuf]) -> Self {
        self.library_paths = paths.to_vec();
        self
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

    /// [`Workspace::outline`] の JSON（`globe outline --json` と同じ。S202。C の ABI・NIF・Python・wasm が共有する）。
    /// ファイルが無いか構文の誤りがあれば誤りの文（誤りは最初の診断の `コード: 文`）。
    pub fn outline_json(&self, path: &str) -> Result<String, String> {
        match self.outline(path) {
            None => Err(format!("ファイル {path} がワークスペースに無い")),
            Some(Ok(o)) => to_json(o),
            Some(Err(ds)) => Err(ds
                .first()
                .map_or_else(String::new, |d| format!("{}: {}", d.code, d.message))),
        }
    }

    /// モデルの検査（S0〜S10）。診断の位置は元の本文の中。
    pub fn check(&mut self, model: &str) -> CheckResponse {
        let report = self.db.check_model(model);
        let (passed, failed_stage) = (report.passed(), report.failed_stage());
        let check = report.check;
        let mut response = CheckResponse {
            model: report.model,
            passed,
            failed_stage,
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
            summary: String::new(),
        };
        response.summary = response.summary_line();
        response
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
    /// `options.runtime` が `P5b` なら [`export_fmu_p5b`]（S239。feature `fmu-p5b`。モデルを定義するワークスペースのファイル
    /// 〔無ければライブラリの中のクラス〕と、開いたライブラリから作る。ワークスペースのファイルが複数に跨るモデルは断る）。
    #[cfg(feature = "fmu")]
    pub fn export_fmu(
        &mut self,
        model: &str,
        out_dir: &Path,
        options: &FmuOptions,
    ) -> Result<PathBuf, String> {
        if options.runtime == FmuRuntime::P5b {
            return self.export_fmu_p5b(model, out_dir, options);
        }
        let compiled = self.compile_for_fmu(model)?;
        export_fmu(&compiled, out_dir, options)
    }

    #[cfg(feature = "fmu")]
    fn export_fmu_p5b(
        &mut self,
        model: &str,
        out_dir: &Path,
        options: &FmuOptions,
    ) -> Result<PathBuf, String> {
        #[cfg(not(feature = "fmu-p5b"))]
        {
            let _ = (model, out_dir, options);
            Err("この版には P5b（runtime = p5b）が入っていない（Desktop・CLI・Python の版で使える）".into())
        }
        #[cfg(feature = "fmu-p5b")]
        {
            let files = self.db.deps_closure(model);
            let text = match files.as_slice() {
                [] => None,
                [f] => self.db.text(f),
                _ => {
                    return Err(format!(
                        "{model}: runtime p5b のモデルは 1 つのファイルか、ライブラリの中のクラス（{} 個のファイルに跨る）",
                        files.len()
                    ));
                }
            };
            export_fmu_p5b(
                model,
                text.as_deref(),
                &[],
                &self.library_paths,
                out_dir,
                options,
            )
            .map_err(Into::into)
        }
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

    /// モデルのパラメータ宣言の一覧（`comp.k` の形の名前。S248）。DAE まで変換できなければ `Err`（その診断）。
    /// 既定値が式で数が出なかった Real/Integer は、実行系の tunable な値（lowering の結果）があれば補う（無くても `Ok`）。
    pub fn parameters(&mut self, model: &str) -> Result<ModelParametersResponse, Vec<Diagnostic>> {
        let compiled = self.compile(model, &BTreeMap::new())?;
        let mut parameters = compiled.parameters().to_vec();
        if parameters
            .iter()
            .any(|p| p.value.is_none() && p.dims.is_empty() && p.ty != "String")
            && let Ok(t) = globe_mode_exec::tunable_parameters(&compiled)
        {
            for p in parameters.iter_mut().filter(|p| p.value.is_none()) {
                p.value = t.iter().find(|t| t.name == p.name).map(|t| t.default);
            }
        }
        Ok(ModelParametersResponse {
            model: model.to_string(),
            parameters,
        })
    }

    /// モデルの線形化（S254）。入力（`input` の Real）・出力（`output` の Real）・状態の行列 A・B・C・D を数値微分で返す。
    /// `time` が None か実験の開始時刻なら初期化の直後の点、それより後なら開始時刻から `time` まで実行した点
    /// （入力を外から駆動するモデルは実行できないので、後の時刻は使えない）。DAE まで変換できなければ `Err`（その診断）。
    pub fn linearize(
        &mut self,
        model: &str,
        overrides: &BTreeMap<String, f64>,
        time: Option<f64>,
    ) -> Result<Linearization, Vec<Diagnostic>> {
        let compiled = self.compile(model, overrides)?;
        let start = compiled
            .experiment
            .as_ref()
            .and_then(|e| e.start_time)
            .map_or(0.0, |t| t.get());
        let fail = |message: String| {
            vec![Diagnostic::error(
                globe_mode_core::codes::RUN_INTEGRATION_FAILED,
                Stage::Run,
                format!("線形化に失敗: {message}"),
            )]
        };
        let mut inst = exec::ModelInstance::new(&compiled, overrides, start)?;
        if let Some(t) = time.filter(|t| *t != start) {
            if t < start || !t.is_finite() {
                return Err(fail(format!(
                    "時刻 {t} は開始時刻 {start} より前（か有限でない）"
                )));
            }
            let report = exec::run(
                &compiled,
                &SimSettings {
                    start_time: Some(start),
                    stop_time: Some(t),
                    parameters: overrides.clone(),
                    ..SimSettings::default()
                },
            );
            let ok = report.succeeded();
            let Some(run) = report.run.filter(|_| ok) else {
                return Err(report.diagnostics);
            };
            let x: Option<Vec<f64>> = inst
                .state_names()
                .iter()
                .map(|n| {
                    let v = run.trajectory.variables.iter().find(|v| &v.name == n)?;
                    v.values.last().copied()
                })
                .collect();
            let x = x.ok_or_else(|| fail("結果の列に状態が見つからない".to_string()))?;
            inst.set_states(&x).map_err(fail)?;
            inst.set_time(t);
        }
        inst.linearize().map_err(fail)
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
        // 強行のコンパイルの失敗は、検査と同じ診断を（注が少し違う形で）もう一度返す。コード・位置・文面が同じものは最初の 1 つだけ残す
        let mut unique: Vec<Diagnostic> = Vec::new();
        for d in std::mem::take(&mut s.response.diagnostics) {
            if !unique.iter().any(|x| {
                (x.code, &x.file, x.range, &x.message) == (d.code, &d.file, d.range, &d.message)
            }) {
                unique.push(d);
            }
        }
        s.response.diagnostics = unique;
        s.response.summary = s
            .response
            .refused
            .as_ref()
            .map(|r| refused_summary(&s.response.model, r, &s.response.diagnostics));
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
            summary: None,
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
        let failed_stage = checked.failed_stage();
        response.diagnostics.extend(checked.diagnostics);
        if let Some(stage) = failed_stage
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
                default_plot: Vec::new(),
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
                    default_plot: Vec::new(),
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
        response.diagnostics.extend(report.diagnostics);
        // `ok` は「結果があり、Error の診断が無い」。強行（force）で走っても、検査の Error が残れば ok ではない（#2）
        response.ok =
            report.run.is_some() && !response.diagnostics.iter().any(Diagnostic::is_error);
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
                match csv_text(traj) {
                    Ok(csv) => response.csv = Some(csv),
                    // 黙って捨てると、呼び手が結果も理由も受け取れない（#13）
                    Err(e) => {
                        response.ok = false;
                        response.refused = Some(format!("結果を CSV にできない: {e}"));
                    }
                }
            }
            response.termination = Some(run.termination.clone());
            response.stats = Some(run.stats.clone());
        } else {
            response.refused = Some("実行に失敗した".to_string());
        }
        let default_plot = report.run.as_ref().map_or_else(Vec::new, |r| {
            r.trajectory.default_plot(&compiled.state_names())
        });
        Simulated {
            response,
            run: report.run,
            compiled: Some(compiled),
            default_plot,
        }
    }

    /// 検査器（salsa の DB）。facade に無いクエリを呼ぶとき。
    pub fn db(&mut self) -> &mut GlobeDb {
        &mut self.db
    }
}

/// 時系列を CSV の文字列にする。変数の長さが `time` と違うなどで書けなければ、その理由。
fn csv_text(traj: &globe_mode_ir::Trajectory) -> Result<String, String> {
    let mut buf = Vec::new();
    globe_mode_sim::store::write_csv(traj, &mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

fn library_error(path: &Path, why: &str) -> Diagnostic {
    Diagnostic::error(
        globe_mode_core::codes::LIBRARY,
        Stage::S1,
        format!("ライブラリ {} を読み込めない", path.display()),
    )
    .note(why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_ir::{Trajectory, TrajectoryVariable, VariableRole};

    /// #13: 変数の長さが時刻と違う時系列は、CSV にできない理由を返す（黙って捨てない）
    #[test]
    fn csv_text_reports_why_it_cannot_write() {
        let variable = |values: Vec<f64>| TrajectoryVariable {
            name: "x".to_string(),
            role: VariableRole::State,
            unit: None,
            description: None,
            values,
        };
        let mut traj = Trajectory {
            time: vec![0.0, 1.0],
            variables: vec![variable(vec![1.0, 2.0])],
        };
        assert_eq!(csv_text(&traj).unwrap().lines().count(), 3);
        traj.variables = vec![variable(vec![1.0])];
        let why = csv_text(&traj).unwrap_err();
        assert!(
            why.contains("変数 x の値の数（1）が時刻の数（2）と違う"),
            "{why}"
        );
    }
}

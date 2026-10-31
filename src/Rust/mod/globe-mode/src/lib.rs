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
pub use globe_mode_graph::{ConnectGraph, GraphError};
pub use globe_mode_ir::{FlatSummary, Outline, SimRun};
pub use globe_mode_protocol::{
    CheckResponse, ColumnsLayout, ParametersResponse, SimulateRequest, SimulateResponse,
    StructureCounts,
};
pub use globe_mode_sim::solvers::{SOLVERS, SolverEntry};
// facade に無いものを呼ぶとき（CLI の文字の表示が `FullReport` を読む、結果を CSV / MAT に書く）
pub use globe_mode_db as db;
pub use globe_mode_exec as exec;
pub use globe_mode_sim as sim;

/// ソルバの登録表（`globe simulate --list-solvers` と wasm の `solvers()` の中身）。
pub fn solvers() -> &'static [SolverEntry] {
    SOLVERS
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

    /// ライブラリのディレクトリ（MSL など）を一度だけ読み込んだワークスペース。読めなければ S1 の診断（`GM-E1003`）。
    pub fn with_libraries(paths: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
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
            init: check.init.as_ref().map(|i| self.db.init_in_original(i)),
            flat: check.flat,
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

    /// DAE まで変換したモデル（`overrides` はパラメータ名 → 値）。失敗の診断の位置は元の本文の中。
    pub fn compile(
        &mut self,
        model: &str,
        overrides: &BTreeMap<String, f64>,
    ) -> Result<CompiledModel, Vec<Diagnostic>> {
        self.db.compiled_model_with_overrides(model, overrides)
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
            response.refused = Some(format!(
                "実行しない（{stage:?} {} で Error。force で強行できる）",
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
        let report = globe_mode_exec::run(&compiled, &request.settings);
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

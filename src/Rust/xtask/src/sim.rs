//! `cargo xtask msl-sim`: `msl-funnel` で OK になった MSL の例題を実行し、MAP-LIB の参照結果と管で比べ、報告を書く（S011）。
//!
//! - 実行は別プロセスのワーカー（同じ実行ファイルの隠しサブコマンド `msl-sim-worker`）で回す。Rumoca の時間切れは
//!   協調型で、初期化と積分器の 1 歩の内側では止まらない。そこで親が `--kill-seconds` を超えたワーカーを kill して
//!   立て直す（Rumoca 自身の MSL ハーネスも、モデルごとのワーカーを見張りが止める）。ワーカーが落ちたときも立て直す。
//! - 親とワーカーは標準入出力の JSON の行で話す。親は [`Request`] を 1 本ずつ送り、ワーカーは用意ができたら
//!   [`WorkerMessage::Ready`]、1 本ごとに [`WorkerMessage::Done`] を返す。Rumoca などが標準出力に書いても
//!   混ざらないよう、ワーカーの行には [`MESSAGE_PREFIX`] を付ける（付いていない行は親が標準エラーへ流す）。
//! - ワーカーは検査器（MSL）を一度だけ読み、1 本ごとに `compile_retaining` → `globe_mode_exec::run`（壁時計の予算は
//!   `--wall-seconds`）→ 参照結果があれば `globe_mode_sim::compare`（`comparisonSignals.txt` の信号）で比べる。
//!   panic は `catch_unwind` で拾って続け、N 本ごとに `Checker::reset` で Session を小さく戻す。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use globe_mode_core::{Diagnostic, Stage, codes};
use globe_mode_frontend::Checker;
use globe_mode_ir::{ResolvedSettings, SimSettings, SimStats, Termination, Trajectory};
use globe_mode_sim::compare::{Comparison, TubeOptions, compare, parse_comparison_signals};
use globe_mode_sim::store::{read_csv, write_csv};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::funnel::{gigabytes, make_checker, one_line, panic_message, parse_vm_hwm, peak_rss_kb};
use crate::reference::{self, Local, MAP_LIB_BRANCH, MAP_LIB_REPO, MAP_LIB_SHA};
use crate::{MSL_SHA256, MSL_VERSION, RUMOCA_REV, RUMOCA_VERSION, Result};

/// ワーカーの行の印。この後ろが JSON（[`WorkerMessage`]）。
pub(crate) const MESSAGE_PREFIX: &str = "@globe-mode-sim ";

/// ワーカーの用意（MSL の読み込み）を待つ上限。
const SETUP_TIMEOUT: Duration = Duration::from_secs(900);

/// P3 の完了条件（doc/roadmap.md）: 実行できる例題の数と、そのうち参照結果の管に収まる割合。
const P3_TARGET_RUNS: usize = 153;
const P3_TARGET_PASS_RATE: f64 = 0.9;

/// Rumoca の MSL ハーネスの基準値（`msl_quality_baseline.json`、rev 276e82bd に同梱）の `sim_ok` と `sim_attempted`。
const RUMOCA_BASELINE_SIM_OK: usize = 216;
const RUMOCA_BASELINE_SIM_ATTEMPTED: usize = 286;
const RUMOCA_BASELINE_SIM_TIMEOUT: f64 = 12.0;

// --------------------------------------------------------------------------- 対象

/// 実行する例題 1 本（`msl-funnel --json` の OK のもの）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub model: String,
    pub partial: bool,
}

/// `msl-funnel --json` の結果から、判定が OK の例題を（書かれた順に）取り出す。
pub(crate) fn funnel_ok_models(path: &Path) -> Result<Vec<Target>> {
    let text = fs::read_to_string(path).map_err(|e| {
        format!(
            "{} を読めない: {e}。先に `cargo xtask msl-funnel --json {}` を実行する",
            path.display(),
            path.display()
        )
    })?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{} の JSON が壊れている: {e}", path.display()))?;
    ok_models_in(&root).ok_or_else(|| format!("{} に models が無い", path.display()))
}

fn ok_models_in(root: &Value) -> Option<Vec<Target>> {
    let models = root.get("models")?.as_array()?;
    Some(
        models
            .iter()
            .filter(|m| m.get("verdict").and_then(Value::as_str) == Some("Ok"))
            .filter_map(|m| {
                Some(Target {
                    model: m.get("model")?.as_str()?.to_string(),
                    partial: m.get("partial").and_then(Value::as_bool).unwrap_or(false),
                })
            })
            .collect(),
    )
}

// --------------------------------------------------------------------------- 取り決め（親 ↔ ワーカー）

/// 親からワーカーへ（1 行 1 本）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Request {
    pub model: String,
}

/// ワーカーから親へ（1 行 1 つ、[`MESSAGE_PREFIX`] の後ろに JSON）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorkerMessage {
    /// 検査器を用意した（MSL を読んだ）
    Ready {
        setup_seconds: f64,
        rss_kb: Option<u64>,
    },
    /// 1 本の結果
    Done { record: Box<Record> },
}

/// 1 本の結果の分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    /// 終了時刻まで進んだ
    StopTime,
    /// `terminate()` で終了時刻の前に終わった（GM-I11001）
    Terminated,
    /// DAE まで変換できなかった（`compile_retaining` の診断。S0〜S5 と GM-U）
    CompileFailed,
    /// 設定かプリフライト（S10）で止まった
    PreflightFailed,
    /// 実行で止まった（GM-E11001〜GM-E11006、GM-U0004、GM-U0005）
    RunFailed,
    /// ワーカーの中で panic した
    Panic,
    /// `--kill-seconds` を超えたので、親がワーカーを止めた
    HardTimeout,
    /// ワーカーが結果を返す前に落ちた（または取り決めに無い行を返した）
    Crash,
}

impl Outcome {
    /// 実行できた（結果がある）
    pub(crate) fn simulated(self) -> bool {
        matches!(self, Outcome::StopTime | Outcome::Terminated)
    }

    fn label(self) -> &'static str {
        match self {
            Outcome::StopTime => "終了時刻まで",
            Outcome::Terminated => "terminate で終了",
            Outcome::CompileFailed => "コンパイル NG",
            Outcome::PreflightFailed => "S10 NG",
            Outcome::RunFailed => "実行 NG",
            Outcome::Panic => "**PANIC**",
            Outcome::HardTimeout => "外から停止",
            Outcome::Crash => "**ワーカー異常終了**",
        }
    }
}

/// 管で比べなかった信号 1 つ（外れたか、試験側に無い）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SignalFailure {
    pub name: String,
    /// 試験側か参照側にその信号が無い
    pub missing: bool,
    /// 参照側にその信号が無い（`missing` のうち参照側のもの）
    #[serde(default)]
    pub missing_in_reference: bool,
    /// 試験側の時刻の範囲が参照の区間を覆わない
    #[serde(default)]
    pub uncovered: bool,
    pub checked: usize,
    pub errors: usize,
    /// 管からはみ出した量の最大（有限でなければ None）
    pub max_excess: Option<f64>,
    pub first_error_time: Option<f64>,
}

/// 参照結果との比較の要約。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CompareSummary {
    pub signals: usize,
    pub passed: usize,
    /// 管の外に出た信号（両方にあるもの）
    pub failed: usize,
    /// 試験側に無い信号
    pub missing: usize,
    /// 参照側に無い信号
    #[serde(default)]
    pub missing_in_reference: usize,
    /// 試験側の時刻の範囲が参照の区間を覆わない信号
    #[serde(default)]
    pub uncovered: usize,
    /// 参照結果の行の数
    pub reference_points: usize,
    /// 合格しなかった信号（`comparisonSignals.txt` の順）
    pub failures: Vec<SignalFailure>,
}

impl CompareSummary {
    pub(crate) fn from_comparison(c: &Comparison, reference_points: usize) -> Self {
        let failures: Vec<SignalFailure> = c
            .signals
            .iter()
            .filter(|s| !s.passed)
            .map(|s| SignalFailure {
                name: s.name.clone(),
                missing: s.missing,
                missing_in_reference: s.missing_in_reference,
                uncovered: s.uncovered,
                checked: s.checked,
                errors: s.errors,
                max_excess: s.max_excess.is_finite().then_some(s.max_excess),
                first_error_time: s.first_error_time.filter(|t| t.is_finite()),
            })
            .collect();
        CompareSummary {
            signals: c.signals.len(),
            passed: c.signals.iter().filter(|s| s.passed).count(),
            failed: failures.iter().filter(|f| !f.missing).count(),
            missing: failures
                .iter()
                .filter(|f| f.missing && !f.missing_in_reference)
                .count(),
            missing_in_reference: failures.iter().filter(|f| f.missing_in_reference).count(),
            uncovered: failures.iter().filter(|f| f.uncovered).count(),
            reference_points,
            failures,
        }
    }

    /// すべての信号が管に収まった（信号が 1 つも無ければ false。`Comparison::passed` と同じ）
    pub(crate) fn passed_all(&self) -> bool {
        self.signals > 0 && self.passed == self.signals
    }
}

/// 参照結果との比較（実行できたときだけ）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum RefCheck {
    /// MAP-LIB に参照結果が無い
    NoReference,
    /// 参照結果を取得していない（`--fetch-ref` か `cargo xtask fetch-msl-ref`）
    NotFetched,
    /// 参照結果を読めない
    Unreadable {
        message: String,
    },
    Compared {
        comparison: CompareSummary,
    },
}

/// 1 本の結果（ワーカーの応答で、JSON の `models` の 1 行）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub model: String,
    pub outcome: Outcome,
    /// 最初の Error の段階（`S5`、`S10`、`Run` など）
    pub stage: Option<String>,
    /// 最初の Error のコード
    pub code: Option<String>,
    /// 最初の Error の文言（terminate で終わったときはその文言、panic・停止・異常終了はその説明）
    pub message: String,
    /// 出た診断のコード（情報と警告を含む。出た順、重複なし）
    pub codes: Vec<String>,
    pub settings: Option<ResolvedSettings>,
    pub termination: Option<Termination>,
    pub stats: Option<SimStats>,
    pub reference: Option<RefCheck>,
    /// `compile_retaining` の時間
    pub compile_seconds: f64,
    /// 参照結果の読み込みと比較の時間
    pub compare_seconds: f64,
    /// ワーカーの中での時間（コンパイル・実行・比較）
    pub seconds: f64,
    /// 親が測った時間（要求を送ってから応答まで）
    #[serde(default)]
    pub wall_seconds: f64,
    /// ワーカーの最大の常駐メモリ（VmHWM、kB。そのプロセスの起動から）
    pub rss_kb: Option<u64>,
}

impl Record {
    fn new(model: &str, outcome: Outcome) -> Self {
        Record {
            model: model.to_string(),
            outcome,
            stage: None,
            code: None,
            message: String::new(),
            codes: Vec::new(),
            settings: None,
            termination: None,
            stats: None,
            reference: None,
            compile_seconds: 0.0,
            compare_seconds: 0.0,
            seconds: 0.0,
            wall_seconds: 0.0,
            rss_kb: None,
        }
    }

    /// 親が作る結果（外から止めた・ワーカーが落ちた）。
    fn aborted(
        model: &str,
        outcome: Outcome,
        message: String,
        wall: f64,
        rss: Option<u64>,
    ) -> Self {
        Record {
            message,
            wall_seconds: wall,
            seconds: wall,
            rss_kb: rss,
            ..Record::new(model, outcome)
        }
    }

    fn set_error(&mut self, outcome: Outcome, d: &Diagnostic) {
        self.outcome = outcome;
        self.stage = Some(format!("{:?}", d.stage));
        self.code = Some(d.code.to_string());
        self.message = d.message.clone();
    }

    fn add_codes(&mut self, diagnostics: &[Diagnostic]) {
        for d in diagnostics {
            if !self.codes.iter().any(|c| c == d.code) {
                self.codes.push(d.code.to_string());
            }
        }
    }

    fn has_code(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code) || self.codes.iter().any(|c| c == code)
    }

    fn comparison(&self) -> Option<&CompareSummary> {
        match &self.reference {
            Some(RefCheck::Compared { comparison }) => Some(comparison),
            _ => None,
        }
    }

    /// 進み具合の行と報告の「結果」の欄。
    fn status(&self) -> String {
        match (self.outcome, &self.code) {
            (o, Some(code)) if !o.simulated() => format!("{} {code}", o.label()),
            (o, _) => o.label().to_string(),
        }
    }

    /// 参照結果の欄。
    fn reference_cell(&self) -> String {
        match &self.reference {
            None => "-".to_string(),
            Some(RefCheck::NoReference) => "参照結果なし".to_string(),
            Some(RefCheck::NotFetched) => "未取得".to_string(),
            Some(RefCheck::Unreadable { .. }) => "**読めない**".to_string(),
            Some(RefCheck::Compared { comparison: c }) if c.passed_all() => {
                format!("管 OK {}/{}", c.passed, c.signals)
            }
            Some(RefCheck::Compared { comparison: c }) => {
                let mut notes = Vec::new();
                if c.failed > 0 {
                    notes.push(format!("外れ {}", c.failed));
                }
                if c.missing > 0 {
                    notes.push(format!("無い {}", c.missing));
                }
                if c.missing_in_reference > 0 {
                    notes.push(format!("参照に無い {}", c.missing_in_reference));
                }
                if c.uncovered > 0 {
                    notes.push(format!("区間を覆わない {}", c.uncovered));
                }
                format!(
                    "**管 NG** {}/{}（{}）",
                    c.passed,
                    c.signals,
                    notes.join("・")
                )
            }
        }
    }
}

// --------------------------------------------------------------------------- ワーカー

/// `msl-sim-worker` の引数（親が渡す）。
#[derive(Debug, Clone)]
pub(crate) struct WorkerOptions {
    pub msl: PathBuf,
    pub ref_dir: PathBuf,
    pub wall_seconds: f64,
    pub solver: Option<String>,
    pub reset_every: usize,
    pub keep_results: Option<PathBuf>,
    pub tube_tolerance: f64,
}

/// ワーカーの行を書く。
fn send(message: &WorkerMessage) -> Result<()> {
    let json = serde_json::to_string(message).map_err(|e| e.to_string())?;
    let mut out = std::io::stdout().lock();
    // 前に誰かが改行なしで書いていても、印から後ろを読めるように行を分ける
    writeln!(out, "\n{MESSAGE_PREFIX}{json}").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

/// `cargo xtask msl-sim-worker`（親が起動する。手で呼ばない）。
pub(crate) fn worker(opts: &WorkerOptions) -> Result<ExitCode> {
    let t = Instant::now();
    let checker = make_checker(&opts.msl)?;
    send(&WorkerMessage::Ready {
        setup_seconds: t.elapsed().as_secs_f64(),
        rss_kb: peak_rss_kb(),
    })?;
    let stdin = std::io::stdin();
    let mut done = 0usize;
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Request =
            serde_json::from_str(&line).map_err(|e| format!("要求を読めない: {e}: {line}"))?;
        let t = Instant::now();
        // panic で Mutex が毒されても、Checker は次の検査でライブラリを読んだ直後の Session に戻す
        let record = catch_unwind(AssertUnwindSafe(|| {
            simulate_one(&checker, &request.model, opts)
        }))
        .unwrap_or_else(|payload| {
            let mut r = Record::new(&request.model, Outcome::Panic);
            r.message = panic_message(payload.as_ref());
            r
        });
        let record = Record {
            seconds: t.elapsed().as_secs_f64(),
            rss_kb: peak_rss_kb(),
            ..record
        };
        send(&WorkerMessage::Done {
            record: Box::new(record),
        })?;
        done += 1;
        if opts.reset_every > 0 && done.is_multiple_of(opts.reset_every) {
            checker.reset();
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// 1 本を実行し、参照結果と比べる。
fn simulate_one(checker: &Checker, model: &str, opts: &WorkerOptions) -> Record {
    let mut rec = Record::new(model, Outcome::RunFailed);
    let t = Instant::now();
    let compiled = checker.compile_retaining(&[], model);
    rec.compile_seconds = t.elapsed().as_secs_f64();
    let compiled = match compiled {
        Ok(c) => c,
        Err(diagnostics) => {
            rec.add_codes(&diagnostics);
            match diagnostics.iter().find(|d| d.is_error()) {
                Some(d) => rec.set_error(Outcome::CompileFailed, d),
                None => {
                    rec.outcome = Outcome::CompileFailed;
                    rec.message = "DAE まで変換できなかった（Error の診断が無い）".to_string();
                }
            }
            return rec;
        }
    };
    let settings = SimSettings {
        max_wall_seconds: Some(opts.wall_seconds),
        solver: opts.solver.clone(),
        ..SimSettings::default()
    };
    let report = globe_mode_exec::run(&compiled, &settings);
    drop(compiled);
    rec.add_codes(&report.diagnostics);
    if let Some(d) = report.diagnostics.iter().find(|d| d.is_error()) {
        // 設定とプリフライトは S10、実行は Run の段階で返る
        let outcome = if d.stage == Stage::Run {
            Outcome::RunFailed
        } else {
            Outcome::PreflightFailed
        };
        rec.set_error(outcome, d);
        // 設定は、決まっていれば実行に失敗しても残る（SimReport::settings）
        rec.settings = report
            .settings
            .clone()
            .or_else(|| report.run.as_ref().map(|r| r.settings.clone()));
        rec.stats = report.run.as_ref().map(|r| r.stats.clone());
        return rec;
    }
    let Some(mut run) = report.run else {
        rec.message = "実行系が結果も Error の診断も返さなかった".to_string();
        rec.settings = report.settings;
        return rec;
    };
    rec.outcome = match &run.termination {
        Termination::StopTime => Outcome::StopTime,
        Termination::Terminated { message, .. } => {
            rec.message = message.clone();
            Outcome::Terminated
        }
    };
    rec.settings = Some(run.settings.clone());
    rec.termination = Some(run.termination.clone());
    rec.stats = Some(run.stats.clone());
    if let Some(dir) = &opts.keep_results
        && let Err(e) = keep_result(dir, model, &run.trajectory)
    {
        eprintln!("結果を書けない: {model}: {e}");
    }
    let t = Instant::now();
    rec.reference = Some(check_reference(
        &opts.ref_dir,
        model,
        &mut run.trajectory,
        opts.tube_tolerance,
    ));
    rec.compare_seconds = t.elapsed().as_secs_f64();
    rec
}

/// 結果を `<dir>/<モデル>.csv` に書く。
fn keep_result(dir: &Path, model: &str, trajectory: &Trajectory) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{model}.csv"));
    let file = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut w = std::io::BufWriter::new(file);
    write_csv(trajectory, &mut w).map_err(|e| format!("{}: {e}", path.display()))?;
    w.flush().map_err(|e| format!("{}: {e}", path.display()))
}

/// 名前の中の空白を除く（`a[1, 2]` と `a[1,2]` を同じ名前にする）。
fn normalize_name(name: &str) -> String {
    name.split_whitespace().collect()
}

fn normalize_names(t: &mut Trajectory) {
    for v in &mut t.variables {
        if v.name.contains(char::is_whitespace) {
            v.name = normalize_name(&v.name);
        }
    }
}

/// 置き場の参照結果と比べる。`actual` の名前は空白を除いた形に直す。
fn check_reference(dir: &Path, model: &str, actual: &mut Trajectory, tolerance: f64) -> RefCheck {
    let (csv, signals) = match reference::local(dir, model) {
        Local::Present { csv, signals } => (csv, signals),
        Local::Absent => return RefCheck::NoReference,
        Local::NotFetched => return RefCheck::NotFetched,
    };
    let names = match fs::read_to_string(&signals) {
        Ok(text) => comparison_signals(&text),
        Err(e) => {
            return RefCheck::Unreadable {
                message: format!("{}: {e}", signals.display()),
            };
        }
    };
    let reference = fs::File::open(&csv)
        .map_err(|e| e.to_string())
        .and_then(|f| read_csv(&mut BufReader::new(f)).map_err(|e| e.to_string()));
    let mut reference = match reference {
        Ok(r) => r,
        Err(e) => {
            return RefCheck::Unreadable {
                message: format!("{}: {e}", csv.display()),
            };
        }
    };
    normalize_names(&mut reference);
    normalize_names(actual);
    let names: Vec<String> = names.iter().map(|n| normalize_name(n)).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let c = compare(&reference, actual, &names, &TubeOptions { tolerance });
    RefCheck::Compared {
        comparison: CompareSummary::from_comparison(&c, reference.time.len()),
    }
}

/// `comparisonSignals.txt` の信号の名前（`time` を除く）。
fn comparison_signals(text: &str) -> Vec<String> {
    parse_comparison_signals(text)
}

// --------------------------------------------------------------------------- 親（ワーカーの管理）

/// ワーカーの標準出力の 1 行。
#[derive(Debug)]
enum Line {
    Message(WorkerMessage),
    /// 印は付いているが読めない行
    Bad(String),
}

/// 受け取りの結果。
#[derive(Debug)]
enum Recv {
    Message(WorkerMessage),
    Bad(String),
    Eof,
    Timeout,
}

/// 印の付いた行を読む（付いていなければ None）。
fn parse_line(line: &str) -> Option<std::result::Result<WorkerMessage, String>> {
    let at = line.find(MESSAGE_PREFIX)?;
    let json = line[at + MESSAGE_PREFIX.len()..].trim();
    Some(serde_json::from_str(json).map_err(|e| format!("{e}: {}", one_line(json, 200))))
}

/// ワーカーの標準出力を読み、印の付いた行を送る。付いていない行は標準エラーへ流す。
fn read_lines(stdout: impl Read, tx: Sender<Line>) {
    let mut reader = BufReader::new(stdout);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end();
        let sent = match parse_line(line) {
            Some(Ok(m)) => tx.send(Line::Message(m)),
            Some(Err(e)) => tx.send(Line::Bad(e)),
            None => {
                if !line.trim().is_empty() {
                    eprintln!("    [ワーカー] {line}");
                }
                Ok(())
            }
        };
        if sent.is_err() {
            break;
        }
    }
}

/// 動いているワーカー 1 つ。標準出力を読むスレッドは待たない（ワーカーの子が管を持ち続けても、
/// 親が止まらないように）。管が閉じればスレッドも終わる。
#[derive(Debug)]
struct Worker {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Line>,
}

impl Worker {
    fn spawn(exe: &Path, args: &[OsString]) -> Result<Worker> {
        let mut child = Command::new(exe)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("ワーカー（{}）を起動できない: {e}", exe.display()))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or("ワーカーの標準出力を取れない")?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || read_lines(stdout, tx));
        Ok(Worker { child, stdin, rx })
    }

    fn recv(&self, deadline: Instant) -> Recv {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match self.rx.recv_timeout(remaining) {
            Ok(Line::Message(m)) => Recv::Message(m),
            Ok(Line::Bad(e)) => Recv::Bad(e),
            Err(RecvTimeoutError::Disconnected) => Recv::Eof,
            Err(RecvTimeoutError::Timeout) => Recv::Timeout,
        }
    }

    /// 用意ができるのを待つ。MSL の読み込みの秒数を返す。
    fn wait_ready(&mut self, timeout: Duration) -> Result<(f64, Option<u64>)> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.recv(deadline) {
                Recv::Message(WorkerMessage::Ready {
                    setup_seconds,
                    rss_kb,
                }) => return Ok((setup_seconds, rss_kb)),
                Recv::Message(WorkerMessage::Done { .. }) => {}
                Recv::Bad(e) => {
                    let status = self.stop();
                    return Err(format!("ワーカーが読めない行を返した（{status}）: {e}"));
                }
                Recv::Eof => {
                    let status = self.stop();
                    return Err(format!("ワーカーが用意の途中で終わった（{status}）"));
                }
                Recv::Timeout => {
                    let status = self.stop();
                    return Err(format!(
                        "ワーカーの用意が {} s で終わらない（{status}）",
                        timeout.as_secs()
                    ));
                }
            }
        }
    }

    fn send(&mut self, request: &Request) -> std::io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| std::io::Error::other("標準入力が閉じている"))?;
        let mut line = serde_json::to_string(request).map_err(std::io::Error::other)?;
        line.push('\n');
        stdin.write_all(line.as_bytes())?;
        stdin.flush()
    }

    /// ワーカーの最大の常駐メモリ（Linux の `/proc/<pid>/status` の VmHWM）。
    fn rss_kb(&self) -> Option<u64> {
        let status = fs::read_to_string(format!("/proc/{}/status", self.child.id())).ok()?;
        parse_vm_hwm(&status)
    }

    /// 止めて（まだ動いていれば kill して）終わりの状態を返す。
    fn stop(&mut self) -> String {
        self.stdin = None;
        let _ = self.child.kill();
        match self.child.wait() {
            Ok(s) => s.to_string(),
            Err(e) => format!("終わりの状態を取れない: {e}"),
        }
    }

    /// 標準入力を閉じて、自分で終わるのを待つ（30 s で終わらなければ kill）。
    fn finish(mut self) -> String {
        self.stdin = None;
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => break,
            }
        }
        self.stop()
    }
}

/// ワーカーを起動し、用意ができるのを待つ。
fn start_worker(exe: &Path, args: &[OsString], stats: &mut WorkerStats) -> Result<Worker> {
    let mut w = Worker::spawn(exe, args)?;
    let (setup, rss) = w.wait_ready(SETUP_TIMEOUT)?;
    stats.spawns += 1;
    stats.setup_seconds += setup;
    stats.setup_rss_kb = stats.setup_rss_kb.max(rss);
    println!(
        "ワーカーを起動した（{} 回目、MSL の読み込み {setup:.1} s）",
        stats.spawns
    );
    Ok(w)
}

/// 1 本の要求の結果。
#[derive(Debug)]
enum Reply {
    Done(Box<Record>),
    /// `--kill-seconds` を超えた（止める前に読んだ VmHWM）
    Timeout(Option<u64>),
    /// 応答の前に落ちたか、読めない行を返した
    Crash(String),
}

fn request(worker: &mut Worker, model: &str, kill: Duration) -> Reply {
    if let Err(e) = worker.send(&Request {
        model: model.to_string(),
    }) {
        return Reply::Crash(format!("要求を送れない: {e}"));
    }
    let deadline = Instant::now() + kill;
    loop {
        match worker.recv(deadline) {
            Recv::Message(WorkerMessage::Done { record }) if record.model == model => {
                return Reply::Done(record);
            }
            Recv::Message(_) => {}
            Recv::Bad(e) => return Reply::Crash(format!("読めない行: {e}")),
            Recv::Eof => return Reply::Crash("応答の前に標準出力が閉じた".to_string()),
            Recv::Timeout => return Reply::Timeout(worker.rss_kb()),
        }
    }
}

/// ワーカーの統計。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct WorkerStats {
    /// 起動した回数
    pub spawns: usize,
    /// MSL の読み込みの合計（秒）
    pub setup_seconds: f64,
    /// 外から止めた回数と、落ちた回数
    pub kills: usize,
    pub crashes: usize,
    /// 用意のときの常駐メモリの最大（kB）
    pub setup_rss_kb: Option<u64>,
}

// --------------------------------------------------------------------------- 集計

/// 結果の集計。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct Summary {
    pub total: usize,
    pub stop_time: usize,
    pub terminated: usize,
    pub compile_failed: usize,
    pub preflight_failed: usize,
    pub run_failed: usize,
    pub panics: usize,
    pub hard_timeouts: usize,
    pub crashes: usize,
    /// 止まったモデルの、最初の Error のコードごとの本数（コンパイル・S10・実行）
    pub failed_by_code: BTreeMap<String, usize>,
    /// 協調型の時間切れ（GM-E11005）
    pub cooperative_timeouts: usize,
    /// 結果が有限でない（GM-E11006）
    pub non_finite: usize,
    /// t0 で有限にならない警告（GM-W10002）が出た
    pub t0_not_finite: usize,
    /// ソルバの別名を置き換えた（GM-I10003）
    pub solver_alias: usize,
    /// 実行できたモデルのソルバごとの本数
    pub solvers: BTreeMap<String, usize>,
    /// 参照結果と比べた本数と、その内訳
    pub compared: usize,
    pub ref_passed: usize,
    /// 管から外れた信号がある
    pub ref_failed: usize,
    /// 外れは無いが、どちらかに無い信号がある
    pub ref_missing_only: usize,
    /// 試験側に無い信号がある（外れの有無を問わず）
    pub ref_with_missing: usize,
    /// 参照側に無い信号がある（外れの有無を問わず）
    pub ref_with_missing_in_reference: usize,
    /// 比べた信号の数と、そのうち合格した数
    pub signals: usize,
    pub signals_passed: usize,
    pub no_reference: usize,
    pub not_fetched: usize,
    pub unreadable: usize,
    pub compile_seconds: f64,
    pub build_seconds: f64,
    pub run_seconds: f64,
    pub compare_seconds: f64,
    /// ワーカーの中での時間の合計と、親が測った時間の合計・最大
    pub seconds: f64,
    pub wall_seconds: f64,
    pub max_wall_seconds: f64,
    pub max_rss_kb: Option<u64>,
}

impl Summary {
    pub(crate) fn from_records(records: &[Record]) -> Self {
        let mut s = Summary {
            total: records.len(),
            ..Summary::default()
        };
        for r in records {
            match r.outcome {
                Outcome::StopTime => s.stop_time += 1,
                Outcome::Terminated => s.terminated += 1,
                Outcome::CompileFailed => s.compile_failed += 1,
                Outcome::PreflightFailed => s.preflight_failed += 1,
                Outcome::RunFailed => s.run_failed += 1,
                Outcome::Panic => s.panics += 1,
                Outcome::HardTimeout => s.hard_timeouts += 1,
                Outcome::Crash => s.crashes += 1,
            }
            if matches!(
                r.outcome,
                Outcome::CompileFailed | Outcome::PreflightFailed | Outcome::RunFailed
            ) {
                *s.failed_by_code
                    .entry(r.code.clone().unwrap_or_else(|| "-".to_string()))
                    .or_default() += 1;
            }
            s.cooperative_timeouts += usize::from(r.code.as_deref() == Some(codes::RUN_TIMEOUT));
            s.non_finite += usize::from(r.code.as_deref() == Some(codes::RUN_NOT_FINITE));
            s.t0_not_finite += usize::from(r.has_code(codes::T0_NOT_FINITE));
            s.solver_alias += usize::from(r.has_code(codes::SOLVER_ALIAS));
            if r.outcome.simulated()
                && let Some(settings) = &r.settings
            {
                *s.solvers
                    .entry(settings.solver.name().to_string())
                    .or_default() += 1;
            }
            match &r.reference {
                None => {}
                Some(RefCheck::NoReference) => s.no_reference += 1,
                Some(RefCheck::NotFetched) => s.not_fetched += 1,
                Some(RefCheck::Unreadable { .. }) => s.unreadable += 1,
                Some(RefCheck::Compared { comparison: c }) => {
                    s.compared += 1;
                    s.signals += c.signals;
                    s.signals_passed += c.passed;
                    if c.passed_all() {
                        s.ref_passed += 1;
                    } else if c.failed > 0 || c.missing + c.missing_in_reference == 0 {
                        // 信号が 1 つも無いものも外れに数える（合格にしない）
                        s.ref_failed += 1;
                    } else {
                        s.ref_missing_only += 1;
                    }
                    s.ref_with_missing += usize::from(c.missing > 0);
                    s.ref_with_missing_in_reference += usize::from(c.missing_in_reference > 0);
                }
            }
            s.compile_seconds += r.compile_seconds;
            if let Some(stats) = &r.stats {
                s.build_seconds += stats.build_seconds;
                s.run_seconds += stats.run_seconds;
            }
            s.compare_seconds += r.compare_seconds;
            s.seconds += r.seconds;
            s.wall_seconds += r.wall_seconds;
            s.max_wall_seconds = s.max_wall_seconds.max(r.wall_seconds);
            s.max_rss_kb = s.max_rss_kb.max(r.rss_kb);
        }
        s
    }

    pub(crate) fn simulated(&self) -> usize {
        self.stop_time + self.terminated
    }

    /// 管に収まった割合（比べたうち）
    pub(crate) fn pass_rate(&self) -> Option<f64> {
        (self.compared > 0).then(|| self.ref_passed as f64 / self.compared as f64)
    }

    /// P3 の完了条件を満たすか（実行できた本数と、比べたうちの管に収まった割合）
    pub(crate) fn p3_met(&self) -> bool {
        self.simulated() >= P3_TARGET_RUNS
            && self.pass_rate().is_some_and(|r| r >= P3_TARGET_PASS_RATE)
    }
}

fn percent(n: usize, d: usize) -> String {
    if d == 0 {
        "-".to_string()
    } else {
        format!("{:.1}%", 100.0 * n as f64 / d as f64)
    }
}

fn summary_line(s: &Summary) -> String {
    format!(
        "{} 本: 実行できた {}（終了時刻まで {}・terminate {}）・コンパイル NG {}・S10 NG {}・実行 NG {}（うち時間切れ {}）・外から停止 {}・ワーカー異常終了 {}・panic {}",
        s.total,
        s.simulated(),
        s.stop_time,
        s.terminated,
        s.compile_failed,
        s.preflight_failed,
        s.run_failed,
        s.cooperative_timeouts,
        s.hard_timeouts,
        s.crashes,
        s.panics
    )
}

fn reference_line(s: &Summary) -> String {
    format!(
        "参照結果と比べた {} 本: 管に収まった {}（{}）・外れ {}・信号が足りない {}・参照結果なし {}・未取得 {}・読めない {}",
        s.compared,
        s.ref_passed,
        percent(s.ref_passed, s.compared),
        s.ref_failed,
        s.ref_missing_only,
        s.no_reference,
        s.not_fetched,
        s.unreadable
    )
}

fn p3_line(s: &Summary) -> String {
    let met = |ok: bool| if ok { "達成" } else { "未達" };
    format!(
        "実行できた {} 本（目標 {P3_TARGET_RUNS} 本以上: {}）・管に収まった {}/{}（{}、目標 {:.0}% 以上: {}。実行できたうちでは {}）→ **{}**",
        s.simulated(),
        met(s.simulated() >= P3_TARGET_RUNS),
        s.ref_passed,
        s.compared,
        percent(s.ref_passed, s.compared),
        P3_TARGET_PASS_RATE * 100.0,
        met(s.pass_rate().is_some_and(|r| r >= P3_TARGET_PASS_RATE)),
        percent(s.ref_passed, s.simulated()),
        if s.p3_met() {
            "満たす"
        } else {
            "まだ満たさない"
        }
    )
}

// --------------------------------------------------------------------------- 実行

/// `msl-sim` の引数。
#[derive(Debug)]
pub(crate) struct Options<'a> {
    pub funnel_json: &'a Path,
    pub msl: &'a Path,
    pub ref_dir: &'a Path,
    pub fetch_ref: bool,
    pub jobs: usize,
    pub limit: Option<usize>,
    pub only: Option<&'a str>,
    pub wall_seconds: f64,
    pub kill_seconds: u64,
    pub reset_every: usize,
    pub solver: Option<&'a str>,
    pub keep_results: Option<&'a Path>,
    pub report: Option<&'a Path>,
    pub json: Option<&'a Path>,
}

/// ワーカーに渡す引数。
fn worker_args(opts: &Options<'_>) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "msl-sim-worker".into(),
        "--msl".into(),
        opts.msl.into(),
        "--ref-dir".into(),
        opts.ref_dir.into(),
        "--wall-seconds".into(),
        opts.wall_seconds.to_string().into(),
        "--reset-every".into(),
        opts.reset_every.to_string().into(),
        "--tube-tolerance".into(),
        TubeOptions::default().tolerance.to_string().into(),
    ];
    if let Some(s) = opts.solver {
        args.extend(["--solver".into(), s.into()]);
    }
    if let Some(dir) = opts.keep_results {
        args.extend(["--keep-results".into(), dir.into()]);
    }
    args
}

/// 対象を選ぶ。partial のモデル（テンプレート）は実行しないので除き、`--only` の後に `--limit` を当てる。
/// 戻り値は（実行するもの, partial のため実行しないもの）。後者には `--only` だけを当てる。
pub(crate) fn select(
    all: &[Target],
    only: Option<&str>,
    limit: Option<usize>,
) -> (Vec<Target>, Vec<String>) {
    let matches = |t: &&Target| only.is_none_or(|o| t.model.contains(o));
    let skipped = all
        .iter()
        .filter(|t| t.partial)
        .filter(matches)
        .map(|t| t.model.clone())
        .collect();
    let targets = all
        .iter()
        .filter(|t| !t.partial)
        .filter(matches)
        .take(limit.unwrap_or(usize::MAX))
        .cloned()
        .collect();
    (targets, skipped)
}

pub(crate) fn msl_sim(opts: &Options<'_>) -> Result<ExitCode> {
    if !opts.msl.join(".globe-mode-msl").exists() {
        return Err(format!(
            "{} に MSL が無い。先に `cargo xtask fetch-msl` を実行する",
            opts.msl.display()
        ));
    }
    let started = SystemTime::now();
    let t_all = Instant::now();
    let all = funnel_ok_models(opts.funnel_json)?;
    let (targets, skipped_partial) = select(&all, opts.only, opts.limit);
    println!(
        "{} の OK {} 本のうち {} 本を実行する（partial のため実行しない {} 本。壁時計の予算 {} s、外から止める {} s）",
        opts.funnel_json.display(),
        all.len(),
        targets.len(),
        skipped_partial.len(),
        opts.wall_seconds,
        opts.kill_seconds
    );
    let fetch = if opts.fetch_ref {
        let models: Vec<String> = targets.iter().map(|t| t.model.clone()).collect();
        let stats = reference::fetch(opts.ref_dir, &models, opts.jobs)?;
        println!("参照結果: {}", stats.line());
        Some(stats)
    } else {
        if !reference::check_marker(opts.ref_dir)? {
            println!(
                "注意: {} に参照結果が無い。比べるには `--fetch-ref` を付けるか、`cargo xtask fetch-msl-ref` を先に実行する",
                opts.ref_dir.display()
            );
        }
        None
    };

    let exe =
        std::env::current_exe().map_err(|e| format!("自分の実行ファイルが分からない: {e}"))?;
    let args = worker_args(opts);
    let kill = Duration::from_secs(opts.kill_seconds);
    let mut stats = WorkerStats::default();
    let mut worker: Option<Worker> = None;
    let mut records = Vec::with_capacity(targets.len());
    let width = targets.len().to_string().len();
    for (i, target) in targets.iter().enumerate() {
        let w = match worker.as_mut() {
            Some(w) => w,
            None => match start_worker(&exe, &args, &mut stats) {
                Ok(w) => worker.insert(w),
                // 最初の起動の失敗（MSL が読めないなど）はそこで止める
                Err(e) if stats.spawns == 0 => return Err(e),
                // 途中で立て直せなければ、残りを異常終了として記録し、そこまでの報告を書く
                Err(e) => {
                    println!("ワーカーを立て直せない: {e}");
                    for t in &targets[i..] {
                        records.push(Record::aborted(
                            &t.model,
                            Outcome::Crash,
                            format!("ワーカーを立て直せない: {e}"),
                            0.0,
                            None,
                        ));
                    }
                    break;
                }
            },
        };
        let t = Instant::now();
        let reply = request(w, &target.model, kill);
        let wall = t.elapsed().as_secs_f64();
        let record = match reply {
            Reply::Done(record) => Record {
                wall_seconds: wall,
                ..*record
            },
            Reply::Timeout(rss) => {
                let status = worker.take().map_or_else(String::new, |mut w| w.stop());
                stats.kills += 1;
                Record::aborted(
                    &target.model,
                    Outcome::HardTimeout,
                    format!("{} s で応答が無いので止めた（{status}）", opts.kill_seconds),
                    wall,
                    rss,
                )
            }
            Reply::Crash(why) => {
                let status = worker.take().map_or_else(String::new, |mut w| w.stop());
                stats.crashes += 1;
                Record::aborted(
                    &target.model,
                    Outcome::Crash,
                    format!("{why}（{status}）"),
                    wall,
                    None,
                )
            }
        };
        println!(
            "[{:>width$}/{}] {} {}  {}  {wall:.2} s",
            i + 1,
            targets.len(),
            record.status(),
            target.model,
            record.reference_cell().replace("**", ""),
        );
        if matches!(
            record.outcome,
            Outcome::Panic | Outcome::Crash | Outcome::HardTimeout
        ) {
            println!("    {}", one_line(&record.message, 300));
        }
        let _ = std::io::stdout().flush();
        records.push(record);
    }
    if let Some(w) = worker.take() {
        w.finish();
    }

    let summary = Summary::from_records(&records);
    let run = RunInfo {
        opts,
        started,
        enumerated: all.len(),
        skipped_partial,
        fetch,
        workers: stats,
        total_seconds: t_all.elapsed().as_secs_f64(),
    };
    println!("{}", summary_line(&summary));
    println!("{}", reference_line(&summary));
    println!("P3: {}", p3_line(&summary).replace("**", ""));
    println!("時間: {}", time_line(&summary, &run));
    if let Some(path) = opts.report {
        fs::write(path, render_report(&records, &summary, &run)).map_err(|e| e.to_string())?;
        println!("報告: {}", path.display());
    }
    if let Some(path) = opts.json {
        let text = serde_json::to_string_pretty(&render_json(&records, &summary, &run))
            .map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())?;
        println!("JSON: {}", path.display());
    }
    Ok(if summary.panics == 0 && summary.crashes == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

// --------------------------------------------------------------------------- 報告

/// 報告に書く実行の情報。
#[derive(Debug)]
struct RunInfo<'a> {
    opts: &'a Options<'a>,
    started: SystemTime,
    /// `msl-funnel` で OK の本数
    enumerated: usize,
    /// partial（テンプレート）のため実行しないモデル（`--only` を当てた後）
    skipped_partial: Vec<String>,
    fetch: Option<reference::FetchStats>,
    workers: WorkerStats,
    /// 全体の壁時計（参照結果の取得を含む）
    total_seconds: f64,
}

/// UNIX 時刻（秒）を UTC の `YYYY-MM-DD HH:MM` にする。
fn utc_string(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant の civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

fn started_string(t: SystemTime) -> String {
    t.duration_since(UNIX_EPOCH)
        .map_or_else(|_| "-".to_string(), |d| utc_string(d.as_secs()))
}

fn time_line(s: &Summary, run: &RunInfo<'_>) -> String {
    format!(
        "全体 {:.1} s・ワーカーの中 {:.1} s（コンパイル {:.1} s・実行系の準備 {:.2} s・初期化と積分 {:.2} s・参照結果の読み込みと比較 {:.2} s）・MSL の読み込み {} 回 {:.1} s・1 本あたり（親から見た）平均 {:.2} s・最大 {:.2} s",
        run.total_seconds,
        s.seconds,
        s.compile_seconds,
        s.build_seconds,
        s.run_seconds,
        s.compare_seconds,
        run.workers.spawns,
        run.workers.setup_seconds,
        s.wall_seconds / s.total.max(1) as f64,
        s.max_wall_seconds
    )
}

/// 表の欄に入れる文言（1 行にし、表と強調を壊す文字を逃がす）。
fn md_cell(s: &str, max: usize) -> String {
    one_line(s, max)
        .replace('|', "\\|")
        .replace('*', "\\*")
        .replace('`', "'")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn short(model: &str) -> &str {
    model.strip_prefix("Modelica.").unwrap_or(model)
}

fn float_cell(x: Option<f64>) -> String {
    x.map_or("-".to_string(), |x| format!("{x:.3e}"))
}

fn failed_signals_cell(c: &CompareSummary, max: usize) -> String {
    let mut parts: Vec<String> = c
        .failures
        .iter()
        .take(max)
        .map(|f| {
            if f.missing_in_reference {
                format!("`{}`（参照に無い）", md_cell(&f.name, 80))
            } else if f.missing {
                format!("`{}`（無い）", md_cell(&f.name, 80))
            } else {
                format!(
                    "`{}`（t = {}・はみ出し {}・外れ {}/{} 点{}）",
                    md_cell(&f.name, 80),
                    f.first_error_time
                        .map_or("-".to_string(), |t| format!("{t:.4}")),
                    float_cell(f.max_excess),
                    f.errors,
                    f.checked,
                    if f.uncovered {
                        "・参照の区間を覆わない"
                    } else {
                        ""
                    }
                )
            }
        })
        .collect();
    if c.failures.len() > max {
        parts.push(format!("ほか {} 個", c.failures.len() - max));
    }
    if parts.is_empty() {
        "（比べる信号が無い）".to_string()
    } else {
        parts.join("、")
    }
}

fn render_report(records: &[Record], s: &Summary, run: &RunInfo<'_>) -> String {
    let opts = run.opts;
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# MSL {MSL_VERSION} の例題の実行と参照結果との比較（globe-mode-exec）\n"
    );
    let _ = writeln!(
        md,
        "`cargo xtask msl-sim` が生成した。手で編集しない。`msl-funnel --json` で OK になった例題を、実行系 `globe-mode-exec`（Phase A は Rumoca の実行系）で別プロセスのワーカーから実行し、MAP-LIB の参照結果（MSL {MSL_VERSION}）と矩形の管（csv-compare の既定のアルゴリズムを書き直した `globe_mode_sim::compare`）で比べる。\n"
    );
    let _ = writeln!(md, "| 項目 | 値 |\n| --- | --- |");
    let _ = writeln!(md, "| 実行した日時 | {} |", started_string(run.started));
    let _ = writeln!(
        md,
        "| MSL | {MSL_VERSION}（sha256 `{}…`） |",
        &MSL_SHA256[..12]
    );
    let _ = writeln!(
        md,
        "| Rumoca | {RUMOCA_VERSION}（rev `{}`、ライブラリとして呼ぶ） |",
        &RUMOCA_REV[..8]
    );
    let _ = writeln!(
        md,
        "| globe check の結果 | `{}`（OK {} 本） |",
        opts.funnel_json.display(),
        run.enumerated
    );
    let mut selection = Vec::new();
    if let Some(o) = opts.only {
        selection.push(format!("`--only {o}`"));
    }
    if let Some(n) = opts.limit {
        selection.push(format!("`--limit {n}`"));
    }
    let _ = writeln!(
        md,
        "| 実行した本数 | {} 本（{}） |",
        s.total,
        if selection.is_empty() {
            "partial を除いて全部".to_string()
        } else {
            format!("partial を除き、{}", selection.join("・"))
        }
    );
    let _ = writeln!(
        md,
        "| partial のため実行しない | {} 本（例題のテンプレートの partial なクラス。対象に数えない。名前は「partial のため実行しないモデル」の節） |",
        run.skipped_partial.len()
    );
    let fetch = run.fetch.as_ref().map_or_else(
        || "（この実行では取得していない）".to_string(),
        |f| format!("。この実行での取得: {}", f.line()),
    );
    let _ = writeln!(
        md,
        "| 参照結果 | [{MAP_LIB_REPO}](https://github.com/{MAP_LIB_REPO}) のブランチ `{MAP_LIB_BRANCH}`（commit `{}`）を `{}` に取得したもの{fetch} |",
        &MAP_LIB_SHA[..12],
        opts.ref_dir.display()
    );
    let solvers = if s.solvers.is_empty() {
        "-".to_string()
    } else {
        s.solvers
            .iter()
            .map(|(k, v)| format!("{k} {v} 本"))
            .collect::<Vec<_>>()
            .join("・")
    };
    let _ = writeln!(
        md,
        "| 実行の設定 | StartTime・StopTime・Interval・Tolerance は experiment annotation（無い項目は既定値: StartTime 0、StopTime は開始 + 1、Tolerance 1e-6、Interval は区間の 1/500）。ソルバは{}（実行できたもの: {solvers}）。壁時計の予算 {} s（協調型。超えると {}）。応答が {} s 無ければ親がワーカーを止める |",
        opts.solver.map_or("既定（bdf）".to_string(), |x| format!(
            " `--solver {x}`"
        )),
        opts.wall_seconds,
        codes::RUN_TIMEOUT,
        opts.kill_seconds
    );
    let _ = writeln!(
        md,
        "| 参照結果の設定（参考） | Dymola の Dassl。Tolerance は experiment annotation の 1/10（無ければ 1e-6）、Interval は annotation より細かい。手法も許容誤差も違うので、点ごとには一致しない |"
    );
    let _ = writeln!(
        md,
        "| 管 | 相対の幅 {}（時間と値の両方向。csv-compare の既定）。比べる信号は `comparisonSignals.txt`。名前の中の空白は除いて比べる |",
        TubeOptions::default().tolerance
    );
    let _ = writeln!(
        md,
        "| ワーカー | 起動 {} 回（外から停止 {} 回・異常終了 {} 回）、{} 本ごとに `Checker::reset` |",
        run.workers.spawns, run.workers.kills, run.workers.crashes, opts.reset_every
    );
    let _ = writeln!(md, "| 結果 | **{}** |", summary_line(s));
    let _ = writeln!(md, "| 参照結果との比較 | {} |", reference_line(s));
    let _ = writeln!(md, "| P3 の完了条件 | {} |\n", p3_line(s));

    let _ = writeln!(md, "## 集計\n");
    let _ = writeln!(md, "| 項目 | 本数 |\n| --- | ---: |");
    for (label, n) in [
        (
            "partial のため実行しない（対象に数えない）".to_string(),
            run.skipped_partial.len(),
        ),
        ("対象".to_string(), s.total),
        ("終了時刻まで実行できた".to_string(), s.stop_time),
        (
            format!(
                "terminate() で終了時刻の前に終わった（{}）",
                codes::RUN_TERMINATED
            ),
            s.terminated,
        ),
        ("**実行できた（計）**".to_string(), s.simulated()),
        (
            "DAE まで変換できなかった（compile_retaining の診断）".to_string(),
            s.compile_failed,
        ),
        (
            "設定・プリフライト（S10）で止まった".to_string(),
            s.preflight_failed,
        ),
        ("実行で止まった".to_string(), s.run_failed),
        (
            format!("うち協調型の時間切れ（{}）", codes::RUN_TIMEOUT),
            s.cooperative_timeouts,
        ),
        (
            format!("うち結果が有限でない（{}）", codes::RUN_NOT_FINITE),
            s.non_finite,
        ),
        (
            format!("外から止めた（{} s で応答が無い）", opts.kill_seconds),
            s.hard_timeouts,
        ),
        ("ワーカーが応答の前に落ちた".to_string(), s.crashes),
        ("panic（ワーカーの中で拾った）".to_string(), s.panics),
        (
            format!("t0 で有限にならない警告（{}）が出た", codes::T0_NOT_FINITE),
            s.t0_not_finite,
        ),
        (
            format!("ソルバの別名を置き換えた（{}）", codes::SOLVER_ALIAS),
            s.solver_alias,
        ),
    ] {
        let _ = writeln!(md, "| {label} | {n} |");
    }
    let _ = writeln!(md);

    let _ = writeln!(md, "### 止まったモデルのコード\n");
    if s.failed_by_code.is_empty() {
        let _ = writeln!(md, "（コンパイル・S10・実行で止まったモデルは無い）\n");
    } else {
        let _ = writeln!(md, "| コード | 本数 |\n| --- | ---: |");
        let mut by_count: Vec<(&String, &usize)> = s.failed_by_code.iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (code, n) in by_count {
            let _ = writeln!(md, "| {code} | {n} |");
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "### 参照結果との比較\n");
    let _ = writeln!(md, "| 項目 | 本数 |\n| --- | ---: |");
    for (label, n) in [
        ("実行できた", s.simulated()),
        ("うち参照結果と比べた", s.compared),
        ("**管に収まった（すべての信号）**", s.ref_passed),
        ("管から外れた信号がある", s.ref_failed),
        ("外れは無いが、どちらかに無い信号がある", s.ref_missing_only),
        (
            "試験側に無い信号がある（外れの有無を問わず）",
            s.ref_with_missing,
        ),
        (
            "参照側に無い信号がある（外れの有無を問わず）",
            s.ref_with_missing_in_reference,
        ),
        ("MAP-LIB に参照結果が無い", s.no_reference),
        ("参照結果を取得していない", s.not_fetched),
        ("参照結果を読めない", s.unreadable),
    ] {
        let _ = writeln!(md, "| {label} | {n} |");
    }
    let _ = writeln!(
        md,
        "| 管に収まった割合（比べたうち / 実行できたうち） | {} / {} |",
        percent(s.ref_passed, s.compared),
        percent(s.ref_passed, s.simulated())
    );
    let _ = writeln!(
        md,
        "| 信号の数で（合格 / 比べた信号） | {} / {} |\n",
        s.signals_passed, s.signals
    );

    let _ = writeln!(md, "## P3 の完了条件と基準値\n");
    let _ = writeln!(
        md,
        "P3 の完了条件（[roadmap](roadmap.md)）は「MSL の例題 {P3_TARGET_RUNS} 本以上が実行できる。そのうち {:.0}% 以上が参照結果のファネルに収まる」。この実行では {}。\n",
        P3_TARGET_PASS_RATE * 100.0,
        p3_line(s)
    );
    let _ = writeln!(
        md,
        "基準値は物差しが違う。Rumoca の「一致」は OMC の結果との正規化した平均絶対誤差で、管ではない。OpenModelica の verified は MAP-LIB の参照結果との照合（細かい規則は【未確認】）。\n"
    );
    let _ = writeln!(
        md,
        "| | 実行できた | 参照と比べた | 合う | 物差し | 出典（確認日） |\n| --- | ---: | ---: | ---: | --- | --- |"
    );
    let _ = writeln!(
        md,
        "| **GlobeMode（この実行）** | {} | {} | {} | 矩形の管（幅 {}、MAP-LIB の参照結果） | - |",
        s.simulated(),
        s.compared,
        s.ref_passed,
        TubeOptions::default().tolerance
    );
    let _ = writeln!(
        md,
        "| Rumoca の基準値（`msl_quality_baseline.json`） | {RUMOCA_BASELINE_SIM_OK} | - | - | `sim_ok`（試した {RUMOCA_BASELINE_SIM_ATTEMPTED} 本、制限 {RUMOCA_BASELINE_SIM_TIMEOUT} s） | [rev {}](https://github.com/CogniPilot/rumoca/blob/{RUMOCA_REV}/crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json)（2026-10-19） |",
        &RUMOCA_REV[..8]
    );
    let _ = writeln!(
        md,
        "| Rumoca の文書（2026-06-05 の全 MSL の実行） | 153 | 143（OMC と） | 89（high agreement） | 正規化した平均絶対誤差（管ではない） | [msl-baseline-promotion-analysis-2026-06-05.md](https://github.com/CogniPilot/rumoca/blob/{RUMOCA_REV}/docs/dev-guide/src/tooling/msl-baseline-promotion-analysis-2026-06-05.md)（2026-10-19） |"
    );
    let _ = writeln!(
        md,
        "| OpenModelica master（2026-10-17 の実行、MSL 4.1.0） | 533 | - | 510（verified） | MAP-LIB の参照結果との照合 | [libraries.openmodelica.org](https://libraries.openmodelica.org/branches/master/Modelica_4.1.0/Modelica_4.1.0.html)（2026-10-19） |\n"
    );

    let _ = writeln!(md, "## 時間とメモリ\n");
    let _ = writeln!(md, "| 項目 | 値 |\n| --- | --- |");
    let _ = writeln!(md, "| 時間 | {} |", time_line(s, run));
    let _ = writeln!(
        md,
        "| ワーカーの最大の常駐メモリ（VmHWM。用意のとき / 全体） | {} / {} |\n",
        run.workers.setup_rss_kb.map_or("-".to_string(), gigabytes),
        s.max_rss_kb.map_or("-".to_string(), gigabytes)
    );

    let _ = writeln!(md, "## 実行できなかったモデル\n");
    let failures: Vec<&Record> = records.iter().filter(|r| !r.outcome.simulated()).collect();
    if failures.is_empty() {
        let _ = writeln!(md, "（無い）\n");
    } else {
        let _ = writeln!(
            md,
            "| # | モデル | 結果 | 段階 | コード | 文言（先頭） | 秒 |\n| ---: | --- | --- | --- | --- | --- | ---: |"
        );
        for (i, r) in failures.iter().enumerate() {
            let _ = writeln!(
                md,
                "| {} | `{}` | {} | {} | {} | {} | {:.2} |",
                i + 1,
                short(&r.model),
                r.outcome.label(),
                r.stage.as_deref().unwrap_or("-"),
                r.code.as_deref().unwrap_or("-"),
                md_cell(&r.message, 160),
                r.wall_seconds
            );
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## 管から外れたモデル\n");
    let off: Vec<(&Record, &CompareSummary)> = records
        .iter()
        .filter_map(|r| r.comparison().filter(|c| !c.passed_all()).map(|c| (r, c)))
        .collect();
    if off.is_empty() {
        let _ = writeln!(md, "（無い）\n");
    } else {
        let _ = writeln!(
            md,
            "「t」は最初に管の外に出た時刻、「はみ出し」は管からはみ出した量の最大（値の単位）。\n"
        );
        let _ = writeln!(
            md,
            "| # | モデル | 合格 / 信号 | 外れた信号 |\n| ---: | --- | --- | --- |"
        );
        for (i, (r, c)) in off.iter().enumerate() {
            let _ = writeln!(
                md,
                "| {} | `{}` | {}/{} | {} |",
                i + 1,
                short(&r.model),
                c.passed,
                c.signals,
                failed_signals_cell(c, 5)
            );
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## partial のため実行しないモデル\n");
    if run.skipped_partial.is_empty() {
        let _ = writeln!(md, "（無い）\n");
    } else {
        let _ = writeln!(
            md,
            "`msl-funnel` では OK だが、partial なクラス（例題のテンプレート）なので実行しない。\n"
        );
        for m in &run.skipped_partial {
            let _ = writeln!(md, "- `{}`", short(m));
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## モデルごとの結果\n");
    let _ = writeln!(
        md,
        "- 「準備」は実行系の準備（lowering と組み立て）、「実行」は初期化と積分の時間（どちらも実行系が測った秒）。「計」は親が測った 1 本の時間（コンパイルと参照結果の比較を含む）。\n- 参照結果の欄の「管 OK a/b」は b 個の信号のうち a 個が管に収まったこと。\n"
    );
    let _ = writeln!(
        md,
        "| # | モデル | 結果 | コード | ソルバ | 状態 | 点 | 準備 s | 実行 s | 参照結果 | 計 s |\n| ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: |"
    );
    for (i, r) in records.iter().enumerate() {
        let (states, points, build, runs) = r.stats.as_ref().map_or_else(
            || {
                (
                    "-".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                )
            },
            |x| {
                (
                    x.states.to_string(),
                    x.points.to_string(),
                    format!("{:.2}", x.build_seconds),
                    format!("{:.2}", x.run_seconds),
                )
            },
        );
        let codes_cell = if r.codes.is_empty() {
            r.code.clone().unwrap_or_else(|| "-".to_string())
        } else {
            r.codes.join("・")
        };
        let _ = writeln!(
            md,
            "| {} | `{}` | {} | {codes_cell} | {} | {states} | {points} | {build} | {runs} | {} | {:.2} |",
            i + 1,
            short(&r.model),
            r.outcome.label(),
            r.settings.as_ref().map_or("-", |x| x.solver.name()),
            r.reference_cell(),
            r.wall_seconds
        );
    }
    md
}

fn render_json(records: &[Record], s: &Summary, run: &RunInfo<'_>) -> Value {
    use serde_json::json;
    let opts = run.opts;
    json!({
        "generated_by": "cargo xtask msl-sim",
        "started": started_string(run.started),
        "msl_version": MSL_VERSION,
        "rumoca_rev": RUMOCA_REV,
        "funnel_json": opts.funnel_json.display().to_string(),
        "funnel_ok": run.enumerated,
        "skipped_partial": run.skipped_partial,
        "reference": {
            "repo": MAP_LIB_REPO,
            "branch": MAP_LIB_BRANCH,
            "sha": MAP_LIB_SHA,
            "dir": opts.ref_dir.display().to_string(),
            "fetch": run.fetch.as_ref().map(|f| json!({
                "requested": f.requested,
                "cached": f.cached,
                "fetched": f.fetched,
                "files": f.files,
                "bytes": f.bytes,
                "absent": f.absent,
                "failed": f.failed,
                "seconds": f.seconds,
            })),
        },
        "settings": {
            "wall_seconds": opts.wall_seconds,
            "kill_seconds": opts.kill_seconds,
            "reset_every": opts.reset_every,
            "solver": opts.solver,
            "tube_tolerance": TubeOptions::default().tolerance,
            "only": opts.only,
            "limit": opts.limit,
        },
        "workers": run.workers,
        "total_seconds": run.total_seconds,
        "summary": s,
        "p3": {
            "target_runs": P3_TARGET_RUNS,
            "target_pass_rate": P3_TARGET_PASS_RATE,
            "simulated": s.simulated(),
            "pass_rate": s.pass_rate(),
            "met": s.p3_met(),
        },
        "baselines": {
            "rumoca_quality_baseline": {
                "sim_ok": RUMOCA_BASELINE_SIM_OK,
                "sim_attempted": RUMOCA_BASELINE_SIM_ATTEMPTED,
                "sim_timeout_seconds": RUMOCA_BASELINE_SIM_TIMEOUT,
                "rev": RUMOCA_REV,
            },
            "rumoca_doc_2026_06_05": { "simulation_ok": 153, "compared_to_omc": 143, "high_agreement": 89 },
            "openmodelica_master_2026_10_17": { "simulate": 533, "verified": 510 },
        },
        "models": records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_ir::{SettingOrigin, SettingOrigins, SolverId};
    use globe_mode_sim::compare::SignalComparison;

    fn settings() -> ResolvedSettings {
        let o = SettingOrigin::Experiment;
        ResolvedSettings {
            start_time: 0.0,
            stop_time: 1.0,
            tolerance: 1e-6,
            interval: 0.002,
            solver: SolverId::Bdf,
            max_wall_seconds: Some(30.0),
            origins: SettingOrigins {
                start_time: o,
                stop_time: o,
                tolerance: o,
                interval: o,
                solver: SettingOrigin::Default,
            },
        }
    }

    /// 信号 1 つの比較。`compare` が返す値を元にして、この試験で見る欄だけを書き換える
    /// （`SignalComparison` に欄が増えても試験を直さずに済むように）。
    fn signal(name: &str, passed: bool, missing: bool) -> SignalComparison {
        let t = Trajectory {
            time: vec![0.0, 1.0],
            variables: vec![globe_mode_ir::TrajectoryVariable {
                name: "x".into(),
                role: globe_mode_ir::VariableRole::State,
                unit: None,
                values: vec![1.0, 2.0],
            }],
        };
        let base = compare(&t, &t, &["x"], &TubeOptions::default())
            .signals
            .remove(0);
        SignalComparison {
            name: name.to_string(),
            passed,
            missing,
            missing_in_reference: false,
            checked: if missing { 0 } else { 100 },
            errors: usize::from(!passed && !missing) * 3,
            max_excess: if passed { 0.0 } else { f64::NAN },
            first_error_time: (!passed && !missing).then_some(0.25),
            ..base
        }
    }

    fn simulated(model: &str, comparison: Option<Comparison>) -> Record {
        Record {
            settings: Some(settings()),
            termination: Some(Termination::StopTime),
            stats: Some(SimStats {
                build_seconds: 0.5,
                run_seconds: 1.5,
                states: 3,
                points: 501,
            }),
            reference: Some(match comparison {
                Some(c) => RefCheck::Compared {
                    comparison: CompareSummary::from_comparison(&c, 1000),
                },
                None => RefCheck::NoReference,
            }),
            codes: vec![codes::T0_NOT_FINITE.to_string()],
            compile_seconds: 0.25,
            compare_seconds: 0.1,
            seconds: 2.5,
            wall_seconds: 2.6,
            rss_kb: Some(2 * 1024 * 1024),
            ..Record::new(model, Outcome::StopTime)
        }
    }

    fn failed(model: &str, outcome: Outcome, code: Option<&str>) -> Record {
        let mut r = Record::new(model, outcome);
        r.code = code.map(str::to_string);
        r.stage = code.map(|_| "Run".to_string());
        r.message = "解けない | 2 * x < 0".to_string();
        r.wall_seconds = 1.0;
        r
    }

    fn records() -> Vec<Record> {
        vec![
            simulated(
                "Modelica.A.Examples.Pass",
                Some(Comparison {
                    signals: vec![signal("x", true, false), signal("y", true, false)],
                }),
            ),
            simulated(
                "Modelica.A.Examples.Off",
                Some(Comparison {
                    signals: vec![signal("x", true, false), signal("v[1]", false, false)],
                }),
            ),
            simulated(
                "Modelica.A.Examples.Missing",
                Some(Comparison {
                    signals: vec![signal("x", true, false), signal("z", false, true)],
                }),
            ),
            simulated("Modelica.A.Examples.NoRef", None),
            Record {
                termination: Some(Termination::Terminated {
                    time: 0.5,
                    message: "done".into(),
                }),
                outcome: Outcome::Terminated,
                reference: Some(RefCheck::NotFetched),
                ..simulated("Modelica.A.Examples.Term", None)
            },
            failed(
                "Modelica.A.Examples.Slow",
                Outcome::RunFailed,
                Some(codes::RUN_TIMEOUT),
            ),
            failed(
                "Modelica.A.Examples.NaN",
                Outcome::RunFailed,
                Some(codes::RUN_NOT_FINITE),
            ),
            failed(
                "Modelica.A.Examples.Init",
                Outcome::RunFailed,
                Some(codes::RUN_INIT_FAILED),
            ),
            failed(
                "Modelica.A.Examples.Solver",
                Outcome::PreflightFailed,
                Some(codes::SOLVER_UNAVAILABLE),
            ),
            failed("Modelica.A.Examples.Hang", Outcome::HardTimeout, None),
            failed("Modelica.A.Examples.Boom", Outcome::Crash, None),
            failed("Modelica.A.Examples.Bug", Outcome::Panic, None),
        ]
    }

    #[test]
    fn protocol_round_trips() {
        let messages = [
            WorkerMessage::Ready {
                setup_seconds: 12.5,
                rss_kb: Some(1234),
            },
            WorkerMessage::Done {
                record: Box::new(records()[1].clone()),
            },
            WorkerMessage::Done {
                record: Box::new(records()[4].clone()),
            },
            WorkerMessage::Done {
                record: Box::new(records()[5].clone()),
            },
        ];
        for m in messages {
            let json = serde_json::to_string(&m).unwrap();
            assert!(!json.contains('\n'), "1 行に収まる: {json}");
            let line = format!("{MESSAGE_PREFIX}{json}");
            let back = match parse_line(&line) {
                Some(Ok(back)) => back,
                other => panic!("読めない: {other:?}"),
            };
            assert_eq!(back, m);
            // 前に改行なしの出力があっても印から後ろを読む
            let noisy = format!("progress 50%{line}");
            assert!(matches!(parse_line(&noisy), Some(Ok(ref b)) if *b == m));
        }
        let r = Request {
            model: "Modelica.A.B".into(),
        };
        let back: Request = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        // 印の無い行は None、壊れた行は Err
        assert!(parse_line("[rumoca] hello").is_none());
        assert!(matches!(
            parse_line(&format!("{MESSAGE_PREFIX}{{\"kind\":")),
            Some(Err(_))
        ));
    }

    #[test]
    fn non_finite_excess_becomes_none() {
        let mut in_reference = signal("w", false, true);
        in_reference.missing_in_reference = true;
        let c = Comparison {
            signals: vec![
                signal("x", true, false),
                signal("v", false, false),
                signal("u", false, true),
                in_reference,
            ],
        };
        let s = CompareSummary::from_comparison(&c, 10);
        assert_eq!(
            (
                s.signals,
                s.passed,
                s.failed,
                s.missing,
                s.missing_in_reference
            ),
            (4, 1, 1, 1, 1)
        );
        assert_eq!(s.failures[0].max_excess, None);
        assert_eq!(s.failures[0].first_error_time, Some(0.25));
        assert!(!s.passed_all());
        // 信号が 1 つも無ければ合格にしない
        assert!(!CompareSummary::from_comparison(&Comparison::default(), 0).passed_all());
    }

    #[test]
    fn summary_counts_outcomes_and_references() {
        let s = Summary::from_records(&records());
        assert_eq!(s.total, 12);
        assert_eq!((s.stop_time, s.terminated, s.simulated()), (4, 1, 5));
        assert_eq!(
            (s.run_failed, s.preflight_failed, s.compile_failed),
            (3, 1, 0)
        );
        assert_eq!((s.hard_timeouts, s.crashes, s.panics), (1, 1, 1));
        assert_eq!((s.cooperative_timeouts, s.non_finite), (1, 1));
        assert_eq!(s.failed_by_code.get(codes::RUN_INIT_FAILED), Some(&1));
        assert_eq!(s.failed_by_code.get(codes::SOLVER_UNAVAILABLE), Some(&1));
        assert_eq!(s.t0_not_finite, 5);
        assert_eq!(s.solvers.get("bdf"), Some(&5));
        assert_eq!(
            (s.compared, s.ref_passed, s.ref_failed, s.ref_missing_only),
            (3, 1, 1, 1)
        );
        assert_eq!((s.no_reference, s.not_fetched, s.unreadable), (1, 1, 0));
        assert_eq!((s.signals, s.signals_passed), (6, 4));
        assert_eq!(s.max_rss_kb, Some(2 * 1024 * 1024));
        assert!(!s.p3_met());
        assert_eq!(percent(1, 3), "33.3%");
        assert_eq!(percent(1, 0), "-");
    }

    #[test]
    fn report_shows_counts_and_tables() {
        let records = records();
        let s = Summary::from_records(&records);
        let opts = Options {
            funnel_json: Path::new("target/msl-funnel.json"),
            msl: Path::new("data/msl"),
            ref_dir: Path::new("data/msl-ref"),
            fetch_ref: false,
            jobs: 4,
            limit: Some(12),
            only: None,
            wall_seconds: 30.0,
            kill_seconds: 120,
            reset_every: 50,
            solver: None,
            keep_results: None,
            report: None,
            json: None,
        };
        let run = RunInfo {
            opts: &opts,
            started: UNIX_EPOCH + Duration::from_secs(1_792_368_000),
            enumerated: 369,
            skipped_partial: vec!["Modelica.A.Examples.ExampleTemplates.T".into()],
            fetch: None,
            workers: WorkerStats {
                spawns: 3,
                setup_seconds: 45.0,
                kills: 1,
                crashes: 1,
                setup_rss_kb: Some(1024 * 1024),
            },
            total_seconds: 99.0,
        };
        let md = render_report(&records, &s, &run);
        for needle in [
            "| 実行した日時 | 2026-10-19 00:00 UTC |",
            "| **実行できた（計）** | 5 |",
            "| うち協調型の時間切れ（GM-E11005） | 1 |",
            "| うち結果が有限でない（GM-E11006） | 1 |",
            "| 外から止めた（120 s で応答が無い） | 1 |",
            "| ワーカーが応答の前に落ちた | 1 |",
            "| panic（ワーカーの中で拾った） | 1 |",
            "| うち参照結果と比べた | 3 |",
            "| **管に収まった（すべての信号）** | 1 |",
            "| MAP-LIB に参照結果が無い | 1 |",
            "| GM-E10005 | 1 |",
            "目標 153 本以上: 未達",
            "| Rumoca の基準値（`msl_quality_baseline.json`） | 216 |",
            "| 533 | - | 510（verified） |",
            "`v[1]`（t = 0.2500・はみ出し -・外れ 3/100 点）",
            "`z`（無い）",
            "解けない \\| 2 \\* x &lt; 0",
            "| ワーカー | 起動 3 回（外から停止 1 回・異常終了 1 回）、50 本ごとに `Checker::reset` |",
            "1.0 GiB / 2.0 GiB",
            "| 実行した本数 | 12 本（partial を除き、`--limit 12`） |",
            "| partial のため実行しない | 1 本",
            "| partial のため実行しない（対象に数えない） | 1 |",
            "- `A.Examples.ExampleTemplates.T`",
        ] {
            assert!(md.contains(needle), "報告に {needle:?} が無い:\n{md}");
        }
        // 見出し・表の前後は空行（markdownlint の MD022 / MD058）
        let lines: Vec<&str> = md.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if l.starts_with('#') {
                assert!(i == 0 || lines[i - 1].is_empty(), "見出しの前: {l}");
                assert!(
                    lines.get(i + 1).is_some_and(|n| n.is_empty()),
                    "見出しの後: {l}"
                );
            }
            if l.starts_with('|') {
                let prev = lines[i - 1];
                assert!(prev.is_empty() || prev.starts_with('|'), "表の前: {l}");
                if let Some(next) = lines.get(i + 1) {
                    assert!(next.is_empty() || next.starts_with('|'), "表の後: {l}");
                }
            }
        }
        // 表の列の数がそろう（MD056）
        let mut columns = None;
        for l in &lines {
            if l.starts_with('|') {
                let n = l.replace("\\|", "").matches('|').count();
                match columns {
                    None => columns = Some(n),
                    Some(c) => assert_eq!(c, n, "列の数: {l}"),
                }
            } else {
                columns = None;
            }
        }
        let json = render_json(&records, &s, &run);
        assert_eq!(json["summary"]["ref_passed"], 1);
        assert_eq!(json["models"].as_array().map(Vec::len), Some(12));
        assert_eq!(json["reference"]["sha"], MAP_LIB_SHA);
        // JSON の models はワーカーの応答と同じ形で読み戻せる
        let back: Record = serde_json::from_value(json["models"][1].clone()).unwrap();
        assert_eq!(back, records[1]);
    }

    #[test]
    fn funnel_json_ok_models_and_selection() {
        let root: Value = serde_json::from_str(
            r#"{"models":[
                {"model":"Modelica.A.Examples.X","partial":false,"verdict":"Ok"},
                {"model":"Modelica.A.Examples.Y","partial":false,"verdict":"Ng"},
                {"model":"Modelica.A.Examples.Template","partial":true,"verdict":"Ok"},
                {"model":"Modelica.B.Examples.Z","verdict":"Ok"}
            ]}"#,
        )
        .unwrap();
        let all = ok_models_in(&root).unwrap();
        assert_eq!(
            all.iter().map(|t| t.model.as_str()).collect::<Vec<_>>(),
            [
                "Modelica.A.Examples.X",
                "Modelica.A.Examples.Template",
                "Modelica.B.Examples.Z"
            ]
        );
        assert!(all[1].partial);
        // partial は実行しない（--limit に数えず、別に返す）
        let (targets, skipped) = select(&all, None, Some(2));
        assert_eq!(
            targets.iter().map(|t| t.model.as_str()).collect::<Vec<_>>(),
            ["Modelica.A.Examples.X", "Modelica.B.Examples.Z"]
        );
        assert_eq!(skipped, ["Modelica.A.Examples.Template"]);
        let (targets, skipped) = select(&all, Some("Examples.Z"), None);
        assert_eq!((targets.len(), skipped.len()), (1, 0));
        let (targets, skipped) = select(&all, Some(".A."), Some(1));
        assert_eq!(targets[0].model, "Modelica.A.Examples.X");
        assert_eq!(skipped.len(), 1);
        assert!(ok_models_in(&serde_json::json!({})).is_none());
    }

    #[test]
    fn names_are_compared_without_spaces() {
        assert_eq!(normalize_name("a[1, 2]"), "a[1,2]");
        assert_eq!(normalize_name("C1.v"), "C1.v");
        assert_eq!(
            comparison_signals("time\r\nC1.v\r\n\n  L.i  \na[1, 2]\n"),
            ["C1.v", "L.i", "a[1, 2]"]
        );
    }

    /// 親の側（起動・応答・外から止める・落ちる）を、`sh` で書いた偽のワーカーで確かめる。
    #[cfg(unix)]
    #[test]
    fn parent_handles_replies_hangs_and_crashes() {
        let ready = serde_json::to_string(&WorkerMessage::Ready {
            setup_seconds: 0.5,
            rss_kb: Some(10),
        })
        .unwrap();
        let mut record = Record::new("Modelica.A.B", Outcome::StopTime);
        record.message = "ok".into();
        let done = serde_json::to_string(&WorkerMessage::Done {
            record: Box::new(record.clone()),
        })
        .unwrap();
        // 1 本目に答え（前に印の無い行を混ぜる）、2 本目で止まる
        let script = format!(
            "printf '%s\\n' 'loading MSL' '{MESSAGE_PREFIX}{ready}'; read line; printf 'noise %s\\n' '{MESSAGE_PREFIX}{done}'; read line; exec sleep 30"
        );
        let args: Vec<OsString> = vec!["-c".into(), script.into()];
        let mut stats = WorkerStats::default();
        let mut w = start_worker(Path::new("sh"), &args, &mut stats).unwrap();
        assert_eq!((stats.spawns, stats.setup_seconds), (1, 0.5));
        match request(&mut w, "Modelica.A.B", Duration::from_secs(10)) {
            Reply::Done(r) => assert_eq!(*r, record),
            other => panic!("答えが無い: {other:?}"),
        }
        let t = Instant::now();
        assert!(matches!(
            request(&mut w, "Modelica.A.C", Duration::from_millis(300)),
            Reply::Timeout(_)
        ));
        assert!(t.elapsed() < Duration::from_secs(5));
        let status = w.stop();
        assert!(!status.is_empty());

        // 用意の後、答えずに終わる
        let script = format!("printf '%s\\n' '{MESSAGE_PREFIX}{ready}'; read line; exit 3");
        let args: Vec<OsString> = vec!["-c".into(), script.into()];
        let mut w = start_worker(Path::new("sh"), &args, &mut stats).unwrap();
        assert!(matches!(
            request(&mut w, "Modelica.A.B", Duration::from_secs(10)),
            Reply::Crash(_)
        ));
        assert!(w.stop().contains('3'));

        // 用意の前に終わる・読めない行を返す
        let args: Vec<OsString> = vec!["-c".into(), "exit 1".into()];
        assert!(start_worker(Path::new("sh"), &args, &mut stats).is_err());
        let script = format!("printf '%s\\n' '{MESSAGE_PREFIX}{{broken'; exec sleep 30");
        let args: Vec<OsString> = vec!["-c".into(), script.into()];
        assert!(start_worker(Path::new("sh"), &args, &mut stats).is_err());
        assert_eq!(stats.spawns, 2);
    }

    #[test]
    fn utc_dates() {
        assert_eq!(utc_string(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_string(951_782_400), "2000-02-29 00:00 UTC");
        assert_eq!(utc_string(1_792_368_000 + 3_723), "2026-10-19 01:02 UTC");
    }

    #[test]
    fn worker_arguments_carry_options() {
        let opts = Options {
            funnel_json: Path::new("f.json"),
            msl: Path::new("data/msl"),
            ref_dir: Path::new("data/msl-ref"),
            fetch_ref: false,
            jobs: 4,
            limit: None,
            only: None,
            wall_seconds: 30.0,
            kill_seconds: 120,
            reset_every: 50,
            solver: Some("dopri45"),
            keep_results: Some(Path::new("out")),
            report: None,
            json: None,
        };
        let args: Vec<String> = worker_args(&opts)
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "msl-sim-worker");
        for pair in [
            ["--msl", "data/msl"],
            ["--ref-dir", "data/msl-ref"],
            ["--wall-seconds", "30"],
            ["--reset-every", "50"],
            ["--tube-tolerance", "0.002"],
            ["--solver", "dopri45"],
            ["--keep-results", "out"],
        ] {
            assert!(
                args.windows(2).any(|w| w[0] == pair[0] && w[1] == pair[1]),
                "{pair:?} が無い: {args:?}"
            );
        }
    }
}

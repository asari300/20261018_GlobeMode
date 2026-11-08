//! `globe fmu info` / `globe fmu run`（S204、P5 の FMI Import の最初の一切れ。ADR-0026）。
//!
//! `info` は `.fmu`（zip）の `modelDescription.xml` だけを読む（ネイティブコードは実行しない）。
//! `run` は FMU の共有ライブラリを読み込んで Co-Simulation か Model Exchange（S205）を回す。**FMU のネイティブコードを実行する**。
//! S210 から、読み込みと実行は**自分自身を子プロセス（隠しの副命令 `fmu __worker`）として起動した中**で行う。
//! FMU が落ちても `globe` は落ちず、終了の状態を添えた誤りで終わる（ADR-0026 の (n)）。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Subcommand, ValueEnum};
use globe_mode_fmi_import::{
    Kind, TRUST_WARNING, WorkerRequest, WorkerSpec, read_model_description, run_in_worker,
    worker_main,
};

/// `globe fmu run --kind`
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FmuKind {
    /// Co-Simulation（FMU の中の積分器に任せる）
    Cs,
    /// Model Exchange（FMU の微分を GlobeMode の積分器で積分する）
    Me,
}

#[derive(Debug, Subcommand)]
pub enum FmuImportCommand {
    /// FMU の要約（版・モデル名・インターフェース・変数）を表示する。ネイティブコードは実行しない
    #[command(after_help = "例:\n  globe fmu info Dahlquist.fmu")]
    Info {
        /// `.fmu`
        file: PathBuf,
    },
    /// FMU の Co-Simulation（固定刻み）か Model Exchange（誤差制御つきの積分と事象）を回し、時刻と出力を CSV で出す。**FMU の中のネイティブコードを別プロセス（ワーカー）で実行する**（落ちても globe は落ちない。ディスク・ネットワークは隔離しない）
    #[command(
        after_help = "例:\n  globe fmu run BouncingBall.fmu --stop-time 3 --step 0.01\n  globe fmu run BouncingBall.fmu --kind me --stop-time 3\n  globe fmu run Dahlquist.fmu --set k=2 --output x --out x.csv\n  globe fmu run Untrusted.fmu --timeout 60 --memory-limit 1024 --progress"
    )]
    Run {
        /// `.fmu`
        file: PathBuf,
        /// 回し方。省略時は Co-Simulation を持てばそれ、無ければ Model Exchange
        #[arg(long, value_enum)]
        kind: Option<FmuKind>,
        /// 開始時刻（省略時は DefaultExperiment、無ければ 0）
        #[arg(long)]
        start_time: Option<f64>,
        /// 終了時刻（省略時は DefaultExperiment）
        #[arg(long)]
        stop_time: Option<f64>,
        /// 通信刻み・出力の刻み（省略時は Model Exchange なら DefaultExperiment の stepSize、無ければ (終了 - 開始) / 500）
        #[arg(long)]
        step: Option<f64>,
        /// 出力する変数の名前（何度でも。省略時は causality が output の Real すべて）
        #[arg(long)]
        output: Vec<String>,
        /// 初期化の前に与える Real のパラメータ `名前=値`（何度でも）
        #[arg(long, value_parser = parse_set)]
        set: Vec<(String, f64)>,
        /// CSV の書き先（省略時は標準出力）
        #[arg(long)]
        out: Option<PathBuf>,
        /// 実行の時間の上限（秒）。超えたらワーカーを kill して失敗にする（省略時は無し）
        #[arg(long)]
        timeout: Option<f64>,
        /// ワーカーの仮想メモリの上限（MiB。Linux だけ）。超えると FMU の確保が失敗して落ちる
        #[arg(long)]
        memory_limit: Option<u64>,
        /// ワーカーが報告する時刻を標準エラーに出す
        #[arg(long)]
        progress: bool,
    },
    /// 内部用: FMU を回すワーカー（`fmu run` が自分自身を子プロセスとして起動する。標準入力の 1 行目が要求）
    #[command(name = "__worker", hide = true)]
    Worker,
}

fn parse_set(s: &str) -> Result<(String, f64), String> {
    let (name, value) = s.split_once('=').ok_or("名前=値 の形で渡す")?;
    let value = value
        .trim()
        .parse()
        .map_err(|_| format!("{value:?} は数でない"))?;
    Ok((name.trim().to_owned(), value))
}

pub fn run(command: FmuImportCommand) -> ExitCode {
    let result = match command {
        FmuImportCommand::Info { file } => info(&file),
        FmuImportCommand::Worker => return ExitCode::from(worker_main() as u8),
        FmuImportCommand::Run {
            file,
            kind,
            start_time,
            stop_time,
            step,
            output,
            set,
            out,
            timeout,
            memory_limit,
            progress,
        } => {
            // 書けない出力先と .mo は、実行の前に断る（simulate と同じ守り。S218）
            if let Some(path) = out.as_deref()
                && let Err(message) =
                    crate::check_writable_file(path, std::slice::from_ref(&file), &[])
            {
                eprintln!("error: 結果を {message}");
                return ExitCode::from(2);
            }
            run_fmu(
                &file,
                kind,
                Common {
                    start_time,
                    stop_time,
                    step,
                    outputs: output,
                    set,
                },
                out.as_deref(),
                Limits {
                    timeout,
                    memory_limit,
                    progress,
                },
            )
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn info(file: &std::path::Path) -> Result<(), globe_mode_fmi_import::Error> {
    let md = read_model_description(file)?;
    println!("{}（FMI {}）", md.model_name, md.fmi_version);
    for (label, i) in [
        ("ModelExchange", &md.model_exchange),
        ("CoSimulation", &md.co_simulation),
        ("ScheduledExecution", &md.scheduled_execution),
    ] {
        if let Some(i) = i {
            // ME は連続状態の数（ModelStructure の Derivatives）と event indicator の数も出す
            let me = if label == "ModelExchange" {
                format!(
                    "（連続状態 {}、event indicator {}）",
                    md.model_structure.derivatives.len(),
                    md.number_of_event_indicators
                )
            } else {
                String::new()
            };
            println!("  {label}: {}{me}", i.model_identifier);
        }
    }
    if let Some(e) = &md.default_experiment {
        let f = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
        println!(
            "  DefaultExperiment: 開始 {} 終了 {}",
            f(&e.start_time),
            f(&e.stop_time)
        );
    }
    println!("  変数 {}:", md.variables.len());
    for v in &md.variables {
        let start = v
            .start
            .as_deref()
            .map(|s| format!(" = {s}"))
            .unwrap_or_default();
        println!("    {} ({:?}, {:?}){start}", v.name, v.causality, v.ty);
    }
    Ok(())
}

/// CS と ME で共通のオプション。
struct Common {
    start_time: Option<f64>,
    stop_time: Option<f64>,
    step: Option<f64>,
    outputs: Vec<String>,
    set: Vec<(String, f64)>,
}

struct Limits {
    timeout: Option<f64>,
    memory_limit: Option<u64>,
    progress: bool,
}

fn run_fmu(
    file: &std::path::Path,
    kind: Option<FmuKind>,
    c: Common,
    out: Option<&std::path::Path>,
    limits: Limits,
) -> Result<(), globe_mode_fmi_import::Error> {
    use globe_mode_fmi_import::Error;
    eprintln!("{TRUST_WARNING}");
    let mut spec = WorkerSpec::new(
        std::env::current_exe().map_err(|e| Error::Worker(e.to_string()))?,
        &["fmu", "__worker"],
    );
    spec.timeout = limits
        .timeout
        .map(|s| {
            std::time::Duration::try_from_secs_f64(s)
                .map_err(|_| Error::Worker(format!("--timeout {s} が不正（0 以上の秒数）")))
        })
        .transpose()?;
    spec.memory_limit = limits.memory_limit.map(|m| m.saturating_mul(1 << 20));
    let run = run_in_worker(
        &spec,
        &WorkerRequest {
            file: file.to_owned(),
            kind: kind.map(|k| match k {
                FmuKind::Cs => Kind::Cs,
                FmuKind::Me => Kind::Me,
            }),
            start_time: c.start_time,
            stop_time: c.stop_time,
            step: c.step,
            outputs: c.outputs,
            set: c.set,
            work_dir: None, // run_in_worker が親の側で作る
        },
        &mut |t| {
            if limits.progress {
                eprintln!("進捗: t = {t}");
            }
        },
    )?;
    if let Some(me) = &run.me {
        eprintln!(
            "Model Exchange: 歩数 {}、事象 {}（state {}、time {}、step {}）{}",
            me.steps,
            me.state_events + me.time_events + me.step_events,
            me.state_events,
            me.time_events,
            me.step_events,
            if me.terminated_by_fmu {
                "、FMU が打ち切りを求めた"
            } else {
                ""
            }
        );
    }
    let mut text = format!("time,{}\n", run.names.join(","));
    for (t, values) in &run.rows {
        text.push_str(&t.to_string());
        for v in values {
            text.push_str(&format!(",{v}"));
        }
        text.push('\n');
    }
    match out {
        Some(p) => std::fs::write(p, text).map_err(|e| Error::Io(e.to_string()))?,
        None => print!("{text}"),
    }
    Ok(())
}

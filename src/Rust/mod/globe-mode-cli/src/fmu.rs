//! `globe fmu info` / `globe fmu run`（S204、P5 の FMI Import の最初の一切れ。ADR-0026）。
//!
//! `info` は `.fmu`（zip）の `modelDescription.xml` だけを読む（ネイティブコードは実行しない）。
//! `run` は FMU の共有ライブラリを読み込んで Co-Simulation か Model Exchange（S205）を回す。**FMU のネイティブコードを実行する**。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Subcommand, ValueEnum};
use globe_mode_fmi_import::{
    CsOptions, EventKind, Fmu, MeOptions, TRUST_WARNING, read_model_description, run_cs, run_me,
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
    /// FMU の Co-Simulation（固定刻み）か Model Exchange（誤差制御つきの積分と事象）を回し、時刻と出力を CSV で出す。**FMU の中のネイティブコードを実行する**
    #[command(
        after_help = "例:\n  globe fmu run BouncingBall.fmu --stop-time 3 --step 0.01\n  globe fmu run BouncingBall.fmu --kind me --stop-time 3\n  globe fmu run Dahlquist.fmu --set k=2 --output x --out x.csv"
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
    },
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
        FmuImportCommand::Run {
            file,
            kind,
            start_time,
            stop_time,
            step,
            output,
            set,
            out,
        } => run_fmu(
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
        ),
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

fn run_fmu(
    file: &std::path::Path,
    kind: Option<FmuKind>,
    c: Common,
    out: Option<&std::path::Path>,
) -> Result<(), globe_mode_fmi_import::Error> {
    eprintln!("{TRUST_WARNING}");
    let fmu = Fmu::open(file)?;
    let kind = kind.unwrap_or(if fmu.description.co_simulation.is_some() {
        FmuKind::Cs
    } else {
        FmuKind::Me
    });
    let (names, rows) = match kind {
        FmuKind::Cs => {
            let run = run_cs(
                &fmu,
                &CsOptions {
                    start_time: c.start_time,
                    stop_time: c.stop_time,
                    step: c.step,
                    outputs: c.outputs,
                    set: c.set,
                },
            )?;
            (run.names, run.rows)
        }
        FmuKind::Me => {
            let run = run_me(
                &fmu,
                &MeOptions {
                    start_time: c.start_time,
                    stop_time: c.stop_time,
                    step: c.step,
                    outputs: c.outputs,
                    set: c.set,
                    ..Default::default()
                },
            )?;
            let n = |k| run.events.iter().filter(|e| e.kind == k).count();
            eprintln!(
                "Model Exchange: 歩数 {}、事象 {}（state {}、time {}、step {}）{}",
                run.steps,
                run.events.len(),
                n(EventKind::State),
                n(EventKind::Time),
                n(EventKind::Step),
                if run.terminated_by_fmu {
                    "、FMU が打ち切りを求めた"
                } else {
                    ""
                }
            );
            (run.names, run.rows)
        }
    };
    let mut text = format!("time,{}\n", names.join(","));
    for (t, values) in &rows {
        text.push_str(&t.to_string());
        for v in values {
            text.push_str(&format!(",{v}"));
        }
        text.push('\n');
    }
    match out {
        Some(p) => {
            std::fs::write(p, text).map_err(|e| globe_mode_fmi_import::Error::Io(e.to_string()))?
        }
        None => print!("{text}"),
    }
    Ok(())
}

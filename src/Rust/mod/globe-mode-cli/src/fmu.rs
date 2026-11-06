//! `globe fmu info` / `globe fmu run`（S204、P5 の FMI Import の最初の一切れ。ADR-0026）。
//!
//! `info` は `.fmu`（zip）の `modelDescription.xml` だけを読む（ネイティブコードは実行しない）。
//! `run` は FMU の共有ライブラリを読み込んで Co-Simulation を回す。**FMU のネイティブコードを実行する**。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use globe_mode_fmi_import::{CsOptions, Fmu, TRUST_WARNING, read_model_description, run_cs};

#[derive(Debug, Subcommand)]
pub enum FmuImportCommand {
    /// FMU の要約（版・モデル名・インターフェース・変数）を表示する。ネイティブコードは実行しない
    #[command(after_help = "例:\n  globe fmu info Dahlquist.fmu")]
    Info {
        /// `.fmu`
        file: PathBuf,
    },
    /// FMU の Co-Simulation を固定刻みで回し、時刻と出力を CSV で出す。**FMU の中のネイティブコードを実行する**
    #[command(
        after_help = "例:\n  globe fmu run BouncingBall.fmu --stop-time 3 --step 0.01\n  globe fmu run Dahlquist.fmu --set k=2 --output x --out x.csv"
    )]
    Run {
        /// `.fmu`
        file: PathBuf,
        /// 開始時刻（省略時は DefaultExperiment、無ければ 0）
        #[arg(long)]
        start_time: Option<f64>,
        /// 終了時刻（省略時は DefaultExperiment）
        #[arg(long)]
        stop_time: Option<f64>,
        /// 通信刻み（省略時は (終了 - 開始) / 500）
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
            start_time,
            stop_time,
            step,
            output,
            set,
            out,
        } => {
            let opts = CsOptions {
                start_time,
                stop_time,
                step,
                outputs: output,
                set,
            };
            run_fmu(&file, &opts, out.as_deref())
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
            println!("  {label}: {}", i.model_identifier);
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

fn run_fmu(
    file: &std::path::Path,
    opts: &CsOptions,
    out: Option<&std::path::Path>,
) -> Result<(), globe_mode_fmi_import::Error> {
    eprintln!("{TRUST_WARNING}");
    let fmu = Fmu::open(file)?;
    let run = run_cs(&fmu, opts)?;
    let mut text = format!("time,{}\n", run.names.join(","));
    for (t, values) in &run.rows {
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

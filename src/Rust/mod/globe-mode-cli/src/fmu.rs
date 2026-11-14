//! `globe fmu info` / `globe fmu run`（S204、P5 の FMI Import の最初の一切れ。ADR-0026）。
//!
//! `info` は `.fmu`（zip）の `modelDescription.xml` だけを読む（ネイティブコードは実行しない）。
//! `run` は FMU の共有ライブラリを読み込んで Co-Simulation か Model Exchange（S205）を回す。**FMU のネイティブコードを実行する**。
//! S210 から、読み込みと実行は**自分自身を子プロセス（隠しの副命令 `fmu __worker`）として起動した中**で行う。
//! FMU が落ちても `globe` は落ちず、終了の状態を添えた誤りで終わる（ADR-0026 の (n)）。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Subcommand, ValueEnum};
use globe_mode_fmi_import::worker_main;

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
    /// 複数の Co-Simulation の FMU を固定の通信刻みでつなぎ（既定は Jacobi 型で入力は 1 刻み遅れる。`algorithm: gauss-seidel` で遅れなし）、全 FMU の出力を 1 つの CSV に出す。接続は Real・Int32・Boolean の同じ型どうし。FMU は別プロセス（ワーカー）で実行する
    #[command(
        after_help = "spec.json の例:\n  {\"fmus\": {\"a\": \"Dahlquist.fmu\", \"b\": \"Feedthrough.fmu\"}, \"connections\": [[\"a.x\", \"b.Float64_continuous_input\"]], \"stop_time\": 1, \"step\": 0.01}\n  （FMU のパスは spec.json からの相対。`algorithm`・`start`〔{\"a.k\": 2}〕・`timeout`・`memory_limit_mib` も書ける）\n.ssp（SSP 2.0.1。入れ子の System・ParameterBindings・単位の変換は未対応）も渡せる:\n  globe fmu cosim system.ssp --step 0.01 -o out.csv\n例:\n  globe fmu cosim spec.json -o out.csv"
    )]
    Cosim {
        /// 結線の記述（JSON）または `.ssp`（拡張子で分ける）
        spec: PathBuf,
        /// `.ssp` のとき: 終了時刻（省略時は DefaultExperiment の stopTime）
        #[arg(long)]
        stop_time: Option<f64>,
        /// `.ssp` のとき: 通信刻み（省略時は (終了 - 開始) / 100）
        #[arg(long)]
        step: Option<f64>,
        /// CSV の書き先（省略時は標準出力）
        #[arg(short, long)]
        out: Option<PathBuf>,
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
        FmuImportCommand::Cosim {
            spec,
            stop_time,
            step,
            out,
        } => cosim(&spec, stop_time, step, out.as_deref()),
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

fn info(file: &std::path::Path) -> Result<(), String> {
    let info = globe_mode::import_fmu_info(file)?;
    println!("{}（FMI {}）", info.model_name, info.fmi_version);
    for i in &info.interfaces {
        // ME は連続状態の数（ModelStructure の Derivatives）と event indicator の数も出す
        let me = match (i.continuous_states, i.event_indicators) {
            (Some(s), Some(e)) => format!("（連続状態 {s}、event indicator {e}）"),
            _ => String::new(),
        };
        println!("  {}: {}{me}", i.kind, i.model_identifier);
    }
    if let Some(e) = &info.default_experiment {
        let f = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
        println!(
            "  DefaultExperiment: 開始 {} 終了 {}",
            f(&e.start_time),
            f(&e.stop_time)
        );
    }
    println!("  変数 {}:", info.variables.len());
    for v in &info.variables {
        let start = v
            .start
            .as_deref()
            .map(|s| format!(" = {s}"))
            .unwrap_or_default();
        println!("    {} ({}, {}){start}", v.name, v.causality, v.ty);
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
) -> Result<(), String> {
    eprintln!("{}", globe_mode::FMU_TRUST_WARNING);
    let worker = std::env::current_exe().map_err(|e| e.to_string())?;
    let request = globe_mode::FmuRunRequest {
        kind: kind.map(|k| match k {
            FmuKind::Cs => globe_mode::FmuKind::Cs,
            FmuKind::Me => globe_mode::FmuKind::Me,
        }),
        start_time: c.start_time,
        stop_time: c.stop_time,
        step: c.step,
        outputs: c.outputs,
        set: c.set.into_iter().collect(),
        timeout: limits.timeout,
        memory_limit_mib: limits.memory_limit,
    };
    let run = globe_mode::run_fmu(
        &worker,
        file,
        &request,
        &mut |t| {
            if limits.progress {
                eprintln!("進捗: t = {t}");
            }
        },
        None,
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
    write_csv(&run, out)
}

/// `globe fmu cosim`（S249）。
fn cosim(
    spec: &std::path::Path,
    stop_time: Option<f64>,
    step: Option<f64>,
    out: Option<&std::path::Path>,
) -> Result<(), String> {
    if let Some(path) = out
        && let Err(message) = crate::check_writable_file(path, &[spec.to_owned()], &[])
    {
        return Err(format!("結果を {message}"));
    }
    // `.ssp`（S253）は展開先（drop で消える）の FMU を指す spec にする。`ssp` は回し終わるまで持つ
    let ssp;
    let spec_data = if spec
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ssp"))
    {
        ssp = globe_mode::Ssp::open(spec, stop_time, step).map_err(|e| e.to_string())?;
        ssp.spec.clone()
    } else {
        if stop_time.is_some() || step.is_some() {
            return Err("--stop-time・--step は .ssp のときだけ（JSON は中に書く）".into());
        }
        let text = std::fs::read_to_string(spec).map_err(|e| format!("{}: {e}", spec.display()))?;
        let mut spec_data: globe_mode::CosimSpec =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", spec.display()))?;
        // FMU のパスは spec.json のあるディレクトリからの相対
        let dir = spec.parent().unwrap_or(std::path::Path::new("."));
        for p in spec_data.fmus.values_mut() {
            *p = dir.join(&*p);
        }
        spec_data
    };
    eprintln!("{}", globe_mode::FMU_TRUST_WARNING);
    let worker = std::env::current_exe().map_err(|e| e.to_string())?;
    let run = globe_mode::run_cosim(&worker, &spec_data, &mut |_| {}, None)?;
    write_csv(&run, out)
}

fn write_csv(run: &globe_mode::FmuRun, out: Option<&std::path::Path>) -> Result<(), String> {
    let (layout, columns) = run.results.columns();
    let mut text = csv_header(&layout.names[1..]);
    for i in 0..layout.points {
        let row: Vec<String> = columns.iter().map(|c| c[i].to_string()).collect();
        text.push_str(&row.join(","));
        text.push('\n');
    }
    match out {
        Some(p) => std::fs::write(p, text).map_err(|e| format!("入出力の失敗: {e}"))?,
        None => print!("{text}"),
    }
    Ok(())
}

/// CSV の見出し行。`A[1,2]` のように `,` `"` を含む名前だけを引用する（simulate と同じ `push_quoted`。#52）。
/// ほかは今までどおり引用しない（`time,x`）。
fn csv_header(names: &[String]) -> String {
    let mut line = String::from("time");
    for n in names {
        line.push(',');
        if n.contains([',', '"', '\n', '\r']) {
            globe_mode::sim::store::push_quoted(&mut line, n);
        } else {
            line.push_str(n);
        }
    }
    line.push('\n');
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #52: カンマを含む変数名（配列の `A[1,2]`）は引用し、列がずれない。
    #[test]
    fn csv_header_quotes_names_with_commas() {
        let names = ["x".to_string(), "A[1,2]".to_string(), "q\"".to_string()];
        assert_eq!(csv_header(&names), "time,x,\"A[1,2]\",\"q\"\"\"\n");
    }
}

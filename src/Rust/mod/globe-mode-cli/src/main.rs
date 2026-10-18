//! GlobeMode の CLI `globe`。
//!
//! ```text
//! globe outline <FILE> [--json]
//! globe check <FILE>... --model <NAME> [--json]
//! ```
//!
//! 終了コード: 0 = 成功 / 1 = Error の診断がある / 2 = 入力の読み込みに失敗。

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use globe_mode_core::{Diagnostic, line_col};
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_ir::{ClassOutline, Outline};

#[derive(Debug, Parser)]
#[command(
    name = "globe",
    version,
    about = "GlobeMode — an open-source Modelica IDE (CLI)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// ファイルのアウトライン（クラス・コンポーネント・connect）を表示する
    Outline {
        file: PathBuf,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// モデルを DAE まで変換して検査する（S0〜S5）
    Check {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// 検査するモデルの完全修飾名（例: GlobeModeExamples.RC）
        #[arg(long, short)]
        model: String,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let frontend = RumocaFrontend::new();
    match cli.command {
        Command::Outline { file, json } => outline(&frontend, &file, json),
        Command::Check { files, model, json } => check(&frontend, &files, &model, json),
    }
}

fn read(path: &PathBuf) -> Result<(String, String), ExitCode> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok((path.display().to_string(), text)),
        Err(e) => {
            eprintln!("error: {} を読めない: {e}", path.display());
            Err(ExitCode::from(2))
        }
    }
}

fn outline(frontend: &impl Frontend, file: &PathBuf, json: bool) -> ExitCode {
    let (name, text) = match read(file) {
        Ok(v) => v,
        Err(code) => return code,
    };
    match frontend.outline(Source {
        name: &name,
        text: &text,
    }) {
        Ok(o) if json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&o).expect("serialize outline")
            );
            ExitCode::SUCCESS
        }
        Ok(o) => {
            print_outline(&o, &text);
            ExitCode::SUCCESS
        }
        Err(diags) => {
            let texts = HashMap::from([(name, text)]);
            print_diagnostics(&diags, &texts, json);
            ExitCode::from(1)
        }
    }
}

fn check(frontend: &impl Frontend, files: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let mut texts = HashMap::new();
    let mut order = Vec::new();
    for f in files {
        match read(f) {
            Ok((name, text)) => {
                order.push(name.clone());
                texts.insert(name, text);
            }
            Err(code) => return code,
        }
    }
    let sources: Vec<Source<'_>> = order
        .iter()
        .map(|n| Source {
            name: n,
            text: &texts[n],
        })
        .collect();
    let report = frontend.check(&sources, model);

    if json {
        let value = serde_json::json!({
            "model": report.model,
            "passed": report.passed(),
            "failed_stage": report.failed_stage(),
            "balance": report.balance,
            "diagnostics": report.diagnostics,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serialize report")
        );
    } else {
        print_diagnostics(&report.diagnostics, &texts, false);
        if let Some(b) = report.balance {
            println!(
                "{}: 方程式 {} / 変数 {}（状態 {}・代数 {}・出力 {}・離散 {}）",
                report.model,
                b.equations,
                b.unknowns,
                b.states,
                b.algebraics,
                b.outputs,
                b.discrete
            );
        }
        match report.failed_stage() {
            None => println!("{}: OK（S0〜S5 を通過）", report.model),
            Some(stage) => println!(
                "{}: NG（{:?} {} で停止）",
                report.model,
                stage,
                stage.title()
            ),
        }
    }
    if report.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_outline(o: &Outline, text: &str) {
    if let Some(w) = &o.within {
        println!("within {w};");
    }
    for (qualified, c) in o.qualified_classes() {
        print_class(&qualified, c, text);
    }
}

fn print_class(qualified: &str, c: &ClassOutline, text: &str) {
    let at = line_col(text, c.range.start);
    let kind = serde_json::to_value(c.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    let partial = if c.partial { "partial " } else { "" };
    println!("{partial}{kind} {qualified}  ({}:{})", at.line, at.column);
    for e in &c.extends {
        println!("    extends {e}");
    }
    for comp in &c.components {
        println!("    {} {}", comp.type_name, comp.name);
    }
    for k in &c.connects {
        println!("    connect({}, {})", k.lhs, k.rhs);
    }
}

fn print_diagnostics(diags: &[Diagnostic], texts: &HashMap<String, String>, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(diags).expect("serialize diagnostics")
        );
        return;
    }
    for d in diags {
        let place = match (&d.file, d.range) {
            (Some(f), Some(r)) => {
                let at = texts.get(f).map(|t| line_col(t, r.start));
                at.map_or_else(
                    || format!("{f}: "),
                    |at| format!("{f}:{}:{}: ", at.line, at.column),
                )
            }
            (Some(f), None) => format!("{f}: "),
            _ => String::new(),
        };
        let severity = serde_json::to_value(d.severity)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        println!(
            "{place}{severity}[{}] {:?} {}: {}",
            d.code,
            d.stage,
            d.stage.title(),
            d.message
        );
        for n in &d.notes {
            println!("    note: {n}");
        }
    }
}

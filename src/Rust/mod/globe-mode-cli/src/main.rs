//! GlobeMode の CLI `globe`。
//!
//! ```text
//! globe outline <FILE> [--json]
//! globe check [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe graph <FILE|DIR>... --model <NAME> [--depth N] [--json]
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
    /// connect グラフ（flatten しない）を表示する。--json は graphology の形式
    Graph {
        /// ファイルまたはディレクトリ（ディレクトリは下の .mo をすべて読む）
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// モデルの完全修飾名
        #[arg(long, short)]
        model: String,
        /// 展開する深さ（1 は直下の部品だけ）
        #[arg(long, short, default_value_t = 1)]
        depth: u32,
        /// graphology の JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// モデルを DAE まで変換して検査する（S0〜S5）
    Check {
        /// 検査するファイルまたはディレクトリ（ディレクトリは下の .mo をすべて読む）。
        /// モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present = "library")]
        files: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
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
        Command::Graph {
            inputs,
            model,
            depth,
            json,
        } => graph(&inputs, &model, depth, json),
        Command::Check {
            files,
            library,
            model,
            json,
        } => check(&frontend, &files, &library, &model, json),
    }
}

/// 入力のうちディレクトリは、その下の `.mo` をすべて（名前順に）展開する。
///
/// 作業中のパッケージ（`src/Modelica/GlobeModeValidation` など）は、ライブラリ（`--library`）ではなく
/// 文書として読む。Rumoca v0.10.2 は、ライブラリとして読んだクラスから別のライブラリ（MSL）を
/// extends できない（S004 で確かめた）ため。
fn expand_inputs(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, ExitCode> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let p = entry?.path();
            if p.is_dir() {
                walk(&p, out)?;
            } else if p.extension().is_some_and(|e| e == "mo") {
                out.push(p);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let mut found = Vec::new();
            if let Err(e) = walk(input, &mut found) {
                eprintln!("error: {} を読めない: {e}", input.display());
                return Err(ExitCode::from(2));
            }
            found.sort();
            out.extend(found);
        } else {
            out.push(input.clone());
        }
    }
    Ok(out)
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

fn check(
    frontend: &impl Frontend,
    files: &[PathBuf],
    library: &[PathBuf],
    model: &str,
    json: bool,
) -> ExitCode {
    for lib in library {
        if !lib.is_dir() {
            eprintln!("error: ライブラリ {} がディレクトリではない", lib.display());
            return ExitCode::from(2);
        }
    }
    let libraries: Vec<&std::path::Path> = library.iter().map(PathBuf::as_path).collect();
    let mut texts = HashMap::new();
    let mut order = Vec::new();
    let files = match expand_inputs(files) {
        Ok(f) => f,
        Err(code) => return code,
    };
    for f in &files {
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
    let report = frontend.check(&sources, &libraries, model);

    if json {
        let value = serde_json::json!({
            "model": report.model,
            "passed": report.passed(),
            "failed_stage": report.failed_stage(),
            "balance": report.balance,
            "dae": report.dae,
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
        if let Some(d) = report.dae {
            println!(
                "{}: DAE の変数（配列は 1 個）: 状態 {}・代数 {}・出力 {}・連続方程式 {}",
                report.model, d.states, d.algebraics, d.outputs, d.continuous_equations
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

fn graph(inputs: &[PathBuf], model: &str, depth: u32, json: bool) -> ExitCode {
    let files = match expand_inputs(inputs) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let mut db = globe_mode_db::GlobeDb::new();
    for f in &files {
        match read(f) {
            Ok((name, text)) => db.set_file(&name, text),
            Err(code) => return code,
        }
    }
    match db.connect_graph(model, depth) {
        Ok(g) if json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&g.to_graphology()).expect("serialize graph")
            );
            ExitCode::SUCCESS
        }
        Ok(g) => {
            println!(
                "{}（depth {}）: ノード {} / 辺 {}",
                g.model,
                g.depth,
                g.nodes.len(),
                g.edges.len()
            );
            for e in &g.edges {
                let domain = serde_json::to_value(e.domain)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                println!(
                    "  {:<8} {} -- {}   ({} — {})",
                    domain, e.source, e.target, e.lhs, e.rhs
                );
            }
            if !g.unresolved.is_empty() {
                println!("  未解決の型: {}", g.unresolved.join(", "));
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
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

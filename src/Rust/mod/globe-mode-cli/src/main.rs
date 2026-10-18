//! GlobeMode の CLI `globe`。
//!
//! ```text
//! globe outline <FILE> [--json]
//! globe check [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe flat  [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe graph <FILE|DIR>... --model <NAME> [--depth N] [--json]
//! ```
//!
//! `check` と `flat` は salsa の DB（`globe-mode-db`）を通す。ライブラリは一度だけ読む。
//!
//! 終了コード: 0 = 成功 / 1 = Error の診断がある / 2 = 入力の読み込みに失敗。

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use globe_mode_core::{Diagnostic, Stage, line_col};
use globe_mode_db::{GlobeDb, normalize_path};
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_ir::{ClassOutline, FlatSummary, Outline};

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
    /// モデルを DAE まで変換して検査する（S0〜S6）
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
    /// flat モデルの要約（変数・方程式の数、部品ごとの方程式数）を表示する
    Flat {
        /// ファイルまたはディレクトリ。モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present = "library")]
        files: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
        /// モデルの完全修飾名
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
        } => check(&files, &library, &model, json),
        Command::Flat {
            files,
            library,
            model,
            json,
        } => flat(&files, &library, &model, json),
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

/// 入力を読み込んだ DB と、診断の位置を行と列に直すための本文
/// （キーは DB と同じく区切りを `/` にそろえたパス）。
struct Loaded {
    db: GlobeDb,
    texts: HashMap<String, String>,
}

/// ライブラリを読んだ DB に、入力のファイルを入れる。ライブラリを読めなければ `Ok(Err(診断))`。
fn load_db(
    files: &[PathBuf],
    library: &[PathBuf],
) -> Result<Result<Loaded, Vec<Diagnostic>>, ExitCode> {
    for lib in library {
        if !lib.is_dir() {
            eprintln!("error: ライブラリ {} がディレクトリではない", lib.display());
            return Err(ExitCode::from(2));
        }
    }
    let files = expand_inputs(files)?;
    let mut inputs = Vec::new();
    for f in &files {
        inputs.push(read(f)?);
    }
    let mut db = match GlobeDb::with_libraries(library) {
        Ok(db) => db,
        Err(diags) => return Ok(Err(diags)),
    };
    let mut texts = HashMap::new();
    for (name, text) in inputs {
        db.set_file(&name, text.clone());
        texts.insert(normalize_path(&name), text);
    }
    Ok(Ok(Loaded { db, texts }))
}

fn check(files: &[PathBuf], library: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let Loaded { mut db, texts } = match load_db(files, library) {
        Ok(Ok(v)) => v,
        // ライブラリを読めない（S1）
        Ok(Err(diags)) => return report_early_failure(model, &diags, json),
        Err(code) => return code,
    };
    let report = db.check_model(model);
    let check = &report.check;

    if json {
        let structure = check.structure.as_ref().map(|s| {
            serde_json::json!({
                "unknowns": s.unknowns.len(),
                "equations": s.equations.len(),
                "derivatives": s.unknowns.iter().filter(|u| u.kind == globe_mode_ir::UnknownKind::Derivative).count(),
            })
        });
        let value = serde_json::json!({
            "model": report.model,
            "passed": report.passed(),
            "failed_stage": report.failed_stage(),
            "balance": check.balance,
            "dae": check.dae,
            "structure": structure,
            "s6": report.structural,
            "flat": check.flat,
            "diagnostics": report.diagnostics,
            "notes": check.notes,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serialize report")
        );
    } else {
        print_diagnostics(&report.diagnostics, &texts, false);
        if let Some(b) = check.balance {
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
        if let Some(d) = check.dae {
            println!(
                "{}: DAE の変数（配列は 1 個）: 状態 {}・代数 {}・出力 {}・連続方程式 {}",
                report.model, d.states, d.algebraics, d.outputs, d.continuous_equations
            );
        }
        for note in &check.notes {
            println!("{}: note: {note}", report.model);
        }
        if let Some(s6) = &report.structural {
            // S6 の判定は x と der(x) を 1 つの列にまとめたグラフ（structural_matching）。
            // der の形（matching）で完全マッチングが無いのは、微分インデックスが 1 より大きい見込み（S7 の情報）。
            if s6.is_regular() {
                let index = if s6.index_reduction_needed {
                    format!(
                        "。der の形では {} / {}（インデックス低減が要る見込み）",
                        s6.matching, s6.n_equations
                    )
                } else {
                    String::new()
                };
                println!(
                    "{}: S6: 完全マッチング {} / {}{index}",
                    report.model, s6.structural_matching, s6.n_equations
                );
            } else {
                println!(
                    "{}: S6: 最大マッチング {}（方程式 {} / 未知数 {}）。構造特異",
                    report.model, s6.structural_matching, s6.n_equations, s6.n_unknowns
                );
            }
        }
        match report.failed_stage() {
            None if report.structural.is_some() => {
                println!("{}: OK（S0〜S6 を通過）", report.model)
            }
            None => println!(
                "{}: OK（S0〜S5 を通過。S6 は判定できなかった）",
                report.model
            ),
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

fn flat(files: &[PathBuf], library: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let Loaded { mut db, texts } = match load_db(files, library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, json),
        Err(code) => return code,
    };
    let Some(summary) = db.flat_summary(model) else {
        // flatten まで通らなかった。検査の診断を出す
        let report = db.check_model(model);
        if json {
            print_diagnostics(&report.diagnostics, &texts, true);
        } else {
            print_diagnostics(&report.diagnostics, &texts, false);
            println!("{model}: flat モデルを作れなかった");
        }
        return ExitCode::from(1);
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).expect("serialize flat summary")
        );
    } else {
        print_flat(model, &summary);
    }
    ExitCode::SUCCESS
}

fn print_flat(model: &str, f: &FlatSummary) {
    let partial = if f.partial { "（partial）" } else { "" };
    println!(
        "{model}{partial}: 変数 {}（配列は 1 個）・方程式 {}・初期方程式 {}（スカラー）",
        f.variables, f.equations, f.initial_equations
    );
    if !f.top_level_connectors.is_empty() {
        println!(
            "  トップレベルのコネクタ: {}",
            f.top_level_connectors.join(", ")
        );
    }
    println!("  部品ごとの方程式:");
    for (component, n) in &f.equations_by_component {
        let component = if component.is_empty() {
            "（トップレベル）"
        } else {
            component
        };
        println!("    {n:>5}  {component}");
    }
}

/// ライブラリを読めないなど、検査の前に止まったときの出力（`check` と同じ形）。
fn report_early_failure(model: &str, diags: &[Diagnostic], json: bool) -> ExitCode {
    let stage = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.stage)
        .min()
        .unwrap_or(Stage::S1);
    if json {
        let value = serde_json::json!({
            "model": model,
            "passed": false,
            "failed_stage": stage,
            "balance": null,
            "dae": null,
            "structure": null,
            "s6": null,
            "flat": null,
            "diagnostics": diags,
            "notes": [],
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serialize report")
        );
    } else {
        print_diagnostics(diags, &HashMap::new(), false);
        println!("{model}: NG（{stage:?} {} で停止）", stage.title());
    }
    ExitCode::from(1)
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
                let at = texts
                    .get(f)
                    .or_else(|| texts.get(&normalize_path(f)))
                    .map(|t| line_col(t, r.start));
                at.map_or_else(
                    || format!("{f}: "),
                    |at| format!("{f}:{}:{}: ", at.line, at.column),
                )
            }
            (Some(f), None) => format!("{f}: "),
            _ => String::new(),
        };
        // 「コンパイラが未対応」（GM-U）は、モデルの誤りと見分けがつくように別の名前で出す
        let severity = if d.is_unsupported() {
            "unsupported".to_string()
        } else {
            serde_json::to_value(d.severity)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        };
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

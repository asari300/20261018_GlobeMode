//! GlobeMode の CLI `globe`。
//!
//! ```text
//! globe outline <FILE> [--json]
//! globe check [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe flat  [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe graph <FILE|DIR>... --model <NAME> [--depth N] [--json]
//! globe deps  <FILE|DIR>... --model <NAME> [--json]
//! globe simulate [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--stop-time T] [--solver NAME] [--step H]
//!                [--tolerance X] [--interval X] [--start-time T] [--timeout S] [--param NAME=VALUE]...
//!                [--output result.csv|result.mat|-] [--format csv|mat] [--list-parameters] [--force] [--json]
//! globe simulate --list-solvers [--json]
//! ```
//!
//! `check` と `flat` は salsa の DB（`globe-mode-db`）を通す。ライブラリは一度だけ読む。
//! Rumoca に渡すのは、モデルが使うファイル（`globe deps` が表示する閉包）だけ。
//!
//! `simulate` は検査（S0〜S10）を通してから実行する（Error があれば `--force` が無い限り実行しない）。
//!
//! 終了コード: 0 = 成功 / 1 = Error の診断がある / 2 = 入力の読み込みに失敗。

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use globe_mode_core::{Diagnostic, Stage, line_col};
use globe_mode_db::{ExperimentVerdict, FullReport, GlobeDb, normalize_path};
use globe_mode_frontend::{Frontend, RumocaFrontend, Source};
use globe_mode_ir::{ClassOutline, FlatSummary, Outline, SimSettings};

mod simulate;

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
    /// モデルを DAE まで変換して検査する（S0〜S8）
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
    /// 検査に渡すファイル（モデルが使うファイルの閉包）を表示する
    Deps {
        /// ファイルまたはディレクトリ（ディレクトリは下の .mo をすべて読む）
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// モデルの完全修飾名
        #[arg(long, short)]
        model: String,
        /// JSON（パスの配列）で出力する
        #[arg(long)]
        json: bool,
    },
    /// 検査（S0〜S10）を通してから実行し、結果を CSV か MAT v4 に書く（S011、S012）
    Simulate {
        /// ファイルまたはディレクトリ。モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present_any = ["library", "list_solvers"])]
        files: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
        /// モデルの完全修飾名（--list-solvers だけなら要らない）
        #[arg(long, short, required_unless_present = "list_solvers")]
        model: Option<String>,
        /// 開始時刻（省略すると experiment annotation、無ければ 0）
        #[arg(long)]
        start_time: Option<f64>,
        /// 終了時刻（省略すると experiment annotation、無ければ開始 + 1）
        #[arg(long)]
        stop_time: Option<f64>,
        /// 許容誤差（相対と絶対の両方。省略すると experiment annotation、無ければ 1e-6。下限 1e-10）
        #[arg(long)]
        tolerance: Option<f64>,
        /// 出力の間隔（省略すると experiment annotation、無ければ区間の 1/500）
        #[arg(long)]
        interval: Option<f64>,
        /// ソルバ（bdf・dopri45・auto・esdirk34・trbdf2・tsit45・euler・heun・rk4・impeuler・trapezoid・esdirk23・esdirk45・sdirk34hw・dopri853。別名 dassl → bdf など。
        /// 一覧は --list-solvers）
        #[arg(long)]
        solver: Option<String>,
        /// 固定刻みのソルバ（euler・heun・rk4・impeuler・trapezoid）の刻み幅（省略すると出力の間隔）
        #[arg(long)]
        step: Option<f64>,
        /// パラメータの上書き `名前=値`。何度でも指定できる（変えられるものは --list-parameters）
        #[arg(long = "param", value_name = "NAME=VALUE")]
        params: Vec<String>,
        /// 壁時計の予算（秒）。初期化と積分器の 1 歩の内側では止まらない
        #[arg(long)]
        timeout: Option<f64>,
        /// 結果の書き先（`-` は標準出力）。拡張子 `.mat` なら MAT v4、それ以外は CSV
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// 結果の形式（拡張子より優先する）
        #[arg(long, value_enum)]
        format: Option<simulate::OutputFormat>,
        /// 再コンパイル無しに変えられるパラメータの一覧を表示する（実行しない）
        #[arg(long)]
        list_parameters: bool,
        /// ソルバの一覧を表示する（実行しない。モデルは要らない）
        #[arg(long)]
        list_solvers: bool,
        /// S0〜S10 に Error があっても実行する
        #[arg(long)]
        force: bool,
        /// JSON で出力する（時系列は含めない。結果は --output）
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
        Command::Deps {
            inputs,
            model,
            json,
        } => deps(&inputs, &model, json),
        Command::Flat {
            files,
            library,
            model,
            json,
        } => flat(&files, &library, &model, json),
        Command::Simulate {
            files,
            library,
            model,
            start_time,
            stop_time,
            tolerance,
            interval,
            solver,
            step,
            params,
            timeout,
            output,
            format,
            list_parameters,
            list_solvers,
            force,
            json,
        } => {
            if list_solvers {
                return simulate::list_solvers(json);
            }
            let parameters = match simulate::parse_params(&params) {
                Ok(p) => p,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(2);
                }
            };
            simulate::simulate(simulate::SimulateArgs {
                files,
                library,
                model: model.expect("clap が --model を求める"),
                settings: SimSettings {
                    start_time,
                    stop_time,
                    tolerance,
                    interval,
                    solver,
                    max_wall_seconds: timeout,
                    step,
                    parameters,
                },
                format: simulate::OutputFormat::resolve(format, output.as_deref()),
                output,
                list_parameters,
                force,
                json,
            })
        }
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
            "s7": report.index,
            "s8": report.init,
            "s9": report.loops,
            "s10": report.preflight,
            // 位置は元の本文の中に戻す（check の結果のままでは、正規化した本文の中の位置）
            "init": check.init.as_ref().map(|i| db.init_in_original(i)),
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
        print_s7(&report);
        print_s8(&report);
        print_s9(&report);
        print_s10(&report);
        match report.failed_stage() {
            None if report.preflight.is_some() && report.loops.is_some() => {
                println!("{}: OK（S0〜S10 を通過）", report.model)
            }
            None if report.init.is_some() => {
                println!("{}: OK（S0〜S8 を通過）", report.model)
            }
            None if report.index.is_some() => println!(
                "{}: OK（S0〜S7 を通過。S8 は判定できなかった）",
                report.model
            ),
            None if report.structural.is_some() => println!(
                "{}: OK（S0〜S6 を通過。S7 以降は判定できなかった）",
                report.model
            ),
            None => println!(
                "{}: OK（S0〜S5 を通過。S6 以降は判定できなかった）",
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

/// S7 の行（構造インデックスの見込みと、状態選択の候補）。
fn print_s7(report: &FullReport) {
    let Some(s7) = &report.index else {
        return;
    };
    if s7.failed {
        println!(
            "{}: S7: Pantelides 法を打ち切った（構造インデックスを決められない）",
            report.model
        );
        return;
    }
    if s7.index <= 1 {
        println!(
            "{}: S7: 構造インデックス {}（微分なし）",
            report.model, s7.index
        );
        return;
    }
    let max = s7.differentiations.iter().max().copied().unwrap_or(0);
    // 状態選択の候補は、GM-I7002 と同じく「動的な状態 < 状態の数」のときだけ出す
    let unknowns = report.check.structure.as_ref().map(|s| &s.unknowns);
    let states = unknowns.map_or(0, |u| {
        u.iter()
            .filter(|u| u.kind == globe_mode_ir::UnknownKind::Derivative)
            .count()
    });
    let selection = if s7.dynamic_states < states {
        let names: Vec<String> = s7
            .dummy_candidates
            .iter()
            .map(|&i| {
                unknowns
                    .and_then(|u| u.get(i))
                    .map_or_else(|| format!("#{i}"), |u| state_name(&u.name).to_string())
            })
            .collect();
        format!(
            "。動的な状態 {} / {states}、状態選択の候補: {}",
            s7.dynamic_states,
            names.join(", ")
        )
    } else {
        String::new()
    };
    println!(
        "{}: S7: 構造インデックス {} の見込み（微分した方程式 {} 本、最大 {} 回）{selection}",
        report.model, s7.index, s7.n_differentiated, max
    );
}

/// `der(x[2])` → `x[2]`（状態選択の候補は状態の名前で出す）。
fn state_name(unknown: &str) -> &str {
    unknown
        .strip_prefix("der(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(unknown)
}

/// S8 の行（初期条件の数と、過剰・不足）。
fn print_s8(report: &FullReport) {
    let Some(s8) = &report.init else {
        return;
    };
    let init = report.check.init.as_ref();
    let initial_equations = init.map_or(0, |i| i.initial_equations.len());
    let states = init.map_or(0, |i| i.states.len());
    let discretes = init.map_or(0, |i| i.discretes.len());
    // fixed でない離散変数の分の不足は確定しない（analysis::init の柔らかい警告と同じ条件）
    let open_discretes = init.map_or(0, |i| i.discretes.iter().filter(|d| !d.fixed).count());
    let fixed_discretes = discretes - open_discretes;
    let fixed_algebraics = init.map_or(0, |i| i.algebraics.iter().filter(|a| a.fixed).count());
    // 過剰が確定するのは、離散変数が無く、インデックスの低減で状態が減っていないときだけ（analysis::init と同じ条件）
    let index_reduced = report
        .index
        .as_ref()
        .is_some_and(|s7| !s7.failed && s7.dynamic_states < states);
    let balance = match s8.provided.cmp(&s8.required) {
        _ if s8.matching.is_some() => String::new(),
        std::cmp::Ordering::Equal => String::new(),
        std::cmp::Ordering::Greater if discretes > 0 || index_reduced => format!(
            "。{} 個多い見込み（離散変数かインデックスの低減のため確定できない）",
            s8.provided - s8.required
        ),
        std::cmp::Ordering::Greater => {
            format!("。{} 個多い（過剰）", s8.provided - s8.required)
        }
        std::cmp::Ordering::Less if s8.required - s8.provided <= open_discretes => format!(
            "。{} 個足りない見込み（離散変数の pre の初期値に start を使えば足りる）",
            s8.required - s8.provided
        ),
        std::cmp::Ordering::Less => format!(
            "。{} 個足りない（start 値を推定値として使う見込み）",
            s8.required - s8.provided
        ),
    };
    // 要る数は「動的な状態（S7）+ fixed = false のパラメータ」なので、S7 の数を添える
    let dynamic = report
        .index
        .as_ref()
        .filter(|s7| !s7.failed)
        .map_or_else(String::new, |s7| {
            format!("動的な状態 {} / ", s7.dynamic_states)
        });
    println!(
        "{}: S8: 初期条件 {} / {}（{dynamic}状態 {states}・離散 {discretes}・初期方程式 {}・fixed = true の状態 {}・離散 {fixed_discretes}・代数 {fixed_algebraics}・fixed = false のパラメータ {}）{balance}",
        report.model,
        s8.provided,
        s8.required,
        initial_equations,
        s8.fixed_states.len(),
        s8.free_parameters.len()
    );
    // S009: 初期化系のマッチング（冗長な初期条件と、初期条件の無い未知数）
    if let Some(m) = &s8.matching {
        let verdict = match (m.extra, m.missing) {
            (0, 0) => "合う".to_string(),
            (e, 0) => format!("冗長 {e}"),
            (0, u) => format!("決まらない {u}"),
            (e, u) => format!("冗長 {e}・決まらない {u}"),
        };
        println!(
            "{}: S8: 初期化系のマッチング: 方程式 {}（DAE とその微分、初期方程式、fixed = true）/ 未知数 {}、マッチング {}。{verdict}{}",
            report.model,
            m.n_equations,
            m.n_unknowns,
            m.matching,
            if m.extra > 0 && m.over_with_dae {
                "（冗長な初期条件は拘束と重なる。整合する値なら冗長なだけ）"
            } else {
                ""
            }
        );
    } else if report.check.init.as_ref().is_some_and(|i| !i.has_incidence) {
        println!(
            "{}: S8: 初期方程式の参照を取り出せなかったので、数と属性だけで判定した",
            report.model
        );
    }
}

/// S9 の行（代数ループの数と大きさ、非線形のループの反復変数）。
fn print_s9(report: &FullReport) {
    let Some(s9) = &report.loops else {
        return;
    };
    if let Some(reason) = &s9.skipped {
        println!("{}: S9: 代数ループを解析しなかった: {reason}", report.model);
        return;
    }
    let reduced = if s9.reduced {
        "。インデックス低減の最高階の系で数えた（状態選択の前の見込み）"
    } else {
        ""
    };
    if s9.n_loops() == 0 {
        println!(
            "{}: S9: 代数ループなし（BLT のブロック {}）{reduced}",
            report.model, s9.blocks
        );
        return;
    }
    let start = if s9.start_checked {
        format!("（start 値なし {}）", s9.n_iteration_without_start())
    } else {
        String::new()
    };
    println!(
        "{}: S9: 代数ループ {}（線形 {}・非線形 {}、最大 {} 本の方程式）、BLT のブロック {}。非線形のループの反復変数 {}{start}{reduced}",
        report.model,
        s9.n_loops(),
        s9.n_linear(),
        s9.n_nonlinear(),
        s9.max_loop_size(),
        s9.blocks,
        s9.n_iteration_variables()
    );
}

/// S10 の行（評価したパラメータと start 値の数、失敗の数、experiment annotation）。
fn print_s10(report: &FullReport) {
    let Some(s10) = &report.preflight else {
        return;
    };
    let failures = s10.not_finite_parameters.len()
        + s10.assertion_parameters.len()
        + s10.not_finite_starts.len();
    let external = if s10.external_parameters.is_empty() {
        String::new()
    } else {
        format!(
            "、静的に評価できないもの {}（外部関数）",
            s10.external_parameters.len()
        )
    };
    let experiment = match &s10.experiment {
        ExperimentVerdict::Missing => "experiment annotation なし".to_string(),
        ExperimentVerdict::Checked {
            invalid,
            suspicious,
        } if invalid.is_empty() && suspicious.is_empty() => {
            "experiment annotation は妥当".to_string()
        }
        ExperimentVerdict::Checked {
            invalid,
            suspicious,
        } => format!(
            "experiment annotation の誤り {}・疑わしい値 {}",
            invalid.len(),
            suspicious.len()
        ),
    };
    println!(
        "{}: S10: パラメータ {}・start 値 {} を評価し、有限にならないか assert が失敗したもの {}{external}。{experiment}",
        report.model, s10.parameters_evaluated, s10.starts_evaluated, failures
    );
}

/// `globe deps`: 検査に渡すファイル（閉包）と、ワークスペースの全ファイル数。
fn deps(inputs: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let files = match expand_inputs(inputs) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let mut db = GlobeDb::new();
    for f in &files {
        match read(f) {
            Ok((name, text)) => db.set_file(&name, text),
            Err(code) => return code,
        }
    }
    let closure = db.deps_closure(model);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&closure).expect("serialize closure")
        );
        return ExitCode::SUCCESS;
    }
    for path in &closure {
        println!("{path}");
    }
    let total = db.paths().count();
    if closure.is_empty() {
        println!(
            "{model}: 0 / {total} ファイル（モデルがワークスペースに無い。ライブラリの中のモデルか、存在しないモデル）"
        );
    } else {
        println!("{model}: {} / {total} ファイル", closure.len());
    }
    ExitCode::SUCCESS
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
            "s7": null,
            "s8": null,
            "s9": null,
            "s10": null,
            "init": null,
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

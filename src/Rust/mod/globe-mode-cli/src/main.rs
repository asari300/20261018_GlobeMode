//! GlobeMode の CLI `globe`。
//!
//! ```text
//! globe outline <FILE> [--json]
//! globe check [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe flat  [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--json]
//! globe graph [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--depth N] [--json]
//! globe deps  <FILE|DIR>... --model <NAME> [--json]
//! globe simulate [<FILE|DIR>...] [--library <DIR>]... --model <NAME> [--stop-time T] [--solver NAME] [--step H]
//!                [--tolerance X] [--interval X] [--start-time T] [--timeout S] [--param NAME=VALUE]...
//!                [--output result.csv|result.mat|-] [--format csv|mat] [--list-parameters] [--force] [--json]
//! globe simulate --list-solvers [--json]
//! globe lsp [--library <DIR>]...
//! ```
//!
//! すべて facade（`globe-mode` の `Workspace`）を通す（S035）。`--json` の形は `globe-mode-protocol` の型。
//! ライブラリは一度だけ読む。
//! Rumoca に渡すのは、モデルが使うファイル（`globe deps` が表示する閉包）だけ。
//!
//! `simulate` は検査（S0〜S10）を通してから実行する（Error があれば `--force` が無い限り実行しない）。
//!
//! 終了コード: 0 = 成功 / 1 = Error の診断がある・整形が要る・lint の指摘がある / 2 = 使い方の誤り・入力の読み込みに失敗・
//! 書き出しに失敗 / 3 = 未対応（GM-U）だけで止まった（`check`・`simulate`。S199）。
//! 標準出力が閉じられたとき（`| head`）は、以後の出力を黙って捨てて処理を最後まで進め、本来の終了コードで終わる（S182・S199）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};
use globe_mode::db::normalize_path;
use globe_mode::{CheckResponse, Workspace};
use globe_mode_core::{Diagnostic, Severity, line_col};
use globe_mode_ir::{ClassOutline, FlatSummary, Outline, SimSettings};

/// 標準出力へ書く。閉じられていたら（`globe … | head`）以後の出力を黙って捨て、処理は最後まで進めて本来の終了コードで終わる（S199）。
/// Rust は SIGPIPE を無視するので標準の `println!` は panic し、`unsafe` は禁止なのでシグナルは戻せない。
macro_rules! println {
    ($($arg:tt)*) => { $crate::stdout_ok(|out| ::std::io::Write::write_fmt(
        out, format_args!("{}\n", format_args!($($arg)*)))) };
}
macro_rules! print {
    ($($arg:tt)*) => { $crate::stdout_ok(|out| ::std::io::Write::write_fmt(
        out, format_args!($($arg)*))) };
}

/// 読み手が標準出力を閉じた（BrokenPipe）。以後の標準出力への書き込みは何もしない。
static STDOUT_CLOSED: AtomicBool = AtomicBool::new(false);

fn stdout_ok(write: impl FnOnce(&mut std::io::Stdout) -> std::io::Result<()>) {
    if STDOUT_CLOSED.load(Ordering::Relaxed) {
        return;
    }
    if let Err(e) = write(&mut std::io::stdout()) {
        stdout_failed(&e);
    }
}

/// 標準出力への書き込みの失敗。BrokenPipe は誤りではない。それ以外（ディスク満杯など）は 1 行出して終了コード 2。
fn stdout_failed(e: &std::io::Error) {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        STDOUT_CLOSED.store(true, Ordering::Relaxed);
    } else {
        eprintln!("error: 標準出力に書けない: {}", io_reason(e));
        std::process::exit(2);
    }
}

/// `-v`: info の診断も全部出す（既定は件数だけ）。
static VERBOSE: AtomicBool = AtomicBool::new(false);

mod fmu;
mod report;
mod simulate;

#[derive(Debug, Parser)]
#[command(
    name = "globe",
    version,
    about = "GlobeMode — an open-source Modelica IDE (CLI)",
    after_help = "終了コード:\n  0  成功\n  1  モデルの誤り（Error の診断）・整形が要る（fmt --check）・lint の指摘がある\n  2  使い方の誤り・入力を読めない・結果を書けない\n  3  未対応（GM-U）だけで止まった（check・simulate。モデルの誤りではなく、コンパイラが未対応）\n\n`| head` で読み手が先に閉じても、結果の終了コードは変わらない。"
)]
struct Cli {
    /// info の診断も全部表示する（既定は件数だけ）
    #[arg(long, short, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// ファイルのアウトライン（クラス・コンポーネント・connect）を表示する
    #[command(after_help = "例:\n  globe outline model.mo\n  globe outline model.mo --json")]
    Outline {
        /// アウトラインを出す .mo ファイル 1 つ（ライブラリのファイルも、そのパスを渡せば出せる。ディレクトリは受けない）
        file: PathBuf,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// connect グラフ（flatten しない）を表示する。--json は graphology の形式
    #[command(
        after_help = "例:\n  globe graph model.mo -m RC\n  globe graph src/Modelica/GlobeModeExamples -m GlobeModeExamples.RC --depth 2 --json\n  globe graph -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit"
    )]
    Graph {
        /// ファイルまたはディレクトリ（ディレクトリは下の .mo をすべて読む）。モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present = "library")]
        inputs: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
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
    /// モデルを DAE まで変換して検査する（S0〜S10）
    #[command(
        after_help = "例:\n  globe check model.mo -m RC\n  globe check -L data/msl -m Modelica.Blocks.Examples.PID_Controller\n  globe check model.mo -m RC --json\n\ninfo の診断は件数だけ出す。-v で全部表示する。--json は info も含む"
    )]
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
    /// Modelica のファイルを整形する（`doc/format-style.md`）。標準入力は `-` を明示したときだけ。
    /// 設定は対象から上へたどって最初の `globe-mode.toml`。コマンドラインの引数が優先
    #[command(
        after_help = "例:\n  globe fmt .               カレントディレクトリの下の .mo を整形する\n  globe fmt -               標準入力を整形して標準出力へ\n  globe fmt --check src     書き換えず、整形が要るファイルを列挙する（要整形なら終了コード 1）\n  globe fmt model.mo --indent 4"
    )]
    Fmt {
        /// ファイルまたはディレクトリ（下の .mo を .gitignore と隠しディレクトリを除いてすべて）。
        /// `-`（パスなしの `--json` も）は標準入力 → 標準出力。パスも `-` も無ければ使い方を出して終了コード 2
        files: Vec<PathBuf>,
        /// 書き換えず、整形が要るファイルを列挙して終了コード 1
        #[arg(long)]
        check: bool,
        /// 1 段のインデントの空白の数（既定 2。`globe-mode.toml` の `[fmt]` より優先）
        #[arg(long)]
        indent: Option<usize>,
        /// 行幅（既定 80）
        #[arg(long)]
        width: Option<usize>,
        /// 図形プリミティブに `origin = {0, 0}` を足さない
        #[arg(long)]
        no_insert_origin: bool,
        /// `equation` / `algorithm` の前後に空行を足さない
        #[arg(long)]
        no_blank_around_sections: bool,
        /// `Documentation(…)` の HTML の文字列を整えない
        #[arg(long)]
        no_format_html: bool,
        /// 標準入力のとき、結果を JSON（{"ok":bool,"text":…,"error":…}）で返す
        #[arg(long)]
        json: bool,
    },
    /// Modelica のファイルを lint する（line-too-long / class-name-pascal-case / missing-description）
    #[command(
        after_help = "例:\n  globe lint .\n  globe lint -               標準入力を lint する\n  globe lint model.mo --disable line-too-long\n  globe lint src --json"
    )]
    Lint {
        /// ファイルまたはディレクトリ。`-` は標準入力（`fmt` と同じ規則）
        files: Vec<PathBuf>,
        /// 止める規則（何度でも。`globe-mode.toml` の `[lint] disable` に足す）
        #[arg(long)]
        disable: Vec<String>,
        /// JSON（{"messages":[{file,rule,line,column,message}]}）で出力する
        #[arg(long)]
        json: bool,
    },
    /// LSP サーバ（標準入出力。診断・documentSymbol・hover・整形・補完）
    #[command(after_help = "例:\n  globe lsp -L data/msl")]
    Lsp {
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
    },
    /// 検査に渡すファイル（モデルが使うファイルの閉包）を表示する
    #[command(
        after_help = "例:\n  globe deps src/Modelica/GlobeModeExamples -m GlobeModeExamples.RC\n  globe deps -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit\n  globe deps model.mo -L data/msl -m RC"
    )]
    Deps {
        /// ファイルまたはディレクトリ（ディレクトリは下の .mo をすべて読む）。モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present = "library")]
        inputs: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる。ライブラリのファイルも閉包に含める
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
        /// モデルの完全修飾名
        #[arg(long, short)]
        model: String,
        /// JSON（パスの配列）で出力する
        #[arg(long)]
        json: bool,
    },
    /// 検査（S0〜S10）を通してから実行し、結果を CSV か MAT v4 に書く
    #[command(
        allow_negative_numbers = true,
        after_help = "例:\n  globe simulate model.mo -m RC -o rc.csv          結果を CSV に書く（.mat なら MAT v4、`-` は標準出力）\n  globe simulate -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit --stop-time 5 -o chua.csv\n  globe simulate model.mo -m RC --solver rk4 --step 0.01 --param R=200 -o rc.csv\n  globe simulate --list-solvers\n\n-o を省くと結果は書かず、要約だけを出す。info の診断は件数だけ出す（-v で全部）"
    )]
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
        /// ソルバ（bdf・dopri45・auto・esdirk34・trbdf2・tsit45・euler・heun・rk4・impeuler・trapezoid・esdirk23・esdirk45・sdirk34hw・dopri853・cerk23・cerk34・cerk45・radau5・qss・cvode・ida（feature `sundials` のとき）。別名 dassl → bdf など。
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
    /// FMU（`export` で書き出し、`info` と `run` で読む）
    Fmu {
        #[command(subcommand)]
        command: FmuCommand,
    },
    /// flat モデルの要約（変数・方程式の数、部品ごとの方程式数）を表示する
    #[command(
        after_help = "例:\n  globe flat model.mo -m RC\n  globe flat -L data/msl -m Modelica.Blocks.Examples.PID_Controller --json"
    )]
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

#[derive(Debug, Subcommand)]
enum FmuCommand {
    /// S204: FMU を読む（`info`・`run`）
    #[command(flatten)]
    Import(fmu::FmuImportCommand),
    /// FMI 2.0 / 3.0 のソースの FMU（ME と CS）を書き出す。中身は Rumoca の `fmi2` / `fmi3` のテンプレート
    #[command(
        after_help = "例:\n  globe fmu export model.mo -m RC --out build\n  globe fmu export model.mo -m RC --out build --fmi-version 3\n  globe fmu export model.mo -m RC --out build --build --fmi-headers <fmi2Functions.h のあるディレクトリ>"
    )]
    Export {
        /// ファイルまたはディレクトリ。モデルがライブラリの中にあるなら省略できる
        #[arg(required_unless_present = "library")]
        files: Vec<PathBuf>,
        /// ライブラリのディレクトリ（例: data/msl）。何度でも指定できる
        #[arg(long, short = 'L')]
        library: Vec<PathBuf>,
        /// モデルの完全修飾名
        #[arg(long, short)]
        model: String,
        /// 書き先のディレクトリ（`<名前>/` と `<名前>-fmi2.fmu`（3.0 は `-fmi3`）を置く）
        #[arg(long)]
        out: PathBuf,
        /// FMI の版（2 か 3）
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(2..=3))]
        fmi_version: u8,
        /// `cc` で `binaries/linux64/<modelIdentifier>.so`（3.0 は `binaries/x86_64-linux/`）も作る（x86_64 の Linux だけ）
        #[arg(long, requires = "fmi_headers")]
        build: bool,
        /// --build で `zig cc -target <triple>` を使い、その置き場に作る（何度でも。例: x86_64-windows-gnu → win64/*.dll、
        /// aarch64-macos → darwin64/*.dylib（3.0 は aarch64-darwin）、x86_64-linux-gnu → linux64/*.so）。省略すると host を cc で
        #[arg(long, requires = "build")]
        target: Vec<String>,
        /// `fmi2Functions.h`（3.0 は `fmi3Functions.h`）などのあるディレクトリ（--build で使う。例: FMPy の `c-code`）
        #[arg(long)]
        fmi_headers: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::try_parse().unwrap_or_else(|e| {
        let missing_files = e.kind() == clap::error::ErrorKind::MissingRequiredArgument
            && (e.to_string().contains("<FILES>") || e.to_string().contains("<INPUTS>"));
        if missing_files {
            // S180: ライブラリのモデルなら FILES の代わりに -L と -m で足りる
            let _ = e.print();
            eprintln!(
                "\nライブラリのモデルなら FILES は要らない。-L にライブラリのルートを、-m に完全修飾名を渡す:\n  \
                 globe check -L data/msl -m Modelica.Blocks.Examples.PID_Controller"
            );
            std::process::exit(2);
        }
        e.exit()
    });
    VERBOSE.store(cli.verbose, Ordering::Relaxed);
    match cli.command {
        Command::Outline { file, json } => outline(&file, json),
        Command::Graph {
            inputs,
            library,
            model,
            depth,
            json,
        } => graph(&inputs, &library, &model, depth, json),
        Command::Check {
            files,
            library,
            model,
            json,
        } => check(&files, &library, &model, json),
        Command::Fmt {
            files,
            check,
            indent,
            width,
            no_insert_origin,
            no_blank_around_sections,
            no_format_html,
            json,
        } => fmt(
            &files,
            check,
            &|o: &mut globe_mode_fmt::Options| {
                o.indent = indent.unwrap_or(o.indent);
                o.width = width.unwrap_or(o.width);
                o.insert_origin &= !no_insert_origin;
                o.blank_around_sections &= !no_blank_around_sections;
                o.format_html &= !no_format_html;
            },
            json,
        ),
        Command::Lint {
            files,
            disable,
            json,
        } => lint(&files, &disable, json),
        Command::Lsp { library } => match globe_mode_lsp::run_stdio(&library) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::Deps {
            inputs,
            library,
            model,
            json,
        } => deps(&inputs, &library, &model, json),
        Command::Flat {
            files,
            library,
            model,
            json,
        } => flat(&files, &library, &model, json),
        Command::Fmu {
            command:
                FmuCommand::Export {
                    files,
                    library,
                    model,
                    out,
                    fmi_version,
                    build,
                    target,
                    fmi_headers,
                },
        } => fmu_export(
            &files,
            &library,
            &model,
            &out,
            &globe_mode::FmuOptions {
                fmi_version,
                build,
                targets: target,
                fmi_headers,
            },
        ),
        Command::Fmu {
            command: FmuCommand::Import(command),
        } => fmu::run(command),
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
                eprintln!("error: {}", io_error(input, false, &e));
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

fn read_stdin() -> Result<String, ExitCode> {
    let mut text = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut text).map_err(|e| {
        eprintln!("error: 標準入力を読めない: {}", io_reason(&e));
        ExitCode::from(2)
    })?;
    Ok(text)
}

/// 入出力の誤りの理由（`os error 2` のような素の表示は出さない）。
fn io_reason(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "見つからない".to_string(),
        PermissionDenied => "権限がない".to_string(),
        IsADirectory => "ディレクトリである".to_string(),
        NotADirectory => "途中のパスがディレクトリではない".to_string(),
        InvalidData => "UTF-8 の文字列として読めない".to_string(),
        _ => {
            let text = e.to_string();
            text.split(" (os error").next().unwrap_or(&text).to_string()
        }
    }
}

/// 「<パス> を読めない: 見つからない」「<パス> に書けない: …」の形にそろえる。
fn io_error(path: &Path, writing: bool, e: &std::io::Error) -> String {
    let op = if writing {
        "に書けない"
    } else {
        "を読めない"
    };
    format!("{} {op}: {}", path.display(), io_reason(e))
}

/// 結果の書き出し先（ファイル）を、計算の前に確かめる。無ければ作って消す。既にあるものは変えない（追記で開くだけ）。
/// 入力のファイル・ライブラリの配下・拡張子 `.mo` は、ソースを結果で上書きしてしまうので断る（S199）。
fn check_writable_file(path: &Path, files: &[PathBuf], library: &[PathBuf]) -> Result<(), String> {
    if path.is_dir() {
        return Err(format!("{} に書けない: ディレクトリである", path.display()));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
        && !parent.is_dir()
    {
        return Err(format!(
            "{} に書けない: 親のフォルダ {} が無い",
            path.display(),
            parent.display()
        ));
    }
    // リンクを辿らずに調べる（壊れた symlink は exists() が false になり、消してしまう）
    let existed = path.symlink_metadata().is_ok();
    if existed && path.metadata().is_err() {
        return Err(format!(
            "{} に書けない: リンク先が無い symlink である",
            path.display()
        ));
    }
    let is_mo = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mo"));
    let resolved = resolve(path);
    if is_mo(path) || is_mo(&resolved) {
        return Err(format!(
            "{} に書けない: Modelica のソース（.mo）を結果で上書きしてしまう",
            path.display()
        ));
    }
    if files.iter().any(|f| f.is_file() && resolve(f) == resolved)
        || library.iter().any(|l| resolved.starts_with(resolve(l)))
    {
        return Err(format!(
            "{} に書けない: 入力のファイルかライブラリの中のファイルを上書きしてしまう",
            path.display()
        ));
    }
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| io_error(path, true, &e))?;
    if !existed {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

/// `dir` の下に `.mo` があるか（見つけたらそこで止める）。
fn contains_mo(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            let p = e.path();
            // symlink は辿らない（輪になっていても止まる）
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                contains_mo(&p)
            } else {
                p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mo"))
            }
        })
    })
}

/// リンクを辿った絶対パス。まだ無いファイルは、親を辿ってファイル名を足す。
fn resolve(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    match (parent.map(Path::canonicalize), path.file_name()) {
        (Some(Ok(parent)), Some(name)) => parent.join(name),
        (None, Some(name)) => std::env::current_dir()
            .and_then(|d| d.canonicalize())
            .map_or_else(|_| name.into(), |d| d.join(name)),
        _ => path.to_path_buf(),
    }
}

/// 書き出し先のディレクトリを、計算の前に作る（無ければ親ごと。ファイルを指していたら誤り）。
/// FMU は `<out>/<名前>/` を作り直す（あれば消す）ので、そこに `.mo` か入力・ライブラリがあるなら断る（S199）。
fn prepare_out_dir(
    dir: &Path,
    model: &str,
    files: &[PathBuf],
    library: &[PathBuf],
) -> Result<(), String> {
    if dir.exists() && !dir.is_dir() {
        return Err(format!(
            "{} に書けない: ディレクトリではない",
            dir.display()
        ));
    }
    let root = resolve(&dir.join(model.replace('.', "_")));
    if root.exists()
        && (contains_mo(&root)
            || files
                .iter()
                .chain(library)
                .any(|s| resolve(s).starts_with(&root)))
    {
        return Err(format!(
            "{} に書けない: 作り直す {} に Modelica のソースか入力が入っている",
            dir.display(),
            root.display()
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| io_error(dir, true, &e))
}

/// S130: `path` から上へたどって最初の `globe-mode.toml`（無ければ既定）。読めない・誤りがあれば終了コード 2。
fn config(path: &Path) -> Result<globe_mode_fmt::LintOptions, ExitCode> {
    globe_mode_fmt::LintOptions::find(path)
        .map(Option::unwrap_or_default)
        .map_err(|e| {
            eprintln!("error: {e}");
            ExitCode::from(2)
        })
}

/// `globe fmt`。ファイルが無ければ標準入力 → 標準出力。`cli` はコマンドラインの引数で設定ファイルの値を上書きする。
fn fmt(
    files: &[PathBuf],
    check: bool,
    cli: &dyn Fn(&mut globe_mode_fmt::Options),
    json: bool,
) -> ExitCode {
    let options_for = |path: &Path| {
        config(path).map(|c| {
            let mut o = c.format;
            cli(&mut o);
            o
        })
    };
    let files = match paths_or_stdin("fmt", files, json) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let Some(files) = files else {
        let Ok(text) = read_stdin() else {
            return ExitCode::from(2);
        };
        let options = match options_for(Path::new(".")) {
            Ok(o) => o,
            Err(code) => return code,
        };
        return match globe_mode_fmt::format(&text, &options) {
            Ok(out) if json => {
                println!(
                    "{}",
                    serde_json::json!({"ok": true, "text": out, "error": null})
                );
                ExitCode::SUCCESS
            }
            Ok(out) => {
                print!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) if json => {
                println!(
                    "{}",
                    serde_json::json!({"ok": false, "text": null, "error": e.to_string()})
                );
                ExitCode::from(1)
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        };
    };
    let Ok(paths) = collect_mo(&files) else {
        return ExitCode::from(2);
    };
    // ruff と同じく、壊れたファイルがあっても残りを続け、最後に要約と終了コード 2 を返す
    let (mut changed, mut failed) = (0, 0);
    for path in &paths {
        let options = match options_for(path) {
            Ok(o) => o,
            Err(code) => return code,
        };
        let (name, text) = match read(path) {
            Ok(x) => x,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        match globe_mode_fmt::format(&text, &options) {
            Ok(out) if out == text => {}
            Ok(_) if check => {
                changed += 1;
                println!("整形が要る: {name}");
            }
            Ok(out) => match std::fs::write(path, out) {
                Ok(()) => changed += 1,
                Err(e) => {
                    eprintln!("error: {}", io_error(path, true, &e));
                    failed += 1;
                }
            },
            Err(e) => {
                eprintln!("error: {name} を整形できない: {e}");
                failed += 1;
            }
        }
    }
    let same = paths.len() - changed - failed;
    if check {
        println!("{changed} ファイルは整形が要る、{same} ファイルは整形済み");
    } else {
        println!("{changed} ファイルを整形した、{same} ファイルは変更なし");
    }
    if failed > 0 {
        println!("{failed} ファイルは整形できなかった");
        ExitCode::from(2)
    } else if check && changed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// 入力の決め方。`None` は標準入力。標準入力は `-` を明示したとき（と、標準入力の契約の `--json`）だけ。
/// パスを 1 つも渡さず `-` も `--json` も無いときは、端末かパイプかに関係なく使い方を出して終了コード 2
/// （S199。ウェブアプリ・エディタは `--json` で呼ぶ。`globe fmt --check` が何も検査せず 0 で終わるのを防ぐ）。
fn paths_or_stdin(
    command: &str,
    files: &[PathBuf],
    json: bool,
) -> Result<Option<Vec<PathBuf>>, ExitCode> {
    match files {
        [one] if one.as_os_str() == "-" => Ok(None),
        [] if json => Ok(None),
        [] => {
            eprintln!(
                "error: 対象のパスが無い。使い方: globe {command} <FILES…>（ディレクトリも可。例: globe {command} .）か globe {command} -（標準入力）"
            );
            Err(ExitCode::from(2))
        }
        _ => Ok(Some(files.to_vec())),
    }
}

/// `fmt` / `lint` の対象。ディレクトリは ruff と同じく `.gitignore`・隠しディレクトリを除いた下の `.mo`（名前順）。
/// 明示したファイルは除外しない。
fn collect_mo(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, ExitCode> {
    let mut out = Vec::new();
    for input in inputs {
        if !input.is_dir() {
            out.push(input.clone());
            continue;
        }
        let mut found = Vec::new();
        // require_git(false): git の外でも .gitignore を守る（ruff と同じ）
        for entry in ignore::WalkBuilder::new(input).require_git(false).build() {
            match entry {
                Ok(e) => {
                    if e.file_type().is_some_and(|t| t.is_file())
                        && e.path().extension().is_some_and(|x| x == "mo")
                    {
                        found.push(e.into_path());
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    return Err(ExitCode::from(2));
                }
            }
        }
        found.sort();
        out.extend(found);
    }
    Ok(out)
}

/// `globe lint`。1 件以上あれば終了コード 1。
fn lint(files: &[PathBuf], disable: &[String], json: bool) -> ExitCode {
    // S130: ファイルごとに設定ファイルの `[lint] disable` にコマンドラインの `--disable` を足す
    let options_for = |path: &Path| {
        config(path).map(|mut c| {
            c.disabled.extend_from_slice(disable);
            c
        })
    };
    let files = match paths_or_stdin("lint", files, json) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let inputs: Vec<(String, String, globe_mode_fmt::LintOptions)> = if let Some(files) = files {
        let Ok(paths) = collect_mo(&files) else {
            return ExitCode::from(2);
        };
        let mut v = Vec::new();
        for p in &paths {
            let (Ok(options), Ok((name, text))) = (options_for(p), read(p)) else {
                return ExitCode::from(2);
            };
            v.push((name, text, options));
        }
        v
    } else {
        let (Ok(options), Ok(text)) = (options_for(Path::new(".")), read_stdin()) else {
            return ExitCode::from(2);
        };
        vec![("<stdin>".to_string(), text, options)]
    };
    // S062: 渡したファイルをまとめて 1 つのワークスペースにし、connect の型を突き合わせる
    let check_types =
        |o: &globe_mode_fmt::LintOptions| !o.disabled.iter().any(|d| d == "connect-type-mismatch");
    let mut ws = Workspace::new();
    if inputs.iter().any(|(_, _, o)| check_types(o)) {
        for (name, text, _) in &inputs {
            ws.set_file(name, text.clone());
        }
    }
    let mut all = Vec::new();
    for (name, text, options) in &inputs {
        let mut found: Vec<(usize, usize, &str, String)> = globe_mode_fmt::lint(text, options)
            .into_iter()
            .map(|m| (m.line, m.column, m.rule, m.message))
            .collect();
        if check_types(options) {
            for m in ws.connect_mismatches(name) {
                let lc = globe_mode_core::line_col(text, m.connect.range.start);
                found.push((
                    lc.line as usize,
                    lc.column as usize,
                    "connect-type-mismatch",
                    format!(
                        "{} は {}、{} は {} で、型が合わない",
                        m.connect.lhs, m.lhs_type, m.connect.rhs, m.rhs_type
                    ),
                ));
            }
            found.sort();
        }
        for (line, column, rule, message) in found {
            if !json {
                println!("{name}:{line}:{column}: {rule}: {message}");
            }
            all.push(serde_json::json!({"file": name, "rule": rule, "line": line, "column": column, "message": message}));
        }
    }
    if json {
        println!("{}", serde_json::json!({"messages": all}));
    } else if all.is_empty() {
        println!("問題なし（{} ファイル）", inputs.len());
    } else {
        println!("{} 件（{} ファイル）", all.len(), inputs.len());
    }
    if all.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn read(path: &PathBuf) -> Result<(String, String), ExitCode> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok((path.display().to_string(), text)),
        Err(e) => {
            eprintln!("error: {}", io_error(path, false, &e));
            Err(ExitCode::from(2))
        }
    }
}

fn outline(file: &PathBuf, json: bool) -> ExitCode {
    let (name, text) = match read(file) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let mut ws = Workspace::new();
    ws.set_file(&name, text.clone());
    match ws.outline(&name).expect("足したファイルのアウトライン") {
        Ok(o) if json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&o).expect("serialize outline")
            );
            ExitCode::SUCCESS
        }
        Ok(o) => {
            print_outline(o, &text);
            ExitCode::SUCCESS
        }
        Err(diags) => {
            let texts = HashMap::from([(name, text)]);
            print_diagnostics(diags, &texts, json);
            ExitCode::from(1)
        }
    }
}

/// 入力を読み込んだ DB と、診断の位置を行と列に直すための本文
/// （キーは DB と同じく区切りを `/` にそろえたパス）。
struct Loaded {
    ws: Workspace,
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
    let mut ws = match Workspace::with_libraries(library) {
        Ok(ws) => ws,
        Err(diags) => return Ok(Err(diags)),
    };
    let mut texts = HashMap::new();
    for (name, text) in inputs {
        ws.set_file(&name, text.clone());
        texts.insert(normalize_path(&name), text);
    }
    Ok(Ok(Loaded { ws, texts }))
}

fn check(files: &[PathBuf], library: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let Loaded { mut ws, texts } = match load_db(files, library) {
        Ok(Ok(v)) => v,
        // ライブラリを読めない（S1）
        Ok(Err(diags)) => return report_early_failure(model, &diags, json, false),
        Err(code) => return code,
    };
    let (passed, diagnostics) = if json {
        let response = ws.check(model);
        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("serialize report")
        );
        (response.passed, response.diagnostics)
    } else {
        // 文字の表示は DAE の未知数や初期化系の構造まで読むので、protocol の型でなく DB の報告を使う
        let report = ws.db().check_model(model);
        let check = &report.check;
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
        report::print_s7(&report);
        report::print_s8(&report);
        report::print_s9(&report);
        report::print_s10(&report);
        // 最後の 1 行は facade の応答の `summary`（Python・LSP・wasm と同じ。検査は salsa が覚えている）
        println!("{}", ws.check(model).summary);
        (report.passed(), report.diagnostics)
    };
    if passed {
        ExitCode::SUCCESS
    } else {
        failure_code(&diagnostics)
    }
}

/// `globe deps`: 検査に渡すファイル（閉包）と、ワークスペースの全ファイル数。
fn deps(inputs: &[PathBuf], library: &[PathBuf], model: &str, json: bool) -> ExitCode {
    // 閉包にライブラリのファイルも入れるので、ライブラリは Rumoca に読ませず、ワークスペースのファイルとして読む
    let all: Vec<PathBuf> = inputs.iter().chain(library).cloned().collect();
    let files = match expand_inputs(&all) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let mut ws = Workspace::new();
    for f in &files {
        match read(f) {
            Ok((name, text)) => ws.set_file(&name, text),
            Err(code) => return code,
        }
    }
    let closure = ws.deps(model);
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
    let total = ws.files().count();
    if closure.is_empty() {
        println!(
            "{model}: 0 / {total} ファイル（モデルが読んだファイルの中に無い。ライブラリの中のモデルなら -L でライブラリを渡す）"
        );
    } else {
        println!("{model}: {} / {total} ファイル", closure.len());
    }
    ExitCode::SUCCESS
}

fn flat(files: &[PathBuf], library: &[PathBuf], model: &str, json: bool) -> ExitCode {
    let Loaded { mut ws, texts } = match load_db(files, library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, json, false),
        Err(code) => return code,
    };
    let Some(summary) = ws.flat(model) else {
        // flatten まで通らなかった。検査の診断を出す
        let report = ws.check(model);
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

fn fmu_export(
    files: &[PathBuf],
    library: &[PathBuf],
    model: &str,
    out: &std::path::Path,
    options: &globe_mode::FmuOptions,
) -> ExitCode {
    if let Err(message) = prepare_out_dir(out, model, files, library) {
        eprintln!("error: --out {message}");
        return ExitCode::from(2);
    }
    let Loaded { mut ws, texts } = match load_db(files, library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, false, false),
        Err(code) => return code,
    };
    let compiled = match ws.compile(model, &Default::default()) {
        Ok(c) => c,
        Err(diags) => {
            print_diagnostics(&diags, &texts, false);
            println!("{model}: DAE まで変換できないので FMU を作れない");
            return ExitCode::from(1);
        }
    };
    match globe_mode::export_fmu(&compiled, out, options) {
        Ok(fmu) => {
            println!("{}", fmu.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
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

/// ライブラリを読めないなど、検査の前に止まったときの出力（`check` と同じ形）。`to_stderr` は `simulate -o -` のとき。
fn report_early_failure(
    model: &str,
    diags: &[Diagnostic],
    json: bool,
    to_stderr: bool,
) -> ExitCode {
    let response = CheckResponse::failed(model, diags.to_vec());
    if json {
        let text = serde_json::to_string_pretty(&response).expect("serialize report");
        emit(to_stderr, &text);
    } else {
        for line in diagnostic_lines(diags, &HashMap::new()) {
            emit(to_stderr, &line);
        }
        emit(to_stderr, &response.summary);
    }
    failure_code(diags)
}

/// 失敗の終了コード。Error がすべて未対応（GM-U）なら 3、そうでなければ 1（S199。分類は protocol の `unsupported_summary`）。
fn failure_code(diagnostics: &[Diagnostic]) -> ExitCode {
    if globe_mode::unsupported_summary(diagnostics).is_some() {
        ExitCode::from(3)
    } else {
        ExitCode::from(1)
    }
}

fn graph(inputs: &[PathBuf], library: &[PathBuf], model: &str, depth: u32, json: bool) -> ExitCode {
    let Loaded { mut ws, .. } = match load_db(inputs, library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, json, false),
        Err(code) => return code,
    };
    match ws.graph(model, depth) {
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

/// 診断の表示行（`ファイル:行:列: severity[コード] …` と補足。info は件数だけ、`-v` で全部）。
fn diagnostic_lines(diags: &[Diagnostic], texts: &HashMap<String, String>) -> Vec<String> {
    let (diags, hidden) = shown(diags);
    let mut lines = Vec::new();
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
        lines.push(format!(
            "{place}{severity}[{}] {:?} {}: {}",
            d.code,
            d.stage,
            d.stage.title(),
            d.message
        ));
        lines.extend(d.notes.iter().map(|n| format!("    note: {n}")));
    }
    lines.extend(hidden_text(&hidden));
    lines
}

/// 1 行を標準出力か標準エラーに出す（`simulate -o -` は CSV が標準出力を使うので、ほかは標準エラー）。
fn emit(to_stderr: bool, line: &str) {
    if to_stderr {
        eprintln!("{line}");
    } else {
        println!("{line}");
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
    for line in diagnostic_lines(diags, texts) {
        println!("{line}");
    }
}

/// info を除いた診断と、省いた info（`-v` なら省かない）。
fn shown(diags: &[Diagnostic]) -> (Vec<&Diagnostic>, Vec<&Diagnostic>) {
    if VERBOSE.load(Ordering::Relaxed) {
        return (diags.iter().collect(), Vec::new());
    }
    diags.iter().partition(|d| d.severity != Severity::Info)
}

/// 省いた info の 1 行（件数とコード）。
fn hidden_text(hidden: &[&Diagnostic]) -> Option<String> {
    if hidden.is_empty() {
        return None;
    }
    let mut codes: Vec<&str> = hidden.iter().map(|d| d.code).collect();
    codes.sort_unstable();
    codes.dedup();
    Some(format!(
        "info {} 件を省いた（{}。-v で全部表示）",
        hidden.len(),
        codes.join(", ")
    ))
}

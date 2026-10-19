//! GlobeMode の開発用タスク。`cargo xtask <コマンド>`（.cargo/config.toml の alias）。
//!
//! ```text
//! cargo xtask fetch-msl        MSL 4.1.0 を data/msl/ に取得する（sha256 で確かめる）
//! cargo xtask fetch-rumoca     Rumoca の CLI（v0.10.2、Linux x86_64）を data/rumoca/ に取得する
//! cargo xtask msl-compare      MSL の例題で globe check と Rumoca の CLI を突き合わせる
//! cargo xtask msl-funnel       MSL の全例題（566 本）に globe check の判定を出し、報告を書く
//! cargo xtask omc-compare      msl-funnel の結果（--json）を omc（OpenModelica、別プロセス）と突き合わせ、報告を書く
//! cargo xtask fetch-msl-ref    MAP-LIB の参照結果（MSL 4.1.0、commit 固定）を data/msl-ref/ に取得する
//! cargo xtask msl-sim          msl-funnel で OK の例題を実行し（別プロセスのワーカー）、参照結果と管で比べ、報告を書く
//! cargo xtask bump-rumoca      固定している Rumoca の rev と、最新のタグを表示する
//! ```
//!
//! ダウンロードは `curl` を呼ぶ（Linux・Windows 10 以降・CI のランナーに入っている）。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use clap::{Parser, Subcommand};
use globe_mode_frontend::{Frontend, RumocaFrontend};
use sha2::{Digest, Sha256};

mod funnel;
mod omc;
mod reference;
mod sim;

/// MSL 4.1.0 の公式の配布物（BSD-3-Clause）。
const MSL_URL: &str = "https://github.com/modelica/ModelicaStandardLibrary/releases/download/v4.1.0/ModelicaStandardLibrary_v4.1.0.zip";
const MSL_SHA256: &str = "5409cfa17797c52d866dc9d2675badc1378b4bbf88066e60899a489cb1cb8a2d";
const MSL_VERSION: &str = "4.1.0";

/// Rumoca の CLI（Apache-2.0）。Cargo.toml で固定している rev（v0.10.2）と同じ版。
const RUMOCA_VERSION: &str = "v0.10.2";
const RUMOCA_REV: &str = "276e82bd0fd48960ac201e782918d28bf19cf7ce";
const RUMOCA_URL: &str =
    "https://github.com/CogniPilot/rumoca/releases/download/v0.10.2/rumoca-linux-x86_64";
const RUMOCA_SHA256: &str = "d6c9b51eb207477908aa2b74636ac2aa3c5d149e4ff9e9cdd0d9cfeca54111ba";

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "GlobeMode の開発用タスク")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// MSL 4.1.0 を取得して展開する
    FetchMsl {
        #[arg(long, default_value = "data/msl")]
        dest: PathBuf,
    },
    /// Rumoca の CLI を取得する（Linux x86_64 だけ）
    FetchRumoca {
        #[arg(long, default_value = "data/rumoca")]
        dest: PathBuf,
    },
    /// MSL の例題で、globe check（ライブラリとして呼ぶ）と Rumoca の CLI（別プロセス）を突き合わせる
    MslCompare {
        #[arg(long, default_value = "data/msl")]
        msl: PathBuf,
        #[arg(long, default_value = "data/rumoca/rumoca-linux-x86_64")]
        rumoca: PathBuf,
        /// 1 行 1 モデル（# から後はコメント）
        #[arg(long, default_value = "src/Modelica/msl-compare.txt")]
        models: PathBuf,
        /// Markdown の報告を書くファイル
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// MSL の全例題（Rumoca と同じ名前の規則で選ぶ）に判定（S0〜S6 / OK / GM-U）を出し、報告を書く
    MslFunnel {
        #[arg(long, default_value = "data/msl")]
        msl: PathBuf,
        /// 先頭から N 本だけ検査する（--filter の後に当てる）
        #[arg(long)]
        limit: Option<usize>,
        /// 名前にこの文字列を含む例題だけを検査する
        #[arg(long)]
        filter: Option<String>,
        /// Markdown の報告を書くファイル
        #[arg(long)]
        report: Option<PathBuf>,
        /// モデルごとの結果を JSON で書くファイル
        #[arg(long)]
        json: Option<PathBuf>,
        /// 例題の名前だけを表示して終わる
        #[arg(long)]
        list: bool,
        /// N 本ごとに Session を MSL を読んだ直後に戻す（メモリを抑える。0 なら戻さない）
        #[arg(long, default_value_t = 50)]
        reset_every: usize,
    },
    /// msl-funnel の結果（--json）を omc（OpenModelica のコンパイラ、別プロセス）と突き合わせ、報告を書く（S008）
    OmcCompare {
        #[arg(long, default_value = "data/msl")]
        msl: PathBuf,
        /// `msl-funnel --json` が書いたファイル
        #[arg(long, default_value = "target/msl-funnel.json")]
        funnel_json: PathBuf,
        /// omc の実行ファイル（名前かパス）
        #[arg(long, default_value = "omc")]
        omc: String,
        /// Markdown の報告を書くファイル
        #[arg(long)]
        report: Option<PathBuf>,
        /// omc の生の結果と照合の結果を JSON で書くファイル
        #[arg(long)]
        json: Option<PathBuf>,
        /// omc を呼ばず、前に --json で書いた結果を読んで照合だけやり直す
        #[arg(long)]
        reuse: Option<PathBuf>,
        /// 1 つの omc プロセスで検査する本数
        #[arg(long, default_value_t = 50)]
        chunk: usize,
        /// omc プロセス 1 つの制限時間（秒）
        #[arg(long, default_value_t = 900)]
        timeout: u64,
        /// 名前にこの文字列を含む例題だけを照合する
        #[arg(long)]
        filter: Option<String>,
        /// 先頭から N 本だけ照合する（--filter の後に当てる）
        #[arg(long)]
        limit: Option<usize>,
        /// スクリプトと omc の生成物を置く場所
        #[arg(long, default_value = "target/omc-compare")]
        work_dir: PathBuf,
        /// translateModel を呼ばない（checkModel だけ。S5 の照合だけなら速い）
        #[arg(long)]
        no_translate: bool,
    },
    /// MAP-LIB の参照結果（MSL 4.1.0、commit 固定）の CSV と comparisonSignals.txt を取得する（S011）
    FetchMslRef {
        /// 置き場（取得元と同じ形で置く）
        #[arg(long, default_value = "data/msl-ref")]
        dir: PathBuf,
        /// `msl-funnel --json` が書いたファイル（--model が無ければ、判定が OK の例題を partial を除いて取得する）
        #[arg(long, default_value = "target/msl-funnel.json")]
        funnel_json: PathBuf,
        /// 取得するモデル（何度でも。指定すると --funnel-json を使わない）
        #[arg(long = "model")]
        models: Vec<String>,
        /// 同時に取得する本数
        #[arg(long, default_value_t = 4)]
        jobs: usize,
    },
    /// msl-funnel で OK の例題を実行し（別プロセスのワーカー）、MAP-LIB の参照結果と管で比べ、報告を書く（S011）
    MslSim {
        /// `msl-funnel --json` が書いたファイル（判定が OK の例題を、partial を除いて実行する）
        #[arg(long, default_value = "target/msl-funnel.json")]
        funnel_json: PathBuf,
        #[arg(long, default_value = "data/msl")]
        msl: PathBuf,
        /// 参照結果の置き場（`fetch-msl-ref` の --dir）
        #[arg(long, default_value = "data/msl-ref")]
        ref_dir: PathBuf,
        /// 実行の前に、対象の参照結果を取得する（取得済みのものは取り直さない）
        #[arg(long)]
        fetch_ref: bool,
        /// --fetch-ref で同時に取得する本数
        #[arg(long, default_value_t = 4)]
        jobs: usize,
        /// 先頭から N 本だけ実行する（--only の後に当てる）
        #[arg(long)]
        limit: Option<usize>,
        /// 名前にこの文字列を含む例題だけを実行する
        #[arg(long)]
        only: Option<String>,
        /// 1 本の壁時計の予算（秒、実行系の協調型の時間切れ GM-E11005）
        #[arg(long, default_value_t = 30.0)]
        wall_seconds: f64,
        /// 1 本の応答をこの秒数だけ待ち、無ければワーカーを kill して立て直す
        #[arg(long, default_value_t = 120)]
        kill_seconds: u64,
        /// 時間切れ（GM-E11005）になったものだけを、この予算（秒）で回し直す。外から止めるのは予算 + 60 s。
        /// --wall-seconds 以下（0 など）なら回し直さない（S012）
        #[arg(long, default_value_t = 180.0)]
        retry_wall_seconds: f64,
        /// N 本ごとにワーカーの Session を MSL を読んだ直後に戻す（0 なら戻さない。msl-funnel と同じ既定）
        #[arg(long, default_value_t = 50)]
        reset_every: usize,
        /// ソルバ（`bdf`、`dopri45`、`auto` など。既定の `auto` は、BDF が積分で失敗したら dopri45 で回し直す。S012）
        #[arg(long, default_value = "auto")]
        solver: Option<String>,
        /// 実行の結果の CSV を、このディレクトリに `<モデル>.csv` で残す
        #[arg(long)]
        keep_results: Option<PathBuf>,
        /// 管の既知の外れの一覧（1 行 = `<モデル> # 理由と出典`。報告で分けて数える。S012）
        #[arg(long, default_value = "src/Modelica/msl-sim-known-deviations.txt")]
        known_deviations: PathBuf,
        /// 比べる前回の結果（`msl-sim --json`。--json と同じファイルでもよい。書く前に読む。S012）
        #[arg(long)]
        baseline_json: Option<PathBuf>,
        /// Markdown の報告を書くファイル
        #[arg(long)]
        report: Option<PathBuf>,
        /// モデルごとの結果と集計を JSON で書くファイル
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// msl-sim のワーカー（msl-sim が起動する。標準入出力の JSON の行で話す）
    #[command(hide = true)]
    MslSimWorker {
        #[arg(long)]
        msl: PathBuf,
        #[arg(long)]
        ref_dir: PathBuf,
        #[arg(long)]
        wall_seconds: f64,
        #[arg(long)]
        solver: Option<String>,
        #[arg(long, default_value_t = 50)]
        reset_every: usize,
        #[arg(long)]
        keep_results: Option<PathBuf>,
        #[arg(long, default_value_t = 0.002)]
        tube_tolerance: f64,
    },
    /// 固定している Rumoca の rev と、上流の最新のタグを表示する
    BumpRumoca,
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Cmd::FetchMsl { dest } => fetch_msl(&dest),
        Cmd::FetchRumoca { dest } => fetch_rumoca(&dest),
        Cmd::MslCompare {
            msl,
            rumoca,
            models,
            report,
        } => msl_compare(&msl, &rumoca, &models, report.as_deref()),
        Cmd::MslFunnel {
            msl,
            limit,
            filter,
            report,
            json,
            list,
            reset_every,
        } => funnel::msl_funnel(&funnel::Options {
            msl: &msl,
            limit,
            filter: filter.as_deref(),
            report: report.as_deref(),
            json: json.as_deref(),
            list,
            reset_every,
        }),
        Cmd::OmcCompare {
            msl,
            funnel_json,
            omc,
            report,
            json,
            reuse,
            chunk,
            timeout,
            filter,
            limit,
            work_dir,
            no_translate,
        } => omc::omc_compare(&omc::Options {
            msl: &msl,
            funnel_json: &funnel_json,
            omc: &omc,
            report: report.as_deref(),
            json: json.as_deref(),
            reuse: reuse.as_deref(),
            chunk,
            timeout_secs: timeout,
            filter: filter.as_deref(),
            limit,
            work_dir: &work_dir,
            no_translate,
        }),
        Cmd::FetchMslRef {
            dir,
            funnel_json,
            models,
            jobs,
        } => reference::fetch_msl_ref(&dir, &funnel_json, &models, jobs),
        Cmd::MslSim {
            funnel_json,
            msl,
            ref_dir,
            fetch_ref,
            jobs,
            limit,
            only,
            wall_seconds,
            kill_seconds,
            retry_wall_seconds,
            reset_every,
            solver,
            keep_results,
            known_deviations,
            baseline_json,
            report,
            json,
        } => sim::msl_sim(&sim::Options {
            funnel_json: &funnel_json,
            msl: &msl,
            ref_dir: &ref_dir,
            fetch_ref,
            jobs,
            limit,
            only: only.as_deref(),
            wall_seconds,
            kill_seconds,
            retry_wall_seconds,
            reset_every,
            solver: solver.as_deref(),
            keep_results: keep_results.as_deref(),
            known_deviations: Some(&known_deviations),
            baseline_json: baseline_json.as_deref(),
            report: report.as_deref(),
            json: json.as_deref(),
        }),
        Cmd::MslSimWorker {
            msl,
            ref_dir,
            wall_seconds,
            solver,
            reset_every,
            keep_results,
            tube_tolerance,
        } => sim::worker(&sim::WorkerOptions {
            msl,
            ref_dir,
            wall_seconds,
            solver,
            reset_every,
            keep_results,
            tube_tolerance,
        }),
        Cmd::BumpRumoca => bump_rumoca(),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

type Result<T> = std::result::Result<T, String>;

// --------------------------------------------------------------------------- 取得

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    }))
}

/// `url` を `path` に取得し、sha256 を確かめる。すでに正しいファイルがあれば何もしない。
fn download(url: &str, sha256: &str, path: &Path) -> Result<()> {
    if path.exists() && sha256_file(path)? == sha256 {
        println!("取得済み: {}", path.display());
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    println!("取得中: {url}");
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(path)
        .arg(url)
        .status()
        .map_err(|e| format!("curl を実行できない: {e}"))?;
    if !status.success() {
        return Err(format!("curl が失敗した（{status}）"));
    }
    let got = sha256_file(path)?;
    if got != sha256 {
        let _ = fs::remove_file(path);
        return Err(format!("sha256 が一致しない: 期待 {sha256}、実際 {got}"));
    }
    println!("sha256 一致: {got}");
    Ok(())
}

fn fetch_msl(dest: &Path) -> Result<ExitCode> {
    let marker = dest.join(".globe-mode-msl");
    let expected = format!("{MSL_VERSION} {MSL_SHA256}\n");
    if fs::read_to_string(&marker).ok().as_deref() == Some(expected.as_str()) {
        println!("MSL {MSL_VERSION} は展開済み: {}", dest.display());
        return Ok(ExitCode::SUCCESS);
    }
    let zip_path = dest
        .join(".download")
        .join(format!("ModelicaStandardLibrary_v{MSL_VERSION}.zip"));
    download(MSL_URL, MSL_SHA256, &zip_path)?;

    let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name は「..」や絶対パスで展開先の外に出る名前を拒む
        let Some(rel) = entry.enclosed_name() else {
            let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
            return Err(format!("展開先の外を指す名前: {name}"));
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(dir) = out.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut w = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut w).map_err(|e| e.to_string())?;
        count += 1;
    }
    fs::write(&marker, expected).map_err(|e| e.to_string())?;
    println!(
        "MSL {MSL_VERSION} を展開した: {} ファイル → {}",
        count,
        dest.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn fetch_rumoca(dest: &Path) -> Result<ExitCode> {
    if !(cfg!(target_os = "linux") && cfg!(target_arch = "x86_64")) {
        return Err("Rumoca の CLI の取得は、今は Linux x86_64 だけに対応している".into());
    }
    let path = dest.join("rumoca-linux-x86_64");
    download(RUMOCA_URL, RUMOCA_SHA256, &path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(ExitCode::SUCCESS)
}

// --------------------------------------------------------------------------- 突き合わせ

/// 1 つのツールでの検査結果。
#[derive(Debug, Clone, PartialEq)]
struct Outcome {
    ok: bool,
    /// 成功したときの (状態, 代数, 出力)
    counts: Option<(usize, usize, usize)>,
    /// 成功したときの連続方程式の数（DAE の粒度）
    equations: Option<usize>,
    /// 成功したときの（スカラーの方程式数, スカラーの変数数）。GlobeMode だけが返す
    scalar: Option<(usize, usize)>,
    /// 失敗したときの要約（段階や Rumoca のコード）
    detail: String,
    seconds: f64,
}

fn read_models(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect())
}

fn globe_outcome(msl: &Path, model: &str) -> Outcome {
    let t = Instant::now();
    let r = RumocaFrontend::new().check(&[], &[msl], model);
    let seconds = t.elapsed().as_secs_f64();
    match (r.passed(), r.dae, r.balance) {
        (true, Some(d), Some(b)) => Outcome {
            ok: true,
            counts: Some((d.states, d.algebraics, d.outputs)),
            equations: Some(d.continuous_equations),
            scalar: Some((b.equations, b.unknowns)),
            detail: String::new(),
            seconds,
        },
        _ => {
            let d = r.diagnostics.iter().find(|d| d.is_error());
            let detail = d.map_or_else(String::new, |d| {
                let rumoca = d
                    .notes
                    .iter()
                    .find_map(|n| n.strip_prefix("rumoca: "))
                    .unwrap_or("-");
                format!("{:?} {} ({rumoca})", d.stage, d.code)
            });
            Outcome {
                ok: false,
                counts: None,
                equations: None,
                scalar: None,
                detail,
                seconds,
            }
        }
    }
}

/// Rumoca の CLI の `compile --emit dae-json` の結果を数える。
fn rumoca_outcome(rumoca: &Path, msl: &Path, model: &str) -> Outcome {
    let package = fs::read_dir(msl)
        .ok()
        .and_then(|it| {
            it.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("Modelica "))
            })
        })
        .map(|p| p.join("package.mo"));
    let t = Instant::now();
    let Some(package) = package else {
        return Outcome {
            ok: false,
            counts: None,
            equations: None,
            scalar: None,
            detail: "MSL が無い".into(),
            seconds: 0.0,
        };
    };
    let out = Command::new(rumoca)
        .arg("compile")
        .arg(&package)
        .args(["-m", model, "--emit", "dae-json", "-v", "--source-root"])
        .arg(msl)
        .output();
    let seconds = t.elapsed().as_secs_f64();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            return Outcome {
                ok: false,
                counts: None,
                equations: None,
                scalar: None,
                detail: format!("実行できない: {e}"),
                seconds,
            };
        }
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // 最初のエラーコード（[EI008] など）を拾う
        let code = stderr
            .split('[')
            .filter_map(|s| s.split(']').next())
            .find(|c| {
                c.len() >= 4
                    && c.starts_with('E')
                    && c[1..].chars().all(|ch| ch.is_ascii_alphanumeric())
            })
            .unwrap_or("-")
            .to_string();
        return Outcome {
            ok: false,
            counts: None,
            equations: None,
            scalar: None,
            detail: format!("exit {} ({code})", out.status.code().unwrap_or(-1)),
            seconds,
        };
    }
    let json: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(e) => {
            return Outcome {
                ok: false,
                counts: None,
                equations: None,
                scalar: None,
                detail: format!("JSON を読めない: {e}"),
                seconds,
            };
        }
    };
    let storage = &json["storage"];
    let mut roles: BTreeMap<String, usize> = BTreeMap::new();
    for v in storage["variables"].as_array().into_iter().flatten() {
        *roles
            .entry(v["role"].as_str().unwrap_or("?").to_string())
            .or_default() += 1;
    }
    let get = |k: &str| roles.get(k).copied().unwrap_or(0);
    // 連続方程式の数は、-v が表示する `[rumoca]   Continuous equations (f_x): N`（DAE の方程式の数）を使う。
    // dae-json の continuous_equation_operations は「操作」の数で、配列や for を含むと多くなる。
    let stderr = String::from_utf8_lossy(&out.stderr);
    let continuous = stderr
        .lines()
        .find_map(|l| l.strip_prefix("[rumoca]   Continuous equations (f_x): "))
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| {
            storage["continuous_equation_operations"]
                .as_array()
                .map_or(0, Vec::len)
        });
    Outcome {
        ok: true,
        counts: Some((get("state"), get("algebraic"), get("output"))),
        equations: Some(continuous),
        scalar: None,
        detail: String::new(),
        seconds,
    }
}

/// 一致の判定: 成否が同じで、両方成功なら DAE の（状態, 代数, 出力）と連続方程式の数が同じ。
fn agrees(a: &Outcome, b: &Outcome) -> bool {
    a.ok == b.ok && (!a.ok || (a.counts == b.counts && a.equations == b.equations))
}

fn cell(o: &Outcome) -> String {
    match (o.ok, o.counts, o.equations) {
        (true, Some((s, a, out)), Some(eq)) => match o.scalar {
            Some((se, su)) => {
                format!("OK {s} / {a} / {out}・連続方程式 {eq}（スカラー {se} = {su}）")
            }
            None => format!("OK {s} / {a} / {out}・連続方程式 {eq}"),
        },
        _ => format!("NG {}", o.detail),
    }
}

fn msl_compare(
    msl: &Path,
    rumoca: &Path,
    models: &Path,
    report: Option<&Path>,
) -> Result<ExitCode> {
    if !msl.join(".globe-mode-msl").exists() {
        return Err(format!(
            "{} に MSL が無い。先に `cargo xtask fetch-msl` を実行する",
            msl.display()
        ));
    }
    if !rumoca.exists() {
        return Err(format!(
            "{} が無い。先に `cargo xtask fetch-rumoca` を実行する",
            rumoca.display()
        ));
    }
    let models = read_models(models)?;
    let mut rows = Vec::new();
    for (i, m) in models.iter().enumerate() {
        let g = globe_outcome(msl, m);
        let r = rumoca_outcome(rumoca, msl, m);
        let ok = agrees(&g, &r);
        println!(
            "[{:>2}/{}] {} {m}  globe: {}  rumoca: {}",
            i + 1,
            models.len(),
            if ok { "一致" } else { "不一致" },
            cell(&g),
            cell(&r)
        );
        rows.push((m.clone(), g, r, ok));
    }
    let agreed = rows.iter().filter(|r| r.3).count();
    let both_ok = rows.iter().filter(|r| r.1.ok && r.2.ok).count();
    let summary = format!(
        "{} 本中 {agreed} 本が一致（両方とも成功 {both_ok} 本）",
        rows.len()
    );
    println!("{summary}");

    if let Some(path) = report {
        let mut md = String::new();
        let _ = writeln!(
            md,
            "# MSL {MSL_VERSION} での globe check と Rumoca の CLI の突き合わせ\n"
        );
        let _ = writeln!(
            md,
            "`cargo xtask msl-compare` が生成した。手で編集しない。\n"
        );
        let _ = writeln!(md, "| | |\n| --- | --- |");
        let _ = writeln!(
            md,
            "| MSL | {MSL_VERSION}（sha256 `{}…`） |",
            &MSL_SHA256[..12]
        );
        let _ = writeln!(
            md,
            "| Rumoca | CLI {RUMOCA_VERSION}（GlobeMode の依存も同じ rev `{}`） |",
            &RUMOCA_REV[..8]
        );
        let _ = writeln!(md, "| 結果 | **{summary}** |\n");
        let _ = writeln!(
            md,
            "- 一致の判定: 成否が同じで、両方とも成功したときは DAE の（状態 / 代数 / 出力）と連続方程式の数が同じ。DAE の数は、配列を 1 個として数える（Rumoca の CLI の `-v` と同じ粒度）。"
        );
        let _ = writeln!(
            md,
            "- 連続方程式の数は、Rumoca の CLI が `-v` で表示する `[rumoca] Continuous equations (f_x)` と同じ数え方（`--emit dae-json` の `continuous_equation_operations` の数ではない。こちらは操作の数で、配列や for を含むと多くなる）。\n- 「スカラー」は配列を展開した方程式数 = 変数数で、S5 の釣り合いの検査に使う数。GlobeMode だけが表示する。"
        );
        let _ = writeln!(
            md,
            "- 失敗の欄は、GlobeMode は「段階 GM コード（Rumoca のコード）」、Rumoca の CLI は「終了コード（エラーコード）」。\n"
        );
        let _ = writeln!(
            md,
            "| # | モデル | globe check（状態 / 代数 / 出力） | Rumoca の CLI（状態 / 代数 / 出力） | 判定 |"
        );
        let _ = writeln!(md, "| --- | --- | --- | --- | --- |");
        for (i, (m, g, r, ok)) in rows.iter().enumerate() {
            let short = m.strip_prefix("Modelica.").unwrap_or(m);
            let _ = writeln!(
                md,
                "| {} | `{short}` | {} | {} | {} |",
                i + 1,
                cell(g),
                cell(r),
                if *ok { "一致" } else { "**不一致**" }
            );
        }
        fs::write(path, md).map_err(|e| e.to_string())?;
        println!("報告: {}", path.display());
    }
    Ok(if agreed == rows.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

// --------------------------------------------------------------------------- Rumoca の更新

fn bump_rumoca() -> Result<ExitCode> {
    println!("固定している Rumoca: {RUMOCA_VERSION}（rev {RUMOCA_REV}）");
    let out = Command::new("git")
        .args([
            "ls-remote",
            "--tags",
            "--refs",
            "https://github.com/CogniPilot/rumoca.git",
        ])
        .output()
        .map_err(|e| format!("git を実行できない: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let latest = text
        .lines()
        .filter_map(|l| l.split("refs/tags/").nth(1))
        .filter(|t| t.starts_with('v'))
        .max_by_key(|t| {
            t.trim_start_matches('v')
                .split('.')
                .map(|p| p.parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>()
        })
        .unwrap_or("?");
    println!("上流の最新のタグ: {latest}");
    if latest == RUMOCA_VERSION {
        println!("最新を使っている。");
    } else {
        println!("更新の手順（doc/adr/0002-rumoca-adapter.md）:");
        println!(
            "  1. Cargo.toml の rumoca-compile の rev と、xtask の RUMOCA_* の定数を新しいタグに合わせる"
        );
        println!("  2. cargo xtask fetch-rumoca && cargo xtask msl-compare --report <報告>");
        println!("  3. mise run check。deny.toml の advisories の例外がまだ要るかも見直す");
        println!("  4. 突き合わせの差分を作業記録に残す");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(ok: bool, counts: Option<(usize, usize, usize)>) -> Outcome {
        Outcome {
            ok,
            counts,
            equations: None,
            scalar: None,
            detail: String::new(),
            seconds: 0.0,
        }
    }

    #[test]
    fn agreement_rules() {
        assert!(agrees(
            &o(true, Some((3, 41, 0))),
            &o(true, Some((3, 41, 0)))
        ));
        assert!(!agrees(
            &o(true, Some((3, 41, 0))),
            &o(true, Some((2, 41, 0)))
        ));
        assert!(agrees(&o(false, None), &o(false, None)));
        assert!(!agrees(&o(true, Some((1, 1, 0))), &o(false, None)));
    }

    #[test]
    fn msl_sim_arguments_parse() {
        let cli = Cli::try_parse_from([
            "xtask",
            "msl-sim",
            "--funnel-json",
            "f.json",
            "--limit",
            "5",
            "--fetch-ref",
            "--only",
            "Analog",
            "--wall-seconds",
            "10.5",
            "--kill-seconds",
            "60",
            "--keep-results",
            "out",
        ])
        .unwrap();
        match cli.command {
            Cmd::MslSim {
                funnel_json,
                limit,
                fetch_ref,
                only,
                wall_seconds,
                kill_seconds,
                keep_results,
                ref_dir,
                reset_every,
                ..
            } => {
                assert_eq!(funnel_json, PathBuf::from("f.json"));
                assert_eq!(limit, Some(5));
                assert!(fetch_ref);
                assert_eq!(only.as_deref(), Some("Analog"));
                assert_eq!(wall_seconds, 10.5);
                assert_eq!(kill_seconds, 60);
                assert_eq!(keep_results, Some(PathBuf::from("out")));
                assert_eq!(ref_dir, PathBuf::from("data/msl-ref"));
                assert_eq!(reset_every, 50);
            }
            other => panic!("msl-sim でない: {other:?}"),
        }
        // 既定値（mise の msl:sim と同じ呼び方）
        let cli = Cli::try_parse_from(["xtask", "msl-sim", "--report", "r.md"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::MslSim { wall_seconds, kill_seconds: 120, fetch_ref: false, .. } if wall_seconds == 30.0
        ));
        // S012 の既定: ソルバは auto、時間切れは 180 s で回し直す、既知の外れの一覧を読む、前回の結果は読まない
        match Cli::try_parse_from(["xtask", "msl-sim"]).unwrap().command {
            Cmd::MslSim {
                solver,
                retry_wall_seconds,
                known_deviations,
                baseline_json,
                ..
            } => {
                assert_eq!(solver.as_deref(), Some("auto"));
                assert_eq!(retry_wall_seconds, 180.0);
                assert_eq!(
                    known_deviations,
                    PathBuf::from("src/Modelica/msl-sim-known-deviations.txt")
                );
                assert_eq!(baseline_json, None);
            }
            other => panic!("msl-sim でない: {other:?}"),
        }
        let cli = Cli::try_parse_from([
            "xtask",
            "msl-sim",
            "--solver",
            "bdf",
            "--retry-wall-seconds",
            "0",
            "--baseline-json",
            "target/msl-sim.json",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Cmd::MslSim { ref solver, retry_wall_seconds, baseline_json: Some(_), .. }
                if solver.as_deref() == Some("bdf") && retry_wall_seconds == 0.0
        ));
        let cli = Cli::try_parse_from([
            "xtask",
            "fetch-msl-ref",
            "--model",
            "Modelica.A.B",
            "--model",
            "Modelica.C.D",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Cmd::FetchMslRef { ref models, jobs: 4, .. } if models.len() == 2
        ));
        // ワーカーの引数は親（sim::worker_args）が組む形で読める
        let cli = Cli::try_parse_from([
            "xtask",
            "msl-sim-worker",
            "--msl",
            "data/msl",
            "--ref-dir",
            "data/msl-ref",
            "--wall-seconds",
            "30",
            "--reset-every",
            "25",
            "--tube-tolerance",
            "0.002",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Cmd::MslSimWorker { solver: None, .. }
        ));
    }

    #[test]
    fn model_list_skips_comments() {
        let dir = std::env::temp_dir().join("globe-mode-xtask-test");
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("models.txt");
        fs::write(&p, "# comment\nA.B  # trailing\n\nC\n").unwrap();
        assert_eq!(read_models(&p).unwrap(), ["A.B", "C"]);
    }
}

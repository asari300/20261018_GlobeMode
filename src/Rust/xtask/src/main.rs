//! GlobeMode の開発用タスク。`cargo xtask <コマンド>`（.cargo/config.toml の alias）。
//!
//! ```text
//! cargo xtask fetch-msl        MSL 4.1.0 を data/msl/ に取得する（sha256 で確かめる）
//! cargo xtask fetch-rumoca     Rumoca の CLI（v0.10.2、Linux x86_64）を data/rumoca/ に取得する
//! cargo xtask msl-compare      MSL の例題で globe check と Rumoca の CLI を突き合わせる
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
    fn model_list_skips_comments() {
        let dir = std::env::temp_dir().join("globe-mode-xtask-test");
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("models.txt");
        fs::write(&p, "# comment\nA.B  # trailing\n\nC\n").unwrap();
        assert_eq!(read_models(&p).unwrap(), ["A.B", "C"]);
    }
}

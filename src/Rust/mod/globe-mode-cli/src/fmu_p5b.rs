//! `globe fmu export --runtime p5b`（S238、P5b。[ADR-0031](../../../../../doc/adr/0031-fmi-export-p5a-first.md)）。
//!
//! FMU は `globe-mode-fmu`（自前の実行時）の cdylib と、`modelDescription.xml`、`resources/<完全修飾名>.mo`、
//! モデルが使うライブラリのファイルだけを入れた `resources/lib/` を zip にしたもの。実行時（`fmi2Instantiate`）が
//! `resources` から compile し直すので、`resources/lib` に足りないクラスがあると instantiate が失敗する。そこで
//! `modelDescription.xml` も同じ `resources/lib` から作り、足りなければここで断る。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use globe_mode_fmi::export::{check_out_root, platform, zip_dir};
use globe_mode_fmu::Kind;
use globe_mode_frontend::{Source, collect_mo_files, library_closure};

/// `globe fmu export --runtime`
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FmuRuntime {
    /// Rumoca のテンプレートが描く C のソースの FMU（既定。`--build` で共有ライブラリも作る）
    P5a,
    /// `globe-mode-fmu`（自前の実行時）の共有ライブラリを入れた FMU。イベント・MSL のモデルも載る
    P5b,
}

pub struct Args<'a> {
    pub files: &'a [PathBuf],
    pub library: &'a [PathBuf],
    pub model: &'a str,
    pub out: &'a Path,
    pub fmi_version: u8,
    pub runtime_lib: Option<&'a Path>,
}

pub fn export(a: &Args) -> ExitCode {
    match run(a) {
        Ok(fmu) => {
            println!("{}", fmu.display());
            ExitCode::SUCCESS
        }
        Err((code, e)) => {
            eprintln!("error: {e}");
            ExitCode::from(code)
        }
    }
}

type Failure = (u8, String);

fn run(a: &Args) -> Result<PathBuf, Failure> {
    let usage = |e: String| (2, e);
    let fail = |e: String| (1, e);
    // shortcut: 実行時が読む resources の .mo は 1 つだけ。利用者のモデルは 1 ファイルか、-L のライブラリの中のクラス。複数ファイルなら実行時を直すときに広げる
    let text = match a.files {
        [] if !a.library.is_empty() => "// library model (S238)\n".to_string(),
        [f] if f.is_file() => std::fs::read_to_string(f)
            .map_err(|e| usage(format!("{} を読めない: {e}", f.display())))?,
        _ => {
            return Err(usage(
                "--runtime p5b のモデルは 1 つの .mo ファイル、または -L のライブラリの中のクラス（ファイルは渡さない）"
                    .into(),
            ));
        }
    };
    let name = format!("{}-p5b", a.model.replace('.', "_"));
    let root = a.out.join(&name);
    let (dir, ext) = platform(
        &format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        a.fmi_version,
    )
    .map_err(usage)?;
    let runtime = match a.runtime_lib {
        Some(p) if p.is_file() => p.to_path_buf(),
        Some(p) => return Err(usage(format!("--runtime-lib {} が無い", p.display()))),
        None => build_runtime().map_err(usage)?,
    };
    if root.exists()
        && a.files.iter().chain(a.library).any(|s| {
            s.canonicalize()
                .is_ok_and(|s| s.starts_with(root.canonicalize().unwrap_or_default()))
        })
    {
        return Err(usage(format!(
            "作り直す {} に入力が入っている",
            root.display()
        )));
    }
    check_out_root(&root).map_err(usage)?;
    let io = |e: std::io::Error| fail(e.to_string());
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(io)?;
    }
    let result = stage(a, &text, &root, &runtime, &dir, ext);
    if result.is_err() {
        // 作りかけは modelDescription.xml が無く、次の Export の守りで断られるので消す（root は上で作り直したもの）
        let _ = std::fs::remove_dir_all(&root);
    }
    result.map_err(fail)?;
    let fmu = a.out.join(format!("{name}-fmi{}.fmu", a.fmi_version));
    zip_dir(&root, &fmu).map_err(fail)?;
    Ok(fmu)
}

fn stage(
    a: &Args,
    text: &str,
    root: &Path,
    runtime: &Path,
    dir: &str,
    ext: &str,
) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let res = root.join("resources");
    std::fs::create_dir_all(&res).map_err(io)?;
    std::fs::write(res.join(format!("{}.mo", a.model)), text).map_err(io)?;
    // ライブラリのうちモデルが使うファイルだけを resources/lib に（相対パスは -L のルートからのもの）
    let mut files: BTreeMap<String, (PathBuf, String)> = BTreeMap::new();
    for lib in a.library {
        let mut paths = Vec::new();
        collect_mo_files(lib, &mut paths).map_err(|e| format!("{}: {e}", lib.display()))?;
        for p in paths {
            let rel = p
                .strip_prefix(lib)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let body = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            if files.insert(rel.clone(), (p, body)).is_some() {
                return Err(format!("{rel} が複数の -L にある"));
            }
        }
    }
    let sources: Vec<Source> = files
        .iter()
        .map(|(rel, (_, body))| Source {
            name: rel,
            text: body,
        })
        .collect();
    let lib = res.join("lib");
    for rel in library_closure(&sources, a.model, &[text]) {
        let to = lib.join(&rel);
        std::fs::create_dir_all(to.parent().expect("lib の下")).map_err(io)?;
        std::fs::write(to, &files[&rel].1).map_err(io)?;
    }
    let (v3, kind) = (a.fmi_version == 3, Kind::Both);
    let xml = if lib.is_dir() {
        globe_mode_fmu::model_description_lib(a.model, text, v3, kind, &lib)
    } else {
        globe_mode_fmu::model_description_kind(a.model, text, v3, kind)
    }
    .map_err(|e| format!("{}: FMU にできない（{e}）", a.model))?;
    std::fs::write(root.join("modelDescription.xml"), xml).map_err(io)?;
    let bin = root.join("binaries").join(dir);
    std::fs::create_dir_all(&bin).map_err(io)?;
    std::fs::copy(runtime, bin.join(format!("globe_mode_fmu.{ext}"))).map_err(io)?;
    Ok(())
}

/// `cargo build --release -p globe-mode-fmu` で cdylib を作り、そのパスを返す（globe のソースの作業ツリーの中で）。
/// 作れない（cargo が無い・作業ツリーの外）ときは `--runtime-lib` を使うよう添えた理由。
fn build_runtime() -> Result<PathBuf, String> {
    let hint = "globe のソースの作業ツリーの中で回すか、--runtime-lib で globe-mode-fmu の共有ライブラリを渡す";
    let out = Command::new(std::env::var_os("CARGO").unwrap_or("cargo".into()))
        .args([
            "build",
            "--release",
            "-p",
            "globe-mode-fmu",
            "--lib",
            "--message-format",
            "json-render-diagnostics",
        ])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo を起動できない（{e}）。{hint}"))?;
    if !out.status.success() {
        return Err(format!("cargo build -p globe-mode-fmu が失敗した。{hint}"));
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|m| m["reason"] == "compiler-artifact" && m["target"]["name"] == "globe_mode_fmu")
        .flat_map(|m| m["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f.as_str().map(PathBuf::from))
        .find(|f| {
            matches!(
                f.extension().and_then(|e| e.to_str()),
                Some("so" | "dll" | "dylib")
            )
        })
        .ok_or_else(|| format!("cargo が共有ライブラリを作らなかった。{hint}"))
}

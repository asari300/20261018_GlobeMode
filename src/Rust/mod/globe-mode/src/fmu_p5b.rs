//! P5b の FMU の書き出し（S238 で CLI に、S239 で facade へ。[ADR-0031](../../../../../doc/adr/0031-fmi-export-p5a-first.md)）。
//!
//! FMU は `globe-mode-fmu`（自前の実行時）の cdylib と、`modelDescription.xml`、`resources/<完全修飾名>.mo`、
//! モデルが使うライブラリのファイルだけを入れた `resources/lib/` を zip にしたもの。実行時（`fmi2Instantiate`）が
//! `resources` から compile し直すので、`resources/lib` に足りないクラスがあると instantiate が失敗する。そこで
//! `modelDescription.xml` も同じ `resources/lib` から作り、足りなければここで断る。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use globe_mode_fmi::export::{check_out_root, platform, zip_dir};
use globe_mode_fmu::Kind;
use globe_mode_frontend::{Source, collect_mo_files, library_slice};

use crate::{FmuExportKind, FmuOptions};

/// 書き出せなかった理由。CLI は `Usage` を終了コード 2、`Failed` を 1 にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum P5bError {
    /// 頼み方の誤り（旗の組み合わせ、共有ライブラリが無い・作れない、入力を消してしまう出力先、非対応の OS）
    Usage(String),
    /// 書いている途中の失敗（ファイル、`modelDescription.xml` にできないモデル）
    Failed(String),
}

impl std::fmt::Display for P5bError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (Self::Usage(e) | Self::Failed(e)) = self;
        f.write_str(e)
    }
}

impl From<P5bError> for String {
    fn from(e: P5bError) -> String {
        e.to_string()
    }
}

/// `out_dir/<名前>-p5b/`（展開した FMU）と `out_dir/<名前>-p5b-fmi<版>.fmu` を書き、`.fmu` のパスを返す。
///
/// `text` はモデルを定義する利用者のファイルの本文（`None` は `library` の中のクラス）。`files` は入力のファイル
/// （出力先に入っていたら断るためのもの）。`options.runtime_lib` が `None` なら `cargo build --release -p globe-mode-fmu-cdylib` で作る。
/// shortcut: 実行時が読む resources の .mo は 1 つだけ（利用者のモデルは 1 ファイルか、`library` の中のクラス）。複数ファイルなら実行時を直すときに広げる
pub fn export_fmu_p5b(
    model: &str,
    text: Option<&str>,
    files: &[PathBuf],
    library: &[PathBuf],
    out_dir: &Path,
    options: &FmuOptions,
) -> Result<PathBuf, P5bError> {
    use P5bError::{Failed, Usage};
    if options.build || !options.targets.is_empty() || options.fmi_headers.is_some() {
        return Err(Usage(
            "build・targets・fmi_headers は runtime p5a のもの（p5b は共有ライブラリを入れる）"
                .into(),
        ));
    }
    let se = options.kind == FmuExportKind::Se;
    if se && options.fmi_version != 3 {
        return Err(Usage("kind se は FMI 3.0（fmi_version 3）だけ".into()));
    }
    let text = text.unwrap_or("// library model (S238)\n");
    let name = format!(
        "{}-p5b{}",
        model.replace('.', "_"),
        if se { "-se" } else { "" }
    );
    let root = out_dir.join(&name);
    let (dir, ext) = platform(
        &format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        options.fmi_version,
    )
    .map_err(Usage)?;
    let runtime = match &options.runtime_lib {
        Some(p) if p.is_file() => p.clone(),
        Some(p) => return Err(Usage(format!("runtime_lib {} が無い", p.display()))),
        None => build_runtime().map_err(Usage)?,
    };
    if root.exists()
        && files.iter().chain(library).any(|s| {
            s.canonicalize()
                .is_ok_and(|s| s.starts_with(root.canonicalize().unwrap_or_default()))
        })
    {
        return Err(Usage(format!(
            "作り直す {} に入力が入っている",
            root.display()
        )));
    }
    check_out_root(&root).map_err(Usage)?;
    let io = |e: std::io::Error| Failed(e.to_string());
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(io)?;
    }
    let result = stage(
        model,
        text,
        library,
        options.fmi_version,
        se,
        &root,
        &runtime,
        &dir,
        ext,
    );
    if result.is_err() {
        // 作りかけは modelDescription.xml が無く、次の Export の守りで断られるので消す（root は上で作り直したもの）
        let _ = std::fs::remove_dir_all(&root);
    }
    result.map_err(Failed)?;
    let fmu = out_dir.join(format!("{name}-fmi{}.fmu", options.fmi_version));
    zip_dir(&root, &fmu).map_err(Failed)?;
    Ok(fmu)
}

#[allow(clippy::too_many_arguments)]
fn stage(
    model: &str,
    text: &str,
    library: &[PathBuf],
    fmi_version: u8,
    se: bool,
    root: &Path,
    runtime: &Path,
    dir: &str,
    ext: &str,
) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let res = root.join("resources");
    std::fs::create_dir_all(&res).map_err(io)?;
    std::fs::write(res.join(format!("{model}.mo")), text).map_err(io)?;
    // ライブラリのうちモデルが使うファイル（とクラス）だけを resources/lib に（相対パスは library のルートからのもの）
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for lib in library {
        let mut paths = Vec::new();
        collect_mo_files(lib, &mut paths).map_err(|e| format!("{}: {e}", lib.display()))?;
        for p in paths {
            let rel = p
                .strip_prefix(lib)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let body = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            if files.insert(rel.clone(), body).is_some() {
                return Err(format!("{rel} が複数のライブラリにある"));
            }
        }
    }
    let sources: Vec<Source> = files
        .iter()
        .map(|(rel, body)| Source {
            name: rel,
            text: body,
        })
        .collect();
    let lib = res.join("lib");
    // S243: ファイルだけでなく、使わないクラスも落とした本文を入れる
    for (rel, body) in library_slice(&sources, model, &[text]) {
        let to = lib.join(&rel);
        std::fs::create_dir_all(to.parent().expect("lib の下")).map_err(io)?;
        std::fs::write(to, body).map_err(io)?;
    }
    // MSL など、BSD 3-Clause の著作権表示を再配布物に付ける（S239。ライブラリの根にある LICENSE を写す）
    if lib.is_dir() {
        for lib_root in library.iter().filter(|r| r.join("LICENSE").is_file()) {
            std::fs::copy(lib_root.join("LICENSE"), lib.join("LICENSE")).map_err(io)?;
        }
    }
    let (v3, kind) = (fmi_version == 3, if se { Kind::Se } else { Kind::Both });
    let xml = if lib.is_dir() {
        globe_mode_fmu::model_description_lib(model, text, v3, kind, &lib)
    } else {
        globe_mode_fmu::model_description_kind(model, text, v3, kind)
    }
    .map_err(|e| format!("{model}: FMU にできない（{e}）"))?;
    std::fs::write(root.join("modelDescription.xml"), xml).map_err(io)?;
    let bin = root.join("binaries").join(dir);
    std::fs::create_dir_all(&bin).map_err(io)?;
    let to = bin.join(format!("globe_mode_fmu.{ext}"));
    std::fs::copy(runtime, &to).map_err(io)?;
    strip(&to);
    Ok(())
}

/// 写した共有ライブラリのシンボルを落として小さくする（S239。Linux は `strip`、macOS は `strip -x`、Windows は何もしない）。
/// 写しに対してだけ行い、渡された `runtime_lib` は触らない。`strip` が無い・失敗したときは、そのまま入れる。
fn strip(lib: &Path) {
    let args: &[&str] = if cfg!(target_os = "linux") {
        &[]
    } else if cfg!(target_os = "macos") {
        &["-x"]
    } else {
        return;
    };
    let _ = Command::new("strip")
        .args(args)
        .arg(lib)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// 共有ライブラリを小さくするプロファイル（S243。`panic = "abort"` は `extern "C"` の境界で panic が abort になる今の挙動と同じ）
const SMALL_PROFILE: [(&str, &str); 5] = [
    ("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z"),
    ("CARGO_PROFILE_RELEASE_LTO", "fat"),
    ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1"),
    ("CARGO_PROFILE_RELEASE_PANIC", "abort"),
    ("CARGO_PROFILE_RELEASE_STRIP", "true"),
];

/// `cargo build --release -p globe-mode-fmu-cdylib` で cdylib を作り、そのパスを返す（globe のソースの作業ツリーの中で）。
/// 作れない（cargo が無い・作業ツリーの外）ときは `runtime_lib` を使うよう添えた理由。
fn build_runtime() -> Result<PathBuf, String> {
    let hint = "globe のソースの作業ツリーの中で回すか、runtime_lib（--runtime-lib）で globe-mode-fmu-cdylib の共有ライブラリを渡す";
    let mut cargo = Command::new(std::env::var_os("CARGO").unwrap_or("cargo".into()));
    // S243: 小さくするプロファイルを環境変数で与える（workspace の [profile.release] は変えない。mise の wasm:build と同じ）。利用者が決めたものは尊重する
    for (k, v) in SMALL_PROFILE {
        if std::env::var_os(k).is_none() {
            cargo.env(k, v);
        }
    }
    let out = cargo
        .args([
            "build",
            "--release",
            "-p",
            "globe-mode-fmu-cdylib",
            "--lib",
            "--message-format",
            "json-render-diagnostics",
        ])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo を起動できない（{e}）。{hint}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo build -p globe-mode-fmu-cdylib が失敗した。{hint}"
        ));
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|m| {
            m["reason"] == "compiler-artifact" && m["target"]["name"] == "globe_mode_fmu_cdylib"
        })
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

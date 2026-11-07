//! S094（P5a）: FMI 2.0 / 3.0（S098）のソースの FMU を書き出す（[ADR-0031]）。
//!
//! 中身は Rumoca の組み込みターゲット `fmi2` / `fmi3` のテンプレートが描く（`globe-mode-frontend` の `fmi_export`）。
//! ここではそれを FMU の形（展開したディレクトリと zip）に置き、求められれば `cc` で共有ライブラリにする。
//!
//! [ADR-0031]: https://github.com/asari300/20261018_GlobeMode/blob/main/doc/adr/0031-fmi-export-p5a-first.md

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use globe_mode_frontend::CompiledModel;

/// テンプレートは Rumoca（Apache-2.0）のものなので、FMU に Rumoca の著作権表示とライセンスを同梱する（doc/licenses.md）。
const RUMOCA_LICENSE: &str = concat!(
    "This FMU's sources were generated from the built-in FMI templates of Rumoca\n",
    "(https://github.com/CogniPilot/rumoca, rev 276e82bd0fd48960ac201e782918d28bf19cf7ce),\n",
    "which are licensed under the Apache License, Version 2.0.\n\n",
    "   Copyright 2025-2026 CogniPilot Foundation\n\n",
    include_str!("../../../../../LICENSE-APACHE")
);

/// FMI 2.0 / 3.0 の ME（と CS）のソースの FMU の中身。
#[derive(Debug, Clone)]
pub struct MeSources {
    /// FMI の主版（2 か 3）。
    pub fmi_version: u8,
    /// FMU のファイル名の元（モデルの完全修飾名の `.` を `_` にしたもの。版は [`MeSources::file_name`] が付ける）。
    pub name: String,
    /// `ModelExchange` の `modelIdentifier`（共有ライブラリの名前）。
    pub model_identifier: String,
    pub model_description_xml: String,
    /// `sources/` の下のファイル（FMU の中の相対パスと中身）。
    pub c_sources: Vec<(String, String)>,
}

impl MeSources {
    /// 既定の `.fmu` のファイル名 `<名前>-fmi2.fmu`（S189。FMI 2.0 と 3.0 を同じ出力先に書いても上書きし合わない。
    /// CLI・facade・Python・LSP・C ABI・NIF・UI のダウンロード名はこの形。中身の `modelName` と `modelIdentifier` は変えない）。
    pub fn file_name(&self) -> String {
        format!("{}-fmi{}.fmu", self.name, self.fmi_version)
    }
}

/// モデルを FMI `fmi_version`（2 か 3）のソースの FMU にする。Rumoca の対象外（イベント・外部関数など）は `Err`。
pub fn export_me_sources(model: &CompiledModel, fmi_version: u8) -> Result<MeSources, String> {
    if !matches!(fmi_version, 2 | 3) {
        return Err(format!("FMI {fmi_version} は未対応（2 か 3）"));
    }
    let target = format!("fmi{fmi_version}");
    // S189: Rumoca の限界は英語の 1 行で返る（例 `FMI C profile: …`）ので、FMU にできないことを日本語で前に付ける
    let mut files = globe_mode_frontend::fmi_export::render_rumoca_fmi(model, &target)
        .map_err(|e| format!("このモデルは FMU に書き出せない（理由: {e}）"))?;
    let at = files
        .iter()
        .position(|(p, _)| p == "modelDescription.xml")
        .ok_or(format!(
            "Rumoca の {target} が modelDescription.xml を描かなかった"
        ))?;
    let (_, xml) = files.remove(at);
    let md = crate::parse_model_description(&xml).map_err(|e| e.to_string())?;
    let me = md
        .model_exchange
        .ok_or("modelDescription.xml に ModelExchange が無い")?;
    Ok(MeSources {
        fmi_version,
        name: model.model.replace('.', "_"),
        model_identifier: me.model_identifier,
        model_description_xml: xml,
        c_sources: files,
    })
}

/// `out_dir/<name>/`（展開した FMU）と `out_dir/<name>-fmi<版>.fmu`（[`MeSources::file_name`]）を書き、`.fmu` のパスを返す。
///
/// `fmi_headers` を渡すと、その中の `fmi2Functions.h`（3.0 は `fmi3Functions.h`）などで共有ライブラリも作る。
/// `targets` が空なら host（x86_64 の Linux だけ）を `cc` で `binaries/linux64/`（3.0 は `binaries/x86_64-linux/`）に、
/// 空でなければ triple ごとに `zig cc -target <triple>` で `binaries/<platform>/` に作る（S105）。
pub fn write_fmu(
    src: &MeSources,
    out_dir: &Path,
    fmi_headers: Option<&Path>,
    targets: &[String],
) -> Result<PathBuf, String> {
    let root = out_dir.join(&src.name);
    let io = |e: std::io::Error| e.to_string();
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(io)?;
    }
    std::fs::create_dir_all(root.join("sources")).map_err(io)?;
    std::fs::write(
        root.join("modelDescription.xml"),
        &src.model_description_xml,
    )
    .map_err(io)?;
    for (path, text) in &src.c_sources {
        std::fs::write(root.join(path), text).map_err(io)?;
    }
    let licenses = root.join("documentation/licenses");
    std::fs::create_dir_all(&licenses).map_err(io)?;
    std::fs::write(licenses.join("Rumoca-LICENSE.txt"), RUMOCA_LICENSE).map_err(io)?;
    if let Some(headers) = fmi_headers {
        if targets.is_empty() {
            build(src, &root, headers, None)?;
        }
        for t in targets {
            build(src, &root, headers, Some(t))?;
        }
    }
    let fmu = out_dir.join(src.file_name());
    zip_dir(&root, &fmu)?;
    Ok(fmu)
}

/// ソースの FMU（`write_fmu` の `build` 無しと同じ中身）を zip のバイト列にする（S144。ファイルに触らないので wasm32 でも呼べる）。
pub fn fmu_bytes(src: &MeSources) -> Result<Vec<u8>, String> {
    let e = |e: zip::result::ZipError| e.to_string();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let licenses = ("documentation/licenses/Rumoca-LICENSE.txt", RUMOCA_LICENSE);
    let md = ("modelDescription.xml", src.model_description_xml.as_str());
    for (path, text) in std::iter::once(md)
        .chain(src.c_sources.iter().map(|(p, t)| (p.as_str(), t.as_str())))
        .chain(std::iter::once(licenses))
    {
        zip.start_file(path, options).map_err(e)?;
        zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(e)?.into_inner())
}

/// zig の target triple（例 `x86_64-windows-gnu`）から、`binaries/` の下の名前と共有ライブラリの拡張子を返す。
/// FMI 2.0 は `linux64`・`win64`・`darwin64`、3.0 は `<arch>-<os>`（`x86_64-windows`・`aarch64-darwin` など）。
fn platform(triple: &str, fmi_version: u8) -> Result<(String, &'static str), String> {
    let mut parts = triple.split('-');
    let arch = parts.next().unwrap_or_default();
    let (os, ext) = match parts.next() {
        Some("linux") => ("linux", "so"),
        Some("windows") => ("windows", "dll"),
        Some("macos") => ("darwin", "dylib"),
        _ => {
            return Err(format!(
                "--target {triple}: OS は linux・windows・macos だけ"
            ));
        }
    };
    if !matches!(arch, "x86_64" | "aarch64") {
        return Err(format!("--target {triple}: arch は x86_64・aarch64 だけ"));
    }
    if fmi_version == 3 {
        return Ok((format!("{arch}-{os}"), ext));
    }
    // FMI 2.0 の darwin64 は arch を区別しない（FMPy も Apple silicon で darwin64 を引く）
    match (arch, os) {
        ("x86_64", "linux") => Ok(("linux64".into(), ext)),
        ("x86_64", "windows") => Ok(("win64".into(), ext)),
        (_, "darwin") => Ok(("darwin64".into(), ext)),
        _ => Err(format!("--target {triple}: FMI 2.0 に置き場の名前が無い")),
    }
}

/// `CC` の値から起動するコマンドを作る。`CC="ccache cc"` のように空白で区切った引数があってもよい。空なら `cc`。
fn host_cc(cc: &str) -> Command {
    let mut words = cc.split_whitespace();
    let mut c = Command::new(words.next().unwrap_or("cc"));
    c.args(words);
    c
}

/// `triple` が None なら host を `cc`（`CC` で上書き可）で、Some なら `zig cc -target <triple>` で作る。
fn build(src: &MeSources, root: &Path, headers: &Path, triple: Option<&str>) -> Result<(), String> {
    if triple.is_none() && !(cfg!(target_os = "linux") && cfg!(target_arch = "x86_64")) {
        return Err("--target を付けない --build は x86_64 の Linux だけ".into());
    }
    let header = format!("fmi{}Functions.h", src.fmi_version);
    if !headers.join(&header).is_file() {
        return Err(format!("{} に {header} が無い", headers.display()));
    }
    let (platform, ext) = platform(triple.unwrap_or("x86_64-linux"), src.fmi_version)?;
    let bin = root.join("binaries").join(platform);
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let out = bin.join(format!("{}.{ext}", src.model_identifier));
    let sources = root.join("sources");
    let mut cc = match triple {
        None => host_cc(&std::env::var("CC").unwrap_or_default()),
        Some(t) => {
            let mut c = Command::new(std::env::var_os("ZIG").unwrap_or("zig".into()));
            c.args(["cc", "-target", t]);
            c
        }
    };
    cc.args(["-shared", "-fPIC", "-O2", "-std=c99"])
        .arg("-I")
        .arg(headers)
        .arg("-I")
        .arg(&sources);
    // FMI 3.0 の buildDescription.xml の前処理の定義（Rumoca の fmi3 は FMI3_OVERRIDE_FUNCTION_PREFIX=1 で接頭辞なしの ABI を出す）
    for (path, xml) in &src.c_sources {
        if path != "sources/buildDescription.xml" {
            continue;
        }
        let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
        for d in doc
            .descendants()
            .filter(|n| n.has_tag_name("PreprocessorDefinition"))
        {
            let name = d
                .attribute("name")
                .ok_or("PreprocessorDefinition に name が無い")?;
            cc.arg(match d.attribute("value") {
                Some(v) => format!("-D{name}={v}"),
                None => format!("-D{name}"),
            });
        }
    }
    for (path, _) in src.c_sources.iter().filter(|(p, _)| p.ends_with(".c")) {
        cc.arg(root.join(path));
    }
    cc.arg("-o").arg(&out).arg("-lm");
    let status = cc.output().map_err(|e| format!("cc を起動できない: {e}"))?;
    if !status.status.success() {
        return Err(format!(
            "cc が失敗した:\n{}",
            String::from_utf8_lossy(&status.stderr)
        ));
    }
    // zig cc の Windows 向けは .pdb と import library（.lib）も置くので、共有ライブラリだけを残す
    for e in std::fs::read_dir(&bin).map_err(|e| e.to_string())? {
        let path = e.map_err(|e| e.to_string())?.path();
        if matches!(
            path.extension().and_then(|x| x.to_str()),
            Some("pdb" | "lib")
        ) {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn zip_dir(root: &Path, fmu: &Path) -> Result<(), String> {
    let file = std::fs::File::create(fmu).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path)
            } else {
                files.push(path)
            }
        }
    }
    files.sort();
    for path in files {
        let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
        let name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&std::fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{host_cc, platform};

    #[test]
    fn cc_with_arguments_is_split_on_whitespace() {
        let c = host_cc("ccache  gcc -m64");
        assert_eq!(c.get_program(), "ccache");
        assert_eq!(c.get_args().collect::<Vec<_>>(), ["gcc", "-m64"]);
        assert_eq!(host_cc("").get_program(), "cc");
        assert_eq!(host_cc("  ").get_program(), "cc");
    }

    #[test]
    fn platform_names() {
        let p = |t, v| platform(t, v).unwrap();
        assert_eq!(p("x86_64-linux-gnu", 2), ("linux64".into(), "so"));
        assert_eq!(p("x86_64-windows-gnu", 2), ("win64".into(), "dll"));
        assert_eq!(p("aarch64-macos", 2), ("darwin64".into(), "dylib"));
        assert_eq!(p("x86_64-windows-gnu", 3), ("x86_64-windows".into(), "dll"));
        assert_eq!(p("aarch64-macos", 3), ("aarch64-darwin".into(), "dylib"));
        assert!(platform("aarch64-linux-gnu", 2).is_err());
        assert!(platform("wasm32-wasi", 3).is_err());
    }
}

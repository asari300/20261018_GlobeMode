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
    /// FMU のファイル名の元（モデルの完全修飾名の `.` を `_` にしたもの）。
    pub name: String,
    /// `ModelExchange` の `modelIdentifier`（共有ライブラリの名前）。
    pub model_identifier: String,
    pub model_description_xml: String,
    /// `sources/` の下のファイル（FMU の中の相対パスと中身）。
    pub c_sources: Vec<(String, String)>,
}

/// モデルを FMI `fmi_version`（2 か 3）のソースの FMU にする。Rumoca の対象外（イベント・外部関数など）は `Err`。
pub fn export_me_sources(model: &CompiledModel, fmi_version: u8) -> Result<MeSources, String> {
    if !matches!(fmi_version, 2 | 3) {
        return Err(format!("FMI {fmi_version} は未対応（2 か 3）"));
    }
    let target = format!("fmi{fmi_version}");
    let mut files = globe_mode_frontend::fmi_export::render_rumoca_fmi(model, &target)?;
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

/// `out_dir/<name>/`（展開した FMU）と `out_dir/<name>.fmu` を書き、`.fmu` のパスを返す。
///
/// `fmi_headers` を渡すと、その中の `fmi2Functions.h`（3.0 は `fmi3Functions.h`）などで `cc` を呼び、
/// `binaries/linux64/`（3.0 は `binaries/x86_64-linux/`）に `<modelIdentifier>.so` も入れる（x86_64 の Linux だけ）。
pub fn write_fmu(
    src: &MeSources,
    out_dir: &Path,
    fmi_headers: Option<&Path>,
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
        build_linux64(src, &root, headers)?;
    }
    let fmu = out_dir.join(format!("{}.fmu", src.name));
    zip_dir(&root, &fmu)?;
    Ok(fmu)
}

fn build_linux64(src: &MeSources, root: &Path, headers: &Path) -> Result<(), String> {
    if !(cfg!(target_os = "linux") && cfg!(target_arch = "x86_64")) {
        return Err("--build は x86_64 の Linux だけ".into());
    }
    let header = format!("fmi{}Functions.h", src.fmi_version);
    if !headers.join(&header).is_file() {
        return Err(format!("{} に {header} が無い", headers.display()));
    }
    // FMI 2.0 と 3.0 でプラットフォームの名前が違う
    let platform = if src.fmi_version == 3 {
        "x86_64-linux"
    } else {
        "linux64"
    };
    let bin = root.join("binaries").join(platform);
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let out = bin.join(format!("{}.so", src.model_identifier));
    let sources = root.join("sources");
    let mut cc = Command::new(std::env::var_os("CC").unwrap_or("cc".into()));
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

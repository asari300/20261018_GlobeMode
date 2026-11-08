//! `globe fmu export --runtime p5b`（S238。本体は S239 から facade の [`globe_mode::export_fmu_p5b`]。
//! [ADR-0031](../../../../../doc/adr/0031-fmi-export-p5a-first.md)）。ここは引数を読み、終了コードにするだけ。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use globe_mode::P5bError;

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
    /// `runtime` は p5b にして渡された設定（fmi_version・runtime_lib、p5a の旗）
    pub options: &'a globe_mode::FmuOptions,
}

pub fn export(a: &Args) -> ExitCode {
    let usage = |e: String| P5bError::Usage(e);
    let text = match a.files {
        [] if !a.library.is_empty() => Ok(None),
        [f] if f.is_file() => std::fs::read_to_string(f)
            .map(Some)
            .map_err(|e| usage(format!("{} を読めない: {e}", f.display()))),
        _ => Err(usage(
            "--runtime p5b のモデルは 1 つの .mo ファイル、または -L のライブラリの中のクラス（ファイルは渡さない）"
                .into(),
        )),
    };
    match text.and_then(|t| {
        globe_mode::export_fmu_p5b(a.model, t.as_deref(), a.files, a.library, a.out, a.options)
    }) {
        Ok(fmu) => {
            println!("{}", fmu.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(if matches!(e, P5bError::Usage(_)) {
                2
            } else {
                1
            })
        }
    }
}

//! `globe results`（S219）。結果の MAT v4 / CSV を読み、CSV か MAT v4 に書き直す。
//!
//! 形は拡張子で決める（入力も出力も `.mat` は MAT v4、それ以外は CSV。`globe simulate -o` と同じ規則）。
//! CSV にパラメータの列は無いので、MAT の `data_1` のパラメータは CSV へ移らない。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::simulate::OutputFormat;
use super::{check_writable_file, io_error};

pub(crate) fn results(input: &Path, output: Option<&Path>) -> ExitCode {
    // 入力を上書きしてしまう書き先は、読む前に断る（`-o` が入力と同じファイル）
    if let Some(path) = output.filter(|p| *p != Path::new("-"))
        && let Err(message) = check_writable_file(path, &[PathBuf::from(input)], &[])
    {
        eprintln!("error: 結果を {message}");
        return ExitCode::from(2);
    }
    let contents = match globe_mode::sim::mat::read_results(input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", io_error(input, false, &e));
            return ExitCode::from(2);
        }
    };
    let write = |out: &mut dyn Write| match OutputFormat::resolve(None, output) {
        OutputFormat::Csv => globe_mode::sim::store::write_csv(&contents.trajectory, out),
        OutputFormat::Mat => globe_mode::sim::mat::write_mat4_parts(
            &contents.trajectory,
            &contents.parameters,
            (0.0, 0.0),
            out,
        ),
    };
    let result = match output.filter(|p| *p != Path::new("-")) {
        Some(path) => std::fs::File::create(path).and_then(|file| {
            let mut out = std::io::BufWriter::new(file);
            write(&mut out)?;
            out.flush()
        }),
        None => {
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            write(&mut out).and_then(|()| out.flush())
        }
    };
    match result {
        Ok(()) => {
            if let Some(path) = output.filter(|p| *p != Path::new("-")) {
                eprintln!(
                    "書いた: {}（{} 点、変数 {}）",
                    path.display(),
                    contents.trajectory.time.len(),
                    contents.trajectory.variables.len()
                );
            }
            ExitCode::SUCCESS
        }
        // `| head` で読み手が閉じたのは誤りではない
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "error: 結果を {}",
                io_error(output.unwrap_or(Path::new("-")), true, &e)
            );
            ExitCode::from(2)
        }
    }
}

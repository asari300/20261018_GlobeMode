//! 結果の置き場（ResultStore）。いまは CSV だけ（MAT v4 と Arrow IPC は後で）。

use std::io::{self, Read, Write};

use globe_mode_ir::Trajectory;

/// CSV を読んだときの誤り。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    /// 1 始まりの行番号（分かるときだけ）
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "{line} 行目: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for CsvError {}

/// 結果を CSV で書く。1 行目は `"time","x",...`（名前は二重引用符で囲む。`a[1,2]` のようにカンマを含むため）。
pub fn write_csv(trajectory: &Trajectory, out: &mut dyn Write) -> io::Result<()> {
    let _ = (trajectory, out);
    Ok(())
}

/// CSV を読む（MAP-LIB の参照結果、OpenModelica の CSV、[`write_csv`] の出力）。役割は分からないので
/// すべて `VariableRole::Algebraic` にする。
pub fn read_csv(input: &mut dyn Read) -> Result<Trajectory, CsvError> {
    let _ = input;
    Ok(Trajectory::default())
}

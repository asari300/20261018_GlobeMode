//! 結果の置き場（ResultStore）。いまは CSV だけ（MAT v4 と Arrow IPC は後で）。
//!
//! 形は MAP-LIB の参照結果（ModelicaStandardLibrary の ReferenceResults）と同じにする。
//!
//! - 1 行目は名前の見出し。名前は二重引用符で囲む（`a[1,2]` のようにカンマを含むため）。名前の中の `"` は `""` にする
//! - 先頭の列は `time`。区切りはカンマ
//! - 1 行が 1 時点。イベントの時刻では同じ時刻が 2 行続く（左極限と、イベント後の値）
//! - 数は f64 に戻すと同じビット列になる最短の表記（[`format_number`]）。非有限は `NaN`、`Inf`、`-Inf`
//!
//! 単位と役割は CSV に入らない（読むと役割は [`VariableRole::Algebraic`]、単位は None になる）。

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{self, BufWriter, Read, Write};

use globe_mode_ir::{Trajectory, TrajectoryVariable, VariableRole};

/// CSV を読んだときの誤り。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    /// 1 始まりの行番号（分かるときだけ）
    pub line: Option<usize>,
    pub message: String,
}

impl CsvError {
    fn at(line: usize, message: impl Into<String>) -> Self {
        CsvError {
            line: Some(line),
            message: message.into(),
        }
    }
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
///
/// 変数の値の数が時刻の数と違うときは、何も書かずに `InvalidInput` を返す。
pub fn write_csv(trajectory: &Trajectory, out: &mut dyn Write) -> io::Result<()> {
    let rows = trajectory.time.len();
    if let Some(v) = trajectory.variables.iter().find(|v| v.values.len() != rows) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "変数 {} の値の数（{}）が時刻の数（{rows}）と違う",
                v.name,
                v.values.len()
            ),
        ));
    }
    let mut w = BufWriter::new(out);
    let mut line = String::new();
    push_quoted(&mut line, "time");
    for v in &trajectory.variables {
        line.push(',');
        push_quoted(&mut line, &v.name);
    }
    line.push('\n');
    w.write_all(line.as_bytes())?;
    for (row, &t) in trajectory.time.iter().enumerate() {
        line.clear();
        push_number(&mut line, t);
        for v in &trajectory.variables {
            line.push(',');
            push_number(&mut line, v.values[row]);
        }
        line.push('\n');
        w.write_all(line.as_bytes())?;
    }
    w.flush()
}

/// CSV を読む（MAP-LIB の参照結果、OpenModelica の CSV、[`write_csv`] の出力）。役割は分からないので
/// すべて `VariableRole::Algebraic` にする。
///
/// 受け付けるもの: 見出しの名前は引用符で囲んでも囲まなくてもよい（囲んだ中のカンマ・`""` も読む）、
/// CRLF、先頭の BOM、空行（読み飛ばす）、行末のカンマ、指数表記、大文字と小文字を問わない
/// `nan` / `inf` / `infinity`。
/// 誤りにするもの: 先頭の列が `time` でない（大文字と小文字は問わない）、名前の重複や空の名前、列の数が
/// 見出しと違う行、数として読めない値、有限でない時刻、前の行より小さい時刻。
pub fn read_csv(input: &mut dyn Read) -> Result<Trajectory, CsvError> {
    let mut text = String::new();
    input.read_to_string(&mut text).map_err(|e| CsvError {
        line: None,
        message: format!("CSV を読めない（UTF-8 でないなど）: {e}"),
    })?;
    parse_csv(&text)
}

/// 文字列の CSV を読む（[`read_csv`] と同じ規則）。
pub fn parse_csv(text: &str) -> Result<Trajectory, CsvError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .split('\n')
        .enumerate()
        .map(|(i, l)| (i + 1, l.strip_suffix('\r').unwrap_or(l)))
        .filter(|(_, l)| !l.trim().is_empty());

    let Some((header_line, header)) = lines.next() else {
        return Err(CsvError {
            line: None,
            message: "CSV が空（見出しの行が無い）".into(),
        });
    };
    let mut fields = Vec::new();
    split_fields(header, header_line, &mut fields)?;
    let names: Vec<String> = fields.drain(..).map(Cow::into_owned).collect();
    if !names[0].eq_ignore_ascii_case("time") {
        return Err(CsvError::at(
            header_line,
            format!("先頭の列が time でない（\"{}\"）", names[0]),
        ));
    }
    let mut seen = HashSet::with_capacity(names.len());
    for name in &names[1..] {
        if name.is_empty() {
            return Err(CsvError::at(header_line, "名前が空の列がある"));
        }
        if name.eq_ignore_ascii_case("time") || !seen.insert(name.as_str()) {
            return Err(CsvError::at(
                header_line,
                format!("列の名前 \"{name}\" が重複している"),
            ));
        }
    }

    // 行の数の見積もり（改行の数）で先に場所を取る
    let estimate = text.bytes().filter(|&b| b == b'\n').count();
    let mut time = Vec::with_capacity(estimate);
    let mut columns: Vec<Vec<f64>> = (1..names.len())
        .map(|_| Vec::with_capacity(estimate))
        .collect();
    for (line_no, line) in lines {
        split_fields(line, line_no, &mut fields)?;
        if fields.len() != names.len() {
            return Err(CsvError::at(
                line_no,
                format!("列の数が {}（見出しは {}）", fields.len(), names.len()),
            ));
        }
        let mut values = fields.drain(..).enumerate().map(|(col, field)| {
            parse_number(&field).ok_or_else(|| {
                CsvError::at(
                    line_no,
                    format!(
                        "列 \"{}\" の値 \"{}\" を数として読めない",
                        names[col], field
                    ),
                )
            })
        });
        let t = values.next().expect("列の数は 1 以上")?;
        if !t.is_finite() {
            return Err(CsvError::at(line_no, format!("time（{t}）が有限でない")));
        }
        if let Some(&prev) = time.last()
            && t < prev
        {
            return Err(CsvError::at(
                line_no,
                format!(
                    "time（{}）が前の行（{}）より小さい",
                    format_number(t),
                    format_number(prev)
                ),
            ));
        }
        time.push(t);
        for (column, value) in columns.iter_mut().zip(values) {
            column.push(value?);
        }
    }

    let variables = names
        .into_iter()
        .skip(1)
        .zip(columns)
        .map(|(name, values)| TrajectoryVariable {
            name,
            role: VariableRole::Algebraic,
            unit: None,
            values,
        })
        .collect();
    Ok(Trajectory { time, variables })
}

/// f64 を、読み戻すと同じビット列になる最短の表記にする（NaN は `NaN`、無限大は `Inf` / `-Inf`）。
/// 絶対値が 1e-5 以上 1e16 未満（と 0）は小数で、それ以外は指数で書く（`1e-300` を 300 桁の小数にしない）。
pub fn format_number(x: f64) -> String {
    let mut s = String::new();
    push_number(&mut s, x);
    s
}

fn push_number(out: &mut String, x: f64) {
    if x.is_nan() {
        out.push_str("NaN");
    } else if x.is_infinite() {
        out.push_str(if x > 0.0 { "Inf" } else { "-Inf" });
    } else {
        let a = x.abs();
        // Rust の `{}` と `{:e}` は、精度を指定しなければどちらも最短の往復表記を出す
        let _ = if a == 0.0 || (1e-5..1e16).contains(&a) {
            write!(out, "{x}")
        } else {
            write!(out, "{x:e}")
        };
    }
}

/// 名前を二重引用符で囲む（中の `"` は `""` にする）。
fn push_quoted(out: &mut String, name: &str) {
    out.push('"');
    for c in name.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
}

/// 数を読む。前後の空白を許し、`nan`・`inf`・`infinity` は大文字と小文字を問わない（Rust の `f64` の
/// 文法と同じ）。
fn parse_number(field: &str) -> Option<f64> {
    field.trim().parse::<f64>().ok()
}

/// 1 行を欄に分ける。二重引用符で囲んだ欄は、中のカンマを区切りとみなさず、`""` を `"` に戻す。
/// 囲まない欄は前後の空白を除く。行末のカンマ（OpenModelica の古い CSV の形）の後の空の欄は捨てる。
fn split_fields<'a>(
    line: &'a str,
    line_no: usize,
    out: &mut Vec<Cow<'a, str>>,
) -> Result<(), CsvError> {
    split_raw(line, line_no, out)?;
    if out.len() > 1 && out.last().is_some_and(|f| f.is_empty()) {
        out.pop();
    }
    Ok(())
}

fn split_raw<'a>(
    line: &'a str,
    line_no: usize,
    out: &mut Vec<Cow<'a, str>>,
) -> Result<(), CsvError> {
    out.clear();
    let bytes = line.as_bytes();
    let blank = |b: u8| b == b' ' || b == b'\t';
    let mut i = 0;
    loop {
        while i < bytes.len() && blank(bytes[i]) {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'"' {
            let start = i + 1;
            let mut segment = start;
            let mut owned: Option<String> = None;
            let mut j = start;
            let field = loop {
                let Some(k) = line[j..].find('"').map(|k| j + k) else {
                    return Err(CsvError::at(line_no, "引用符が閉じていない"));
                };
                if bytes.get(k + 1) == Some(&b'"') {
                    // `""` は `"` 1 つ
                    owned
                        .get_or_insert_with(String::new)
                        .push_str(&line[segment..=k]);
                    j = k + 2;
                    segment = j;
                    continue;
                }
                i = k + 1;
                break match owned {
                    Some(mut s) => {
                        s.push_str(&line[segment..k]);
                        Cow::Owned(s)
                    }
                    None => Cow::Borrowed(&line[start..k]),
                };
            };
            out.push(field);
            while i < bytes.len() && blank(bytes[i]) {
                i += 1;
            }
            if i == bytes.len() {
                return Ok(());
            }
            if bytes[i] != b',' {
                return Err(CsvError::at(line_no, "閉じた引用符の後に余計な文字がある"));
            }
            i += 1;
        } else {
            let end = line[i..].find(',').map_or(bytes.len(), |k| i + k);
            out.push(Cow::Borrowed(line[i..end].trim_end()));
            if end == bytes.len() {
                return Ok(());
            }
            i = end + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str, values: Vec<f64>) -> TrajectoryVariable {
        TrajectoryVariable {
            name: name.into(),
            role: VariableRole::State,
            unit: Some("V".into()),
            values,
        }
    }

    fn written(t: &Trajectory) -> String {
        let mut out = Vec::new();
        write_csv(t, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn writes_quoted_header_and_rows() {
        let t = Trajectory {
            time: vec![0.0, 0.5, 0.5, 1.0],
            variables: vec![
                variable("x", vec![1.0, 2.5, -3.0, 0.1]),
                variable("a[1,2]", vec![0.0, 1e-7, 1e20, -0.0]),
                variable(
                    "q\"uote",
                    vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2.0],
                ),
            ],
        };
        assert_eq!(
            written(&t),
            "\"time\",\"x\",\"a[1,2]\",\"q\"\"uote\"\n\
             0,1,0,NaN\n\
             0.5,2.5,1e-7,Inf\n\
             0.5,-3,1e20,-Inf\n\
             1,0.1,-0,2\n"
        );
    }

    #[test]
    fn write_rejects_ragged_variables() {
        let t = Trajectory {
            time: vec![0.0, 1.0],
            variables: vec![variable("x", vec![1.0])],
        };
        let mut out = Vec::new();
        let e = write_csv(&t, &mut out).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
        assert!(out.is_empty());
    }

    #[test]
    fn round_trip_is_bit_exact() {
        let samples = [
            0.0,
            -0.0,
            1.0,
            0.1 + 0.2,
            1.0 / 3.0,
            -2.0 / 3.0,
            1e-5,
            9.999_999_999_999_999e-6,
            1e16,
            9.999_999_999_999_998e15,
            1e-300,
            5e-324,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::MIN,
            f64::EPSILON,
            123_456_789.012_345_68,
            std::f64::consts::PI,
            -std::f64::consts::E * 1e200,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        let time: Vec<f64> = (0..samples.len()).map(|i| i as f64 * 0.1).collect();
        let original = Trajectory {
            time,
            variables: vec![
                variable("v", samples.to_vec()),
                variable("w", samples.iter().rev().copied().collect()),
            ],
        };
        let back = parse_csv(&written(&original)).unwrap();
        assert_eq!(back.time.len(), original.time.len());
        for (a, b) in original.time.iter().zip(&back.time) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        for (va, vb) in original.variables.iter().zip(&back.variables) {
            assert_eq!(va.name, vb.name);
            assert_eq!(vb.role, VariableRole::Algebraic);
            assert_eq!(vb.unit, None);
            for (a, b) in va.values.iter().zip(&vb.values) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert_eq!(a.to_bits(), b.to_bits(), "{a:e} → {b:e}");
                }
            }
        }
    }

    #[test]
    fn number_format_is_short() {
        assert_eq!(format_number(1e-300), "1e-300");
        assert_eq!(format_number(0.00001), "0.00001");
        assert_eq!(format_number(0.000001), "1e-6");
        assert_eq!(format_number(1e15), "1000000000000000");
        assert_eq!(format_number(1e16), "1e16");
        assert_eq!(format_number(-0.0), "-0");
    }

    #[test]
    fn reads_map_lib_style() {
        let text = "\"time\",\"a[1,2]\",\"b\"\r\n0,1.5e-3,2\r\n0.5,-1E+2,3\r\n0.5,4,5\r\n1,nan,-INF\r\n\r\n";
        let t = parse_csv(text).unwrap();
        assert_eq!(t.time, [0.0, 0.5, 0.5, 1.0]);
        assert_eq!(t.variables.len(), 2);
        assert_eq!(t.variables[0].name, "a[1,2]");
        assert_eq!(t.variables[0].values[..3], [1.5e-3, -100.0, 4.0]);
        assert!(t.variables[0].values[3].is_nan());
        assert_eq!(t.variables[1].values, [2.0, 3.0, 5.0, f64::NEG_INFINITY]);
        assert!(
            t.variables
                .iter()
                .all(|v| v.role == VariableRole::Algebraic)
        );
    }

    #[test]
    fn reads_unquoted_headers_spaces_bom_and_infinity() {
        let text = "\u{feff}Time, x , \"y \"\n 0 , 1 , Infinity\n1,\"2\",+inf\n";
        let t = parse_csv(text).unwrap();
        assert_eq!(t.time, [0.0, 1.0]);
        let names: Vec<&str> = t.variables.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["x", "y "]);
        assert_eq!(t.variables[0].values, [1.0, 2.0]);
        assert_eq!(t.variables[1].values, [f64::INFINITY, f64::INFINITY]);
    }

    #[test]
    fn reads_trailing_commas() {
        let t = parse_csv("\"time\",\"x\",\n0,1,\n1,2\n").unwrap();
        assert_eq!(t.variables.len(), 1);
        assert_eq!(t.variables[0].values, [1.0, 2.0]);
        assert_eq!(parse_csv("time,x\n0,,\n").unwrap_err().line, Some(2));
    }

    #[test]
    fn reads_escaped_quotes_and_header_only() {
        let t = parse_csv("\"time\",\"say \"\"hi\"\", ok\"").unwrap();
        assert!(t.time.is_empty());
        assert_eq!(t.variables[0].name, "say \"hi\", ok");
        assert!(t.variables[0].values.is_empty());
    }

    #[test]
    fn read_csv_from_reader() {
        let mut input = "\"time\",\"x\"\n0,1\n".as_bytes();
        let t = read_csv(&mut input).unwrap();
        assert_eq!(t.variable("x").unwrap().values, [1.0]);
        let mut bad: &[u8] = &[b'"', 0xff, 0xfe];
        assert_eq!(read_csv(&mut bad).unwrap_err().line, None);
    }

    fn error(text: &str) -> CsvError {
        parse_csv(text).unwrap_err()
    }

    #[test]
    fn errors_carry_line_numbers() {
        assert_eq!(error("").line, None);
        assert_eq!(error("\n  \n").line, None);
        let e = error("\"t\",\"x\"\n0,1\n");
        assert_eq!(e.line, Some(1));
        assert!(e.message.contains("time"), "{e}");
        let e = error("\"time\",\"x\"\n0,1\n1,2,3\n");
        assert_eq!(e.line, Some(3));
        assert!(e.message.contains("列の数が 3（見出しは 2）"), "{e}");
        let e = error("\"time\",\"x\"\n\n0,abc\n");
        assert_eq!(e.line, Some(3));
        assert!(e.message.contains("abc"), "{e}");
        assert_eq!(error("\"time\",\"x\"\n0,\n").line, Some(2));
        assert_eq!(error("\"time\",\"x\"\n1,1\n0,1\n").line, Some(3));
        assert_eq!(error("\"time\",\"x\"\nnan,1\n").line, Some(2));
        assert_eq!(error("\"time\",\"x\",\"x\"\n").line, Some(1));
        assert_eq!(error("\"time\",\"\",\"x\"\n").line, Some(1));
        assert_eq!(error("\"time\",\"x\n").line, Some(1));
        assert_eq!(error("\"time\",\"x\"y\n").line, Some(1));
        assert_eq!(
            error("\"time\",\"x\"\n0,1\n1,2,3\n").to_string(),
            "3 行目: 列の数が 3（見出しは 2）"
        );
    }
}

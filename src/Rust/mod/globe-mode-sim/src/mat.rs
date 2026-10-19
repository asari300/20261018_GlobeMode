//! 結果の MAT v4 ファイル（OpenModelica・Dymola の dsres 形式、binTrans。S012）。
//!
//! 配置は omc 1.27.1 が書いた実物（`simulate` の `*_res.mat`）で確かめた（コードは読んでも写していない）。
//! 行列を 6 つ、この順に並べる。どれも頭は int32 が 5 つ（MOPT、行の数、列の数、虚部の有無 0、
//! 名前の長さ（NUL を含む））で、続けて NUL で終わる名前、列優先（Fortran の順）のデータ。
//! 数はリトルエンディアン（MOPT の千の位 0）。MOPT は double の数値 0、int32 の数値 20、文字（uint8）51。
//!
//! | 行列 | 形 | 中身 |
//! | --- | --- | --- |
//! | `Aclass` | 文字 4 × 11 | 行が `Atrajectory`・`1.1`・空・`binTrans`（これだけは行が 1 つの文字列） |
//! | `name` | 文字（最長の長さ）× 変数の数 | 1 列が 1 つの名前（NUL で埋める）。1 つ目は `time` |
//! | `description` | 文字 × 変数の数 | 説明。単位があれば ` [単位]` を後ろに付ける（omc と同じ。説明が空でも付ける） |
//! | `dataInfo` | int32 4 × 変数の数 | 変数ごとに（ブロック, 1 始まりの行, 補間 0, 外挿）。time は (0, 1, 0, −1) |
//! | `data_1` | double（パラメータの数 + 1）× 2 | 1 行目は time の [t0, t1]、残りはパラメータ（両列に同じ値） |
//! | `data_2` | double（列の数 + 1）× 時点の数 | 1 列が 1 時点。1 行目は time |
//!
//! パラメータは `dataInfo` が (1, 行, 0, 0)、時系列は (2, 行, 0, 0)。時系列のうち、前の列と値がすべて同じ列は
//! その列の別名 (2, 行) に、符号だけが反対の列は負の別名 (2, −行) に畳む（`data_2` に列を足さない）。
//! イベントの時刻では同じ時刻の時点が 2 つ続く（そのまま書く）。Integer・Boolean も double で書く。

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};

use globe_mode_ir::{ResultParameter, SimRun, Trajectory, TrajectoryVariable, VariableRole};

/// MOPT: double の数値行列。
const MOPT_DOUBLE: i32 = 0;
/// MOPT: int32 の数値行列。
const MOPT_INT32: i32 = 20;
/// MOPT: 文字（uint8）の行列。
const MOPT_TEXT: i32 = 51;

/// `Aclass` の 4 行。
const ACLASS: [&str; 4] = ["Atrajectory", "1.1", "", "binTrans"];

/// time の説明（omc と同じ）。
const TIME_DESCRIPTION: &str = "Simulation time [s]";

/// [`read_mat4`] で読んだもの。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatContents {
    /// 時系列（役割は分からないので、すべて [`VariableRole::Algebraic`]。別名も 1 つの変数として展開する）
    pub trajectory: Trajectory,
    /// `data_1` のパラメータ（time を除く）
    pub parameters: Vec<ResultParameter>,
}

/// 結果を MAT v4（binTrans）で書く。`run.parameters` を `data_1` に、時系列を `data_2` に入れる。
///
/// 時系列のうち `run.parameters` と同じ名前の変数は書かない（`data_1` の値を使う）。名前が `time` の変数も
/// 書かない。変数の値の数が時刻の数と違うときは、何も書かずに `InvalidInput` を返す。
pub fn write_mat4(run: &SimRun, out: &mut dyn Write) -> io::Result<()> {
    let trajectory = &run.trajectory;
    let n_time = trajectory.time.len();
    if let Some(v) = trajectory
        .variables
        .iter()
        .find(|v| v.values.len() != n_time)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "変数 {} の値の数（{}）が時刻の数（{n_time}）と違う",
                v.name,
                v.values.len()
            ),
        ));
    }

    let mut names: Vec<&str> = vec!["time"];
    let mut descriptions: Vec<String> = vec![TIME_DESCRIPTION.to_string()];
    let mut info: Vec<[i32; 4]> = vec![[0, 1, 0, -1]];

    // data_1: time の [t0, t1] とパラメータ
    let (t0, t1) = match (trajectory.time.first(), trajectory.time.last()) {
        (Some(&a), Some(&b)) => (a, b),
        _ => (run.settings.start_time, run.settings.stop_time),
    };
    let mut data_1 = vec![t0];
    for p in &run.parameters {
        names.push(&p.name);
        descriptions.push(described(p.description.as_deref(), p.unit.as_deref()));
        data_1.push(p.value);
        info.push([1, to_i32(data_1.len())?, 0, 0]);
    }

    // data_2: 1 行目は time。同じ列・符号だけ反対の列は別名に畳む
    let skip: std::collections::HashSet<&str> =
        run.parameters.iter().map(|p| p.name.as_str()).collect();
    let mut columns: Vec<&[f64]> = vec![&trajectory.time];
    let mut seen: HashMap<u64, Vec<usize>> = HashMap::new();
    for v in &trajectory.variables {
        if v.name == "time" || skip.contains(v.name.as_str()) {
            continue;
        }
        names.push(&v.name);
        descriptions.push(described(v.description.as_deref(), v.unit.as_deref()));
        let row = match find_alias(&seen, &columns, &v.values) {
            Some(row) => row,
            None => {
                columns.push(&v.values);
                let index = columns.len() - 1;
                seen.entry(column_hash(&v.values, false))
                    .or_default()
                    .push(index);
                to_i32(index + 1)?
            }
        };
        info.push([2, row, 0, 0]);
    }

    write_text(out, "Aclass", &ACLASS)?;
    write_text(out, "name", &names)?;
    let descriptions: Vec<&str> = descriptions.iter().map(String::as_str).collect();
    write_text(out, "description", &descriptions)?;
    write_header(out, MOPT_INT32, 4, info.len(), "dataInfo")?;
    for column in &info {
        for x in column {
            out.write_all(&x.to_le_bytes())?;
        }
    }
    write_header(out, MOPT_DOUBLE, data_1.len(), 2, "data_1")?;
    // 列優先: 1 列目は [t0, パラメータ…]、2 列目は [t1, パラメータ…]
    for time in [t0, t1] {
        out.write_all(&time.to_le_bytes())?;
        for x in &data_1[1..] {
            out.write_all(&x.to_le_bytes())?;
        }
    }
    write_header(out, MOPT_DOUBLE, columns.len(), n_time, "data_2")?;
    for k in 0..n_time {
        for c in &columns {
            out.write_all(&c[k].to_le_bytes())?;
        }
    }
    Ok(())
}

/// 説明に単位を付ける（omc と同じく ` [単位]`。説明が空でも付ける）。
fn described(description: Option<&str>, unit: Option<&str>) -> String {
    let description = description.unwrap_or("");
    match unit {
        Some(u) if !u.is_empty() => format!("{description} [{u}]"),
        _ => description.to_string(),
    }
}

/// 列の値を 1 つの数にする（`negate` なら符号を反対にした列の）。0 と −0 は同じに、NaN はどれも同じにする
/// （[`same`] の等しさと合わせる）。
fn column_hash(values: &[f64], negate: bool) -> u64 {
    let mut h = DefaultHasher::new();
    for &x in values {
        let x = if negate { -x } else { x };
        let bits = if x.is_nan() {
            f64::NAN.to_bits()
        } else if x == 0.0 {
            0
        } else {
            x.to_bits()
        };
        bits.hash(&mut h);
    }
    h.finish()
}

/// 値が等しいか（0 と −0 は等しい、NaN どうしも等しい）。
fn same(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

/// 前の列（time を除く）に同じ列があればその行（1 始まり）、符号だけ反対の列があれば負の行を返す。
fn find_alias(seen: &HashMap<u64, Vec<usize>>, columns: &[&[f64]], values: &[f64]) -> Option<i32> {
    let candidates = |hash: u64| seen.get(&hash).into_iter().flatten().copied();
    for index in candidates(column_hash(values, false)) {
        if columns[index].iter().zip(values).all(|(&a, &b)| same(a, b)) {
            return i32::try_from(index + 1).ok();
        }
    }
    for index in candidates(column_hash(values, true)) {
        if columns[index]
            .iter()
            .zip(values)
            .all(|(&a, &b)| same(-a, b))
        {
            return i32::try_from(index + 1).ok().map(|r| -r);
        }
    }
    None
}

fn to_i32(n: usize) -> io::Result<i32> {
    i32::try_from(n).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "行列が大きすぎる"))
}

fn write_header(
    out: &mut dyn Write,
    mopt: i32,
    rows: usize,
    columns: usize,
    name: &str,
) -> io::Result<()> {
    let name_len = to_i32(name.len() + 1)?;
    for x in [mopt, to_i32(rows)?, to_i32(columns)?, 0, name_len] {
        out.write_all(&x.to_le_bytes())?;
    }
    out.write_all(name.as_bytes())?;
    out.write_all(&[0])
}

/// 文字列を 1 列ずつの文字の行列で書く（行の数は最長のバイト数、短いものは NUL で埋める）。
/// `Aclass` だけは行が文字列なので、転置して書く。
fn write_text(out: &mut dyn Write, name: &str, strings: &[&str]) -> io::Result<()> {
    let longest = strings.iter().map(|s| s.len()).max().unwrap_or(0).max(1);
    if name == "Aclass" {
        write_header(out, MOPT_TEXT, strings.len(), longest, name)?;
        for k in 0..longest {
            for s in strings {
                out.write_all(&[s.as_bytes().get(k).copied().unwrap_or(0)])?;
            }
        }
        return Ok(());
    }
    write_header(out, MOPT_TEXT, longest, strings.len(), name)?;
    for s in strings {
        out.write_all(s.as_bytes())?;
        out.write_all(&vec![0; longest - s.len()])?;
    }
    Ok(())
}

/// 読んだ行列 1 つ。
struct Matrix {
    /// MOPT の P（0 double、1 float、2 int32、5 uint8）
    precision: i32,
    text: bool,
    rows: usize,
    columns: usize,
    data: Vec<u8>,
}

impl Matrix {
    /// 列優先の (行, 列) の数（文字の行列なら文字コード）。
    fn number(&self, row: usize, column: usize) -> f64 {
        let i = row + self.rows * column;
        let at = |n: usize| &self.data[i * n..(i + 1) * n];
        match self.precision {
            0 => f64::from_le_bytes(at(8).try_into().expect("8 バイト")),
            1 => f64::from(f32::from_le_bytes(at(4).try_into().expect("4 バイト"))),
            2 => f64::from(i32::from_le_bytes(at(4).try_into().expect("4 バイト"))),
            3 => f64::from(i16::from_le_bytes(at(2).try_into().expect("2 バイト"))),
            4 => f64::from(u16::from_le_bytes(at(2).try_into().expect("2 バイト"))),
            _ => f64::from(self.data[i]),
        }
    }

    /// 文字の行列の列 `column` を文字列にする（後ろの NUL と空白を除く）。
    fn column_text(&self, column: usize) -> String {
        let bytes: Vec<u8> = (0..self.rows)
            .map(|r| self.number(r, column) as u8)
            .collect();
        trimmed(&bytes)
    }

    /// 文字の行列の行 `row` を文字列にする（`Aclass` 用）。
    fn row_text(&self, row: usize) -> String {
        let bytes: Vec<u8> = (0..self.columns)
            .map(|c| self.number(row, c) as u8)
            .collect();
        trimmed(&bytes)
    }
}

fn trimmed(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|&b| b != 0 && b != b' ')
        .map_or(0, |i| i + 1);
    let start = bytes[..end].iter().position(|&b| b != 0).unwrap_or(end);
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// MAT v4 の結果（[`write_mat4`] の出力、omc の `*_res.mat`。binTrans でリトルエンディアンのもの）を読む。
///
/// 別名・負の別名は、指す行の値（負なら符号を反対にした値）を持つ変数に展開する。説明の後ろの ` [単位]` は
/// 単位として外す。binNormal（転置していない形）とビッグエンディアンは読まない（`InvalidData`）。
pub fn read_mat4(input: &mut dyn Read) -> io::Result<MatContents> {
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    let mut matrices: HashMap<String, Matrix> = HashMap::new();
    let mut off = 0;
    while off < bytes.len() {
        let head = bytes
            .get(off..off + 20)
            .ok_or_else(|| invalid("行列の頭が途中で切れている"))?;
        let int =
            |k: usize| i32::from_le_bytes(head[4 * k..4 * k + 4].try_into().expect("4 バイト"));
        let (mopt, rows, columns, imagf, name_len) = (int(0), int(1), int(2), int(3), int(4));
        if !(0..=51).contains(&mopt) || mopt / 1000 != 0 {
            return Err(invalid(format!(
                "MOPT {mopt} は読めない（リトルエンディアンだけ）"
            )));
        }
        if imagf != 0 {
            return Err(invalid("虚部のある行列は読めない"));
        }
        let (Ok(rows), Ok(columns), Ok(name_len)) = (
            usize::try_from(rows),
            usize::try_from(columns),
            usize::try_from(name_len),
        ) else {
            return Err(invalid("行列の大きさが負"));
        };
        let precision = (mopt / 10) % 10;
        let size = match precision {
            0 => 8,
            1 | 2 => 4,
            3 | 4 => 2,
            5 => 1,
            _ => return Err(invalid(format!("MOPT {mopt} の型は読めない"))),
        };
        off += 20;
        let name = bytes
            .get(off..off + name_len)
            .ok_or_else(|| invalid("行列の名前が途中で切れている"))?;
        let name = trimmed(name);
        off += name_len;
        let len = rows
            .checked_mul(columns)
            .and_then(|n| n.checked_mul(size))
            .ok_or_else(|| invalid("行列が大きすぎる"))?;
        let data = bytes
            .get(off..off + len)
            .ok_or_else(|| invalid(format!("行列 {name} のデータが途中で切れている")))?
            .to_vec();
        off += len;
        matrices.insert(
            name,
            Matrix {
                precision,
                text: mopt % 10 == 1,
                rows,
                columns,
                data,
            },
        );
    }

    let get = |name: &str| {
        matrices
            .get(name)
            .ok_or_else(|| invalid(format!("行列 {name} が無い")))
    };
    let aclass = get("Aclass")?;
    if !aclass.text || aclass.rows < 4 || aclass.row_text(3) != "binTrans" {
        return Err(invalid("Aclass が binTrans でない"));
    }
    let names = get("name")?;
    let info = get("dataInfo")?;
    let data_1 = get("data_1")?;
    let data_2 = get("data_2")?;
    let descriptions = matrices.get("description");
    let n = names.columns;
    if info.rows < 2 || info.columns != n {
        return Err(invalid("dataInfo の形が name と合わない"));
    }
    let n_time = data_2.columns;
    let mut contents = MatContents::default();
    let mut time_row = 0;
    for i in 0..n {
        let name = names.column_text(i);
        let (description, unit) = match descriptions {
            Some(d) if i < d.columns => split_unit(&d.column_text(i)),
            _ => (None, None),
        };
        let block = info.number(0, i) as i64;
        let row = info.number(1, i) as i64;
        let index = usize::try_from(row.unsigned_abs())
            .ok()
            .and_then(|r| r.checked_sub(1))
            .ok_or_else(|| invalid(format!("変数 {name} の行が 0")))?;
        let sign = if row < 0 { -1.0 } else { 1.0 };
        if name == "time" && i == 0 {
            time_row = index;
            continue;
        }
        match block {
            1 => {
                if index >= data_1.rows || data_1.columns == 0 {
                    return Err(invalid(format!("変数 {name} の行が data_1 の外")));
                }
                contents.parameters.push(ResultParameter {
                    name,
                    value: sign * data_1.number(index, 0),
                    unit,
                    description,
                });
            }
            0 | 2 => {
                if index >= data_2.rows {
                    return Err(invalid(format!("変数 {name} の行が data_2 の外")));
                }
                contents.trajectory.variables.push(TrajectoryVariable {
                    name,
                    role: VariableRole::Algebraic,
                    unit,
                    description,
                    values: (0..n_time)
                        .map(|k| sign * data_2.number(index, k))
                        .collect(),
                });
            }
            _ => {
                return Err(invalid(format!(
                    "変数 {name} のブロック {block} は読めない"
                )));
            }
        }
    }
    if n_time > 0 && time_row >= data_2.rows {
        return Err(invalid("time の行が data_2 の外"));
    }
    contents.trajectory.time = (0..n_time).map(|k| data_2.number(time_row, k)).collect();
    Ok(contents)
}

/// 説明の後ろの ` [単位]` を外す（説明, 単位）。空の説明は None。
fn split_unit(text: &str) -> (Option<String>, Option<String>) {
    let some = |s: &str| (!s.is_empty()).then(|| s.to_string());
    if let Some(body) = text.strip_suffix(']')
        && let Some(open) = body.rfind('[')
        && (open == 0 || body[..open].ends_with(' '))
    {
        let description = body[..open].strip_suffix(' ').unwrap_or(&body[..open]);
        return (some(description), some(&body[open + 1..]));
    }
    (some(text), None)
}

#[cfg(test)]
mod tests {
    use globe_mode_ir::{
        ResolvedSettings, SettingOrigin, SettingOrigins, SimStats, SolverId, Termination,
    };

    use super::*;

    fn variable(name: &str, values: Vec<f64>) -> TrajectoryVariable {
        TrajectoryVariable {
            name: name.to_string(),
            role: VariableRole::State,
            unit: None,
            description: None,
            values,
        }
    }

    fn run(
        time: Vec<f64>,
        variables: Vec<TrajectoryVariable>,
        parameters: Vec<ResultParameter>,
    ) -> SimRun {
        let o = SettingOrigin::Default;
        SimRun {
            model: "M".to_string(),
            settings: ResolvedSettings {
                start_time: 0.0,
                stop_time: 1.0,
                tolerance: 1e-6,
                interval: 0.5,
                solver: SolverId::Bdf,
                max_wall_seconds: None,
                step: None,
                parameters: Default::default(),
                origins: SettingOrigins {
                    start_time: o,
                    stop_time: o,
                    tolerance: o,
                    interval: o,
                    solver: o,
                    step: None,
                },
            },
            trajectory: Trajectory { time, variables },
            termination: Termination::StopTime,
            stats: SimStats::default(),
            parameters,
        }
    }

    fn parameter(name: &str, value: f64) -> ResultParameter {
        ResultParameter {
            name: name.to_string(),
            value,
            unit: None,
            description: None,
        }
    }

    fn bytes(r: &SimRun) -> Vec<u8> {
        let mut out = Vec::new();
        write_mat4(r, &mut out).unwrap();
        out
    }

    /// 行列を名前 → (MOPT, 行, 列, データ) に分ける（試験用の素朴な読み方）。
    fn matrices(b: &[u8]) -> Vec<(String, i32, usize, usize, Vec<u8>)> {
        let mut out = Vec::new();
        let mut off = 0;
        while off < b.len() {
            let int =
                |k: usize| i32::from_le_bytes(b[off + 4 * k..off + 4 * k + 4].try_into().unwrap());
            let (mopt, rows, cols, imagf, nl) = (
                int(0),
                int(1) as usize,
                int(2) as usize,
                int(3),
                int(4) as usize,
            );
            assert_eq!(imagf, 0);
            let name = String::from_utf8(b[off + 20..off + 20 + nl - 1].to_vec()).unwrap();
            assert_eq!(b[off + 20 + nl - 1], 0, "名前は NUL で終わる");
            let size = match (mopt / 10) % 10 {
                0 => 8,
                2 => 4,
                5 => 1,
                p => panic!("{p}"),
            };
            let start = off + 20 + nl;
            out.push((
                name,
                mopt,
                rows,
                cols,
                b[start..start + rows * cols * size].to_vec(),
            ));
            off = start + rows * cols * size;
        }
        out
    }

    fn doubles(data: &[u8]) -> Vec<f64> {
        data.chunks(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }

    fn ints(data: &[u8]) -> Vec<i32> {
        data.chunks(4)
            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn header_bytes_match_the_omc_layout() {
        let b = bytes(&run(
            vec![0.0, 1.0],
            vec![variable("x", vec![1.0, 2.0])],
            vec![],
        ));
        // Aclass: MOPT 51、4 行 × 11 列、虚部 0、名前の長さ 7（NUL を含む）
        let head: Vec<i32> = ints(&b[..20]);
        assert_eq!(head, [51, 4, 11, 0, 7]);
        assert_eq!(&b[20..27], b"Aclass\0");
        // 列優先: 1 列目は各行の 1 文字目（A、1、NUL、b）。omc の実物と同じバイト
        assert_eq!(&b[27..35], b"A1\0bt.\0i");
        let m = matrices(&b);
        let names: Vec<(&str, i32)> = m.iter().map(|(n, mopt, ..)| (n.as_str(), *mopt)).collect();
        assert_eq!(
            names,
            [
                ("Aclass", 51),
                ("name", 51),
                ("description", 51),
                ("dataInfo", 20),
                ("data_1", 0),
                ("data_2", 0)
            ]
        );
        // name: 行 = 最長（4）、列 = 変数の数。1 列が 1 つの名前（NUL で埋める）
        assert_eq!((m[1].2, m[1].3), (4, 2));
        assert_eq!(m[1].4, b"timex\0\0\0");
        // description: time は omc と同じ説明
        assert_eq!(&m[2].4[..19], TIME_DESCRIPTION.as_bytes());
    }

    #[test]
    fn data_2_is_transposed_one_column_per_time_point() {
        let b = bytes(&run(
            vec![0.0, 0.5, 1.0],
            vec![
                variable("x", vec![1.0, 2.0, 3.0]),
                variable("y", vec![4.0, 5.0, 6.0]),
            ],
            vec![],
        ));
        let m = matrices(&b);
        let (_, _, rows, cols, data) = &m[5];
        assert_eq!((*rows, *cols), (3, 3));
        assert_eq!(doubles(data), [0.0, 1.0, 4.0, 0.5, 2.0, 5.0, 1.0, 3.0, 6.0]);
        // dataInfo: time (0,1,0,-1)、x (2,2,0,0)、y (2,3,0,0)
        assert_eq!(ints(&m[3].4), [0, 1, 0, -1, 2, 2, 0, 0, 2, 3, 0, 0]);
        // data_1: time だけ（[t0, t1]）
        assert_eq!((m[4].2, m[4].3), (1, 2));
        assert_eq!(doubles(&m[4].4), [0.0, 1.0]);
    }

    #[test]
    fn equal_and_negated_columns_become_aliases() {
        let x = vec![1.0, 0.0, -2.0];
        let b = bytes(&run(
            vec![0.0, 0.5, 1.0],
            vec![
                variable("x", x.clone()),
                variable("w", vec![1.0, 4.0, 9.0]),
                variable("y", x.clone()),
                // 0 は符号が付いても同じとみなす
                variable("z", vec![-1.0, -0.0, 2.0]),
            ],
            vec![],
        ));
        let m = matrices(&b);
        assert_eq!(
            ints(&m[3].4),
            [0, 1, 0, -1, 2, 2, 0, 0, 2, 3, 0, 0, 2, 2, 0, 0, 2, -2, 0, 0]
        );
        // data_2 は time、x、w の 3 行だけ
        assert_eq!((m[5].2, m[5].3), (3, 3));
        let back = read_mat4(&mut b.as_slice()).unwrap();
        assert_eq!(back.trajectory.variable("y").unwrap().values, x);
        assert_eq!(
            back.trajectory.variable("z").unwrap().values,
            [-1.0, -0.0, 2.0]
        );
    }

    #[test]
    fn event_times_are_kept_as_two_time_points() {
        let time = vec![0.0, 0.5, 0.5, 1.0];
        let n = vec![0.0, 0.0, 1.0, 1.0];
        let r = run(time.clone(), vec![variable("n", n.clone())], vec![]);
        let b = bytes(&r);
        let m = matrices(&b);
        assert_eq!((m[5].2, m[5].3), (2, 4));
        assert_eq!(doubles(&m[5].4), [0.0, 0.0, 0.5, 0.0, 0.5, 1.0, 1.0, 1.0]);
        let back = read_mat4(&mut b.as_slice()).unwrap();
        assert_eq!(back.trajectory.time, time);
        assert_eq!(back.trajectory.variable("n").unwrap().values, n);
    }

    #[test]
    fn parameters_go_to_data_1() {
        let mut k = parameter("k", 2.0);
        k.unit = Some("1/s".into());
        k.description = Some("rate".into());
        let r = run(
            vec![0.0, 1.0],
            vec![
                variable("x", vec![1.0, 0.5]),
                // data_1 に書くパラメータは、時系列にあっても data_2 に書かない
                variable("k", vec![2.0, 2.0]),
            ],
            vec![k.clone(), parameter("k2", 4.0)],
        );
        let b = bytes(&r);
        let m = matrices(&b);
        // 名前は time、k、k2、x
        assert_eq!(
            ints(&m[3].4),
            [0, 1, 0, -1, 1, 2, 0, 0, 1, 3, 0, 0, 2, 2, 0, 0]
        );
        assert_eq!((m[4].2, m[4].3), (3, 2));
        assert_eq!(doubles(&m[4].4), [0.0, 2.0, 4.0, 1.0, 2.0, 4.0]);
        assert_eq!((m[5].2, m[5].3), (2, 2));
        let back = read_mat4(&mut b.as_slice()).unwrap();
        assert_eq!(back.parameters, [k, parameter("k2", 4.0)]);
        assert_eq!(back.trajectory.variables.len(), 1);
    }

    #[test]
    fn units_are_appended_to_descriptions_like_omc() {
        assert_eq!(described(Some("voltage"), Some("V")), "voltage [V]");
        assert_eq!(described(None, Some("A")), " [A]");
        assert_eq!(described(Some("x"), None), "x");
        assert_eq!(described(None, Some("")), "");
        assert_eq!(
            split_unit("voltage [V]"),
            (Some("voltage".into()), Some("V".into()))
        );
        assert_eq!(split_unit(" [A]"), (None, Some("A".into())));
        assert_eq!(split_unit("a[1] value"), (Some("a[1] value".into()), None));
        assert_eq!(split_unit("x[1]"), (Some("x[1]".into()), None));
        assert_eq!(split_unit(""), (None, None));
    }

    #[test]
    fn round_trip_keeps_names_values_units_and_descriptions() {
        let mut v = variable("C1.v", vec![0.0, 0.25, f64::NAN]);
        v.unit = Some("V".into());
        v.description = Some("電圧".into());
        let mut a = variable("a[1,2]", vec![3.0, 2.0, 1.0]);
        a.description = Some("matrix entry".into());
        let r = run(
            vec![0.0, 0.5, 1.0],
            vec![v.clone(), a.clone()],
            vec![parameter("R", 10.0)],
        );
        let back = read_mat4(&mut bytes(&r).as_slice()).unwrap();
        assert_eq!(back.trajectory.time, r.trajectory.time);
        let got = back.trajectory.variable("C1.v").unwrap();
        assert_eq!(got.unit.as_deref(), Some("V"));
        assert_eq!(got.description.as_deref(), Some("電圧"));
        assert!(got.values[2].is_nan());
        let got = back.trajectory.variable("a[1,2]").unwrap();
        assert_eq!(got.values, a.values);
        assert_eq!(got.description, a.description);
        assert_eq!(back.parameters, [parameter("R", 10.0)]);
    }

    #[test]
    fn empty_trajectory_uses_the_settings_interval() {
        let b = bytes(&run(vec![], vec![], vec![parameter("k", 1.0)]));
        let m = matrices(&b);
        assert_eq!(doubles(&m[4].4), [0.0, 1.0, 1.0, 1.0]);
        assert_eq!((m[5].2, m[5].3), (1, 0));
        let back = read_mat4(&mut b.as_slice()).unwrap();
        assert!(back.trajectory.time.is_empty());
        assert_eq!(back.parameters.len(), 1);
    }

    #[test]
    fn mismatched_values_are_rejected_without_writing() {
        let r = run(vec![0.0, 1.0], vec![variable("x", vec![1.0])], vec![]);
        let mut out = Vec::new();
        let e = write_mat4(&r, &mut out).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
        assert!(out.is_empty());
    }

    #[test]
    fn truncated_or_foreign_files_are_invalid_data() {
        let b = bytes(&run(
            vec![0.0, 1.0],
            vec![variable("x", vec![1.0, 2.0])],
            vec![],
        ));
        let e = read_mat4(&mut &b[..b.len() - 3]).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        // ビッグエンディアンの MOPT（1000 の位が 1）
        let mut big = b.clone();
        big[..4].copy_from_slice(&1051i32.to_le_bytes());
        assert_eq!(
            read_mat4(&mut big.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(read_mat4(&mut &[][..]).is_err());
    }
}

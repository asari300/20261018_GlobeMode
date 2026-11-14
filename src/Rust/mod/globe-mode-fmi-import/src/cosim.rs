//! Co-Simulation のマスター（最小版。S249、ADR-0026 の (q)）: 複数の CS の FMU を固定の通信刻みでつなぐ。
//!
//! 既定は Jacobi 型: 全 FMU が同じ刻み `[t, t + h]` を進め、**刻みの頭で**前の刻みの終わりの出力を入力へコピーする
//! （入力は 1 刻み遅れる。代数ループは扱わないので要らない）。`"algorithm": "gauss-seidel"` なら FMU を結線から決めた順に 1 つずつ進め、
//! 同じ刻みで更新された出力をすぐ次の FMU の入力に渡す（S259）。つなげるのは同じ型（Float64・Int32・Boolean）の出力→入力だけ。
//! 結果は全 FMU の Float64 の出力（と、Int32・Boolean の接続の始点。値は f64 に直す）を `<FMU の名前>.<変数名>` の列にしたもの。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode_fmi::{Causality, ModelDescription, VarType};
use serde::{Deserialize, Serialize};

use crate::{CsRun, Error, Fmu, FmuCs, Result, find_float, grid_points, output_names};

/// 結線の解き方。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    /// 刻みの頭の出力を全員に渡し、全 FMU を進める（入力は 1 刻み遅れる）
    #[default]
    Jacobi,
    /// 結線の元から先の順（ループは FMU の名前の順）に 1 つずつ進め、更新された出力をすぐ渡す
    GaussSeidel,
}

/// 結線の記述。JSON で渡せる（未知の鍵は誤り）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CosimSpec {
    /// FMU の名前 → `.fmu` のパス。名前は英数字・`_`・`-` だけ（列名の前置きになる。`.` を含められない）
    pub fmus: BTreeMap<String, PathBuf>,
    /// `["a.y", "b.u"]`（FMU の名前、`.`、変数名。出力 → 入力）。変数名に `.` を含んでよい
    pub connections: Vec<(String, String)>,
    /// 省略時は 0
    pub start_time: Option<f64>,
    pub stop_time: f64,
    /// 通信刻み
    pub step: f64,
    /// 省略時は Jacobi（S259）
    pub algorithm: Algorithm,
    /// 初期化の前に設定する値。`{"a.k": 2.0}`（FMU の名前、`.`、変数名）。Float64・Int32（整数値）・Boolean（0 か 1）（S259）
    pub start: BTreeMap<String, f64>,
    /// ワーカーの時間の上限（秒）。親が使う
    pub timeout: Option<f64>,
    /// ワーカーの仮想メモリの上限（MiB。Linux だけ）。親が使う
    pub memory_limit_mib: Option<u64>,
}

/// `"a.y"` を（FMU の添字, 変数名）に割る。
fn endpoint<'a>(names: &[&String], s: &'a str) -> Result<(usize, &'a str)> {
    let bad = || {
        Error::Fmu(format!(
            "接続の端 {s:?} が `<FMU の名前>.<変数名>` の形でない"
        ))
    };
    let (fmu, var) = s.split_once('.').ok_or_else(bad)?;
    let i = names
        .iter()
        .position(|n| n.as_str() == fmu)
        .ok_or_else(|| Error::Fmu(format!("接続の端 {s:?} の FMU {fmu:?} が `fmus` に無い")))?;
    Ok((i, var))
}

/// 結線の記述を回す。`work_dir` があれば FMU を `<work_dir>/<名前>/` に展開する（ワーカーが親の作った展開先を使う）。
pub fn run_cosim_with(
    spec: &CosimSpec,
    work_dir: Option<&Path>,
    progress: &mut dyn FnMut(f64),
) -> Result<CsRun> {
    let start = spec.start_time.unwrap_or(0.0);
    let (stop, step) = (spec.stop_time, spec.step);
    if stop.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("終了時刻が開始時刻以下".into()));
    }
    if step.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("刻みが正でない".into()));
    }
    let n = grid_points(start, stop, step)?;
    let names: Vec<&String> = spec.fmus.keys().collect();
    if names.is_empty() {
        return Err(Error::Fmu("`fmus` が空".into()));
    }
    let mut fmus = Vec::new();
    for (name, path) in &spec.fmus {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
        {
            return Err(Error::Fmu(format!(
                "FMU の名前 {name:?} が不正（英数字・`_`・`-` だけ）"
            )));
        }
        fmus.push(match work_dir {
            Some(dir) => {
                let d = dir.join(name);
                std::fs::create_dir_all(&d).map_err(crate::io)?;
                Fmu::open_in(path, &d)?
            }
            None => Fmu::open(path)?,
        });
    }
    // 集める列（各 FMU の Float64 の出力）と結線の解決。実行の前に型・向きを断る
    let outs: Vec<Vec<String>> = fmus
        .iter()
        .map(|f| output_names(&f.description, &[]))
        .collect();
    let causality = |i: usize, v: &str| {
        fmus[i]
            .description
            .variables
            .iter()
            .find(|x| x.name == v)
            .map(|x| x.causality)
    };
    let mut wires = Vec::new(); // (出力の FMU, その value reference, 入力の FMU, その value reference, 型)
    let mut extra: Vec<Vec<(String, u32, Kind)>> = vec![Vec::new(); fmus.len()]; // Int32・Boolean の始点の列
    for (a, b) in &spec.connections {
        let ctx = |e: Error| Error::Fmu(format!("接続 {a} -> {b}: {e}"));
        let (si, sv) = endpoint(&names, a)?;
        let (di, dv) = endpoint(&names, b)?;
        let (svr, sk) = find_scalar(&fmus[si].description, sv).map_err(ctx)?;
        let (dvr, dk) = find_scalar(&fmus[di].description, dv).map_err(ctx)?;
        if sk != dk {
            return Err(ctx(Error::Fmu(format!(
                "型が違う（{sv:?} は {sk:?}、{dv:?} は {dk:?}。同じ型どうしだけ）"
            ))));
        }
        if causality(si, sv) != Some(Causality::Output) {
            return Err(ctx(Error::Fmu(format!("{sv:?} は出力でない"))));
        }
        if causality(di, dv) != Some(Causality::Input) {
            return Err(ctx(Error::Fmu(format!("{dv:?} は入力でない"))));
        }
        if sk != Kind::Real && !extra[si].iter().any(|e| e.0 == sv) {
            extra[si].push((sv.to_string(), svr, sk));
        }
        wires.push(Wire {
            si,
            svr,
            di,
            dvr,
            kind: sk,
        });
    }
    // 初期値（初期化の前に設定する。実行の前に型と値を断る）
    let mut starts = Vec::new(); // (FMU, value reference, 型, 値)
    for (key, &value) in &spec.start {
        let ctx = |e: Error| Error::Fmu(format!("start {key}: {e}"));
        let (i, v) = endpoint(&names, key)?;
        let (vr, kind) = find_scalar(&fmus[i].description, v).map_err(ctx)?;
        let ok = match kind {
            Kind::Real => value.is_finite(),
            Kind::Int => value.fract() == 0.0 && value.abs() <= f64::from(i32::MAX),
            Kind::Bool => value == 0.0 || value == 1.0,
        };
        if !ok {
            return Err(ctx(Error::Fmu(format!(
                "値 {value} は {kind:?} として不正（Int は整数値、Bool は 0 か 1）"
            ))));
        }
        starts.push((i, vr, kind, value));
    }
    let mut insts: Vec<Box<dyn FmuCs + '_>> = Vec::new();
    for (f, name) in fmus.iter().zip(&names) {
        insts.push(f.instantiate_cs(name)?);
    }
    for &(i, vr, kind, value) in &starts {
        put(insts[i].as_mut(), kind, vr, value)?;
    }
    let vrs: Vec<Vec<u32>> = fmus
        .iter()
        .zip(&outs)
        .map(|(f, o)| {
            o.iter()
                .map(|n| find_float(&f.description, n))
                .collect::<Result<_>>()
        })
        .collect::<Result<_>>()?;
    for inst in &mut insts {
        inst.initialize(start, Some(stop))?;
    }
    let columns: Vec<String> = names
        .iter()
        .zip(outs.iter().zip(&extra))
        .flat_map(|(f, (o, e))| {
            o.iter()
                .map(move |v| format!("{f}.{v}"))
                .chain(e.iter().map(move |x| format!("{f}.{}", x.0)))
        })
        .collect();
    let read = |insts: &mut Vec<Box<dyn FmuCs + '_>>| -> Result<Vec<f64>> {
        let mut row = Vec::new();
        for (i, inst) in insts.iter_mut().enumerate() {
            row.extend(inst.get_real(&vrs[i])?);
            for &(_, vr, kind) in &extra[i] {
                row.push(get(inst.as_mut(), kind, vr)?);
            }
        }
        Ok(row)
    };
    // 結線の元から先の順。ループは名前の順で破る（Gauss-Seidel だけが使う）
    let mut order: Vec<usize> = Vec::new();
    while order.len() < insts.len() {
        let ready = |i: usize| {
            wires
                .iter()
                .all(|w| w.di != i || w.si == i || order.contains(&w.si))
        };
        let pick = (0..insts.len())
            .find(|&i| !order.contains(&i) && ready(i))
            .or_else(|| (0..insts.len()).find(|i| !order.contains(i)))
            .expect("未処理の FMU がある");
        order.push(pick);
    }
    let mut rows = vec![(start, read(&mut insts)?)];
    for i in 0..n {
        let t = start + i as f64 * step;
        progress(t);
        let h = step.min(stop - t);
        let mut terminated = false;
        match spec.algorithm {
            Algorithm::Jacobi => {
                // 全部読んでから全部渡す（渡す途中で出力が動いても、前の刻みの終わりの値を使う）
                let vals = wires
                    .iter()
                    .map(|w| get(insts[w.si].as_mut(), w.kind, w.svr))
                    .collect::<Result<Vec<_>>>()?;
                for (w, v) in wires.iter().zip(vals) {
                    put(insts[w.di].as_mut(), w.kind, w.dvr, v)?;
                }
                for inst in &mut insts {
                    terminated |= inst.do_step(t, h)?;
                }
            }
            Algorithm::GaussSeidel => {
                for &d in &order {
                    for w in wires.iter().filter(|w| w.di == d) {
                        let v = get(insts[w.si].as_mut(), w.kind, w.svr)?;
                        put(insts[d].as_mut(), w.kind, w.dvr, v)?;
                    }
                    terminated |= insts[d].do_step(t, h)?;
                }
            }
        }
        rows.push((t + h, read(&mut insts)?));
        if terminated {
            break;
        }
    }
    Ok(CsRun {
        names: columns,
        rows,
    })
}

/// 接続・`start` で扱える変数の型。値は f64 で受け渡す（Int32 は厳密、Boolean は 0 / 1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Real,
    Int,
    Bool,
}

struct Wire {
    si: usize,
    svr: u32,
    di: usize,
    dvr: u32,
    kind: Kind,
}

/// スカラの Float64・Int32・Boolean の変数の（value reference, 型）。
fn find_scalar(md: &ModelDescription, name: &str) -> Result<(u32, Kind)> {
    let v = md
        .variables
        .iter()
        .find(|v| v.name == name)
        .ok_or_else(|| Error::Fmu(format!("変数 {name:?} が無い")))?;
    let kind = match v.ty {
        VarType::Float64 => Kind::Real,
        VarType::Int32 => Kind::Int,
        VarType::Boolean => Kind::Bool,
        _ => {
            return Err(Error::Fmu(format!(
                "変数 {name:?} は Float64・Int32・Boolean のどれでもない（この版はこの 3 つだけ）"
            )));
        }
    };
    if v.dimensions {
        return Err(Error::Fmu(format!(
            "変数 {name:?} は配列（<Dimension>）で、この版は未対応（スカラーだけ）"
        )));
    }
    Ok((v.value_reference, kind))
}

fn get(inst: &mut dyn FmuCs, kind: Kind, vr: u32) -> Result<f64> {
    Ok(match kind {
        Kind::Real => inst.get_real(&[vr])?[0],
        Kind::Int => f64::from(inst.get_int(&[vr])?[0]),
        Kind::Bool => f64::from(u8::from(inst.get_bool(&[vr])?[0])),
    })
}

/// Int の値は `start` の検査済みの整数値か Int32 の出力なので、`as i32` で切れない。
fn put(inst: &mut dyn FmuCs, kind: Kind, vr: u32, v: f64) -> Result<()> {
    match kind {
        Kind::Real => inst.set_real(&[vr], &[v]),
        Kind::Int => inst.set_int(&[vr], &[v as i32]),
        Kind::Bool => inst.set_bool(&[vr], &[v != 0.0]),
    }
}

//! Co-Simulation のマスター（最小版。S249、ADR-0026 の (q)）: 複数の CS の FMU を固定の通信刻みでつなぐ。
//!
//! Jacobi 型: 全 FMU が同じ刻み `[t, t + h]` を進め、**刻みの頭で**前の刻みの終わりの出力を入力へコピーする
//! （入力は 1 刻み遅れる。代数ループは扱わないので要らない）。つなげるのは Float64（Real）の出力→入力だけ。
//! 結果は全 FMU の Float64 の出力を `<FMU の名前>.<変数名>` の列にしたもの。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode_fmi::Causality;
use serde::{Deserialize, Serialize};

use crate::{CsRun, Error, Fmu, FmuCs, Result, find_float, grid_points, output_names};

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
    let mut wires = Vec::new(); // (出力の FMU, その列の位置, 入力の FMU, 入力の value reference)
    for (a, b) in &spec.connections {
        let ctx = |e: Error| Error::Fmu(format!("接続 {a} -> {b}: {e}"));
        let (si, sv) = endpoint(&names, a)?;
        let (di, dv) = endpoint(&names, b)?;
        find_float(&fmus[si].description, sv).map_err(ctx)?;
        let vr = find_float(&fmus[di].description, dv).map_err(ctx)?;
        let causality = |i: usize, v: &str| {
            fmus[i]
                .description
                .variables
                .iter()
                .find(|x| x.name == v)
                .map(|x| x.causality)
        };
        if causality(si, sv) != Some(Causality::Output) {
            return Err(ctx(Error::Fmu(format!("{sv:?} は出力でない"))));
        }
        if causality(di, dv) != Some(Causality::Input) {
            return Err(ctx(Error::Fmu(format!("{dv:?} は入力でない"))));
        }
        let pos = outs[si]
            .iter()
            .position(|o| o == sv)
            .ok_or_else(|| ctx(Error::Fmu(format!("{sv:?} は集める出力の列に無い"))))?;
        wires.push((si, pos, di, vr));
    }
    let mut insts: Vec<Box<dyn FmuCs + '_>> = Vec::new();
    for (f, name) in fmus.iter().zip(&names) {
        insts.push(f.instantiate_cs(name)?);
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
        .zip(&outs)
        .flat_map(|(f, o)| o.iter().map(move |v| format!("{f}.{v}")))
        .collect();
    let read = |insts: &mut Vec<Box<dyn FmuCs + '_>>| -> Result<Vec<Vec<f64>>> {
        insts
            .iter_mut()
            .zip(&vrs)
            .map(|(i, v)| i.get_real(v))
            .collect()
    };
    let flat = |v: &[Vec<f64>]| v.concat();
    let mut last = read(&mut insts)?;
    let mut rows = vec![(start, flat(&last))];
    for i in 0..n {
        let t = start + i as f64 * step;
        progress(t);
        let h = step.min(stop - t);
        for &(si, pos, di, vr) in &wires {
            insts[di].set_real(&[vr], &[last[si][pos]])?;
        }
        let mut terminated = false;
        for inst in &mut insts {
            terminated |= inst.do_step(t, h)?;
        }
        last = read(&mut insts)?;
        rows.push((t + h, flat(&last)));
        if terminated {
            break;
        }
    }
    Ok(CsRun {
        names: columns,
        rows,
    })
}

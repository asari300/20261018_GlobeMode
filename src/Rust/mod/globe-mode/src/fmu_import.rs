//! FMI Import の facade（S229、ADR-0026 の (p)）。`globe fmu info` / `globe fmu run`・LSP の `globe/fmuInfo`・`globe/fmuRun`・
//! Python の `fmu_info` / `run_fmu` が同じ道を通る。feature `fmu-import`（共有ライブラリを読むので wasm32・Android では付けない）。
//!
//! **FMU のネイティブコードは別プロセス（ワーカー）で動く**（[`run_fmu`]。S210）。ワーカーの実行ファイルは呼び手が渡す。
//! `globe` 自身（CLI・LSP の sidecar）は `std::env::current_exe()`、Python は `GLOBE_BIN` か PATH の `globe`。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use globe_mode_fmi_import as imp;
use globe_mode_ir::{Trajectory, TrajectoryVariable, VariableRole};

use crate::{CancelToken, OpenedResults};

pub use imp::{Kind as FmuKind, MeSummary, TRUST_WARNING as FMU_TRUST_WARNING};

/// FMU の要約（`globe fmu info` と同じ情報。ネイティブコードは実行しない）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct FmuInfo {
    pub model_name: String,
    pub fmi_version: String,
    pub interfaces: Vec<FmuInterface>,
    /// `DefaultExperiment` の開始・終了（属性の文字列そのまま）
    pub default_experiment: Option<FmuExperiment>,
    pub variables: Vec<FmuVariable>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FmuExperiment {
    pub start_time: Option<String>,
    pub stop_time: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FmuInterface {
    /// `ModelExchange`・`CoSimulation`・`ScheduledExecution`
    pub kind: &'static str,
    pub model_identifier: String,
    /// ME だけ: 連続状態の数
    pub continuous_states: Option<usize>,
    /// ME だけ: event indicator の数
    pub event_indicators: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FmuVariable {
    pub name: String,
    pub causality: String,
    pub ty: String,
    pub start: Option<String>,
}

/// `.fmu` の `modelDescription.xml` だけを読む。
pub fn import_fmu_info(path: &Path) -> Result<FmuInfo, String> {
    let md = imp::read_model_description(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let interfaces = [
        ("ModelExchange", &md.model_exchange),
        ("CoSimulation", &md.co_simulation),
        ("ScheduledExecution", &md.scheduled_execution),
    ]
    .into_iter()
    .filter_map(|(kind, i)| {
        let me = kind == "ModelExchange";
        i.as_ref().map(|i| FmuInterface {
            kind,
            model_identifier: i.model_identifier.clone(),
            continuous_states: me.then_some(md.model_structure.derivatives.len()),
            event_indicators: me.then_some(md.number_of_event_indicators),
        })
    })
    .collect();
    Ok(FmuInfo {
        model_name: md.model_name,
        fmi_version: md.fmi_version,
        interfaces,
        default_experiment: md.default_experiment.map(|e| FmuExperiment {
            start_time: e.start_time,
            stop_time: e.stop_time,
        }),
        variables: md
            .variables
            .into_iter()
            .map(|v| FmuVariable {
                name: v.name,
                causality: format!("{:?}", v.causality),
                ty: format!("{:?}", v.ty),
                start: v.start,
            })
            .collect(),
    })
}

/// FMU の実行の依頼。JSON（省略した鍵は既定。未知の鍵は誤り）でも渡せる（LSP・Python）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FmuRunRequest {
    /// 省略時は Co-Simulation を持てばそれ、無ければ Model Exchange
    pub kind: Option<FmuKind>,
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    /// 通信刻み・出力の刻み
    pub step: Option<f64>,
    /// 出力する変数の名前。空なら causality が output の Real すべて
    pub outputs: Vec<String>,
    /// 初期化の前に与える Real のパラメータ
    pub set: BTreeMap<String, f64>,
    /// 実行の時間の上限（秒）。超えたらワーカーを kill する
    pub timeout: Option<f64>,
    /// ワーカーの仮想メモリの上限（MiB。Linux だけ）
    pub memory_limit_mib: Option<u64>,
}

/// [`run_fmu`] の結果。
#[derive(Debug)]
pub struct FmuRun {
    /// 時刻と出力の列（[`OpenedResults::columns`] が実行の結果と同じ形で返す。既定で描くのは先頭の 3 本）
    pub results: OpenedResults,
    /// Model Exchange を回したときだけ
    pub me: Option<MeSummary>,
}

/// FMU を別プロセスのワーカーで回す。`worker` は `fmu __worker` の副命令を持つ実行ファイル（`globe`）。
/// `on_progress` は子が流す時刻（0.25 秒以上おき）を受ける。`cancel` が立つと 50 ms 以内にワーカーを kill して
/// 「中断した」の誤りで終わる。ワーカーが落ちた・時間切れ・FMU の失敗も誤り（文面に理由）。
pub fn run_fmu(
    worker: &Path,
    path: &Path,
    request: &FmuRunRequest,
    on_progress: &mut dyn FnMut(f64),
    cancel: Option<&CancelToken>,
) -> Result<FmuRun, String> {
    let mut spec = imp::WorkerSpec::new(worker, &["fmu", "__worker"]);
    spec.timeout = request
        .timeout
        .map(|s| {
            Duration::try_from_secs_f64(s)
                .map_err(|_| format!("timeout {s} が不正（0 以上の秒数）"))
        })
        .transpose()?;
    spec.memory_limit = request.memory_limit_mib.map(|m| m.saturating_mul(1 << 20));
    spec.cancel = cancel.map(CancelToken::flag);
    let run = imp::run_in_worker(
        &spec,
        &imp::WorkerRequest {
            file: path.to_owned(),
            kind: request.kind,
            start_time: request.start_time,
            stop_time: request.stop_time,
            step: request.step,
            outputs: request.outputs.clone(),
            set: request.set.clone().into_iter().collect(),
            work_dir: None, // run_in_worker が親の側で作る
        },
        on_progress,
    )
    .map_err(|e| e.to_string())?;
    // 列を転置する（ワーカーは行で返す）
    let mut variables: Vec<TrajectoryVariable> = run
        .names
        .into_iter()
        .map(|name| TrajectoryVariable {
            name,
            role: VariableRole::Output,
            unit: None,
            description: None,
            values: Vec::with_capacity(run.rows.len()),
        })
        .collect();
    let mut time = Vec::with_capacity(run.rows.len());
    for (t, values) in run.rows {
        time.push(t);
        for (v, x) in variables.iter_mut().zip(values) {
            v.values.push(x);
        }
    }
    Ok(FmuRun {
        results: OpenedResults(Trajectory { time, variables }),
        me: run.me,
    })
}

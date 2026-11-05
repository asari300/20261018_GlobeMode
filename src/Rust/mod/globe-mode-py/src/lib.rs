//! GlobeMode の Python の束ね（S039）。facade `globe-mode` の `Workspace` を PyO3 で包む。
//!
//! 入出力は globe-mode-wasm と同じく JSON の文字列にして、`dict` への変換と結果の表（polars）は
//! Python 側の `globe_mode/__init__.py` が受け持つ。Rust の型を 1 つずつ Python の型に写さない。

use std::path::{Path, PathBuf};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde::Serialize;

fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON: {e}"))
}

/// `settings_json`（`{"stop_time": 2, "solver": "bdf"}` など。省略可）から実行の依頼を作る。
fn request(
    model: &str,
    settings_json: Option<&str>,
) -> Result<globe_mode::SimulateRequest, String> {
    let settings: serde_json::Value = match settings_json {
        Some(s) => {
            serde_json::from_str(s).map_err(|e| format!("settings の JSON が読めない: {e}"))?
        }
        None => serde_json::json!({}),
    };
    serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
        .map_err(|e| format!("settings が読めない: {e}"))
}

fn simulate_json(
    ws: &mut globe_mode::Workspace,
    model: &str,
    settings_json: Option<&str>,
) -> Result<String, String> {
    json(&ws.simulate(&request(model, settings_json)?))
}

/// 結果の JSON（`csv` は無い）と、列の名前（`time` が先頭）と値（f64 の little-endian を並べたもの）。
type Columns = (String, Vec<String>, Vec<Vec<u8>>);

/// [`simulate_json`] と同じだが、時系列を CSV にせず列ごとに返す（S085。並びは facade の `Simulated::columns`）。
fn simulate_columns(
    ws: &mut globe_mode::Workspace,
    model: &str,
    settings_json: Option<&str>,
) -> Result<Columns, String> {
    let request = columns_request(model, settings_json)?;
    columns_of(ws.simulate_with(&request, &globe_mode::now_ms))
}

fn columns_request(
    model: &str,
    settings_json: Option<&str>,
) -> Result<globe_mode::SimulateRequest, String> {
    let mut request = request(model, settings_json)?;
    request.no_csv = true;
    Ok(request)
}

fn columns_of(simulated: globe_mode::Simulated) -> Result<Columns, String> {
    let (layout, columns) = simulated.columns();
    let columns = columns
        .iter()
        .map(|v| v.iter().flat_map(|x| x.to_le_bytes()).collect())
        .collect();
    // 既定で描く信号（S190）は結果の dict の `defaults` に添える
    let mut response =
        serde_json::to_value(&simulated.response).map_err(|e| format!("JSON: {e}"))?;
    response["defaults"] = serde_json::json!(layout.defaults);
    Ok((json(&response)?, layout.names, columns))
}

/// `Progress` を Python の dict に（`stage` は `"integration"` などの小文字、`time` は積分の段だけ）。
fn progress_dict<'py>(py: Python<'py>, p: &globe_mode::Progress) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("stage", format!("{:?}", p.stage).to_lowercase())?;
    d.set_item("time", p.time)?;
    d.set_item("start_time", p.start_time)?;
    d.set_item("stop_time", p.stop_time)?;
    Ok(d)
}

/// ADR-0033 の Python の段（S129）。GIL を離して `simulate_observed` を回し、進捗のたびに GIL を取り直して
/// `check_signals`（Ctrl-C）と `on_progress(dict)` を呼ぶ。`True`（真の値）が返るか例外が出たらトークンを立てて
/// 次の区切りで止め、例外（`KeyboardInterrupt` など）は実行が戻ってから投げ直す。`finish` も GIL の外で回す。
fn simulate_observed<T: Send>(
    py: Python<'_>,
    ws: &mut globe_mode::Workspace,
    request: &globe_mode::SimulateRequest,
    on_progress: Py<PyAny>,
    finish: impl FnOnce(globe_mode::Simulated) -> Result<T, String> + Send,
) -> PyResult<T> {
    let token = globe_mode::CancelToken::new();
    let mut raised: Option<PyErr> = None;
    let mut notify = |p: &globe_mode::Progress| {
        if raised.is_some() {
            return;
        }
        let stop = Python::attach(|py| {
            py.check_signals()?;
            on_progress
                .call1(py, (progress_dict(py, p)?,))?
                .is_truthy(py)
        });
        match stop {
            Ok(false) => {}
            Ok(true) => token.cancel(),
            Err(e) => {
                raised = Some(e);
                token.cancel();
            }
        }
    };
    let cancel = token.clone();
    let out = py.detach(move || {
        finish(ws.simulate_observed(
            request,
            &globe_mode::now_ms,
            globe_mode::SimulateHandle {
                on_progress: Some(&mut notify),
                cancel: Some(cancel),
            },
        ))
    });
    if let Some(e) = raised {
        return Err(e);
    }
    out.map_err(PyValueError::new_err)
}

fn graph_json(ws: &mut globe_mode::Workspace, model: &str, depth: u32) -> Result<String, String> {
    json(&ws.graph(model, depth).map_err(|e| e.to_string())?)
}

/// ライブラリを 1 回だけ読み、ファイルを足しながら、検査・グラフ・実行を繰り返す。
// unsendable: salsa のデータベースは Sync でないので、作ったスレッドからだけ使える
#[pyclass(unsendable, module = "globe_mode._native")]
struct Workspace(globe_mode::Workspace);

#[pymethods]
impl Workspace {
    /// `libraries` はライブラリのディレクトリ（MSL など）。読めなければ診断の JSON を添えた `RuntimeError`。
    #[new]
    #[pyo3(signature = (libraries = Vec::new()))]
    fn new(libraries: Vec<PathBuf>) -> PyResult<Self> {
        let inner = if libraries.is_empty() {
            globe_mode::Workspace::new()
        } else {
            globe_mode::Workspace::with_libraries(&libraries)
                .map_err(|ds| PyRuntimeError::new_err(json(&ds).unwrap_or_default()))?
        };
        Ok(Workspace(inner))
    }

    fn set_file(&mut self, path: &str, text: String) {
        self.0.set_file(path, text);
    }

    /// ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。
    fn load_dir(&mut self, dir: PathBuf) -> PyResult<usize> {
        let dir: &Path = &dir;
        self.0
            .load_dir(dir)
            .map_err(|e| PyRuntimeError::new_err(format!("{}: {e}", dir.display())))
    }

    /// 検査（S0〜S10）の結果（`CheckResponse` の JSON）。
    fn check(&mut self, model: &str) -> PyResult<String> {
        json(&self.0.check(model)).map_err(PyRuntimeError::new_err)
    }

    /// connect グラフ（`ConnectGraph` の JSON）。モデルが索引に無ければ `ValueError`。
    #[pyo3(signature = (model, depth = 1))]
    fn graph(&mut self, model: &str, depth: u32) -> PyResult<String> {
        graph_json(&mut self.0, model, depth).map_err(PyValueError::new_err)
    }

    /// ファイル（`set_file` に渡したパス）のアウトライン（`globe outline --json` と同じ `Outline` の JSON）。
    /// ファイルが無いか、構文の誤りがあれば `ValueError`（誤りは最初の診断の `コード: 文`）。
    fn outline(&self, path: &str) -> PyResult<String> {
        match self.0.outline(path) {
            None => Err(PyValueError::new_err(format!(
                "ファイル {path} がワークスペースに無い"
            ))),
            Some(Ok(o)) => json(o).map_err(PyRuntimeError::new_err),
            Some(Err(ds)) => Err(PyValueError::new_err(
                ds.first()
                    .map_or_else(String::new, |d| format!("{}: {}", d.code, d.message)),
            )),
        }
    }

    /// `check` に渡すファイル（モデルが使うものの閉包。パスの昇順）の JSON の配列。モデルが無ければ `[]`。
    fn deps(&mut self, model: &str) -> PyResult<String> {
        json(&self.0.deps(model)).map_err(PyRuntimeError::new_err)
    }

    /// flat モデルの要約（`FlatSummary` の JSON）。flatten まで通らなければ `null`。
    fn flat(&mut self, model: &str) -> PyResult<String> {
        json(&self.0.flat(model)).map_err(PyRuntimeError::new_err)
    }

    /// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。モデルの誤りは例外にせず `ok` と `diagnostics` で返す。
    /// `on_progress` を渡すと GIL を離して回し、進捗を dict で知らせる。`True` を返すか Ctrl-C で止まる（S129）。
    #[pyo3(signature = (model, settings = None, on_progress = None))]
    fn simulate(
        &mut self,
        py: Python<'_>,
        model: &str,
        settings: Option<&str>,
        on_progress: Option<Py<PyAny>>,
    ) -> PyResult<String> {
        let Some(on_progress) = on_progress else {
            return simulate_json(&mut self.0, model, settings).map_err(PyValueError::new_err);
        };
        let request = request(model, settings).map_err(PyValueError::new_err)?;
        simulate_observed(py, &mut self.0, &request, on_progress, |s| {
            json(&s.response)
        })
    }

    /// [`Workspace::simulate`] と同じだが、時系列を `(結果の JSON, 列の名前, 列の bytes)` で返す（S085）。
    /// `on_progress` は [`Workspace::simulate`] と同じ（S129）。
    #[pyo3(signature = (model, settings = None, on_progress = None))]
    fn simulate_columns<'py>(
        &mut self,
        py: Python<'py>,
        model: &str,
        settings: Option<&str>,
        on_progress: Option<Py<PyAny>>,
    ) -> PyResult<(String, Vec<String>, Vec<Bound<'py, PyBytes>>)> {
        let (response, names, columns) = match on_progress {
            None => {
                simulate_columns(&mut self.0, model, settings).map_err(PyValueError::new_err)?
            }
            Some(f) => {
                let request = columns_request(model, settings).map_err(PyValueError::new_err)?;
                simulate_observed(py, &mut self.0, &request, f, columns_of)?
            }
        };
        let columns = columns.iter().map(|c| PyBytes::new(py, c)).collect();
        Ok((response, names, columns))
    }

    /// FMU を書き出し（S118。`globe fmu export` と同じ）、`.fmu` のパスを返す。`options` は
    /// `{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}` の JSON（省略可）。失敗は `ValueError`。
    #[pyo3(signature = (model, out_dir, options = None))]
    fn export_fmu(
        &mut self,
        model: &str,
        out_dir: PathBuf,
        options: Option<&str>,
    ) -> PyResult<PathBuf> {
        let options: globe_mode::FmuOptions = match options {
            Some(s) => serde_json::from_str(s)
                .map_err(|e| PyValueError::new_err(format!("options が読めない: {e}")))?,
            None => Default::default(),
        };
        self.0
            .export_fmu(model, &out_dir, &options)
            .map_err(PyValueError::new_err)
    }
}

/// 設定のキーと値の型の見本（`SimSettings` を全部埋めた JSON）。Python が未知のキーと型の誤りを弾く元で、
/// `SimSettings` に項目を足すとここがコンパイルできなくなる（Python 側の一覧と食い違わない）。
#[pyfunction]
fn settings_sample() -> String {
    json(&globe_mode::SimSettings {
        start_time: Some(0.0),
        stop_time: Some(0.0),
        tolerance: Some(0.0),
        interval: Some(0.0),
        solver: Some(String::new()),
        max_wall_seconds: Some(0.0),
        step: Some(0.0),
        parameters: [(String::new(), 0.0)].into(),
    })
    .unwrap_or_default()
}

/// ソルバの登録表（JSON の配列）。
#[pyfunction]
fn solvers() -> PyResult<String> {
    json(&globe_mode::solvers()).map_err(PyRuntimeError::new_err)
}

/// 本文を整形する（S111）。文法が壊れていれば `ValueError`。
#[pyfunction]
#[pyo3(signature = (text, indent = 2, insert_origin = true))]
fn format(text: &str, indent: usize, insert_origin: bool) -> PyResult<String> {
    globe_mode::format(text, indent, insert_origin).map_err(PyValueError::new_err)
}

/// lint の結果（`[{rule, line, column, message}]` の JSON。S111）。
#[pyfunction]
fn lint(text: &str) -> String {
    globe_mode::lint(text)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Workspace>()?;
    m.add_function(wrap_pyfunction!(solvers, m)?)?;
    m.add_function(wrap_pyfunction!(settings_sample, m)?)?;
    m.add_function(wrap_pyfunction!(format, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECAY: &str =
        "model Decay Real x(start = 1, fixed = true); equation der(x) = -x; end Decay;";

    #[test]
    fn simulate_and_graph_round_trip_as_json() {
        let mut ws = globe_mode::Workspace::new();
        ws.set_file("Decay.mo", DECAY);
        let res: serde_json::Value = serde_json::from_str(
            &simulate_json(&mut ws, "Decay", Some(r#"{"stop_time": 1}"#)).unwrap(),
        )
        .unwrap();
        assert_eq!(res["ok"], true);
        assert!(
            res["csv"]
                .as_str()
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .contains("time"),
            "{res}"
        );
        assert!(simulate_json(&mut ws, "Decay", Some("{")).is_err());
        let (res, names, columns) =
            simulate_columns(&mut ws, "Decay", Some(r#"{"stop_time": 1}"#)).unwrap();
        assert!(!res.contains("\"csv\":\""), "{res}");
        assert_eq!(names, ["time", "x"]);
        let last = f64::from_le_bytes(columns[1][columns[1].len() - 8..].try_into().unwrap());
        assert!((last - (-1f64).exp()).abs() < 1e-4, "{last}");
        assert!(graph_json(&mut ws, "NoSuchModel", 1).is_err());
    }
}

//! GlobeMode の Python の束ね（S039）。facade `globe-mode` の `Workspace` を PyO3 で包む。
//!
//! 入出力は globe-mode-wasm と同じく JSON の文字列にして、`dict` への変換と結果の表（polars）は
//! Python 側の `globe_mode/__init__.py` が受け持つ。Rust の型を 1 つずつ Python の型に写さない。

use std::path::{Path, PathBuf};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use globe_mode::{simulate_request as request, to_json as json};

fn columns_request(
    model: &str,
    settings_json: Option<&str>,
) -> Result<globe_mode::SimulateRequest, String> {
    let mut request = request(model, settings_json)?;
    request.no_csv = true;
    Ok(request)
}

/// Python 側の例外（`globe_mode/_errors.py`）を名前で引いて作る。`message` が `None` なら診断から読める本文を作る。
/// コールバックの例外や `PanicException` はここを通らず、そのまま Python に出る。
fn error(py: Python<'_>, class: &str, message: Option<String>, diagnostics: Option<&str>) -> PyErr {
    let build = || -> PyResult<PyErr> {
        let class = py.import("globe_mode._errors")?.getattr(class)?;
        let ds = match diagnostics {
            Some(j) => py.import("json")?.call_method1("loads", (j,))?,
            None => PyList::empty(py).into_any(),
        };
        Ok(PyErr::from_value(class.call1((message, ds))?))
    };
    build().unwrap_or_else(|e| e)
}

/// `Progress` を Python の dict に（`stage` は `"integration"` などの小文字、`time` は積分の段だけ）。
fn progress_dict<'py>(py: Python<'py>, p: &globe_mode::Progress) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("stage", p.stage.as_str())?;
    d.set_item("time", p.time)?;
    d.set_item("start_time", p.start_time)?;
    d.set_item("stop_time", p.stop_time)?;
    Ok(d)
}

/// ADR-0033 の Python の段（S129。S200 から `on_progress` が無くても通る 1 本の道）。GIL を離して
/// `simulate_observed` を回し、進捗のたびに GIL を取り直して `check_signals`（Ctrl-C）と、あれば
/// `on_progress(dict)` を呼ぶ。`True`（真の値）が返るか例外が出たらトークンを立てて次の区切りで止め、
/// 例外（`KeyboardInterrupt` など）は実行が戻ってから投げ直す。`finish` も GIL の外で回す。
/// `#[pyclass(unsendable)]` の `&mut self` は、GIL を離しても同じスレッドで使う（`detach` はスレッドを移さない）。
fn simulate_observed<T: Send>(
    py: Python<'_>,
    ws: &mut globe_mode::Workspace,
    request: &globe_mode::SimulateRequest,
    on_progress: Option<Py<PyAny>>,
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
            match &on_progress {
                Some(f) => f.call1(py, (progress_dict(py, p)?,))?.is_truthy(py),
                None => Ok(false),
            }
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
    out.map_err(PyRuntimeError::new_err)
}

/// ライブラリを 1 回だけ読み、ファイルを足しながら、検査・グラフ・実行を繰り返す。
// unsendable: salsa のデータベースは Sync でないので、作ったスレッドからだけ使える
#[pyclass(unsendable, module = "globe_mode._native")]
struct Workspace(globe_mode::Workspace);

#[pymethods]
impl Workspace {
    /// `libraries` はライブラリのディレクトリ（MSL など）。読めなければ `LibraryError`（`.diagnostics` つき）。
    #[new]
    #[pyo3(signature = (libraries = Vec::new()))]
    fn new(py: Python<'_>, libraries: Vec<PathBuf>) -> PyResult<Self> {
        let inner = globe_mode::Workspace::open(&libraries).map_err(|ds| {
            error(
                py,
                "LibraryError",
                None,
                Some(&json(&ds).unwrap_or_default()),
            )
        })?;
        Ok(Workspace(inner))
    }

    fn set_file(&mut self, path: &str, text: String) {
        self.0.set_file(path, text);
    }

    /// ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。読めなければ `LibraryError`。
    fn load_dir(&mut self, py: Python<'_>, dir: PathBuf) -> PyResult<usize> {
        let dir: &Path = &dir;
        self.0.load_dir(dir).map_err(|e| {
            error(
                py,
                "LibraryError",
                Some(format!("{}: {e}", dir.display())),
                None,
            )
        })
    }

    /// 検査（S0〜S10）の結果（`CheckResponse` の JSON）。GIL を離して回す（S200）。
    fn check(&mut self, py: Python<'_>, model: &str) -> PyResult<String> {
        py.detach(|| json(&self.0.check(model)))
            .map_err(PyRuntimeError::new_err)
    }

    /// connect グラフ（`ConnectGraph` の JSON）。モデルが索引に無ければ `ModelNotFoundError`。
    #[pyo3(signature = (model, depth = 1))]
    fn graph(&mut self, py: Python<'_>, model: &str, depth: u32) -> PyResult<String> {
        let graph = py.detach(|| self.0.graph(model, depth).map_err(|e| e.to_string()));
        let graph = graph.map_err(|e| error(py, "ModelNotFoundError", Some(e), None))?;
        json(&graph).map_err(PyRuntimeError::new_err)
    }

    /// ファイル（`set_file` に渡したパス）のアウトライン（`globe outline --json` と同じ `Outline` の JSON）。
    /// ファイルが無いか、構文の誤りがあれば `SourceError`（誤りは最初の診断の `コード: 文`、`.diagnostics` つき）。
    fn outline(&self, py: Python<'_>, path: &str) -> PyResult<String> {
        self.0.outline_json(path).map_err(|message| {
            let ds = self.0.outline(path).and_then(|r| r.as_ref().err());
            let ds = ds.map(|ds| json(ds).unwrap_or_default());
            error(py, "SourceError", Some(message), ds.as_deref())
        })
    }

    /// `check` に渡すファイル（モデルが使うものの閉包。パスの昇順）の JSON の配列。モデルが無ければ `[]`。
    fn deps(&mut self, py: Python<'_>, model: &str) -> PyResult<String> {
        py.detach(|| json(&self.0.deps(model)))
            .map_err(PyRuntimeError::new_err)
    }

    /// flat モデルの要約（`FlatSummary` の JSON）。flatten まで通らなければ `null`。
    fn flat(&mut self, py: Python<'_>, model: &str) -> PyResult<String> {
        py.detach(|| json(&self.0.flat(model)))
            .map_err(PyRuntimeError::new_err)
    }

    /// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。モデルの誤りは例外にせず `ok` と `diagnostics` で返す。
    /// GIL を離して回し（S200 から `on_progress` が無くても）、進捗を dict で知らせる。`True` を返すか Ctrl-C で止まる（S129）。
    #[pyo3(signature = (model, settings = None, on_progress = None))]
    fn simulate(
        &mut self,
        py: Python<'_>,
        model: &str,
        settings: Option<&str>,
        on_progress: Option<Py<PyAny>>,
    ) -> PyResult<String> {
        let request =
            request(model, settings).map_err(|e| error(py, "SettingsError", Some(e), None))?;
        simulate_observed(py, &mut self.0, &request, on_progress, |s| {
            json(&s.response)
        })
    }

    /// [`Workspace::simulate`] と同じだが、時系列を `(結果の JSON, 列の名前, 列の bytes)` で返す（S085）。
    /// 列の bytes は GIL を取り直してから Rust の列から直接詰める（中間の `Vec<u8>` は作らない。S200）。
    #[pyo3(signature = (model, settings = None, on_progress = None))]
    fn simulate_columns<'py>(
        &mut self,
        py: Python<'py>,
        model: &str,
        settings: Option<&str>,
        on_progress: Option<Py<PyAny>>,
    ) -> PyResult<(String, Vec<String>, Vec<Bound<'py, PyBytes>>)> {
        let request = columns_request(model, settings)
            .map_err(|e| error(py, "SettingsError", Some(e), None))?;
        let (response, simulated) =
            simulate_observed(py, &mut self.0, &request, on_progress, |s| {
                // 既定で描く信号（S190）は結果の dict の `defaults` に添える
                let mut response =
                    serde_json::to_value(&s.response).map_err(|e| format!("JSON: {e}"))?;
                response["defaults"] = serde_json::json!(s.columns().0.defaults);
                Ok((json(&response)?, s))
            })?;
        let (layout, columns) = simulated.columns();
        let columns = pack(py, &columns)?;
        Ok((response, layout.names, columns))
    }

    /// FMU を書き出し（S118。`globe fmu export` と同じ）、`.fmu` のパスを返す。`options` は
    /// `{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}` の JSON（省略可）。失敗は `ExportError`。
    #[pyo3(signature = (model, out_dir, options = None))]
    fn export_fmu(
        &mut self,
        py: Python<'_>,
        model: &str,
        out_dir: PathBuf,
        options: Option<&str>,
    ) -> PyResult<PathBuf> {
        let options = globe_mode::FmuOptions::from_json(options)
            .map_err(|e| error(py, "ExportError", Some(e), None))?;
        py.detach(|| self.0.export_fmu(model, &out_dir, &options))
            .map_err(|e| error(py, "ExportError", Some(e), None))
    }
}

/// 列を little-endian の f64 の bytes に詰める。
fn pack<'py>(py: Python<'py>, columns: &[&[f64]]) -> PyResult<Vec<Bound<'py, PyBytes>>> {
    columns
        .iter()
        .map(|c| {
            PyBytes::new_with(py, c.len() * 8, |buf| {
                for (b, x) in buf.as_chunks_mut::<8>().0.iter_mut().zip(c.iter()) {
                    b.copy_from_slice(&x.to_le_bytes());
                }
                Ok(())
            })
        })
        .collect()
}

/// 結果のファイル（`.mat` は MAT v4、それ以外は CSV。S219）を読み、`(パラメータの JSON, 列の名前, 列の bytes)` で返す。
/// 読めなければ `ResultError`。
#[pyfunction]
fn read_results<'py>(
    py: Python<'py>,
    path: PathBuf,
) -> PyResult<(String, Vec<String>, Vec<Bound<'py, PyBytes>>)> {
    let c = globe_mode::sim::mat::read_results(&path).map_err(|e| {
        error(
            py,
            "ResultError",
            Some(format!("{}: {e}", path.display())),
            None,
        )
    })?;
    let t = &c.trajectory;
    let names = std::iter::once("time".to_string())
        .chain(t.variables.iter().map(|v| v.name.clone()))
        .collect();
    let columns: Vec<&[f64]> = std::iter::once(t.time.as_slice())
        .chain(t.variables.iter().map(|v| v.values.as_slice()))
        .collect();
    let parameters = json(&c.parameters).map_err(PyRuntimeError::new_err)?;
    Ok((parameters, names, pack(py, &columns)?))
}

/// FMU の要約（`FmuInfo` の JSON。S229）。読めなければ `FmuError`。ネイティブコードは実行しない。
#[pyfunction]
fn fmu_info(py: Python<'_>, path: PathBuf) -> PyResult<String> {
    globe_mode::import_fmu_info(&path)
        .and_then(|i| json(&i))
        .map_err(|e| error(py, "FmuError", Some(e), None))
}

/// FMU を別プロセスのワーカー（`worker` の `fmu __worker`）で回し、`(ME の要約の JSON か null, 列の名前, 列の bytes)` を返す（S229）。
/// GIL を離して回し、進捗のたびに `check_signals`（Ctrl-C）と `on_progress(t)` を呼ぶ。真の値を返すか例外が出たらワーカーを止める。
/// 例外（`KeyboardInterrupt` など）は戻ってから投げ直す。失敗は `FmuError`。
// shortcut: Ctrl-C と on_progress はワーカーの進捗（0.25 秒おき）のときだけ見る。進捗の出ない FMU は `timeout` で止める
#[pyfunction]
#[pyo3(signature = (worker, path, request = None, on_progress = None))]
fn run_fmu<'py>(
    py: Python<'py>,
    worker: PathBuf,
    path: PathBuf,
    request: Option<&str>,
    on_progress: Option<Py<PyAny>>,
) -> PyResult<(String, Vec<String>, Vec<Bound<'py, PyBytes>>)> {
    let request: globe_mode::FmuRunRequest = request
        .map_or_else(
            || Ok(Default::default()),
            |r| serde_json::from_str(r).map_err(|e| format!("request が読めない: {e}")),
        )
        .map_err(|e| error(py, "FmuError", Some(e), None))?;
    let token = globe_mode::CancelToken::new();
    let mut raised: Option<PyErr> = None;
    let mut notify = |t: f64| {
        if raised.is_some() {
            return;
        }
        let stop = Python::attach(|py| {
            py.check_signals()?;
            match &on_progress {
                Some(f) => f.call1(py, (t,))?.is_truthy(py),
                None => Ok(false),
            }
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
    let run = py
        .detach(move || globe_mode::run_fmu(&worker, &path, &request, &mut notify, Some(&cancel)));
    if let Some(e) = raised {
        return Err(e);
    }
    let run = run.map_err(|e| error(py, "FmuError", Some(e), None))?;
    let (layout, columns) = run.results.columns();
    let me = json(&run.me).map_err(PyRuntimeError::new_err)?;
    Ok((me, layout.names, pack(py, &columns)?))
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

/// 本文を整形する（S111）。文法が壊れていれば `SourceError`。
#[pyfunction]
#[pyo3(signature = (text, indent = 2, insert_origin = true))]
fn format(py: Python<'_>, text: &str, indent: usize, insert_origin: bool) -> PyResult<String> {
    globe_mode::format(text, indent, insert_origin)
        .map_err(|e| error(py, "SourceError", Some(e), None))
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
    m.add_function(wrap_pyfunction!(read_results, m)?)?;
    m.add_function(wrap_pyfunction!(fmu_info, m)?)?;
    m.add_function(wrap_pyfunction!(run_fmu, m)?)?;
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
    fn requests_parse_settings_and_columns_drop_the_csv() {
        assert!(request("Decay", Some("{")).is_err());
        assert!(columns_request("Decay", Some(r#"{"stop_time": "a"}"#)).is_err());
        let r = columns_request("Decay", Some(r#"{"stop_time": 1}"#)).unwrap();
        assert!(r.no_csv);
        let mut ws = globe_mode::Workspace::new();
        ws.set_file("Decay.mo", DECAY);
        let simulated = ws.simulate_with(&r, &globe_mode::now_ms);
        let (layout, columns) = simulated.columns();
        assert_eq!(layout.names, ["time", "x"]);
        assert!((columns[1][columns[1].len() - 1] - (-1f64).exp()).abs() < 1e-4);
    }
}

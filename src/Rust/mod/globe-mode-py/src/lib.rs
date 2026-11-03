//! GlobeMode の Python の束ね（S039）。facade `globe-mode` の `Workspace` を PyO3 で包む。
//!
//! 入出力は globe-mode-wasm と同じく JSON の文字列にして、`dict` への変換と結果の表（polars）は
//! Python 側の `globe_mode/__init__.py` が受け持つ。Rust の型を 1 つずつ Python の型に写さない。

use std::path::{Path, PathBuf};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
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

/// [`simulate_json`] と同じだが、時系列を CSV にせず列ごとに返す（S085。wasm の `SimColumns` と同じ並び）。
fn simulate_columns(
    ws: &mut globe_mode::Workspace,
    model: &str,
    settings_json: Option<&str>,
) -> Result<Columns, String> {
    let mut request = request(model, settings_json)?;
    request.no_csv = true;
    let globe_mode::Simulated { response, run, .. } =
        ws.simulate_with(&request, &globe_mode::now_ms);
    let bytes = |v: &[f64]| v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let (names, columns) = match &run {
        Some(run) => std::iter::once(("time".to_string(), bytes(&run.trajectory.time)))
            .chain(
                run.trajectory
                    .variables
                    .iter()
                    .map(|v| (v.name.clone(), bytes(&v.values))),
            )
            .unzip(),
        None => (Vec::new(), Vec::new()),
    };
    Ok((json(&response)?, names, columns))
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

    /// 検査 → コンパイル → 実行（`SimulateResponse` の JSON）。モデルの誤りは例外にせず `ok` と `diagnostics` で返す。
    #[pyo3(signature = (model, settings = None))]
    fn simulate(&mut self, model: &str, settings: Option<&str>) -> PyResult<String> {
        simulate_json(&mut self.0, model, settings).map_err(PyValueError::new_err)
    }

    /// [`Workspace::simulate`] と同じだが、時系列を `(結果の JSON, 列の名前, 列の bytes)` で返す（S085）。
    #[pyo3(signature = (model, settings = None))]
    fn simulate_columns<'py>(
        &mut self,
        py: Python<'py>,
        model: &str,
        settings: Option<&str>,
    ) -> PyResult<(String, Vec<String>, Vec<Bound<'py, PyBytes>>)> {
        let (response, names, columns) =
            simulate_columns(&mut self.0, model, settings).map_err(PyValueError::new_err)?;
        let columns = columns.iter().map(|c| PyBytes::new(py, c)).collect();
        Ok((response, names, columns))
    }
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

//! GlobeMode の Python の束ね（S039）。facade `globe-mode` の `Workspace` を PyO3 で包む。
//!
//! 入出力は globe-mode-wasm と同じく JSON の文字列にして、`dict` への変換と結果の表（polars）は
//! Python 側の `globe_mode/__init__.py` が受け持つ。Rust の型を 1 つずつ Python の型に写さない。

use std::path::{Path, PathBuf};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use serde::Serialize;

fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON: {e}"))
}

/// `settings_json`（`{"stop_time": 2, "solver": "bdf"}` など。省略可）から実行の依頼を作って回す。
fn simulate_json(
    ws: &mut globe_mode::Workspace,
    model: &str,
    settings_json: Option<&str>,
) -> Result<String, String> {
    let settings: serde_json::Value = match settings_json {
        Some(s) => {
            serde_json::from_str(s).map_err(|e| format!("settings の JSON が読めない: {e}"))?
        }
        None => serde_json::json!({}),
    };
    let request =
        serde_json::from_value(serde_json::json!({ "model": model, "settings": settings }))
            .map_err(|e| format!("settings が読めない: {e}"))?;
    json(&ws.simulate(&request))
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
}

/// ソルバの登録表（JSON の配列）。
#[pyfunction]
fn solvers() -> PyResult<String> {
    json(&globe_mode::solvers()).map_err(PyRuntimeError::new_err)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Workspace>()?;
    m.add_function(wrap_pyfunction!(solvers, m)?)
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
        assert!(graph_json(&mut ws, "NoSuchModel", 1).is_err());
    }
}

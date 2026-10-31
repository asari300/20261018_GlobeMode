"""GlobeMode の Python API（Rust の facade `globe-mode` を PyO3 で包む。S039）。

    ws = Workspace(libraries=["src/Modelica"])
    ws.check("GlobeModeValidation.Simulation.Decay")["passed"]
    res = ws.simulate("GlobeModeValidation.Simulation.Decay", stop_time=2)
    table(res)  # polars.DataFrame（time 列と変数の列）
"""

import io
import json

import polars as pl

from . import _native


class Workspace:
    def __init__(self, libraries: list[str] = ()):
        self._ws = _native.Workspace([str(p) for p in libraries])

    def set_file(self, path: str, text: str) -> None:
        self._ws.set_file(path, text)

    def load_dir(self, dir: str) -> int:
        return self._ws.load_dir(str(dir))

    def check(self, model: str) -> dict:
        return json.loads(self._ws.check(model))

    def graph(self, model: str, depth: int = 1) -> dict:
        return json.loads(self._ws.graph(model, depth))

    def simulate(self, model: str, **settings) -> dict:
        """settings は `start_time`・`stop_time`・`tolerance`・`interval`・`solver`・`parameters` など。"""
        return json.loads(self._ws.simulate(model, json.dumps(settings)))


def table(response: dict) -> pl.DataFrame:
    """`Workspace.simulate` の結果の CSV を表にする。実行できなかったら ValueError。"""
    if response["csv"] is None:
        raise ValueError(response["refused"] or "結果が無い")
    return pl.read_csv(io.StringIO(response["csv"]))


def solvers() -> list[dict]:
    return json.loads(_native.solvers())


def main() -> None:
    print(f"GlobeMode: ソルバ {len(solvers())} 種。CLI は `cargo run -p globe-mode-cli -- --help`。")

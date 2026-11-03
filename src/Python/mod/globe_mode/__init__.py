"""GlobeMode の Python API（Rust の facade `globe-mode` を PyO3 で包む。S039）。

    ws = Workspace(libraries=["src/Modelica"])
    ws.check("GlobeModeValidation.Simulation.Decay")["passed"]
    res = ws.simulate_columns("GlobeModeValidation.Simulation.Decay", stop_time=2)  # CSV の道は simulate
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

    def simulate_columns(self, model: str, **settings) -> dict:
        """`simulate` と同じだが CSV を作らず、`columns` に列（名前 → polars.Series、`time` が先頭）を添える（S085）。"""
        response, names, columns = self._ws.simulate_columns(model, json.dumps(settings))
        response = json.loads(response)
        # shortcut: memoryview の "d" は native-endian（Rust は little-endian で詰める）、big-endian の機械なら byteswap が要る
        response["columns"] = {n: pl.Series(n, memoryview(c).cast("d"), pl.Float64) for n, c in zip(names, columns)}
        return response


def table(response: dict) -> pl.DataFrame:
    """`Workspace.simulate_columns`（列）か `simulate`（CSV）の結果を表にする。実行できなかったら ValueError。"""
    if response.get("columns"):
        return pl.DataFrame(list(response["columns"].values()))
    if response.get("csv") is None:
        raise ValueError(response["refused"] or "結果が無い")
    return pl.read_csv(io.StringIO(response["csv"]))


def solvers() -> list[dict]:
    return json.loads(_native.solvers())


def format(text: str, indent: int = 2, insert_origin: bool = True) -> str:
    """本文を整形する（S111）。文法が壊れていれば ValueError。"""
    return _native.format(text, indent, insert_origin)


def lint(text: str) -> list[dict]:
    """lint の結果（`rule`・`line`・`column`・`message` の dict のリスト。S111）。"""
    return json.loads(_native.lint(text))


def main() -> None:
    print(f"GlobeMode: ソルバ {len(solvers())} 種。CLI は `cargo run -p globe-mode-cli -- --help`。")

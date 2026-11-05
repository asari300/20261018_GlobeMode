"""GlobeMode の Python API（Rust の facade `globe-mode` を PyO3 で包む。S039）。

ライブラリ（MSL など）を開いて、検査・実行・整形をする。結果は `dict`（と polars の表）で返る。

    from globe_mode import Workspace, table

    ws = Workspace(libraries=["data/msl"])           # 1 つだけなら Workspace("data/msl") でもよい
    ws.check("Modelica.Electrical.Analog.Examples.ChuaCircuit")["passed"]
    res = ws.simulate_columns("Modelica.Electrical.Analog.Examples.ChuaCircuit", stop_time=2)
    table(res)  # polars.DataFrame（time 列と変数の列）

- 使い方: `Workspace`（開く・`check`・`outline`・`deps`・`flat`・`graph`・`simulate`・`simulate_columns`・`export_fmu`）、`table`、`format`、
  `lint`、`solvers`。それぞれ `help()` に引数・戻り値・誤りがある。
- 誤りの扱い: **モデルの誤りは例外にせず、結果の `diagnostics`（`ok`・`passed`）で返す**（未知のモデルも `check` は
  `passed: False`）。呼び方の誤りは例外: 未知の設定のキーは `TypeError`、設定の値の型の誤りは `ValueError`、
  ライブラリが読めなければ `RuntimeError`（元の診断は `.diagnostics`）。
- 型: `py.typed` つき。結果は `dict[str, Any]` で、キーは CLI の `--json` と同じ形（`simulate` の成否は `ok`）。
"""

import io
import json
import os
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import Any, TypedDict, Unpack

import polars as pl

from . import _native


class Progress(TypedDict):
    """`on_progress` に渡る dict。`stage` は `lowering`・`preflight`・`build`・`initialization`・`integration`、
    `time` は積分の時刻（session の道の `integration` だけ。ほかは `None`）。"""

    stage: str
    time: float | None
    start_time: float
    stop_time: float


class Settings(TypedDict, total=False):
    """`simulate` / `simulate_columns` の設定。省略した項目はモデルの experiment か既定値で決まる。"""

    start_time: float | None
    stop_time: float | None
    tolerance: float | None  # 相対許容誤差（絶対にも同じ値）
    interval: float | None  # 出力の間隔
    solver: str | None  # `solvers()` の名前（bdf・dopri45・auto・esdirk34 …）
    max_wall_seconds: float | None  # 壁時計の予算（秒）
    step: float | None  # 固定刻みのソルバ（euler・heun・rk4・impeuler・trapezoid）の刻み幅
    parameters: dict[str, float]  # パラメータの上書き（名前 → 値。tunable なものだけ）


# 設定のキーと値の型は Rust の `SimSettings`（全部埋めた見本）から取る。Settings との一致は試験が見る。
_SAMPLE: dict[str, Any] = json.loads(_native.settings_sample())


def _is(value: Any, sample: Any) -> bool:
    if isinstance(sample, dict):
        return isinstance(value, dict) and all(isinstance(k, str) and _is(v, 0.0) for k, v in value.items())
    if isinstance(sample, float):
        return isinstance(value, int | float) and not isinstance(value, bool)
    return isinstance(value, type(sample))


def _settings_json(settings: Mapping[str, Any]) -> str:
    """未知のキーは `TypeError`、型の誤りは（どの設定かを言う）`ValueError`。"""
    if unknown := settings.keys() - _SAMPLE.keys():
        raise TypeError(f"未知の設定 {sorted(unknown)}。使えるのは {', '.join(_SAMPLE)}")
    for key, value in settings.items():
        if value is not None and not _is(value, _SAMPLE[key]):
            want = {float: "数値", str: "文字列", dict: "名前（文字列）から数値への dict"}[type(_SAMPLE[key])]
            raise ValueError(f"設定 {key} は{want} のはずが {value!r}（{type(value).__name__}）")
    return json.dumps(settings)


class Workspace:
    """ライブラリを 1 回だけ読み、ファイルを足しながら、検査・グラフ・実行を繰り返す作業の場。

    `libraries` はライブラリのディレクトリ（例: MSL の `data/msl`。`package.mo` のある 1 つ上＝MSL ごと）のリスト。
    1 つだけなら文字列か `Path` を直接渡してもよい（1 つのライブラリとして読む。1 文字ずつには分けない）。
    読めなければ `RuntimeError`（本文は最初の error の `コード: 文`、元の診断のリストは `.diagnostics`）。

        ws = Workspace(["data/msl"])
        ws.set_file("RC.mo", "model RC ... end RC;")   # ファイルを足す・差し替える
        ws.check("RC")["passed"]

    1 つの `Workspace` は作ったスレッドからだけ使える。
    """

    def __init__(self, libraries: str | os.PathLike[str] | Iterable[str | os.PathLike[str]] = ()):
        if isinstance(libraries, str | os.PathLike):
            libraries = [libraries]
        try:
            self._ws = _native.Workspace([str(p) for p in libraries])
        except RuntimeError as e:
            raise _readable(e) from None

    def set_file(self, path: str, text: str) -> None:
        """ファイルを 1 つ足す（同じ `path` なら本文を差し替える）。`path` は名前でよく、実在しなくてもよい。"""
        self._ws.set_file(path, text)

    def load_dir(self, dir: str | os.PathLike[str]) -> int:
        """ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。読めなければ `RuntimeError`。"""
        return self._ws.load_dir(str(dir))

    def check(self, model: str) -> dict[str, Any]:
        """モデル（完全修飾名）を検査する（S0〜S10。`globe check --json` と同じ形）。

        戻り値: `passed`（bool）・`failed_stage`・`diagnostics`（`code`・`severity`・`stage`・`message` の
        dict のリスト）・`balance`・`dae`・`structure`・`s6`〜`s10`・`init`・`flat`・`summary`（結果の 1 行。
        `globe check` の最後の行と同じ。例 `RC: OK（S0〜S10 を通過）`）。
        **モデルの誤りは例外にならない**: 未知のモデルも `passed: False` と `diagnostics`（`GM-E1001` など）で返る。

            res = ws.check("RC")
            if not res["passed"]:
                print(res["diagnostics"][0]["message"])
        """
        return json.loads(self._ws.check(model))

    def graph(self, model: str, depth: int = 1) -> dict[str, Any]:
        """connect のグラフ（`nodes`・`edges`）。`depth` は部品の入れ子を何段まで開くか。

        モデルが索引に無ければ `ValueError`。"""
        return json.loads(self._ws.graph(model, depth))

    def outline(self, path: str) -> dict[str, Any]:
        """`set_file` に渡した `path` のファイルのアウトライン（`globe outline --json` と同じ。`within`・`classes`）。

        `classes` の各要素は `name`・`kind`（model・package など）・`range`（バイトの位置）・`extends`・入れ子の `classes` など。
        ファイルが無いか、構文の誤りがあれば `ValueError`（誤りは最初の診断の `コード: 文`）。

            ws.set_file("A.mo", "model A Real x; end A;")
            [c["name"] for c in ws.outline("A.mo")["classes"]]   # ["A"]
        """
        return json.loads(self._ws.outline(path))

    def deps(self, model: str) -> list[str]:
        """`check(model)` が読むファイル（モデルが使うものの閉包。`set_file` したパスの昇順。`globe deps --json` と同じ）。

        ワークスペースに無いモデル（ライブラリの中のモデルや存在しない名前）は空のリスト。"""
        return json.loads(self._ws.deps(model))

    def flat(self, model: str) -> dict[str, Any] | None:
        """flat モデルの要約（`globe flat --json` と同じ。`variables`・`equations`・`initial_equations`・`partial`・
        `top_level_connectors`・`equations_by_component`）。flatten まで通らなければ `None`（理由は `check` の `diagnostics`）。"""
        return json.loads(self._ws.flat(model))

    def simulate(
        self,
        model: str,
        on_progress: Callable[[Progress], Any] | None = None,
        **settings: Unpack[Settings],
    ) -> dict[str, Any]:
        """検査 → コンパイル → 実行をして、結果を dict で返す（時系列は CSV の文字列 `csv`。表にするのは `table`）。

        `settings` は `Settings`（`start_time`・`stop_time`・`tolerance`・`interval`・`solver`・`max_wall_seconds`・
        `step`・`parameters`）。省略した項目はモデルの experiment か既定値で決まり、決めた値は結果の `settings` に出る。
        **未知のキー（`stoptime` など）は `TypeError`**（使えるキーを添える）、値の型の誤りは `ValueError`（どの設定かを言う）。

        戻り値: `ok`（結果があり Error が無い）・`refused`（実行しなかった理由）・`diagnostics`・`settings`・`termination`・
        `stats`・`final`（最後の時刻の値）・`csv`・`timings_ms`・`summary`（実行しなかった・失敗したときだけ。理由と最初の error の 1 行で、
        `globe simulate` の最後の行と同じ）。**モデルの誤り（未知のソルバ名、実行できないモデル）は例外にせず**
        `ok: False` と `diagnostics` で返る。

            res = ws.simulate("RC", stop_time=5, solver="auto", parameters={"R": 2.0})
            if res["ok"]:
                df = table(res)

        `on_progress(p)` を渡すと GIL を離して回し、進捗を `Progress` の dict で呼ぶ。`True`（真の値）を返すと止まり、
        `termination` が `cancelled` で途中までの結果と情報 `GM-I11004` が返る。コールバックが例外を出したらそれを投げ直す。
        Ctrl-C は `KeyboardInterrupt`（S129、ADR-0033）。

        **既定のソルバ（`bdf`・`auto`・`dopri45`）では `on_progress` は積分の中に届かない**（ADR-0033 の既知の制約。
        Rumoca に口が無い）。呼ばれるのは段（lowering・preflight・build・initialization・integration）の変わり目の
        5 回ほどで `time` は `None`。積分の段が始まってから `True` を返しても（Ctrl-C も）積分は止まらず最後まで回る
        （積分より前の段で返せば次の段に入る前に止まるが、その結果は `ok: False` で時系列が無い）。
        積分の時刻が付き、途中で止められるのは session の道のソルバ（`esdirk34` など。`solvers()` で確かめる）で、
        t は 100 ms に 1 回まで。長い実行を止めたいなら `max_wall_seconds` を使う。
        """
        return json.loads(self._ws.simulate(model, _settings_json(settings), on_progress))

    def simulate_columns(
        self,
        model: str,
        on_progress: Callable[[Progress], Any] | None = None,
        **settings: Unpack[Settings],
    ) -> dict[str, Any]:
        """`simulate` と同じ（設定・誤り・`on_progress` も同じ）だが、CSV を作らず `columns` に列を添える（S085）。

        `columns` は名前 → `polars.Series`（`time` が先頭）。`csv` は `None`。10 万点級の結果は `simulate` + `table` より速い。

            res = ws.simulate_columns("RC", stop_time=5)
            table(res)["time"]
        """
        response, names, columns = self._ws.simulate_columns(model, _settings_json(settings), on_progress)
        response = json.loads(response)
        # shortcut: memoryview の "d" は native-endian（Rust は little-endian で詰める）、big-endian の機械なら byteswap が要る
        response["columns"] = {n: pl.Series(n, memoryview(c).cast("d"), pl.Float64) for n, c in zip(names, columns)}
        return response

    def export_fmu(
        self,
        model: str,
        out_dir: str | os.PathLike[str],
        *,
        fmi_version: int = 2,
        build: bool = False,
        targets: Iterable[str] = (),
        fmi_headers: str | os.PathLike[str] | None = None,
    ) -> Path:
        """FMU を `out_dir/<名前>-fmi2.fmu`（3.0 は `-fmi3`。と展開したディレクトリ）に書き、`.fmu` のパスを返す（S118。`globe fmu export` と同じ）。

        `fmi_version` は 2 か 3。`build=True` で共有ライブラリも作る（`fmi_headers`＝`fmi2Functions.h` などのある
        ディレクトリが要る。`targets` が空なら host を `cc` で、あれば `zig cc -target`）。
        未知のキーは `TypeError`、書き出せなければ（版が 2・3 でない、モデルの誤りなど）`ValueError`。

            ws.export_fmu("RC", "out", fmi_version=3)
        """
        options = dict(fmi_version=fmi_version, build=build, targets=list(targets), fmi_headers=None if fmi_headers is None else str(fmi_headers))
        return Path(self._ws.export_fmu(model, str(out_dir), json.dumps(options)))


def _readable(e: RuntimeError) -> RuntimeError:
    """診断の JSON を本文にした `RuntimeError` を、最初の error の `コード: 文（位置・補足）` に直す。元は `.diagnostics`。"""
    try:
        ds = json.loads(e.args[0])
        d = next((d for d in ds if d["severity"] == "error"), ds[0])
    except (json.JSONDecodeError, IndexError, KeyError, TypeError):
        return e  # 診断の JSON でなければそのまま
    where = [d["file"]] if d.get("file") else []
    if r := d.get("range"):
        where.append(f"バイト {r['start']}〜{r['end']}")
    text = f"{d['code']}: {d['message']}" + (f"（{'、'.join(where + d.get('notes', []))}）" if where or d.get("notes") else "")
    err = RuntimeError(text + (f" ほか {len(ds) - 1} 件" if len(ds) > 1 else ""))
    err.diagnostics = ds  # type: ignore[attr-defined]
    return err


def table(response: dict[str, Any]) -> pl.DataFrame:
    """`Workspace.simulate_columns`（列）か `simulate`（CSV）の結果を表（`time` 列と変数の列）にする。

    実行できなかった結果（`ok: False` で時系列が無い）なら `ValueError`（本文は `refused` の理由）。"""
    if response.get("columns"):
        return pl.DataFrame(list(response["columns"].values()))
    if response.get("csv") is None:
        raise ValueError(response["refused"] or "結果が無い")
    return pl.read_csv(io.StringIO(response["csv"]))


def solvers() -> list[dict[str, Any]]:
    """ソルバの登録表（`name`・別名・実装の有無など）。`simulate(solver=…)` の名前はここから選ぶ。"""
    return json.loads(_native.solvers())


def format(text: str, indent: int = 2, insert_origin: bool = True) -> str:
    """Modelica の本文を整形して返す（S111。`globe fmt` と同じ規則）。文法が壊れていれば `ValueError`。

    `indent` はインデントの幅、`insert_origin` は annotation の `origin` の補完。

        format("model A Real x; end A;")
    """
    return _native.format(text, indent, insert_origin)


def lint(text: str) -> list[dict[str, Any]]:
    """lint の結果（`rule`・`line`・`column`・`message` の dict のリスト。S111。問題が無ければ空）。"""
    return json.loads(_native.lint(text))


def main() -> None:
    """`globe-mode` コマンド。版とソルバの数を出すだけで、検査や実行は CLI の `globe`（別の実行ファイル）か Python から。"""
    from importlib.metadata import version

    print(f"GlobeMode {version('globe-mode')}: ソルバ {len(solvers())} 種。Python は `help(globe_mode)`、CLI は `globe --help`。")

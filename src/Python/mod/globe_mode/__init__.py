"""GlobeMode の Python API（Rust の facade `globe-mode` を PyO3 で包む。S039）。

ライブラリ（MSL など）を開いて、検査・実行・整形をする。結果は `dict`（と polars の表）で返る。

    from globe_mode import Workspace, table

    ws = Workspace(libraries=["data/msl"])           # 1 つだけなら Workspace("data/msl") でもよい
    ws.check("Modelica.Electrical.Analog.Examples.ChuaCircuit")["passed"]
    res = ws.simulate_columns("Modelica.Electrical.Analog.Examples.ChuaCircuit", stop_time=2)
    table(res)  # polars.DataFrame（time 列と変数の列）

- 使い方: `Workspace`（開く・`check`・`outline`・`deps`・`flat`・`graph`・`simulate`・`simulate_columns`・`export_fmu`）、`table`、`format`、
  `lint`、`solvers`、`read_results`（結果の MAT v4・CSV を読む）、`fmu_info`・`run_fmu`（FMU を読む・回す。FMI Import、S229）。
  それぞれ `help()` に引数・戻り値・誤りがある。
- 誤りの扱い: **モデルの誤りは例外にせず、結果の `diagnostics`（`ok`・`passed`）で返す**（未知のモデルも `check` は
  `passed: False`）。ライブラリが出す例外はすべて `GlobeModeError` の子（`.diagnostics` と最初の診断の `.code` を持つ）で、
  これまでの組み込みの型も継ぐので `except ValueError` などでも受けられる:

  | 例外 | 継ぐ型 | 出る場合 |
  | --- | --- | --- |
  | `LibraryError` | `RuntimeError` | `Workspace(libraries)`・`load_dir` がライブラリを読めない（`.diagnostics`） |
  | `SettingsError` | `ValueError` | `simulate` の設定の値や型の誤り（NaN・inf を含む。どの設定かを言う） |
  | `ModelNotFoundError` | `ValueError` | `graph` のモデルが索引に無い |
  | `SourceError` | `ValueError` | `outline` のファイルが無い・構文の誤り、`format` の構文の誤り |
  | `ExportError` | `ValueError` | `export_fmu` が書き出せない |
  | `ResultError` | `ValueError` | `table` に時系列の無い結果を渡した、`read_results` が結果のファイルを読めない |
  | `FmuError` | `RuntimeError` | `fmu_info` が FMU を読めない、`run_fmu` が回せない（FMU が落ちた・時間切れ・止めた・`globe` が見つからない） |

  **呼び方の誤りは素の `TypeError`**（未知の設定のキー、`on_progress` が呼べない、`libraries` が bytes）で、
  コールバックが出した例外と Rust の panic（`pyo3_runtime.PanicException`）はそのまま通る。
- 型: `py.typed` つき。結果は `dict[str, Any]` で、キーは CLI の `--json` と同じ形（`simulate` の成否は `ok`）。
"""

import io
import json
import math
import numbers
import os
import shutil
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import Any, Literal, TypedDict, Unpack

import polars as pl

from . import _native
from ._errors import (
    ExportError,
    FmuError,
    GlobeModeError,
    LibraryError,
    ModelNotFoundError,
    ResultError,
    SettingsError,
    SourceError,
)

__all__ = [
    "ExportError",
    "FmuError",
    "GlobeModeError",
    "LibraryError",
    "ModelNotFoundError",
    "Progress",
    "ResultError",
    "Settings",
    "SettingsError",
    "SourceError",
    "Workspace",
    "fmu_info",
    "format",
    "lint",
    "main",
    "read_results",
    "run_fmu",
    "solvers",
    "table",
]


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


_BAD = object()


def _number(value: Any) -> Any:
    """`bool` でない実数（numpy の数も）を有限の `float` に。それ以外は `_BAD`。"""
    if isinstance(value, bool) or not isinstance(value, numbers.Real):
        return _BAD
    try:
        x = float(value)
    except OverflowError:
        return _BAD
    return x if math.isfinite(x) else _BAD


def _coerce(value: Any, sample: Any) -> Any:
    if isinstance(sample, dict):
        if not (isinstance(value, dict) and all(isinstance(k, str) for k in value)):
            return _BAD
        nums = {k: _number(v) for k, v in value.items()}
        return _BAD if _BAD in nums.values() else nums
    if isinstance(sample, float):
        return _number(value)
    return value if isinstance(value, type(sample)) else _BAD


def _settings_json(settings: Mapping[str, Any]) -> str:
    """未知のキーは `TypeError`、型の誤りは（どの設定かを言う）`SettingsError`。`None` は「省略」で送らない。"""
    if unknown := settings.keys() - _SAMPLE.keys():
        raise TypeError(f"未知の設定 {sorted(unknown)}。使えるのは {', '.join(_SAMPLE)}")
    out = {}
    for key, value in settings.items():
        if value is None:
            continue
        out[key] = _coerce(value, _SAMPLE[key])
        if out[key] is _BAD:
            want = {float: "数値（有限）", str: "文字列", dict: "名前（文字列）から数値（有限）への dict"}[type(_SAMPLE[key])]
            raise SettingsError(f"設定 {key} は{want} のはずが {value!r}（{type(value).__name__}）")
    return json.dumps(out)


def _callback(on_progress: Any) -> Any:
    if on_progress is not None and not callable(on_progress):
        raise TypeError(f"on_progress は dict を 1 つ受ける呼べるもののはずが {on_progress!r}（設定は stop_time=2 のようにキーワードで渡す）")
    return on_progress


class Workspace:
    """ライブラリを 1 回だけ読み、ファイルを足しながら、検査・グラフ・実行を繰り返す作業の場。

    `libraries` はライブラリのディレクトリ（例: MSL の `data/msl`。`package.mo` のある 1 つ上＝MSL ごと）のリスト。
    1 つだけなら文字列か `Path` を直接渡してもよい（1 つのライブラリとして読む。1 文字ずつには分けない）。
    読めなければ `LibraryError`（`RuntimeError` でもある。本文は最初の error の `コード: 文`、元の診断のリストは `.diagnostics`）。

        ws = Workspace(["data/msl"])
        ws.set_file("RC.mo", "model RC ... end RC;")   # ファイルを足す・差し替える
        ws.check("RC")["passed"]

    1 つの `Workspace` は作ったスレッドからだけ使える。
    """

    def __init__(self, libraries: str | os.PathLike[str] | Iterable[str | os.PathLike[str]] = ()):
        if isinstance(libraries, str | os.PathLike):
            libraries = [libraries]
        elif isinstance(libraries, bytes | bytearray):
            raise TypeError("libraries は文字列か Path（かそのリスト）のはずが bytes")
        self._ws = _native.Workspace(list(libraries))

    def set_file(self, path: str, text: str) -> None:
        """ファイルを 1 つ足す（同じ `path` なら本文を差し替える）。`path` は名前でよく、実在しなくてもよい。"""
        self._ws.set_file(path, text)

    def load_dir(self, dir: str | os.PathLike[str]) -> int:
        """ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。読めなければ `LibraryError`。"""
        return self._ws.load_dir(dir)

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

        モデルが索引に無ければ `ModelNotFoundError`（`ValueError` でもある）。"""
        return json.loads(self._ws.graph(model, depth))

    def outline(self, path: str) -> dict[str, Any]:
        """`set_file` に渡した `path` のファイルのアウトライン（`globe outline --json` と同じ。`within`・`classes`）。

        `classes` の各要素は `name`・`kind`（model・package など）・`range`（バイトの位置）・`extends`・入れ子の `classes` など。
        ファイルが無いか、構文の誤りがあれば `SourceError`（`ValueError` でもある。誤りは最初の診断の `コード: 文`、診断は `.diagnostics`）。

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
        *,
        on_progress: Callable[[Progress], Any] | None = None,
        **settings: Unpack[Settings],
    ) -> dict[str, Any]:
        """検査 → コンパイル → 実行をして、結果を dict で返す（時系列は CSV の文字列 `csv`。表にするのは `table`）。

        `settings` は `Settings`（`start_time`・`stop_time`・`tolerance`・`interval`・`solver`・`max_wall_seconds`・
        `step`・`parameters`）。省略した項目はモデルの experiment か既定値で決まり、決めた値は結果の `settings` に出る。
        **未知のキー（`stoptime` など）は `TypeError`**（使えるキーを添える）、値の型の誤り・NaN・inf は `SettingsError`
        （`ValueError` でもある。どの設定かを言う）。`None` は省略と同じ。numpy の数も使える。設定は必ずキーワードで渡す（`on_progress` もキーワード専用）。

        戻り値: `ok`（結果があり Error が無い）・`refused`（実行しなかった理由）・`diagnostics`・`settings`・`termination`・
        `stats`・`final`（最後の時刻の値）・`csv`・`timings_ms`・`summary`（実行しなかった・失敗したときだけ。理由と最初の error の 1 行で、
        `globe simulate` の最後の行と同じ）。**モデルの誤り（未知のソルバ名、実行できないモデル）は例外にせず**
        `ok: False` と `diagnostics` で返る。

            res = ws.simulate("RC", stop_time=5, solver="auto", parameters={"R": 2.0})
            if res["ok"]:
                df = table(res)

        実行の間は GIL を離す（ほかの Python のスレッドは動き、Ctrl-C が効く）。
        `on_progress(p)` を渡すと、進捗を `Progress` の dict で呼ぶ。`True`（真の値）を返すと止まり、
        `termination` が `cancelled` で途中までの結果と情報 `GM-I11004` が返る。コールバックが例外を出したらそれを
        そのまま投げ直す。Ctrl-C は `KeyboardInterrupt`（S129、ADR-0033）。**コールバックの中から同じ `Workspace` は呼べない**
        （`RuntimeError: Already borrowed`）。止めたあとも同じ `Workspace` を使える。

        **既定のソルバ（`bdf`・`auto`・`dopri45`）では `on_progress` は積分の中に届かない**（ADR-0033 の既知の制約。
        Rumoca に口が無い）。呼ばれるのは段（lowering・preflight・build・initialization・integration）の変わり目の
        5 回ほどで `time` は `None`。積分の段が始まってから `True` を返しても（Ctrl-C も）積分は止まらず最後まで回る
        （積分より前の段で返せば次の段に入る前に止まるが、その結果は `ok: False` で時系列が無い）。
        積分の時刻が付き、途中で止められるのは session の道のソルバ（`esdirk34` など。`solvers()` で確かめる）で、
        t は 100 ms に 1 回まで。長い実行を止めたいなら `max_wall_seconds` を使う。
        """
        return json.loads(self._ws.simulate(model, _settings_json(settings), _callback(on_progress)))

    def simulate_columns(
        self,
        model: str,
        *,
        on_progress: Callable[[Progress], Any] | None = None,
        **settings: Unpack[Settings],
    ) -> dict[str, Any]:
        """`simulate` と同じ（設定・誤り・`on_progress` も同じ）だが、CSV を作らず `columns` に列を添える（S085）。

        `columns` は名前 → `polars.Series`（`time` が先頭）。`csv` は `None`。10 万点級の結果は `simulate` + `table` より速い。
        `defaults` は既定で描く信号の名前（最大 3 本。状態、足りなければ出力。UI のプロットと同じ。S190）。

            res = ws.simulate_columns("RC", stop_time=5)
            table(res)["time"]
        """
        response, names, columns = self._ws.simulate_columns(model, _settings_json(settings), _callback(on_progress))
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
        runtime: Literal["p5a", "p5b"] = "p5a",
        runtime_lib: str | os.PathLike[str] | None = None,
    ) -> Path:
        """FMU を `out_dir/<名前>-fmi2.fmu`（3.0 は `-fmi3`。と展開したディレクトリ）に書き、`.fmu` のパスを返す（S118。`globe fmu export` と同じ）。

        `fmi_version` は 2 か 3。`build=True` で共有ライブラリも作る（`fmi_headers`＝`fmi2Functions.h` などのある
        ディレクトリが要る。`targets` が空なら host を `cc` で、あれば `zig cc -target`）。
        未知のキーは `TypeError`、書き出せなければ（版が 2・3 でない、モデルの誤りなど）`ExportError`（`ValueError` でもある）。
        `targets` は 1 つなら文字列でもよい。

        `runtime="p5b"`（S239）は自前の実行時（`globe-mode-fmu`）の共有ライブラリを入れた FMU を `out_dir/<名前>-p5b-fmi2.fmu` に書く
        （イベント・MSL のモデルも載る。host の OS 用だけ）。共有ライブラリは `runtime_lib` で渡すか、省略すると globe のソースの
        作業ツリーの中で `cargo build --release -p globe-mode-fmu-cdylib` を呼んで作る（cargo が無い・作業ツリーの外なら `ExportError`）。
        モデルは 1 つのファイルか、開いたライブラリの中のクラス。`build`・`targets`・`fmi_headers` とは組み合わせられない（`ExportError`）。

            ws.export_fmu("RC", "out", fmi_version=3)
            ws.export_fmu("RC", "out", runtime="p5b", runtime_lib="target/release/libglobe_mode_fmu_cdylib.so")
        """
        targets = [targets] if isinstance(targets, str) else list(targets)
        options = dict(fmi_version=fmi_version, build=build, targets=targets, fmi_headers=None if fmi_headers is None else os.fspath(fmi_headers),
                       runtime=runtime, runtime_lib=None if runtime_lib is None else os.fspath(runtime_lib))
        return Path(self._ws.export_fmu(model, out_dir, json.dumps(options)))


def table(response: dict[str, Any]) -> pl.DataFrame:
    """`Workspace.simulate_columns`（列）か `simulate`（CSV）の結果を表（`time` 列と変数の列）にする。

    実行できなかった結果（`ok: False` で時系列が無い）なら `ResultError`（`ValueError` でもある。本文は `refused` の理由）。"""
    if response.get("columns"):
        return pl.DataFrame(list(response["columns"].values()))
    if response.get("csv") is None:
        raise ResultError(response.get("refused") or "結果に時系列が無い（ok を確かめてから table に渡す）")
    return pl.read_csv(io.StringIO(response["csv"]))


def read_results(path: str | os.PathLike[str]) -> dict[str, Any]:
    """結果のファイルを読む（S219。`.mat` は MAT v4＝OpenModelica・Dymola・`globe simulate -o x.mat` の結果、それ以外は CSV）。

    `simulate_columns` と同じ形の dict を返す: `ok`（True）・`columns`（名前 → `polars.Series`、`time` が先頭）・`csv`（None）。
    ほかに `parameters`（MAT の `data_1`。`name`・`value`・`unit`・`description` の dict のリスト。CSV では空）。
    `table(res)` で表になる。読めなければ（無い・MAT でも CSV でもない・時刻が崩れている）`ResultError`。

        table(read_results("res.mat"))
    """
    parameters, names, columns = _native.read_results(path)
    # shortcut: memoryview の "d" は native-endian（Rust は little-endian で詰める）、big-endian の機械なら byteswap が要る
    cols = {n: pl.Series(n, memoryview(c).cast("d"), pl.Float64) for n, c in zip(names, columns)}
    return {"ok": True, "columns": cols, "csv": None, "parameters": json.loads(parameters)}


def fmu_info(path: str | os.PathLike[str]) -> dict[str, Any]:
    """FMU（`.fmu`）の要約（S229。`globe fmu info` と同じ情報。**ネイティブコードは実行しない**）。

    `model_name`・`fmi_version`・`interfaces`（`kind`＝ModelExchange / CoSimulation / ScheduledExecution、`model_identifier`、ME だけ
    `continuous_states` と `event_indicators`）・`default_experiment`（`start_time`・`stop_time` の文字列。無ければ `None`）・
    `variables`（`name`・`causality`・`ty`・`start`）。読めなければ `FmuError`。"""
    return json.loads(_native.fmu_info(path))


def _globe_bin() -> str:
    """FMU のワーカーになる `globe`。`GLOBE_BIN`、無ければ PATH の `globe`（wheel の中には実行ファイルが無い）。"""
    found = os.environ.get("GLOBE_BIN") or shutil.which("globe")
    if not found:
        raise FmuError("FMU を回すワーカー（CLI の `globe`）が見つからない。`globe` を PATH に置くか、環境変数 GLOBE_BIN にパスを入れる")
    return found


def run_fmu(
    path: str | os.PathLike[str],
    *,
    kind: Literal["cs", "me"] | None = None,
    start_time: float | None = None,
    stop_time: float | None = None,
    step: float | None = None,
    outputs: Iterable[str] | None = None,
    parameters: Mapping[str, float] | None = None,
    timeout: float | None = None,
    on_progress: Callable[[float], Any] | None = None,
) -> dict[str, Any]:
    """FMU を回す（S229。`globe fmu run` と同じ。FMI Import、ADR-0026）。`read_results` と同じ形の dict を返す:
    `ok`・`columns`（名前 → `polars.Series`、`time` が先頭）・`csv`（None）・`me`（Model Exchange だけ。`steps`・`state_events` など。CS は None）。
    `table(res)` で表になる。

    **FMU のネイティブコードを実行する**。ただし別プロセス（ワーカー）で動くので、FMU が落ちても Python は落ちず `FmuError` になる
    （ディスク・ネットワークは隔離しない。信頼できる FMU だけを回すこと）。ワーカーは `globe`（`GLOBE_BIN` か PATH。見つからなければ `FmuError`）。

    `kind` は `"cs"`（FMU の中の積分器）か `"me"`（GlobeMode の積分器。事象あり）。省略は CS を持てば CS。`start_time`・`stop_time`・`step` の
    省略は FMU の DefaultExperiment、`outputs` の省略は causality が output の Real すべて、`parameters` は初期化の前に与える Real（名前 → 値）。
    `timeout`（秒）を超えるとワーカーを kill して `FmuError`。`on_progress(t)` は FMU の時刻を 0.25 秒以上おきに受け、真の値を返すと止める（`FmuError`）。
    Ctrl-C と `on_progress` は進捗が出たときだけ見る（進捗の出ない FMU は `timeout` で止める）。

        res = run_fmu("Dahlquist.fmu", stop_time=1, outputs=["x"])
        table(res)
    """
    if kind not in (None, "cs", "me"):
        raise TypeError(f"kind は 'cs' か 'me' のはずが {kind!r}")
    request = dict(kind=kind, start_time=start_time, stop_time=stop_time, step=step, timeout=timeout)
    if outputs is not None:
        request["outputs"] = [outputs] if isinstance(outputs, str) else list(outputs)
    if parameters is not None:
        request["set"] = dict(parameters)
    me, names, columns = _native.run_fmu(_globe_bin(), path, json.dumps({k: v for k, v in request.items() if v is not None}), _callback(on_progress))
    # shortcut: memoryview の "d" は native-endian（Rust は little-endian で詰める）、big-endian の機械なら byteswap が要る
    cols = {n: pl.Series(n, memoryview(c).cast("d"), pl.Float64) for n, c in zip(names, columns)}
    return {"ok": True, "columns": cols, "csv": None, "me": json.loads(me)}


def solvers() -> list[dict[str, Any]]:
    """ソルバの登録表（`name`・別名・実装の有無など）。`simulate(solver=…)` の名前はここから選ぶ。"""
    return json.loads(_native.solvers())


def format(text: str, indent: int = 2, insert_origin: bool = True) -> str:
    """Modelica の本文を整形して返す（S111。`globe fmt` と同じ規則）。文法が壊れていれば `SourceError`（`ValueError` でもある）。

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

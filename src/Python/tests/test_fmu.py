"""`fmu_info` と `run_fmu`（S229、FMI Import）。Reference-FMUs の Dahlquist（`cargo xtask fetch-reference-fmus`。無ければ飛ばす）。
ワーカーは CLI の `globe`: 環境変数 GLOBE_BIN を試験のビルドに向ける（無ければ成功の試験は飛ばす）。"""

import os
import shutil
from pathlib import Path

import pytest

from globe_mode import FmuError, GlobeModeError, fmu_info, run_fmu, table

FMU = Path(__file__).parents[3] / "data/reference-fmus/3.0/Dahlquist.fmu"
need_fmu = pytest.mark.skipif(not FMU.is_file(), reason=f"{FMU} が無い（cargo xtask fetch-reference-fmus）")
need_globe = pytest.mark.skipif(not (os.environ.get("GLOBE_BIN") or shutil.which("globe")), reason="GLOBE_BIN も PATH の globe も無い")


@need_fmu
def test_fmu_info_reads_without_running_anything():
    info = fmu_info(FMU)
    assert info["model_name"] == "Dahlquist" and info["fmi_version"].startswith("3")
    assert "CoSimulation" in [i["kind"] for i in info["interfaces"]]
    assert {"name": "k", "causality": "Parameter", "ty": "Float64", "start": "1"}.items() <= next(v for v in info["variables"] if v["name"] == "k").items()
    assert info["default_experiment"].keys() == {"start_time", "stop_time"}


def test_fmu_info_and_run_errors_are_fmu_errors(tmp_path, monkeypatch):
    assert issubclass(FmuError, GlobeModeError) and issubclass(FmuError, RuntimeError)
    with pytest.raises(FmuError, match="no_such"):
        fmu_info(tmp_path / "no_such.fmu")
    # ワーカー（globe）が見つからなければ、置き方を言う
    monkeypatch.delenv("GLOBE_BIN", raising=False)
    monkeypatch.setenv("PATH", str(tmp_path))
    with pytest.raises(FmuError, match="GLOBE_BIN"):
        run_fmu(FMU)
    with pytest.raises(TypeError, match="kind"):
        run_fmu(FMU, kind="xx")  # type: ignore[arg-type]


@need_fmu
@need_globe
def test_run_fmu_gives_columns_like_read_results():
    res = run_fmu(FMU, kind="cs", stop_time=1, parameters={"k": 2})
    assert res["ok"] and res["csv"] is None and res["me"] is None
    df = table(res)
    assert df.columns == ["time", "x"] and df.height == 501
    assert abs(df["x"][-1] - 0.8**10) < 1e-12  # 内部刻み 0.1 の前進 Euler
    me = run_fmu(FMU, kind="me", stop_time=1, outputs="x")
    assert me["me"]["steps"] > 0 and abs(table(me)["x"][-1] - 2.718281828459045**-1) < 1e-5


@need_fmu
@need_globe
def test_run_fmu_failures_progress_and_stop():
    with pytest.raises(FmuError, match="nope"):
        run_fmu(FMU, outputs=["nope"])
    with pytest.raises(FmuError, match="timeout"):
        run_fmu(FMU, timeout=-1)
    with pytest.raises(FmuError, match="No such file"):
        run_fmu(FMU.with_name("no_such.fmu"))
    # on_progress は引数に時刻を受ける呼べるもの。真を返せば止まる（進捗が出るほど長い FMU は無いので、呼ばれたときの形だけ）
    seen: list[float] = []
    run_fmu(FMU, on_progress=lambda t: seen.append(t) or False)
    assert all(isinstance(t, float) for t in seen)
    with pytest.raises(TypeError):
        run_fmu(FMU, on_progress=3)  # type: ignore[arg-type]

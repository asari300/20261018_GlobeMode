"""`fmu_info` と `run_fmu`（S229、FMI Import）。Reference-FMUs の Dahlquist（`cargo xtask fetch-reference-fmus`。無ければ飛ばす）。
ワーカーは CLI の `globe`: 環境変数 GLOBE_BIN を試験のビルドに向ける（無ければ成功の試験は飛ばす）。"""

import json
import os
import shutil
import zipfile
from pathlib import Path

import pytest

from globe_mode import FmuError, GlobeModeError, cosim, fmu_info, run_fmu, table

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


REF = FMU.parents[1]
need_ref = pytest.mark.skipif(not (REF / "2.0/Dahlquist.fmu").is_file(), reason="Reference-FMUs 2.0 が無い（cargo xtask fetch-reference-fmus）")
SSD = """<?xml version="1.0" encoding="UTF-8"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon" name="demo" version="2.0">
  <ssd:System name="top"><ssd:Elements>
    <ssd:Component name="a" source="resources/a.fmu"/><ssd:Component name="b" source="resources/b.fmu"/>
  </ssd:Elements><ssd:Connections>
    <ssd:Connection startElement="a" startConnector="x" endElement="b" endConnector="Float64_continuous_input"/>
  </ssd:Connections></ssd:System>
  <ssd:DefaultExperiment startTime="0" stopTime="1"/>
</ssd:SystemStructureDescription>"""


@need_ref
@need_globe
def test_cosim_dict_json_and_ssp_give_the_same_columns(tmp_path):
    """Dahlquist（2.0）→ Feedthrough（3.0）。Jacobi 型なので b の出力は a.x の 1 刻み遅れにちょうど等しい。"""
    a, b = REF / "2.0/Dahlquist.fmu", REF / "3.0/Feedthrough.fmu"
    spec = {"fmus": {"a": a, "b": b}, "connections": [["a.x", "b.Float64_continuous_input"]], "stop_time": 1, "step": 0.1}
    res = cosim(spec)
    assert res["ok"] and res["csv"] is None and res["me"] is None
    df = table(res)
    assert df.height == 11 and df["a.x"][0] == 1.0
    assert df["b.Float64_continuous_output"].to_list()[1:] == df["a.x"].to_list()[:-1]
    # JSON: FMU のパスは JSON のあるディレクトリから
    (tmp_path / "s.json").write_text(json.dumps({**spec, "fmus": {"a": os.path.relpath(a, tmp_path), "b": os.path.relpath(b, tmp_path)}}))
    assert table(cosim(tmp_path / "s.json")).equals(df)
    # SSP: step は引数で（終了時刻は DefaultExperiment）
    with zipfile.ZipFile(tmp_path / "s.ssp", "w") as z:
        z.writestr("SystemStructure.ssd", SSD)
        z.write(a, "resources/a.fmu")
        z.write(b, "resources/b.fmu")
    assert table(cosim(tmp_path / "s.ssp", step=0.1)).equals(df)
    with pytest.raises(FmuError, match="入力でない"):
        cosim({**spec, "connections": [["a.x", "a.k"]]})

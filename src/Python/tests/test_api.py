"""参照シナリオ: ライブラリを開く → 検査 → connect グラフ → 実行 → 結果を表で読む。"""

import math
from pathlib import Path

import pytest

from globe_mode import Workspace, format, lint, table

MODELICA = Path(__file__).parents[2] / "Modelica"


def test_reference_scenario():
    ws = Workspace(libraries=[MODELICA])  # グラフもライブラリのモデルを引ける（S079）

    check = ws.check("GlobeModeValidation.Simulation.Decay")
    assert check["passed"]
    assert not [d for d in check["diagnostics"] if d["severity"] == "error"]

    graph = ws.graph("GlobeModeValidation.Connect.RealCircuit")
    assert graph["nodes"] and graph["edges"]

    res = ws.simulate("GlobeModeValidation.Simulation.Decay")  # experiment: StopTime = 1
    assert res["ok"]
    df = table(res)
    assert df["time"][-1] == 1.0
    assert math.isclose(df["x"][-1], math.exp(-1), rel_tol=1e-4)

    # S111: 参照シナリオの最後（format）と lint
    formatted = format((MODELICA / "GlobeModeValidation/Simulation/Decay.mo").read_text())
    assert "der(x) = -k * x;" in formatted
    with pytest.raises(ValueError):
        format("model Decay")
    assert isinstance(lint("model Decay Real x; end Decay;"), list)


def test_library_is_readable():
    assert Workspace(libraries=[MODELICA]).check("GlobeModeValidation.Simulation.Decay")["passed"]


def test_export_fmu(tmp_path):
    """S118: Decay の FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある。"""
    ws = Workspace(libraries=[MODELICA])
    fmu = ws.export_fmu("GlobeModeValidation.Simulation.Decay", tmp_path)
    assert fmu == tmp_path / "GlobeModeValidation_Simulation_Decay.fmu" and fmu.is_file()
    assert (tmp_path / "GlobeModeValidation_Simulation_Decay/modelDescription.xml").is_file()
    with pytest.raises(ValueError):
        ws.export_fmu("GlobeModeValidation.Simulation.Decay", tmp_path, fmi_version=4)

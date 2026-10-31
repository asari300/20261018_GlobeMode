"""参照シナリオ: ライブラリを開く → 検査 → connect グラフ → 実行 → 結果を表で読む。"""

import math
from pathlib import Path

from globe_mode import Workspace, table

MODELICA = Path(__file__).parents[2] / "Modelica"


def test_reference_scenario():
    ws = Workspace()
    assert ws.load_dir(MODELICA) > 0  # グラフと実行は作業中のファイルを引く（libraries= はライブラリとして読む）

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


def test_library_is_readable():
    assert Workspace(libraries=[MODELICA]).check("GlobeModeValidation.Simulation.Decay")["passed"]

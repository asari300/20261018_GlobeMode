"""`simulate_columns` の列が `simulate` の CSV と同じ（S085）。"""

from pathlib import Path

from globe_mode import Workspace, table

MODELICA = Path(__file__).parents[2] / "Modelica"


def test_columns_match_csv():
    ws = Workspace()
    ws.load_dir(MODELICA)
    model = "GlobeModeValidation.Simulation.Decay"
    cols = ws.simulate_columns(model)
    assert cols["ok"] and cols["csv"] is None
    assert list(cols["columns"]) == ["time", "x"]
    csv, df = table(ws.simulate(model)), table(cols)
    assert df.columns == csv.columns and df.height == csv.height == 501
    assert df.equals(csv)  # CSV の f64 は往復で同じ値になる書き方なので、完全に一致する


def test_columns_defaults_are_the_states():
    ws = Workspace()
    ws.load_dir(MODELICA)
    res = ws.simulate_columns("GlobeModeValidation.Simulation.Decay")
    assert res["defaults"] == ["x"]  # 既定で描く信号。応答の列の名前（time を除く）の部分集合
    assert set(res["defaults"]) <= set(res["columns"])

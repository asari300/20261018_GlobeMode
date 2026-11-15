"""`simulate_columns` の列が `simulate` の CSV と同じ（S085）。"""

from pathlib import Path

import polars as pl

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


def test_table_reads_inf_and_nan_as_numbers():
    # polars は Inf を数として読まず列が String になる（#92）。time のような整数の列も Float64 に揃う
    df = table({"csv": '"time","x"\n0,1\n1,Inf\n2,-Inf\n3,NaN\n'})
    assert df.dtypes == [pl.Float64, pl.Float64]
    assert df["x"][1] == float("inf") and df["x"][2] == float("-inf")


def test_simulate_many_compares_solvers_and_notes_the_ignored_max_order():
    """S322: 複数のソルバで回して比べる。逐次と並列は同じ列、最大次数を持たないソルバには注記、閾値で合否。"""
    ws = Workspace()
    ws.load_dir(MODELICA)
    model = "GlobeModeValidation.Simulation.Decay"
    solvers = ["bdf", "esdirk34", "tsit45"]
    seq = ws.simulate_many(model, solvers, tolerance=1e-9, max_order=3, compare=4)
    par = ws.simulate_many(model, solvers, tolerance=1e-9, max_order=3, compare=4, parallel=True)
    assert seq["ok"] and seq["mode"] == "sequential" and seq["comparison"]["passed"]
    assert [r["solver"] for r in seq["runs"]] == solvers
    assert all(len(r["notes"]) == 1 and "最大次数" in r["notes"][0] for r in seq["runs"])  # 3 つとも可変刻み
    for a, b in zip(seq["runs"], par["runs"]):
        assert a["columns"]["x"].equals(b["columns"]["x"])
    bad = ws.simulate_many(model, ["tsit45", "euler"], compare=6)
    assert not bad["comparison"]["passed"] and bad["comparison"]["pairs"][0]["solver"] == "euler"

"""`read_results` が結果の MAT v4（omc の実物）と CSV を読む（S219）。"""

from pathlib import Path

import pytest

from globe_mode import ResultError, Workspace, read_results, table

ROOT = Path(__file__).parents[2]
OMC_MAT = ROOT / "Rust/mod/globe-mode-sim/tests/data/omc_decay_res.mat"


def test_reads_the_mat_omc_wrote():
    res = read_results(OMC_MAT)
    assert res["ok"] and res["csv"] is None
    assert list(res["columns"])[:2] == ["time", "x"]
    assert table(res)["time"].to_list() == [0.0, 0.25, 0.5, 0.75, 1.0, 1.0]
    assert [(p["name"], p["value"]) for p in res["parameters"]] == [("k", 1.0)]


def test_csv_round_trips_a_simulation(tmp_path):
    ws = Workspace()
    ws.load_dir(ROOT / "Modelica")
    run = ws.simulate("GlobeModeValidation.Simulation.Decay")
    (tmp_path / "decay.csv").write_text(run["csv"])
    res = read_results(tmp_path / "decay.csv")
    assert res["parameters"] == []
    assert table(res).equals(table(run))


def test_unreadable_files_raise_result_error(tmp_path):
    (tmp_path / "bad.mat").write_bytes(b"not a mat file")
    for name in ("bad.mat", "missing.csv"):
        with pytest.raises(ResultError, match=name):
            read_results(tmp_path / name)

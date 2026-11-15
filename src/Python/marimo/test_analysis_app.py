"""analysis_app.py（解析の見本。S319）を全セル実行して、結果の値を確かめる。コシミュレーションの節は FMU と globe があるときだけ。"""

import os
import shutil
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
from analysis_app import app  # noqa: E402

ROOT = Path(__file__).parents[3]
HAVE_COSIM = (ROOT / "data/reference-fmus/3.0/Feedthrough.fmu").is_file() and (os.environ.get("GLOBE_BIN") or shutil.which("globe"))


def test_notebook_runs_and_shows_the_expected_numbers():
    _, defs = app.run()
    lin = defs["lin"]
    assert [float(v) for r in lin["A"] for v in r] == pytest.approx([-2, 1, 1, -2], abs=1e-5)
    assert [float(v) for r in lin["D"] for v in r] == pytest.approx([3, 0], abs=1e-5)
    assert lin["stability"] == "stable"
    assert defs["st"]["index"]["index"] == 3
    assert [b["size"] for b in defs["blocks"] if b["algebraic_loop"]] == [2]
    assert defs["compare"]["frontend"].to_list() == ["rumoca", "native"]


@pytest.mark.skipif(not HAVE_COSIM, reason="Reference-FMUs か globe が無い")
def test_cosim_gauss_seidel_removes_the_one_step_lag():
    _, defs = app.run()
    assert defs["lag"]["gauss-seidel"] < 1e-9 < defs["lag"]["jacobi"]

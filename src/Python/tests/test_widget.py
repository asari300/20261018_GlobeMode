"""SimulationPlot が結果を traitlets に入れる（ブラウザは使わない。S078）。"""

import math
from pathlib import Path

import pytest

from globe_mode import Workspace
from globe_mode.widget import SimulationPlot

MODELICA = Path(__file__).parents[2] / "Modelica"


def test_simulation_plot_traits():
    ws = Workspace()
    ws.load_dir(MODELICA)
    plot = SimulationPlot(ws.simulate("GlobeModeValidation.Simulation.Decay"))
    assert list(plot.series) == ["x"] and plot.visible == ["x"]
    assert len(plot.time) == len(plot.series["x"]) == 501
    assert plot.time[0] == 0.0 and plot.time[-1] == 1.0
    assert math.isclose(plot.series["x"][-1], math.exp(-1), rel_tol=1e-4)
    assert "export default" in plot._esm


def test_refused_result_raises():
    with pytest.raises(ValueError, match="no model"):
        SimulationPlot({"csv": None, "refused": "no model"})

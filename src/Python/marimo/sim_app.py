# GlobeMode の実行と折れ線（marimo + anywidget）。起動: uv run marimo run src/Python/marimo/sim_app.py
# 計算は Python パッケージ globe_mode（wheel、Rust）に任せ、SimulationPlot で描く（S078）。

import marimo

__generated_with = "0.25.1"
app = marimo.App(width="medium", app_title="GlobeMode 実行と折れ線")


@app.cell
def _():
    from pathlib import Path

    import marimo as mo

    from globe_mode import Workspace
    from globe_mode.widget import SimulationPlot

    return Path, SimulationPlot, Workspace, mo


@app.cell
def _(Path, Workspace):
    ws = Workspace()
    ws.load_dir(Path(__file__).parents[2] / "Modelica")
    result = ws.simulate("GlobeModeValidation.Simulation.Decay")
    return (result,)


@app.cell
def _(SimulationPlot, mo, result):
    plot = mo.ui.anywidget(SimulationPlot(result))
    plot
    return


if __name__ == "__main__":
    app.run()

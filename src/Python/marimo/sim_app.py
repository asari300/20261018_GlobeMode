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
def _(Path, mo):
    # 既定はリポジトリの src/Modelica の Decay。MSL なら data/msl と Modelica.Electrical.Analog.Examples.ChuaCircuit（S183）
    libs = mo.ui.text(value=str(Path(__file__).parents[2] / "Modelica"), label="ライブラリのディレクトリ（-L）", full_width=True)
    model = mo.ui.text(value="GlobeModeValidation.Simulation.Decay", label="モデル（完全修飾名）", full_width=True)
    mo.vstack([libs, model])
    return libs, model


@app.cell
def _(Workspace, libs, mo, model):
    ws = Workspace([libs.value])
    result = ws.simulate_columns(model.value)
    why = result["refused"] or "; ".join(d["message"] for d in result["diagnostics"][:1])
    mo.stop(not result["ok"], mo.md(f"実行できない: {why}"))
    return (result,)


@app.cell
def _(SimulationPlot, mo, result):
    plot = mo.ui.anywidget(SimulationPlot(result))
    plot
    return


if __name__ == "__main__":
    app.run()

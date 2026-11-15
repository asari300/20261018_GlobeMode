# GlobeMode の解析の機能の見本（marimo）。起動: uv run marimo run src/Python/marimo/analysis_app.py
# 線形化・BLT と段・コシミュレーション・自前の検査（native）を Python パッケージ globe_mode で 1 本に並べる（S319）。
# 依存は既存のもの（globe_mode・polars・plotly）だけ。numpy があれば行列が ndarray になるが、無くても動く。

import marimo

__generated_with = "0.25.1"
app = marimo.App(width="medium", app_title="GlobeMode 解析の見本")


@app.cell
def _():
    import os
    from pathlib import Path

    import marimo as mo
    import plotly.graph_objects as go
    import polars as pl

    from globe_mode import FmuError, Workspace, cosim, table

    root = Path(__file__).parents[3]
    return FmuError, Path, Workspace, cosim, go, mo, os, pl, root, table


@app.cell
def _(mo, root):
    mo.md("""
    # GlobeMode 解析の見本

    1. 線形化（`Workspace.linearize`）
    2. BLT と変換の段（`Workspace.blt`・`Workspace.stages`）
    3. コシミュレーション（`cosim`。Jacobi と Gauss–Seidel の差）
    4. 検査の frontend を並べる（`Workspace.check(model, frontend="native")`）

    計算は Rust 側（wheel）で、このノートブックは結果を表と図にするだけ。
    """)
    libs = mo.ui.text(value=str(root / "src" / "Modelica"), label="ライブラリのディレクトリ（-L）", full_width=True)
    libs
    return (libs,)


@app.cell
def _(Workspace, libs):
    ws = Workspace([libs.value])
    return (ws,)


@app.cell
def _(mo):
    mo.md("""
    ## 1. 線形化

    `GlobeModeValidation.Simulation.LinearSystem` は入力 1・出力 2・状態 2 で、`A = [-2 1; 1 -2]`・`B = [1; 2]`・`C = [1 2; 1 -1]`・`D = [3; 0]`
    が手で分かる。数値の中心差分で求めた行列がそれに合う。
    """)
    return


@app.cell
def _(mo, pl, ws):
    lin = ws.linearize("GlobeModeValidation.Simulation.LinearSystem")

    def mat(name, rows, cols, row_names):
        # numpy の ndarray でも list のリストでも同じに読む
        df = pl.DataFrame([[round(float(v), 6) for v in row] for row in rows], schema=cols or ["-"], orient="row")
        return mo.vstack([mo.md(f"**{name}**"), df.insert_column(0, pl.Series("", row_names))])

    eig = [complex(e["re"], e["im"]) for e in lin["eigenvalues"]]
    mo.vstack([
        mo.hstack([mat("A", lin["A"], lin["x"], lin["x"]), mat("B", lin["B"], lin["u"], lin["x"])]),
        mo.hstack([mat("C", lin["C"], lin["x"], lin["y"]), mat("D", lin["D"], lin["u"], lin["y"])]),
        mo.md(f"固有値: {', '.join(f'{e.real:.4f}{e.imag:+.4f}j' for e in eig)} → **{lin['stability']}**"),
    ])
    return eig, lin


@app.cell
def _(mo):
    mo.md("""
    ## 2. BLT と変換の段

    `Index.Pendulum` は index 3 の DAE で、index の低減で方程式と未知数が増える。`Loops.LinearLoop` は大きさ 2 の代数ループを持つ。
    """)
    return


@app.cell
def _(mo, pl, ws):
    st = ws.stages("GlobeModeValidation.Index.Pendulum")
    blocks = ws.blt("GlobeModeValidation.Loops.LinearLoop")
    idx = st["index"]
    mo.vstack([
        mo.md(
            f"Pendulum: index **{idx['index']}**、足した方程式 {idx['added_equations']}・足した未知数 {idx['added_unknowns']}"
            f"、段 = {', '.join(k for k, v in st.items() if v is not None)}"
        ),
        mo.md("LinearLoop の BLT（解く順）"),
        pl.DataFrame(
            {
                "size": [b["size"] for b in blocks],
                "algebraic_loop": [b["algebraic_loop"] for b in blocks],
                "nonlinear": [b["nonlinear"] for b in blocks],
                "variables": [", ".join(b["variables"]) for b in blocks],
            }
        ),
    ])
    return blocks, st


@app.cell
def _(mo, root):
    mo.md("""
    ## 3. コシミュレーション

    Reference-FMUs の Dahlquist（`der(x) = -k x`）の出力 `x` を Feedthrough の入力につなぐ。Feedthrough は入力をそのまま出力に返すので、
    Jacobi（刻みの頭の出力を渡す）は 1 刻み遅れ、Gauss–Seidel（結線の元から先へ 1 つずつ進める）は遅れない。
    `data/reference-fmus` は `cargo xtask fetch-reference-fmus` で取得する。FMU は別プロセスの `globe`（PATH か環境変数 `GLOBE_BIN`）で回す。
    """)
    fmus = mo.ui.text(value=str(root / "data" / "reference-fmus"), label="Reference-FMUs のディレクトリ", full_width=True)
    step = mo.ui.slider(0.01, 0.2, step=0.01, value=0.1, label="通信刻み")
    mo.vstack([fmus, step])
    return fmus, step


@app.cell
def _(FmuError, Path, cosim, fmus, mo, os, pl, step, table):
    _dir = Path(fmus.value)
    _a, _b = _dir / "2.0" / "Dahlquist.fmu", _dir / "3.0" / "Feedthrough.fmu"
    mo.stop(
        not (_a.is_file() and _b.is_file()),
        mo.md(f"`{_a}` と `{_b}` が無いので、この節は飛ばす（`cargo xtask fetch-reference-fmus` で取得する）。"),
    )
    cosims = {}
    try:
        for algorithm in ("jacobi", "gauss-seidel"):
            spec = {
                "fmus": {"a": _a, "b": _b},
                "connections": [["a.x", "b.Float64_continuous_input"]],
                "stop_time": 1,
                "step": step.value,
                "algorithm": algorithm,
            }
            cosims[algorithm] = table(cosim(spec))
    except FmuError as e:
        mo.stop(True, mo.md(f"コシミュレーションを回せない: `{e}`（`globe` を PATH か `GLOBE_BIN` に置く）"))
    _out = "b.Float64_continuous_output"
    lag = {k: float((df["a.x"] - df[_out]).slice(1).abs().max()) for k, df in cosims.items()}
    mo.md(f"`|a.x - b.out|` の最大（最初の行は初期化直後で Feedthrough の出力がまだ 0 なので除く）: " + "、".join(f"{k} = {v:.4g}" for k, v in lag.items()))
    return algorithm, cosims, lag, spec


@app.cell
def _(cosims, go, mo):
    fig = go.Figure()
    for _alg, _df in cosims.items():
        fig.add_scatter(x=_df["time"].to_list(), y=_df["b.Float64_continuous_output"].to_list(), mode="lines+markers", name=f"b.out（{_alg}）")
    fig.add_scatter(x=cosims["jacobi"]["time"].to_list(), y=cosims["jacobi"]["a.x"].to_list(), mode="lines", name="a.x", line=dict(dash="dot"))
    fig.update_layout(xaxis_title="time [s]", yaxis_title="値", height=360, margin=dict(t=20))
    mo.ui.plotly(fig)
    return (fig,)


@app.cell
def _(mo):
    mo.md("""
    ## 4. 検査の frontend を並べる

    `check(model)` は Rumoca（S0〜S10）、`check(model, frontend="native")` は自前の段（S0〜S6。未対応の形は `GM-U0006`）。
    """)
    model = mo.ui.dropdown(
        [f"GlobeModeValidation.{m}" for m in ("Simulation.LinearSystem", "Simulation.Decay", "Index.Pendulum", "Loops.LinearLoop")],
        value="GlobeModeValidation.Simulation.LinearSystem",
        label="モデル",
    )
    model
    return (model,)


@app.cell
def _(mo, model, pl, ws):
    _rows = []
    for _fe in ("rumoca", "native"):
        _r = ws.check(model.value, frontend=_fe)
        _rows.append(
            {
                "frontend": _fe,
                "passed": _r["passed"],
                "failed_stage": str(_r["failed_stage"]),
                "codes": ", ".join(sorted({d["code"] for d in _r["diagnostics"]})),
                "summary": _r["summary"],
            }
        )
    compare = pl.DataFrame(_rows)
    mo.vstack([compare, mo.md("`passed` が違うとき、`codes` に `GM-U0006` があれば native が未対応の形として断っただけ。")])
    return (compare,)


if __name__ == "__main__":
    app.run()

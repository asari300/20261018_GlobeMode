"""`Workspace.simulate` の結果を折れ線で描く anywidget（S078）。marimo・Jupyter で使う。

    SimulationPlot(ws.simulate("GlobeModeValidation.Simulation.Decay"))

JS は依存なしの <canvas>（CDN を使わず、オフラインで開ける）。結果の整形は `plot_data` に置く。
"""

try:
    import anywidget
    import traitlets
except ImportError as e:
    raise ImportError('globe_mode.widget には extra が要る: pip install "globe-mode[widget]"') from e

from . import table


def plot_data(response: dict) -> dict:
    """結果を `{"time": [...], "series": {名前: [...]}}` にする。実行できなかったら ValueError（`table` と同じ）。"""
    df = table(response)
    return {"time": df["time"].to_list(), "series": {c: df[c].to_list() for c in df.columns if c != "time"}}


_ESM = r"""
const COLORS = ["#2a6fdb", "#d9480f", "#2b8a3e", "#ae3ec9", "#e67700", "#0c8599", "#c2255c", "#5c940d"];

function render({ model, el }) {
  const boxes = document.createElement("div");
  const canvas = document.createElement("canvas");
  canvas.width = 640;
  canvas.height = 320;
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", "シミュレーション結果の折れ線");
  el.append(boxes, canvas);

  function draw() {
    const t = model.get("time");
    const series = model.get("series");
    const names = Object.keys(series);
    const shown = names.filter((n) => model.get("visible").includes(n));
    const ctx = canvas.getContext("2d");
    const W = canvas.width, H = canvas.height, P = 40;
    ctx.clearRect(0, 0, W, H);
    ctx.strokeStyle = "#888";
    ctx.strokeRect(P, 8, W - P - 8, H - P - 8);
    if (t.length < 2 || shown.length === 0) return;
    const ys = shown.flatMap((n) => series[n]).filter(Number.isFinite);
    if (ys.length === 0) return;
    // Math.min(...ys) は引数が約 15 万個で RangeError になる（#91）
    let lo = Infinity, hi = -Infinity;
    for (const v of ys) { if (v < lo) lo = v; if (v > hi) hi = v; }
    if (lo === hi) { lo -= 1; hi += 1; }
    const t0 = t[0], t1 = t[t.length - 1];
    const x = (v) => P + ((v - t0) / (t1 - t0 || 1)) * (W - P - 8);
    const y = (v) => H - P - ((v - lo) / (hi - lo)) * (H - P - 16);
    ctx.fillStyle = "#666";
    ctx.font = "11px sans-serif";
    ctx.fillText(hi.toPrecision(4), 2, 16);
    ctx.fillText(lo.toPrecision(4), 2, H - P);
    ctx.fillText(String(t0), P, H - P + 14);
    ctx.fillText(String(t1), W - 40, H - P + 14);
    for (const n of shown) {
      ctx.strokeStyle = COLORS[names.indexOf(n) % COLORS.length];
      ctx.beginPath();
      series[n].forEach((v, i) => (i ? ctx.lineTo(x(t[i]), y(v)) : ctx.moveTo(x(t[i]), y(v))));
      ctx.stroke();
    }
  }

  function boxesFor() {
    boxes.replaceChildren();
    Object.keys(model.get("series")).forEach((n, i) => {
      const label = document.createElement("label");
      const box = document.createElement("input");
      box.type = "checkbox";
      box.checked = model.get("visible").includes(n);
      box.onchange = () => {
        const rest = model.get("visible").filter((v) => v !== n);
        model.set("visible", box.checked ? [...rest, n] : rest);
        model.save_changes();
      };
      label.style.color = COLORS[i % COLORS.length];
      label.style.marginRight = "1em";
      label.append(box, " " + n);
      boxes.append(label);
    });
  }

  boxesFor();
  draw();
  model.on("change:series", () => { boxesFor(); draw(); });
  model.on("change:time", draw);
  model.on("change:visible", draw);
}
export default { render };
"""


class SimulationPlot(anywidget.AnyWidget):
    """traitlets: `time`（時刻の列）、`series`（変数名 → 値の列）、`visible`（描く変数名。チェックボックスと同期）。"""

    _esm = _ESM
    time = traitlets.List(traitlets.Float()).tag(sync=True)
    series = traitlets.Dict().tag(sync=True)
    visible = traitlets.List(traitlets.Unicode()).tag(sync=True)

    def __init__(self, response: dict, **kwargs):
        data = plot_data(response)
        super().__init__(time=data["time"], series=data["series"], visible=list(data["series"]), **kwargs)

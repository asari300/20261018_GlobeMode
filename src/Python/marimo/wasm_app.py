# marimo の WASM（Pyodide）で、anywidget の _esm から globe-mode-wasm を直接呼ぶ試作（S102）。計算は Web Worker で回す（S104）。
# Pyodide 側は文面と設定を traitlets で渡すだけで、Rust を Python から呼ばない（globe_mode の wheel は Pyodide に入らない）。
# 書き出し: uv run marimo export html-wasm src/Python/marimo/wasm_app.py -o OUT --mode run
#   そのあと OUT/wasm/ に globe-mode-wasm/web の client.js・worker.js と、mise run wasm:build の wasm-pkg/web を pkg/ として置き、
#   OUT/msl/MSL.json に src/Svelte の pack-msl.mjs の出力を置く（S104）。
# 検査: uv run python src/Python/marimo/wasm_check.py（書き出し・配信・Chromium での x(1) とメモリ）

import marimo

__generated_with = "0.25.1"
app = marimo.App(width="medium", app_title="GlobeMode WASM 試作")


@app.cell
def _():
    import anywidget
    import marimo as mo
    import traitlets

    return anywidget, mo, traitlets


@app.cell
def _(anywidget, traitlets):
    _ESM = r"""
    async function render({ model, el }) {
      const out = document.createElement("pre");
      out.textContent = "globe-mode-wasm を読み込み中…";
      el.append(out);
      const url = (p) => new URL(p, document.baseURI).href; // _esm は data: の URL で読まれるので、ページを基準にする
      const wasm = url(model.get("wasm"));
      const name = model.get("model");
      let r;
      try {
        // S104: src/Svelte の WorkerBackend と同じ GlobeWorker（client.js）と worker.js で、計算を Web Worker に出す
        const { GlobeWorker } = await import(`${wasm}/client.js`);
        const w = new GlobeWorker(() => new Worker(`${wasm}/worker.js`, { type: "module" }), { pkg: `${wasm}/pkg` });
        let msl_ms;
        if (model.get("msl")) { // ブラウザ版（App.svelte）と同じく MSL.json を fetch して library に渡す。Pyodide を通さない
          out.textContent = "MSL を読み込み中…";
          const t = performance.now();
          const res = await fetch(url(model.get("msl")));
          if (!res.ok) throw new Error(`MSL.json を取得できない（${res.status}）`);
          await w.library(await res.text(), "MSL");
          msl_ms = performance.now() - t;
        }
        if (model.get("text")) await w.setFile(model.get("file"), model.get("text"));
        out.textContent = `${name} を検査・実行中…`;
        const c = await w.check(name);
        // メインのスレッドが固まらないこと: simulate の間、20 ms ごとの timer が走り続け、間隔の最大が小さい
        let last = performance.now(), max_gap_ms = 0, ticks = 0;
        const timer = setInterval(() => { const n = performance.now(); max_gap_ms = Math.max(max_gap_ms, n - last); last = n; ticks++; }, 20);
        const t = performance.now();
        let s;
        try {
          s = await w.simulate({ model: name, settings: model.get("settings"), no_csv: true });
        } finally {
          clearInterval(timer);
        }
        r = { passed: c.passed, ok: s.ok, final: s.final, version: await w.version(), wasm_memory: await w.memory(),
              msl_ms, sim_ms: performance.now() - t, ticks, max_gap_ms,
              errors: [...(c.diagnostics ?? []), ...(s.diagnostics ?? [])].filter((d) => d.severity === "error") };
      } catch (e) {
        r = { ok: false, error: String(e?.stack ?? e) };
      }
      model.set("result", r);
      model.save_changes();
      (window.__globe ??= {})[name] = r; // wasm_check.py が読む
      out.textContent = r.ok ? `${name}: ${JSON.stringify(r.final).slice(0, 120)}（check ${r.passed ? "通過" : "失敗"}、` +
                               `実行 ${(r.sim_ms / 1000).toFixed(1)} s、wasm の線形メモリ ${(r.wasm_memory / 2 ** 20).toFixed(0)} MiB）`
                             : `失敗: ${r.error ?? JSON.stringify(r.errors)}`;
    }
    export default { render };
    """


    class WasmSim(anywidget.AnyWidget):
        """入力: wasm（client.js・worker.js・pkg/ の置き場の URL）、msl（MSL.json の URL。空なら読まない）、
        file・text（.mo の名前と文面。空なら置かない）、model、settings。出力: result。"""

        _esm = _ESM
        wasm = traitlets.Unicode("wasm").tag(sync=True)
        msl = traitlets.Unicode().tag(sync=True)
        file = traitlets.Unicode("Decay.mo").tag(sync=True)
        text = traitlets.Unicode().tag(sync=True)
        model = traitlets.Unicode().tag(sync=True)
        settings = traitlets.Dict().tag(sync=True)
        result = traitlets.Dict().tag(sync=True)

    return (WasmSim,)


@app.cell
def _(WasmSim, mo):
    DECAY = """model Decay "1 次の減衰 der(x) = -k*x（解析解 x(1) = exp(-1)）"
      parameter Real k = 1;
      Real x(start = 1, fixed = true);
    equation
      der(x) = -k * x;
      annotation (experiment(StopTime = 1, Tolerance = 1e-8));
    end Decay;
    """
    sim = mo.ui.anywidget(WasmSim(text=DECAY, model="Decay", settings={"solver": "bdf"}))
    sim
    return


@app.cell
def _(WasmSim, mo):
    # S104: MSL（msl/MSL.json、src/Svelte の pack-msl.mjs が作るもの）を読み、MSL の例題を回す
    chua = mo.ui.anywidget(WasmSim(msl="msl/MSL.json", model="Modelica.Electrical.Analog.Examples.ChuaCircuit", settings={"solver": "bdf"}))
    chua
    return


if __name__ == "__main__":
    app.run()

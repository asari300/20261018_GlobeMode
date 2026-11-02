# marimo の WASM（Pyodide）で、anywidget の _esm から globe-mode-wasm を直接呼ぶ試作（S102）。
# Pyodide 側は文面と設定を traitlets で渡すだけで、Rust を Python から呼ばない（globe_mode の wheel は Pyodide に入らない）。
# 書き出し: uv run marimo export html-wasm src/Python/marimo/wasm_app.py -o OUT --mode run
#   そのあと mise run wasm:build の wasm-pkg/web の globe_mode_wasm.js・globe_mode_wasm_bg.wasm を OUT/pkg/ に置く。
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
      const pkg = new URL(model.get("pkg"), document.baseURI).href; // _esm は blob の URL で読まれるので、ページを基準にする
      let r;
      try {
        const gm = await import(`${pkg}/globe_mode_wasm.js`);
        await gm.default({ module_or_path: `${pkg}/globe_mode_wasm_bg.wasm` });
        const ws = new gm.Workspace();
        try {
          ws.set_file(model.get("file"), model.get("text"));
          const c = JSON.parse(ws.check(model.get("model")));
          const s = JSON.parse(ws.simulate(JSON.stringify({ model: model.get("model"), settings: model.get("settings") })));
          r = { passed: c.passed, ok: s.ok, final: s.final, version: gm.version(), wasm_memory: gm.memory_bytes(),
                errors: (s.diagnostics ?? []).filter((d) => d.severity === "error") };
        } finally {
          ws.free();
        }
      } catch (e) {
        r = { ok: false, error: String(e?.stack ?? e) };
      }
      model.set("result", r);
      model.save_changes();
      window.__globe = r; // wasm_check.py が読む
      out.textContent = r.ok ? `x(1) = ${r.final.x}（check ${r.passed ? "通過" : "失敗"}、wasm の線形メモリ ${(r.wasm_memory / 2 ** 20).toFixed(0)} MiB）`
                             : `失敗: ${r.error ?? JSON.stringify(r.errors)}`;
    }
    export default { render };
    """


    class WasmSim(anywidget.AnyWidget):
        """入力: pkg（wasm-bindgen --target web の置き場の URL）、file・text（.mo の名前と文面）、model、settings。出力: result。"""

        _esm = _ESM
        pkg = traitlets.Unicode("pkg").tag(sync=True)
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


if __name__ == "__main__":
    app.run()

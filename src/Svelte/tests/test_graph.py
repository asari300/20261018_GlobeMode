#!/usr/bin/env python3
"""connect グラフ（Graph.svelte）を Playwright の Chromium で確かめる（S046）。

  uv run python src/Svelte/tests/test_graph.py     # 先に pnpm build（mise run ui:check がやる）

dist/graph-test.html（Graph を単体で mount する試験用の入口）で、
1. GlobeModeValidation.Connect.RealCircuit のグラフを wasm から取り、sigma の canvas が出て、ノード数が JSON と一致し、
   ノードのクリックで onSelect が呼ばれることを確かめる
2. 深さ 2 にすると Hierarchical のノードが増える
3. 合成した 5k ノード・約 1 万辺のグラフで、カメラを動かしながら 2 秒間の fps を測る（ヘッドレスなので参考値）
console の error は失敗。
"""

from __future__ import annotations

import functools
import math
import http.server
import sys
import threading
import time
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

ROOT = Path(__file__).resolve().parents[3]
DIST = ROOT / "src/Svelte/dist"
MODEL = "GlobeModeValidation.Connect.RealCircuit"
HIER = "GlobeModeValidation.Connect.Hierarchical"

# ページ側で requestAnimationFrame の回数を数え、Python 側が本物のマウスでドラッグして動かす
FPS_START = """() => {
  const c = window.fps = { frames: 0, t0: 0, t1: 0, on: true };
  (function f(t) { c.t0 ||= t; c.t1 = t; c.frames++; if (c.on) requestAnimationFrame(f); })(performance.now());
}"""
FPS_STOP = "() => { const c = window.fps; c.on = false; return (c.frames - 1) / ((c.t1 - c.t0) / 1000); }"


def measure_fps(page, canvas) -> float:
    box = canvas.bounding_box()
    cx, cy = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2
    page.evaluate(FPS_START)
    page.mouse.move(cx, cy)
    page.mouse.down()
    t0 = time.monotonic()
    k = 0
    while time.monotonic() - t0 < 2:  # 2 秒、円を描いてドラッグ（sigma は動かしている間、辺とラベルを描かない）
        k += 1
        page.mouse.move(cx + 100 * math.sin(k / 5), cy + 100 * math.cos(k / 5))
    page.mouse.up()
    return page.evaluate(FPS_STOP)


CAMERA = "() => JSON.stringify(window.current.getSigma().getCamera().getState())"
NODE_POS = """(id) => {
  const s = window.current.getSigma();
  const a = s.getGraph().getNodeAttributes(id);
  return s.graphToViewport({ x: a.x, y: a.y });
}"""


def main() -> int:
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(DIST))
    handler.log_message = lambda *a: None  # type: ignore[attr-defined]
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    files = {
        str(p.relative_to(ROOT)).replace("\\", "/"): p.read_text(encoding="utf-8")
        for p in sorted((ROOT / "src/Modelica/GlobeModeValidation").rglob("*.mo"))
    }
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.on("console", lambda m: m.type == "error" and errors.append(m.text))
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.goto(f"http://127.0.0.1:{server.server_port}/graph-test.html")
        page.evaluate(
            "async (files) => { for (const [n, t] of Object.entries(files)) await window.backend.setFile(n, t); }", files
        )
        page.evaluate(
            "(m) => { window.selected = []; window.mountGraph(window.backend, m, (id) => window.selected.push(id)); }", MODEL
        )
        canvas = page.locator(".canvas")
        # 初回は wasm の読み込みを待つ
        expect(canvas).not_to_have_attribute("data-nodes", "0", timeout=120_000)
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        want = page.evaluate("async (m) => { const g = await window.backend.graph(m, 1); return [g.nodes.length, g.edges.length]; }", MODEL)
        expect(canvas).to_have_attribute("data-nodes", str(want[0]))
        expect(canvas).to_have_attribute("data-edges", str(want[1]))
        assert page.locator(".canvas canvas").count() > 0, "sigma の canvas が無い"
        print(f"RealCircuit: ノード {want[0]}、辺 {want[1]}、canvas {page.locator('.canvas canvas').count()} 枚")

        ids = page.evaluate("() => window.current.getSigma().getGraph().nodes()")
        pos = page.evaluate(NODE_POS, ids[0])
        canvas.click(position={"x": pos["x"], "y": pos["y"]}, force=True)
        selected = page.evaluate("window.selected")
        assert selected == [ids[0]], (selected, ids[0])
        print(f"クリック: onSelect({selected[0]!r})")

        # 深さ 2 で Hierarchical のノードが増える
        page.evaluate("(m) => window.mountGraph(window.backend, m, () => {})", HIER)
        expect(canvas).to_have_attribute("data-nodes", str(page.evaluate("async (m) => (await window.backend.graph(m, 1)).nodes.length", HIER)), timeout=60_000)
        page.get_by_label("深さ").select_option("2")
        deep = page.evaluate("async (m) => (await window.backend.graph(m, 2)).nodes.length", HIER)
        expect(canvas).to_have_attribute("data-nodes", str(deep), timeout=60_000)
        print(f"Hierarchical: 深さ 2 でノード {deep}")

        # 描画の fps（合成 5k ノード・約 1 万辺）
        page.evaluate("() => window.mountGraph(window.synthetic(5000), 'Synthetic', () => {})")
        expect(canvas).to_have_attribute("data-nodes", "5000", timeout=120_000)
        edges = canvas.get_attribute("data-edges")
        c0 = page.evaluate(CAMERA)
        fps = measure_fps(page, canvas)
        assert page.evaluate(CAMERA) != c0, "カメラが動いていない"
        print(f"fps: {fps:.1f}（ノード 5000、辺 {edges}、カメラを動かして 2 秒。ヘッドレス Chromium の参考値）")
        browser.close()
    server.shutdown()
    assert not errors, errors
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

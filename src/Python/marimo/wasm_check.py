#!/usr/bin/env python3
"""wasm_app.py を marimo の WASM（Pyodide）で書き出し、Chromium で Decay と MSL の ChuaCircuit の結果とメモリを確かめる（S102・S104）。

  uv run python src/Python/marimo/wasm_check.py [--pkg DIR] [--msl FILE] [--out DIR] [--offline]

1. `marimo export html-wasm --mode run` で OUT に書き出し、OUT/wasm/ に globe-mode-wasm/web の client.js・worker.js（GlobeWorker）、
   OUT/wasm/pkg/ に wasm-bindgen --target web の糊と .wasm（mise run wasm:build、既定は ${CARGO_TARGET_DIR:-target}/wasm-pkg/web）、
   OUT/msl/MSL.json に MSL（既定は src/Svelte/public/msl/MSL.json。src/Svelte の pack-msl.mjs が data/msl から作る）を写す。
2. OUT を localhost で配り（src/Svelte/tests/test_ui.py と同じ流儀）、Playwright の Chromium（--enable-precise-memory-info）で開く。
   --offline のときは CSP の header（CSP、S104）を付け、違反の報告（console の文言）が 0 件であることを確かめる。
   --offline が無いと Pyodide と anywidget を CDN・PyPI から取るので CSP は付けない。
3. widget が window.__globe[モデル名] に入れた結果で、Decay の x(1) = exp(-1)（誤差 1e-6 未満）、ChuaCircuit の check と simulate、
   simulate の間のメインのスレッドの timer の最大の間隔（1 s 未満なら固まっていない）を確かめ、JS の heap（performance.memory）、
   各 Worker の wasm の線形メモリ、ブラウザのプロセス（この Python の子孫）の RSS の合計を出す。2 GiB 以上なら失敗。
"""

from __future__ import annotations

import argparse
import functools
import http.server
import math
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import psutil
from playwright.sync_api import sync_playwright

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CHUA = "Modelica.Electrical.Analog.Examples.ChuaCircuit"
# --offline の書き出しに合う最小の CSP（S104 で 1 つずつ外して確かめた。理由は doc/research/marimo-wasm-2026-11-02.md）
CSP = "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' data:; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pkg", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "wasm-pkg/web")
    ap.add_argument("--msl", type=Path, default=ROOT / "src/Svelte/public/msl/MSL.json", help="MSL.json（pack-msl.mjs の出力）")
    ap.add_argument("--csp", default=CSP, help="--offline のときに付ける CSP（試す用）")
    ap.add_argument("--out", type=Path, default=None, help="書き出し先（既定は一時ディレクトリ）")
    ap.add_argument("--offline", action="store_true", help="marimo export の --offline（Pyodide とパッケージを書き出し先に取る）")
    a = ap.parse_args()
    if not a.msl.is_file():
        sys.exit(f"{a.msl} が無い。data/msl を取得して node src/Svelte/scripts/pack-msl.mjs")
    out = a.out or Path(tempfile.mkdtemp(prefix="globe-marimo-wasm-"))
    subprocess.run(["marimo", "export", "html-wasm", str(HERE / "wasm_app.py"), "-o", str(out), "--mode", "run", "-f", *(["--offline"] if a.offline else [])], check=True)
    (out / "wasm/pkg").mkdir(parents=True, exist_ok=True)
    (out / "msl").mkdir(exist_ok=True)
    for name in ("client.js", "worker.js"):
        shutil.copy2(ROOT / "src/Rust/mod/globe-mode-wasm/web" / name, out / "wasm" / name)
    for name in ("globe_mode_wasm.js", "globe_mode_wasm_bg.wasm"):
        shutil.copy2(a.pkg / name, out / "wasm/pkg" / name)
    shutil.copy2(a.msl, out / "msl/MSL.json")
    csp = a.csp if a.offline else ""

    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

        def end_headers(self) -> None:
            if csp:
                self.send_header("Content-Security-Policy", csp)
            super().end_headers()

        def log_message(self, *a: object) -> None:
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(out)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f"http://127.0.0.1:{server.server_port}/"
    errors: list[str] = []
    outside: set[str] = set()
    violations: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch(args=["--enable-precise-memory-info"])
        page = browser.new_page()
        page.on("pageerror", lambda e: errors.append(str(e)))
        # CSP の違反は console に出る（Worker の中の違反も page の console に来る）
        page.on("console", lambda m: "Content Security Policy" in m.text and violations.append(m.text))
        page.context.on("request", lambda r: r.url.startswith((origin, "data:", "blob:")) or outside.add(r.url.split("/")[2]))
        t0 = time.monotonic()
        page.goto(origin)
        # wait_for_function は文字列の式をページの中で eval するので CSP（'unsafe-eval' 無し）に断られる。evaluate（CDP）で待つ
        while len(g := page.evaluate("window.__globe ?? {}")) < 2:
            assert time.monotonic() - t0 < 900, (g, violations[:5], errors[:5])
            time.sleep(1)
        secs = time.monotonic() - t0
        heap = page.evaluate("({used: performance.memory.usedJSHeapSize, total: performance.memory.totalJSHeapSize})")
        rss = sum(c.memory_info().rss for c in psutil.Process().children(recursive=True))
        browser.close()
    server.shutdown()

    r, c = g["Decay"], g[CHUA]
    assert r.get("ok"), r
    assert c.get("ok"), c
    err = abs(r["final"]["x"] - math.exp(-1))
    print(f"globe-mode-wasm {r['version']}（marimo の WASM、anywidget の _esm、GlobeWorker、Chromium）: Decay check passed={r['passed']}、x(1) = {r['final']['x']}（誤差 {err:.1e}）")
    print(f"ChuaCircuit: MSL の読み込み {c['msl_ms'] / 1000:.1f} s、check passed={c['passed']}、simulate {c['sim_ms'] / 1000:.1f} s、C1.v(T) = {c['final']['C1.v']}、"
          f"simulate の間の timer {c['ticks']} 回・最大の間隔 {c['max_gap_ms']:.0f} ms")
    print(f"JS heap used {heap['used'] / 2**20:.0f} MiB / total {heap['total'] / 2**20:.0f} MiB、wasm の線形メモリ Decay {r['wasm_memory'] / 2**20:.0f} MiB・"
          f"ChuaCircuit {c['wasm_memory'] / 2**20:.0f} MiB、ブラウザの RSS の合計 {rss / 2**20:.0f} MiB")
    print(f"開いてから結果まで {secs:.1f} s。origin の外への要求: {sorted(outside)}。CSP: {csp or '（付けない）'}、違反 {len(violations)} 件")
    # marimo の RPC は Pyodide の起動が遅いと（手元の負荷で約 45 s）先に時間切れを投げるが、起動後の bridge は通る（S102）
    errors = [e for e in errors if e != "RPC request timed out."]
    assert r["passed"] and err < 1e-6, r
    assert c["passed"] and math.isfinite(c["final"]["C1.v"]), c
    assert c["ticks"] > 0 and c["max_gap_ms"] < 1000, c  # simulate が Worker にあり、メインのスレッドが固まらない
    assert not violations, violations
    assert rss < 2 * 2**30, rss
    assert not errors, errors
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

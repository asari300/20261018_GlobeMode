#!/usr/bin/env python3
"""wasm_app.py を marimo の WASM（Pyodide）で書き出し、Chromium で Decay の x(1) とメモリを確かめる（S102）。

  uv run python src/Python/marimo/wasm_check.py [--pkg DIR] [--out DIR] [--offline]

1. `marimo export html-wasm --mode run` で OUT に書き出し、wasm-bindgen --target web の糊と .wasm（mise run wasm:build、
   既定は ${CARGO_TARGET_DIR:-target}/wasm-pkg/web）を OUT/pkg/ に写す。
2. OUT を localhost で配り（src/Svelte/tests/test_ui.py と同じ流儀）、Playwright の Chromium（--enable-precise-memory-info）で開く。
   CSP の header は付けない（marimo の WASM に合う CSP は持ち越し）。--offline が無いと Pyodide と anywidget を CDN・PyPI から取る。
3. widget が window.__globe に入れた結果で x(1) = exp(-1)（誤差 1e-6 未満）を確かめ、JS の heap（performance.memory）、
   wasm の線形メモリ、ブラウザのプロセス（この Python の子孫）の RSS の合計を出す。2 GiB 以上なら失敗。
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


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pkg", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "wasm-pkg/web")
    ap.add_argument("--out", type=Path, default=None, help="書き出し先（既定は一時ディレクトリ）")
    ap.add_argument("--offline", action="store_true", help="marimo export の --offline（Pyodide とパッケージを書き出し先に取る）")
    a = ap.parse_args()
    out = a.out or Path(tempfile.mkdtemp(prefix="globe-marimo-wasm-"))
    subprocess.run(["marimo", "export", "html-wasm", str(HERE / "wasm_app.py"), "-o", str(out), "--mode", "run", "-f", *(["--offline"] if a.offline else [])], check=True)
    (out / "pkg").mkdir(exist_ok=True)
    for name in ("globe_mode_wasm.js", "globe_mode_wasm_bg.wasm"):
        shutil.copy2(a.pkg / name, out / "pkg" / name)

    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

        def log_message(self, *a: object) -> None:
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(out)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f"http://127.0.0.1:{server.server_port}/"
    errors: list[str] = []
    outside: set[str] = set()
    with sync_playwright() as p:
        browser = p.chromium.launch(args=["--enable-precise-memory-info"])
        page = browser.new_page()
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.context.on("request", lambda r: r.url.startswith((origin, "data:", "blob:")) or outside.add(r.url.split("/")[2]))
        t0 = time.monotonic()
        page.goto(origin)
        page.wait_for_function("window.__globe !== undefined", timeout=600_000, polling=1000)
        r = page.evaluate("window.__globe")
        secs = time.monotonic() - t0
        heap = page.evaluate("({used: performance.memory.usedJSHeapSize, total: performance.memory.totalJSHeapSize})")
        rss = sum(c.memory_info().rss for c in psutil.Process().children(recursive=True))
        browser.close()
    server.shutdown()

    assert r.get("ok"), r
    err = abs(r["final"]["x"] - math.exp(-1))
    print(f"globe-mode-wasm {r['version']}（marimo の WASM、anywidget の _esm、Chromium）: check passed={r['passed']}、x(1) = {r['final']['x']}（誤差 {err:.1e}）")
    print(f"JS heap used {heap['used'] / 2**20:.0f} MiB / total {heap['total'] / 2**20:.0f} MiB、wasm の線形メモリ {r['wasm_memory'] / 2**20:.0f} MiB、"
          f"ブラウザの RSS の合計 {rss / 2**20:.0f} MiB")
    print(f"開いてから結果まで {secs:.1f} s。origin の外への要求: {sorted(outside)}")
    # marimo の RPC は Pyodide の起動が遅いと（手元の負荷で約 45 s）先に時間切れを投げるが、起動後の bridge は通る（S102）
    errors = [e for e in errors if e != "RPC request timed out."]
    assert r["passed"] and err < 1e-6, r
    assert rss < 2 * 2**30, rss
    assert not errors, errors
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

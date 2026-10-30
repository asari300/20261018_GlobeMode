#!/usr/bin/env python3
"""Web Worker 経路をブラウザ（Playwright の Chromium）で確かめる（S036）。

  uv run python src/Rust/mod/globe-mode-wasm/web/browser.py [--pkg DIR]

HTTP サーバは立てず、page.route で http://globe.test/ を手元のファイルに割り当てる
（/web/ → このディレクトリ、/pkg/ → wasm-bindgen --target web の出力、/mo.json → GlobeModeValidation の .mo）。
index.html が module Worker で Decay（bdf）を回し、panic のあと作り直されて、もう一度回ることを確かめる。
初回は `uv run playwright install chromium` が要る。失敗すれば終了コード 1。
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
TYPES = {".js": "text/javascript", ".wasm": "application/wasm", ".html": "text/html"}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pkg", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "wasm-pkg/web")
    pkg = ap.parse_args().pkg.resolve()
    mo = {
        p.relative_to(ROOT).as_posix(): p.read_text(encoding="utf-8")
        for p in sorted((ROOT / "src/Modelica/GlobeModeValidation").rglob("*.mo"))
    }

    def handle(route):
        path = route.request.url.removeprefix("http://globe.test")
        if path == "/mo.json":
            return route.fulfill(status=200, content_type="application/json", body=json.dumps(mo))
        base, rel = {"/web/": (HERE, path[5:]), "/pkg/": (pkg, path[5:])}.get(path[:5], (None, ""))
        f = (base / rel).resolve() if base else None
        if not f or base.resolve() not in f.parents or not f.is_file():
            return route.fulfill(status=404, body="not found")
        route.fulfill(status=200, content_type=TYPES.get(f.suffix, "application/octet-stream"), body=f.read_bytes())

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        errors: list[str] = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.route("http://globe.test/**", handle)
        page.goto("http://globe.test/web/index.html")
        page.wait_for_function("window.__result !== undefined", timeout=120_000)
        r = page.evaluate("window.__result")
        browser.close()

    assert r["ok"], r.get("error")
    print(f"globe-mode-wasm {r['version']}（Web Worker、Chromium）")
    for label, key in (("Decay bdf", "first"), ("作り直し後の Decay bdf", "second")):
        s = r[key]
        err = abs(s["final"]["x"] - math.exp(-1))
        assert s["ok"] and s["termination"]["kind"] == "stop_time" and err < 1e-6, s
        print(f"{label}: x(1) = {s['final']['x']}（誤差 {err:.1e}）")
    print(f"wasm の線形メモリ {r['memory'] / 2**20:.0f} MiB")
    assert r["panicError"] and "試験の panic" in r["panicError"], r["panicError"]
    assert r["restarts"] == 1 and r["restartsAfter"] == 1, (r["restarts"], r["restartsAfter"])
    print("panic: reject され、worker を作り直した（restarts = 1）")
    assert not errors, errors  # ページの未捕捉の例外（panic の console.error は想定内なので見ない）
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

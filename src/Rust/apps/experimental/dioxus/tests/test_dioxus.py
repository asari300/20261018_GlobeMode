#!/usr/bin/env python3
"""Dioxus 版（S155、ADR-0036）を Playwright の Chromium で確かめる。

  uv run python src/Rust/apps/experimental/dioxus/tests/test_dioxus.py     # 先に mise run dioxus:build

dist/ を test_ui.serve() で配り（CSP は index.html の meta と同じものを header にも付ける）、次を確かめる。
- `?smoke=1` で `GM-SMOKE check=ok simulate=ok x1=… msl=ok ready_ms=…` が出る（dist/msl/MSL.json が無ければ msl は見ない）
- 画面の「検査」「実行」で診断が error 0 件、結果の表に x の最後の値 e⁻¹
- ダークモード: OS の設定に追従し、「テーマ」のボタンで light → dark に固定できる
- console の error と、配った origin の外への要求は失敗
"""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
sys.path.insert(0, str(ROOT / "src/Svelte/tests"))
import test_ui  # noqa: E402
from playwright.sync_api import expect, sync_playwright  # noqa: E402

test_ui.DIST = HERE.parent / "dist"
DARK, LIGHT = "rgb(13, 17, 23)", "rgb(255, 255, 255)"


def background(page) -> str:
    return page.evaluate("getComputedStyle(document.body).backgroundColor")


def main() -> int:
    dist = test_ui.DIST
    assert (dist / "pkg").exists(), "dist/ が無い（mise run dioxus:build）"
    has_msl = (dist / "msl/MSL.json").exists()
    server = test_ui.serve()
    base = f"http://127.0.0.1:{server.server_port}/"
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()

        def new_page(scheme: str = "light"):
            page = browser.new_context(color_scheme=scheme).new_page()
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
            page.on("pageerror", lambda e: errors.append(str(e)))
            page.on("request", lambda r: errors.append(f"外への要求: {r.url}") if not r.url.startswith((base, "data:")) else None)
            return page

        # smoke
        page = new_page()
        with page.expect_console_message(lambda m: m.text.startswith("GM-SMOKE"), timeout=600_000) as message:
            page.goto(base + "?smoke=1")
        line = message.value.text
        print(line)
        f = dict(re.findall(r"(\w+)=(\S+)", line.split(" failure=")[0]))
        assert f["check"] == "ok" and f["simulate"] == "ok", line
        assert not has_msl or f["msl"] == "ok", line
        assert all(k in f for k in ("ready_ms", "decay_ms", "msl_ms")), line

        # 画面の操作
        page = new_page()
        page.goto(base)
        page.click("#check")
        expect(page.locator("#status")).to_contain_text("通った")
        assert page.locator("#diagnostics .error").count() == 0
        page.click("#run")
        expect(page.locator("#finals tbody tr")).to_have_count(1)
        x = float(page.locator("#finals tbody td").nth(1).inner_text())
        assert abs(x - math.exp(-1)) < 1e-3, x
        page.fill("textarea", "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\n  x = 1;\nend Decay;\n")
        page.click("#check")
        expect(page.locator("#diagnostics .error").first).to_be_visible()

        # ダークモード
        dark = new_page("dark")
        dark.goto(base)
        assert background(dark) == DARK, background(dark)  # OS に追従
        light = new_page("light")
        light.goto(base)
        assert background(light) == LIGHT, background(light)
        light.click("#theme")  # auto → light
        assert background(light) == LIGHT
        light.click("#theme")  # light → dark
        assert background(light) == DARK, background(light)
        dark.click("#theme")  # auto → light: OS が dark でも固定できる
        assert background(dark) == LIGHT, background(dark)
        browser.close()
    server.shutdown()
    assert not errors, errors
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

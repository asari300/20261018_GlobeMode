#!/usr/bin/env python3
"""ブラウザ UI を Playwright の Chromium で確かめる（S041）。

  uv run python src/Svelte/tests/test_ui.py     # 先に pnpm build（mise run ui:check がやる）

dist/ を localhost で配り（CSP は index.html の meta のまま）、Decay を検査して error の診断が 0 件であること、
構造的に特異なモデルを検査して診断の行が出て、行を押すとエディタの選択が動くことを確かめる。console の error は失敗。
"""

from __future__ import annotations

import functools
import http.server
import sys
import threading
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

DIST = Path(__file__).resolve().parents[1] / "dist"
BAD = "model Bad\n  Real x;\nequation\n  der(x) = -x;\n  x = 1;\nend Bad;\n"


def main() -> int:
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(DIST))
    handler.log_message = lambda *a: None  # type: ignore[attr-defined]
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.on("console", lambda m: m.type == "error" and errors.append(m.text))
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.goto(f"http://127.0.0.1:{server.server_port}/")
        rows = page.locator("table.diagnostics tr")

        def check() -> None:
            button = page.get_by_role("button", name="検査")
            button.click(force=True)  # 初期表示の間は frame が出ず "stable" を待って詰まるので、確認を飛ばす
            expect(button).to_be_enabled(timeout=120_000)  # CSP が eval を禁じるので wait_for_function は使えない

        page.get_by_role("button", name="検査").wait_for()
        check()  # 初回は wasm の読み込みを待つ
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        assert rows.count() > 0, "info の診断（GM-I10001）は出るはず"  # 描画の確認も兼ねる
        assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
        print(f"Decay: 診断 {rows.count()} 行、error 0 行: {rows.all_inner_texts()}")

        page.get_by_label("モデル名").fill("Bad")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(BAD)
        check()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        texts = rows.all_inner_texts()
        assert page.locator("tr.error").count() > 0, texts
        print(f"Bad: 診断 {len(texts)} 行: {[t.replace(chr(9), ' ')[:70] for t in texts]}")
        rows.first.click(force=True)
        selected = page.evaluate("window.getSelection().toString()")
        print(f"行を押したあとの選択: {selected!r}")
        browser.close()
    server.shutdown()
    assert not errors, errors
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

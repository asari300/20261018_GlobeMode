#!/usr/bin/env python3
"""ブラウザ UI を Playwright の Chromium で確かめる（S041）。

  uv run python src/Svelte/tests/test_ui.py     # 先に pnpm build（mise run ui:check がやる）

dist/ を localhost で配り（CSP は index.html の meta と同じものを応答の header にも付ける。meta は Web Worker に効かないので。S093）、Decay を検査して error の診断が 0 件であること、実行するとプロットが出て信号の選択・右の軸・x の切り替え（S128）が効くこと（ブラウザの実行は列ごとの道で、data-typed が true。S065）、
10 万点（4 状態、出力間隔 0.01 s で 1000 s）の結果でも描けて時間を測ること、構造的に特異なモデルを検査して診断の行が出て、行を押すとエディタの選択が動くことを確かめる。1 ファイルの回路で「グラフ」を押して connect グラフが描かれることを確かめる（S046）。
最後に「MSL を読む」で MSL 4.1.0 を読み（public/msl/MSL.json が無ければ飛ばす）、ChuaCircuit の検査（error 0）と実行（プロットの canvas と系列）を確かめて、秒数を出す（S048）。
長いモデルの実行で進捗（段と t の割合）が出て、「止める」で止まり（data-run="cancelled"）、ボタンが戻ることを確かめる（S133）。止めたあとプロットに途中までの列が残る（S157。dist を cross-origin isolated で配る）。
Decay の `der(x)` の x で F12 を押すと宣言の行（`Real x`）へ動く。ChuaCircuit のファイルで型 `Modelica.Electrical.Analog.Basic.Resistor` の上の F12 は、ライブラリの Resistor.mo をエディタに開き、Alt+← で ChuaCircuit に戻る（S138）。
`.mo` を開く（ファイル選択）とエディタに入りモデル名が本文から付いて検査が通り、「保存」でダウンロードされ、再読み込みで localStorage から戻る（S141）。
名前の上にマウスを置くと宣言の 1 行が出る（Decay の x は `Real x`、ChuaCircuit の Resistor は見出しと説明文字列。S149）。
タブ（S151）: 開くたびにタブが増え、切り替えで本文・モデル名・診断が戻り、再読み込みでタブが戻り、閉じるボタンで減り（未保存なら confirm 1 つ。最後の 1 つは閉じられない）、「最近使ったファイル」から開き直せる。
ダークモード（S152）: `color_scheme="dark"` で開くと背景が暗く文字が明るい（body・エディタ・診断・プロット・graph。プロットの軸の文字は canvas に描くので明るい画素を数える）。テーマの select で「明」にすると戻り、再読み込みで選択が残り、「OS に合わせる」で暗に戻る。
FMU を書き出す（S144）: ブラウザの wasm が作ったソースの FMU がダウンロードされ、modelDescription.xml と sources/ がある。S154 から FMI の版（2.0 / 3.0）を select で選ぶ。
実行の設定（S142）: 終了時刻 2 とソルバ dopri45 で Decay を実行して x(2) が e⁻²、DecayK は experiment の Tolerance が初期値（出どころ experiment）に出て、k = 2 の上書きで x(1) が e⁻²。名前=値でない行は実行しない。
ライブラリの木で Modelica → Electrical → Analog → Examples → ChuaCircuit と開き、そのファイルがエディタに開いて検査が通ることも確かめる（S127）。
別のページで MSL.json を構文の誤りのファイルを含むライブラリに差し替え、読み飛ばしの警告 GM-W1001 の行と notes のファイル名が出ることを確かめる（S071）。
`?smoke=1` で起動すると Decay の検査と実行の結果が console に `GM-SMOKE check=ok simulate=ok x1=… msl=… ready_ms=…` の 1 行で出ること（S100、android.yml の smoke。時刻は S125）。MSL.json があれば msl=ok（MSL を読んで ChuaCircuit の検査が error 0。S103）。
UI の操作の項目 `tree= fmt= graph= cancel= settings= plot= def=`（S147。Desktop と Android の smoke と同じ。MSL.json が無ければ tree・graph は skip）も全部 ok で `failure=` が無いこと。
console の error（CSP の違反を含む）と、配った origin の外への要求は失敗（S093）。
"""

from __future__ import annotations

import base64
import functools
import http.server
import json
import math
import re
import subprocess
import sys
import threading
import time
import zipfile
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

DIST = Path(__file__).resolve().parents[1] / "dist"
TOLERANT = Path(__file__).resolve().parents[2] / "Rust/mod/globe-mode-frontend/tests/data/tolerant/a/A"
# 10 万点: 出力間隔 0.01 s で 1000 s
BIG = (
    "model Big\n  Real a(start = 1, fixed = true);\n  Real b(start = 1, fixed = true);\n  Real c(start = 1, fixed = true);\n"
    "  Real d(start = 1, fixed = true);\nequation\n  der(a) = -a;\n  der(b) = -0.5 * b;\n  der(c) = -0.1 * c;\n  der(d) = -0.01 * d;\n"
    "  annotation(experiment(StartTime = 0, StopTime = 1000, Interval = 0.01));\nend Big;\n"
)
# connect グラフ（S046）: ライブラリ無しで描ける 1 ファイルの回路
CIRCUIT = (
    "model G\n  connector P\n    Real v;\n    flow Real i;\n  end P;\n  model R\n    P p;\n    P n;\n  equation\n    p.i + n.i = 0;\n  end R;\n"
    "  R r1;\n  R r2;\nequation\n  connect(r1.n, r2.p);\nend G;\n"
)
# 進捗と止める（S133）: 1000 s の振動（wasm で数秒かかる）
LONG = "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation(experiment(StopTime = 1000));\nend Long;\n"
# 実行の設定（S142）: パラメータ k を上書きできる。experiment の Tolerance が初期値に出る
DECAY_K = (
    "model DecayK\n  parameter Real k = 1 \"減衰の速さ\";\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -k * x;\n"
    "  annotation(experiment(StopTime = 3, Tolerance = 1e-8));\nend DecayK;\n"
)
CHUA = "Modelica.Electrical.Analog.Examples.ChuaCircuit"
BAD = "model Bad\n  Real x;\nequation\n  der(x) = -x;\n  x = 1;\nend Bad;\n"


def serve(prefix: str = "") -> http.server.ThreadingHTTPServer:
    """dist/ を 127.0.0.1 の空いた port で配る（bench_ui.py も使う。S125）。prefix（例 "/app"）を渡すとその下だけで配る（S145）。"""
    csp = re.search(r'http-equiv="Content-Security-Policy" content="([^"]+)"', (DIST / "index.html").read_text(encoding="utf-8"))
    assert csp, "index.html に CSP の meta が無い"

    class Handler(http.server.SimpleHTTPRequestHandler):
        def translate_path(self, path: str) -> str:
            if prefix and not (path == prefix or path.startswith(prefix + "/") or path.startswith(prefix + "?")):
                return str(DIST / "__not_found__")
            return super().translate_path(path[len(prefix) :] or "/")

        def end_headers(self) -> None:
            self.send_header("Content-Security-Policy", csp[1])  # worker.js にも同じ CSP を効かせる
            # S157: 止めたときに途中の列を返す SharedArrayBuffer は cross-origin isolated が要る
            self.send_header("Cross-Origin-Opener-Policy", "same-origin")
            self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            super().end_headers()

        def log_message(self, *a: object) -> None:
            pass

    handler = functools.partial(Handler, directory=str(DIST))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def main() -> int:
    # 列の道の NaN / Inf は null になる（uPlot の欠損。S065）
    subprocess.run(["node", str(Path(__file__).with_name("test_columns.mjs"))], check=True)
    server = serve()
    errors: list[str] = []
    origin = f"http://127.0.0.1:{server.server_port}/"
    outside: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.on("console", lambda m: m.type == "error" and errors.append(m.text))
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.context.on("request", lambda r: r.url.startswith((origin, "data:", "blob:")) or outside.append(r.url))
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
        # 上書きできるパラメータの一覧（S150）: 検査のあとに取る。この Decay は parameter が無いので「無い」と出る
        expect(page.locator("[data-params]")).to_contain_text("上書きできるパラメータは無い", timeout=60_000)

        # 定義ジャンプ（S138）: 4 行目 `  der(x) = -x;` の末尾の x（`;` の手前）で F12 → 宣言の行へ
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+Home")
        for _ in range(3):
            page.keyboard.press("ArrowDown")
        page.keyboard.press("End")
        page.keyboard.press("ArrowLeft")
        page.keyboard.press("F12")
        expect(page.locator(".cm-activeLine")).to_contain_text("Real x(start", timeout=30_000)
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        print(f"Decay の F12: 動いた先の行 {page.locator('.cm-activeLine').inner_text()!r}")

        def hover_over(line: str, name: str, nth: int = 0) -> None:
            """行（部分一致）の中の name の文字（nth 番目の出現）の真ん中へマウスを動かす（S149）。"""
            box = page.evaluate(
                """([line, name, nth]) => {
                  const el = [...document.querySelectorAll('.cm-line')].find((e) => e.textContent.includes(line));
                  const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
                  const t = w.nextNode();
                  let i = -1;
                  for (let k = 0; k <= nth; k++) i = t.textContent.indexOf(name, i + 1);
                  const r = document.createRange();
                  r.setStart(t, i); r.setEnd(t, i + 1);
                  const b = r.getBoundingClientRect();
                  return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
                }""",
                [line, name, nth],
            )
            page.mouse.move(box["x"] - 30, box["y"])  # 一度ずらしてから置く（mousemove を確実に起こす）
            page.mouse.move(box["x"], box["y"])

        # ホバー（S149）: `der(x)` の x にマウスを置くと、宣言の `Real x` を含む tooltip が出る
        hover_over("der(x)", "x")
        tip = page.locator(".cm-tooltip .cm-hover")
        expect(tip).to_contain_text("Real x", timeout=30_000)
        print(f"Decay のホバー: {tip.inner_text()!r}")
        page.mouse.move(5, 5)
        expect(tip).to_have_count(0)

        run = page.get_by_role("button", name="実行")

        def simulate() -> None:
            run.click(force=True)
            expect(run).to_be_enabled(timeout=300_000)

        simulate()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        expect(page.locator(".plot canvas").first).to_be_visible()
        boxes = page.locator(".plot .signals input[name=signal]")
        shown = page.locator(".plot .signals input[name=signal]:checked").count()
        lines = page.locator(".plot .u-legend .u-series:not(.u-off)").count()  # time の行も含む
        print(f"Decay の実行: 信号 {boxes.count()} 本、選択 {shown} 本、凡例の表示中 {lines} 行、点 {page.locator('.plot').get_attribute('data-points')}")
        assert boxes.count() >= 1 and shown >= 1 and lines >= 2

        # 書き出し（S146）: CSV は選んだ系列と x の列（見出し time,x と数値の行）、PNG は canvas（image/png の本物で 0 バイトでない）
        with page.expect_download() as csv_dl:
            page.get_by_role("button", name="CSV").click(force=True)
        assert csv_dl.value.suggested_filename == "result.csv"
        csv_lines = Path(csv_dl.value.path()).read_text(encoding="utf-8").splitlines()
        assert csv_lines[0].startswith("time,x"), csv_lines[:2]
        assert len(csv_lines) == int(page.locator(".plot").get_attribute("data-points")) + 1, len(csv_lines)
        assert all(math.isfinite(float(v)) for v in csv_lines[1].split(",")[:2]), csv_lines[1]
        with page.expect_download() as png_dl:
            page.get_by_role("button", name="PNG").click(force=True)
        png = Path(png_dl.value.path()).read_bytes()
        assert png_dl.value.suggested_filename == "result.png" and png.startswith(b"\x89PNG\r\n\x1a\n") and len(png) > 1000, len(png)
        print(f"Decay の書き出し: CSV {len(csv_lines)} 行 {csv_lines[:2]}、PNG {len(png)} バイト")

        # 選択を外すと系列が消える
        boxes.first.uncheck(force=True)
        assert page.locator(".plot .u-legend .u-series:not(.u-off)").count() == lines - 1
        # 系列の選択・右の軸・パラメトリックの x（S128）: 選んだ系列だけを uPlot に渡し、軸と x の選択が DOM に出る
        plot = page.locator(".plot")
        expect(plot).to_have_attribute("data-series", "0")
        boxes.first.check(force=True)
        expect(plot).to_have_attribute("data-series", "1")
        page.get_by_label("x を右の軸に").check(force=True)
        expect(plot).to_have_attribute("data-right", "1")
        assert page.locator(".plot .u-axis").count() == 2  # x と右の y だけ（左の y は使う系列が無いので出さない）
        page.get_by_label("x 軸").select_option(label="x")
        expect(plot).to_have_attribute("data-x", "x")
        expect(page.locator(".plot canvas").first).to_be_visible()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        print(f"Decay の系列の選択: data-series={plot.get_attribute('data-series')}、data-right={plot.get_attribute('data-right')}、data-x={plot.get_attribute('data-x')}")

        # 実行の設定（S142）: 終了時刻 2・dopri45 で Decay を実行すると x(2) = e⁻²。DecayK は experiment の Tolerance が初期値に出て、k = 2 の上書きで x(1) = e⁻²
        tolerance, stop = page.get_by_label("許容誤差"), page.get_by_label("終了時刻")
        expect(page.locator("[data-origin=tolerance]")).to_have_text("既定")
        stop.fill("2")
        page.get_by_label("ソルバ").select_option("dopri45")
        simulate()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        last = page.locator("[data-last]").inner_text()
        assert "終了 2（指定）" in last and "ソルバ dopri45（指定）" in last, last
        x2 = json.loads(page.locator("[data-final]").get_attribute("data-final"))["x"]
        assert abs(x2 - math.exp(-2)) < 1e-4, x2
        page.get_by_label("モデル名").fill("DecayK")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(DECAY_K)
        check()
        expect(tolerance).to_have_value("1e-8")
        k_row = page.locator("[data-param=k]")  # DecayK の k（モデルの値 1、説明つき）が一覧に出る
        expect(k_row).to_be_visible(timeout=60_000)
        assert "減衰の速さ" in k_row.inner_text() and page.get_by_label("k の値", exact=True).get_attribute("placeholder") == "1", k_row.inner_text()
        expect(page.locator("[data-origin=tolerance]")).to_have_text("experiment")
        stop.fill("1")
        page.get_by_label("パラメータの上書き").fill("k = 2")
        simulate()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        last = page.locator("[data-last]").inner_text()
        assert "許容誤差 1e-8（experiment）" in last, last
        x1 = json.loads(page.locator("[data-final]").get_attribute("data-final"))["x"]
        assert abs(x1 - math.exp(-2)) < 1e-4, x1
        # 一覧の行に打った値も同じ道で送られる（S150）: textarea を空にして一覧の k に 2
        page.get_by_label("パラメータの上書き").fill("")
        page.get_by_label("k の値", exact=True).fill("2")
        simulate()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        xk = json.loads(page.locator("[data-final]").get_attribute("data-final"))["x"]
        assert abs(xk - math.exp(-2)) < 1e-4, xk
        print(f"一覧の k = 2: x(1) = {xk:.6f}")
        page.get_by_label("k の値", exact=True).fill("")
        page.get_by_label("パラメータの上書き").fill("k 2")  # 名前=値でない行は実行しないで理由を出す
        expect(page.get_by_role("alert")).to_contain_text("名前=値")
        print(f"実行の設定: x(2) = {x2:.6f}（e⁻² = {math.exp(-2):.6f}）、k = 2 で x(1) = {x1:.6f}、{last!r}")
        # 後の試験に残さない
        page.get_by_label("パラメータの上書き").fill("")
        stop.fill("")
        stop.dispatch_event("change")
        page.get_by_label("ソルバ").select_option("")

        page.get_by_label("モデル名").fill("Big")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(BIG)
        t = time.perf_counter()
        simulate()
        took = time.perf_counter() - t
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        expect(page.locator(".plot canvas").first).to_be_visible()
        n, ms = page.locator(".plot").get_attribute("data-points"), page.locator(".plot").get_attribute("data-ms")
        typed = page.locator(".plot").get_attribute("data-typed")
        assert typed == "true", "ブラウザの実行は列ごとの Float64Array の道（S065。CSV の解析は無い）"
        print(f"10 万点の描画: 点 {n}、系列 {page.locator('.plot .signals input[name=signal]:checked').count()} 本、列の道で最初の描画まで {ms} ms（目安 33 ms）、「実行」を押してから戻るまで {took * 1000:.0f} ms")
        assert int(n or 0) >= 100_000

        # 進捗と「止める」（S133、ADR-0033）: 長い実行で段と t の割合が出て、「止める」で cancelled になり、ボタンが戻る
        page.get_by_label("モデル名").fill("Long")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(LONG)
        run.click(force=True)
        status = page.get_by_role("status")
        expect(status).to_contain_text("integration", timeout=120_000)
        expect(status).to_contain_text("%")
        shown_progress = status.inner_text()
        t = time.perf_counter()
        page.get_by_role("button", name="止める").click(force=True)
        expect(status).to_have_attribute("data-run", "cancelled", timeout=10_000)
        expect(run).to_be_enabled(timeout=10_000)
        assert page.get_by_role("button", name="止める").count() == 0
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        # S157: 止めたあとも、そこまでの時系列がプロットに残る（前の 10 万点でなく、Long の途中までの列）。SharedArrayBuffer が使えない配信では出ない
        kept = int(page.locator(".plot").get_attribute("data-points") or 0)
        assert 2 <= kept < 100_000 and int(page.locator(".plot").get_attribute("data-series") or 0) >= 1, f"止めたあとのプロットの点 {kept}"
        expect(status).to_contain_text("そこまでの")
        print(f"止める: 進捗「{shown_progress}」から {1000 * (time.perf_counter() - t):.0f} ms で「{status.inner_text()}」、プロットに {kept} 点が残り、実行のボタンが戻った")
        check()  # 作り直した worker で次の呼び出しが回る
        assert page.locator("tr.error").count() == 0, rows.all_inner_texts()

        # 整形（S053）: 詰めた本文が doc/format-style.md の形になり、lint の波線（説明文字列が無い）が出る
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text("model Fmt\n  Real x(start=1,fixed = true);\nequation\n  der(x)=-x;\nend Fmt;\n")
        fmt = page.get_by_role("button", name="整形")
        fmt.click(force=True)
        expect(fmt).to_be_enabled(timeout=60_000)
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        formatted = page.locator(".cm-content").inner_text()
        assert "Real x(start = 1, fixed = true);" in formatted and "der(x) = -x;" in formatted, formatted
        expect(page.locator(".cm-lintRange-warning").first).to_be_visible(timeout=30_000)
        print("整形: 空白を直し、lint の波線あり")

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

        page.get_by_label("モデル名").fill("G")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(CIRCUIT)
        page.get_by_role("button", name="グラフ").click(force=True)
        canvas = page.locator(".canvas[data-nodes]")
        expect(canvas).not_to_have_attribute("data-nodes", "0", timeout=120_000)
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        print(f"G のグラフ: ノード {canvas.get_attribute('data-nodes')}、辺 {canvas.get_attribute('data-edges')}")

        if (DIST / "msl/MSL.json").exists():
            note = page.locator("p.note")
            t = time.perf_counter()
            page.get_by_role("button", name="MSL を読む").click(force=True)
            expect(note).to_have_attribute("data-msl", "loaded", timeout=600_000)
            print(f"MSL を読む: {time.perf_counter() - t:.1f} s（{note.inner_text()[:60]}）")
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()

            page.get_by_label("モデル名").fill(CHUA)
            t = time.perf_counter()
            check()
            took = time.perf_counter() - t
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()  # 診断は 0 行でよい（experiment がある）
            print(f"ChuaCircuit の検査: {took:.1f} s、診断 {rows.count()} 行、error 0 行")
            # 上書きできるパラメータの一覧（S150）: 絞り込みで本数が減る
            all_params = page.locator("[data-param]")
            expect(all_params.first).to_be_visible(timeout=120_000)
            n_all = all_params.count()
            page.get_by_label("パラメータの絞り込み").fill("Ro.")
            n_some = all_params.count()
            assert n_all > 12 and 0 < n_some < n_all, (n_all, n_some)
            print(f"ChuaCircuit の上書きできるパラメータ: {n_all} 本、「Ro.」で {n_some} 本")
            page.get_by_label("パラメータの絞り込み").fill("")

            t = time.perf_counter()
            simulate()
            took = time.perf_counter() - t
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
            expect(page.locator(".plot canvas").first).to_be_visible()
            boxes = page.locator(".plot .signals input[name=signal]")
            lines = page.locator(".plot .u-legend .u-series:not(.u-off)").count()
            print(f"ChuaCircuit の実行: {took:.1f} s、信号 {boxes.count()} 本、凡例の表示中 {lines} 行、点 {page.locator('.plot').get_attribute('data-points')}")
            assert boxes.count() >= 1 and lines >= 2
            assert "C1.v" in page.locator(".plot .signals").inner_text()
            # パラメトリック（S128）: C1.v 対 C2.v
            page.get_by_label("x 軸").select_option(label="C1.v")
            page.get_by_role("checkbox", name="C2.v", exact=True).check(force=True)
            expect(page.locator(".plot")).to_have_attribute("data-x", "C1.v")
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            print(f"ChuaCircuit の C1.v 対 C2.v: 系列 {page.locator('.plot').get_attribute('data-series')} 本")

            # ライブラリのモデルのグラフも引ける（S079）
            page.get_by_role("button", name="グラフ").click(force=True)
            canvas = page.locator(".canvas")
            expect(canvas).not_to_have_attribute("data-nodes", "0", timeout=60_000)
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            print(f"ChuaCircuit のグラフ: ノード {canvas.get_attribute('data-nodes')}、辺 {canvas.get_attribute('data-edges')}")

            # ライブラリの木（S127）: Modelica → Electrical → Analog → Examples と開き、ChuaCircuit のファイルをエディタに開いて検査する
            t = time.perf_counter()
            for path in ["Modelica", "Modelica.Electrical", "Modelica.Electrical.Analog", "Modelica.Electrical.Analog.Examples"]:
                page.locator(f'.library li[data-path="{path}"] > button').click(force=True)
            page.locator(f'.library li[data-path="{CHUA}"] > button').click(force=True)
            expect(page.get_by_label("モデル名")).to_have_value(CHUA, timeout=60_000)
            expect(page.locator(".cm-content")).to_contain_text("model ChuaCircuit")
            opened = time.perf_counter() - t
            check()
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
            print(f"木から ChuaCircuit を開く: {opened:.1f} s、検査まで {time.perf_counter() - t:.1f} s、診断 {rows.count()} 行、error 0 行")
            # 定義ジャンプ（S138）: 9 行目 `  Modelica.Electrical.Analog.Basic.Resistor Ro(…` の Resistor の中で F12 → ライブラリのファイルが開く
            page.locator(".cm-content").focus()
            page.keyboard.press("Control+Home")
            for _ in range(8):
                page.keyboard.press("ArrowDown")
            for _ in range(38):
                page.keyboard.press("ArrowRight")
            page.keyboard.press("F12")
            expect(page.locator(".cm-content")).to_contain_text("model Resistor", timeout=60_000)
            expect(page.locator(".cm-activeLine")).to_contain_text("model Resistor")
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            page.keyboard.press("Alt+ArrowLeft")
            expect(page.locator(".cm-content")).to_contain_text("model ChuaCircuit")
            print("ChuaCircuit の F12: Resistor.mo が開き、Alt+← で ChuaCircuit に戻った")
            # ホバー（S149）: ライブラリのクラス Resistor の上で、見出しと説明文字列が出る
            hover_over("Basic.Resistor Ro", "Resistor")
            expect(tip).to_contain_text("Ideal linear electrical resistor", timeout=30_000)
            print(f"ChuaCircuit のホバー: {tip.inner_text()!r}")
        else:
            print("public/msl/MSL.json が無いので MSL の試験は飛ばす")
        # ライブラリに構文の誤りのファイルがあっても読み、読み飛ばしを GM-W1001 の行（ファイル名は notes）で出す（S071）。
        # MSL.json の取得を差し替えて、frontend の fixture（tests/data/tolerant/a）を渡す
        lib = TOLERANT.parent
        files = {p.relative_to(lib).as_posix(): p.read_text(encoding="utf-8") for p in sorted(TOLERANT.glob("*.mo"))}
        tolerant = browser.new_page()
        tolerant.on("console", lambda m: m.type == "error" and errors.append(m.text))
        tolerant.on("pageerror", lambda e: errors.append(str(e)))
        tolerant.route("**/msl/MSL.json", lambda route: route.fulfill(status=200, content_type="application/json", body=json.dumps(files)))
        tolerant.goto(f"http://127.0.0.1:{server.server_port}/")
        tolerant.get_by_role("button", name="MSL を読む").click(force=True)
        expect(tolerant.locator("p.note")).to_have_attribute("data-msl", "loaded", timeout=120_000)
        tolerant.get_by_label("モデル名").fill("A.Broken")
        button = tolerant.get_by_role("button", name="検査")
        button.click(force=True)
        expect(button).to_be_enabled(timeout=120_000)
        warned = tolerant.locator("table.diagnostics tr", has_text="GM-W1001")
        expect(warned).to_have_count(1, timeout=30_000)
        assert "Broken.mo" in warned.inner_text(), warned.inner_text()
        print(f"構文の誤りのライブラリ: {warned.inner_text().replace(chr(9), ' ')[:120]!r}")
        # 開く・保存（S141）: .mo を選ぶとエディタに入り（モデル名は本文から）、検査が通る。「保存」でダウンロードが起き、再読み込みでも本文が戻る
        own = browser.new_page()
        own.on("console", lambda m: m.type == "error" and errors.append(m.text))
        own.on("pageerror", lambda e: errors.append(str(e)))
        own.goto(f"http://127.0.0.1:{server.server_port}/")
        mine = "model Mine\n  Real y(start = 2, fixed = true);\nequation\n  der(y) = -y;\nend Mine;\n"
        with own.expect_file_chooser() as chooser:
            own.get_by_role("button", name="開く").click(force=True)
        chooser.value.set_files(files=[{"name": "Mine.mo", "mimeType": "text/plain", "buffer": mine.encode()}])
        expect(own.locator(".cm-content")).to_contain_text("model Mine", timeout=30_000)
        expect(own.get_by_label("モデル名")).to_have_value("Mine")
        button = own.get_by_role("button", name="検査")
        button.click(force=True)
        expect(button).to_be_enabled(timeout=120_000)
        assert own.get_by_role("alert").count() == 0 and own.locator("tr.error").count() == 0, own.locator("table.diagnostics").inner_text()
        with own.expect_download() as download:
            own.get_by_role("button", name="保存").click(force=True)
        assert download.value.suggested_filename == "Mine.mo"
        assert Path(download.value.path()).read_text(encoding="utf-8") == mine
        expect(own.locator("p[data-saved]")).to_contain_text("Mine.mo")
        own.reload()
        expect(own.locator(".cm-content")).to_contain_text("model Mine", timeout=30_000)
        expect(own.get_by_label("モデル名")).to_have_value("Mine")
        print("開く・保存: Mine.mo を開いて検査が通り、ダウンロードの内容が一致し、再読み込みで戻った")
        # FMU を書き出す（S144）: wasm が作ったソースの FMU（zip）がダウンロードされる
        with own.expect_download() as fmu:
            own.get_by_role("button", name="FMU を書き出す").click(force=True)
        assert fmu.value.suggested_filename == "Mine.fmu"
        with zipfile.ZipFile(fmu.value.path()) as z:
            assert "modelDescription.xml" in z.namelist() and any(n.startswith("sources/") for n in z.namelist()), z.namelist()
            assert b'modelName="Mine"' in z.read("modelDescription.xml")
        expect(own.locator("p[data-saved]")).to_contain_text("Mine.fmu")
        print("FMU: Mine.fmu がダウンロードされ、modelDescription.xml と sources/ がある")
        # FMI の版を選ぶ（S154）: 既定は 2.0、3.0 を選ぶと fmiVersion="3.0" の modelDescription.xml
        assert b'fmiVersion="2.0"' in zipfile.ZipFile(fmu.value.path()).read("modelDescription.xml")
        own.get_by_label("FMI の版").select_option("3")
        with own.expect_download() as fmu3:
            own.get_by_role("button", name="FMU を書き出す").click(force=True)
        with zipfile.ZipFile(fmu3.value.path()) as z:
            assert b'fmiVersion="3.0"' in z.read("modelDescription.xml") and any(n.startswith("sources/") for n in z.namelist()), z.namelist()
        print("FMU: FMI の版に 3.0 を選ぶと modelDescription.xml が fmiVersion=3.0")
        # タブと最近使ったファイル（S151）: 開くとタブが増え、切り替えで本文・モデル名・診断が戻り、再読み込みでタブが戻り、閉じて、最近使ったファイルから開き直せる
        tab = browser.new_page()
        tab.on("console", lambda m: m.type == "error" and errors.append(m.text))
        tab.on("pageerror", lambda e: errors.append(str(e)))
        tab.goto(f"http://127.0.0.1:{server.server_port}/")
        other = "model Other\n  Real z(start = 3, fixed = true);\nequation\n  der(z) = -z;\nend Other;\n"

        def open_mo(name: str, body: str) -> None:
            with tab.expect_file_chooser() as ch:
                tab.get_by_role("button", name="開く").click(force=True)
            ch.value.set_files(files=[{"name": name, "mimeType": "text/plain", "buffer": body.encode()}])

        tabs = tab.get_by_role("tab")
        content, name_box = tab.locator(".cm-content"), tab.get_by_label("モデル名")
        tab.get_by_role("button", name="検査").wait_for()
        open_mo("Mine.mo", mine)
        expect(content).to_contain_text("model Mine", timeout=30_000)
        expect(tabs).to_have_count(2)  # 初めの Model.mo と Mine.mo
        tabs.nth(0).click()
        expect(content).to_contain_text("model Decay")
        expect(name_box).to_have_value("Decay")
        button = tab.get_by_role("button", name="検査")
        button.click(force=True)
        expect(button).to_be_enabled(timeout=120_000)
        decay_rows = tab.locator("table.diagnostics tr").count()
        assert decay_rows > 0
        tabs.nth(1).click()  # 診断は文書ごと
        expect(content).to_contain_text("model Mine")
        expect(name_box).to_have_value("Mine")
        expect(tab.locator("table.diagnostics tr")).to_have_count(0)
        tabs.nth(0).click()
        expect(tab.locator("table.diagnostics tr")).to_have_count(decay_rows)
        open_mo("Other.mo", other)
        expect(content).to_contain_text("model Other", timeout=30_000)
        expect(tabs).to_have_count(3)
        tab.reload()
        expect(tabs).to_have_count(3, timeout=30_000)
        expect(tabs.nth(2)).to_have_attribute("aria-selected", "true")
        expect(content).to_contain_text("model Other")
        tabs.nth(1).click()
        expect(content).to_contain_text("model Mine")
        # 未保存のタブを閉じるときは confirm 1 つ（取り消せば残る）
        content.click()
        tab.keyboard.type("x")
        dialogs: list[str] = []
        tab.once("dialog", lambda d: (dialogs.append(d.message), d.dismiss()))
        tab.get_by_role("button", name="Mine.mo を閉じる").click(force=True)
        assert len(dialogs) == 1 and "保存していない" in dialogs[0], dialogs
        expect(tabs).to_have_count(3)
        content.click()
        tab.keyboard.press("Control+z")  # 元に戻して開いたときと同じ本文なら確かめない（別のタブを閉じる）
        tab.get_by_role("button", name="Other.mo を閉じる").click(force=True)
        expect(tabs).to_have_count(2)
        tab.get_by_label("最近使ったファイル").select_option(label="Other.mo")
        expect(tabs).to_have_count(3)
        expect(content).to_contain_text("model Other")
        expect(name_box).to_have_value("Other")
        tab.get_by_role("button", name="Model.mo を閉じる").click(force=True)
        tab.get_by_role("button", name="Mine.mo を閉じる").click(force=True)
        expect(tabs).to_have_count(1)
        assert tab.get_by_role("button", name="を閉じる").count() == 0, "最後の 1 つは閉じられない"
        print("タブ: 開く 2→3、切り替えで本文・モデル名・診断が戻り、再読み込みで 3 つ戻り、閉じて、最近使ったファイルから開き直せた")
        # ダークモード（S152）: OS が暗いと背景が暗く文字が明るい。エディタ・プロット・graph も暗い。手動で「明」にすると戻り、再読み込みで選択が残る
        dark = browser.new_page(color_scheme="dark")
        dark.on("console", lambda m: m.type == "error" and errors.append(m.text))
        dark.on("pageerror", lambda e: errors.append(str(e)))
        dark.goto(origin)
        dark.get_by_role("button", name="検査").wait_for()

        def lum(css: str) -> float:
            r, g, b = (int(float(v)) for v in re.findall(r"[\d.]+", css)[:3])
            return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255

        # 要素の実効の背景（透明なら親をたどる）と文字の色
        look = """(sel) => {
          const e = document.querySelector(sel);
          let b = e;
          while (b && /rgba\\(.*, 0\\)|transparent/.test(getComputedStyle(b).backgroundColor)) b = b.parentElement;
          return [getComputedStyle(b ?? document.body).backgroundColor, getComputedStyle(e).color];
        }"""

        def tone(sel: str) -> tuple[float, float]:
            bg, fg = dark.evaluate(look, sel)
            return lum(bg), lum(fg)

        def check_dark(sel: str) -> None:
            bg, fg = tone(sel)
            assert bg < 0.2 and fg > 0.6, (sel, bg, fg)

        def check_light(sel: str) -> None:
            bg, fg = tone(sel)
            assert bg > 0.8 and fg < 0.3, (sel, bg, fg)

        # 要素を撮って、色 rgb に近い画素を数える（作り直した直後に空になっていないか。sigma は WebGL なので canvas を直接は読めない）
        near_js = """async ([b64, rgb, tol]) => {
          const img = new Image();
          img.src = 'data:image/png;base64,' + b64;
          await img.decode();
          const c = document.createElement('canvas');
          c.width = img.width; c.height = img.height;
          const g = c.getContext('2d');
          g.drawImage(img, 0, 0);
          const d = g.getImageData(0, 0, c.width, c.height).data;
          let n = 0;
          for (let i = 0; i < d.length; i += 4) if (d[i + 3] > 200 && Math.abs(d[i] - rgb[0]) < tol && Math.abs(d[i + 1] - rgb[1]) < tol && Math.abs(d[i + 2] - rgb[2]) < tol) n++;
          return n;
        }"""

        def css_rgb(name: str) -> list[int]:
            h = dark.evaluate("(n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim()", name).lstrip("#")
            return [int(h[i : i + 2], 16) for i in (0, 2, 4)]

        def wait_near(sel: str, rgb: list[int], minimum: int, what: str) -> None:
            n = 0
            for _ in range(40):  # 作り直しの直後は待つ（最大 12 秒）
                n = dark.evaluate(near_js, [base64.b64encode(dark.locator(sel).first.screenshot()).decode(), rgb, 40])
                if n >= minimum:
                    return
                time.sleep(0.3)
            raise AssertionError(f"{what}: 色 {rgb} に近い画素が {n} 個（{minimum} 個以上のはず。空のまま？）")

        for sel in ["body", ".cm-editor", ".cm-content", ".cm-gutters", "select[aria-label=テーマ]"]:
            check_dark(sel)
        button = dark.get_by_role("button", name="検査")
        button.click(force=True)
        expect(button).to_be_enabled(timeout=120_000)
        expect(dark.locator("table.diagnostics td").first).to_be_visible()
        check_dark("table.diagnostics td")
        run_dark = dark.get_by_role("button", name="実行")
        run_dark.click(force=True)
        expect(run_dark).to_be_enabled(timeout=300_000)
        expect(dark.locator(".plot canvas").first).to_be_visible()
        check_dark(".plot")
        # uPlot の軸の文字は canvas に描くので、いまの文字色（--fg）に近い画素があること（明のときの黒い軸のままなら、暗い地で見えない）
        ink_px = """() => {
          const [r, g, b] = getComputedStyle(document.body).color.match(/[\\d.]+/g).map(Number);
          const c = document.querySelector('.plot canvas'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
          let n = 0;
          for (let i = 0; i < d.length; i += 4) if (d[i + 3] > 200 && Math.abs(d[i] - r) < 12 && Math.abs(d[i + 1] - g) < 12 && Math.abs(d[i + 2] - b) < 12) n++;
          return n;
        }"""
        assert dark.evaluate(ink_px) > 30, "暗いのに軸の文字が文字色になっていない"
        series = [0x1F, 0x77, 0xB4]  # Plot.svelte の COLORS[0]。x の線
        wait_near(".plot canvas", series, 50, "暗のプロットの系列の線")
        dark.get_by_label("モデル名").fill("G")
        dark.locator(".cm-content").focus()
        dark.keyboard.press("Control+A")
        dark.keyboard.insert_text(CIRCUIT)
        dark.get_by_role("button", name="グラフ").click(force=True)
        expect(dark.locator(".canvas[data-nodes]")).not_to_have_attribute("data-nodes", "0", timeout=120_000)
        check_dark(".canvas")
        wait_near(".canvas", css_rgb("--accent"), 20, "暗の graph のノード")
        # 手動で明に: 背景が明るく文字が暗くなり、プロットは作り直されて軸が暗い字になる。再読み込みで選択が残る
        dark.get_by_label("テーマ").select_option("light")
        for sel in ["body", ".cm-editor", ".cm-content", ".plot", ".canvas"]:
            check_light(sel)
        time.sleep(1)  # プロットの作り直しを待つ
        assert dark.evaluate(ink_px) > 30, "明にしても軸の文字が文字色になっていない（暗いままの色を渡している）"
        wait_near(".plot canvas", series, 50, "明のプロットの系列の線（作り直したあと）")
        expect(dark.locator(".canvas[data-nodes]")).not_to_have_attribute("data-nodes", "0")
        wait_near(".canvas", css_rgb("--accent"), 20, "明の graph のノード（作り直したあと）")
        assert dark.evaluate("document.documentElement.dataset.theme") == "light"
        dark.reload()
        dark.get_by_role("button", name="検査").wait_for()
        expect(dark.get_by_label("テーマ")).to_have_value("light")
        check_light("body")
        dark.get_by_label("テーマ").select_option("dark")
        check_dark(".cm-editor")
        dark.get_by_label("テーマ").select_option("auto")  # OS（暗）に合わせる
        assert dark.evaluate("document.documentElement.dataset.theme") is None
        check_dark("body")
        light_os = browser.new_page(color_scheme="light")  # OS が明なら明
        light_os.goto(origin)
        light_os.get_by_role("button", name="検査").wait_for()
        bg, fg = light_os.evaluate(look, "body")
        assert lum(bg) > 0.8 and lum(fg) < 0.3, (bg, fg)
        light_os.close()
        dark.close()
        print("ダークモード: OS が暗いと body・エディタ・診断・プロット・graph が暗く、軸の文字が明るく、系列の線とノードが描かれている。明に切り替え（作り直し）ても線とノードが残り、戻り、再読み込みで残り、「OS に合わせる」で暗に戻る")
        # ?smoke=1 で起動すると Decay を検査・実行して GM-SMOKE の 1 行を console に出す（S100。android.yml が logcat で待つ行）
        smoke = browser.new_page()
        smoke.on("pageerror", lambda e: errors.append(str(e)))
        with smoke.expect_console_message(lambda m: m.text.startswith("GM-SMOKE"), timeout=600_000) as message:
            smoke.goto(f"http://127.0.0.1:{server.server_port}/?smoke=1")
        line = message.value.text
        assert line.startswith("GM-SMOKE check=ok simulate=ok x1=0.36"), line
        assert ("msl=ok" in line) == (DIST / "msl/MSL.json").exists(), line  # S103
        # S147: UI の操作の項目。MSL が無ければ木とグラフは skip
        skipped = "skip" if not (DIST / "msl/MSL.json").exists() else "ok"
        assert f" tree={skipped} fmt=ok graph={skipped} cancel=ok settings=ok plot=ok def=ok " in line and "failure=" not in line, line
        assert re.search(r" ready_ms=\d+ decay_ms=\d+ msl_ms=", line), line  # S125: bench_ui.py が読む時刻
        print(line)
        # サブパス配信（S145）: dist/ を /app/ の下だけで配っても、worker・wasm・MSL.json が相対パスで引けて、検査と MSL の読み込みができる
        sub_server = serve("/app")
        sub = browser.new_page()
        sub.on("console", lambda m: m.type == "error" and errors.append(m.text))
        sub.on("pageerror", lambda e: errors.append(str(e)))
        sub.goto(f"http://127.0.0.1:{sub_server.server_port}/app/")
        button = sub.get_by_role("button", name="検査")
        button.wait_for()
        button.click(force=True)
        expect(button).to_be_enabled(timeout=120_000)
        assert sub.get_by_role("alert").count() == 0 and sub.locator("tr.error").count() == 0
        if (DIST / "msl/MSL.json").exists():
            sub.get_by_role("button", name="MSL を読む").click()
            expect(sub.locator("p[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=120_000)
        print("サブパス /app/: 検査が通り、MSL も読めた")
        sub_server.shutdown()
        # 小画面（S145）: 幅 400 px で横スクロールが出ず、主要なボタンが画面の幅に収まって見える（左の木が上に積まれる）
        narrow = browser.new_page(viewport={"width": 400, "height": 800})
        narrow.on("pageerror", lambda e: errors.append(str(e)))
        narrow.goto(f"http://127.0.0.1:{server.server_port}/")
        narrow.get_by_role("button", name="検査").wait_for()
        if (DIST / "msl/MSL.json").exists():
            narrow.get_by_role("button", name="MSL を読む").click()
            expect(narrow.locator("p[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=120_000)
            expect(narrow.locator(".library")).to_be_visible()
        for name in ["開く", "保存", "検査", "実行", "グラフ", "整形"]:
            box = narrow.get_by_role("button", name=name, exact=True).bounding_box()
            assert box and box["x"] >= 0 and box["x"] + box["width"] <= 400, (name, box)
        width = narrow.evaluate("document.documentElement.scrollWidth")
        assert width <= 400, f"小画面で横スクロールが出る: scrollWidth {width}"
        editor = narrow.locator(".editor").bounding_box()
        assert editor and editor["width"] >= 300, f"小画面でエディタが細い（左の木が横に並んでいる）: {editor}"
        print(f"小画面 400 px: 主要なボタンが収まり、scrollWidth {width}、エディタの幅 {editor['width']:.0f} px")
        browser.close()
    server.shutdown()
    assert not errors, errors
    assert not outside, outside
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

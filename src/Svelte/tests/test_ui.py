#!/usr/bin/env python3
"""ブラウザ UI を Playwright の Chromium で確かめる（S041）。

  uv run python src/Svelte/tests/test_ui.py     # 先に pnpm build（mise run ui:check がやる）

dist/ を localhost で配り（CSP は index.html の meta と同じものを応答の header にも付ける。meta は Web Worker に効かないので。S093）、Decay を検査して error の診断が 0 件であること、実行するとプロットが出て信号の選択・右の軸・x の切り替え（S128）が効くこと（ブラウザの実行は列ごとの道で、data-typed が true。S065）、
10 万点（4 状態、出力間隔 0.01 s で 1000 s）の結果でも描けて時間を測ること、構造的に特異なモデルを検査して診断の行が出て、行を押すとエディタの選択が動くことを確かめる。1 ファイルの回路で「グラフ」を押して connect グラフが描かれることを確かめる（S046）。
クラス名のリネーム（S186）: モデル名のクラス（入れ子の途中も）を F2 で変えると欄も追って検査が通り、Ctrl+Z で戻る。ライブラリの木・検索で入れ子のクラスを開くと宣言の行にカーソルがあって画面に入る。候補が開いたままの Ctrl+Enter は候補を閉じる。「MSL を読む」を Enter で押すとフォーカスは検索欄へ、「結果へ」は結果を画面へ入れる。
最後に「MSL を読む」で MSL 4.1.0 を読み（public/msl/MSL.json が無ければ飛ばす）、ChuaCircuit の検査（error 0）と実行（プロットの canvas と系列）を確かめて、秒数を出す（S048）。
長いモデルの実行で進捗（段と t の割合）が出て、「止める」で止まり（data-run="cancelled"）、ボタンが戻ることを確かめる（S133）。止めたあとプロットに途中までの列が残る（S157。dist を cross-origin isolated で配る）。
説明パネル（S232）: カーソルのあるクラスの Documentation が右のパネルに出る。script・onerror・javascript: は出ず走らず（白リストで組み直す）、表と改訂履歴は出て、無ければ「Documentation が無い」。暗でも読める。MSL の PID を検索で開くと本文が出る。
折りたたみ（S225）: gutter の印で model Decay を畳むと `end Decay;` の行が隣に来て、開くと戻る（範囲は Rust の foldingRange）。
Decay の `der(x)` の x で F12 を押すと宣言の行（`Real x`）へ動く。ChuaCircuit のファイルで型 `Modelica.Electrical.Analog.Basic.Resistor` の上の F12 は、ライブラリの Resistor.mo をエディタに開き、Alt+← で ChuaCircuit に戻る（S138）。
構文の色付け（S163）: 予約語・型・数値・文字列・コメントに別々の色のクラス（cm-mo-*）が付き、エディタの地に対してコントラスト 4.5 以上。暗でも同じ。editor.ts の KEYWORDS は Rust の KEYWORDS と同じ並び。
Decay の x で Shift+F12 を押すと参照の一覧（3 行）が出て行を押すと飛べ、F2 で pos に変えると全出現が変わって検査が通り（予約語は断られ、Ctrl+Z 1 回で戻る。S161）。クラス名も同じ（`extends Base` の Base を Core に変えると宣言・end・extends が変わる。S166）。
FMU を開く（S229）: 「開く」で .fmu を選ぶと、ブラウザは「Desktop だけ」と理由を出す。Desktop（偽の window.__TAURI__）は警告つきの設定フォーム（S236。終了時刻・刻み・予算・種類が request に乗り、FMU ごとに残る）のあと結果のタブにプロットを出し、やめれば回さず、FMU の失敗は理由を出す。
`.mo` を開く（ファイル選択）とエディタに入りモデル名が本文から付いて検査が通り、「保存」でダウンロードされ、再読み込みで localStorage から戻る（S141）。
名前の上にマウスを置くと宣言の 1 行が出る（Decay の x は `Real x`、ChuaCircuit の Resistor は見出しと説明文字列。S149）。
タブ（S151）: 開くたびにタブが増え、切り替えで本文・モデル名・診断が戻り、再読み込みでタブが戻り、閉じるボタンで減り（未保存なら confirm 1 つ。最後の 1 つは閉じられない）、「最近使ったファイル」から開き直せる。
結果の位置（S170）: 1280x800 と 390 px 幅で、実行の進捗・止める・失敗の理由が最初の画面の中に出て、終わるとプロットの上端が画面の中に入り、利用者が戻したスクロールは引き戻さない。
開く・モデル名（S170）: 2 モデルのファイルは select で選び直せて再読み込みで残り、未保存の編集があるタブと同じ名前を開き直しても編集が残る。ライブラリのモデル名を打ったときは食い違いの案内。
キーボードだけ（S175）: マウスを使わず Tab・Enter・Space とショートカットで、検査、診断と参照の行からエディタへ、実行（プロットの canvas の `aria-label` に信号名と時間の範囲）、止める（Ctrl+.）、保存（Ctrl+S）まで通る。フォーカスの輪郭が見える。
エディタの Tab（S188）: Tab は空白 2 つの字下げ（選んだ行も）・Shift+Tab で戻る。補完の一覧が開いているときだけ Tab は候補の確定（S196）。出る手は Esc のあと Tab / Shift+Tab と Ctrl+M。説明は aria-describedby と画面。
初回の案内（S179）: 保存したタブが無い初回だけ「はじめに」の 3 手順（検査・実行のショートカットつき）が出て、閉じると再読み込みでも出ない（smoke では出ない）。スキップリンクは Tab 1 回でエディタへ・2 回で結果へ。タブは矢印・Home・End で移り（roving tabindex）、未保存の印（●）は編集で付いて再読み込みをまたぎ保存で消え、未保存のとき beforeunload が出る。
ダークモード（S152）: `color_scheme="dark"` で開くと背景が暗く文字が明るい（body・エディタ・診断・プロット・graph。プロットの軸の文字は canvas に描くので明るい画素を数える）。テーマの select で「明」にすると戻り、再読み込みで選択が残り、「OS に合わせる」で暗に戻る。
FMU を書き出す（S144）: ブラウザの wasm が作ったソースの FMU がダウンロードされ、modelDescription.xml と sources/ がある。S154 から FMI の版（2.0 / 3.0）を select で選ぶ。
実行の設定（S142）: 終了時刻 2 とソルバ dopri45 で Decay を実行して x(2) が e⁻²、DecayK は experiment の Tolerance が初期値（出どころ experiment）に出て、k = 2 の上書きで x(1) が e⁻²。名前=値でない行は実行しない。
ライブラリの木で Modelica → Electrical → Analog → Examples → ChuaCircuit と開き、そのファイルがエディタに開いて検査が通ることも確かめる（S127）。
別のページで MSL.json を構文の誤りのファイルを含むライブラリに差し替え、読み飛ばしの警告 GM-W1001 の行と notes のファイル名が出ることを確かめる（S071）。
`?smoke=1` で起動すると Decay の検査と実行の結果が console に `GM-SMOKE check=ok simulate=ok x1=… msl=… ready_ms=…` の 1 行で出ること（S100、android.yml の smoke。時刻は S125）。MSL.json があれば msl=ok（MSL を読んで ChuaCircuit の検査が error 0。S103）。
UI の操作の項目 `tree= fmt= graph= cancel= settings= plot= def=`（S147。Desktop と Android の smoke と同じ。MSL.json が無ければ tree・graph は skip）も全部 ok で `failure=` が無いこと。
console の error（CSP の違反を含む）と、配った origin の外への要求は失敗（S093）。
使い心地（S228）: 実行中の Esc で止まる（入力欄の Esc と補完の一覧を閉じる Esc では止まらない）。タブは Ctrl+Shift+PageUp / PageDown とドラッグで並べ替わり、開いているタブは同じまま、順序が自動保存に乗って再読み込みで戻る。実行の設定はモデル名ごとに残り（`globe-mode.sim`）、切り替えると戻り、「設定を既定に戻す」で消える。開始時刻・刻み幅（固定刻みだけ）・予算の欄が要求に乗り、GM-E10004 が欄の下に出る。
S196（レビューの指摘）: Desktop（偽の window.__TAURI__）で同じ名前の別のファイルが別のタブになり保存が取り違えない。ライブラリのタブは読み取り専用（打てず案内が出る。整形・リネームは断る）。タブを切り替えるとリネームの入力欄・参照の一覧・最後の値が残らない。実行中は F12 に入らず進捗の行は role を持たない。自動保存は遅らせても離れる前に書き、書けなければ 1 文出す。別のウィンドウの保存を知らせる。Tab は補完の一覧が開いていれば確定。Modelica.Units.SI.Vol で Voltage が出る（一覧が 200 件で切れていても問い直す）。
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
SRC = Path(__file__).resolve().parents[2]
RUST_KW = re.findall(r'"(\w+)"', re.search(r"pub const KEYWORDS.*?\];", (SRC / "Rust/mod/globe-mode-fmt/src/lib.rs").read_text(), re.S).group(0))
SYN = 'model S\n  Real x = 1.5; // c\n  String s = "t";\nend S;\n'  # 色付けの試験（S163）
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
# 進捗と止める（S133）: 1e6 s の振動（実行は止めるまで終わらない。CI の速い機械では 1000 s だと F12 や画面の確認の待ちの間に終わる。S201・#71）
LONG = "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation(experiment(StopTime = 1e6, Interval = 1));\nend Long;\n"
# 実行の設定（S142）: パラメータ k を上書きできる。experiment の Tolerance が初期値に出る
DECAY_K = (
    "model DecayK\n  parameter Real k = 1 \"減衰の速さ\";\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -k * x;\n"
    "  annotation(experiment(StopTime = 3, Tolerance = 1e-8));\nend DecayK;\n"
)
# 既定で描く信号（S190）: 状態 x・v と代数変数 e。既定は状態の 2 本
SPRING = "model Spring\n  Real x(start = 1, fixed = true);\n  Real v(start = 0, fixed = true);\n  Real e;\nequation\n  der(x) = v;\n  der(v) = -x;\n  e = 0.5 * (x * x + v * v);\nend Spring;\n"
# 再実行で選び直しを保つ（S201）: Spring にパラメータ k と、信号が無くなった版（e を消す）、全部変わった版
SPRING_K = SPRING.replace("Real e;", "parameter Real k = 1;\n  Real e;").replace("der(v) = -x", "der(v) = -k * x")
SPRING_NO_E = "model Spring\n  Real x(start = 1, fixed = true);\n  Real v(start = 0, fixed = true);\nequation\n  der(x) = v;\n  der(v) = -x;\nend Spring;\n"
SPRING_RENAMED = "model Spring\n  Real p(start = 1, fixed = true);\n  Real q(start = 0, fixed = true);\nequation\n  der(p) = q;\n  der(q) = -p;\nend Spring;\n"
CHUA = "Modelica.Electrical.Analog.Examples.ChuaCircuit"
# 未対応と強行（S173）: OVER は初期条件が余る（GM-E8001。強行で解ける）、DIV は 1 / 0 のパラメータで Rumoca が止まる（GM-U0001。強行しても進まない）
OVER = "model Over\n  Real x(start = 1, fixed = true);\ninitial equation\n  x = 1;\nequation\n  der(x) = -x;\nend Over;\n"
DIV = "model Div\n  parameter Real p = 1 / 0;\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Div;\n"
FIRST = "Modelica.Mechanics.Rotational.Examples.First"  # 信号に `$formal_derivative.*` と `$state_coordinates[*]` が混じる（S169）
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


def contrast(a: str, b: str) -> float:
    """WCAG のコントラスト比（CSS の rgb(...) 2 つ）"""

    def lum(css: str) -> float:
        c = [int(float(v)) / 255 for v in re.findall(r"[\d.]+", css)[:3]]
        c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
        return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]

    hi, lo = sorted((lum(a), lum(b)), reverse=True)
    return (hi + 0.05) / (lo + 0.05)


def check_syntax(pg) -> None:
    """構文の色付け（S163）: 予約語・型・数値・文字列・コメントに別々の色のクラスが付き、エディタの地に対して読める（4.5 以上）。pg のエディタに model / Real / 数値 / "文字列" / // コメント がある前提"""
    want = {"kw": "model", "type": "Real", "num": "", "str": '"', "cmt": "//"}
    look = "(e) => [getComputedStyle(e).color, getComputedStyle(document.querySelector('.cm-editor')).backgroundColor]"
    colors = {}
    for cls, head in want.items():
        el = pg.locator(f".cm-content .cm-mo-{cls}").first
        expect(el).to_be_visible()
        assert el.inner_text().startswith(head), (cls, el.inner_text())
        colors[cls], bg = el.evaluate(look)
        assert contrast(colors[cls], bg) >= 4.5, (cls, colors[cls], bg)
    assert len(set(colors.values())) == len(want), colors
    assert all(t in RUST_KW for t in pg.locator(".cm-content .cm-mo-kw").all_inner_texts()), "予約語でない語に予約語の色が付いている"


# Desktop の dialog / fs / invoke の偽物（S196）。TauriBackend を通した App の保存先の持ち方を、実機なしで確かめる
FAKE_TAURI = """
(() => {
  const fake = (window.__fake = { files: {}, opens: [], saves: [], writes: [], scope: new Set() });
  const listen = async () => () => {};
  const rpc = (window.__rpc = { "globe/models": [], "globe/lint": [], "globe/completion": [], "globe/solvers": [], "globe/tunableParameters": { parameters: [] } });
  window.__TAURI__ = {
    core: { invoke: async (cmd, args) => (cmd === "libraries" ? [] : cmd === "bench" ? false : cmd === "read_recent" ? fake.files[args.path] : cmd === "write_recent" ? (fake.writes.push([args.path, args.text]), (fake.files[args.path] = args.text), null) : cmd === "rpc" ? (rpc[args.method] ?? null) : null) },
    event: { listen },
    // 本物の fs プラグインは dialog で選んだパスにしか許可を与えない（#67。以前の偽物はどのパスも受けて、欠陥を隠していた）
    dialog: { open: async () => { const p = fake.opens.shift() ?? null; if (p) fake.scope.add(p); return p; }, save: async () => { const p = fake.saves.shift() ?? null; if (p) fake.scope.add(p); return p; } },
    fs: { readTextFile: async (p) => fake.files[p], writeTextFile: async (p, t) => { if (!fake.scope.has(p)) throw new Error("forbidden path: " + p); fake.writes.push([p, t]); fake.files[p] = t; }, writeFile: async () => {} },
  };
})();
"""


def review_fixes(browser, origin: str, errors: list[str]) -> None:
    """S196: UI のレビューの指摘（Desktop の保存先の取り違え、ライブラリのタブの読み取り専用、タブを切り替えたときの残り、保存の遅延と失敗、Tab での補完の確定ほか）。"""
    has_msl = (DIST / "msl/MSL.json").exists()

    def new_page(ctx=None, dialog=lambda d: d.accept(), **kw):
        pg = (ctx or browser).new_page(**kw)
        pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("dialog", dialog)
        return pg

    def ready(pg) -> None:
        pg.goto(origin)
        pg.get_by_role("button", name="検査").wait_for()

    def open_in(pg, name: str, body: str) -> None:
        with pg.expect_file_chooser() as ch:
            pg.get_by_role("button", name="開く").click(force=True)
        ch.value.set_files(files=[{"name": name, "mimeType": "text/plain", "buffer": body.encode()}])

    def stored(pg) -> dict:
        return json.loads(pg.evaluate("localStorage.getItem('globe-mode.tabs')"))

    # 1. Desktop: 同じ名前の別のファイルを開いても、保存はそのタブが本文を受け取ったパスに書く
    asked: list[str] = []
    answer = {"ok": True}  # confirm への答え（#70 の確かめで切り替える）

    def on_dialog(d) -> None:
        asked.append(d.message)
        d.accept() if answer["ok"] else d.dismiss()

    dt = new_page(dialog=on_dialog)
    dt.add_init_script(FAKE_TAURI)
    ready(dt)
    P1, P2 = "model P1\nend P1;\n", "model P2\nend P2;\n"

    def desktop_open(path: str, text: str) -> None:
        dt.evaluate("([p, t]) => { __fake.files[p] = t; __fake.opens.push(p); }", [path, text])
        dt.get_by_role("button", name="開く", exact=True).click(force=True)

    desktop_open("/d1/package.mo", P1)
    expect(dt.locator(".cm-content")).to_contain_text("model P1", timeout=30_000)
    dt.locator(".cm-content").click()
    dt.keyboard.press("Control+End")
    dt.keyboard.type("// e1")
    desktop_open("/d2/package.mo", P2)  # d1 のタブは未保存。別のファイルなので、別のタブに開く
    expect(dt.locator(".cm-content")).to_contain_text("model P2", timeout=30_000)
    dt.locator(".cm-content").click()
    dt.keyboard.press("Control+s")
    expect(dt.locator("[data-saved]")).to_contain_text("/d2/package.mo")
    assert dt.evaluate("__fake.writes") == [["/d2/package.mo", P2]], dt.evaluate("__fake.writes")
    dt.locator("[role=tab][title=\"package.mo\"]").click()  # d1（key は package.mo）
    expect(dt.locator(".cm-content")).to_contain_text("// e1")
    dt.locator(".cm-content").click()
    dt.keyboard.press("Control+s")
    expect(dt.locator("[data-saved]")).to_contain_text("/d1/package.mo")
    assert dt.evaluate("__fake.writes")[1] == ["/d1/package.mo", P1 + "// e1"], dt.evaluate("__fake.writes")
    assert dt.get_by_role("tab").count() == 3
    # 同じパスをもう一度開いても、タブは増えず、未保存の編集は残る
    desktop_open("/d1/package.mo", P1)
    dt.wait_for_timeout(300)
    assert dt.get_by_role("tab").count() == 3
    # 別名で保存（ダイアログで選び直す）と、その先に書く
    dt.evaluate("__fake.saves.push('/d3/other.mo')")
    dt.get_by_role("button", name="名前を付けて保存").click(force=True)
    expect(dt.locator("[data-saved]")).to_contain_text("/d3/other.mo")
    dt.keyboard.press("Control+s")
    dt.wait_for_timeout(300)
    assert dt.evaluate("__fake.writes")[-1][0] == "/d3/other.mo", dt.evaluate("__fake.writes")
    # #67: dialog を通していない path（再起動後や最近使ったファイル）でも、Ctrl+S は Rust の write_recent で書ける
    desktop_open("/d4/recent.mo", "model R\nend R;\n")
    expect(dt.locator(".cm-content")).to_contain_text("model R", timeout=30_000)
    dt.evaluate("__fake.scope.clear()")
    dt.locator(".cm-content").click()
    dt.keyboard.press("Control+End")
    dt.keyboard.type("// e4")
    dt.keyboard.press("Control+s")
    expect(dt.locator("[data-saved]")).to_contain_text("/d4/recent.mo")
    assert dt.evaluate("__fake.writes")[-1] == ["/d4/recent.mo", "model R\nend R;\n// e4"], dt.evaluate("__fake.writes")
    # #70: 開いたあとに外で書き換わったファイルは、確かめてから上書きする（取り消せば書かず、承知すれば書く）
    asked.clear()
    answer["ok"] = False
    desktop_open("/d5/ext.mo", "model E\nend E;\n")
    expect(dt.locator(".cm-content")).to_contain_text("model E", timeout=30_000)
    dt.evaluate("__fake.files['/d5/ext.mo'] = 'model E // 外で直した\\nend E;\\n'")
    dt.locator(".cm-content").click()
    dt.keyboard.press("Control+End")
    dt.keyboard.type("// e5")
    n = len(dt.evaluate("__fake.writes"))
    dt.keyboard.press("Control+s")
    dt.wait_for_timeout(500)
    assert len(asked) == 1 and "外で書き換わっている" in asked[0], asked
    assert len(dt.evaluate("__fake.writes")) == n, "取り消したのに書いた"
    answer["ok"] = True
    dt.keyboard.press("Control+s")
    expect(dt.locator("[data-saved]")).to_contain_text("/d5/ext.mo")
    assert dt.evaluate("__fake.writes")[-1][0] == "/d5/ext.mo" and len(asked) == 2, asked
    dt.keyboard.press("Control+s")  # 自分で書いたあとは、確かめない
    dt.wait_for_timeout(300)
    assert len(asked) == 2, asked
    dt.close()
    print("Desktop の保存先: 同じ名前の別のファイル（/d1/package.mo と /d2/package.mo）は別のタブで、それぞれのパスに保存される")

    # #68: Android の Tauri（dialog / fs が無い）でも、プロットの CSV はブラウザと同じダウンロードで書き出せる（TypeError にならない）
    an = new_page(user_agent="Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/120 Mobile Safari/537.36")
    an.add_init_script("window.__TAURI__ = { core: { invoke: async (c) => (c === 'libraries' ? [] : null) }, event: { listen: async () => () => {} } };")
    ready(an)
    an.get_by_role("button", name="実行", exact=True).click(force=True)
    expect(an.locator(".plot canvas").first).to_be_visible(timeout=120_000)
    with an.expect_download(timeout=10_000) as dl:
        an.get_by_role("button", name="CSV").click(force=True)
    assert dl.value.suggested_filename.endswith(".csv"), dl.value.suggested_filename
    an.close()
    print("Android: dialog / fs が無くても、プロットの CSV がダウンロードで書き出せる")

    # 2. タブを切り替えたとき、リネームの入力欄・参照の一覧・最後の値が前のタブに残らない
    tb = new_page()
    ready(tb)
    open_in(tb, "A.mo", "model A\n  Real a(start = 1, fixed = true);\nequation\n  der(a) = -a;\nend A;\n")
    expect(tb.get_by_role("tab")).to_have_count(2, timeout=30_000)
    tb.locator(".cm-content").click()
    tb.keyboard.press("Control+Home")
    for _ in range(3):
        tb.keyboard.press("ArrowDown")
    tb.keyboard.press("End")
    tb.keyboard.press("ArrowLeft")
    tb.keyboard.press("Shift+F12")
    expect(tb.locator("table.refs")).to_be_visible(timeout=30_000)
    tb.keyboard.press("F2")
    expect(tb.locator("form.rename")).to_be_visible(timeout=30_000)
    tb.get_by_role("button", name="実行", exact=True).click(force=True)
    expect(tb.locator("p.final")).to_be_visible(timeout=120_000)
    tb.get_by_role("tab").nth(0).click()
    for sel in ["table.refs", "form.rename", "p.final"]:
        assert tb.locator(sel).count() == 0, f"前のタブの {sel} が残っている"
    tb.close()
    print("タブの切り替え: 参照の一覧・リネームの入力欄・最後の値が前のタブから残らない")

    # 3. 実行中は定義ジャンプ（F12）に入らない。進捗の行は実行中は role=status にしない（更新のたびに読み上げない）
    bz = new_page()
    ready(bz)
    bz.get_by_label("モデル名").fill("Long")
    bz.locator(".cm-content").click()
    bz.keyboard.press("Control+A")
    bz.keyboard.insert_text(LONG)
    bz.get_by_role("button", name="実行", exact=True).click(force=True)
    expect(bz.locator("p.run")).to_contain_text("integration", timeout=120_000)
    assert bz.locator("p.run").get_attribute("role") is None, "実行中の進捗の行は role を持たない"
    bz.locator(".cm-content").click()
    bz.keyboard.press("Control+Home")
    for _ in range(3):
        bz.keyboard.press("ArrowDown")
    bz.keyboard.press("End")
    for _ in range(14):
        bz.keyboard.press("ArrowLeft")  # `  der(x) = sin(100 * time);` の x のすぐ後ろ
    head = bz.evaluate("document.querySelector('.cm-activeLine').textContent")
    assert "der(x)" in head, head
    bz.keyboard.press("F12")
    bz.wait_for_timeout(1500)
    assert "der(x)" in bz.evaluate("document.querySelector('.cm-activeLine').textContent"), "実行中に定義ジャンプが動いた"
    bz.get_by_role("button", name="止める").click(force=True)
    expect(bz.locator("p.run")).to_have_attribute("data-run", "cancelled", timeout=10_000)
    assert bz.locator("p.run").get_attribute("role") == "status"
    bz.close()
    print("実行中: F12 は動かず、進捗の行は role を持たず、止めたあとは role=status")

    # 4. 保存の遅延: 打鍵ごとには書かず、再読み込み・閉じる直前には必ず書く。書けないときは黙らない
    pr = new_page()
    ready(pr)
    pr.locator(".cm-content").click()
    pr.keyboard.press("Control+End")
    pr.keyboard.type("// late")
    pr.reload()
    pr.get_by_role("button", name="検査").wait_for()
    expect(pr.locator(".cm-content")).to_contain_text("// late")  # 遅らせた書き込みを、離れる前に flush した
    pr.locator(".cm-content").click()
    pr.keyboard.press("Control+End")
    pr.keyboard.type("// bye")
    pr.evaluate("window.dispatchEvent(new Event('pagehide'))")
    assert "// bye" in stored(pr)["tabs"][0]["text"], "pagehide で書いていない"
    pr.evaluate("Object.defineProperty(Storage.prototype, 'setItem', { value() { throw new DOMException('full', 'QuotaExceededError'); } })")
    pr.keyboard.type("x")
    expect(pr.locator("[data-store-note]")).to_contain_text("保存できない", timeout=5_000)
    pr.close()
    print("保存の遅延: 再読み込み・pagehide の前に書き、localStorage に書けなければ 1 文出す")

    # 5. 2 つのウィンドウ: 一方で編集すると、もう一方に注意が出る
    ctx = browser.new_context()
    w1, w2 = new_page(ctx), new_page(ctx)
    ready(w1)
    ready(w2)
    assert w1.locator("[data-other-window]").count() == 0
    w2.locator(".cm-content").click()
    w2.keyboard.type("// w2")
    expect(w1.locator("[data-other-window]")).to_be_visible(timeout=5_000)
    ctx.close()
    print("2 つのウィンドウ: 他方の保存を storage イベントで知らせる")

    # 6. Tab: 補完の一覧が開いていれば候補の確定（Enter と同じ）、閉じていれば字下げ
    tc = new_page()
    ready(tc)
    tc.locator(".cm-content").press("Control+a")
    tc.keyboard.insert_text("mod")
    opt = tc.locator(".cm-tooltip-autocomplete li[aria-selected]")
    expect(opt).to_be_visible(timeout=10_000)
    tc.wait_for_timeout(300)  # CodeMirror は一覧が出て 75 ms 以内の確定を無視する（誤爆よけ）
    tc.keyboard.press("Tab")
    line = tc.eval_on_selector_all(".cm-line", "els => els[0].textContent")
    assert line.startswith("mod") and line != "mod" and not line.startswith(" "), line
    tc.close()

    if not has_msl:
        print("public/msl/MSL.json が無いので、ライブラリのタブと補完の再問い合わせの試験は飛ばす")
        return
    # 7. ライブラリのタブは読み取り専用: 打てず、案内が出て、整形・リネームは断られ、未保存にならない
    lb = new_page()
    ready(lb)
    lb.get_by_role("button", name="MSL を読む").click(force=True)
    expect(lb.locator("p.note[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=120_000)
    pid = "Modelica.Blocks.Examples.PID_Controller"
    lb.get_by_label("モデル名").fill(pid)
    lb.get_by_role("button", name="検査").click(force=True)
    expect(lb.get_by_role("button", name="検査")).to_be_enabled(timeout=120_000)
    expect(lb.locator("[data-stray]")).to_contain_text("ライブラリのもの", timeout=120_000)
    lb.locator("[data-stray]").get_by_role("button", name="ライブラリのファイルを開く").click(force=True)
    expect(lb.locator(".cm-content")).to_contain_text("package Examples", timeout=60_000)
    expect(lb.locator("[data-readonly]")).to_contain_text("読み取り専用")
    before = lb.eval_on_selector(".cm-content", "e => e.textContent")
    lb.locator(".cm-content").click()
    lb.keyboard.type("zzz")
    lb.keyboard.press("Backspace")
    assert lb.eval_on_selector(".cm-content", "e => e.textContent") == before, "読み取り専用のタブに入力できた"
    expect(lb.locator("[data-readonly]")).to_have_class(re.compile(r"\bhit\b"))
    lb.get_by_role("button", name="整形").click(force=True)
    expect(lb.get_by_role("alert")).to_contain_text("読み取り専用")
    assert lb.eval_on_selector(".cm-content", "e => e.textContent") == before
    assert lb.locator(".dirty").count() == 0
    assert all(t["file"] == "Model.mo" for t in stored(lb)["tabs"]), stored(lb)["tabs"]
    lb.get_by_role("tab").nth(0).click()
    expect(lb.locator("[data-readonly]")).to_have_count(0)
    # 8. 補完の一覧が 200 件で切れていても、打ち足すと問い直して Voltage が出る
    lb.locator(".cm-content").click()
    lb.keyboard.press("Control+a")
    lb.keyboard.insert_text("model T\n  Modelica.Units.SI")
    lb.keyboard.type(".")  # 型が 516 個あるので、ドットの直後の一覧は上位 200 件で切れている
    expect(lb.locator(".cm-tooltip-autocomplete li").first).to_be_visible(timeout=30_000)
    lb.wait_for_timeout(1000)
    lb.keyboard.type("Vol")
    expect(lb.locator(".cm-tooltip-autocomplete li", has_text=re.compile(r"^Voltage")).first).to_be_visible(timeout=30_000)
    lb.close()
    print("ライブラリのタブ: 読み取り専用（入力できず、案内、整形は断る、未保存にならない）。Modelica.Units.SI.Vol で Voltage が出る")


def ui_polish(browser, origin: str, errors: list[str]) -> None:
    """S228: 使い心地の 4 点。Esc で止める、タブの並べ替え（ドラッグとキーボード、順序が保存に乗る）、設定のモデルごとの保存、開始時刻・刻み幅・予算の欄。"""

    def new_page():
        pg = browser.new_page()
        pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("dialog", lambda d: d.accept())
        pg.goto(origin)
        pg.get_by_role("button", name="検査").wait_for()
        return pg

    def put(pg, model: str, body: str) -> None:
        pg.get_by_label("モデル名").fill(model)
        pg.locator(".cm-content").focus()
        pg.keyboard.press("Control+A")
        pg.keyboard.insert_text(body)

    # 1. Esc で止める: 入力欄の中では奪わない。補完の一覧が開いていれば閉じるだけ。それ以外は Ctrl+. と同じ
    pg = new_page()
    put(pg, "Long", LONG)
    run = pg.get_by_role("button", name="実行", exact=True)
    running = pg.locator("[data-run=running]")
    run.click(force=True)
    expect(running).to_contain_text("t = ", timeout=120_000)  # 積分が進み出してから（段の名前だけの時点で押すと、止める旗が積分の開始で消されることがあった）
    pg.get_by_label("モデル名").focus()
    pg.keyboard.press("Escape")
    pg.wait_for_timeout(500)
    expect(running).to_have_count(1)  # 入力欄の Esc では止まらない
    pg.locator(".cm-content").focus()
    pg.keyboard.type("Rea")
    expect(pg.locator(".cm-tooltip-autocomplete")).to_be_visible(timeout=30_000)
    pg.keyboard.press("Escape")
    expect(pg.locator(".cm-tooltip-autocomplete")).to_have_count(0)
    expect(running).to_have_count(1)  # 補完の一覧を閉じただけ
    pg.keyboard.press("Escape")
    expect(pg.locator("[data-run=cancelled]")).to_be_visible(timeout=60_000)
    expect(run).to_be_enabled(timeout=60_000)
    pg.keyboard.press("Escape")  # 実行中でなければ何も起きない
    assert pg.get_by_role("alert").count() == 0, pg.get_by_role("alert").all_inner_texts()
    put(pg, "Long", LONG)  # 打った "Rea" を戻す
    run.click(force=True)  # フォーカスが本文以外（body）でも止まる
    expect(running).to_contain_text("t = ", timeout=120_000)  # 積分が進み出してから（段の名前だけの時点で押すと、止める旗が積分の開始で消されることがあった）
    pg.locator("body").evaluate("() => document.activeElement?.blur()")
    pg.keyboard.press("Escape")
    expect(pg.locator("[data-run=cancelled]")).to_be_visible(timeout=60_000)
    print("Esc: 入力欄では止まらず、補完の一覧は閉じるだけ、本文・body では止まる")
    pg.close()

    # 2. タブの並べ替え: Ctrl+Shift+PageUp / PageDown とドラッグ。開いているタブは動かした先でも同じ、順序は保存に乗り再読み込みで戻る
    pg = new_page()
    for name in ("A.mo", "B.mo"):
        with pg.expect_file_chooser() as ch:
            pg.get_by_role("button", name="開く").click(force=True)
        ch.value.set_files(files=[{"name": name, "mimeType": "text/plain", "buffer": f"model {name[0]}\nend {name[0]};\n".encode()}])
        expect(pg.locator("[role=tab][aria-selected=true]")).to_contain_text(name, timeout=30_000)
    tabs = pg.locator("[role=tab]")
    order = lambda: [t.strip(" ●") for t in tabs.all_inner_texts()]
    active = lambda: pg.locator("[role=tab][aria-selected=true]").inner_text().strip(" ●")
    assert order() == ["Model.mo", "A.mo", "B.mo"] and active() == "B.mo", order()
    pg.locator("[role=tab][aria-selected=true]").focus()
    pg.keyboard.press("Control+Shift+PageUp")
    assert order() == ["Model.mo", "B.mo", "A.mo"] and active() == "B.mo", order()
    expect(pg.locator("[role=tab][aria-selected=true]")).to_be_focused()  # フォーカスも動いたタブについていく
    pg.keyboard.press("Control+Shift+PageUp")
    pg.keyboard.press("Control+Shift+PageUp")  # 端では動かない
    assert order() == ["B.mo", "Model.mo", "A.mo"] and active() == "B.mo", order()
    pg.keyboard.press("Control+Shift+PageDown")
    assert order() == ["Model.mo", "B.mo", "A.mo"], order()
    expect(pg.locator("p[aria-live=polite]")).to_contain_text("B.mo を 2 番目へ移した")
    pg.locator(".tab").nth(2).drag_to(pg.locator(".tab").nth(0))  # A.mo を先頭へ
    assert order() == ["A.mo", "Model.mo", "B.mo"] and active() == "B.mo", order()
    pg.wait_for_timeout(600)  # 保存は 300 ms のまとめ書き
    saved = json.loads(pg.evaluate("localStorage.getItem('globe-mode.tabs')"))
    assert [t["file"] for t in saved["tabs"]] == ["A.mo", "Model.mo", "B.mo"] and saved["active"] == 2, saved
    pg.reload()
    pg.get_by_role("button", name="検査").wait_for()
    assert order() == ["A.mo", "Model.mo", "B.mo"] and active() == "B.mo", order()
    print(f"タブの並べ替え: キーボードとドラッグで {order()}、再読み込みで戻る")
    pg.close()

    # 3. 設定のモデルごとの保存: モデルを切り替えると、そのモデルの設定に戻る。「既定に戻す」で消える
    pg = new_page()
    stop, start, step, budget, solver = (pg.get_by_label(n, exact=True) for n in ("終了時刻", "開始時刻", "刻み幅", "予算（秒）", "ソルバ"))
    reset = pg.get_by_role("button", name="設定を既定に戻す")
    sim = lambda: json.loads(pg.evaluate("localStorage.getItem('globe-mode.sim')") or "{}")
    expect(step).to_be_disabled()  # 刻み幅は固定刻みのソルバのときだけ
    expect(reset).to_be_disabled()
    stop.fill("7")
    solver.select_option("rk4")
    expect(step).to_be_enabled()
    step.fill("0.5")
    assert sim()["Decay"] == {"solver": "rk4", "text": {"stop_time": "7", "step": "0.5"}, "parameters": "", "picked": {}}, sim()
    pg.get_by_label("モデル名").fill("Other")
    expect(stop).to_have_value("")
    expect(solver).to_have_value("")
    expect(step).to_be_disabled()
    stop.fill("3")
    pg.get_by_label("モデル名").fill("Decay")
    expect(stop).to_have_value("7")
    expect(solver).to_have_value("rk4")
    expect(step).to_have_value("0.5")
    pg.reload()
    pg.get_by_role("button", name="検査").wait_for()
    expect(stop).to_have_value("7")  # 再読み込みのあとも
    expect(solver).to_have_value("rk4")
    reset.click()
    expect(stop).to_have_value("")
    expect(solver).to_have_value("")
    assert "Decay" not in sim() and sim()["Other"]["text"] == {"stop_time": "3"}, sim()
    pg.get_by_label("モデル名").fill("Other")
    expect(stop).to_have_value("3")  # ほかのモデルは残る
    print(f"設定のモデルごとの保存: {sim()}")

    # 4. 開始時刻・刻み幅・予算の欄: 開始時刻 1・終了時刻 3 は実行に使われ、刻み幅が小さすぎれば GM-E10004 が欄の近くに出る。予算は壁時計で打ち切る
    pg.get_by_label("モデル名").fill("Decay")
    start.fill("1")
    stop.fill("3")
    run = pg.get_by_role("button", name="実行", exact=True)
    run.click(force=True)
    expect(run).to_be_enabled(timeout=300_000)
    last = pg.locator("[data-last]").inner_text()
    assert "開始 1" in last and "終了 3（指定）" in last, last
    solver.select_option("rk4")
    step.fill("1e-9")  # 区間 2 s / 1e-9 = 20 億点 > 1000 万（S212）
    run.click(force=True)
    expect(run).to_be_enabled(timeout=300_000)
    refusal = pg.locator("[data-refusal]")
    expect(refusal).to_contain_text("GM-E10004")
    assert pg.locator(".settings [data-refusal]").count() == 1
    step.fill("0.5")
    budget.fill("-1")
    put(pg, "Long", LONG)
    stop.fill("")
    start.fill("")
    solver.select_option("")
    pg.get_by_label("モデル名").fill("Long")
    budget.fill("2")
    run.click(force=True)
    expect(run).to_be_enabled(timeout=300_000)
    text = pg.locator("table.diagnostics").inner_text() + pg.locator(".failure").all_inner_texts().__str__()
    assert "GM-E11005" in text, text  # 予算（2 秒）で打ち切られた
    print("新しい欄: 開始時刻・刻み幅・予算が要求に乗る")
    pg.close()


def open_results_file(browser, origin: str, errors: list[str]) -> None:
    """S222: 「開く」で .csv と .mat（MAT v4）の結果のファイルを開くと、結果のタブにプロットが出る。エディタは隠れ、モデルのタブに戻ると結果は消える。読めないファイルは理由が出る。"""
    pg = browser.new_page()
    pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.on("dialog", lambda d: d.accept())
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()

    def choose(name: str, data: bytes) -> None:
        with pg.expect_file_chooser() as ch:
            pg.get_by_role("button", name="開く").click(force=True)
        ch.value.set_files(files=[{"name": name, "mimeType": "application/octet-stream", "buffer": data}])

    plot = pg.locator(".plot")
    choose("decay.csv", b"time,x,y\n0,1,2\n1,0.5,NaN\n2,0.25,0\n")
    expect(plot).to_have_attribute("data-series", "2", timeout=60_000)
    expect(pg.get_by_role("tab", name="結果 decay.csv")).to_have_attribute("aria-selected", "true")
    expect(pg.locator("p[data-results]")).to_contain_text("3 点")
    expect(pg.locator("#editor")).to_be_hidden()
    expect(pg.get_by_label("モデル名")).to_have_value("decay")
    pg.get_by_role("button", name="保存").click(force=True)
    expect(pg.get_by_role("alert")).to_contain_text("保存できない")
    # .mat（omc の実物の MAT v4）。同じ「開く」で、別の結果のタブになる
    mat = (SRC / "Rust/mod/globe-mode-sim/tests/data/omc_decay_res.mat").read_bytes()
    choose("omc_decay_res.mat", mat)
    expect(pg.get_by_role("tab", name="結果 omc_decay_res.mat")).to_have_attribute("aria-selected", "true")
    expect(plot).to_have_count(1)
    assert int(plot.get_attribute("data-series")) >= 1
    assert pg.get_by_role("button", name="CSV").count() >= 1 and pg.get_by_role("button", name="PNG").count() >= 1
    # モデルのタブに戻ると結果は消え、エディタが見える。結果のタブに戻ると列が戻る
    pg.get_by_role("tab").nth(0).click()
    expect(plot).to_have_count(0)
    expect(pg.locator("#editor")).to_be_visible()
    pg.get_by_role("tab", name="結果 decay.csv").click()
    expect(plot).to_have_attribute("data-series", "2")
    # 読めない結果は理由を出し、タブは増えない
    n = pg.get_by_role("tab").count()
    choose("bad.csv", b"x,y\n1,2\n")
    expect(pg.get_by_role("alert")).to_contain_text("bad.csv", timeout=10_000)
    assert pg.get_by_role("tab").count() == n
    pg.close()
    print("結果を開く: .csv と .mat が結果のタブでプロットされ、エディタは隠れ、タブの切り替えで列が戻り、保存と読めないファイルは理由が出る")


def docs_panel(browser, origin: str, errors: list[str]) -> None:
    """S232: 説明パネル。カーソルのあるクラスの Documentation（Rust が生の HTML を引く）が、白リストで洗われて出る。
    script・onerror・javascript: は出ず走らず、表と改訂履歴は出る。Documentation の無いクラスは「無い」。暗でも読める。MSL があれば PID を木（検索）で選ぶと本文が出る。"""
    html = (
        '<html><p>Hello <b>bold</b></p><script>window.__pwned = 1</script><img src=\\"x\\" onerror=\\"window.__pwned = 2\\" alt=\\"picalt\\">'
        '<a href=\\"javascript:window.__pwned = 3\\" onclick=\\"window.__pwned = 4\\">js</a> <a href=\\"https://example.com/\\">ok</a>'
        '<table><tr><td>cell</td></tr></table></html>'
    )
    body = (
        'model Docs "d"\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\n'
        f'  annotation(Documentation(info = "{html}", revisions = "<ul><li>rev1</li></ul>"));\nend Docs;\n'
        'model Plain\n  Real y;\nequation\n  y = 1;\nend Plain;\n'
    )
    pg = browser.new_page(color_scheme="dark")
    # DOMParser の不活性な文書でも、onerror= などの属性を読むと CSP が「インラインのイベントハンドラ」を console に報告する（走りはしない）。
    # この悪意のある本文のページでは、そのときの報告だけを別に集める（ほかの error は今までどおり失敗）
    csp: list[str] = []
    pg.on("console", lambda m: m.type == "error" and (csp if "Content Security Policy" in m.text and "inline event handler" in m.text else errors).append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    with pg.expect_file_chooser() as ch:
        pg.get_by_role("button", name="開く").click(force=True)
    ch.value.set_files(files=[{"name": "Docs.mo", "mimeType": "text/plain", "buffer": body.encode()}])
    pg.locator(".cm-content").focus()
    pg.keyboard.press("Control+Home")
    pg.keyboard.press("ArrowDown")  # model Docs の中
    info = pg.locator("[data-docs-info]")
    expect(info).to_contain_text("Hello bold", timeout=60_000)
    expect(pg.locator(".docs .cls")).to_have_text("Docs")
    expect(info.locator("td")).to_have_text("cell")
    expect(info).to_contain_text("picalt")  # img は alt の文字になる
    assert info.locator("script, img, iframe").count() == 0
    assert pg.evaluate("document.querySelectorAll('.docs [onerror], .docs [onclick], .docs [style], .docs [src]').length") == 0
    links = info.locator("a")
    assert links.count() == 1 and links.get_attribute("href") == "https://example.com/" and links.get_attribute("target") == "_blank" and "noopener" in links.get_attribute("rel")
    assert pg.evaluate("window.__pwned") is None  # script も onerror も javascript: も走っていない
    pg.locator(".docs summary").click()
    expect(pg.locator("[data-docs-revisions]")).to_contain_text("rev1")
    colors = info.evaluate("(e) => [getComputedStyle(e).color, getComputedStyle(document.body).backgroundColor]")
    assert contrast(*colors) >= 4.5, colors  # 暗でも読める（色は theme.css の変数）
    # Documentation の無いクラスへカーソルを動かすと「無い」（クラス名は出る）。クラスの外では何も引かない
    pg.locator(".cm-content").focus()
    pg.keyboard.press("Control+End")
    pg.keyboard.press("ArrowUp")
    pg.keyboard.press("ArrowUp")
    expect(pg.locator(".docs .cls")).to_have_text("Plain", timeout=30_000)
    expect(pg.locator("[data-docs-empty]")).to_have_text("Documentation が無い")
    assert info.count() == 0
    pg.close()
    print("説明パネル: カーソルのクラスの Documentation が出る（script・onerror・javascript: は出ず走らない、表と改訂履歴は出る、無ければ「無い」、暗でも読める）")
    if not (DIST / "msl/MSL.json").exists():
        print("public/msl/MSL.json が無いので、PID の説明の試験は飛ばす")
        return
    pg = browser.new_page()
    pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.goto(origin)
    pg.get_by_role("button", name="MSL を読む").click(force=True)
    expect(pg.locator("p[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=300_000)
    pg.get_by_label("ライブラリを検索").fill("Modelica.Blocks.Continuous.PID")
    expect(pg.locator("#library-hits [role=option]").first).to_contain_text("Modelica.Blocks.Continuous.PID", timeout=30_000)  # 候補が出てから Enter（先に押すと開かない）
    pg.get_by_label("ライブラリを検索").press("Enter")
    expect(pg.locator(".docs .cls")).to_have_text("Modelica.Blocks.Continuous.PID", timeout=60_000)
    expect(pg.locator("[data-docs-info]")).to_contain_text("text-book version of a PID controller")
    expect(pg.locator("[data-docs-info] table")).to_have_count(1)
    pg.close()
    print("説明パネル: MSL の PID を木（検索）で開くと、本文と表が出る")


def fmu_runtime_select(browser, origin: str, errors: list[str]) -> None:
    """S239: FMU の実行時（P5a / P5b）の選択。ブラウザには選択肢が無い（cdylib を作れない）。Desktop（偽の window.__TAURI__）は既定が p5a で、
    P5b を選ぶと globe/exportFmu の request に runtime: "p5b" が乗り、書き出したあとの文が P5b のものになる。"""

    def page():
        pg = browser.new_page()
        pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("dialog", lambda d: d.accept())
        return pg

    pg = page()
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    assert pg.get_by_label("実行時").count() == 0
    pg.close()

    dt = page()
    dt.add_init_script(FAKE_TAURI)
    dt.goto(origin)
    dt.get_by_role("button", name="検査").wait_for()
    dt.evaluate(
        """() => {
      const reqs = (window.__fmuExport = []);
      const inv = __TAURI__.core.invoke;
      __TAURI__.core.invoke = async (cmd, a) => { if (cmd === "rpc" && a.method === "globe/exportFmu") reqs.push(a.params); return inv(cmd, a); };
      __rpc["globe/exportFmu"] = { path: "/o/Decay-p5b-fmi2.fmu" };
    }"""
    )
    sel = dt.get_by_label("実行時")
    assert sel.locator("option").all_inner_texts() == ["P5a（Rumoca の C のソース）", "P5b（自前の実行時。cargo が要る）"]
    for runtime in ["p5a", "p5b"]:
        sel.select_option(runtime)
        dt.evaluate("__fake.saves.push('/o/x.fmu')")
        dt.get_by_role("button", name="FMU を書き出す").click(force=True)
        expect(dt.locator("p[data-saved]")).to_contain_text("FMU を書き出した", timeout=10_000)
        assert dt.evaluate("__fmuExport").pop()["runtime"] == runtime
        has_p5b = "P5b" in dt.locator("p[data-saved]").inner_text()
        assert has_p5b == (runtime == "p5b"), dt.locator("p[data-saved]").inner_text()
    dt.close()
    print("FMU の実行時: ブラウザに選択肢は無く、Desktop は既定が p5a で、P5b を選ぶと globe/exportFmu の request に runtime: p5b が乗る")


def open_fmu_file(browser, origin: str, errors: list[str]) -> None:
    """S229: 「開く」で .fmu を選ぶ。ブラウザは「Desktop だけ」と理由を出す。Desktop（偽の window.__TAURI__）は警告と実行の設定のフォーム（S236。
    確認ダイアログの代わり）のあと sidecar（偽の rpc）が回した列を結果のタブに出し、やめれば何も回さない。フォームの終了時刻・刻み・予算・種類が
    globe/fmuRun の request に乗り、FMU のパスごとに残る。実行の失敗は理由を出す。"""
    asked: list[str] = []

    def on_dialog(d) -> None:
        asked.append(d.message)
        d.accept()

    def page():
        pg = browser.new_page()
        pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("dialog", on_dialog)
        return pg

    # ブラウザ: zip を読まず、理由だけ
    pg = page()
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    with pg.expect_file_chooser() as ch:
        pg.get_by_role("button", name="開く").click(force=True)
    ch.value.set_files(files=[{"name": "d.fmu", "mimeType": "application/octet-stream", "buffer": b"PK\x03\x04"}])
    expect(pg.get_by_role("alert")).to_contain_text("Desktop だけ", timeout=10_000)
    assert pg.get_by_role("tab").count() == 1 and not asked, asked
    pg.close()

    # Desktop
    dt = page()
    dt.add_init_script(FAKE_TAURI)
    dt.goto(origin)
    dt.get_by_role("button", name="検査").wait_for()
    dt.evaluate(
        """() => {
      const calls = (window.__fmuCalls = []);
      const reqs = (window.__fmuReq = []);
      const inv = __TAURI__.core.invoke;
      __TAURI__.core.invoke = async (cmd, a) => { if (cmd === "rpc" && a.method === "globe/fmuRun") reqs.push(a.params.request); return inv(cmd, a); };
      __rpc["globe/fmuInfo"] = { model_name: "Dahlquist", fmi_version: "3.0", interfaces: [{ kind: "CoSimulation", model_identifier: "Dahlquist" }], warning: "警告: FMU の中のネイティブコードを別プロセスで実行する" };
      Object.defineProperty(__rpc, "globe/fmuRun", { get() { calls.push("run"); if (window.__fmuFail) throw new Error("FMU のワーカー（別プロセス）: 結果を返す前に終わった（signal: 11）"); return { layout: { names: ["time", "x"], points: 3, defaults: ["x"] }, columns: [[0, 1, 2], [1, 0.5, 0.25]], me: null }; } });
    }"""
    )
    form = dt.locator("[data-fmu-form]")

    def desktop_open() -> None:
        dt.evaluate("__fake.opens.push('/d1/Dahlquist.fmu')")
        dt.get_by_role("button", name="開く", exact=True).click(force=True)

    desktop_open()
    expect(form).to_be_visible(timeout=10_000)
    text = form.inner_text()
    assert "ネイティブコード" in text and "Dahlquist" in text and "CoSimulation" in text, text
    form.get_by_role("button", name="やめる").click()
    expect(form).to_have_count(0)
    desktop_open()  # Esc でもやめられる
    expect(form).to_be_visible()
    dt.keyboard.press("Escape")
    expect(form).to_have_count(0)
    dt.wait_for_timeout(300)
    assert dt.evaluate("__fmuCalls.length") == 0 and dt.get_by_role("tab").count() == 1  # やめれば回さない
    # S236: 設定が request に乗る。種類の選択肢はこの FMU が持つものだけ（CS）
    desktop_open()
    form.get_by_label("終了時刻").fill("2")
    form.get_by_label("刻み", exact=True).fill("0.1")
    form.get_by_label("予算（秒）").fill("30")
    form.get_by_label("FMU の種類").select_option("cs")
    assert form.get_by_label("FMU の種類").locator("option").all_inner_texts() == ["既定", "Co-Simulation（CS）"]
    form.get_by_label("終了時刻").fill("abc")  # 数でない欄は実行できず、理由が出る
    expect(form.get_by_role("button", name="実行")).to_be_disabled()
    expect(form.get_by_role("alert")).to_contain_text("終了時刻「abc」は数でない")
    form.get_by_label("終了時刻").fill("2")
    form.get_by_role("button", name="実行").click()
    expect(dt.get_by_role("tab", name="結果 Dahlquist.fmu")).to_have_attribute("aria-selected", "true", timeout=30_000)
    expect(dt.locator(".plot")).to_have_attribute("data-series", "1")
    expect(dt.locator("p[data-results]")).to_contain_text("3 点")
    expect(dt.locator("p[data-results]")).to_contain_text("再読み込みで閉じる")  # FMU の結果は path を持たず、再読み込みで戻らない（ネイティブコードを黙って回し直さない）
    assert dt.evaluate("__fmuCalls.length") == 1
    assert dt.evaluate("__fmuReq") == [{"kind": "cs", "stop_time": 2, "step": 0.1, "timeout": 30}], dt.evaluate("__fmuReq")
    # 同じ FMU を開き直すと前の設定が入っている。「既定に戻す」で消えて、request は空
    desktop_open()
    expect(form.get_by_label("予算（秒）")).to_have_value("30")
    expect(form.get_by_label("FMU の種類")).to_have_value("cs")
    form.get_by_role("button", name="設定を既定に戻す").click()
    expect(form.get_by_label("予算（秒）")).to_have_value("")
    form.get_by_role("button", name="実行").click()
    expect(form).to_have_count(0, timeout=30_000)
    assert dt.evaluate("__fmuReq")[-1] == {}, dt.evaluate("__fmuReq")
    # S236: 最近使ったファイルから開いても同じフォーム
    dt.get_by_label("最近使ったファイル").select_option(label="Dahlquist.fmu")
    expect(form).to_be_visible()
    form.get_by_role("button", name="やめる").click()
    # 実行の失敗は理由を出す
    dt.evaluate("window.__fmuFail = true")
    desktop_open()
    form.get_by_role("button", name="実行").click()
    expect(dt.get_by_role("alert")).to_contain_text("signal: 11", timeout=10_000)
    assert not asked, asked  # 確認ダイアログはもう出ない
    dt.close()
    print("FMU を開く: ブラウザは Desktop だけと理由、Desktop は警告つきの設定フォーム（やめれば回さない）で終了時刻・刻み・予算・種類が request に乗り FMU ごとに残り、結果のタブにプロット、失敗は理由を出す")


# S236: 結果のタブの復元、MAT のパラメータ、最近使ったファイルへの結果
RESULT_RPC = """
(() => {
  window.__reads = [];
  Object.defineProperty(window.__rpc, "globe/readResults", { configurable: true, get() {
    window.__reads.push(1);
    if (localStorage.getItem("__gm_fail")) throw new Error("消えた");
    return { layout: { names: ["time", "x"], points: 3, defaults: ["x"] }, columns: [[0, 1, 2], [1, 0.5, 0.25]], parameters: [{ name: "k", value: 2, unit: "1/s", description: "減衰" }, { name: "R", value: 10 }] };
  } });
})();
"""


def results_followups(browser, origin: str, errors: list[str]) -> None:
    """S236: Desktop は結果のファイル（.mat・.csv）のタブをパスで残し、再読み込みで読み直す（読めなければ閉じて理由）。結果のタブに MAT のパラメータの
    折りたたみの表。最近使ったファイルに結果が入り、選ぶと開く。ブラウザはパスが無く、再読み込みで閉じる（案内が出て、最近の一覧にも入らない）。"""
    pg = browser.new_page()
    pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.on("dialog", lambda d: d.accept())
    pg.add_init_script(FAKE_TAURI)
    pg.add_init_script(RESULT_RPC)
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    pg.evaluate("__fake.opens.push('/d1/out.mat')")
    pg.get_by_role("button", name="開く", exact=True).click(force=True)
    tab = pg.get_by_role("tab", name="結果 out.mat")
    expect(tab).to_have_attribute("aria-selected", "true", timeout=30_000)
    expect(pg.locator("p[data-results]")).to_contain_text("再読み込みでは同じファイルを読み直す")
    # パラメータの折りたたみ（名前・値・単位・説明）
    params = pg.locator("details[data-result-params]")
    expect(params.locator("summary")).to_contain_text("2 個")
    params.locator("summary").click()
    row = params.locator("tr[data-param=k]")
    assert [c.strip() for c in row.locator("th,td").all_inner_texts()] == ["k", "2", "1/s", "減衰"], row.inner_text()
    # 最近使ったファイルに入る
    recent = pg.get_by_label("最近使ったファイル")
    assert "out.mat" in recent.locator("option").all_inner_texts()
    # 再読み込み: タブがパスから戻り、パラメータもある
    pg.reload()
    expect(pg.get_by_role("tab", name="結果 out.mat")).to_have_attribute("aria-selected", "true", timeout=30_000)
    expect(pg.locator(".plot")).to_have_attribute("data-series", "1", timeout=30_000)
    expect(pg.locator("details[data-result-params] summary")).to_contain_text("2 個")
    assert pg.evaluate("__reads.length") == 1
    # 別のタブに移ってから最近使ったファイルで開いても結果のタブに行く
    pg.get_by_role("tab").nth(0).click()
    pg.get_by_label("最近使ったファイル").select_option(label="out.mat")
    expect(pg.get_by_role("tab", name="結果 out.mat")).to_have_attribute("aria-selected", "true")
    assert pg.get_by_role("tab").count() == 2 and pg.evaluate("__reads.length") == 2
    # ファイルが読めなくなっていれば、タブを閉じて理由を出す
    pg.evaluate("localStorage.setItem('__gm_fail', '1')")
    pg.reload()
    expect(pg.locator(".failure")).to_contain_text("out.mat を読み直せなかった", timeout=30_000)
    expect(pg.get_by_role("tab", name="結果 out.mat")).to_have_count(0)
    assert pg.get_by_role("tab").count() == 1
    pg.close()

    # ブラウザ: パスが無いので再読み込みで閉じる（案内）。最近使ったファイルにも入れない
    pg = browser.new_page()
    pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.on("dialog", lambda d: d.accept())
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    with pg.expect_file_chooser() as ch:
        pg.get_by_role("button", name="開く").click(force=True)
    ch.value.set_files(files=[{"name": "decay.csv", "mimeType": "text/csv", "buffer": b"time,x\n0,1\n1,0.5\n"}])
    expect(pg.get_by_role("tab", name="結果 decay.csv")).to_have_attribute("aria-selected", "true", timeout=60_000)
    expect(pg.locator("p[data-results]")).to_contain_text("再読み込みで閉じる")
    assert pg.get_by_label("最近使ったファイル").count() == 0
    pg.reload()
    pg.get_by_role("button", name="検査").wait_for()
    assert pg.get_by_role("tab").count() == 1 and pg.get_by_role("tab", name="結果 decay.csv").count() == 0
    pg.close()
    print("結果の続き: Desktop の結果のタブは再読み込みでパスから戻り（読めなければ閉じて理由）、MAT のパラメータの表と最近使ったファイルに結果が出る。ブラウザは再読み込みで閉じる案内")


def plot_fixes(browser, origin: str, errors: list[str]) -> None:
    """S201: プロットの選択を保つ（同じモデルの再実行・パラメータを変えた再実行・別のモデル・タブの切り替え・信号が無くなった場合）、既定に戻す、uPlot の作り直しが投げたときに古い図を出さない。"""
    pg = browser.new_page(accept_downloads=True)
    pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.on("dialog", lambda d: d.accept())
    pg.goto(origin)
    pg.get_by_role("button", name="検査").wait_for()
    run = pg.get_by_role("button", name="実行", exact=True)
    plot = pg.locator(".plot")
    box = lambda n: pg.get_by_role("checkbox", name=n, exact=True)
    state = lambda: (plot.get_attribute("data-series"), plot.get_attribute("data-right"), plot.get_attribute("data-x"))
    picked = lambda: sorted(n for n in ("x", "v", "e", "p", "q") if box(n).count() and box(n).is_checked())

    def put(model: str, body: str) -> None:
        pg.get_by_label("モデル名").fill(model)
        pg.locator(".cm-content").focus()
        pg.keyboard.press("Control+A")
        pg.keyboard.insert_text(body)

    def go() -> None:
        run.click(force=True)
        expect(run).to_be_enabled(timeout=300_000)
        assert pg.get_by_role("alert").count() == 0, pg.get_by_role("alert").all_inner_texts()

    put("Spring", SPRING_K)
    go()
    assert picked() == ["v", "x"], picked()  # 既定は状態の 2 本
    # 選び直す: e を足し、v を外し、e を右の軸へ、x 軸を v に、絞り込みの文字と「内部の変数も表示」
    box("e").check(force=True)
    box("v").uncheck(force=True)
    pg.get_by_label("e を右の軸に").check(force=True)
    pg.get_by_label("x 軸").select_option(label="v")
    pg.get_by_label("信号を絞り込む").fill("e")
    pg.get_by_label("内部の変数も表示").check(force=True)
    expect(plot).to_have_attribute("data-series", "2")
    want = ("2", "1", "v")
    assert state() == want, state()

    def kept(what: str) -> None:
        expect(plot).to_have_attribute("data-x", want[2])
        assert state() == want and pg.get_by_label("信号を絞り込む").input_value() == "e" and pg.get_by_label("内部の変数も表示").is_checked(), (what, state())
        pg.get_by_label("信号を絞り込む").fill("")
        assert picked() == ["e", "x"], (what, picked())
        pg.get_by_label("信号を絞り込む").fill("e")
        label = pg.locator(".plot canvas").first.get_attribute("aria-label")
        assert "信号 2 本: x、e" in label and "v の範囲" in label, (what, label)  # 代替テキストは保たれた選択と一致する

    # 1. 同じモデルの再実行
    go()
    kept("同じモデルの再実行")
    # CSV は保たれた選択（x 軸の v、そのあと x と e）
    with pg.expect_download() as dl:
        pg.get_by_role("button", name="CSV").click(force=True)
    assert Path(dl.value.path()).read_text(encoding="utf-8").splitlines()[0] == "v,x,e", Path(dl.value.path()).read_text(encoding="utf-8")[:40]
    # 2. パラメータを変えて再実行（k = 2。同じモデルなので選択は保たれ、値は変わる）
    pg.get_by_label("パラメータの上書き").fill("k = 2")
    go()
    kept("パラメータを変えた再実行")
    pg.get_by_label("パラメータの上書き").fill("")
    # 3. 信号が無くなった: e が消えたモデルでは e を外し（右の軸も消える）、残った x を選ぶ。全部無くなったら既定に戻す
    put("Spring", SPRING_NO_E)
    go()
    expect(plot).to_have_attribute("data-series", "1")
    pg.get_by_label("信号を絞り込む").fill("")
    assert picked() == ["x"] and state() == ("1", "0", "v"), (picked(), state())
    put("Spring", SPRING_RENAMED)
    go()
    assert pg.get_by_label("信号を絞り込む").input_value() == "" and not pg.get_by_label("内部の変数も表示").is_checked()  # 保てるものが無ければ全部既定
    assert picked() == ["p", "q"] and state() == ("2", "0", "time"), (picked(), state())
    # 4. 別のモデルは既定から（絞り込みの文字と内部名の表示も戻る）。先に選び直す
    box("p").uncheck(force=True)
    pg.get_by_label("信号を絞り込む").fill("q")
    put("Decay", "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n")
    go()
    assert state()[0] == "1" and pg.get_by_label("信号を絞り込む").input_value() == "" and not pg.get_by_label("内部の変数も表示").is_checked(), state()
    # 5. 選び直していなければ、再実行でも既定（今の挙動）。Decay の x を外して選び直し、既定に戻すボタンをキーボードで押す
    box("x").uncheck(force=True)
    expect(plot).to_have_attribute("data-series", "0")
    go()
    expect(plot).to_have_attribute("data-series", "0")  # 選び直し（全部外した）は保たれる
    pg.get_by_label("信号を絞り込む").fill("zzz")
    reset = pg.get_by_role("button", name="既定に戻す", exact=True)
    reset.focus()
    pg.keyboard.press("Enter")
    expect(plot).to_have_attribute("data-series", "1")
    assert pg.get_by_label("信号を絞り込む").input_value() == "" and box("x").is_checked()
    go()
    expect(plot).to_have_attribute("data-series", "1")
    # 6. タブの切り替え: Decay の結果は消え、戻って実行すると既定（保たない）
    box("x").uncheck(force=True)
    with pg.expect_file_chooser() as ch:
        pg.get_by_role("button", name="開く").click(force=True)
    ch.value.set_files(files=[{"name": "Other.mo", "mimeType": "text/plain", "buffer": b"model Other\nend Other;\n"}])
    expect(pg.get_by_role("tab")).to_have_count(2)
    pg.get_by_role("tab").nth(0).click()
    expect(plot).to_have_count(0)
    go()
    expect(plot).to_have_attribute("data-series", "1")
    print("プロットの選択を保つ: 同じモデルの再実行・パラメータを変えた再実行で信号・右の軸・x 軸・絞り込み・内部名が残り、書き出しと代替テキストが合う。信号が無くなれば外し、全部無くなれば既定。別のモデル・タブの切り替えは既定。「既定に戻す」はキーボードで押せる")
    # 7. uPlot の作り直しが投げたら、古い図を PNG に出さない（canvas の 2d context を一度だけ投げさせる）
    pg.evaluate("""() => { const g = HTMLCanvasElement.prototype.getContext; window.__ctx = g;
      HTMLCanvasElement.prototype.getContext = function (...a) { if (window.__throw) throw new Error("試験: context を作れない"); return g.apply(this, a); }; }""")
    pg.evaluate("window.__throw = true")
    box("x").uncheck(force=True)  # 選択が変わって uPlot を作り直す
    expect(pg.get_by_role("alert")).to_contain_text("context", timeout=10_000)
    pg.evaluate("window.__throw = false")
    got = []
    pg.on("download", lambda d: got.append(d.suggested_filename))
    pg.get_by_role("button", name="PNG").click(force=True)
    pg.wait_for_timeout(1500)
    assert got == [], f"作り直しが失敗したあとの PNG に古い図が出た {got}"
    # 戻れば描ける
    box("x").check(force=True)
    expect(pg.get_by_role("alert")).to_have_count(0)
    pg.close()
    print("uPlot の作り直しが投げたあと: PNG は古い図を書き出さず、選択を戻すと描ける")


def main() -> int:
    # 色付けの予約語は Rust の KEYWORDS と同じ並び（S163。editor.ts の KEYWORDS は補完と色付けの共通の一覧）
    ts_kw = re.findall(r'"(\w+)"', re.search(r"export const KEYWORDS = \[(.*?)\];", (SRC / "Svelte/src/lib/editor.ts").read_text(), re.S).group(1))
    assert ts_kw == RUST_KW, set(ts_kw) ^ set(RUST_KW)
    # 列の道の NaN / Inf は null になる（uPlot の欠損。S065）
    subprocess.run(["node", str(Path(__file__).with_name("test_columns.mjs"))], check=True)
    subprocess.run(["node", "--test", str(Path(__file__).with_name("test_html_sanitize.mjs"))], check=True)  # S232: 説明パネルの HTML の白リスト
    subprocess.run(["node", str(Path(__file__).with_name("test_tauri.mjs"))], check=True)  # S196: Desktop の保存先・outDir・タブの key
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
        expect(page.locator(".cm-content .cm-mo-kw").first).to_have_text("model")  # 色付け（S163）: 開いた Decay の予約語に色のクラスが付く
        # 折りたたみ（S225）: Rust の foldingRange の範囲が gutter に印として出る。畳むと `end Decay;` の行が隣に来て、開くと戻る
        mark = page.locator('.cm-foldGutter span[title="Fold line"]')
        expect(mark.first).to_be_visible(timeout=30_000)
        mark.first.click()
        expect(page.locator(".cm-foldPlaceholder")).to_have_count(1)
        expect(page.locator(".cm-line").nth(1)).to_have_text("end Decay;")
        page.locator(".cm-foldPlaceholder").click()  # 開く印は、畳んだ行の「…」を押しても同じ
        expect(page.locator(".cm-foldPlaceholder")).to_have_count(0)
        expect(page.locator(".cm-line").nth(1)).to_contain_text("Real x(start")
        print("折りたたみ: model Decay を畳むと end Decay; が隣に来て、開くと戻る")
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
                  let i = -1;
                  for (let k = 0; k <= nth; k++) i = el.textContent.indexOf(name, i + 1);
                  // 色付け（S163）で行が複数の span に分かれるので、i 文字目を含むテキストノードを探す
                  const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
                  let t, off = 0;
                  while ((t = w.nextNode()) && i >= off + t.textContent.length) off += t.textContent.length;
                  const r = document.createRange();
                  r.setStart(t, i - off); r.setEnd(t, i - off + 1);
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

        # 参照とリネーム（S161）: Decay の `der(x)` の x で Shift+F12 → 一覧 3 行（宣言・der(x)・-x のどれか）から押して飛べる。
        # F2 で名前を変えると全出現が変わり、検査が通る。予約語は断られ、1 回の undo で戻る
        base = page.locator(".cm-content").inner_text()
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+End")
        page.keyboard.type("\n// x と \"x\" は変えない")  # コメントと文字列の中の x は参照にもリネームにも入らない
        page.wait_for_timeout(700)  # 入力と次の操作を履歴の別の塊にする
        original = page.locator(".cm-content").inner_text()
        page.keyboard.press("Control+Home")
        for _ in range(3):
            page.keyboard.press("ArrowDown")
        page.keyboard.press("End")
        page.keyboard.press("ArrowLeft")
        page.keyboard.press("Shift+F12")
        refs = page.locator("table.refs tr")
        expect(refs).to_have_count(3, timeout=30_000)
        refs.first.click()
        expect(page.locator(".cm-activeLine")).to_contain_text("Real x(start")
        print(f"Decay の Shift+F12: 参照 {refs.all_inner_texts()}")
        page.get_by_role("button", name="参照を閉じる").click()
        page.locator(".cm-content").focus()
        page.keyboard.press("F2")
        name = page.get_by_label("新しい名前")
        name.fill("model")
        name.press("Enter")
        expect(page.get_by_role("alert")).to_contain_text("予約語", timeout=30_000)
        assert page.locator(".cm-content").inner_text() == original
        page.locator(".cm-content").focus()
        page.keyboard.press("F2")
        name.fill("pos")
        name.press("Enter")
        expect(page.locator(".cm-content")).to_contain_text("der(pos) = -pos;", timeout=30_000)
        assert "// x と \"x\" は変えない" in page.locator(".cm-content").inner_text()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        check()
        assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
        page.locator(".cm-content").focus()  # 検査のボタンからフォーカスを戻す
        page.keyboard.press("Control+z")
        assert page.locator(".cm-content").inner_text() == original, "リネームは 1 回の undo で戻る"
        page.keyboard.press("Control+z")  # 足したコメントも消す
        assert page.locator(".cm-content").inner_text() == base
        print("Decay の F2: x を pos に変えて検査が通り、予約語は断られ、Ctrl+Z 1 回で戻った")
        # クラス名の参照とリネーム（S166）: Base を extends する Decay に差し替え、`extends Base` の Base で Shift+F12 3 行（宣言・end・extends）
        # F2 で Core に変えると宣言・`end`・extends が変わり（コメントの Base は変えない）、検査が通る。予約語の上の F2 は入力欄を出さずに理由を出す
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(
            "model Base\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Base;\n\n"
            "model Decay\n  extends Base;\nend Decay; // Base は変えない\n"
        )
        page.wait_for_timeout(700)
        classes = page.locator(".cm-content").inner_text()
        page.keyboard.press("Control+Home")
        page.keyboard.press("F2")  # 1 行目の `model` の上
        expect(page.get_by_role("alert")).to_contain_text("宣言した部品・クラスでない", timeout=30_000)
        expect(page.get_by_label("新しい名前")).to_have_count(0)
        for _ in range(7):
            page.keyboard.press("ArrowDown")
        page.keyboard.press("End")
        page.keyboard.press("ArrowLeft")
        page.keyboard.press("Shift+F12")
        refs = page.locator("table.refs tr")
        expect(refs).to_have_count(3, timeout=30_000)  # model Base・end Base・extends Base
        page.get_by_role("button", name="参照を閉じる").click()
        page.locator(".cm-content").focus()
        page.keyboard.press("F2")
        name = page.get_by_label("新しい名前")
        name.fill("Core")
        name.press("Enter")
        expect(page.locator(".cm-content")).to_contain_text("extends Core;", timeout=30_000)
        text = page.locator(".cm-content").inner_text()
        assert "model Core" in text and "end Core;" in text and "// Base は変えない" in text and "Base;" not in text, text
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        check()
        assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+z")
        assert page.locator(".cm-content").inner_text() == classes, "クラス名のリネームも 1 回の undo で戻る"
        page.keyboard.press("Control+z")
        assert page.locator(".cm-content").inner_text() == base
        print("Decay のクラス名: Shift+F12 で 3 行、F2 で Base を Core に変えて検査が通り、予約語の上は入力欄を出さず、Ctrl+Z で戻った")
        # 補完（S171）: 予約語と本文の単語に、Rust 側の候補が足される。MSL なしでも、文書のクラス名・部品名（`pin.` の後ろは型 Pin の部品名）が出る

        def undo_to(want: str) -> None:
            page.locator(".cm-content").focus()
            for _ in range(60):
                if page.locator(".cm-content").inner_text() == want:
                    return
                page.keyboard.press("Control+z")
                page.wait_for_timeout(50)
            assert page.locator(".cm-content").inner_text() == want, "undo で戻らない"

        # クラス名のリネームでモデル名の欄も追う（S186）: モデル T を Net に変えると欄が Net になって検査が通り、Ctrl+Z で本文も欄も戻る。入れ子（P.M の P を Q に）も
        content, name_box = page.locator(".cm-content"), page.get_by_label("モデル名")
        for text, qualified, old, new in [
            ("model T\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend T;\n", "T", "T", "Net"),
            ("package P\n  model M\n    Real x(start = 1, fixed = true);\n  equation\n    der(x) = -x;\n  end M;\nend P;\n", "P.M", "P", "Q"),
        ]:
            content.focus()
            page.keyboard.press("Control+A")
            page.keyboard.insert_text(text)
            name_box.fill(qualified)
            content.focus()
            page.keyboard.press("Control+Home")
            page.keyboard.press("End")
            page.keyboard.press("ArrowLeft")  # 1 行目の末尾のクラス名の前
            page.keyboard.press("F2")
            renamer = page.get_by_label("新しい名前")
            renamer.fill(new)
            renamer.press("Enter")
            expect(content).to_contain_text(f"end {new};", timeout=30_000)
            followed = qualified.replace(old, new, 1)
            expect(name_box).to_have_value(followed)
            check()
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
            content.focus()
            page.keyboard.press("Control+z")
            expect(content).to_contain_text(f"end {old};")
            expect(name_box).to_have_value(qualified)
        undo_to(base)
        name_box.fill("Decay")
        content.focus()
        print("クラス名のリネーム（S186）: モデル T を Net に、P.M の P を Q に変えるとモデル名の欄も追って検査が通り、Ctrl+Z で本文もモデル名も戻った")

        popup = page.locator(".cm-tooltip-autocomplete")
        page.keyboard.press("Control+A")
        page.keyboard.insert_text("connector Pin\n  Real v;\n  flow Real i;\nend Pin;\n\nmodel Probe\n  Pin pin;\n  Real level;\nequation\n  \nend Probe;\n")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+Home")
        for _ in range(9):
            page.keyboard.press("ArrowDown")
        page.keyboard.press("End")
        page.keyboard.type("lev")
        expect(popup.locator(".cm-completionDetail", has_text="Real")).to_be_visible(timeout=30_000)  # Rust 側の候補 level（型 Real つき）
        page.keyboard.press("Escape")
        page.keyboard.press("Backspace")
        page.keyboard.press("Backspace")
        page.keyboard.press("Backspace")
        page.keyboard.type("pin.")
        expect(popup.locator(".cm-completionLabel", has_text="v")).to_be_visible(timeout=30_000)
        expect(popup.locator(".cm-completionLabel", has_text="i")).to_be_visible()
        # 候補が開いたまま Ctrl+Enter（実行）を押すと、候補は閉じ、実行が終わっても残らない（S186）
        page.keyboard.press("Control+Enter")
        expect(popup).to_have_count(0)
        expect(page.get_by_role("button", name="実行")).to_be_enabled(timeout=120_000)
        expect(popup).to_have_count(0)
        page.keyboard.press("Backspace")
        page.keyboard.type(".")
        expect(popup.locator(".cm-completionLabel", has_text="v")).to_be_visible(timeout=30_000)
        page.keyboard.type("v")
        expect(popup.locator("li[aria-selected] .cm-completionLabel")).to_have_text("v")
        page.keyboard.press("Enter")
        expect(page.locator(".cm-activeLine")).to_have_text("  pin.v")
        undo_to(base)
        print("Decay の補完: 部品名 level（型 Real つき）と、pin. の後ろの型 Pin の部品名 v・i が候補に出て、選ぶと入った")
        # 構文の色付け（S163）: 別の本文を入れて色を見て、Ctrl+Z で Decay に戻す
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(SYN)
        check_syntax(page)
        page.keyboard.press("Control+z")
        assert page.locator(".cm-content").inner_text() == base
        print("色付け（明）: 予約語・型・数値・文字列・コメントに別々の色のクラスが付き、地に対して 4.5 以上")

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
        assert re.fullmatch(r"Decay-\d{8}-\d{6}\.csv", csv_dl.value.suggested_filename), csv_dl.value.suggested_filename  # モデル名と時刻（S169）
        csv_lines = Path(csv_dl.value.path()).read_text(encoding="utf-8").splitlines()
        assert csv_lines[0].startswith("time,x"), csv_lines[:2]
        assert len(csv_lines) == int(page.locator(".plot").get_attribute("data-points")) + 1, len(csv_lines)
        assert all(math.isfinite(float(v)) for v in csv_lines[1].split(",")[:2]), csv_lines[1]
        with page.expect_download() as png_dl:
            page.get_by_role("button", name="PNG").click(force=True)
        png = Path(png_dl.value.path()).read_bytes()
        assert re.fullmatch(r"Decay-\d{8}-\d{6}\.png", png_dl.value.suggested_filename) and png.startswith(b"\x89PNG\r\n\x1a\n") and len(png) > 1000, len(png)
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

        # 信号の絞り込み（S169）: 一致しない語で一覧が空になっても選択は残り、消すと戻る。「内部の変数も表示」の入力はキーボードで届く
        box = page.get_by_label("信号を絞り込む")
        box.fill("zzz")
        expect(plot).to_have_attribute("data-listed", "0")
        expect(plot).to_have_attribute("data-series", "1")
        box.fill("")
        expect(plot).to_have_attribute("data-listed", str(boxes.count()))
        print("Decay の絞り込み: 一致なしで一覧 0 本・選択 1 本のまま、空に戻すと一覧が戻った")

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
        page.locator("[data-params]").evaluate("e => (e.open = true)")  # 一覧は閉じて始まる（S170）
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

        # 既定で描く信号は状態（S190）: 2 状態 + 代数変数のモデルで、先頭の 3 本でなく状態の 2 本が選ばれる。選び直しは絞り込み・内部名の切り替えで上書きされない
        page.get_by_label("モデル名").fill("Spring")
        page.locator(".cm-content").focus()
        page.keyboard.press("Control+A")
        page.keyboard.insert_text(SPRING)
        simulate()
        assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
        plot, signals = page.locator(".plot"), page.locator(".plot .signals input[name=signal]")
        picked = lambda: sorted(n for n in ("x", "v", "e") if page.get_by_role("checkbox", name=n, exact=True).is_checked())
        expect(plot).to_have_attribute("data-series", "2")
        assert signals.count() == 3 and picked() == ["v", "x"], (signals.count(), picked())
        page.get_by_role("checkbox", name="e", exact=True).check(force=True)
        page.get_by_label("信号を絞り込む").fill("zzz")
        page.get_by_label("信号を絞り込む").fill("")
        page.get_by_label("内部の変数も表示").check(force=True)
        page.get_by_label("内部の変数も表示").uncheck(force=True)
        expect(plot).to_have_attribute("data-series", "3")
        assert picked() == ["e", "v", "x"], picked()
        print("Spring の既定: 状態の x・v の 2 本（代数変数 e は選ばれない）。e を足した選択は絞り込みと内部名の切り替えで保たれた")

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
        status = page.locator("p.run")  # 実行中は role を持たない（更新のたびに読み上げない。S196）
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

            # 補完（S171）: 型の位置で `Modelica.Blocks.Sources.` と打つと、ライブラリの子（Sine）が候補に出て、選ぶと入る
            circuit = page.locator(".cm-content").inner_text()
            page.locator(".cm-content").focus()
            page.keyboard.press("Control+A")
            page.keyboard.insert_text("model T\n  Modelica.Blocks.Sources.\n  Real x;\nend T;\n")
            page.keyboard.press("Control+Home")
            page.keyboard.press("ArrowDown")
            page.keyboard.press("End")
            t = time.perf_counter()
            page.keyboard.type("Si")
            sine_option = popup.locator(".cm-completionLabel", has_text=re.compile(r"^Sine$"))
            expect(sine_option).to_be_visible(timeout=30_000)
            first = time.perf_counter() - t
            sine_option.click()
            expect(page.locator(".cm-activeLine")).to_have_text("  Modelica.Blocks.Sources.Sine")
            undo_to(circuit)
            print(f"MSL の補完: Modelica.Blocks.Sources.Si で Sine が候補に出るまで {first:.2f} s、選ぶと入った")

            page.get_by_label("モデル名").fill(CHUA)
            t = time.perf_counter()
            check()
            took = time.perf_counter() - t
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()  # 診断は 0 行でよい（experiment がある）
            print(f"ChuaCircuit の検査: {took:.1f} s、診断 {rows.count()} 行、error 0 行")
            # 上書きできるパラメータの一覧（S150）: 絞り込みで本数が減る
            page.locator("[data-params]").evaluate("e => (e.open = true)")
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

            # 信号の絞り込みと内部名（S169）: `$` で始まる名前を持つ例題。既定で隠れ、チェックで出て、CSV はチェックに合わせる
            page.get_by_label("モデル名").fill(FIRST)
            simulate()
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            plot = page.locator(".plot")
            box = page.get_by_label("信号を絞り込む")
            hidden = page.locator(".plot .signals input[name=signal]")
            assert hidden.count() > 20 and not any(n.startswith("$") for n in page.locator(".plot .signals label").all_inner_texts())
            total = int(plot.get_attribute("data-matched"))
            page.get_by_label("内部の変数も表示").check(force=True)
            with_internal = int(plot.get_attribute("data-matched"))
            assert with_internal > total, (total, with_internal)
            box.fill("FORMAL_derivative inertia1")  # 大文字小文字を区別せず、空白区切りは AND
            n_some = int(plot.get_attribute("data-matched"))
            assert 0 < n_some < with_internal and all("$formal_derivative" in t and "inertia1" in t for t in page.locator(".plot .signals label").all_inner_texts() if "右" not in t)
            page.get_by_role("button", name="表示中をすべて選ぶ").click(force=True)
            assert page.locator(".plot .signals input[name=signal]:not(:checked)").count() == 0 and int(plot.get_attribute("data-series")) >= n_some
            with page.expect_download() as dl:
                page.get_by_role("button", name="CSV").click(force=True)
            assert re.fullmatch(r"First-\d{8}-\d{6}\.csv", dl.value.suggested_filename), dl.value.suggested_filename
            assert "$formal_derivative" in Path(dl.value.path()).read_text(encoding="utf-8").splitlines()[0]
            page.get_by_label("内部の変数も表示").uncheck(force=True)  # 選択は残っているが、隠せばプロットにも CSV にも出ない
            with page.expect_download() as dl:
                page.get_by_role("button", name="CSV").click(force=True)
            assert "$" not in Path(dl.value.path()).read_text(encoding="utf-8").splitlines()[0]
            box.fill("")
            final = json.loads(page.locator("p.final").get_attribute("data-final"))  # 最後の時刻の値にも内部名を出さない（S173）
            assert final and not [k for k in final if k.startswith("$")], list(final)
            print(f"First の内部名: 既定で一覧 {total} 本（隠れた内部名 {with_internal - total} 本）、絞り込みで {n_some} 本、チェックを外すと CSV から消えた")
            page.get_by_label("モデル名").fill(CHUA)

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
            # 入れ子のクラスと検索（S167）: 検索欄に PID_Con と打って Enter（キーボードだけ）で、Blocks/package.mo の中の入れ子のクラスが開く
            pid = "Modelica.Blocks.Examples.PID_Controller"
            box = page.get_by_label("ライブラリを検索")
            t = time.perf_counter()
            box.fill("PID_Con")
            expect(page.locator("#library-hits [role=option]").first).to_contain_text(pid, timeout=30_000)
            found = time.perf_counter() - t
            box.press("Enter")
            expect(page.get_by_label("モデル名")).to_have_value(pid, timeout=60_000)
            expect(page.locator(".cm-content")).to_contain_text("model PID_Controller")
            expect(page.locator(".cm-activeLine")).to_contain_text("model PID_Controller")  # S186: 開いた入れ子のクラスの宣言の行にカーソルがある
            check()
            assert page.get_by_role("alert").count() == 0, page.get_by_role("alert").all_inner_texts()
            assert page.locator("tr.error").count() == 0, rows.all_inner_texts()
            print(f"検索 PID_Con: 候補まで {found:.2f} s、Enter で PID_Controller を開いて検査 error 0 行")
            # 木からも同じクラスへ行ける: Blocks（ファイルの package.mo）の入れ子の Examples を開き、Sources.Sine は別のファイルの中の入れ子
            page.get_by_label("モデル名").fill("")
            for path in ["Modelica.Blocks", "Modelica.Blocks.Examples"]:
                page.locator(f'.library li[data-path="{path}"] > button').click(force=True)
            page.locator(f'.library li[data-path="{pid}"] > button').click(force=True)
            expect(page.get_by_label("モデル名")).to_have_value(pid, timeout=60_000)
            sine = "Modelica.Blocks.Sources.Sine"
            page.get_by_label("ライブラリを検索").fill("sources.sine")
            expect(page.locator("#library-hits [role=option]").first).to_contain_text(sine, timeout=30_000)
            page.get_by_label("ライブラリを検索").press("Enter")
            expect(page.get_by_label("モデル名")).to_have_value(sine, timeout=60_000)
            # S186: ファイルの途中（Sources.mo の 291 行目、Blocks/package.mo の 991 行目）にある入れ子のクラスは、宣言の行にカーソルが置かれて画面（エディタの窓）に入る
            def on_screen(want: str) -> None:
                expect(page.locator(".cm-activeLine")).to_contain_text(want, timeout=30_000)
                for _ in range(40):  # スクロールは描画の後に決まる
                    inside = page.evaluate("""() => { const l = document.querySelector('.cm-activeLine').getBoundingClientRect(), s = document.querySelector('.cm-scroller').getBoundingClientRect();
                      return l.top >= s.top - 1 && l.bottom <= s.bottom + 1 && l.bottom <= innerHeight; }""")
                    if inside:
                        return
                    page.wait_for_timeout(100)
                raise AssertionError(f"{want} の行が画面に入らない")

            on_screen("block Sine")
            assert page.get_by_role("tab").nth(page.get_by_role("tab").count() - 1).get_attribute("title").endswith("Blocks/Sources.mo"), "タブの title にパスが出る"
            page.get_by_label("ライブラリを検索").fill("Examples.BusUsage")  # Blocks/package.mo の 991 行目
            page.get_by_label("ライブラリを検索").press("Enter")
            expect(page.get_by_label("モデル名")).to_have_value("Modelica.Blocks.Examples.BusUsage", timeout=60_000)
            on_screen("model BusUsage")
            print("木と検索: Blocks.Examples.PID_Controller と Blocks.Sources.Sine・Examples.BusUsage（入れ子のクラス）が開き、宣言の行にカーソルがあって画面に入った")
            # 未対応の例題（S173）: Friction は Rumoca の既知の制約（GM-U0003）。要約が出て、error の行は無く、強行の button も無い
            page.get_by_label("モデル名").fill("Modelica.Mechanics.Rotational.Examples.Friction")
            check()
            expect(page.locator("[data-unsupported]")).to_contain_text("モデルの誤りではなく", timeout=60_000)
            assert page.locator("tr.error").count() == 0 and page.locator("tr.unsup").count() >= 1 and page.locator("[data-can-force]").count() == 0
            assert "unbalanced model" in page.locator("tr.unsup details").first.text_content()  # Rumoca の英語の原文は詳細に残る
            print(f"Friction: 未対応の要約と tr.unsup {page.locator('tr.unsup').count()} 行（error 0 行）、原文は詳細に残る")
            # 「MSL を読む」をキーボードで押すと、読み終えたあとフォーカスはライブラリの検索欄へ移る（ボタンが無効になって body に落ちない。S186）
            mk = browser.new_page(viewport={"width": 1280, "height": 800})
            mk.on("console", lambda m: m.type == "error" and errors.append(m.text))
            mk.on("pageerror", lambda e: errors.append(str(e)))
            mk.goto(origin)
            mk.get_by_role("button", name="検査").wait_for()
            for _ in range(60):
                mk.keyboard.press("Tab")
                if mk.evaluate("document.activeElement.textContent") == "MSL を読む":
                    break
            else:
                raise AssertionError("Tab で「MSL を読む」に届かない")
            mk.keyboard.press("Enter")
            expect(mk.locator("p.note[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=300_000)
            expect(mk.get_by_label("ライブラリを検索")).to_be_focused(timeout=10_000)
            mk.close()
            print("MSL を読む: Enter で押すと、読み終えたあとフォーカスがライブラリの検索欄へ移った")
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
        assert fmu.value.suggested_filename == "Mine-fmi2.fmu"
        with zipfile.ZipFile(fmu.value.path()) as z:
            assert "modelDescription.xml" in z.namelist() and any(n.startswith("sources/") for n in z.namelist()), z.namelist()
            assert b'modelName="Mine"' in z.read("modelDescription.xml")
        expect(own.locator("p[data-saved]")).to_contain_text("Mine-fmi2.fmu")
        print("FMU: Mine-fmi2.fmu がダウンロードされ、modelDescription.xml と sources/ がある")
        # FMI の版を選ぶ（S154）: 既定は 2.0、3.0 を選ぶと fmiVersion="3.0" の modelDescription.xml
        assert b'fmiVersion="2.0"' in zipfile.ZipFile(fmu.value.path()).read("modelDescription.xml")
        own.get_by_label("FMI の版").select_option("3")
        with own.expect_download() as fmu3:
            own.get_by_role("button", name="FMU を書き出す").click(force=True)
        assert fmu3.value.suggested_filename == "Mine-fmi3.fmu"  # S189: 版つき（2.0 と 3.0 が上書きし合わない）
        with zipfile.ZipFile(fmu3.value.path()) as z:
            assert b'fmiVersion="3.0"' in z.read("modelDescription.xml") and any(n.startswith("sources/") for n in z.namelist()), z.namelist()
        print("FMU: FMI の版に 3.0 を選ぶと modelDescription.xml が fmiVersion=3.0")
        # FMU はソースだけ（S173）: ボタンの近くと書き出したあとの文に、バイナリが無いことと作り方（CLI の --build）が出る
        expect(own.locator("[data-fmu-note]")).to_contain_text("バイナリ無し")
        expect(own.locator("[data-fmu-note]")).to_contain_text("globe fmu export --build")
        expect(own.locator("p[data-saved]")).to_contain_text("バイナリは入っていない")
        expect(own.locator("p[data-saved]")).to_contain_text("globe fmu export --build")
        print("FMU: ボタンの近くと書き出したあとに、ソースだけであることと globe fmu export --build が出る")
        # 未対応・強行・エラー文面（S173）
        ux = browser.new_page()
        ux.on("console", lambda m: m.type == "error" and errors.append(m.text))
        ux.on("pageerror", lambda e: errors.append(str(e)))
        ux.goto(f"http://127.0.0.1:{server.server_port}/")

        def press(name: str) -> None:
            b = ux.get_by_role("button", name=name, exact=True)
            b.wait_for(timeout=120_000)
            b.click(force=True)
            expect(b).to_be_enabled(timeout=180_000)

        def put(model: str, text: str) -> None:
            ux.get_by_label("モデル名").fill(model)
            ux.locator(".cm-content").focus()
            ux.keyboard.press("Control+A")
            ux.keyboard.insert_text(text)

        # 強行が意味を持つ Error（GM-E8001）: 実行は断られ、「それでも実行する」で進み、結果に強行の印が出る
        put("Over", OVER)
        press("実行")
        expect(ux.locator(".failure")).to_contain_text("force で強行できる")
        expect(ux.locator("[data-can-force]")).to_be_visible()
        assert ux.locator("[data-unsupported]").count() == 0 and ux.locator("[data-forced]").count() == 0
        assert ux.locator(".failure").inner_text().count("\n") == 1, ux.locator(".failure").inner_text()  # 1 行目の理由 + 最初の error の 1 行
        ux.get_by_role("button", name="それでも実行する").click(force=True)  # 押すと消える button なので press は使わない
        expect(ux.locator("[data-forced]")).to_be_visible(timeout=60_000)
        expect(ux.locator(".plot")).to_be_visible(timeout=60_000)
        assert ux.locator(".failure").count() == 0 and ux.locator("[data-can-force]").count() == 0
        expect(ux.locator("p.final")).to_contain_text("x = ")
        press("実行")  # 普通の実行に戻ると、強行の印は消える（断られるので failure と button が戻る）
        assert ux.locator("[data-forced]").count() == 0 and ux.locator("[data-can-force]").count() == 1
        print("強行: GM-E8001 は「それでも実行する」で進み、結果に強行の印が出て、普通の実行に戻ると消える")
        # 未対応（GM-U）: 要約が出て（error とは別のクラス・色）、原文は畳まれ、強行の button は出ず、文面に force を出さない
        put("Div", DIV)
        press("検査")
        summary = ux.locator("[data-unsupported]")
        expect(summary).to_contain_text("モデルの誤りではなく")
        assert ux.locator("tr.error").count() == 0 and ux.locator("tr.unsup").count() > 0, ux.locator("table.diagnostics").inner_text()
        assert ux.locator("[data-can-force]").count() == 0
        original = ux.locator("tr.unsup details").first
        assert original.get_attribute("open") is None and "Rumoca" in original.inner_text(), original.inner_text()
        col = lambda sel: ux.evaluate("(sel) => getComputedStyle(document.querySelector(sel)).color", sel)
        err = ux.evaluate("() => { const e = document.createElement('span'); e.style.color = 'var(--err)'; document.body.append(e); return getComputedStyle(e).color; }")
        assert col("[data-unsupported]") == col("tr.unsup") != err, (col("[data-unsupported]"), col("tr.unsup"), err)  # 誤り（--err）とは別の色
        press("実行")
        failure = ux.locator(".failure")
        expect(failure).to_contain_text("強行しても実行できない")
        assert "force で強行できる" not in failure.inner_text() and ux.locator("[data-can-force]").count() == 0
        assert col(".failure") == col("[data-unsupported]"), (col(".failure"), col("[data-unsupported]"))
        print(f"未対応: 要約 {summary.inner_text()[:40]!r}、tr.unsup {ux.locator('tr.unsup').count()} 行、原文は畳まれ、強行の button なし")
        # 数でない終了時刻: 実行する前に欄の近くで 1 回だけ示す（aria-invalid と文面）
        put("Decay", "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n")
        stop = ux.get_by_label("終了時刻")
        stop.fill("abc")
        expect(stop).to_have_attribute("aria-invalid", "true")
        press("実行")
        expect(ux.locator(".settings .problem")).to_contain_text("終了時刻「abc」は数でない")
        assert ux.locator(".failure").count() == 0 and ux.locator("main").inner_text().count("は数でない") == 1, ux.locator("main").inner_text()
        # 終了時刻 -1・存在しないパラメータ: 失敗の表示に最初の error の診断を 1 行（理由）。同じ文は 2 回出さない
        stop.fill("-1")
        expect(stop).to_have_attribute("aria-invalid", "false")
        press("実行")
        lines = ux.locator(".failure").inner_text().split("\n")
        assert len(lines) == 2 and "GM-E10004" in lines[1] and lines[0] not in lines[1], lines
        stop.fill("")
        ux.get_by_label("パラメータの上書き").fill("zz=1")
        press("実行")
        lines = ux.locator(".failure").inner_text().split("\n")
        assert len(lines) == 2 and "GM-E10006" in lines[1] and "zz" in lines[1], lines
        print(f"エラー文面: abc は欄で 1 回、-1 は {lines[0][:12]!r} + GM-E10004、zz=1 は GM-E10006 の理由が 1 行")
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
        # 結果が見える位置（S170）: 1280x800 で実行すると進捗・止める・失敗の理由が最初の画面の中に出て、終わるとプロットの上端が画面の中に入る。
        # 390 px 幅でも同じ。利用者が自分で戻したスクロールは、次の完了まで引き戻さない
        many = "model Many\n" + "".join(f'  parameter Real p{i} = {i} "説明 {i}";\n' for i in range(20)) + "  Real x(start = 1, fixed = true);\nequation\n  der(x) = -p1 * x;\n  annotation(experiment(StopTime = 3));\nend Many;\n"

        def open_in(pg, name: str, body: str) -> None:
            with pg.expect_file_chooser() as ch:
                pg.get_by_role("button", name="開く").click(force=True)
            ch.value.set_files(files=[{"name": name, "mimeType": "text/plain", "buffer": body.encode()}])

        def settle(f, what: str) -> None:
            for _ in range(60):
                if f():
                    return
                time.sleep(0.1)
            raise AssertionError(what)

        def inside(pg, selector: str) -> bool:
            b = pg.locator(selector).first.bounding_box()
            vh = pg.viewport_size["height"]
            return b is not None and 0 <= b["y"] < vh

        for width, height in [(1280, 800), (390, 844)]:
            vis = browser.new_page(viewport={"width": width, "height": height})
            vis.on("console", lambda m: m.type == "error" and errors.append(m.text))
            vis.on("pageerror", lambda e: errors.append(str(e)))
            vis.goto(origin)
            vis.get_by_role("button", name="検査").wait_for()
            open_in(vis, "Many.mo", many)
            expect(vis.locator(".cm-content")).to_contain_text("model Many", timeout=30_000)
            run_button = vis.get_by_role("button", name="実行")
            run_button.click(force=True)
            expect(run_button).to_be_enabled(timeout=120_000)
            expect(vis.locator(".plot")).to_be_visible(timeout=30_000)
            settle(lambda: inside(vis, ".plot"), f"{width} px: 実行の後にプロットの上端が画面の外 {vis.locator('.plot').bounding_box()}")
            # 利用者が上へ戻したら、勝手に引き戻さない
            plot_y = vis.locator(".plot").bounding_box()["y"]
            vis.evaluate("scrollTo(0, 0)")
            time.sleep(0.8)
            assert vis.evaluate("scrollY") == 0, "完了の後に引き戻された"
            # 進捗と止める: 長い実行を始めた時点で、実行中の表示と「止める」が画面の中
            open_in(vis, "Long.mo", LONG)
            expect(vis.locator(".cm-content")).to_contain_text("model Long", timeout=30_000)
            vis.evaluate("scrollTo(0, 0)")
            run_button.click(force=True)
            expect(vis.locator("p.run")).to_contain_text("integration", timeout=120_000)
            assert inside(vis, "p.run") and inside(vis, "button:has-text('止める')"), (width, vis.locator("p.run").bounding_box())
            vis.get_by_role("button", name="止める").click(force=True)
            expect(vis.locator("p.run")).to_have_attribute("data-run", "cancelled", timeout=60_000)
            # 失敗の理由（実行の設定が数でない）も実行のボタンの近く
            vis.get_by_label("終了時刻").fill("abc")
            run_button.click(force=True)
            expect(vis.locator(".settings .problem")).to_contain_text("終了時刻", timeout=30_000)  # S173: 実行の設定の問題は欄の近くに 1 回だけ（failure には二重に出さない）
            settle(lambda: inside(vis, ".settings .problem"), f"{width} px: 失敗の理由が画面の外 {vis.locator('.settings .problem').bounding_box()}")
            print(f"結果の位置 {width}x{height}: 実行の後のプロットの上端 y={plot_y:.0f}、進捗・止める・失敗の理由は画面の中")
            vis.close()
        # 開く・モデル名（S170）: 2 つのモデルを持つファイルは select で選び直せて、再読み込みで残る。未保存の編集があるタブと同じ名前を開き直しても編集は消えない
        two = "model Helper\n  Real h(start = 1, fixed = true);\nequation\n  der(h) = -h;\nend Helper;\nmodel Main\n  Real m(start = 2, fixed = true);\nequation\n  der(m) = -m;\nend Main;\n"
        op = browser.new_page()
        op.on("console", lambda m: m.type == "error" and errors.append(m.text))
        op.on("pageerror", lambda e: errors.append(str(e)))
        op.goto(origin)
        op.get_by_role("button", name="検査").wait_for()
        open_in(op, "Two.mo", two)
        content, name_box, pick = op.locator(".cm-content"), op.get_by_label("モデル名"), op.get_by_label("ファイルのモデル")
        expect(content).to_contain_text("model Helper", timeout=30_000)
        expect(name_box).to_have_value("Helper")  # 選ばれるのは最初のモデル
        expect(pick).to_have_value("Helper", timeout=30_000)  # それが select に出ている
        assert pick.locator("option").all_inner_texts() == ["Helper", "Main"], pick.locator("option").all_inner_texts()
        pick.select_option("Main")
        expect(name_box).to_have_value("Main")
        run_button = op.get_by_role("button", name="実行")
        run_button.click(force=True)
        expect(run_button).to_be_enabled(timeout=120_000)
        expect(op.locator("p.final")).to_contain_text("m = ", timeout=30_000)  # Main を回した（Helper なら h）
        op.reload()
        expect(op.locator(".cm-content")).to_contain_text("model Helper", timeout=30_000)
        expect(op.get_by_label("モデル名")).to_have_value("Main")  # 再読み込みでタブごとのモデル名が戻る
        expect(op.get_by_label("ファイルのモデル")).to_have_value("Main", timeout=30_000)
        # 未保存の編集があるタブと同じ名前を開き直す（開く・最近使ったファイル）
        op.locator(".cm-content").click()
        op.keyboard.press("Control+End")
        op.keyboard.type("// mine")
        open_in(op, "Two.mo", two)
        expect(op.locator("p[data-saved]")).to_contain_text("未保存の編集", timeout=30_000)
        expect(op.get_by_role("tab")).to_have_count(2)
        expect(op.locator(".cm-content")).to_contain_text("// mine")
        op.get_by_role("tab").nth(0).click()  # 別のタブにいても、開き直すと編集のあるタブへ切り替わるだけ
        expect(op.locator(".cm-content")).to_contain_text("model Decay")
        open_in(op, "Two.mo", two)
        expect(op.get_by_role("tab").nth(1)).to_have_attribute("aria-selected", "true")
        expect(op.locator(".cm-content")).to_contain_text("// mine")
        expect(op.get_by_role("tab")).to_have_count(2)
        # 編集を戻して（保存した本文と同じ）開き直すと、ディスクの内容に差し替わる
        op.locator(".cm-content").click()
        op.keyboard.press("Control+z")
        open_in(op, "Two.mo", two + "// disk\n")
        expect(op.locator(".cm-content")).to_contain_text("// disk")
        print("開く・モデル名: 2 モデルは select で選び直せて再読み込みで残り、未保存の編集があるタブを開き直しても編集は残る")
        if (DIST / "msl/MSL.json").exists():
            # モデル名を打ち替えたのに、エディタは前のファイルのまま（S164 の N8）: ライブラリにある名前なら、食い違いを出して「開く」を促す
            op.get_by_role("button", name="MSL を読む").click(force=True)
            expect(op.locator("p.note[data-msl]")).to_have_attribute("data-msl", "loaded", timeout=120_000)
            pid = "Modelica.Blocks.Examples.PID_Controller"
            op.get_by_label("モデル名").fill(pid)
            check_button = op.get_by_role("button", name="検査")
            check_button.click(force=True)
            expect(check_button).to_be_enabled(timeout=120_000)
            stray = op.locator("[data-stray]")
            expect(stray).to_contain_text("ライブラリのもの", timeout=30_000)
            assert inside(op, "[data-stray]")
            stray.get_by_role("button", name="ライブラリのファイルを開く").click(force=True)
            expect(op.locator(".cm-content")).to_contain_text("package Examples", timeout=60_000)
            expect(op.get_by_label("モデル名")).to_have_value(pid)
            expect(stray).to_have_count(0)
            op.get_by_label("モデル名").fill("Nope.Missing")  # ライブラリにも無い名前は食い違いの案内でなく、検査の診断（error）に出る
            check_button.click(force=True)
            expect(check_button).to_be_enabled(timeout=120_000)
            assert op.locator("[data-stray]").count() == 0
            assert op.locator("tr.error").count() > 0 or op.get_by_role("alert").count() > 0, "存在しないモデル名の理由が出ない"
            print("モデル名の食い違い: ライブラリのモデル名は「開く」を促し、開くと食い違いが消える。存在しない名前は診断に出る")
        op.close()
        # キーボードだけ（S175）: マウスを使わず Tab・Enter・ショートカットで、検査 → 診断の行からエディタへ → 実行 → プロットの代替テキスト → 止める → 保存まで通す
        kb = browser.new_page(viewport={"width": 1280, "height": 800})
        kb.on("console", lambda m: m.type == "error" and errors.append(m.text))
        kb.on("pageerror", lambda e: errors.append(str(e)))
        kb.goto(origin)
        kb.get_by_role("button", name="検査").wait_for()

        def tab_to(js: str, limit: int = 80) -> int:
            """js（document.activeElement を e として評価する式）が真になるまで Tab を押し、押した回数を返す。届かなければ落ちる"""
            for n in range(1, limit + 1):
                if kb.evaluate("!!document.activeElement.closest('.cm-editor')"):
                    kb.keyboard.press("Escape")  # S188: エディタの Tab は字下げ。出るときは Esc のあと Tab
                kb.keyboard.press("Tab")
                if kb.evaluate(f"(() => {{ const e = document.activeElement; return !!e && ({js}); }})()"):
                    return n
            raise AssertionError(f"Tab {limit} 回で届かない: {js}")

        in_editor = "!!e.closest('.cm-editor')"
        tab_to("e.getAttribute('aria-label') === 'モデル名'")
        kb.keyboard.press("Control+a")
        kb.keyboard.type("Bad")
        ring = kb.evaluate("(() => { const s = getComputedStyle(document.activeElement); return [s.outlineStyle, s.outlineWidth]; })()")
        assert ring == ["solid", "2px"], f"フォーカスの輪郭が見えない: {ring}"  # 明（theme.css の :focus-visible）
        tab_to(in_editor)
        expect(kb.locator(".cm-editor")).to_have_css("outline-style", "solid")  # エディタの輪郭も（暗で見えない 1px の点線にしない）
        kb.keyboard.press("Control+a")
        kb.keyboard.insert_text(BAD)
        kb.keyboard.press("Control+Shift+Enter")  # エディタの中から検査
        expect(kb.locator("tr.error").first).to_be_visible(timeout=120_000)
        live = kb.locator("p.sr-only[aria-live=polite]")
        expect(live).to_have_text(re.compile("検査が完了した"))
        assert kb.locator("table.diagnostics button.jump").count() > 0, "診断の行に押せる button が無い"
        tab_to("!!e.closest('table.diagnostics')")
        kb.keyboard.press("Enter")  # 診断の行 → エディタの該当位置
        assert kb.evaluate("!!document.activeElement.closest('.cm-editor')"), "診断の行から Enter してもエディタにフォーカスが移らない"
        assert kb.locator(".cm-selectionBackground").count() > 0 or kb.locator(".cm-activeLine").count() > 0
        kb.keyboard.press("Control+Home")
        kb.keyboard.press("ArrowDown")
        kb.keyboard.press("End")
        kb.keyboard.press("ArrowLeft")
        kb.keyboard.press("Shift+F12")  # 参照の一覧の行も Tab で選べる（Bad の x）
        expect(kb.locator("table.refs tr").first).to_be_visible(timeout=30_000)
        tab_to("!!e.closest('table.refs')")
        kb.keyboard.press("Space")
        assert kb.evaluate("!!document.activeElement.closest('.cm-editor')"), "参照の行から Space してもエディタに移らない"
        # 実行（Ctrl+Enter）: Decay に戻してプロットの代替テキストに信号名と時間の範囲が入る
        kb.keyboard.press("Control+a")
        kb.keyboard.insert_text("model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n")
        kb.get_by_label("モデル名").press("Control+a")
        kb.get_by_label("モデル名").press_sequentially("Decay")
        kb.get_by_label("モデル名").press("Control+Enter")
        alt = kb.locator(".plot canvas")
        expect(alt).to_have_attribute("aria-label", re.compile("Decay.*信号 1 本: x.*time の範囲は 0 から 1"), timeout=120_000)
        assert alt.get_attribute("role") == "img"
        expect(live).to_have_text(re.compile("実行が完了した"))
        print(f"キーボード: {alt.get_attribute('aria-label')!r}")
        # 止める（Ctrl+.）と保存（Ctrl+S）
        kb.locator(".cm-content").press("Control+a")  # フォーカスを戻す（プログラムからの focus。マウスは使わない）
        kb.keyboard.insert_text("model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation(experiment(StopTime = 1e6));\nend Long;\n")
        kb.get_by_label("モデル名").press("Control+a")
        kb.get_by_label("モデル名").press_sequentially("Long")
        kb.get_by_label("モデル名").press("Control+Enter")
        expect(kb.locator("p.run")).to_contain_text("integration", timeout=120_000)
        kb.keyboard.press("Control+.")
        expect(kb.locator("p.run")).to_have_attribute("data-run", "cancelled", timeout=30_000)
        with kb.expect_download(timeout=10_000) as dl:
            kb.keyboard.press("Control+s")
        assert dl.value.suggested_filename.endswith(".mo")
        for name, keys in {"検査": "Control+Shift+Enter", "実行": "Control+Enter", "保存": "Control+S", "開く": "Control+O"}.items():  # ボタンに示してある
            assert kb.get_by_role("button", name=name, exact=True).get_attribute("aria-keyshortcuts").startswith(keys), name
        kb.close()
        # エディタの Tab は字下げ（S188）。出る手は Esc のあと Tab（Shift+Tab で前へ）、Ctrl+M で切り替え。説明は aria-describedby と画面の両方に
        ti = browser.new_page(viewport={"width": 1280, "height": 800})
        ti.on("console", lambda m: m.type == "error" and errors.append(m.text))
        ti.on("pageerror", lambda e: errors.append(str(e)))
        ti.goto(origin)
        ti.get_by_role("button", name="検査").wait_for()
        lines = lambda: ti.eval_on_selector_all(".cm-line", "els => els.map(e => e.textContent)")  # noqa: E731
        in_cm = "!!document.activeElement.closest('.cm-editor')"
        help_id = ti.locator(".cm-content").get_attribute("aria-describedby")
        assert help_id == "editor-help", help_id
        help_text = ti.locator("#editor-help").inner_text()
        assert all(k in help_text for k in ["Tab", "Shift+Tab", "Esc", "Ctrl+M"]), help_text
        assert inside(ti, "#editor-help")
        ti.locator(".cm-content").press("Control+a")
        ti.keyboard.insert_text("x\ny\n")
        ti.keyboard.press("Control+Home")
        ti.keyboard.press("Tab")
        assert lines()[:2] == ["  x", "y"], lines()  # 空白 2 つ（タブ文字ではない）
        assert ti.evaluate(in_cm), "Tab でフォーカスが出た"
        ti.keyboard.press("Shift+Tab")
        assert lines()[:2] == ["x", "y"], lines()
        ti.keyboard.press("Shift+ArrowDown")  # 2 行を選んで字下げし、Shift+Tab で戻す
        ti.keyboard.press("Shift+End")
        ti.keyboard.press("Tab")
        assert lines()[:3] == ["  x", "  y", ""], lines()
        ti.keyboard.press("Shift+Tab")
        assert lines()[:3] == ["x", "y", ""], lines()
        assert "\t" not in "".join(lines())
        # 補完の一覧が開いているときの Tab は候補の確定（S196。Enter と同じ）。一覧が開いていなければ字下げ。Enter で確定は従来どおり
        opt = ti.locator(".cm-tooltip-autocomplete li[aria-selected]")
        ti.keyboard.press("Control+End")
        ti.keyboard.type("mod")
        expect(opt).to_be_visible(timeout=10_000)
        ti.wait_for_timeout(300)  # CodeMirror は一覧が出て 75 ms 以内の確定を無視する（負荷が高いと Enter が改行になる）
        ti.keyboard.press("Enter")
        assert lines()[2].startswith("mod") and lines()[2] != "mod" and len(lines()) == 3, lines()
        ti.keyboard.press("Control+z")
        assert lines()[2] == "mod", lines()
        ti.keyboard.press("Backspace")
        ti.keyboard.type("d")  # 打ち直して一覧を開く
        expect(opt).to_be_visible(timeout=10_000)
        ti.wait_for_timeout(300)  # CodeMirror は一覧が出て 75 ms 以内の確定を無視する
        ti.keyboard.press("Tab")
        assert ti.evaluate(in_cm) and lines()[2].startswith("mod") and lines()[2] != "mod" and len(lines()) == 3, lines()
        ti.keyboard.press("Control+z")
        assert lines()[2] == "mod", lines()
        ti.keyboard.press("ArrowLeft")  # カーソルを動かすと一覧は閉じる（Esc は 2 秒の Tab 移動の窓を開くので使わない）
        expect(opt).to_have_count(0)
        ti.keyboard.press("Tab")
        assert ti.evaluate(in_cm) and lines()[2] == "  mod", lines()  # 一覧が閉じていれば字下げ
        # Esc のあと Tab / Shift+Tab でエディタの外へ。Esc を押さずに 2 秒たったあとの Tab は字下げ
        ti.keyboard.press("Escape")
        ti.keyboard.press("Shift+Tab")
        assert not ti.evaluate(in_cm), "Esc のあとの Shift+Tab でエディタの外の前の要素へ移らない"
        before = ti.evaluate("document.activeElement.tagName + ':' + (document.activeElement.getAttribute('aria-label') || document.activeElement.textContent)")
        assert not before.startswith("BODY"), before
        ti.locator(".cm-content").focus()
        ti.keyboard.press("Escape")
        ti.keyboard.press("Tab")
        assert not ti.evaluate(in_cm), "Esc のあとの Tab でエディタの外の次の要素へ移らない"
        ti.locator(".cm-content").focus()
        ti.keyboard.press("Control+Home")  # 以降は 1 行目の字下げを数える
        n = len(lines()[0]) - len(lines()[0].lstrip())
        ti.keyboard.press("Escape")
        ti.wait_for_timeout(2300)  # 2 秒の窓が閉じる
        ti.keyboard.press("Tab")
        assert ti.evaluate(in_cm) and len(lines()[0]) - len(lines()[0].lstrip()) == n + 2, "窓の外の Tab は字下げ"
        # Ctrl+M で「Tab でフォーカス移動」に切り替わり、もう一度で字下げに戻る
        ti.keyboard.press("Control+m")
        ti.keyboard.press("Tab")
        assert not ti.evaluate(in_cm), "Ctrl+M のあとの Tab でエディタの外へ移らない"
        ti.locator(".cm-content").focus()
        ti.keyboard.press("Control+m")
        ti.keyboard.press("Tab")
        assert ti.evaluate(in_cm) and len(lines()[0]) - len(lines()[0].lstrip()) == n + 4
        # Esc は従来どおり補完の一覧を閉じる（1 回で閉じ、フォーカスはエディタに残る）
        ti.keyboard.press("Control+End")
        ti.keyboard.type(" mod")
        expect(ti.locator(".cm-tooltip-autocomplete")).to_be_visible(timeout=10_000)
        ti.keyboard.press("Escape")
        expect(ti.locator(".cm-tooltip-autocomplete")).to_have_count(0)
        assert ti.evaluate(in_cm)
        ti.close()
        review_fixes(browser, origin, errors)  # S196
        plot_fixes(browser, origin, errors)  # S201
        open_results_file(browser, origin, errors)  # S222
        ui_polish(browser, origin, errors)  # S228
        docs_panel(browser, origin, errors)  # S232
        open_fmu_file(browser, origin, errors)  # S229
        fmu_runtime_select(browser, origin, errors)  # S239
        results_followups(browser, origin, errors)  # S236
        print("エディタの Tab: 空白 2 つの字下げ（選択した 2 行も）と Shift+Tab、補完の一覧が開いていれば確定・閉じていれば字下げ、Esc のあと Tab・Shift+Tab と Ctrl+M で外へ、説明は aria-describedby と画面に")
        print("キーボードだけ: モデル名・エディタへ Tab、Ctrl+Shift+Enter で検査、診断と参照の行を Tab・Enter・Space でエディタへ、Ctrl+Enter で実行（プロットの aria-label に信号名と時間の範囲）、Ctrl+. で止め、Ctrl+S で保存")
        # 初回の案内・スキップリンク・タブの矢印キー・未保存の印（S179）
        fr = browser.new_page(viewport={"width": 1280, "height": 800})
        fr.on("console", lambda m: m.type == "error" and errors.append(m.text))
        fr.on("pageerror", lambda e: errors.append(str(e)))
        unloads: list[str] = []
        fr.on("dialog", lambda d: (unloads.append(d.type), d.accept()))  # 未保存のまま再読み込みすると beforeunload の確認が出る
        fr.goto(origin)
        fr.get_by_role("button", name="検査").wait_for()
        welcome = fr.locator("section.welcome")
        expect(welcome).to_be_visible()
        guide = welcome.inner_text()
        assert all(k in guide for k in ["検査", "実行", "Ctrl/Cmd+Enter", "Ctrl/Cmd+Shift+Enter", "MSL を読む"]), guide
        assert inside(fr, "section.welcome")
        fr.reload()  # 何も書かず・閉じずに開き直しても、まだ初回
        expect(fr.locator("section.welcome")).to_be_visible()
        # スキップリンク: Tab 1 回で「エディタへ」が見え、Enter でエディタ、Tab 2 回で「結果へ」
        fr.get_by_role("button", name="検査").wait_for()
        fr.keyboard.press("Tab")
        assert fr.evaluate("document.activeElement.textContent") == "エディタへ"
        assert fr.evaluate("document.activeElement.getBoundingClientRect().left") >= 0, "スキップリンクがフォーカスで見えない"
        fr.keyboard.press("Tab")
        assert fr.evaluate("document.activeElement.textContent") == "結果へ"
        fr.keyboard.press("Enter")
        assert fr.evaluate("document.activeElement.id") == "results", "結果へ飛ばない"
        # 結果があってページの先頭にいても、「結果へ」はフォーカスと一緒に結果を画面へ入れる（S186）
        fr.keyboard.press("Control+Enter")
        expect(fr.locator(".plot canvas").first).to_be_visible(timeout=120_000)
        fr.get_by_role("link", name="結果へ").focus()  # Tab の起点はエディタに残るので、リンクへ直接（押すのは本物の Enter）
        fr.evaluate("window.scrollTo(0, 0)")
        assert fr.evaluate("document.querySelector('.plot').getBoundingClientRect().top") > 600, "試験の前提: ページの先頭ではプロットが画面の下"
        fr.keyboard.press("Enter")
        fr.wait_for_timeout(300)
        top = fr.evaluate("document.querySelector('.plot').getBoundingClientRect().top")
        assert 0 <= top < 600, f"結果へ飛んでもプロットが画面の下に残る: top = {top}"
        fr.reload()
        fr.get_by_role("button", name="検査").wait_for()
        fr.keyboard.press("Tab")
        fr.keyboard.press("Enter")
        assert fr.evaluate("!!document.activeElement.closest('.cm-editor')"), "エディタへ飛ばない"
        # 案内を閉じると、再読み込みでも出ない（localStorage）。smoke（?smoke=1）では最初から出ない
        fr.get_by_role("button", name="閉じる").click()
        expect(fr.locator("section.welcome")).to_have_count(0)
        assert fr.evaluate("!!document.activeElement.closest('.cm-editor')")
        fr.reload()
        fr.get_by_role("button", name="検査").wait_for()
        expect(fr.locator("section.welcome")).to_have_count(0)
        sm = browser.new_page()  # 別の（保存の無い）ブラウザの文脈で ?smoke=1
        sm.goto(origin + "?smoke=1")
        sm.get_by_role("button", name="検査").wait_for()
        expect(sm.locator("section.welcome")).to_have_count(0)
        sm.close()
        # タブ: 左右の矢印・Home・End で移り（選ぶと開く）、フォーカスと tabindex（roving）が付いてくる
        open_in(fr, "A.mo", "model A\n  Real a(start = 1, fixed = true);\nequation\n  der(a) = -a;\nend A;\n")
        open_in(fr, "B.mo", "model B\n  Real b(start = 1, fixed = true);\nequation\n  der(b) = -b;\nend B;\n")
        ftabs = fr.get_by_role("tab")
        expect(ftabs).to_have_count(3, timeout=30_000)
        expect(ftabs.nth(2)).to_have_attribute("aria-selected", "true")
        assert [ftabs.nth(i).get_attribute("tabindex") for i in range(3)] == ["-1", "-1", "0"]
        ftabs.nth(2).focus()
        for key, want in [("ArrowLeft", 1), ("Home", 0), ("ArrowLeft", 2), ("ArrowRight", 0), ("End", 2)]:  # ArrowLeft は先頭から末尾へ、ArrowRight は末尾から先頭へ回る
            fr.keyboard.press(key)
            expect(ftabs.nth(want)).to_have_attribute("aria-selected", "true")
            assert fr.evaluate(f"document.activeElement === document.querySelectorAll('[role=tab]')[{want}]"), (key, want)
        expect(fr.locator(".cm-content")).to_contain_text("model B")
        # 未保存の印: 編集で付き、再読み込みをまたいでも残り（保存済みの本文との差）、保存で消える
        expect(fr.locator(".dirty")).to_have_count(0)
        fr.locator(".cm-content").click()
        fr.keyboard.type("x")
        expect(fr.locator(".dirty")).to_have_count(1)
        assert fr.locator("[role=tab][aria-selected=true] .dirty").get_attribute("aria-label") == "未保存"
        fr.reload()
        expect(fr.get_by_role("tab")).to_have_count(3, timeout=30_000)
        assert "beforeunload" in unloads, unloads  # 未保存があると再読み込みの前に確かめる
        expect(fr.locator("[role=tab][aria-selected=true] .dirty")).to_have_count(1)
        fr.locator(".cm-content").click()
        with fr.expect_download(timeout=10_000):
            fr.keyboard.press("Control+s")
        expect(fr.locator(".dirty")).to_have_count(0)
        fr.reload()
        expect(fr.get_by_role("tab")).to_have_count(3, timeout=30_000)
        expect(fr.locator(".dirty")).to_have_count(0)
        n = len(unloads)
        fr.reload()
        assert len(unloads) == n, "未保存が無いのに確かめた"
        fr.close()
        print("初回の案内: 初回だけ出て閉じると残り、smoke では出ない。スキップリンクは Tab 1 回でエディタ・2 回で結果、タブは矢印・Home・End で移り、未保存の印は編集で付いて再読み込みをまたぎ保存で消える（未保存のとき beforeunload）")
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
        dark.locator(".cm-content").focus()
        dark.keyboard.press("Control+A")
        dark.keyboard.insert_text(SYN)
        check_syntax(dark)  # S163: 暗でも別々の色で、暗い地に対して読める
        print("色付け（暗）: 同じ 5 種が別々の色で、暗い地に対して 4.5 以上")
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
        assert " isolated=1 cancel_path=flag ready_ms=" in line, line  # S159: serve() が COOP / COEP を付けるので、止めるのは SharedArrayBuffer のフラグの道
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

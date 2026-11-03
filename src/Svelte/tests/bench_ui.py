#!/usr/bin/env python3
"""UI の負荷を測り、doc/bench/ui.md の表を作る（S125、ADR-0034）。

  uv run python src/Svelte/tests/bench_ui.py browser [--runs 3]          # 先に pnpm build（dist/ と msl/MSL.json）
  uv run python src/Svelte/tests/bench_ui.py add svelte-tauri/windows bench.txt --source "desktop run 123"
  uv run python src/Svelte/tests/bench_ui.py report                       # target/ui-bench.json から doc/bench/ui.md

browser: dist/ を配り、Chromium を毎回起動し直して `?smoke=1` を開き、GM-SMOKE の行（ready_ms・decay_ms・msl_ms）と、
Chromium のプロセスのうち最大の VmHWM（Linux だけ。S056 の time -v と同じく木の中の最大の 1 つ）を記録する。
add: desktop.yml の artifact `ui-bench-<os>` の bench.txt（GM-SMOKE の行に rss_kib= を足したものと GM-BUNDLE kib= の行）を取り込む。
結果は target/ui-bench.json の「<UI>/<OS>」の項にまとめる（同じ項は上書き）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
JSON_PATH = ROOT / "target/ui-bench.json"
DOC = ROOT / "doc/bench/ui.md"
UIS = [("svelte-browser", "Svelte ブラウザ"), ("svelte-tauri", "Svelte Tauri"), ("leptos", "Leptos"), ("dioxus", "Dioxus")]
OSES = {"linux": "Linux", "windows": "Windows", "macos": "macOS"}
ROWS = [
    ("ready_ms", "起動から操作可能まで（ms）", 1),
    ("decay_ms", "Decay の検査・実行・プロット（ms）", 1),
    ("msl_ms", "MSL を読み ChuaCircuit を検査（ms）", 1),
    ("rss_kib", "最大の RSS（MiB）", 1024),
    ("bundle_kib", "バンドル（MiB）", 1024),
]


def fields(line: str) -> dict[str, str]:
    return dict(re.findall(r"(\w+)=(\S+)", line.split(" failure=")[0]))


def run_fields(line: str) -> dict[str, float]:
    f = fields(line)
    if not (f.get("check") == f.get("simulate") == f.get("msl") == "ok"):
        raise SystemExit(f"GM-SMOKE が ok でない: {line}")
    return {k: float(v) for k, v in f.items() if k.endswith(("_ms", "_kib"))}


def peak_rss_kib() -> float | None:
    """このプロセスの子孫（Playwright の driver と Chromium）のうち、最大の VmHWM（KiB）。Linux 以外は None。"""
    if not Path("/proc/self/status").exists():
        return None
    parent: dict[int, int] = {}
    for d in Path("/proc").iterdir():
        if d.name.isdigit():
            try:
                parent[int(d.name)] = int((d / "stat").read_text().rsplit(")", 1)[1].split()[1])
            except (OSError, IndexError, ValueError):
                continue
    mine, grew = {os.getpid()}, True
    while grew:
        more = {p for p, pp in parent.items() if pp in mine} - mine
        mine |= more
        grew = bool(more)
    peak = 0.0
    for pid in mine - {os.getpid()}:
        try:
            m = re.search(r"VmHWM:\s+(\d+)", Path(f"/proc/{pid}/status").read_text())
        except OSError:
            continue
        peak = max(peak, float(m[1])) if m else peak
    return peak


def browser(runs: int) -> dict:
    from playwright.sync_api import sync_playwright

    from test_ui import DIST, serve

    assert (DIST / "msl/MSL.json").exists(), "dist/msl/MSL.json が無い（data/msl を取得して pnpm build）"
    server = serve()
    out = []
    with sync_playwright() as p:
        for i in range(runs):
            browser = p.chromium.launch()  # 毎回起動し直す（wasm と MSL.json の cache を持ち越さない）
            page = browser.new_page()
            with page.expect_console_message(lambda m: m.text.startswith("GM-SMOKE"), timeout=600_000) as message:
                page.goto(f"http://127.0.0.1:{server.server_port}/?smoke=1")
            r = run_fields(message.value.text)
            if (rss := peak_rss_kib()) is not None:
                r["rss_kib"] = rss
            print(f"{i + 1}/{runs}: {message.value.text}  rss_kib={r.get('rss_kib')}")
            out.append(r)
            browser.close()
    server.shutdown()
    bundle = sum(f.stat().st_size for f in DIST.rglob("*") if f.is_file() and "msl" not in f.relative_to(DIST).parts)
    return {"runs": out, "bundle_kib": bundle / 1024}


def parse_bench(text: str) -> dict:
    runs = [run_fields(line) for line in text.splitlines() if line.startswith("GM-SMOKE")]
    assert runs, "GM-SMOKE の行が無い"
    entry: dict = {"runs": runs}
    if m := re.search(r"^GM-BUNDLE kib=(\d+)", text, re.M):
        entry["bundle_kib"] = float(m[1])
    return entry


def median(entry: dict, key: str) -> float | None:
    if key == "bundle_kib":
        return entry.get("bundle_kib")
    xs = [r[key] for r in entry["runs"] if key in r]
    return statistics.median(xs) if xs else None


def report(data: dict) -> str:
    cols = [(f"{ui}/{os_}", f"{name} {OSES[os_]}") for ui, name in UIS for os_ in OSES]
    lines = [
        "# UI の比較（S125〜）",
        "",
        "<!-- src/Svelte/tests/bench_ui.py report が target/ui-bench.json から作る。手で直さない -->",
        "",
        "測り方は [ADR-0034](../adr/0034-ui-bench-method.md)。数は回の中央値（バンドルは 1 回）。「未」はまだ測っていない（UI が無い・OS が matrix に無い）。",
        "",
        "| 測るもの | " + " | ".join(h for _, h in cols) + " |",
        "| --- | " + " | ".join("---:" for _ in cols) + " |",
    ]
    for key, label, div in ROWS:
        cells = []
        for k, _ in cols:
            v = median(data[k], key) if k in data else None
            cells.append("未" if v is None else f"{v / div:,.0f}" if div == 1 else f"{v / div:,.1f}")
        lines.append(f"| {label} | " + " | ".join(cells) + " |")
    lines += ["", "## 出典", ""]
    lines += [f"- {k}: {data[k]['source']}（{len(data[k]['runs'])} 回）" for k, _ in cols if k in data]
    return "\n".join(lines) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("browser")
    b.add_argument("--runs", type=int, default=3)
    b.add_argument("--source", default="手元")
    a = sub.add_parser("add")
    a.add_argument("key")
    a.add_argument("file", type=Path)
    a.add_argument("--source", required=True)
    sub.add_parser("report")
    args = ap.parse_args()
    data = json.loads(JSON_PATH.read_text(encoding="utf-8")) if JSON_PATH.exists() else {}
    if args.cmd == "report":
        DOC.parent.mkdir(parents=True, exist_ok=True)
        DOC.write_text(report(data), encoding="utf-8")
        print(DOC)
        return 0
    if args.cmd == "browser":
        os_ = {"linux": "linux", "win32": "windows", "darwin": "macos"}[sys.platform]
        key, entry = f"svelte-browser/{os_}", browser(args.runs)
    else:
        key, entry = args.key, parse_bench(args.file.read_text(encoding="utf-8"))
    assert key in {f"{ui}/{o}" for ui, _ in UIS for o in OSES}, key
    data[key] = {"source": args.source, **entry}
    JSON_PATH.parent.mkdir(parents=True, exist_ok=True)
    JSON_PATH.write_text(json.dumps(data, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{key}: " + ", ".join(f"{k}={median(entry, k)}" for k, _, _ in ROWS))
    return 0


if __name__ == "__main__":
    # 自己検査: 行の解析と表（Leptos・Dioxus は「未」）
    _e = parse_bench("GM-SMOKE check=ok simulate=ok x1=0.36 msl=ok ready_ms=10 decay_ms=20 msl_ms=30 rss_kib=2048\nGM-BUNDLE kib=1024\n")
    assert _e == {"runs": [{"ready_ms": 10, "decay_ms": 20, "msl_ms": 30, "rss_kib": 2048}], "bundle_kib": 1024}, _e
    assert "| 2.0 |" in report({"svelte-tauri/linux": {"source": "x", **_e}}) and "未" in report({})
    sys.exit(main())

#!/usr/bin/env python3
"""npm のサードパーティのライセンス表 THIRD-PARTY-npm.md を作る（S380。Rust 側の cargo-about の THIRD-PARTY*.md の npm 版）。

使い方: (cd src/Svelte && pnpm licenses list --long --json) | npm_licenses.py > THIRD-PARTY-npm.md
                                                          npm_licenses.py --test

入力は pnpm の JSON（ライセンス名 → パッケージの一覧）。各パッケージの LICENSE の写しを、入っている場所（paths）から読んで載せる。
許可外のライセンス（GPL・LGPL・AGPL・NC など）が 1 つでもあれば、表を出さずに終了コード 1。
dev 依存も含める（Svelte の実行時などが dist に入るので、prod だけでは足りない。dist に入らないビルド用の分も載るが、多めに載せる側）。
"""

import json
import os
import sys

# deny.toml の allow と同じ集合（Rust 側と同じ基準）。MPL-2.0 は Vite 8 が引く lightningcss だけ（ビルド時の CSS 変換。dist に入らず改変もしない。doc/licenses.md）
ALLOWED = {
    "Apache-2.0", "MIT", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause",
    "ISC", "Zlib", "Unicode-3.0", "CC0-1.0", "BSL-1.0", "MPL-2.0",
}


def allowed(expr):
    """SPDX の式。OR はどれか 1 つ、AND は全部が許可のとき通す（括弧は無視する）。"""
    options = [o.replace("(", "").replace(")", "").split(" AND ") for o in expr.split(" OR ")]
    return any(all(t.strip() in ALLOWED for t in o) for o in options)


def license_text(path):
    names = sorted(f for f in os.listdir(path) if f.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")))
    return "\n\n".join(open(os.path.join(path, f), encoding="utf-8", errors="replace").read().strip() for f in names)


def render(data):
    bad = sorted(k for k in data if not allowed(k))
    if bad:
        sys.exit(f"許可外のライセンス: {bad}（{[p['name'] for k in bad for p in data[k]]}）")
    out = [
        "# Third-party licenses (npm) / サードパーティのライセンス（npm）",
        "",
        "This file lists the npm packages that `src/Svelte` (the GlobeMode UI) installs, with their license texts. It is generated from `pnpm licenses list`."
        " It also covers build-time packages, so it may list more than the UI bundle contains. GlobeMode itself is MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).",
        "",
        "GlobeMode の UI（`src/Svelte`）が入れる npm パッケージと、そのライセンス文の一覧。`pnpm licenses list` から作った。"
        "ビルド時だけのパッケージも含むので、UI の配布物に入るより多く載る。",
        "",
        "## Overview / 概要",
        "",
    ]
    out += [f"- {k}: {len(v)}" for k, v in sorted(data.items(), key=lambda kv: -len(kv[1]))]
    for lic, pkgs in sorted(data.items()):
        for p in sorted(pkgs, key=lambda p: p["name"]):
            for ver, path in zip(p["versions"], p["paths"]):
                out += ["", f"## {p['name']} {ver} (`{lic}`)", ""]
                if p.get("homepage"):
                    out += [f"{p['homepage']}", ""]
                out += ["~~~~text", license_text(path) or f"(no license file; license field: {lic})", "~~~~"]
    return "\n".join(out) + "\n"


def test():
    assert allowed("MIT") and allowed("MIT OR GPL-3.0-or-later") and allowed("(MIT AND Zlib)")
    assert not allowed("GPL-3.0-only") and not allowed("MIT AND GPL-2.0") and not allowed("CC-BY-NC-4.0")
    try:
        render({"GPL-3.0-only": [{"name": "x", "versions": ["1"], "paths": ["."]}]})
    except SystemExit as e:
        assert "許可外" in str(e)
    else:
        raise AssertionError("GPL を止めなかった")
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        test()
    else:
        sys.stdout.write(render(json.load(sys.stdin)))

#!/usr/bin/env python3
"""変更されたファイルから、CI で何を検査するかを決める（ADR-0006）。

  python3 .github/scripts/ci_plan.py --base origin/main --head HEAD     # 手元で計画を見る
  python3 .github/scripts/ci_plan.py --base <sha> --head <sha> --github-output "$GITHUB_OUTPUT"

方針:
  - 変更した crate と、それに依存する crate（逆依存の閉包）だけを fmt 以外で検査する。
    fmt は安いので Rust を触ったら全体にかける。
  - 依存の集合（Cargo.lock / Cargo.toml / deny.toml）を変えていなければ、cargo deny は省く。
    依存は前回の CI で検証済みのため。
  - 次のときは全体を検査する（full）:
      * 依存・ツール・CI の定義を変えた（Cargo.toml / Cargo.lock / mise.toml / mise.lock / deny.toml / .github/）
      * 変更したファイルが LARGE_CHANGE 件を超えた（大規模な変更）
      * コミットメッセージに [full-ci] がある
      * 定期実行（schedule）、手動実行で full を指定、比較元が無い（新しいブランチ・履歴の書き換え）

出力（JSON。--github-output を付けると GitHub Actions の出力にも書く）:
  full, rust, rust_packages（"-p a -p b"）, wasm, deny, python, docs, slides, msl, reasons
標準ライブラリだけで動く（Python 3.11 以上、tomllib を使う）。
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from fnmatch import fnmatch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LARGE_CHANGE = 40
ZERO_SHA = "0" * 40

# これを変えたら全体を検査する
FULL_TRIGGERS = ["Cargo.toml", "Cargo.lock", "mise.toml", "mise.lock", "deny.toml", ".github/*"]
# 依存の集合が変わる（cargo deny が要る）
DEPENDENCY_FILES = ["Cargo.toml", "Cargo.lock", "deny.toml", "src/Rust/mod/*/Cargo.toml"]
# wasm でビルドできることを守る crate
WASM_CRATES = {"globe-mode-core", "globe-mode-ir", "globe-mode-frontend", "globe-mode-graph", "globe-mode-db"}
# Rust 以外のファイルで、特定の crate のテストが読むもの
EXTRA_INPUTS = {
    "src/Modelica/msl-compare.txt": {"xtask"},
    "src/Modelica/*": {"globe-mode-frontend", "globe-mode-cli", "globe-mode-db"},
}
PYTHON_FILES = ["pyproject.toml", "uv.lock", ".python-version", "src/Python/*"]
DOC_FILES = ["*.md", "doc/*", "log/*", ".claude/skills/*", "src/*/README.md"]
SLIDE_FILES = ["doc/sessions/*", "doc/assets/*", ".claude/skills/session-docs/*"]


def match(path: str, patterns: list[str]) -> bool:
    return any(fnmatch(path, p) for p in patterns)


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def workspace_crates() -> dict[str, dict]:
    """crate 名 → {dir, deps（workspace 内の依存）}。Cargo.toml を直接読む（cargo が無くても動く）。"""
    crates = {}
    manifests = sorted((ROOT / "src/Rust/mod").glob("globe-mode-*/Cargo.toml"))
    manifests += [m for m in [ROOT / "src/Rust/xtask/Cargo.toml"] if m.exists()]
    for manifest in manifests:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        deps = set()
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            deps |= {d for d in data.get(table, {}) if d.startswith("globe-mode-") or d == "xtask"}
        crates[data["package"]["name"]] = {"dir": manifest.parent.relative_to(ROOT).as_posix(), "deps": deps}
    return crates


def affected_crates(changed: list[str], crates: dict[str, dict]) -> set[str]:
    """変更が直接触れた crate と、それに依存する crate すべて。"""
    direct = set()
    for path in changed:
        for name, c in crates.items():
            if path.startswith(c["dir"] + "/"):
                direct.add(name)
        # 最初に当たった規則だけを使う（具体的な規則を先に書く）
        for pattern, names in EXTRA_INPUTS.items():
            if fnmatch(path, pattern):
                direct |= names & crates.keys()
                break
    result = set(direct)
    grew = True
    while grew:
        grew = False
        for name, c in crates.items():
            if name not in result and c["deps"] & result:
                result.add(name)
                grew = True
    return result


def plan(changed: list[str] | None, message: str = "", force_full: bool = False, force_reason: str = "") -> dict:
    crates = workspace_crates()
    reasons: list[str] = []
    if force_full:
        reasons.append(force_reason or "full を指定")
    if changed is None:
        if not force_full:
            reasons.append("比較元が無い")
        changed = []
    if "[full-ci]" in message:
        reasons.append("コミットメッセージに [full-ci]")
    triggers = sorted({p for p in changed if match(p, FULL_TRIGGERS)})
    if triggers:
        reasons.append("依存・ツール・CI の定義を変更: " + ", ".join(triggers[:5]))
    if len(changed) > LARGE_CHANGE:
        reasons.append(f"大規模な変更（{len(changed)} ファイル > {LARGE_CHANGE}）")
    full = bool(reasons)

    if full:
        affected = set(crates)
    else:
        affected = affected_crates(changed, crates)

    out = {
        "full": full,
        "rust": bool(affected),
        "rust_packages": " ".join(f"-p {c}" for c in sorted(affected)),
        "wasm": full or bool(affected & WASM_CRATES),
        "deny": full or any(match(p, DEPENDENCY_FILES) for p in changed),
        "python": full or any(match(p, PYTHON_FILES) for p in changed),
        "docs": full or any(match(p, DOC_FILES) for p in changed),
        "slides": full or any(match(p, SLIDE_FILES) for p in changed),
        # MSL での Rumoca との突き合わせは重い（MSL と Rumoca の CLI の取得、20 本の検査）ので、全体を検査するときだけ
        "msl": full,
        "changed_files": len(changed),
        "reasons": "; ".join(reasons) if reasons else "差分のみ",
    }
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--base", help="比較元のコミット（push の before、PR の base）")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("--full", action="store_true", help="全体を検査する")
    ap.add_argument("--reason", default="", help="--full の理由（schedule / workflow_dispatch など）")
    ap.add_argument("--github-output", help="GitHub Actions の $GITHUB_OUTPUT のパス")
    args = ap.parse_args(argv)

    changed: list[str] | None = None
    message = ""
    if args.base and args.base != ZERO_SHA:
        try:
            changed = [p for p in git("diff", "--name-only", f"{args.base}...{args.head}").splitlines() if p]
            message = git("log", "--format=%B", f"{args.base}..{args.head}")
        except subprocess.CalledProcessError:
            changed = None  # 比較元が履歴に無い（force push など）→ 全体を検査する

    result = plan(changed, message, args.full, args.reason)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    if args.github_output:
        with open(args.github_output, "a", encoding="utf-8") as f:
            for k, v in result.items():
                f.write(f"{k}={str(v).lower() if isinstance(v, bool) else v}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

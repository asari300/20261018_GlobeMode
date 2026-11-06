#!/usr/bin/env python3
"""release.yml の最初のジョブ: タグの版が、成果物に入る版（Cargo・desktop・tauri.conf.json・pyproject）と合っているか確かめる（S195）。

使い方: check_release_tag.py <タグ>   /   check_release_tag.py --test
pre-release の接尾辞（-rc1 など）は自由。接尾辞を除いた版が、4 か所と同じでなければ止める。
成果物の版は 4 か所のもの（rc でも 0.2.0 のまま）。MSI は pre-release に数字しか許さないので、接尾辞は版に入れない。
"""

import json
import re
import sys
import tomllib
from pathlib import Path

TAG_RE = re.compile(r"^v(\d+\.\d+\.\d+)(-[0-9A-Za-z.]+)?$")
ROOT = Path(__file__).resolve().parents[2]


def file_versions():
    toml = lambda p: tomllib.loads((ROOT / p).read_text(encoding="utf-8"))  # noqa: E731
    return {
        "Cargo.toml [workspace.package]": toml("Cargo.toml")["workspace"]["package"]["version"],
        "src/Rust/apps/desktop/Cargo.toml": toml("src/Rust/apps/desktop/Cargo.toml")["package"]["version"],
        "src/Rust/apps/desktop/tauri.conf.json": json.loads((ROOT / "src/Rust/apps/desktop/tauri.conf.json").read_text(encoding="utf-8"))["version"],
        "pyproject.toml": toml("pyproject.toml")["project"]["version"],
    }


def problems(tag, versions):
    m = TAG_RE.match(tag)
    if not m:
        return [f"タグ {tag!r} が v<数字>.<数字>.<数字>[-<接尾辞>]（例 v0.2.0、v0.2.0-rc1）の形でない"]
    return [f"{k} の版 {v} がタグの版 {m.group(1)} と違う（doc/architecture.md の 2.4 の表のとおり揃えてから打つ）" for k, v in versions.items() if v != m.group(1)]


def test():
    ok = {"a": "0.2.0", "b": "0.2.0"}
    assert problems("v0.2.0", ok) == [] and problems("v0.2.0-rc1", ok) == []
    assert len(problems("v0.2.1", ok)) == 2
    assert len(problems("0.2.0", ok)) == 1 and len(problems("v0.2", ok)) == 1
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        test()
        sys.exit(0)
    errs = problems(sys.argv[1], file_versions())
    for e in errs:
        print(f"::error::{e}")
    print(f"{sys.argv[1]}: " + ("NG" if errs else "OK"))
    sys.exit(1 if errs else 0)

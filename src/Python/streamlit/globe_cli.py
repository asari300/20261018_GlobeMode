"""globe CLI（`globe fmt --json -` / `globe lint --json -`）を子プロセスで呼ぶ。計算は Rust 側だけ。"""

import json
import os
import subprocess


MAX_BODY = 1 << 20  # doc/web-apps.md §3.1


class GlobeError(Exception):
    """globe を起動できない、または JSON を返さなかった。"""


def _run(args: list[str], text: str) -> dict:
    if len(text.encode()) > MAX_BODY:
        raise GlobeError("入力が 1 MiB を超えている")
    try:
        proc = subprocess.run(
            args,
            input=text,
            capture_output=True,
            text=True,
            timeout=int(os.environ.get("GLOBE_TIMEOUT_MS", "10000")) / 1000,
        )
    except subprocess.TimeoutExpired as e:
        raise GlobeError("globe が時間内に終わらなかった") from e
    except OSError as e:
        raise GlobeError(f"globe を起動できません（GLOBE_BIN={args[0]!r}）: {e}") from e
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        # 終了コード 1 の文法エラーは JSON で返る。ここに来るのは CLI の異常終了だけ。
        raise GlobeError(proc.stderr.strip() or f"globe が終了コード {proc.returncode} で JSON を返さなかった")


def fmt(text: str, indent: int = 2, insert_origin: bool = True) -> dict:
    """{"ok", "text", "error"} を返す。文法が壊れていれば ok は False。"""
    args = [os.environ.get("GLOBE_BIN", "globe"), "fmt", "--json", "-", "--indent", str(indent)]
    if not insert_origin:
        args.append("--no-insert-origin")
    return _run(args, text)


def lint(text: str) -> list[dict]:
    """{"file", "rule", "line", "column", "message"} の並びを返す。"""
    return _run([os.environ.get("GLOBE_BIN", "globe"), "lint", "--json", "-"], text)["messages"]

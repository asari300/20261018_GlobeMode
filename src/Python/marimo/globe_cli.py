"""globe CLI（`globe fmt --json` / `globe lint --json`）を子プロセスで呼ぶ。計算は Rust 側だけ。

バイナリは環境変数 `GLOBE_BIN`（既定は PATH の `globe`）。終了コード 1 は正常な結果（整形できない / 指摘がある）。
"""

import json
import os
import subprocess


def _run(args: list[str], text: str) -> subprocess.CompletedProcess[str]:
    binary = os.environ.get("GLOBE_BIN", "globe")
    try:
        return subprocess.run([binary, *args], input=text, capture_output=True, text=True)
    except OSError as e:  # 無い・実行できないときも RuntimeError にして画面に出す
        raise RuntimeError(f"globe を起動できない（GLOBE_BIN={binary}）: {e}") from e


def _parse(proc: subprocess.CompletedProcess[str], what: str) -> dict:
    if proc.returncode not in (0, 1):
        raise RuntimeError(f"globe {what} が異常終了（終了コード {proc.returncode}）: {proc.stderr.strip()}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        raise RuntimeError(f"globe {what} の出力を読めない: {proc.stdout[:200]}") from e


def format_text(text: str, indent: int = 2, insert_origin: bool = True) -> dict:
    """{"ok": bool, "text": str | None, "error": str | None} を返す。"""
    args = ["fmt", "--json", "--indent", str(indent)]
    if not insert_origin:
        args.append("--no-insert-origin")
    return _parse(_run(args, text), "fmt")


def lint_text(text: str) -> list[dict]:
    """指摘の一覧（rule / line / column / message など）を返す。0 件なら空のリスト。"""
    return _parse(_run(["lint", "--json"], text), "lint")["messages"]

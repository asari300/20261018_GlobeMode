"""GlobeMode の整形と lint を HTTP で提供する。計算は globe CLI（子プロセス）が行う。"""

import json
import os
import subprocess
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from pydantic import BaseModel, Field

INDEX = Path(__file__).parent / "static" / "index.html"
TIMEOUT_S = 60  # shortcut: 同期の呼び出しを 60 秒で切るだけ, 同時に多くの重い要求が来て詰まったら非同期化やキューを検討する

app = FastAPI(title="GlobeMode 整形・lint（FastAPI）")


class FormatRequest(BaseModel):
    text: str
    indent: int = Field(2, ge=0, le=16)
    width: int = Field(80, ge=1)
    insert_origin: bool = True


class LintRequest(BaseModel):
    text: str
    disable: list[str] = []


def run_globe(args: list[str], text: str) -> subprocess.CompletedProcess:
    binary = os.environ.get("GLOBE_BIN", "globe")
    try:
        # 終了コード 1 は「文法が壊れている」「指摘がある」の正常な結果なので、ここでは失敗にしない
        return subprocess.run(
            [binary, *args],
            input=text,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=TIMEOUT_S,
        )
    except FileNotFoundError as e:
        raise HTTPException(503, f"globe を起動できない（GLOBE_BIN={binary}）: {e}") from e
    except subprocess.TimeoutExpired as e:
        raise HTTPException(504, "globe が時間内に終わらなかった") from e


def parse_json(proc: subprocess.CompletedProcess) -> dict:
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        raise HTTPException(500, f"globe の出力を読めない: {proc.stderr.strip()}") from e


@app.get("/", include_in_schema=False)
def index() -> FileResponse:
    return FileResponse(INDEX, media_type="text/html; charset=utf-8")


@app.get("/healthz")
def healthz() -> dict:
    return {"ok": True}


@app.post("/api/format")
def format_text(req: FormatRequest) -> dict:
    args = ["fmt", "--json", "-", "--indent", str(req.indent), "--width", str(req.width)]
    if not req.insert_origin:
        args.append("--no-insert-origin")
    return parse_json(run_globe(args, req.text))


@app.post("/api/lint")
def lint_text(req: LintRequest) -> dict:
    args = ["lint", "--json", "-"]
    for rule in req.disable:
        args += ["--disable", rule]
    data = parse_json(run_globe(args, req.text))
    keys = ("rule", "line", "column", "message")
    return {"messages": [{k: m[k] for k in keys} for m in data["messages"]]}

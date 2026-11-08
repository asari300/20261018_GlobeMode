"""GlobeMode の整形と lint を HTTP で提供する。計算は globe CLI（子プロセス）が行う。"""

import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Annotated

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel, Field, StringConstraints

INDEX = Path(__file__).parent / "static" / "index.html"
MAX_BODY = 1 << 20  # doc/web-apps.md §3.1
RULE = r"^[a-z][a-z-]*$"  # disable の値を CLI のオプションと取り違えない

log = logging.getLogger("uvicorn.error")

app = FastAPI(title="GlobeMode 整形・lint（FastAPI）")


def timeout_s() -> float:
    # shortcut: 同期の呼び出しを切るだけ, 同時に多くの重い要求が来て詰まったら非同期化やキューを検討する
    return int(os.environ.get("GLOBE_TIMEOUT_MS", "10000")) / 1000


@app.middleware("http")
async def limit_body(request: Request, call_next):
    # shortcut: Content-Length だけを見る。chunked の本文は run_globe が読み終えたあとで測る, 巨大な chunked を防ぐならサーバ（uvicorn の前段）で切る
    if int(request.headers.get("content-length") or 0) > MAX_BODY:
        return JSONResponse({"detail": "本文が 1 MiB を超えている"}, status_code=413)
    return await call_next(request)


class FormatRequest(BaseModel):
    text: str
    indent: int = Field(2, ge=0, le=16)
    width: int = Field(80, ge=1)
    insert_origin: bool = True


class LintRequest(BaseModel):
    text: str
    disable: list[Annotated[str, StringConstraints(pattern=RULE)]] = []


def run_globe(args: list[str], text: str) -> subprocess.CompletedProcess:
    if len(text.encode()) > MAX_BODY:
        raise HTTPException(413, "本文が 1 MiB を超えている")
    binary = os.environ.get("GLOBE_BIN", "globe")
    try:
        # 終了コード 1 は「文法が壊れている」「指摘がある」の正常な結果なので、ここでは失敗にしない
        return subprocess.run(
            [binary, *args],
            input=text,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=timeout_s(),
        )
    except FileNotFoundError as e:
        # 応答には内部のパスを出さず、詳細はサーバのログに残す（#82）
        log.error("globe を起動できない（GLOBE_BIN=%s）: %s", binary, e)
        raise HTTPException(503, "globe を起動できない") from e
    except subprocess.TimeoutExpired as e:
        raise HTTPException(504, "globe が時間内に終わらなかった") from e


def parse_json(proc: subprocess.CompletedProcess) -> dict:
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        log.error("globe の出力を読めない: %s", proc.stderr.strip())
        raise HTTPException(500, "globe の出力を読めない") from e


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
    args = ["lint", "--json", "-", "--disable", "connect-type-mismatch"]  # #73: 型検査は 1 MiB で 2.6 GB になる
    for rule in req.disable:
        args += ["--disable", rule]
    data = parse_json(run_globe(args, req.text))
    keys = ("rule", "line", "column", "message")
    return {"messages": [{k: m[k] for k in keys} for m in data["messages"]]}

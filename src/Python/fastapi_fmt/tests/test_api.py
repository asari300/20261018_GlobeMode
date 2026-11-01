from pathlib import Path

import httpx
import pytest

from app import app

# worktree の src/Rust/... を指す（tests/ から 4 段上がるとリポジトリの根）
SAMPLE = Path(__file__).resolve().parents[4] / "src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo"


@pytest.fixture
def client():
    transport = httpx.ASGITransport(app=app)
    return httpx.AsyncClient(transport=transport, base_url="http://test")


@pytest.mark.anyio
async def test_healthz(client):
    async with client as c:
        r = await c.get("/healthz")
    assert r.status_code == 200
    assert r.json() == {"ok": True}


@pytest.mark.anyio
async def test_index_is_html(client):
    async with client as c:
        r = await c.get("/")
    assert r.status_code == 200
    assert "text/html" in r.headers["content-type"]
    assert 'id="format"' in r.text


@pytest.mark.anyio
async def test_format_sample(client):
    async with client as c:
        r = await c.post("/api/format", json={"text": SAMPLE.read_text(encoding="utf-8")})
    assert r.status_code == 200
    body = r.json()
    assert body["ok"] is True
    assert body["error"] is None
    assert "origin = {0, 0}" in body["text"]


@pytest.mark.anyio
async def test_format_syntax_error_is_ok_false(client):
    async with client as c:
        r = await c.post("/api/format", json={"text": "model A\n  Real x\nend A;\n"})
    assert r.status_code == 200
    body = r.json()
    assert body["ok"] is False
    assert body["text"] is None
    assert body["error"]


@pytest.mark.anyio
async def test_lint_sample(client):
    async with client as c:
        r = await c.post("/api/lint", json={"text": SAMPLE.read_text(encoding="utf-8")})
    assert r.status_code == 200
    messages = r.json()["messages"]
    assert isinstance(messages, list)
    assert messages, "sample has at least one lint message"
    for m in messages:
        assert set(m) == {"rule", "line", "column", "message"}


@pytest.mark.anyio
async def test_missing_binary_is_503(client, monkeypatch):
    monkeypatch.setenv("GLOBE_BIN", "/nonexistent/globe")
    async with client as c:
        r = await c.post("/api/lint", json={"text": "model A end A;"})
    assert r.status_code == 503

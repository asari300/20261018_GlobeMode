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


@pytest.mark.anyio
async def test_oversize_body_is_413(client):
    async with client as c:
        r = await c.post("/api/format", json={"text": "x" * (1 << 20)})  # JSON の枠で 1 MiB を超える
    assert r.status_code == 413


@pytest.mark.anyio
async def test_slow_globe_is_504(client, monkeypatch, tmp_path):
    slow = tmp_path / "slow.sh"
    slow.write_text("#!/bin/sh\nexec sleep 30\n")
    slow.chmod(0o755)
    monkeypatch.setenv("GLOBE_BIN", str(slow))
    monkeypatch.setenv("GLOBE_TIMEOUT_MS", "200")
    async with client as c:
        r = await c.post("/api/format", json={"text": "model A end A;"})
    assert r.status_code == 504


@pytest.mark.anyio
async def test_disable_must_be_a_rule_name(client):
    async with client as c:
        r = await c.post("/api/lint", json={"text": "model A end A;", "disable": ["--json"]})
    assert r.status_code == 422


@pytest.mark.anyio
async def test_lint_always_disables_the_connect_type_check(client, monkeypatch, tmp_path):
    # #73: 1 MiB の lint が 11.8 s・RSS 2.6 GB になるのを避けるため、型検査は常に止める（引数を JSON で返す偽の globe）
    fake = tmp_path / "globe"
    fake.write_text('#!/bin/sh\nprintf \'{"messages":[{"rule":"%s","line":1,"column":1,"message":"m"}]}\' "$*"\n')
    fake.chmod(0o755)
    monkeypatch.setenv("GLOBE_BIN", str(fake))
    async with client as c:
        r = await c.post("/api/lint", json={"text": "model A end A;", "disable": ["line-too-long"]})
    assert "--disable connect-type-mismatch" in r.json()["messages"][0]["rule"]

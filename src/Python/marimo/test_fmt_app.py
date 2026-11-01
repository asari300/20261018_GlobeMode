"""fmt_app が呼ぶ globe CLI を、例題 objects_before.mo で 1 往復させる（GLOBE_BIN、既定は PATH の globe）。"""

from pathlib import Path

from globe_cli import format_text, lint_text

SAMPLE = Path(__file__).parents[2] / "Rust" / "mod" / "globe-mode-fmt" / "tests" / "data" / "objects_before.mo"


def test_format_inserts_origin():
    source = SAMPLE.read_text(encoding="utf-8")
    result = format_text(source)
    assert result["ok"], result["error"]
    assert "origin = {0, 0}" in result["text"]
    assert "origin = {0, 0}" not in format_text(source, insert_origin=False)["text"]


def test_format_reports_syntax_error():
    result = format_text("model A")
    assert not result["ok"] and result["text"] is None and result["error"]


def test_lint_returns_list():
    messages = lint_text(SAMPLE.read_text(encoding="utf-8"))
    assert isinstance(messages, list)
    assert all({"rule", "line", "column", "message"} <= m.keys() for m in messages)

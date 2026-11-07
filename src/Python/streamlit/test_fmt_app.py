"""fmt_app.py を AppTest で 1 往復する。実行には GLOBE_BIN（globe のバイナリ）が要る。"""

from pathlib import Path

import pytest

from globe_cli import GlobeError, fmt
from streamlit.testing.v1 import AppTest

SAMPLE = Path(__file__).resolve().parents[3] / "src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo"


def test_format_sample_inserts_origin():
    at = AppTest.from_file(str(Path(__file__).with_name("fmt_app.py")), default_timeout=60).run()
    at.text_area(key="source").input(SAMPLE.read_text(encoding="utf-8")).run()
    next(b for b in at.button if b.label == "整形").click().run()

    assert not at.exception
    (out,) = [t for t in at.text_area if t.label == "整形後（読み取り専用）"]
    assert "origin = {0, 0}" in out.value


def test_lint_sample_shows_table_and_broken_input_shows_error():
    at = AppTest.from_file(str(Path(__file__).with_name("fmt_app.py")), default_timeout=60).run()
    at.text_area(key="source").input(SAMPLE.read_text(encoding="utf-8")).run()
    next(b for b in at.button if b.label == "lint").click().run()
    assert not at.exception
    assert at.table[0].value.to_dict("records")[0]["rule"] == "missing-description"

    at.text_area(key="source").input("model A\n  Real x\nend B;\n").run()
    next(b for b in at.button if b.label == "整形").click().run()
    assert any("整形できません" in e.value for e in at.error)


def test_oversize_input_is_refused():
    with pytest.raises(GlobeError, match="1 MiB"):
        fmt("x" * ((1 << 20) + 1))


def test_slow_globe_times_out(monkeypatch, tmp_path):
    slow = tmp_path / "slow.sh"
    slow.write_text("#!/bin/sh\nexec sleep 30\n")
    slow.chmod(0o755)
    monkeypatch.setenv("GLOBE_BIN", str(slow))
    monkeypatch.setenv("GLOBE_TIMEOUT_MS", "200")
    with pytest.raises(GlobeError, match="時間内"):
        fmt("model A end A;")

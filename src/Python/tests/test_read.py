"""S183: CLI の読み取り（`globe outline` / `flat` / `deps`）と同じ口と、ライブラリのモデル名の誤りの診断（S180）が Python にも出る。"""

from pathlib import Path

import pytest

from globe_mode import Workspace

MODELICA = Path(__file__).parents[2] / "Modelica"
DECAY = "GlobeModeValidation.Simulation.Decay"
A = "model A Real x(start = 1, fixed = true); equation der(x) = -x; end A;"
USES = "model Uses A a; end Uses;"


def workspace() -> Workspace:
    ws = Workspace()
    ws.set_file("A.mo", A)
    ws.set_file("Uses.mo", USES)
    ws.set_file("Other.mo", "model Other Real y; end Other;")
    return ws


def test_outline_lists_the_classes_of_a_file():
    outline = workspace().outline("A.mo")
    assert [(c["name"], c["kind"]) for c in outline["classes"]] == [("A", "model")]
    r = outline["classes"][0]["range"]
    assert 0 <= r["start"] < r["end"] <= len(A)
    # ディスクのファイルは encoding を指定して読んで渡す（Windows の既定の文字コードに依らない）
    text = (MODELICA / "GlobeModeValidation/Simulation/Decay.mo").read_text(encoding="utf-8")
    ws = Workspace()
    ws.set_file("Decay.mo", text)
    assert ws.outline("Decay.mo")["within"] == "GlobeModeValidation.Simulation"


def test_outline_errors_are_value_errors():
    ws = workspace()
    ws.set_file("Broken.mo", "model Broken")
    with pytest.raises(ValueError, match="GM-E"):
        ws.outline("Broken.mo")
    with pytest.raises(ValueError, match="Nope.mo"):
        ws.outline("Nope.mo")


def test_deps_is_the_closure_of_files_check_reads():
    ws = workspace()
    assert ws.deps("Uses") == ["A.mo", "Uses.mo"]  # Other.mo は使わない
    assert ws.deps("A") == ["A.mo"]
    assert ws.deps("NoSuchModel") == []


def test_flat_summary_or_none():
    ws = workspace()
    flat = ws.flat("A")
    assert (flat["variables"], flat["equations"], flat["initial_equations"]) == (1, 1, 0)
    ws.set_file("Bad.mo", "model Bad Real x; equation y = 1; end Bad;")
    assert ws.flat("Bad") is None and not ws.check("Bad")["passed"]  # 理由は check の diagnostics


def test_library_model_name_hint_reaches_python_check():
    """S180 の診断は Rust の検査の入口 1 か所で作るので、Python の `check` にも同じ形で出る。"""
    res = Workspace([MODELICA]).check("Decay")
    assert not res["passed"]
    d = res["diagnostics"][0]
    assert d["code"] == "GM-E1002"
    assert any(n.startswith("候補") and DECAY in n for n in d["notes"]), d

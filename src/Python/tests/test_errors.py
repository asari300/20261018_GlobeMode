"""S200: 例外の階層。子はすべて GlobeModeError で、これまでの組み込みの型も継ぐ。呼び方の誤りは素の TypeError。"""

from pathlib import Path

import pytest

import globe_mode
from globe_mode import (
    ExportError,
    GlobeModeError,
    LibraryError,
    ModelNotFoundError,
    ResultError,
    SettingsError,
    SourceError,
    Workspace,
    format,
    table,
)

MODELICA = Path(__file__).parents[2] / "Modelica"
DECAY = "GlobeModeValidation.Simulation.Decay"


def test_hierarchy_keeps_the_builtin_types():
    bases = {
        LibraryError: RuntimeError,
        SettingsError: ValueError,
        ModelNotFoundError: ValueError,
        SourceError: ValueError,
        ExportError: ValueError,
        ResultError: ValueError,
    }
    for cls, builtin in bases.items():
        assert issubclass(cls, GlobeModeError) and issubclass(cls, builtin), cls
        assert cls.__module__ == "globe_mode"
    assert set(globe_mode.__all__) >= {c.__name__ for c in bases} | {"GlobeModeError"}


def test_library_error(tmp_path):
    with pytest.raises(GlobeModeError) as e:
        Workspace([tmp_path / "no-such-library"])
    assert isinstance(e.value, LibraryError) and isinstance(e.value, RuntimeError)
    assert e.value.code == "GM-E1003" and e.value.diagnostics[0]["code"] == "GM-E1003"
    with pytest.raises(LibraryError) as e:
        Workspace().load_dir(tmp_path / "nope")
    assert e.value.diagnostics == [] and e.value.code is None


def test_model_not_found_and_source_errors():
    ws = Workspace(MODELICA)
    with pytest.raises(ModelNotFoundError, match="NoSuchModel"):
        ws.graph("NoSuchModel")
    ws.set_file("Broken.mo", "model Broken")
    with pytest.raises(SourceError, match="GM-E") as e:
        ws.outline("Broken.mo")
    assert e.value.code and e.value.code.startswith("GM-E") and e.value.diagnostics
    with pytest.raises(SourceError, match="Nope.mo") as e:
        ws.outline("Nope.mo")
    assert e.value.diagnostics == []
    with pytest.raises(SourceError):
        format("model Decay")


def test_settings_and_export_and_result_errors(tmp_path):
    ws = Workspace(MODELICA)
    with pytest.raises(SettingsError, match="設定 stop_time は数値"):
        ws.simulate(DECAY, stop_time="abc")
    with pytest.raises(ExportError):
        ws.export_fmu(DECAY, tmp_path, fmi_version=4)
    with pytest.raises(ResultError, match="no model"):
        table({"csv": None, "refused": "no model"})
    with pytest.raises(ResultError):
        table(ws.check(DECAY))  # check の結果を渡した誤り。前は KeyError


def test_call_errors_stay_plain_and_model_errors_are_not_exceptions():
    ws = Workspace(MODELICA)
    with pytest.raises(TypeError) as e:
        ws.simulate(DECAY, stoptime=3)
    assert not isinstance(e.value, GlobeModeError)
    with pytest.raises(TypeError) as e:
        ws.simulate(DECAY, on_progress=5)
    assert not isinstance(e.value, GlobeModeError)
    assert ws.check("NoSuchModel")["passed"] is False  # モデルの誤りは例外にしない
    assert ws.simulate("NoSuchModel")["ok"] is False
    with pytest.raises(OverflowError):
        ws.graph(DECAY, depth=-1)

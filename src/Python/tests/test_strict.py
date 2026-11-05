"""S181: 呼び方の誤りを黙って通さない（文字列のライブラリ、未知の設定のキー、型の誤り、例外の文面）と、README の例。"""

import json
import typing
from pathlib import Path

import pytest

from globe_mode import Settings, Workspace, _native

MODELICA = Path(__file__).parents[2] / "Modelica"
DECAY = "GlobeModeValidation.Simulation.Decay"


@pytest.mark.parametrize("single", [str(MODELICA), MODELICA])
def test_a_single_library_is_one_library_not_its_characters(single):
    assert Workspace(single).check(DECAY)["passed"]


def test_unknown_setting_key_is_a_type_error_naming_it_and_the_usable_keys():
    ws = Workspace(MODELICA)
    for call in (ws.simulate, ws.simulate_columns):
        with pytest.raises(TypeError) as e:
            call(DECAY, stoptime=3)
        assert "stoptime" in str(e.value) and "stop_time" in str(e.value)
    with pytest.raises(TypeError):
        ws.export_fmu(DECAY, "out", bogus=1)  # FMU の options も同じ（キーワード引数で弾く）


def test_wrong_setting_type_names_the_setting():
    ws = Workspace(MODELICA)
    with pytest.raises(ValueError, match="設定 stop_time は数値"):
        ws.simulate(DECAY, stop_time="abc")
    with pytest.raises(ValueError, match="設定 parameters"):
        ws.simulate(DECAY, parameters={"k": "a"})
    assert ws.simulate(DECAY, stop_time=None, interval=0.5)["ok"]  # None は「省略」、int は数値


def test_settings_typed_dict_matches_rust_sim_settings():
    assert set(typing.get_type_hints(Settings)) == set(json.loads(_native.settings_sample()))


def test_library_error_is_readable_and_keeps_the_diagnostics(tmp_path):
    missing = tmp_path / "no-such-library"
    with pytest.raises(RuntimeError) as e:
        Workspace([missing])
    text = str(e.value)
    assert text.startswith("GM-E1003: ") and not text.startswith("[") and "no-such-library" in text
    assert e.value.diagnostics[0]["code"] == "GM-E1003"


def test_readme_example_runs():
    """src/Python/README.md の ```python の例（MSL と ChuaCircuit を、この検証用ライブラリと Decay に替えて実行する。落ちなければよい）。"""
    readme = (Path(__file__).parents[1] / "README.md").read_text(encoding="utf-8")
    code = readme.split("```python\n")[1].split("```")[0].replace("data/msl", str(MODELICA)).replace(
        "Modelica.Electrical.Analog.Examples.ChuaCircuit", DECAY
    )
    exec(code, {})  # noqa: S102


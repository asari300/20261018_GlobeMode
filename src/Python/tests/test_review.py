"""S200: 束縛のレビュー（Opus、2026-11-06）の指摘 1〜8 の再現と、直したことの確認。"""

import _thread
import inspect
import json
import math
import numbers
import threading
import time
from fractions import Fraction
from pathlib import Path

import pytest

from globe_mode import Workspace

MODELICA = Path(__file__).parents[2] / "Modelica"
DECAY = "GlobeModeValidation.Simulation.Decay"
WIGGLE = "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;"
WIGGLE_SETTINGS = dict(solver="esdirk34", stop_time=1000, interval=1)


def wiggle() -> Workspace:
    ws = Workspace()
    ws.set_file("Wiggle.mo", WIGGLE)
    return ws


class NumpyLike:
    """`np.float32` と同じく `float` を継がず `numbers.Real` に登録された数（numpy を入れずに確かめる）。"""

    def __init__(self, x: float):
        self.x = x

    def __float__(self) -> float:
        return self.x


numbers.Real.register(NumpyLike)


def test_1_export_fmu_targets_string_is_one_target():
    class Spy:
        def export_fmu(self, model, out_dir, options):
            self.options = json.loads(options)
            return "x.fmu"

    ws = Workspace()
    ws._ws = spy = Spy()
    ws.export_fmu("M", "out", targets="x86_64-linux-gnu")
    assert spy.options["targets"] == ["x86_64-linux-gnu"]


@pytest.mark.parametrize("value", [Fraction(1, 2), NumpyLike(0.5)])
def test_3_numpy_like_numbers_are_numbers(value):
    ws = Workspace(MODELICA)
    res = ws.simulate(DECAY, stop_time=value, parameters={"k": value})
    assert res["ok"] and res["settings"]["stop_time"] == 0.5


@pytest.mark.parametrize("bad", [math.nan, math.inf, -math.inf])
def test_3_non_finite_numbers_name_the_setting(bad):
    ws = Workspace(MODELICA)
    with pytest.raises(ValueError, match="設定 stop_time"):
        ws.simulate(DECAY, stop_time=bad)
    with pytest.raises(ValueError, match="設定 parameters"):
        ws.simulate(DECAY, parameters={"k": bad})


def test_4_on_progress_is_keyword_only_and_checked_at_the_door():
    for name in ("simulate", "simulate_columns"):
        sig = inspect.signature(getattr(Workspace, name))
        assert sig.parameters["on_progress"].kind is inspect.Parameter.KEYWORD_ONLY
    ws = Workspace(MODELICA)
    with pytest.raises(TypeError, match="on_progress"):
        ws.simulate(DECAY, on_progress=5)
    with pytest.raises(TypeError):
        ws.simulate(DECAY, {"stop_time": 2})  # 設定を dict で渡す誤り（前は on_progress に入った）


def test_6_bytes_library_is_a_type_error():
    for bad in (b"data/msl", bytearray(b"x")):
        with pytest.raises(TypeError):
            Workspace(bad)


def test_7_none_settings_mean_omitted():
    ws = Workspace(MODELICA)
    assert ws.simulate(DECAY, parameters=None, solver=None)["ok"]


def test_8a_callback_exceptions_pass_through_unchanged():
    class Boom(ValueError):
        pass

    ws = Workspace(MODELICA)

    def cb(p):
        raise Boom("mine")

    for call in (ws.simulate, ws.simulate_columns):
        with pytest.raises(Boom, match="mine"):
            call(DECAY, on_progress=cb)


def test_8b_calling_the_same_workspace_from_the_callback_is_an_error():
    ws = Workspace(MODELICA)
    with pytest.raises(RuntimeError, match="borrowed"):
        ws.simulate(DECAY, on_progress=lambda p: ws.check(DECAY))
    assert ws.check(DECAY)["passed"]  # そのあとも使える


def test_8c_ws_is_usable_after_ctrl_c():
    ws = wiggle()
    timer = threading.Timer(0.2, _thread.interrupt_main)
    timer.start()
    try:
        with pytest.raises(KeyboardInterrupt):
            ws.simulate("Wiggle", on_progress=lambda p: None, **WIGGLE_SETTINGS)
    finally:
        timer.cancel()
    assert ws.simulate("Wiggle", stop_time=1)["ok"]


def test_2_ctrl_c_stops_a_run_without_on_progress():
    ws = wiggle()
    timer = threading.Timer(0.2, _thread.interrupt_main)
    start = time.monotonic()
    timer.start()
    try:
        with pytest.raises(KeyboardInterrupt):
            ws.simulate("Wiggle", **WIGGLE_SETTINGS)
    finally:
        timer.cancel()
    assert time.monotonic() - start < 0.8  # 止めなければ約 1.1 s 回る
    assert ws.check("Wiggle")["passed"]


@pytest.mark.parametrize("columns", [False, True])
def test_2_other_threads_run_during_simulate_without_on_progress(columns):
    ws = wiggle()
    ticks = 0
    done = threading.Event()

    def spin():
        nonlocal ticks
        while not done.is_set():
            ticks += 1

    t = threading.Thread(target=spin)
    t.start()
    try:
        (ws.simulate_columns if columns else ws.simulate)("Wiggle", **WIGGLE_SETTINGS)
    finally:
        done.set()
        t.join()
    assert ticks > 1000, ticks  # GIL を握ったままなら、実行が終わるまでほぼ 0 回

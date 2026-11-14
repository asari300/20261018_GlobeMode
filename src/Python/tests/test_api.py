"""参照シナリオ: ライブラリを開く → 検査 → connect グラフ → 実行 → 結果を表で読む。"""

import math
from pathlib import Path

import pytest

from globe_mode import ExportError, GlobeModeError, Workspace, format, lint, table

MODELICA = Path(__file__).parents[2] / "Modelica"


def test_reference_scenario():
    ws = Workspace(libraries=[MODELICA])  # グラフもライブラリのモデルを引ける（S079）

    check = ws.check("GlobeModeValidation.Simulation.Decay")
    assert check["passed"]
    assert not [d for d in check["diagnostics"] if d["severity"] == "error"]

    graph = ws.graph("GlobeModeValidation.Connect.RealCircuit")
    assert graph["nodes"] and graph["edges"]

    res = ws.simulate("GlobeModeValidation.Simulation.Decay")  # experiment: StopTime = 1
    assert res["ok"]
    df = table(res)
    assert df["time"][-1] == 1.0
    assert math.isclose(df["x"][-1], math.exp(-1), rel_tol=1e-4)

    # S111: 参照シナリオの最後（format）と lint
    formatted = format((MODELICA / "GlobeModeValidation/Simulation/Decay.mo").read_text(encoding="utf-8"))
    assert "der(x) = -k * x;" in formatted
    with pytest.raises(ValueError):
        format("model Decay")
    assert isinstance(lint("model Decay Real x; end Decay;"), list)


def test_library_is_readable():
    assert Workspace(libraries=[MODELICA]).check("GlobeModeValidation.Simulation.Decay")["passed"]


def test_export_fmu(tmp_path):
    """S118: Decay の FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある。"""
    ws = Workspace(libraries=[MODELICA])
    fmu = ws.export_fmu("GlobeModeValidation.Simulation.Decay", tmp_path)
    assert fmu == tmp_path / "GlobeModeValidation_Simulation_Decay-fmi2.fmu" and fmu.is_file()
    assert (tmp_path / "GlobeModeValidation_Simulation_Decay/modelDescription.xml").is_file()
    with pytest.raises(ValueError):
        ws.export_fmu("GlobeModeValidation.Simulation.Decay", tmp_path, fmi_version=4)


def test_export_fmu_p5b(tmp_path):
    """S239: runtime="p5b" は runtime_lib（ここでは中身は何でもよい。実際に回す試験は CLI 側）を binaries に入れた FMU を書く。
    p5a の旗とは組み合わせられず、無い共有ライブラリは ExportError（cargo は呼ばない）。"""
    import zipfile

    ws = Workspace(libraries=[MODELICA])
    model = "GlobeModeValidation.Simulation.Decay"
    lib = tmp_path / "libglobe_mode_fmu.so"
    lib.write_bytes(b"not a real library")
    fmu = ws.export_fmu(model, tmp_path, runtime="p5b", runtime_lib=lib)
    assert fmu == tmp_path / "GlobeModeValidation_Simulation_Decay-p5b-fmi2.fmu"
    with zipfile.ZipFile(fmu) as z:
        names = z.namelist()
    assert "modelDescription.xml" in names and f"resources/{model}.mo" in names
    assert any(n.startswith("binaries/") and "globe_mode_fmu" in n for n in names)
    with pytest.raises(ExportError, match="p5a のもの"):
        ws.export_fmu(model, tmp_path, runtime="p5b", runtime_lib=lib, build=True)
    with pytest.raises(ExportError, match="runtime_lib"):
        ws.export_fmu(model, tmp_path, runtime="p5b", runtime_lib=tmp_path / "none")
    with pytest.raises(ExportError):
        ws.export_fmu(model, tmp_path, runtime="p6")  # type: ignore[arg-type]


# S129（ADR-0033）: 積分に 100 ms より長くかかる（t が途中で知らされる）ように、細かく振れる入力を長く積む
WIGGLE = "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;"
WIGGLE_SETTINGS = dict(solver="esdirk34", stop_time=1000, interval=1)


def wiggle() -> Workspace:
    ws = Workspace()
    ws.set_file("Wiggle.mo", WIGGLE)
    return ws


def test_on_progress_reports_stages_and_time_without_changing_the_result():
    ws = Workspace(libraries=[MODELICA])
    seen = []
    model = "GlobeModeValidation.Simulation.Decay"
    res = ws.simulate_columns(model, on_progress=seen.append, solver="esdirk34")
    assert res["ok"] and res["termination"] == {"kind": "stop_time"}, res["termination"]
    stages = [p["stage"] for p in seen]
    assert stages[0] == "lowering" and stages[-1] == "integration", stages
    times = [p["time"] for p in seen if p["time"] is not None]
    assert times and times[-1] == res["columns"]["time"][-1] == 1.0
    plain = ws.simulate_columns(model, solver="esdirk34")
    assert table(res).equals(table(plain))
    assert ws.simulate(model, on_progress=lambda p: None, solver="esdirk34")["csv"] == ws.simulate(model, solver="esdirk34")["csv"]


def test_returning_true_cancels_with_the_partial_trajectory():
    res = wiggle().simulate_columns("Wiggle", on_progress=lambda p: p["time"] is not None, **WIGGLE_SETTINGS)
    assert res["ok"], res["diagnostics"]
    assert res["termination"]["kind"] == "cancelled", res["termination"]
    time = res["termination"]["time"]
    assert 0 < time < 1000 and res["columns"]["time"][-1] == time
    assert "GM-I11004" in [d["code"] for d in res["diagnostics"]]


def test_ctrl_c_raises_keyboard_interrupt():
    """別のスレッドから `_thread.interrupt_main()`（Ctrl-C と同じ）。次の進捗の `check_signals` で止まる。"""
    import _thread
    import threading

    timer = threading.Timer(0.2, _thread.interrupt_main)
    timer.start()
    try:
        with pytest.raises(KeyboardInterrupt):
            wiggle().simulate("Wiggle", on_progress=lambda p: None, **WIGGLE_SETTINGS)
    finally:
        timer.cancel()


def test_linearize_gives_state_space_matrices():
    """der(x) = -a x + u、y = 2 x なので A = [-a]・B = [1]・C = [2]・D = [0]。numpy があれば ndarray（shape が行×列）。"""
    ws = Workspace()
    ws.set_file("L.mo", "model L parameter Real a = 1; Real x(start = 1, fixed = true); input Real u = 0; output Real y; equation der(x) = -a*x + u; y = 2*x; end L;")
    lin = ws.linearize("L", parameters={"a": 3})
    assert (lin["x"], lin["u"], lin["y"]) == (["x"], ["u"], ["y"])
    got = [float(lin[k][0][0]) for k in "ABCD"]
    assert got == pytest.approx([-3.0, 1.0, 2.0, 0.0], abs=1e-5)
    assert ws.linearize("L", time=1.0)["time"] == 1.0
    with pytest.raises(GlobeModeError) as e:
        ws.linearize("NoSuch")
    assert e.value.diagnostics

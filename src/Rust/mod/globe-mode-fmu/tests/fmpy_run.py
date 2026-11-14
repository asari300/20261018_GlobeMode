"""S095・S099・S114: globe-mode-fmu の cdylib・modelDescription.xml・モデルの .mo を FMU に詰め、FMPy の ME で回す
（tests/fmpy.rs が呼ぶ）。終了時刻での各変数の値を `名前=値` で出す。
S109: 事象の回数も出す（`events=` は fmi2EnterEventMode の回数、`reinit=` は valuesOfContinuousStatesChanged が真の回数）。
S114: 8 番目の引数（任意）は FMPy の `input=` に渡す入力（`{"u": [[時刻, 値], ...]}` の JSON）。
S137: 7 番目の引数は FMI の主版（2 または 3）。3 なら <arch>-<os>/ に置き、fmi3UpdateDiscreteStates を数える。
S234: 置き場と拡張子は実行している OS で決める（Linux x86_64: linux64・x86_64-linux、Windows: win64・x86_64-windows、
macOS: darwin64・<arch>-darwin）。

S231: 9 番目の引数（任意）に CoSimulation を渡すと、FMPy の CS で回す（modelDescription.xml は CS を含むものを渡す）。

S251: 9 番目の引数が ScheduledExecution なら、FMPy の `FMU3ScheduledExecution` で周期クロックを 1 周期ずつ
`fmi3ActivateModelPartition` で動かし（`simulate_fmu` は SE を回せない）、DefaultExperiment の stopTime で止める。

S237: 10 番目の引数（任意）はライブラリ（MSL など）のディレクトリ。`*.mo` を `resources/lib/` に入れる。

使い方: python fmpy_run.py <cdylib（.so・.dll・.dylib）> <modelDescription.xml> <model.mo> <完全修飾名> <出力の .fmu> <変数,...> <2|3> [入力の JSON] [CoSimulation] [ライブラリ]
"""

import json
import platform
import sys
import zipfile
from pathlib import Path

import numpy as np
from fmpy import fmi2, fmi3, simulate_fmu
from fmpy.validation import validate_fmu

so, md, mo, name, fmu, names, version = sys.argv[1:8]
arch = {"AMD64": "x86_64", "arm64": "aarch64"}.get(platform.machine(), platform.machine())
os_name = {"win32": "windows", "darwin": "darwin"}.get(sys.platform, "linux")
v2_dir = {"windows": "win64", "darwin": "darwin64"}.get(os_name, "linux64")
ext = {"windows": "dll", "darwin": "dylib"}.get(os_name, "so")
with zipfile.ZipFile(fmu, "w") as z:
    z.write(md, "modelDescription.xml")
    z.write(so, f"binaries/{f'{arch}-{os_name}' if version == '3' else v2_dir}/globe_mode_fmu.{ext}")
    z.write(mo, f"resources/{name}.mo")
    if len(sys.argv) > 10 and sys.argv[10]:
        lib = Path(sys.argv[10])
        for f in sorted(lib.rglob("*.mo")):
            z.write(f, f"resources/lib/{f.relative_to(lib).as_posix()}")
problems = validate_fmu(fmu)
assert not problems, problems
if len(sys.argv) > 9 and sys.argv[9] == "ScheduledExecution":
    import xml.etree.ElementTree as ET

    from fmpy import extract, read_model_description

    root = ET.parse(md).getroot()
    clock = root.find("ModelVariables/Clock")
    vr, period = int(clock.get("valueReference")), float(clock.get("intervalDecimal"))
    stop = float(root.find("DefaultExperiment").get("stopTime"))
    d = read_model_description(fmu)
    refs = {v.name: v for v in d.modelVariables}
    se = fmi3.FMU3ScheduledExecution(
        guid=d.guid, unzipDirectory=extract(fmu), modelIdentifier=d.scheduledExecution.modelIdentifier, instanceName="se"
    )
    se.instantiate()
    se.enterInitializationMode(startTime=0.0)
    se.exitInitializationMode()
    steps = round(stop / period)
    for k in range(steps + 1):
        se.activateModelPartition(vr, k * period)
    get = lambda v: (se.getInt32 if v.type == "Int32" else se.getFloat64)([v.valueReference])[0]
    print(" ".join(f"{n}={get(refs[n])!r}" for n in names.split(",")), f"t={steps * period!r}", "clock_vr=" + str(vr))
    se.terminate()
    se.freeInstance()
    sys.exit(0)
counts = {"events": 0, "reinit": 0}
# 2.0 は FMU2Model.newDiscreteStates、3.0 は _FMU3.updateDiscreteStates（どちらも 4 番目が valuesOfContinuousStatesChanged）
model, update = (fmi3._FMU3, "updateDiscreteStates") if version == "3" else (fmi2.FMU2Model, "newDiscreteStates")
enter_event_mode = model.enterEventMode
new_discrete_states = getattr(model, update)


def counted_enter_event_mode(self):
    counts["events"] += 1
    return enter_event_mode(self)


def counted_new_discrete_states(self):
    info = new_discrete_states(self)
    counts["reinit"] += int(info[3])
    return info


model.enterEventMode = counted_enter_event_mode
setattr(model, update, counted_new_discrete_states)
signal = None
if len(sys.argv) > 8 and sys.argv[8]:
    columns = json.loads(sys.argv[8])
    times = next(iter(columns.values()))
    signal = np.array(
        [(row[0], *(columns[n][i][1] for n in columns)) for i, row in enumerate(times)],
        dtype=[("time", np.float64)] + [(n, np.float64) for n in columns],
    )
result = simulate_fmu(fmu, fmi_type=sys.argv[9] if len(sys.argv) > 9 else "ModelExchange", output=names.split(","), input=signal)
print(" ".join(f"{n}={float(result[n][-1])!r}" for n in names.split(",")), f"t={float(result['time'][-1])!r}",
      " ".join(f"{k}={v}" for k, v in counts.items()))

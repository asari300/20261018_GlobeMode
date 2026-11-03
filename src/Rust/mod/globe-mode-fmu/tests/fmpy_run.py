"""S095・S099: globe-mode-fmu の cdylib・modelDescription.xml・モデルの .mo を FMU に詰め、FMPy の ME で回す
（tests/fmpy.rs が呼ぶ）。終了時刻での各変数の値を `名前=値` で出す。
S109: 事象の回数も出す（`events=` は fmi2EnterEventMode の回数、`reinit=` は valuesOfContinuousStatesChanged が真の回数）。

使い方: python fmpy_run.py <libglobe_mode_fmu.so> <modelDescription.xml> <model.mo> <完全修飾名> <出力の .fmu> <変数,...>
"""

import sys
import zipfile

from fmpy import fmi2, simulate_fmu
from fmpy.validation import validate_fmu

so, md, mo, name, fmu, names = sys.argv[1:7]
with zipfile.ZipFile(fmu, "w") as z:
    z.write(md, "modelDescription.xml")
    # FMI 2.0 の置き場（FMI 3.0 なら binaries/x86_64-linux/）
    z.write(so, "binaries/linux64/globe_mode_fmu.so")
    z.write(mo, f"resources/{name}.mo")
problems = validate_fmu(fmu)
assert not problems, problems
counts = {"events": 0, "reinit": 0}
enter_event_mode = fmi2.FMU2Model.enterEventMode
new_discrete_states = fmi2.FMU2Model.newDiscreteStates


def counted_enter_event_mode(self):
    counts["events"] += 1
    return enter_event_mode(self)


def counted_new_discrete_states(self):
    info = new_discrete_states(self)
    counts["reinit"] += info[3]
    return info


fmi2.FMU2Model.enterEventMode = counted_enter_event_mode
fmi2.FMU2Model.newDiscreteStates = counted_new_discrete_states
result = simulate_fmu(fmu, fmi_type="ModelExchange", output=names.split(","))
print(" ".join(f"{n}={float(result[n][-1])!r}" for n in names.split(",")), f"t={float(result['time'][-1])!r}",
      " ".join(f"{k}={v}" for k, v in counts.items()))

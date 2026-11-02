"""S095: globe-mode-fmu の cdylib を FMU に詰め、FMPy で回して x(1) を出す（tests/fmpy.rs が呼ぶ）。

使い方: python fmpy_decay.py <libglobe_mode_fmu.so> <modelDescription.xml> <出力の .fmu>
"""

import sys
import zipfile

from fmpy import simulate_fmu

so, md, fmu = sys.argv[1:4]
with zipfile.ZipFile(fmu, "w") as z:
    z.write(md, "modelDescription.xml")
    # FMI 2.0 の置き場（FMI 3.0 なら binaries/x86_64-linux/）
    z.write(so, "binaries/linux64/globe_mode_fmu.so")
result = simulate_fmu(fmu, fmi_type="ModelExchange", output=["x"], stop_time=1.0)
print(f"x(1)={float(result['x'][-1])!r} t={float(result['time'][-1])!r}")

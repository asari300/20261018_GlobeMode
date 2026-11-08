"""S237: MSL の例題を FMU に書き出し、FMPy で回して `globe simulate` の結果と比べる（P5a。ADR-0031 の追記）。

使い方（FMPy と numpy の入った Python で）:
  python fmu_msl_check.py --globe target/release/globe --lib data/msl --out work MODEL... [--fmi 2|3] [--kinds ModelExchange,CoSimulation]

1 モデルにつき JSON を 1 行、標準出力へ出す。`outcome` は ok（比べた信号がすべて管に収まった）か、落ちた段
（simulate_ref・export・validate・ModelExchange・CoSimulation・compare）。`class` は落ち方の分類（`classify`）。
管は csv-compare の既定に合わせ、時間・値とも相対幅 0.002 の矩形（`globe-mode-sim` の compare.rs の簡略版。ref の折れ線の
窓の最小・最大を使う）。比べるのは ref の CSV と FMU の両方にある変数だけ。
"""

import argparse
import csv
import json
import re
import subprocess
import sys
import zipfile
from pathlib import Path

import numpy as np

# (パターン, 分類)。上から最初に当たったもの。メッセージは Rumoca の `FMI C profile: …` の原文（rev 276e82bd の
# `rumoca-ir-solve/src/fmi/` の `Err(&str)`）と FMPy の例外。event は when・関係式の根・assert の総称（Rumoca の
# 1 つのメッセージ "only unscheduled assertions without relation memory are supported" が全部を指す）
CLASSES = [
    (r"unscheduled assertions|cannot execute time events", "event"),
    (r"homotopy|settled algebraic refresh|can only assign parameters and state starts|ExitInitializationMode", "initialization"),
    (r"clocks", "clock"),
    (r"external|ModelicaStandardTables|table", "external-function"),
    (r"string", "string"),
    (r"GetReal failed|GetDerivatives failed|status 3", "runtime-error"),
]


def classify(text):
    for pattern, name in CLASSES:
        if re.search(pattern, text, re.IGNORECASE):
            return name
    return "other"


def run(args, timeout):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(args, 124, "", "timeout")


def read_csv(path):
    """列の名前（空白は除く）→ 値の配列。FMPy は Boolean を True / False で書く。"""
    with open(path) as f:
        rows = list(csv.reader(f))
    flag = {"True": 1.0, "False": 0.0}
    data = np.array([[flag[c.strip()] if c.strip() in flag else float(c) for c in r] for r in rows[1:]])
    return {h.strip().replace(" ", ""): data[:, i] for i, h in enumerate(rows[0])}


def tube_outside(t_ref, y_ref, t_fmu, y_fmu, w_t, w_y):
    """FMU の点 (t, y) のうち、ref の折れ線の [t - w_t, t + w_t] の窓の最小・最大 ± w_y に収まらない点の割合。"""
    step = max(len(t_fmu) // 500, 1)  # 点が多いモデル（5 万点）は間引く
    n = out = 0
    for t, y in zip(t_fmu[::step], y_fmu[::step]):
        i0, i1 = np.searchsorted(t_ref, [t - w_t, t + w_t])
        win = np.concatenate([np.interp([t - w_t, t, t + w_t], t_ref, y_ref), y_ref[i0:i1]])
        n += 1
        out += not (win.min() - w_y <= y <= win.max() + w_y)
    return out / max(n, 1)


# 管の外の点がこの割合以下の信号は合格にする。FMPy の ME は CVode で、exec（Rumoca の BDF）と事象の時刻が
# 数 ms ずれるため、段差の 1〜2 点が管の外に出る（PID_Controller の PI.I.u など）。割合は JSON にも書く
LENIENT = 0.01


def compare(ref, fmu_csv):
    got = read_csv(fmu_csv)
    t_ref = ref["time"]
    t0, span = float(t_ref[0]), float(t_ref[-1] - t_ref[0])
    # globe-mode-sim の compare.rs と同じ幅（δt = tol·max(区間, |t0|)、δy = tol·max(範囲, |ymin|, 0.001)）
    w_t = 0.002 * max(span, abs(t0), 1e-12)
    common = [n for n in got if n != "time" and n in ref]
    bad = {}
    for n in common:
        r, g = ref[n], got[n]
        if not np.all(np.isfinite(g)):
            bad[n] = 1.0
            continue
        w_y = 0.002 * max(float(r.max() - r.min()), abs(float(r.min())), 0.001)
        frac = tube_outside(t_ref, r, got["time"], g, w_t, w_y)
        if frac > LENIENT:
            bad[n] = round(frac, 3)
    return len(common), bad


def build_p5b(a, model, work):
    """P5b の FMU: cdylib + md + resources（モデルの名前のコメントだけの .mo と、MSL の .mo を resources/lib に）。失敗なら理由の文字列。"""
    cdylib, md_bin = a.p5b
    stub = work / f"{model}.mo"
    stub.write_text("// library model (S237)\n")
    r = run([md_bin, str(stub), model, a.fmi, "both", a.lib], a.timeout)
    if r.returncode != 0:
        return r.stderr
    v3 = a.fmi == "3"
    fmu = work / f"{model.replace('.', '_')}-p5b-fmi{a.fmi}.fmu"
    with zipfile.ZipFile(fmu, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("modelDescription.xml", r.stdout)
        z.write(cdylib, f"binaries/{'x86_64-linux' if v3 else 'linux64'}/globe_mode_fmu.so")
        z.write(stub, f"resources/{model}.mo")
        for mo in sorted(Path(a.lib).rglob("*.mo")):
            z.write(mo, f"resources/lib/{mo.relative_to(a.lib)}")
    return fmu


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--globe", required=True)
    p.add_argument("--lib", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--fmi", default="2")
    p.add_argument("--kinds", default="ModelExchange,CoSimulation")
    p.add_argument("--timeout", type=int, default=300)
    p.add_argument("--p5b", nargs=2, metavar=("CDYLIB", "MD_BIN"), help="P5b（globe-mode-fmu の cdylib と examples/md）で FMU を作る")
    p.add_argument("--strict", action="store_true", help="ok でないモデルがあれば終了コード 1（CI 用）")
    p.add_argument("models", nargs="+")
    a = p.parse_args()
    import fmpy
    from fmpy.validation import validate_fmu

    headers = str(Path(fmpy.__file__).parent / "c-code")
    out = Path(a.out)
    results = []
    for model in a.models:
        work = out / model
        work.mkdir(parents=True, exist_ok=True)
        res = {"model": model, "fmi": a.fmi, "outcome": "ok", "class": None, "detail": ""}

        results.append(res)

        def fail(stage, text):
            res.update(outcome=stage, **{"class": classify(text), "detail": text.strip()[-300:]})
            print(json.dumps(res, ensure_ascii=False), flush=True)

        ref_csv = work / "ref.csv"
        r = run([a.globe, "simulate", "-L", a.lib, "-m", model, "-o", str(ref_csv)], a.timeout)
        if r.returncode != 0:
            fail("simulate_ref", r.stderr)
            continue
        ref = read_csv(ref_csv)
        if a.p5b:
            fmu = build_p5b(a, model, work)
            if isinstance(fmu, str):
                fail("export", fmu)
                continue
        else:
            r = run(
                [a.globe, "fmu", "export", "-L", a.lib, "-m", model, "--out", str(work), "--fmi-version", a.fmi,
                 "--build", "--fmi-headers", headers],
                a.timeout,
            )
            if r.returncode != 0:
                fail("export", r.stderr)
                continue
            fmu = work / f"{model.replace('.', '_')}-fmi{a.fmi}.fmu"
        problems = validate_fmu(str(fmu))
        if problems:
            fail("validate", "\n".join(problems))
            continue
        t_end, n = float(ref["time"][-1]), len(ref["time"])
        names = fmpy.read_model_description(str(fmu)).modelVariables
        outs = [v.name for v in names if v.name.replace(" ", "") in ref and v.name != "time"][:300]
        for kind in a.kinds.split(","):
            csv = work / f"fmu-{kind}.csv"
            code = "import sys; from fmpy.cli import main; sys.argv = ['fmpy'] + sys.argv[1:]; main()"
            r = run(
                [sys.executable, "-I", "-c", code, "simulate", str(fmu), "--interface-type", kind,
                 "--stop-time", repr(t_end), "--output-interval", repr(t_end / (n - 1)),
                 "--relative-tolerance", "1e-6", "--output-variables", *outs, "--output-file", str(csv)],
                a.timeout,
            )
            if r.returncode != 0 or not csv.exists():
                fail(kind, r.stderr or f"exit {r.returncode}")
                break
            count, bad = compare(ref, csv)
            res.setdefault("signals", {})[kind] = {"compared": count, "outside": dict(list(bad.items())[:5])}
            if count == 0 or bad:
                fail("compare", f"{kind}: {count} signals, outside: {dict(list(bad.items())[:5])}")
                break
        else:
            print(json.dumps(res, ensure_ascii=False), flush=True)
    if a.strict and any(r["outcome"] != "ok" for r in results):
        sys.exit(1)


if __name__ == "__main__":
    main()

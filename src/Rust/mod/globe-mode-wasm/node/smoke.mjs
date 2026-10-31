// globe-mode-wasm の Node での試験（S012。CI の wasm-node と mise run wasm:node が回す）。
//
//   node smoke.mjs [--pkg DIR] [--msl DIR]
//
// 1. 版とソルバの登録表（JSON）が読める
// 2. GlobeModeValidation.Simulation.Decay（der(x) = -x、x(0) = 1）を bdf・dopri45・auto で回し、
//    t = 1 の x が exp(-1) に合う（bdf は Rumoca の CLI と同じ道。P3 の「bdf が wasm で動く」）
// 3. --msl があれば、MSL を in-memory のライブラリとして一度だけ読み（S037 から分割読み込み: 使うファイルだけを
//    検査のときに読む）、ChuaCircuit（電気）と Rotational.Examples.First（機械）を bdf で回す（無ければ飛ばす）。
//    読み込み後と各モデルの後のメモリ（MiB）と時間を出す
// 4. panic の文言が last_panic() で読める（最後に回す。panic の後のモジュールは使えない）
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const args = process.argv.slice(2);
const arg = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const pkg = resolve(arg("--pkg") ?? join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg/node"));
const msl = arg("--msl");

const t0 = performance.now();
const gm = createRequire(import.meta.url)(join(pkg, "globe_mode_wasm.js"));
const ms = (t) => +(performance.now() - t).toFixed(1);
const mb = () => +(gm.memory_bytes() / 2 ** 20).toFixed(0);
console.log(`globe-mode-wasm ${gm.version()}（読み込み ${ms(t0)} ms、Node ${process.version}）`);

function readMo(dir, out = {}) {
  for (const e of readdirSync(dir).sort()) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) readMo(p, out);
    else if (p.endsWith(".mo")) out[relative(repo, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}

// 1. ソルバの登録表
const solvers = JSON.parse(gm.solvers());
const available = solvers.filter((s) => s.status.kind === "available").map((s) => s.name);
assert.ok(available.includes("bdf") && available.includes("dopri45") && available.includes("auto"), available.join());
console.log(`ソルバ: ${solvers.length} 件（実装あり ${available.join(", ")}）`);

// 2. Decay
const ws = new gm.Workspace();
for (const [name, text] of Object.entries(readMo(join(repo, "src/Modelica/GlobeModeValidation")))) {
  ws.set_file(name, text);
}
const decay = "GlobeModeValidation.Simulation.Decay";
for (const solver of ["bdf", "dopri45", "auto"]) {
  const t = performance.now();
  const r = JSON.parse(ws.simulate(JSON.stringify({ model: decay, settings: { solver } })));
  const wall = ms(t);
  assert.ok(r.ok, `${solver}: ${JSON.stringify(r.diagnostics)}`);
  assert.equal(r.settings.solver, solver);
  assert.equal(r.termination.kind, "stop_time");
  const x = r.final.x;
  const err = Math.abs(x - Math.exp(-1));
  assert.ok(err < 1e-6, `${solver}: x(1) = ${x}`);
  const last = r.csv.trim().split("\n").at(-1).split(",");
  assert.equal(Number(last[0]), 1);
  assert.equal(Number(last[1]), x);
  const p = Object.entries(r.timings_ms).map(([k, v]) => `${k} ${v.toFixed(1)}`).join(" / ");
  console.log(`${decay} ${solver}: x(1) = ${x}（誤差 ${err.toExponential(1)}、${wall} ms: ${p} ms、${r.stats.points} 行）`);
}
console.log(`wasm の線形メモリ ${mb()} MiB`);

// 3. MSL
if (msl && existsSync(msl)) {
  let t = performance.now();
  const files = readMo(resolve(msl));
  const read = ms(t);
  t = performance.now();
  const lib = new gm.Workspace(JSON.stringify(files), "MSL");
  const load = ms(t);
  console.log(`MSL: ${Object.keys(files).length} ファイル（読み ${read} ms、in-memory のライブラリ ${load} ms、メモリ ${mb()} MiB）`);
  for (const [model, check, shown] of [
    ["Modelica.Electrical.Analog.Examples.ChuaCircuit", (f) => Number.isFinite(f["C1.v"]), "C1.v"],
    ["Modelica.Mechanics.Rotational.Examples.First", (f) => Number.isFinite(f["inertia1.w"]), "inertia1.w"],
  ]) {
    t = performance.now();
    const r = JSON.parse(lib.simulate(JSON.stringify({ model, settings: { solver: "bdf" }, no_csv: true })));
    const wall = ms(t);
    assert.ok(r.ok, `${model}: ${JSON.stringify(r.diagnostics.filter((d) => d.severity === "error"))}`);
    assert.ok(check(r.final), JSON.stringify(r.final));
    const p = Object.entries(r.timings_ms).map(([k, v]) => `${k} ${v.toFixed(0)}`).join(" / ");
    console.log(`${model} bdf: ${wall} ms（${p} ms）、${shown}(T) = ${r.final[shown]}、メモリ ${mb()} MiB`);
  }
} else if (msl) {
  console.log(`MSL（${msl}）が無いので MSL の例題は飛ばす`);
}

// 4. panic の文言
assert.equal(gm.last_panic(), undefined);
assert.throws(() => gm.panic_for_test("試験の panic"), WebAssembly.RuntimeError);
assert.match(gm.last_panic(), /試験の panic/);
console.log(`panic: ${gm.last_panic().split("\n")[0]} …（last_panic で読めた）`);
console.log("ok");

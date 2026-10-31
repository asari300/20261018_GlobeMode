// simulate（CSV の文字列）と simulate_columns（列ごとの Float64Array）の時間を比べる（S051。mise run wasm:node の後に回す）。
//
//   node columns.mjs [--pkg DIR] [--interval 1e-5] [--runs 5]
//
// GlobeModeValidation.Simulation.Decay を Interval = 1e-5 で回して 100,001 点にし（S042 の 10 万点と同じ大きさ）、
//   CSV の道   : simulate → JSON.parse → parseCsv（src/Svelte/src/lib/plot/csv.ts と同じ解析。S042 の UI がする仕事）
//   列ごとの道 : simulate_columns → layout の JSON.parse → column(i)
// を同じ wasm の上で交互に回し、1 回ごとの時間の中央値と、wasm の線形メモリを出す。値が一致することも確かめる。
// そのあと Web Worker（worker_threads）の GlobeWorker.simulateColumns でも同じ値が返ることを確かめる（buffer は転送）。
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";
import assert from "node:assert/strict";
import { GlobeWorker } from "../web/client.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const args = process.argv.slice(2);
const arg = (name, dflt) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : dflt;
};
const T = process.env.CARGO_TARGET_DIR ?? join(repo, "target");
const nodePkg = resolve(arg("--pkg", join(T, "wasm-pkg/node")));
const webPkg = resolve(join(T, "wasm-pkg/web"));
const interval = Number(arg("--interval", 1e-5)); // Decay の出力の間隔（1e-5 で 100,001 点）
const runs = Number(arg("--runs", 5));
const gm = createRequire(import.meta.url)(join(nodePkg, "globe_mode_wasm.js"));

function readMo(dir, out = {}) {
  for (const e of readdirSync(dir).sort()) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) readMo(p, out);
    else if (p.endsWith(".mo")) out[relative(repo, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}
const files = readMo(join(repo, "src/Modelica/GlobeModeValidation"));

// src/Svelte/src/lib/plot/csv.ts の解析（型を除いただけ）
function headerNames(line) {
  const names = [];
  for (const m of line.matchAll(/"((?:[^"]|"")*)"|([^,]+)/g)) names.push(m[1] !== undefined ? m[1].replaceAll('""', '"') : m[2]);
  return names;
}
const num = (s) => (Number.isFinite(Number(s)) ? Number(s) : null);
function parseCsv(text) {
  const lines = text.split("\n").filter((l) => l.trim() !== "");
  const names = headerNames(lines[0].replace(/\r$/, ""));
  const data = names.map(() => []);
  for (let i = 1; i < lines.length; i++) {
    const f = lines[i].replace(/\r$/, "").split(",");
    for (let c = 0; c < names.length; c++) data[c].push(num(f[c]));
  }
  return { names: names.slice(1), data };
}

// S042 の 10 万点の試験と同じ形（4 状態のモデルで、変数は 7 本）。本番の経路で測るため、実際のモデルを実行する
const wide = `model Wide "4 つの振動子 + 代数変数"
  Real x1(start = 1, fixed = true); Real v1(start = 0, fixed = true);
  Real x2(start = 0, fixed = true); Real v2(start = 1, fixed = true);
  Real s; Real p;
equation
  der(x1) = v1; der(v1) = -x1 - 0.1 * v1 + 0.01 * x2;
  der(x2) = v2; der(v2) = -4 * x2 - 0.05 * v2;
  s = x1 + x2; p = v1 * v2;
  annotation (experiment(StopTime = 1000, Interval = 0.01));
end Wide;
`;
const cases = [
  { name: "Decay（2 列）", model: "GlobeModeValidation.Simulation.Decay", settings: { solver: "bdf", interval } },
  { name: "Wide（7 列）", model: "Wide", settings: { solver: "bdf" } },
];
const ws = new gm.Workspace();
for (const [n, t] of Object.entries(files)) ws.set_file(n, t);
ws.set_file("Wide.mo", wide);
const median = (a) => [...a].sort((x, y) => x - y)[a.length >> 1];
const mib = () => +(gm.memory_bytes() / 2 ** 20).toFixed(0);
const f = (a) => median(a).toFixed(0);
const requests = [];

for (const c of cases) {
  const request = { model: c.model, settings: c.settings };
  requests.push([c.name, request]);
  const csvT = { wasm: [], parse: [], total: [] }, colT = { wasm: [], copy: [], total: [] };
  let csvData, colData, points;
  for (let i = 0; i < runs + 1; i++) { // 0 回目は温め
    const t0 = performance.now();
    const text = JSON.parse(ws.simulate(JSON.stringify(request))); // wasm → JSON の文字列 → オブジェクト
    const t1 = performance.now();
    csvData = parseCsv(text.csv);
    const t2 = performance.now();
    const col = ws.simulate_columns(JSON.stringify(request));
    const t3 = performance.now();
    const layout = JSON.parse(col.layout());
    colData = layout.names.map((_, k) => col.column(k));
    col.free();
    const t4 = performance.now();
    points = layout.points;
    if (i === 0) continue;
    csvT.wasm.push(t1 - t0); csvT.parse.push(t2 - t1); csvT.total.push(t2 - t0);
    colT.wasm.push(t3 - t2); colT.copy.push(t4 - t3); colT.total.push(t4 - t2);
  }
  // 値の一致（CSV の非有限は null、列は NaN / Inf）
  assert.equal(points, csvData.data[0].length);
  for (let k = 0; k < colData.length; k++)
    for (let r = 0; r < points; r++) {
      const a = colData[k][r], b = csvData.data[k][r];
      assert.ok(b === null ? !Number.isFinite(a) : a === b, `列 ${k} 行 ${r}: ${a} vs ${b}`);
    }
  console.log(`${c.name}: ${points} 点 × ${colData.length} 列、${runs} 回の中央値（ms）`);
  console.log(`  CSV の道   : simulate ${f(csvT.wasm)}（wasm と JSON.parse）+ parseCsv ${f(csvT.parse)} = ${f(csvT.total)}`);
  console.log(`  列ごとの道 : simulate_columns ${f(colT.wasm)}（wasm）+ column() の受け取り ${f(colT.copy)} = ${f(colT.total)}`);
}
console.log(`wasm の線形メモリ ${mib()} MiB（全部を回した後）`);

// Worker 越し（転送）
const w = new GlobeWorker(() => new Worker(new URL("../web/worker.js", import.meta.url)), {
  pkg: pathToFileURL(webPkg).href,
  wasm: readFileSync(join(webPkg, "globe_mode_wasm_bg.wasm")),
});
try {
  for (const [n, t] of Object.entries(files)) await w.setFile(n, t);
  await w.setFile("Wide.mo", wide);
  for (const [name, request] of requests) {
  const wt = { csv: [], columns: [] };
  let r;
  for (let i = 0; i < runs + 1; i++) {
    let t = performance.now();
    const a = await w.simulate(request);
    const parsed = parseCsv(a.csv);
    const tc = performance.now() - t;
    t = performance.now();
    r = await w.simulateColumns(request);
    const tk = performance.now() - t;
    assert.equal(r.response.csv, null);
    assert.ok(r.response.ok);
    assert.equal(r.columns.length, parsed.names.length + 1);
    assert.ok(r.columns.every((c) => c instanceof Float64Array && c.length === r.layout.points));
    if (i > 0) { wt.csv.push(tc); wt.columns.push(tk); }
  }
  console.log(`Worker 越し（postMessage）${name}: CSV の道 ${f(wt.csv)} ms、列ごとの道 ${f(wt.columns)} ms（中央値、${r.layout.points} 点）`);
  }
  console.log(`Worker の線形メモリ ${+((await w.memory()) / 2 ** 20).toFixed(0)} MiB`);
} finally {
  await w.terminate();
}
console.log("ok");

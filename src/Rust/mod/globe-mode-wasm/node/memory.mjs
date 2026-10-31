// wasm の線形メモリの伸びを測る（S051。ADR-0023 (c) の上限 2 GiB に何本目で達するか）。
//
//   node memory.mjs --msl DIR --list FILE [--limit N] [--recycle-mib 2048] [--json OUT] [--pkg DIR]
//
// MSL を分割読み込み（S037）で Worker に一度読み、FILE（`cargo xtask msl-funnel --list` の出力。1 行 1 モデル）の
// モデルを順に check して、1 本ごとに wasm の線形メモリ（memory_bytes）と時間を記録する。
// 線形メモリが --recycle-mib（ADR-0023 (c) の上限。既定 2048）を超えたら、次の check の前に Worker を作り直して
// MSL を読み直す（UI がする作り直しと同じ。時間も記録する）。作り直しをしない比較は --recycle-mib 0。
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";
import { GlobeWorker } from "../web/client.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const args = process.argv.slice(2);
const arg = (name, dflt) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : dflt;
};
const pkg = resolve(arg("--pkg", join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg/web")));
const mslDir = resolve(arg("--msl", join(repo, "data/msl")));
const models = readFileSync(arg("--list"), "utf8").split("\n").map((s) => s.trim()).filter(Boolean).slice(0, Number(arg("--limit", 1e9)));
const recycleMiB = Number(arg("--recycle-mib", 2048));
const out = arg("--json");

function readMo(dir, out = {}) {
  for (const e of readdirSync(dir).sort()) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) readMo(p, out);
    else if (p.endsWith(".mo")) out[relative(mslDir, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}

const mib = (b) => +(b / 2 ** 20).toFixed(1);
const ms = (t) => +(performance.now() - t).toFixed(1);
const wasm = readFileSync(join(pkg, "globe_mode_wasm_bg.wasm"));
const libJson = JSON.stringify(readMo(mslDir));
const make = () => new GlobeWorker(() => new Worker(new URL("../web/worker.js", import.meta.url)), { pkg: pathToFileURL(pkg).href, wasm });

let w = make();
let t = performance.now();
await w.library(libJson, "MSL");
const loadMs = ms(t);
const afterLoad = await w.memory();
console.log(`MSL を読んだ後の線形メモリ ${mib(afterLoad)} MiB（${loadMs} ms）、対象 ${models.length} 本、作り直しの上限 ${recycleMiB} MiB`);

const rows = [];
let first2GiB = null, recycles = 0, restarts = 0, recycleMs = [], peak = afterLoad, failures = 0;
for (const [i, model] of models.entries()) {
  if (recycleMiB > 0 && (await w.memory()) > recycleMiB * 2 ** 20) {
    t = performance.now();
    restarts += w.restarts;
    await w.terminate();
    w = make();
    await w.library(libJson, "MSL");
    recycles++;
    recycleMs.push(ms(t));
    console.log(`  ${i} 本目の前に作り直し（${ms(t)} ms）`);
  }
  t = performance.now();
  let row;
  try {
    const r = await w.check(model);
    row = { i, model, passed: r.passed, stage: r.failed_stage, ms: ms(t), mem: await w.memory() };
  } catch (e) {
    failures++;
    row = { i, model, error: String(e.message).split("\n")[0], ms: ms(t), mem: await w.memory().catch(() => null) };
  }
  rows.push(row);
  if (row.mem) peak = Math.max(peak, row.mem);
  if (row.mem >= 2 ** 31 && first2GiB === null) first2GiB = i + 1;
  if ((i + 1) % 10 === 0 || i + 1 === models.length) console.log(`${i + 1} 本: ${mib(row.mem ?? 0)} MiB（最大 ${mib(peak)} MiB、直近 ${row.ms} ms）`);
}
restarts += w.restarts;
await w.terminate();

const summary = { models: models.length, load_ms: loadMs, after_load_mib: mib(afterLoad), peak_mib: mib(peak), first_2gib_at: first2GiB, recycle_mib: recycleMiB, recycles, recycle_ms: recycleMs, failures, restarts };
console.log(JSON.stringify(summary));
if (out) writeFileSync(out, JSON.stringify({ summary, rows }, null, 1));

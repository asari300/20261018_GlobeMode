// Web Worker 経路の Node での試験（S036。CI の wasm-node と mise run wasm:web が回す）。
//
//   node worker-node.mjs [--pkg DIR]      DIR は wasm-bindgen --target web の出力（既定 target/wasm-pkg/web）
//
// worker_threads 上の worker.js を GlobeWorker で包み、GlobeModeValidation の Decay を bdf で回して x(1) ≈ exp(-1) を確かめる。
// そのあと panic させ、worker が作り直され（restarts === 1）、ファイルが再生されて、もう一度 Decay が回ることを確かめる。
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";
import assert from "node:assert/strict";
import { GlobeWorker } from "./client.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const args = process.argv.slice(2);
const i = args.indexOf("--pkg");
const pkg = resolve(i >= 0 ? args[i + 1] : join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg/web"));

function readMo(dir, out = {}) {
  for (const e of readdirSync(dir).sort()) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) readMo(p, out);
    else if (p.endsWith(".mo")) out[relative(repo, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}

const ms = (t) => +(performance.now() - t).toFixed(1);
const decay = "GlobeModeValidation.Simulation.Decay";
const w = new GlobeWorker(() => new Worker(new URL("./worker.js", import.meta.url)), {
  pkg: pathToFileURL(pkg).href,
  wasm: readFileSync(join(pkg, "globe_mode_wasm_bg.wasm")), // Node は file: を fetch できないのでバイト列で渡す
});

async function runDecay(label) {
  const t = performance.now();
  const r = await w.simulate({ model: decay, settings: { solver: "bdf" } });
  assert.ok(r.ok, `${label}: ${JSON.stringify(r.diagnostics)}`);
  assert.equal(r.termination.kind, "stop_time");
  const err = Math.abs(r.final.x - Math.exp(-1));
  assert.ok(err < 1e-6, `${label}: x(1) = ${r.final.x}`);
  console.log(`${label}: x(1) = ${r.final.x}（誤差 ${err.toExponential(1)}、${ms(t)} ms）`);
}

try {
  console.log(`globe-mode-wasm ${await w.version()}（Web Worker、worker_threads）`);
  assert.ok((await w.solvers()).some((s) => s.name === "bdf" && s.status.kind === "available"));
  for (const [name, text] of Object.entries(readMo(join(repo, "src/Modelica/GlobeModeValidation")))) await w.setFile(name, text);
  await runDecay("Decay bdf");
  console.log(`wasm の線形メモリ ${(await w.memory()) / 2 ** 20 | 0} MiB`);

  await assert.rejects(w.panicForTest("試験の panic"), /試験の panic/);
  assert.equal(w.restarts, 1);
  console.log("panic: reject され、worker を作り直した（restarts = 1）");
  await runDecay("作り直し後の Decay bdf");
  assert.equal(w.restarts, 1);
  console.log("ok");
} finally {
  await w.terminate();
}

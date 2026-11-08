// Web Worker 経路の Node での試験（S036。CI の wasm-node と mise run wasm:web が回す）。
//
//   node worker-node.mjs [--pkg DIR]      DIR は wasm-bindgen --target web の出力（既定 target/wasm-pkg/web）
//
// worker_threads 上の worker.js を GlobeWorker で包み、GlobeModeValidation の Decay を bdf で回して x(1) ≈ exp(-1) を確かめる。
// 進捗を受け、長い実行を cancel() で止める（S133。S157 から途中の列が返る）。そのあと panic させ、worker が作り直され（restarts === 1）、ファイルが再生されて、もう一度 Decay が回ることを確かめる。
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

const LONG = "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation (experiment(StopTime = 1000));\nend Long;\n";
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
  // 列ごとの結果（S051）: CSV でなく Float64Array。最後の行が final と同じ
  const c = await w.simulateColumns({ model: decay, settings: { solver: "bdf" } });
  assert.ok(c.response.ok && c.response.csv === null);
  assert.deepEqual(c.layout.names, ["time", "x"]);
  assert.ok(c.columns.every((col) => col instanceof Float64Array && col.length === c.layout.points));
  assert.equal(c.columns[1].at(-1), c.response.final.x);
  console.log(`simulateColumns: ${c.layout.points} 点 × ${c.columns.length} 列、x(1) = ${c.columns[1].at(-1)}`);
  // 結果のファイルを開く（S222）
  const opened = await w.readResults("r.csv", new TextEncoder().encode("time,x\n0,1\n1,0.5\n"));
  assert.deepEqual(opened.layout, { names: ["time", "x"], points: 2, defaults: ["x"] });
  assert.deepEqual([...opened.columns[1]], [1, 0.5]);
  assert.deepEqual(opened.parameters, []); // CSV にパラメータは無い（MAT は S236 の smoke.mjs）
  await assert.rejects(w.readResults("bad.csv", new TextEncoder().encode("x,y\n1,2\n")));
  // 進捗と中断（S133、ADR-0033）: onProgress が段と t を受ける。長い実行を cancel() する（下）
  const seen = [];
  await w.simulateColumns({ model: decay, settings: { solver: "esdirk34" } }, { onProgress: (p) => seen.push(p) });
  assert.deepEqual(seen.at(-1), { stage: "integration", time: 1, start_time: 0, stop_time: 1 });
  await w.setFile("Long.mo", LONG);
  // SharedArrayBuffer のフラグがあれば、cancel() は作り直さず、そこまでの列と GM-I11004 を普通の結果として返す（S157）
  let first;
  const started = new Promise((r) => (first = r));
  const long = w.simulateColumns({ model: "Long" }, { onProgress: (p) => p.time !== undefined && first(p) });
  const at = await started;
  w.cancel();
  const part = await long;
  assert.equal(w.restarts, 0);
  assert.ok(part.response.ok && part.response.diagnostics.some((d) => d.code === "GM-I11004"));
  assert.equal(part.response.termination.kind, "cancelled");
  assert.ok(part.layout.points > 1 && part.columns[0].at(-1) === part.response.termination.time && part.response.termination.time < 1000);
  await runDecay("止めた後の Decay bdf");
  console.log(`進捗 ${seen.length} 回（最後の t = ${seen.at(-1).time}）。Long を t = ${at.time.toFixed(1)} で止め、t = ${part.response.termination.time} までの ${part.layout.points} 点を受けた`);
  // SharedArrayBuffer を使わない配信（shared: false）では、terminate() して作り直し、止めた呼び出しは reject する（途中の列は返らない）
  const plain = new GlobeWorker(() => new Worker(new URL("./worker.js", import.meta.url)), {
    pkg: pathToFileURL(pkg).href,
    wasm: readFileSync(join(pkg, "globe_mode_wasm_bg.wasm")),
  }, { shared: false });
  try {
    await plain.setFile("Long.mo", LONG);
    let seen1;
    const go = new Promise((r) => (seen1 = r));
    const run = plain.simulateColumns({ model: "Long" }, { onProgress: (p) => p.time !== undefined && seen1(p) });
    await go;
    plain.cancel();
    await assert.rejects(run, (e) => e.cancelled === true);
    assert.equal(plain.restarts, 1);
    console.log("shared: false: terminate() して作り直し、reject した（restarts = 1）");
  } finally {
    await plain.terminate();
  }
  // 中断の経路（S201。レビューの指摘 1〜6）。猶予（graceMs）は試験が短くする
  const wasm = readFileSync(join(pkg, "globe_mode_wasm_bg.wasm"));
  const sleep = (t) => new Promise((r) => setTimeout(r, t));
  const mk = (opts, wrap = (x) => x) => new GlobeWorker(() => wrap(new Worker(new URL("./worker.js", import.meta.url))), { pkg: pathToFileURL(pkg).href, wasm }, opts);
  const cancelled = (e) => e.cancelled === true;
  async function longRun(g) {
    await g.setFile("Long.mo", LONG);
    let first;
    const started = new Promise((r) => (first = r));
    const run = g.simulateColumns({ model: "Long" }, { onProgress: (p) => p.time !== undefined && first(p) });
    await started;
    return { run }; // Promise のまま返すと async 関数が待ってしまう
  }
  const hang = (p, what) => Promise.race([p, sleep(5000).then(() => { throw new Error(`${what}: 5 秒たっても決まらない`); })]);
  {
    // 1: 止めるを 2 回押しても、応えたあとに残ったタイマーが今の worker を殺さない
    const g = mk({ graceMs: 300 });
    try {
      const { run } = await longRun(g);
      g.cancel(); g.cancel();
      assert.equal((await run).response.termination.kind, "cancelled");
      await sleep(700);
      assert.equal(g.restarts, 0, "1: cancel() 2 回のあとに作り直した");
      assert.match(await g.version(), /./);
    } finally { await g.terminate(); }
    // 2: 起動中に止めても、worker は使えるまま（以前は起動で落ちた扱いで二度と使えなかった）
    const h = mk();
    try {
      h.cancel();
      assert.match(await h.version(), /./, "2: 起動中に止めたあと使えない");
      assert.equal(h.restarts, 0);
    } finally { await h.terminate(); }
    // 3: 実行がキューで待つ間に止めると、その実行だけが cancelled で reject され、前の呼び出しは殺されない
    const q = mk();
    try {
      await q.version(); // 起動を終えておく
      await q.setFile("Long.mo", LONG);
      const before = q.simulate({ model: "Long", settings: { stop_time: 20 } }); // 前の呼び出し（simulate_columns ではない）
      const run = q.simulateColumns({ model: "Long" });
      q.cancel();
      await assert.rejects(hang(run, "3"), cancelled);
      assert.ok((await before).ok, "3: 前の呼び出しが殺された");
      assert.equal(q.restarts, 0);
    } finally { await q.terminate(); }
    // 5a: 応えない実行（Worker にフラグを渡さない）は、猶予のあと terminate() して作り直し、reject する
    const deaf = mk({ graceMs: 300 }, (wk) => { const post = wk.postMessage.bind(wk); wk.postMessage = (m, t) => post({ ...m, cancel: undefined }, t); return wk; });
    try {
      const { run } = await longRun(deaf);
      deaf.cancel();
      await assert.rejects(hang(run, "5a"), cancelled);
      assert.equal(deaf.restarts, 1);
      assert.match(await deaf.version(), /./);
    } finally { await deaf.terminate(); }
    // 4: 捨てた worker の遅れた応答は、新しい worker を殺さない
    const made = [];
    const late = mk({}, (wk) => { const on = wk.on.bind(wk); const rec = { wk, listeners: [] }; made.push(rec); wk.on = (ev, f) => { rec.listeners.push([ev, f]); return on(ev, f); }; return wk; });
    try {
      await assert.rejects(late.panicForTest("遅れ"), /遅れ/);
      assert.equal(late.restarts, 1);
      await late.version();
      for (const [ev, f] of made[0].listeners) if (ev === "message") f({ id: 1, ok: false, error: "遅れた応答", poisoned: true });
      await late.version();
      assert.equal(late.restarts, 1, "4: 捨てた worker の応答で作り直した");
    } finally { await late.terminate(); }
    // 6: 公開の terminate() は待っている呼び出しを reject する
    const t = mk();
    const { run } = await longRun(t);
    const rejected = assert.rejects(hang(run, "6"), /terminate/);
    await t.terminate();
    await rejected;
    await assert.rejects(hang(t.version(), "6 の後"), /./);
    // 7（#69）: 線形メモリが上限を越えたら、次の要求の前に worker を作り直す（ライブラリとファイルは再生する）。上限内なら作り直さない
    const big = mk({ memoryLimit: 1 });
    const fine = mk();
    try {
      const src = "model A\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend A;\n";
      for (const g of [big, fine]) await g.setFile("A.mo", src);
      assert.equal(big.restarts, 0, "7: 上限を越えた応答の前に作り直した");
      assert.ok((await big.check("A")).passed, "7: 作り直したあとファイルが再生されている");
      assert.equal(big.restarts, 1, "7: 上限を越えたのに作り直さない");
      assert.ok((await fine.check("A")).passed);
      assert.equal(fine.restarts, 0, "7: 上限内なのに作り直した");
    } finally { await big.terminate(); await fine.terminate(); }
    console.log("中断の経路: cancel() 2 回・起動中・キューの中・応えない実行・捨てた worker の応答・terminate() を確かめた");
  }
  const g = await w.graph("GlobeModeValidation.Connect.RealCircuit", 1);
  assert.ok(g.nodes.length > 0 && g.edges.length > 0, "graph: ノードと辺がある");
  await assert.rejects(w.graph("No.Such.Model", 1), /索引に無い/);
  console.log(`graph: ノード ${g.nodes.length}、辺 ${g.edges.length}`);
  // 読み取りの口と summary（S191）
  const ol = await w.outline("src/Modelica/GlobeModeValidation/Simulation/Decay.mo");
  assert.equal(ol.within, "GlobeModeValidation.Simulation");
  await assert.rejects(w.outline("No/Such.mo"), /ワークスペースに無い/);
  assert.ok((await w.deps(decay)).some((p) => p.endsWith("Decay.mo")));
  assert.ok((await w.flat(decay)).variables >= 1);
  assert.match((await w.check(decay)).summary, /OK/);
  console.log("outline・deps・flat・summary: 読めた");
  console.log(`wasm の線形メモリ ${(await w.memory()) / 2 ** 20 | 0} MiB`);

  await assert.rejects(w.panicForTest("試験の panic"), /試験の panic/);
  assert.equal(w.restarts, 1);
  console.log("panic: reject され、worker を作り直した（restarts = 1）");
  await runDecay("作り直し後の Decay bdf");
  assert.equal(w.restarts, 1);

  // 起動で落ちる worker（存在しないファイル）は作り直さず、呼び出しを reject する（S041）
  const dead = new GlobeWorker(() => new Worker(new URL("./no-such-worker.js", import.meta.url)), {});
  await assert.rejects(dead.version(), /異常終了/);
  await assert.rejects(dead.version(), /異常終了/);
  assert.equal(dead.restarts, 0);
  console.log("起動で落ちた worker: 作り直さず reject した（restarts = 0）");
  console.log("ok");
} finally {
  await w.terminate();
}

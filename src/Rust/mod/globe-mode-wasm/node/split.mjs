// MSL.json の分割配信（S301）の試験と測定。
//
//   node split.mjs --msl DIR [--list FILE] [--limit N] [--pkg DIR] [--json OUT] [--prefetch N]
//
// MSL を全部読んだ Worker と、分割配信（index.json と塊。web/pack.js が作る）を使う Worker で、FILE（既定 src/Modelica/msl-compare.txt。
// 1 行 1 モデル）の各モデルを check して、結果（passed・段階・診断のコード）が同じことを確かめる。分割は閉包が足りないと名前が解決できず
// 結果が変わる。各モデルの後に、取った塊の数・文字数と、線形メモリを出す。
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";
import assert from "node:assert/strict";
import { GlobeWorker } from "../web/client.js";
import { PREFETCH, pack } from "../web/pack.js";
import { defaultPkg, readMo, repo } from "./common.mjs";

const args = process.argv.slice(2);
const arg = (name, dflt) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : dflt;
};
const pkg = resolve(arg("--pkg", defaultPkg("web")));
const mslDir = resolve(arg("--msl", join(repo, "data/msl")));
const list = arg("--list", join(repo, "src/Modelica/msl-compare.txt"));
const models = readFileSync(list, "utf8").split("\n").map((s) => s.trim()).filter((s) => s && !s.startsWith("#")).slice(0, Number(arg("--limit", 1e9)));
const mib = (b) => +(b / 2 ** 20).toFixed(1);
const ms = (t) => +(performance.now() - t).toFixed(0);

const wasm = readFileSync(join(pkg, "globe_mode_wasm_bg.wasm"));
const gm = await import(pathToFileURL(join(pkg, "globe_mode_wasm.js")).href);
gm.initSync({ module: wasm });
const files = readMo(mslDir, repo);
const { index, chunks } = pack(files, gm.tree_stub, gm.file_deps);
index.prefetch = Number(arg("--prefetch", index.prefetch)); // 先読みの深さを変えて測る（S308）
const dir = mkdtempSync(join(tmpdir(), "globe-msl-"));
mkdirSync(join(dir, "c"));
chunks.forEach((c, i) => writeFileSync(join(dir, "c", `${i}.json`), JSON.stringify(c)));
writeFileSync(join(dir, "index.json"), JSON.stringify(index));
const full = JSON.stringify(files);
console.log(`ファイル ${Object.keys(files).length}、MSL.json ${mib(full.length)} MiB、索引 ${mib(JSON.stringify(index).length)} MiB、塊 ${chunks.length} 本（本文のまま ${Object.keys(index.real).length} ファイル）`);

const make = () => new GlobeWorker(() => new Worker(new URL("../web/worker.js", import.meta.url)), { pkg: pathToFileURL(pkg).href, wasm });
const shape = (r) => JSON.stringify([r.passed, r.failed_stage, (r.diagnostics ?? []).map((d) => d.code)]);

const a = make();
let t = performance.now();
await a.library(full, "MSL");
const fullLoad = ms(t);
const b = make();
await b.version();
t = performance.now();
const got = await b.libraryIndex(pathToFileURL(join(dir, "index.json")).href, "MSL");
const lazyLoad = ms(t);
assert.ok(got && got.files === Object.keys(files).length, JSON.stringify(got));
assert.equal(await b.libraryIndex(pathToFileURL(join(dir, "none.json")).href, "X"), null); // 索引が無ければ null（呼び手は MSL.json に戻す）
console.log(`読み込み: 全部 ${fullLoad} ms、分割（索引だけ）${lazyLoad} ms。線形メモリ ${mib(await a.memory())} → ${mib(await b.memory())} MiB`);

const rows = [];
let bad = 0;
for (const model of models) {
  const t1 = performance.now();
  const x = await a.check(model);
  const tf = ms(t1);
  const t2 = performance.now();
  const y = await b.check(model);
  const tl = ms(t2);
  const st = await b.libraryStats();
  const same = shape(x) === shape(y);
  if (!same) bad++;
  rows.push({ model, same, passed: y.passed, full_ms: tf, split_ms: tl, chunks: st.chunks, rounds: st.rounds, mchars: +(st.bytes / 1e6).toFixed(2), mem_mib: mib(await b.memory()) });
  console.log(`${same ? "一致" : "不一致"} ${model} passed=${y.passed} 全部 ${tf} ms / 分割 ${tl} ms、塊 ${st.chunks}/${st.of}、往復 ${st.rounds}、${(st.bytes / 2 ** 20).toFixed(2)} MiB、線形メモリ ${mib(await b.memory())} MiB（全部 ${mib(await a.memory())}）`);
  if (!same) console.log(`  全部 ${shape(x)}\n  分割 ${shape(y)}`);
}
// 往復の回数（S308）: 既定の先読みなら、最初のモデル（既定は ChuaCircuit。先読み無しでは 11 回）の塊の取得が 4 回以内。取る塊は先読み無しの 1.2 倍以内
if (index.prefetch === PREFETCH && models[0]?.endsWith("ChuaCircuit")) assert.ok(rows[0].rounds <= 4 && rows[0].chunks <= 190, JSON.stringify(rows[0]));
// 作業中の文書が MSL を参照するとき（ライブラリの外のモデル）も、文書の閉包の塊を取る
const doc = "model UserCircuit\n  extends Modelica.Electrical.Analog.Examples.Rectifier;\nend UserCircuit;\n";
await a.setFile("UserCircuit.mo", doc);
await b.setFile("UserCircuit.mo", doc);
const ux = await a.check("UserCircuit"), uy = await b.check("UserCircuit");
assert.equal(shape(ux), shape(uy));
console.log(`作業中の文書: passed=${uy.passed}（全部読みと同じ）、塊 ${(await b.libraryStats()).chunks}`);
// ライブラリの木と、木から開くファイル（本文を取る）
assert.deepEqual(JSON.parse(await a.libraryTree("Modelica.Blocks.Sources").then(JSON.stringify)).map((e) => e.name), (await b.libraryTree("Modelica.Blocks.Sources")).map((e) => e.name));
const sa = await a.librarySource("Modelica.Blocks.Sources.Sine"), sb = await b.librarySource("Modelica.Blocks.Sources.Sine");
assert.equal(sa.text, sb.text);
assert.deepEqual(sa.range, sb.range);
console.log(`木と開くファイル: 一致、塊 ${(await b.libraryStats()).chunks}`);
const stats = await b.libraryStats();
const summary = { files: Object.keys(files).length, models: models.length, mismatched: bad, full_load_ms: fullLoad, lazy_load_ms: lazyLoad, chunks: stats.of, chunks_got: stats.chunks, rounds: stats.rounds, prefetch: index.prefetch, mib_got: mib(stats.bytes), mib_full: mib(full.length) };
console.log(JSON.stringify(summary));
await a.terminate();
await b.terminate();
if (arg("--json")) writeFileSync(arg("--json"), JSON.stringify({ summary, rows }, null, 1));
assert.equal(bad, 0, `${bad} 本が全部読みと違う`);

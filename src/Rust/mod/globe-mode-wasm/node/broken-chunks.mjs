// 分割配信の塊が壊れている・索引と合わないときに、Worker が固まらず誤りで返り、直せば再試行で通ることの試験（#110）。
//
//   node broken-chunks.mjs [--pkg DIR]     （MSL は要らない。3 ファイルの小さなライブラリを pack する）
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";
import assert from "node:assert/strict";
import { GlobeWorker } from "../web/client.js";
import { pack } from "../web/pack.js";
import { defaultPkg } from "./common.mjs";

const args = process.argv.slice(2);
const pkg = resolve(args.includes("--pkg") ? args[args.indexOf("--pkg") + 1] : defaultPkg("web"));
const wasm = readFileSync(join(pkg, "globe_mode_wasm_bg.wasm"));
const gm = await import(pathToFileURL(join(pkg, "globe_mode_wasm.js")).href);
gm.initSync({ module: wasm });
const files = {
  "Lib/package.mo": "package Lib end Lib;\n",
  "Lib/A.mo": "within Lib;\nmodel A\n  extends Lib.B;\nend A;\n",
  "Lib/B.mo": "within Lib;\nmodel B\n  Real x(start=1);\nequation\n  der(x) = -x;\nend B;\n",
};
const { index, chunks } = pack(files, gm.tree_stub, gm.file_deps, 30);
const dir = mkdtempSync(join(tmpdir(), "globe-broken-"));
mkdirSync(join(dir, "c"));
const put = (i, text) => writeFileSync(join(dir, "c", `${i}.json`), text);
chunks.forEach((c, i) => put(i, JSON.stringify(c)));
writeFileSync(join(dir, "index.json"), JSON.stringify(index));
const bad = index.chunk["Lib/B.mo"];
assert.notEqual(bad, undefined, "B.mo が塊に入っていない（試験の前提）");

const w = new GlobeWorker(() => new Worker(new URL("../web/worker.js", import.meta.url)), { pkg: pathToFileURL(pkg).href, wasm });
await w.version();
await w.libraryIndex(pathToFileURL(join(dir, "index.json")).href, "Lib");
// 固まる不具合は await が返らないので、時間切れで落とす
const within = (p, what) => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error(`固まった: ${what}`)), 5000))]);
const check = (what) => within(w.check("Lib.A").then((r) => r.passed, (e) => String(e.message ?? e)), what);

put(bad, "{}"); // 索引が言うファイルが塊に無い
assert.match(await check("塊に無い 1 回目"), /塊に無いファイル/);
assert.match(await check("塊に無い 2 回目"), /塊に無いファイル/);
put(bad, "<html>"); // SPA の fallback
assert.match(await check("壊れた 1 回目"), /壊れている/);
assert.match(await check("壊れた 2 回目"), /壊れている/);
put(bad, JSON.stringify(chunks[bad])); // 配信が直れば再試行で通る
assert.equal(await check("直したあと"), true);
console.log("壊れた塊・索引と合わない塊: 固まらず誤りで返り、直せば通る");
await w.terminate();

// data/msl（cargo xtask fetch-msl の取得物）の .mo を { "data/msl/…": 本文 } の JSON にして public/msl/MSL.json に置く（S048。.gitignore 済み）。
// 形は globe-mode-wasm/node/smoke.mjs の readMo と同じ（Workspace のライブラリの JSON）。data/msl が無ければ警告だけで飛ばす。
// S301: ブラウザ版（WorkerBackend）は分割配信を使う。msl/index.json（索引。ファイルごとの見出し `tree_stub` と、ファイル → 塊の番号）と、
// msl/c/<番号>.json（同じディレクトリのファイルを CAP 以内でまとめた塊 { 名前: 本文 }）を作る。Worker は開いたモデルの閉包に要る塊だけを取る
// （globe-mode-wasm/web/worker.js）。MSL.json は Leptos・Dioxus・marimo と、index.json が無いときの代わり。wasm（public/wasm/pkg）が無ければ作らない。
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { pack } from "../../Rust/mod/globe-mode-wasm/web/pack.js";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const repo = here("../../..");
const dir = join(repo, "data/msl");
const out = here("../public/msl");
rmSync(out, { recursive: true, force: true });
if (!existsSync(dir)) {
  console.warn("data/msl が無いので MSL.json は作らない（cargo xtask fetch-msl）");
  process.exit(0);
}
function readMo(d, acc = {}) {
  for (const e of readdirSync(d).sort()) {
    const p = join(d, e);
    if (statSync(p).isDirectory()) readMo(p, acc);
    else if (p.endsWith(".mo")) acc[relative(repo, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return acc;
}
const files = readMo(dir);
mkdirSync(out, { recursive: true });
writeFileSync(`${out}/MSL.json`, JSON.stringify(files));
console.log(`MSL.json: ${Object.keys(files).length} ファイル、${(statSync(`${out}/MSL.json`).size / 2 ** 20).toFixed(1)} MiB`);

// S301: 分割配信
const pkg = here("../public/wasm/pkg");
if (!existsSync(`${pkg}/globe_mode_wasm.js`)) {
  console.warn("public/wasm/pkg が無いので分割配信（index.json）は作らない（先に copy-wasm）");
  process.exit(0);
}
const gm = await import(pathToFileURL(`${pkg}/globe_mode_wasm.js`).href);
gm.initSync({ module: readFileSync(`${pkg}/globe_mode_wasm_bg.wasm`) });
const { index, chunks } = pack(files, gm.tree_stub);
mkdirSync(`${out}/c`, { recursive: true });
chunks.forEach((c, i) => writeFileSync(`${out}/c/${i}.json`, JSON.stringify(c)));
writeFileSync(`${out}/index.json`, JSON.stringify(index));
console.log(`index.json: ${(statSync(`${out}/index.json`).size / 2 ** 10).toFixed(0)} KiB、塊 ${chunks.length} 本、本文のまま ${Object.keys(index.real).length} ファイル`);

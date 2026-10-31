// data/msl（cargo xtask fetch-msl の取得物）の .mo を { "data/msl/…": 本文 } の JSON にして public/msl/MSL.json に置く（S048。.gitignore 済み）。
// 形は globe-mode-wasm/node/smoke.mjs の readMo と同じ（Workspace のライブラリの JSON）。data/msl が無ければ警告だけで飛ばす。
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

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

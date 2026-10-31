// wasm:build の出力（--target web の糊と .wasm）と worker.js を public/wasm/ に置く（.gitignore 済み）。
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const target = process.env.CARGO_TARGET_DIR ?? here("../../../target");
const pkg = `${target}/wasm-pkg/web`;
if (!existsSync(`${pkg}/globe_mode_wasm.js`)) {
  console.error(`${pkg} が無い。先に mise run wasm:build`);
  process.exit(1);
}
const out = here("../public/wasm");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
cpSync(pkg, `${out}/pkg`, { recursive: true });
cpSync(here("../../Rust/mod/globe-mode-wasm/web/worker.js"), `${out}/worker.js`);

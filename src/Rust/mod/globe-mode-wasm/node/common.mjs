// node/ のスクリプトの共通部分（S203）。
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");

/** `mise run wasm:build` の置き場の `wasm-pkg/<kind>`（kind は node か web）。 */
export const defaultPkg = (kind) => join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg", kind);

/** `p`（ファイルか、その下の .mo をすべて読むディレクトリ）を、`base` からの相対の名前 → 本文で `out` に足す。 */
export function readMo(p, base, out = {}) {
  if (statSync(p).isDirectory()) {
    for (const e of readdirSync(p).sort()) readMo(join(p, e), base, out);
  } else if (p.endsWith(".mo")) {
    out[relative(base, resolve(p)).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}

// wasm（Node）とネイティブの `globe simulate` を突き合わせる（S012）。
//
//   node compare.mjs --globe <globe の実行ファイル> [--pkg DIR] [--solvers bdf,dopri45,auto] [--models A,B]
//        [--rtol 1e-9]
//
// GlobeModeValidation.Simulation の例題（package.order の順）を、ソルバごとに両方で回し、次を比べる。
//   - 成功したか（wasm の ok とネイティブの succeeded）
//   - 診断のコードの集合
//   - 結果の CSV（行数と、すべての値。相対の差が --rtol 以下か。ulp の数も出す）
// --solvers を省くと、登録表（solvers()）の実装のあるソルバすべて。一致しないものがあれば終了コード 1
// （既知の差 KNOWN_CSV_DIFFERENCES は、成功と診断のコードが一致していれば CSV の差を数えない）。
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const args = process.argv.slice(2);
const arg = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const globe = arg("--globe");
if (!globe) {
  console.error("usage: node compare.mjs --globe PATH [--pkg DIR] [--solvers a,b] [--models A,B] [--rtol X]");
  process.exit(2);
}
const pkg = resolve(arg("--pkg") ?? join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg/node"));
const rtol = Number(arg("--rtol") ?? 1e-9);
const gm = createRequire(import.meta.url)(join(pkg, "globe_mode_wasm.js"));

const solvers = arg("--solvers")?.split(",") ??
  JSON.parse(gm.solvers()).filter((s) => s.status.kind === "available").map((s) => s.name);
const simDir = join(repo, "src/Modelica/GlobeModeValidation/Simulation");
const models = (arg("--models")?.split(",") ??
  readFileSync(join(simDir, "package.order"), "utf8").split("\n").map((l) => l.trim()).filter(Boolean))
  .map((m) => (m.includes(".") ? m : `GlobeModeValidation.Simulation.${m}`));

// ネイティブの `globe simulate` と同じく、パッケージのディレクトリを文書として渡す（名前は repo からの相対）
const pkgDir = join(repo, "src/Modelica/GlobeModeValidation");
function readMo(dir, out = {}) {
  for (const e of readdirSync(dir).sort()) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) readMo(p, out);
    else if (p.endsWith(".mo")) out[relative(repo, p).replaceAll("\\", "/")] = readFileSync(p, "utf8");
  }
  return out;
}
const ws = new gm.Workspace();
for (const [name, text] of Object.entries(readMo(pkgDir))) ws.set_file(name, text);

const tmp = mkdtempSync(join(tmpdir(), "globe-wasm-compare-"));
const codes = (ds) => [...new Set(ds.map((d) => d.code))].sort().join(" ");

function native(model, solver) {
  const csv = join(tmp, "native.csv");
  rmSync(csv, { force: true });
  let out;
  try {
    out = execFileSync(globe, ["simulate", relative(repo, pkgDir), "-m", model, "--solver", solver, "--json", "-o", csv], {
      cwd: repo,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    out = e.stdout; // 終了コード 1（モデルの誤り・実行の失敗）でも JSON は出る
  }
  const r = JSON.parse(out);
  let text = null;
  try {
    text = readFileSync(csv, "utf8");
  } catch {}
  return { ok: r.succeeded, codes: codes(r.diagnostics), csv: text };
}

/** 2 つの f64 の間の ulp の数（符号が違えば Infinity）。 */
function ulps(a, b) {
  if (a === b) return 0;
  if (Math.sign(a) !== Math.sign(b)) return Infinity;
  const buf = new DataView(new ArrayBuffer(16));
  buf.setFloat64(0, Math.abs(a));
  buf.setFloat64(8, Math.abs(b));
  return Number(buf.getBigUint64(0) > buf.getBigUint64(8) ? buf.getBigUint64(0) - buf.getBigUint64(8) : buf.getBigUint64(8) - buf.getBigUint64(0));
}

function compareCsv(a, b) {
  if (a === null || b === null) return a === b ? { same: true, rows: 0, maxRel: 0, maxUlps: 0 } : { same: false, why: "片方だけ CSV がある" };
  const ra = a.trim().split("\n");
  const rb = b.trim().split("\n");
  if (ra[0] !== rb[0]) return { same: false, why: "列の名前が違う" };
  if (ra.length !== rb.length) return { same: false, why: `行数が違う（${ra.length - 1} と ${rb.length - 1}）` };
  let maxRel = 0;
  let maxUlps = 0;
  for (let i = 1; i < ra.length; i++) {
    const va = ra[i].split(",").map(Number);
    const vb = rb[i].split(",").map(Number);
    for (let j = 0; j < va.length; j++) {
      if (Number.isNaN(va[j]) && Number.isNaN(vb[j])) continue;
      const d = Math.abs(va[j] - vb[j]);
      const rel = d === 0 ? 0 : d / Math.max(Math.abs(va[j]), Math.abs(vb[j]), 1e-12);
      maxRel = Math.max(maxRel, rel);
      maxUlps = Math.max(maxUlps, ulps(va[j], vb[j]));
    }
  }
  return { same: maxRel <= rtol, rows: ra.length - 1, maxRel, maxUlps };
}

// 既知の差（S012）: 自前の刻み幅を制御する積分器（diffsol の SDIRK・陽的 RK）は、BouncingBall のイベントの
// 近くで、ネイティブと wasm の丸めの違い（faer の SIMD の有無の見込み【未確認】）から、イベントの時刻が出力の
// 格子の点とほぼ重なるところの行の数（1 行）や値が分かれる。成功と診断のコードは一致する。
// S027 の自前の係数表の SDIRK（esdirk23・esdirk45・sdirk34hw）も同じ道で、同じ所で分かれる
const KNOWN_CSV_DIFFERENCES = new Set([
  "BouncingBall/esdirk34",
  "BouncingBall/trbdf2",
  "BouncingBall/tsit45",
  "BouncingBall/esdirk23",
  "BouncingBall/esdirk45",
  "BouncingBall/sdirk34hw",
]);

let failures = 0;
let known = 0;
const rows = [];
for (const model of models) {
  for (const solver of solvers) {
    const t = performance.now();
    const w = JSON.parse(ws.simulate(JSON.stringify({ model, settings: { solver } })));
    const wasmMs = performance.now() - t;
    const n = native(model, solver);
    const c = compareCsv(w.csv ?? null, n.csv);
    const wc = codes(w.diagnostics);
    const same = w.ok === n.ok && wc === n.codes && c.same;
    const name = model.replace("GlobeModeValidation.Simulation.", "");
    const tolerated = !same && w.ok === n.ok && wc === n.codes && KNOWN_CSV_DIFFERENCES.has(`${name}/${solver}`);
    if (tolerated) known++;
    else if (!same) failures++;
    rows.push({
      model: name,
      solver,
      ok: `${w.ok}/${n.ok}`,
      codes: wc === n.codes ? wc || "-" : `${wc || "-"} ≠ ${n.codes || "-"}`,
      rows: c.rows ?? "-",
      max_rel: c.maxRel === undefined ? c.why : c.maxRel.toExponential(1),
      max_ulps: c.maxUlps ?? "-",
      wasm_ms: +wasmMs.toFixed(0),
      same: same ? "○" : tolerated ? "△（既知）" : "×",
    });
  }
}
rmSync(tmp, { recursive: true, force: true });
console.table(rows);
console.log(
  `${rows.length - failures - known} / ${rows.length} 件が一致、既知の差 ${known} 件、一致しない ${failures} 件` +
    `（wasm の線形メモリ ${(gm.memory_bytes() / 2 ** 20).toFixed(0)} MiB）`,
);
process.exit(failures ? 1 : 0);

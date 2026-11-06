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
// （既知の差 KNOWN_CSV_DIFFERENCES は、成功と診断のコードが一致していて、行数の差が 1 以下、同じ行数なら相対の差が
// 一覧の上限以下のときだけ、CSV の差を数えない。それを超えたら一致しないものに数える。S203）。
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import { createRequire } from "node:module";
import { compareCsv, withinKnownBound } from "./csv-compare.mjs";
import { defaultPkg, readMo, repo } from "./common.mjs";

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
const pkg = resolve(arg("--pkg") ?? defaultPkg("node"));
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
const ws = new gm.Workspace();
for (const [name, text] of Object.entries(readMo(pkgDir, repo))) ws.set_file(name, text);

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

// 既知の差（S012）: 自前の刻み幅を制御する積分器（diffsol の SDIRK・陽的 RK）は、BouncingBall のイベントの
// 近くで、ネイティブと wasm の丸めの違い（faer の SIMD の有無の見込み【未確認】）から、イベントの時刻が出力の
// 格子の点とほぼ重なるところの行の数（1 行）や値が分かれる。成功と診断のコードは一致する。
// S027 の自前の係数表の SDIRK（esdirk23・esdirk45・sdirk34hw）も同じ道で、同じ所で分かれる。S028 の dopri853
// は t ≈ 0.4515 でイベントの行が 1 行増え、t ≈ 1.174 で 1 行減るので、行数は同じでもその間の行がずれる。
// S029 の cerk23・cerk34・cerk45 は、BouncingBall で同じように分かれるほか、硬い DiodeSwitching で陽的 RK が
// 安定の限界の刻みを刻むため、最初の出力行（t = 1e-4）から刻みの列が分かれる（相対の差は最大 8.9e-6、
// 絶対では許容誤差 1e-6 の内側。丸めの違いは exp の実装の差の見込み【未確認】）
// S203: 既知の差にも上限を持たせる（行数の差は 1 以下、同じ行数なら相対の差の最大が値以下。観測した最大は
// BouncingBall の esdirk34 9.4e-4・esdirk45 1.8e-3・sdirk34hw 3.1e-3・cerk23 2.6e-3・cerk34 6.0e-4、
// DiodeSwitching の cerk23 1.6e-5・cerk34 5.3e-6・cerk45 8.9e-6。行数が違うものは値を比べない）。
// dopri853 は行がずれて 1.8 になる（相対の差は最大 2 なので、上限は行数・列の名前・有限でない値の食い違いだけを守る）
const BOUNCING_BALL_REL = 1e-2;
const KNOWN_CSV_DIFFERENCES = new Map([
  ["BouncingBall/esdirk34", BOUNCING_BALL_REL],
  ["BouncingBall/trbdf2", BOUNCING_BALL_REL],
  ["BouncingBall/tsit45", BOUNCING_BALL_REL],
  ["BouncingBall/esdirk23", BOUNCING_BALL_REL],
  ["BouncingBall/esdirk45", BOUNCING_BALL_REL],
  ["BouncingBall/sdirk34hw", BOUNCING_BALL_REL],
  ["BouncingBall/dopri853", 2],
  ["BouncingBall/cerk23", BOUNCING_BALL_REL],
  ["BouncingBall/cerk34", BOUNCING_BALL_REL],
  ["BouncingBall/cerk45", BOUNCING_BALL_REL],
  ["DiodeSwitching/cerk23", 1e-4],
  ["DiodeSwitching/cerk34", 1e-4],
  ["DiodeSwitching/cerk45", 1e-4],
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
    const c = compareCsv(w.csv ?? null, n.csv, rtol);
    const wc = codes(w.diagnostics);
    const same = w.ok === n.ok && wc === n.codes && c.same;
    const name = model.replace("GlobeModeValidation.Simulation.", "");
    const bound = KNOWN_CSV_DIFFERENCES.get(`${name}/${solver}`);
    const tolerated = !same && w.ok === n.ok && wc === n.codes && bound !== undefined && withinKnownBound(c, bound);
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

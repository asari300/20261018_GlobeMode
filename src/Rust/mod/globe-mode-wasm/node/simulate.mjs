// globe-mode-wasm を Node で回す（S012。`globe simulate` の wasm 版）。
//
//   node simulate.mjs [--pkg DIR] --model NAME [--library DIR]... [--output CSV]
//        [--solver bdf] [--start-time 0] [--stop-time 1] [--tolerance 1e-6] [--interval 0.01] [--step H]
//        [--param NAME=VALUE]... [--timeout SEC] [--force] [--response] <ファイルかディレクトリ>...
//
// --pkg は `wasm-bindgen --target nodejs` の出力（省略すると mise run wasm:build の置き場
// `${CARGO_TARGET_DIR:-target}/wasm-pkg/node`）。ディレクトリはその下の .mo をすべて読む（globe simulate と
// 同じ）。--library のディレクトリ（例: data/msl）は、in-memory のライブラリとして一度だけ読む。
// 表示は要約の JSON（時間・wasm の線形メモリ・Node の RSS を含む）。--response で応答をすべて出す。
// 終了コードは 0（成功）、1（モデルの誤りか実行の失敗）、2（使い方の誤り）、3（wasm の panic）。
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");

const args = process.argv.slice(2);
const opt = { files: [], libraries: [], settings: {} };
for (let i = 0; i < args.length; i++) {
  const a = args[i];
  const next = () => {
    if (i + 1 >= args.length) usage(`${a} の値が無い`);
    return args[++i];
  };
  const num = () => {
    const v = Number(next());
    if (!Number.isFinite(v)) usage(`${a} の値が数ではない`);
    return v;
  };
  switch (a) {
    case "--pkg": opt.pkg = next(); break;
    case "--model": case "-m": opt.model = next(); break;
    case "--library": case "-L": opt.libraries.push(next()); break;
    case "--output": case "--out": case "-o": opt.output = next(); break;
    case "--solver": opt.settings.solver = next(); break;
    case "--start-time": opt.settings.start_time = num(); break;
    case "--stop-time": opt.settings.stop_time = num(); break;
    case "--tolerance": opt.settings.tolerance = num(); break;
    case "--interval": opt.settings.interval = num(); break;
    case "--step": opt.settings.step = num(); break;
    case "--timeout": opt.settings.max_wall_seconds = num(); break;
    case "--param": {
      const [k, v] = next().split("=");
      if (!k || v === undefined || !Number.isFinite(Number(v))) usage("--param は NAME=VALUE");
      (opt.settings.parameters ??= {})[k] = Number(v);
      break;
    }
    case "--force": opt.force = true; break;
    case "--response": opt.response = true; break;
    case "--help": case "-h": usage(); break;
    default:
      if (a.startsWith("-")) usage(`知らない引数 ${a}`);
      opt.files.push(a);
  }
}
if (!opt.model) usage("--model が要る");

function usage(message) {
  if (message) console.error(`error: ${message}`);
  console.error("usage: node simulate.mjs [--pkg DIR] --model NAME [--library DIR]... [--output CSV] [設定] FILES...");
  process.exit(2);
}

/** `p` の下の .mo を（cwd からの相対の名前 → 本文で）`out` に足す。読んだバイト数を返す。 */
function readMo(p, out) {
  let bytes = 0;
  if (statSync(p).isDirectory()) {
    for (const e of readdirSync(p).sort()) bytes += readMo(join(p, e), out);
  } else if (p.endsWith(".mo")) {
    const text = readFileSync(p, "utf8");
    out[relative(process.cwd(), resolve(p)).replaceAll("\\", "/")] = text;
    bytes += Buffer.byteLength(text);
  }
  return bytes;
}

const pkg = resolve(opt.pkg ?? join(process.env.CARGO_TARGET_DIR ?? join(repo, "target"), "wasm-pkg/node"));
const t0 = performance.now();
const require = createRequire(import.meta.url);
const gm = require(join(pkg, "globe_mode_wasm.js"));
const t1 = performance.now();

const library = {};
let libraryBytes = 0;
for (const l of opt.libraries) libraryBytes += readMo(l, library);
const files = {};
let bytes = 0;
for (const f of opt.files) bytes += readMo(f, files);
const t2 = performance.now();

let response;
let ws;
try {
  const name = opt.libraries.map((l) => basename(resolve(l))).join("+") || undefined;
  ws = opt.libraries.length ? new gm.Workspace(JSON.stringify(library), name) : new gm.Workspace();
} catch (e) {
  failed(e, "ライブラリを読めない");
}
const t3 = performance.now();
try {
  response = JSON.parse(
    ws.simulate(JSON.stringify({ files, model: opt.model, settings: opt.settings, force: !!opt.force })),
  );
} catch (e) {
  failed(e, "実行できない");
}
const t4 = performance.now();

function failed(e, what) {
  const panic = gm.last_panic?.();
  console.error(`error: ${what}: ${panic ?? e}`);
  process.exit(panic ? 3 : 1);
}

if (opt.output && response.csv) writeFileSync(opt.output, response.csv);
const ms = (x) => +x.toFixed(1);
const errors = response.diagnostics.filter((d) => d.severity === "error");
const summary = {
  model: response.model,
  ok: response.ok,
  refused: response.refused,
  files: Object.keys(files).length,
  source_mb: +(bytes / 1e6).toFixed(2),
  library_files: ws.library_files,
  library_mb: +(libraryBytes / 1e6).toFixed(1),
  timings_ms: {
    instantiate: ms(t1 - t0),
    read: ms(t2 - t1),
    library: ms(t3 - t2),
    simulate: ms(t4 - t3),
    ...Object.fromEntries(Object.entries(response.timings_ms).map(([k, v]) => [k, ms(v)])),
  },
  wasm_memory_mb: +(gm.memory_bytes() / 2 ** 20).toFixed(0),
  rss_mb: +(process.memoryUsage().rss / 2 ** 20).toFixed(0),
  settings: response.settings && {
    solver: response.settings.solver,
    start_time: response.settings.start_time,
    stop_time: response.settings.stop_time,
    tolerance: response.settings.tolerance,
    interval: response.settings.interval,
  },
  stats: response.stats,
  termination: response.termination,
  codes: [...new Set(response.diagnostics.map((d) => d.code))].sort(),
  errors: errors.map((d) => `${d.code} ${d.message}`),
};
console.log(JSON.stringify(opt.response ? { ...response, csv: undefined, summary } : summary, null, 2));
process.exit(response.ok ? 0 : 1);

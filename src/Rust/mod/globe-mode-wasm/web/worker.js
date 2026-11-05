// globe-mode-wasm を Web Worker（ブラウザの module Worker と Node の worker_threads の両方）で回す（S036）。
//
//   ブラウザ: new Worker("worker.js", { type: "module" })
//   Node    : new (await import("node:worker_threads")).Worker(new URL("worker.js", import.meta.url))
//
// 呼び方は client.js の GlobeWorker が受け持つ。ここは `{id, op, …}` を受けて `{id, ok, value}` を返すだけ。
//   init {pkg, wasm?}  : `${pkg}/globe_mode_wasm.js`（wasm-bindgen --target web の糊）を読んで初期化。
//                        wasm に ArrayBuffer を渡すと、それで初期化（Node は file: を fetch できないため）
//   simulate_columns {request, progress?}（{response, layout, columns: Float64Array[]}。buffer は転送。progress で進捗も送る。S133）、library {json, name}, set_file {name, text}, remove_file {name}, check {model}（CheckResponse の JSON）, graph {model, depth}（graphology の JSON）, simulate {request}（JSON 文字列）、
//   library_tree {prefix}・library_source {path}（S127。JSON。ライブラリが無ければ [] と例外）, definition {file, text, line, character}（S138。{file, text, range} の JSON か null）, hover {同じ}（S149。{contents, range} の JSON か null）, tunable_parameters {model}（S150。ParametersResponse の JSON。変換できなければ例外）, export_fmu {model, fmiVersion}（S144。ソースの FMU の zip の Uint8Array。失敗は例外）,
//   format {text, indent, insertOrigin}（整形後の本文）, lint {text}（JSON の配列）（S053）,
//   version, solvers, memory, panic_for_test {message}
// 失敗は `{id, ok:false, error, panic, poisoned}`。wasm32 は panic=abort なので、WebAssembly.RuntimeError のあとの
// インスタンスは使えない（poisoned: true）。作り直しは client.js 側（worker を捨てて、新しく起こして、再生する）。
const isWeb = typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;
const port = isWeb ? self : (await import("node:worker_threads")).parentPort;
const send = (m, transfer) => port.postMessage(m, transfer);

let gm; // 糊のモジュール
let ws = null; // 現在の Workspace（library か、最初の set_file で作る）

const ops = {
  async init({ pkg, wasm }) {
    gm = await import(`${pkg}/globe_mode_wasm.js`);
    await gm.default({ module_or_path: wasm ?? `${pkg}/globe_mode_wasm_bg.wasm` });
  },
  library({ json, name }) {
    const next = new gm.Workspace(json, name); // 読めなければ投げ、今の ws は残す
    ws?.free();
    ws = next;
  },
  set_file({ name, text }) {
    (ws ??= new gm.Workspace()).set_file(name, text);
  },
  remove_file({ name }) {
    return (ws ??= new gm.Workspace()).remove_file(name);
  },
  check({ model }) {
    return (ws ??= new gm.Workspace()).check(model);
  },
  graph({ model, depth }) {
    return (ws ??= new gm.Workspace()).graph(model, depth);
  },
  simulate({ request }) {
    return (ws ??= new gm.Workspace()).simulate(request);
  },
  library_tree({ prefix }) {
    return (ws ??= new gm.Workspace()).library_tree(prefix);
  },
  library_source({ path }) {
    return (ws ??= new gm.Workspace()).library_source(path);
  },
  definition({ file, text, line, character }) {
    return (ws ??= new gm.Workspace()).definition(file, text, line, character); // S138。{file, text, range} の JSON か "null"
  },
  hover({ file, text, line, character }) {
    return (ws ??= new gm.Workspace()).hover(file, text, line, character); // S149。{contents, range} の JSON か "null"
  },
  tunable_parameters: ({ model }) => (ws ??= new gm.Workspace()).tunable_parameters(model), // S150。ParametersResponse の JSON
  export_fmu: ({ model, fmiVersion }) => (ws ??= new gm.Workspace()).export_fmu(model, fmiVersion),
  format: ({ text, indent, insertOrigin }) => gm.format(text, indent, insertOrigin), // S053。壊れていれば投げる
  lint: ({ text }) => gm.lint(text),
  // 時系列を列ごとの Float64Array で返す（S051。CSV の文字列を作らず、解析もしない。buffer は転送する）
  // progress が真なら、進捗を {id, type: "progress", stage, time, start_time, stop_time} で親に送る（S133、ADR-0033）
  simulate_columns({ request, progress }, id) {
    const notify = (stage, time, start_time, stop_time) => send({ id, type: "progress", stage, time, start_time, stop_time });
    const c = (ws ??= new gm.Workspace()).simulate_columns(request, progress ? notify : undefined);
    try {
      const layout = JSON.parse(c.layout());
      return { response: c.response, layout, columns: layout.names.map((_, i) => c.column(i)) };
    } finally {
      c.free();
    }
  },
  version: () => gm.version(),
  solvers: () => gm.solvers(),
  memory: () => gm.memory_bytes(),
  panic_for_test: ({ message }) => gm.panic_for_test(message),
};

async function handle({ id, op, ...args }) {
  try {
    if (!ops[op]) throw new Error(`未知の op: ${op}`);
    const value = await ops[op](args, id);
    send({ id, ok: true, value }, value?.columns?.map((c) => c.buffer));
  } catch (e) {
    const poisoned = typeof WebAssembly !== "undefined" && e instanceof WebAssembly.RuntimeError;
    // simulate の失敗は文字列の例外のこともある
    send({ id, ok: false, error: String(e?.message ?? e), panic: poisoned ? gm.last_panic() : undefined, poisoned });
  }
}

if (isWeb) port.onmessage = (e) => handle(e.data);
else port.on("message", handle);

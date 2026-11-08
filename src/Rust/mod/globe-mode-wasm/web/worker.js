// globe-mode-wasm を Web Worker（ブラウザの module Worker と Node の worker_threads の両方）で回す（S036）。
//
//   ブラウザ: new Worker("worker.js", { type: "module" })
//   Node    : new (await import("node:worker_threads")).Worker(new URL("worker.js", import.meta.url))
//
// 呼び方は client.js の GlobeWorker が受け持つ。ここは `{id, op, …}` を受けて `{id, ok, value}` を返すだけ。
//   init {pkg, wasm?}  : `${pkg}/globe_mode_wasm.js`（wasm-bindgen --target web の糊）を読んで初期化。
//                        wasm に ArrayBuffer を渡すと、それで初期化（Node は file: を fetch できないため）
//   simulate_columns {request, progress?, cancel?}（{response, layout, columns: Float64Array[]}。buffer は転送。progress で進捗も送る。S133）、read_results {name, bytes}（{layout, columns}。S222）, library {json, name}, set_file {name, text}, remove_file {name}, check {model}（CheckResponse の JSON）, graph {model, depth}（graphology の JSON）, simulate {request}（JSON 文字列）、
//   library_tree {prefix}・library_search {query, limit}（S167。名前の部分一致。木と同じ形の JSON）・library_source {path}（S127。JSON。ライブラリが無ければ [] と例外）, definition {file, text, line, character}（S138。{file, text, range} の JSON か null）, hover {同じ}（S149。{contents, range} の JSON か null）, completion {同じ}（S171。LSP の CompletionItem の配列の JSON）, references {同じ}・rename {同じ, newName}（S161。[Range]・[{range, newText}] の JSON。rename は断れば例外）, tunable_parameters {model}（S150。ParametersResponse の JSON。変換できなければ例外）, export_fmu {model, fmiVersion}（S144。ソースの FMU の zip の Uint8Array。失敗は例外）,
//   format {text, indent, insertOrigin}（整形後の本文）, lint {text}（JSON の配列）（S053）,
//   version, solvers, memory, panic_for_test {message}
// 失敗は `{id, ok:false, error, panic, poisoned}`。wasm32 は panic=abort なので、WebAssembly.RuntimeError のあとの
// インスタンスは使えない（poisoned: true）。作り直しは client.js 側（worker を捨てて、新しく起こして、再生する）。
const isWeb = typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;
const port = isWeb ? self : (await import("node:worker_threads")).parentPort;
const send = (m, transfer) => port.postMessage(m, transfer);

let gm; // 糊のモジュール
let ws = null; // 現在の Workspace（library か、最初の set_file で作る）
const W = () => (ws ??= new gm.Workspace());

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
    W().set_file(name, text);
  },
  remove_file({ name }) {
    return W().remove_file(name);
  },
  check({ model }) {
    return W().check(model);
  },
  graph({ model, depth }) {
    return W().graph(model, depth);
  },
  outline({ path }) {
    return W().outline(path);
  },
  deps({ model }) {
    return W().deps(model);
  },
  flat({ model }) {
    return W().flat(model);
  },
  simulate({ request }) {
    return W().simulate(request);
  },
  library_tree({ prefix }) {
    return W().library_tree(prefix);
  },
  library_search({ query, limit }) {
    return W().library_search(query, limit);
  },
  library_source({ path }) {
    return W().library_source(path);
  },
  definition({ file, text, line, character }) {
    return W().definition(file, text, line, character); // S138。{file, text, range} の JSON か "null"
  },
  completion({ file, text, line, character }) {
    return W().completion(file, text, line, character); // S171。CompletionItem の配列の JSON
  },
  hover({ file, text, line, character }) {
    return W().hover(file, text, line, character); // S149。{contents, range} の JSON か "null"
  },
  references: ({ file, text, line, character }) => W().references(file, text, line, character), // S161。[Range] の JSON
  folding_ranges: ({ file, text }) => W().folding_ranges(file, text), // S225。[{startLine, endLine, kind}] の JSON（先に set_file）
  models: ({ file }) => W().models(file), // S170。実行できるクラスの修飾名の JSON 配列（先に set_file）
  rename: ({ file, text, line, character, newName }) => W().rename(file, text, line, character, newName), // S161。[{range, newText}] の JSON。断れば投げる
  tunable_parameters: ({ model }) => W().tunable_parameters(model), // S150。ParametersResponse の JSON
  export_fmu: ({ model, fmiVersion }) => W().export_fmu(model, fmiVersion),
  format: ({ text, indent, insertOrigin }) => gm.format(text, indent, insertOrigin), // S053。壊れていれば投げる
  lint: ({ text }) => gm.lint(text),
  // 時系列を列ごとの Float64Array で返す（S051。CSV の文字列を作らず、解析もしない。buffer は転送する）
  // progress が真なら、進捗を {id, type: "progress", stage, time, start_time, stop_time} で親に送る（S133、ADR-0033）
  // cancel（SharedArrayBuffer(4)）があれば、進捗のたびにその Int32 を読み、1 なら true を返して止める（S157。親が cancel() で立てる。
  // 止めると、そこまでの列と GM-I11004 を普通の結果として返す）
  simulate_columns({ request, progress, cancel }, id) {
    const flag = cancel && new Int32Array(cancel);
    const notify = (stage, time, start_time, stop_time) => {
      if (progress) send({ id, type: "progress", stage, time, start_time, stop_time });
      return flag ? Atomics.load(flag, 0) === 1 : undefined;
    };
    const c = W().simulate_columns(request, progress || flag ? notify : undefined);
    try {
      const layout = JSON.parse(c.layout());
      return { response: c.response, layout, columns: layout.names.map((_, i) => c.column(i)) };
    } finally {
      c.free();
    }
  },
  // 結果のファイル（.mat か .csv。S222）を列にする。simulate_columns と同じ {layout, columns}（response は無い）。読めなければ投げる
  read_results({ name, bytes }) {
    const c = gm.read_results(name, bytes);
    try {
      const layout = JSON.parse(c.layout());
      return { layout, columns: layout.names.map((_, i) => c.column(i)) };
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
    send({ id, ok: true, value, mem: gm.memory_bytes() }, value?.columns?.map((c) => c.buffer)); // mem: 線形メモリ（client.js が上限を見て作り直す。ADR-0023 (c)）
  } catch (e) {
    const poisoned = typeof WebAssembly !== "undefined" && e instanceof WebAssembly.RuntimeError;
    // simulate の失敗は文字列の例外のこともある
    send({ id, ok: false, error: String(e?.message ?? e), panic: poisoned ? gm.last_panic() : undefined, poisoned });
  }
}

if (isWeb) port.onmessage = (e) => handle(e.data);
else port.on("message", handle);

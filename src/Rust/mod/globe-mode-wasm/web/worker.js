// globe-mode-wasm を Web Worker（ブラウザの module Worker と Node の worker_threads の両方）で回す（S036）。
//
//   ブラウザ: new Worker("worker.js", { type: "module" })
//   Node    : new (await import("node:worker_threads")).Worker(new URL("worker.js", import.meta.url))
//
// 呼び方は client.js の GlobeWorker が受け持つ。ここは `{id, op, …}` を受けて `{id, ok, value}` を返すだけ。
//   init {pkg, wasm?}  : `${pkg}/globe_mode_wasm.js`（wasm-bindgen --target web の糊）を読んで初期化。
//                        wasm に ArrayBuffer を渡すと、それで初期化（Node は file: を fetch できないため）
//   simulate_many_columns {request, progress?, cancel?}（S322。複数のソルバ。{response, runs: [{layout, columns}]}。進捗に solver が付く）、simulate_columns {request, progress?, cancel?}（{response, layout, columns: Float64Array[]}。buffer は転送。progress で進捗も送る。S133）、read_results {name, bytes}（{layout, columns, parameters}。S222、S236）, library {json, name}, set_file {name, text}, remove_file {name}, check {model}（CheckResponse の JSON）, graph {model, depth}（graphology の JSON）, blt {model}（BltReport の JSON。S262）, stages {model}（StagesReport の JSON。S282）, simulate {request}（JSON 文字列）、
//   library_index {url, name}（S301。分割配信のライブラリ。index.json の URL から索引を読み、見出しだけの Workspace を作る。本文の塊は検査などの前に要るだけ取る。索引が無ければ null、あれば {files, chunks}）,
//   library_tree {prefix}・library_search {query, limit}（S167。名前の部分一致。木と同じ形の JSON）・library_source {path}（S127。JSON。ライブラリが無ければ [] と例外）, definition {file, text, line, character}（S138。{file, text, range} の JSON か null）, hover {同じ}（S149。{contents, range} の JSON か null）, completion {同じ}（S171。LSP の CompletionItem の配列の JSON）, move_part {cls, text, part, dx, dy}（S250。図の部品の移動の [{range, newText}] の JSON か null）, add_connect {cls, text, from, to, points}（S261。図の結線を足す [{range, newText}] の JSON か null）, add_part {cls, text, type, x, y}（S265。図の部品の宣言を足す [{range, newText}] の JSON か null）, references {同じ}・rename {同じ, newName}（S161。[Range]・[{range, newText}] の JSON。rename は断れば例外）, tunable_parameters {model}（S150。ParametersResponse の JSON。変換できなければ例外）, parameters {model}（S248。ModelParametersResponse の JSON。変換できなければ例外）, linearize {model, time?, parameters?}（S260。{model, time, x, u, y, x0, u0, y0, A, B, C, D} の JSON。できなければ例外）, export_fmu {model, fmiVersion}（S144。ソースの FMU の zip の Uint8Array。失敗は例外）,
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

// S301: 分割配信のライブラリ。files は 名前 → 本文（まだ取っていないファイルは見出し）、stub は見出しのままのファイル名、chunk は 名前 → 塊の番号。
// docs は set_file した文書（Workspace を作り直すたびに足し直す）。ok は閉包を確かめ済みの (モデル, 文書) の組
let lazy = null;
const docs = new Map();

// テキストを取る。取れなければ null（Node の file: は fs で読む。ブラウザは CSP の connect-src 'self' の内側の相対 URL だけ）
async function getText(url) {
  if (!isWeb && url.startsWith("file:")) {
    try {
      return await (await import("node:fs/promises")).readFile(new URL(url), "utf8");
    } catch {
      return null;
    }
  }
  const res = await fetch(url);
  return res.ok ? res.text() : null;
}

// S340: 分割配信がパック（索引の pack）なら、パックを Blob で 1 回だけ取り（取れた Blob を持つ。Worker のヒープに載せず、塊は slice で読む）、i 番の塊の本文を返す
async function packText(i) {
  const { url, at } = lazy.pack;
  lazy.blob ??= getBlob(lazy.base + url).catch((e) => ((lazy.blob = null), Promise.reject(e)));
  return (await lazy.blob).slice(at[i], at[i + 1]).text();
}
async function getBlob(url) {
  if (!isWeb && url.startsWith("file:")) return (await import("node:fs")).openAsBlob(new URL(url)); // Node: ファイルを読まずに Blob にする
  const res = await fetch(url);
  if (!res.ok) throw new Error(`MSL のパックを取得できない（${url}）: ${res.status}`);
  return res.blob();
}

function rebuild() {
  const next = new gm.Workspace(JSON.stringify(lazy.files), lazy.name);
  for (const [name, text] of docs) next.set_file(name, text);
  ws?.free();
  ws = next;
}

// `model` と文書（と extra の本文）が使うファイルがすべて本文になるまで、塊を取っては閉包を取り直す（本文だけが参照を持つので、
// 取るたびに閉包が広がる）。全部読んだときと同じ閉包に落ち着く。増えたら Workspace を作り直す（読んだ塊は外さない）
async function ensure(model, extra = [], basesOnly = false) {
  if (!lazy) return;
  const key = JSON.stringify([model, extra, basesOnly, [...docs]]);
  if (lazy.ok.has(key)) return;
  let grown = false;
  for (let round = 0; ; round++) {
    if (round > lazy.chunks + 1) throw new Error(`塊を取っても閉包が決まらない（${round} 回）。索引と塊の版が合っていない`); // 1 回ごとに新しい塊を取るので塊の数で終わる
    const closure = JSON.parse(lazy.closure.closure(model, JSON.stringify([...docs.values(), ...extra]), basesOnly));
    const need = closure.filter((n) => lazy.stub.has(n));
    if (!need.length) break;
    await fetchChunks(need, basesOnly ? 0 : lazy.prefetch); // 基底だけの閉包（木のアイコン）は deps（全部の参照）をたどると取りすぎる
    grown = true;
  }
  if (grown) rebuild();
  if (lazy.ok.size >= 32) lazy.ok.clear();
  lazy.ok.add(key);
}

// 名前のファイルの塊と、その塊から索引の deps を depth 段たどった塊（先読み。S308）をまとめて並行に取る。
// 本文だけが参照を持つので閉包は取るたびに広がる。先読みは、その往復を減らす目安（足りなければ呼び手が繰り返す。取りすぎは起きうる）
async function fetchChunks(names, depth = 0) {
  const seen = new Set(names.map((n) => lazy.chunk[n]));
  for (let frontier = [...seen]; depth-- > 0 && frontier.length; ) {
    frontier = frontier.flatMap((i) => lazy.deps[i] ?? []).filter((d) => !seen.has(d) && !lazy.got.has(d));
    frontier = [...new Set(frontier)];
    for (const d of frontier) seen.add(d);
  }
  const ids = [...seen].filter((i) => !lazy.got.has(i));
  if (!ids.length) return;
  lazy.rounds++;
  // 取れて、読めてから got に入れる（壊れた塊を入れると、次の呼び出しが何も取らず ensure が回り続けた。#110）
  const parsed = await Promise.all(
    ids.map(async (i) => {
      const url = lazy.pack ? `${lazy.base}${lazy.pack.url}#${i}` : `${lazy.base}c/${i}.json`;
      const text = (await (lazy.pack ? packText(i) : getText(url))) ?? Promise.reject(new Error(`MSL の塊 ${i} を取得できない（${url}）`));
      try {
        return [text, JSON.parse(text)];
      } catch (e) {
        throw new Error(`MSL の塊 ${i} が壊れている（${url}）: ${e.message}`);
      }
    }),
  );
  for (const [text, chunk] of parsed) {
    lazy.bytes += text.length;
    for (const [n, body] of Object.entries(chunk)) {
      if (!lazy.stub.delete(n)) continue;
      lazy.files[n] = body;
      lazy.closure.set_text(n, body);
    }
  }
  // 索引が「この塊にある」と言うファイルが見出しのままなら、塊が索引と合っていない。その塊は got に入れない（次に取り直せる）
  const fetched = new Set(ids);
  const missing = Object.keys(lazy.chunk).filter((n) => fetched.has(lazy.chunk[n]) && lazy.stub.has(n));
  const badIds = new Set(missing.map((n) => lazy.chunk[n]));
  for (const i of ids) if (!badIds.has(i)) lazy.got.add(i);
  if (missing.length) throw new Error(`MSL の塊に無いファイル: ${missing.slice(0, 3).join(", ")}（索引と塊の版が合っていない）`);
}

const ops = {
  async init({ pkg, wasm }) {
    gm = await import(`${pkg}/globe_mode_wasm.js`);
    await gm.default({ module_or_path: wasm ?? `${pkg}/globe_mode_wasm_bg.wasm` });
  },
  library({ json, name }) {
    const next = new gm.Workspace(json, name); // 読めなければ投げ、今の ws は残す
    ws?.free();
    ws = next;
    lazy?.closure.free();
    lazy = null;
    docs.clear();
  },
  async library_index({ url, name }) {
    const indexText = await getText(url);
    if (indexText === null) return null;
    let idx;
    try {
      idx = JSON.parse(indexText);
    } catch {
      return null; // SPA の fallback が index.html を返す配信でも、索引が無いのと同じに扱う
    }
    const files = { ...idx.real, ...idx.stubs };
    const next = new gm.Workspace(JSON.stringify(files), name); // 読めなければ投げ、今の ws は残す
    ws?.free();
    ws = next;
    docs.clear();
    lazy?.closure.free();
    lazy = { name, base: url.slice(0, url.lastIndexOf("/") + 1), files, stub: new Set(Object.keys(idx.stubs)), chunk: idx.chunk, got: new Set(), rounds: 0, pack: idx.pack, blob: null, deps: idx.deps ?? [], prefetch: idx.prefetch ?? 0, ok: new Set(), closure: new gm.LibraryClosure(JSON.stringify(files)), bytes: indexText.length, chunks: idx.chunks };
    return { files: Object.keys(files).length, chunks: idx.chunks, bytes: indexText.length };
  },
  // 分割配信で、ここまでに取った塊の数と文字数（索引を含む）。分割配信でなければ null
  library_stats: () => lazy && { chunks: lazy.got.size, of: lazy.chunks, bytes: lazy.bytes, stubs: lazy.stub.size, rounds: lazy.rounds },
  set_file({ name, text }) {
    W().set_file(name, text);
    docs.set(name, text);
  },
  remove_file({ name }) {
    docs.delete(name);
    return W().remove_file(name);
  },
  check({ model }) {
    return W().check(model);
  },
  blt({ model }) {
    return W().blt(model);
  },
  stages({ model }) {
    return W().stages(model);
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
  async library_source({ path }) {
    // 木から開くファイルは、閉包でなくそのファイルだけ本文にする（見出しのままだと中身が出ない）
    const file = lazy && JSON.parse(W().library_source(path)).file;
    if (file && lazy.stub.has(file)) {
      await fetchChunks([file]);
      rebuild();
    }
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
  documentation: ({ cls, text, line, character }) => W().documentation(cls, text, line, character), // S232。{class, info, revisions} の JSON か null（生の HTML。洗うのは UI）
  icon: ({ cls, text, line, character }) => W().icon(cls, text, line, character), // S244。{class, extent, shapes} の JSON か null（形のデータ。描くのは UI）
  diagram: ({ cls, text, line, character }) => W().diagram(cls, text, line, character), // S245。{class, extent, parts, connections, shapes} の JSON か null
  models: ({ file }) => W().models(file), // S170。実行できるクラスの修飾名の JSON 配列（先に set_file）
  add_connect: ({ cls, text, from, to, points }) => W().add_connect(cls, text, from, to, JSON.stringify(points ?? [])), // S261。[{range, newText}] の JSON か null
  add_part: ({ cls, text, type, x, y }) => W().add_part(cls, text, type, x, y), // S265。[{range, newText}] の JSON か null
  set_modifier: ({ cls, text, modifiers }) => W().set_modifier(cls, text, modifiers), // S294。[{range, newText}] の JSON。断れば投げる
  delete_part: ({ cls, text, part }) => W().delete_part(cls, text, part), // S281。[{range, newText}] の JSON か null
  delete_connect: ({ cls, text, from, to }) => W().delete_connect(cls, text, from, to), // S281。同上
  move_waypoint: ({ cls, text, from, to, index, dx, dy }) => W().move_waypoint(cls, text, from, to, index, dx, dy), // S299。結線の途中の点を動かす。同上
  add_waypoint: ({ cls, text, from, to, index, x, y, ends }) => W().add_waypoint(cls, text, from, to, index, x, y, JSON.stringify(ends ?? [])), // S299。同上
  move_part: ({ cls, text, part, dx, dy }) => W().move_part(cls, text, part, dx, dy), // S250。[{range, newText}] の JSON か null
  rename: ({ file, text, line, character, newName }) => W().rename(file, text, line, character, newName), // S161。[{range, newText}] の JSON。断れば投げる
  tunable_parameters: ({ model }) => W().tunable_parameters(model), // S150。ParametersResponse の JSON
  parameters: ({ model }) => W().parameters(model), // S248。ModelParametersResponse の JSON
  linearize: ({ model, time, parameters }) => W().linearize(model, time ?? undefined, JSON.stringify(parameters ?? {})), // S260。線形化の JSON
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
  // S322: 複数のソルバで回す。進捗は {id, type: "progress", stage, time, start_time, stop_time, solver}。cancel は simulate_columns と同じ
  // （全部の実行が止まる）。{response, runs: [{layout, columns}]}
  simulate_many_columns({ request, progress, cancel }, id) {
    const flag = cancel && new Int32Array(cancel);
    const notify = (stage, time, start_time, stop_time, solver) => {
      if (progress) send({ id, type: "progress", stage, time, start_time, stop_time, solver });
      return flag ? Atomics.load(flag, 0) === 1 : undefined;
    };
    const c = W().simulate_many_columns(request, progress || flag ? notify : undefined);
    try {
      const response = c.response;
      const runs = JSON.parse(response).runs.map((_, run) => {
        const layout = JSON.parse(c.layout(run));
        return { layout, columns: layout.names.map((_, i) => c.column(run, i)) };
      });
      return { response, runs };
    } finally {
      c.free();
    }
  },
  // 結果のファイル（.mat か .csv。S222）を列にする。simulate_columns と同じ {layout, columns}（response は無い）。読めなければ投げる
  read_results({ name, bytes }) {
    const c = gm.read_results(name, bytes);
    try {
      const layout = JSON.parse(c.layout());
      return { layout, columns: layout.names.map((_, i) => c.column(i)), parameters: JSON.parse(c.parameters()) };
    } finally {
      c.free();
    }
  },
  version: () => gm.version(),
  solvers: () => gm.solvers(),
  memory: () => gm.memory_bytes(),
  panic_for_test: ({ message }) => gm.panic_for_test(message),
};

// S301: 分割配信のとき、モデルと文書が使うファイルを先に本文にしてから呼ぶ操作（[モデル, 追加の本文] を返す）
const needs = {
  check: ({ model }) => [model],
  blt: ({ model }) => [model],
  stages: ({ model }) => [model],
  graph: ({ model }) => [model],
  flat: ({ model }) => [model],
  tunable_parameters: ({ model }) => [model],
  parameters: ({ model }) => [model],
  linearize: ({ model }) => [model],
  export_fmu: ({ model }) => [model],
  simulate: ({ request }) => [JSON.parse(request).model],
  simulate_columns: ({ request }) => [JSON.parse(request).model],
  simulate_many_columns: ({ request }) => [JSON.parse(request).model],
  definition: ({ text }) => ["", [text]],
  hover: ({ text }) => ["", [text]],
  completion: ({ text }) => ["", [text]],
  // クラス 1 つの説明とアイコンは `extends` の基底だけ（木の行ごとに icon を引くので、パッケージの参照の閉包を全部は取らない）。図は部品の型も要るので閉包
  documentation: ({ cls, text }) => [cls, [text], !!cls],
  icon: ({ cls, text }) => [cls, [text], !!cls],
  diagram: ({ cls, text }) => [cls, [text]],
  move_part: ({ text }) => ["", [text]],
  add_connect: ({ text }) => ["", [text]],
  add_part: ({ type, text }) => [type, [text]],
};
for (const [op, need] of Object.entries(needs)) {
  const f = ops[op];
  ops[op] = async (args, id) => {
    if (lazy) await ensure(...need(args));
    return f(args, id);
  };
}

async function handle({ id, op, ...args }) {
  try {
    if (!ops[op]) throw new Error(`未知の op: ${op}`);
    const value = await ops[op](args, id);
    send({ id, ok: true, value, mem: gm.memory_bytes() }, (value?.columns ?? value?.runs?.flatMap((r) => r.columns))?.map((c) => c.buffer)); // mem: 線形メモリ（client.js が上限を見て作り直す。ADR-0023 (c)）
  } catch (e) {
    const poisoned = typeof WebAssembly !== "undefined" && e instanceof WebAssembly.RuntimeError;
    // simulate の失敗は文字列の例外のこともある
    send({ id, ok: false, error: String(e?.message ?? e), panic: poisoned ? gm.last_panic() : undefined, poisoned });
  }
}

if (isWeb) port.onmessage = (e) => handle(e.data);
else port.on("message", handle);

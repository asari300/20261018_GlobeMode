// worker.js を包む（S036）。ブラウザでも Node でも動く ES module。
//
//   const w = new GlobeWorker(() => new Worker(new URL("worker.js", import.meta.url), { type: "module" }),
//                             { pkg: "/pkg" });                       // ブラウザ。pkg は --target web の出力の URL
//   const w = new GlobeWorker(() => new (wt.Worker)(workerUrl), { pkg: pathToFileHref, wasm: bytes }); // Node
//   await w.library(json, "MSL"); await w.setFile("a.mo", text);
//   const r = await w.simulate({ model: "A", settings: { solver: "bdf" } });
//
// panic（wasm32 は abort）や worker の異常終了のとき、その呼び出しを panic の文言つきの Error で reject し、
// worker を捨てて作り直し、ライブラリとファイルを再生する。次の呼び出しは再生のあとに始まる。restarts が回数。
const GRACE_MS = 2000;
// wasm32 の上限 4 GiB の手前。線形メモリは縮まないので、超えたら次の要求の前に worker を作り直す（ADR-0023 (c)）
const MEMORY_LIMIT = 2 * 2 ** 30;

const cancelledError = () => Object.assign(new Error("止めた"), { cancelled: true });

export class GlobeWorker {
  restarts = 0;
  #make; #init; #w; #pending = new Map(); #id = 0;
  #lib = null; #files = new Map(); // 再生用の記憶
  #ready; #tail = Promise.resolve();
  #alive = false; // init が終わった worker か（終わる前に落ちたら作り直さない）
  #flag; // 中断のフラグ（Int32Array(SharedArrayBuffer)）。使えなければ null（S157）
  #memoryLimit; #mem = 0; // 直近の応答の線形メモリ（バイト）。#call が上限と比べる
  #graceMs; #running = 0; #grace; // 猶予（ms）、実行中の simulate_columns の id と、cancel() のあとの猶予のタイマー
  #queued = 0; #cancelQueued = false; // 順番待ちか実行中の simulate_columns の数と、待っているものを止める印（S201）

  // shared: SharedArrayBuffer を使うか。既定は使えるか（ブラウザは cross-origin isolated のときだけ SharedArrayBuffer がある）
  // graceMs: cancel() のあと Worker が応えるまで待つ時間（試験が短くする）
  constructor(make, init, { shared = typeof SharedArrayBuffer === "function", graceMs = GRACE_MS, memoryLimit = MEMORY_LIMIT } = {}) {
    this.#graceMs = graceMs;
    this.#memoryLimit = memoryLimit;
    this.#make = make;
    this.#init = init;
    this.#flag = shared ? new Int32Array(new SharedArrayBuffer(4)) : null;
    this.#ready = this.#start();
  }

  // Node の Worker は .on、ブラウザは addEventListener
  #start() {
    const w = (this.#w = this.#make());
    const fail = (e) => this.#die(w, new Error(`worker が異常終了: ${e?.message ?? e}`));
    if (w.on) {
      w.on("message", (m) => w === this.#w && this.#reply(m)); // 捨てた worker の遅れた応答は読まない
      w.on("error", fail);
      w.on("exit", (c) => fail(`exit ${c}`));
    } else {
      w.addEventListener("message", (e) => w === this.#w && this.#reply(e.data));
      w.addEventListener("error", (e) => fail(e.error ?? e.message));
    }
    return this.#raw("init", this.#init).then(async () => {
      this.#alive = true;
      if (this.#lib) await this.#raw("library", this.#lib);
      for (const [name, text] of this.#files) await this.#raw("set_file", { name, text });
      this.#mem = 0; // 作り直した worker の分から数える（再生で上限を越えても、続けて作り直さない）
    });
  }

  #raw(op, args = {}, onProgress) {
    return new Promise((resolve, reject) => {
      const id = ++this.#id;
      this.#pending.set(id, { resolve, reject, onProgress });
      if (op === "simulate_columns") {
        this.#running = id;
        if (this.#flag) {
          Atomics.store(this.#flag, 0, 0);
          args = { ...args, cancel: this.#flag.buffer };
        }
      }
      this.#w.postMessage({ id, op, ...args });
    });
  }

  #reply({ id, type, ok, value, error, panic, poisoned, mem, ...progress }) {
    const p = this.#pending.get(id);
    if (type === "progress") return p?.onProgress?.(progress);
    this.#pending.delete(id);
    if (id === this.#running) { this.#running = 0; clearTimeout(this.#grace); }
    if (ok) { if (mem !== undefined) this.#mem = mem; return p?.resolve(value); }
    p?.reject(new Error(panic ? `${error}\n${panic}` : error));
    if (poisoned) this.#die(this.#w, null);
  }

  // 現在の worker を捨てて作り直す。w が古い（既に作り直した）ときは何もしない
  // 起動（init）が終わる前に落ちたときは作り直さない（worker.js や wasm が読めないと無限に作り直して要求を出し続けるため。
  // S041 で 8 秒に 914 回の GET を確認）。以後の呼び出しはその誤りで reject する
  #die(w, err) {
    if (w !== this.#w) return;
    this.#w = null;
    this.#running = 0;
    clearTimeout(this.#grace);
    err ??= new Error("worker を作り直した");
    for (const p of this.#pending.values()) p.reject(err);
    this.#pending.clear();
    w.terminate?.();
    if (!this.#alive) {
      this.#ready = Promise.reject(err);
      this.#ready.catch(() => {});
      return;
    }
    this.#alive = false;
    this.restarts++;
    this.#ready = this.#start();
    this.#ready.catch(() => {}); // 失敗は次の呼び出しが受ける
  }

  // 呼び出しは直列に（再生の前に次の呼び出しが割り込まない）
  #call(op, args, onProgress) {
    const sim = op === "simulate_columns";
    const r = this.#tail.then(() => this.#ready).then(async () => {
      if (this.#mem > this.#memoryLimit) { this.#die(this.#w, null); await this.#ready; } // 前の応答で上限を越えていた。ライブラリとファイルは再生する
      if (sim && this.#cancelQueued) { this.#cancelQueued = false; throw cancelledError(); } // 待つ間に止められた
      return this.#raw(op, args, onProgress);
    });
    if (sim) {
      this.#queued++;
      const done = () => { if (!--this.#queued) this.#cancelQueued = false; };
      r.then(done, done);
    }
    this.#tail = r.catch(() => {});
    return r;
  }

  async library(json, name) {
    await this.#call("library", { json, name });
    this.#lib = { json, name };
    this.#files.clear(); // 新しい Workspace になるので、ファイルの記憶も捨てる
  }
  async setFile(name, text) { await this.#call("set_file", { name, text }); this.#files.set(name, text); }
  async removeFile(name) { const r = await this.#call("remove_file", { name }); this.#files.delete(name); return r; }
  async check(model) { return JSON.parse(await this.#call("check", { model })); }
  async graph(model, depth) { return JSON.parse(await this.#call("graph", { model, depth })); }
  // 読み取り（S191。CLI の `globe outline` / `deps` / `flat --json` と同じ形）: ファイルのアウトライン（無い・構文の誤りは reject）、check が読むファイルの閉包、flat の要約（通らなければ null）
  async outline(path) { return JSON.parse(await this.#call("outline", { path })); }
  async deps(model) { return JSON.parse(await this.#call("deps", { model })); }
  async flat(model) { return JSON.parse(await this.#call("flat", { model })); }
  async simulate(request) { return JSON.parse(await this.#call("simulate", { request: JSON.stringify(request) })); }
  // 上書きできるパラメータの一覧 {model, parameters: [{name, default, unit?, min?, max?, description?}] | null, diagnostics}（S150）。変換できなければ reject
  async tunableParameters(model) { return JSON.parse(await this.#call("tunable_parameters", { model })); }
  // ソースの FMU（FMI 2 か 3、build 無し）の zip を Uint8Array で（S144）。作れなければ reject
  exportFmu(model, fmiVersion = 2) { return this.#call("export_fmu", { model, fmiVersion }); }
  format(text, indent = 2, insertOrigin = true) { return this.#call("format", { text, indent, insertOrigin }); }
  async lint(text) { return JSON.parse(await this.#call("lint", { text })); }
  // ライブラリの木（S127）: prefix の直下の子 [{name, path, kind, description, file, hasChildren}] と、クラスのファイル {file, text}
  async libraryTree(prefix) { return JSON.parse(await this.#call("library_tree", { prefix })); }
  async librarySearch(query, limit = 50) { return JSON.parse(await this.#call("library_search", { query, limit })); } // S167
  async librarySource(path) { return JSON.parse(await this.#call("library_source", { path })); }
  // 定義ジャンプ（S138）: setFile 済みの file の行・列（0 始まり、列は UTF-16）の定義 {file, text, range}。無ければ null
  async definition(file, line, character) {
    return JSON.parse(await this.#call("definition", { file, text: this.#files.get(file) ?? "", line, character }));
  }
  // ホバー（S149）: definition と同じ引数で、宣言の 1 行 {contents, range}（range はカーソルの名前）。無ければ null
  async hover(file, line, character) {
    return JSON.parse(await this.#call("hover", { file, text: this.#files.get(file) ?? "", line, character }));
  }
  // 補完（S171）: definition と同じ引数で、LSP の CompletionItem の配列（最大 200 件。label・kind・detail・documentation）
  async completion(file, line, character) {
    return JSON.parse(await this.#call("completion", { file, text: this.#files.get(file) ?? "", line, character }));
  }
  // 参照（S161）: definition と同じ引数で、同じ名前の範囲 [LSP の Range]。同じ文書の、カーソルのあるいちばん内側のクラスの中だけ
  async references(file, line, character) {
    return JSON.parse(await this.#call("references", { file, text: this.#files.get(file) ?? "", line, character }));
  }
  // モデル名の候補（S170）: setFile 済みの file の実行できるクラス（model・block・class。partial は除く）の修飾名。構文が壊れていれば []
  async models(file) {
    return JSON.parse(await this.#call("models", { file }));
  }
  // 折りたたみ（S225）: setFile 済みの file の [{startLine, endLine, kind}]（行は 0 始まり、endLine を含む。LSP の foldingRange と同じ）
  async foldingRanges(file) {
    return JSON.parse(await this.#call("folding_ranges", { file, text: this.#files.get(file) ?? "" }));
  }
  // 説明（S232）: cls（完全修飾名）の Documentation の生の HTML {class, info, revisions}。cls が空なら at = {file, line, character}（setFile 済み）にあるいちばん内側のクラス。引けなければ null
  async documentation(cls, at) {
    return JSON.parse(await this.#call("documentation", { cls, text: at ? (this.#files.get(at.file) ?? "") : "", line: at?.line ?? 0, character: at?.character ?? 0 }));
  }
  // リネーム（S161）: [{range, newText}]。部品でない・識別子でない・予約語・既にある名前は、理由つきで reject
  async rename(file, line, character, newName) {
    return JSON.parse(await this.#call("rename", { file, text: this.#files.get(file) ?? "", line, character, newName }));
  }
  // 時系列を CSV でなく列ごとの Float64Array で返す（S051）。{response（csv 無し）, layout: {names, points}, columns}
  // onProgress を渡すと進捗 {stage, time, start_time, stop_time} を受ける（S133、ADR-0033。time は積分の t で無ければ undefined）
  async simulateColumns(request, { onProgress } = {}) {
    const r = await this.#call("simulate_columns", { request: JSON.stringify(request), progress: !!onProgress }, onProgress);
    return { ...r, response: JSON.parse(r.response) };
  }
  // 結果のファイル（.mat か .csv の Uint8Array）を列にする（S222）: {layout, columns, parameters}（simulateColumns と同じ。response は無い。parameters は MAT のパラメータで CSV は []。S236）。読めなければ reject
  readResults(name, bytes) { return this.#call("read_results", { name, bytes }); }
  version() { return this.#call("version"); }
  async solvers() { return JSON.parse(await this.#call("solvers")); }
  memory() { return this.#call("memory"); }
  panicForTest(message) { return this.#call("panic_for_test", { message }); }
  // 実行中の呼び出しを止める（S133、ADR-0033）。Worker は計算の間メッセージを受けないので、
  // 実行中の simulateColumns で SharedArrayBuffer が使えるなら、フラグを立てて待つ（S157）: Worker が次の進捗でフラグを読んで止め、
  // そこまでの列と GM-I11004 を普通の結果として返す。lowering の中などで 2 s（GRACE）たっても応えなければ、
  // 使えないとき（cross-origin isolated でない配信）と同じく terminate() して作り直し、ライブラリとファイルを再生する。
  // 作り直したときは、止めた呼び出しを「止めた」の Error（cancelled: true）で reject する（途中の列は返らない）
  // 止めるのは実行（simulate_columns）だけ。起動中や、実行が始まる前の呼び出し（hover・lint など）は殺さない（S201）。
  // 実行がキューで待っているときは、順番が来たところで cancelled で reject する
  cancel() {
    const w = this.#w;
    if (!w) return;
    if (!this.#running) { if (this.#queued) this.#cancelQueued = true; return; }
    const kill = () => this.#die(w, cancelledError());
    if (!this.#flag) return kill();
    Atomics.store(this.#flag, 0, 1);
    clearTimeout(this.#grace);
    this.#grace = setTimeout(kill, this.#graceMs);
  }
  // 待っている呼び出しは reject し、以後の呼び出しも reject する（S201）
  async terminate() {
    const w = this.#w;
    this.#w = null;
    this.#running = 0;
    clearTimeout(this.#grace);
    const err = new Error("terminate された");
    for (const p of this.#pending.values()) p.reject(err);
    this.#pending.clear();
    this.#ready = Promise.reject(err);
    this.#ready.catch(() => {});
    await w?.terminate();
  }
}

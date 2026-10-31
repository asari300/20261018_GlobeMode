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
export class GlobeWorker {
  restarts = 0;
  #make; #init; #w; #pending = new Map(); #id = 0;
  #lib = null; #files = new Map(); // 再生用の記憶
  #ready; #tail = Promise.resolve();
  #alive = false; // init が終わった worker か（終わる前に落ちたら作り直さない）

  constructor(make, init) {
    this.#make = make;
    this.#init = init;
    this.#ready = this.#start();
  }

  // Node の Worker は .on、ブラウザは addEventListener
  #start() {
    const w = (this.#w = this.#make());
    const fail = (e) => this.#die(w, new Error(`worker が異常終了: ${e?.message ?? e}`));
    if (w.on) {
      w.on("message", (m) => this.#reply(m));
      w.on("error", fail);
      w.on("exit", (c) => fail(`exit ${c}`));
    } else {
      w.addEventListener("message", (e) => this.#reply(e.data));
      w.addEventListener("error", (e) => fail(e.error ?? e.message));
    }
    return this.#raw("init", this.#init).then(async () => {
      this.#alive = true;
      if (this.#lib) await this.#raw("library", this.#lib);
      for (const [name, text] of this.#files) await this.#raw("set_file", { name, text });
    });
  }

  #raw(op, args = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.#id;
      this.#pending.set(id, { resolve, reject });
      this.#w.postMessage({ id, op, ...args });
    });
  }

  #reply({ id, ok, value, error, panic, poisoned }) {
    const p = this.#pending.get(id);
    this.#pending.delete(id);
    if (ok) return p?.resolve(value);
    p?.reject(new Error(panic ? `${error}\n${panic}` : error));
    if (poisoned) this.#die(this.#w, null);
  }

  // 現在の worker を捨てて作り直す。w が古い（既に作り直した）ときは何もしない
  // 起動（init）が終わる前に落ちたときは作り直さない（worker.js や wasm が読めないと無限に作り直して要求を出し続けるため。
  // S041 で 8 秒に 914 回の GET を確認）。以後の呼び出しはその誤りで reject する
  #die(w, err) {
    if (w !== this.#w) return;
    this.#w = null;
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
  #call(op, args) {
    const r = this.#tail.then(() => this.#ready).then(() => this.#raw(op, args));
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
  async simulate(request) { return JSON.parse(await this.#call("simulate", { request: JSON.stringify(request) })); }
  version() { return this.#call("version"); }
  async solvers() { return JSON.parse(await this.#call("solvers")); }
  memory() { return this.#call("memory"); }
  panicForTest(message) { return this.#call("panic_for_test", { message }); }
  async terminate() { const w = this.#w; this.#w = null; await w?.terminate(); }
}

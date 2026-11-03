import { GlobeWorker } from "../../../Rust/mod/globe-mode-wasm/web/client.js";
import type { Backend, Definition, LintMessage, Progress, TreeEntry } from "./backend";
import { browserOpen, browserSave, browserSaveBinary } from "./browser-file";
import { gapNonFinite, type Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse, SolverEntry } from "./protocol";

/** Web Worker 上の wasm（ADR-0014、ADR-0023）。呼び出しは直列で、panic のあとは worker を作り直す。 */
export class WorkerBackend implements Backend {
  #w = new GlobeWorker(
    () => new Worker(new URL(`${import.meta.env.BASE_URL}wasm/worker.js`, location.href), { type: "module" }),
    { pkg: new URL(`${import.meta.env.BASE_URL}wasm/pkg`, location.href).href },
  );

  library(name: string, json: string): Promise<void> {
    return this.#w.library(json, name);
  }
  setFile(name: string, text: string): Promise<void> {
    return this.#w.setFile(name, text);
  }
  check(model: string): Promise<CheckResponse> {
    return this.#w.check(model);
  }
  graph(model: string, depth: number): Promise<GraphJson> {
    return this.#w.graph(model, depth);
  }
  simulate(req: SimulateRequest): Promise<SimulateResponse> {
    return this.#w.simulate(req);
  }
  async simulateColumns(req: SimulateRequest, onProgress?: (p: Progress) => void): Promise<{ response: SimulateResponse; columns: Columns }> {
    const r = await this.#w.simulateColumns(req, { onProgress }); // layout.names は time を含む
    return { response: r.response, columns: { names: r.layout.names.slice(1), points: r.layout.points, data: r.columns.map(gapNonFinite) } };
  }
  cancel(): void {
    this.#w.cancel();
  }
  solvers(): Promise<SolverEntry[]> {
    return this.#w.solvers();
  }
  format(text: string, indent = 2, insertOrigin = true): Promise<string> {
    return this.#w.format(text, indent, insertOrigin); // wasm に設定ファイルは無い（S143）
  }
  lint(text: string): Promise<LintMessage[]> {
    return this.#w.lint(text);
  }
  libraryTree(prefix: string): Promise<TreeEntry[]> {
    return this.#w.libraryTree(prefix);
  }
  librarySource(path: string): Promise<{ file: string; text: string }> {
    return this.#w.librarySource(path);
  }
  definition(file: string, line: number, character: number): Promise<Definition | null> {
    return this.#w.definition(file, line, character);
  }
  async exportFmu(model: string, fmiVersion: number): Promise<string | null> {
    return browserSave(`${model.replaceAll(".", "_")}.fmu`, await this.#w.exportFmu(model, fmiVersion)); // S144。名前は facade の FMU と同じ
  }
  openFile = browserOpen;
  saveFile = browserSave;
  saveBinary = browserSaveBinary;
}

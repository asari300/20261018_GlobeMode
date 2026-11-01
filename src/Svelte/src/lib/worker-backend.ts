import { GlobeWorker } from "../../../Rust/mod/globe-mode-wasm/web/client.js";
import type { Backend, LintMessage } from "./backend";
import { gapNonFinite, type Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse } from "./protocol";

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
  async simulateColumns(req: SimulateRequest): Promise<{ response: SimulateResponse; columns: Columns }> {
    const r = await this.#w.simulateColumns(req); // layout.names は time を含む
    return { response: r.response, columns: { names: r.layout.names.slice(1), points: r.layout.points, data: r.columns.map(gapNonFinite) } };
  }
  format(text: string, indent: number, insertOrigin: boolean): Promise<string> {
    return this.#w.format(text, indent, insertOrigin);
  }
  lint(text: string): Promise<LintMessage[]> {
    return this.#w.lint(text);
  }
}

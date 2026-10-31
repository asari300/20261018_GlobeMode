import { GlobeWorker } from "../../../Rust/mod/globe-mode-wasm/web/client.js";
import type { Backend } from "./backend";
import type { CheckResponse, SimulateRequest, SimulateResponse } from "./protocol";

/** Web Worker 上の wasm（ADR-0014、ADR-0023）。呼び出しは直列で、panic のあとは worker を作り直す。 */
export class WorkerBackend implements Backend {
  #w = new GlobeWorker(
    () => new Worker(new URL(`${import.meta.env.BASE_URL}wasm/worker.js`, location.href), { type: "module" }),
    { pkg: new URL(`${import.meta.env.BASE_URL}wasm/pkg`, location.href).href },
  );

  setFile(name: string, text: string): Promise<void> {
    return this.#w.setFile(name, text);
  }
  check(model: string): Promise<CheckResponse> {
    return this.#w.check(model);
  }
  simulate(req: SimulateRequest): Promise<SimulateResponse> {
    return this.#w.simulate(req);
  }
}

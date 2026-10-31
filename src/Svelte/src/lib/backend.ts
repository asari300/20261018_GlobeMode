import type { CheckResponse, SimulateRequest, SimulateResponse } from "./protocol";

/** UI が計算を頼む相手。WorkerBackend（ブラウザの wasm）と、あとで TauriBackend（sidecar）が実装する。 */
export interface Backend {
  setFile(name: string, text: string): Promise<void>;
  check(model: string): Promise<CheckResponse>;
  simulate(req: SimulateRequest): Promise<SimulateResponse>;
}

// Graph.svelte を単体で mount する試験用の入口（tests/test_graph.py が使う）。App.svelte には組み込まれていない。
import { mount, unmount } from "svelte";
import Graph from "./lib/graph/Graph.svelte";
import { WorkerBackend } from "./lib/worker-backend";
import type { Backend } from "./lib/backend";
import type { GraphJson } from "./lib/protocol";

/** n ノード・約 2n 辺の合成グラフを返す Backend（描画の fps を測る）。 */
function synthetic(n: number): Backend {
  const nodes = Array.from({ length: n }, (_, i) => ({
    key: `n${i}`,
    attributes: { label: `n${i}`, class: "X", kind: (i % 3 ? "component" : "connector") as "component" | "connector", level: 1 },
  }));
  const edges: GraphJson["edges"] = [];
  for (let i = 0; i < 2 * n; i++) {
    const source = `n${i % n}`;
    const target = `n${(i * 7 + 1 + (i % 13)) % n}`;
    if (source !== target) edges.push({ key: `e${i}`, source, target, attributes: { lhs: "", rhs: "", domain: "flow", connector: "" } });
  }
  const json: GraphJson = {
    attributes: { model: "Synthetic", depth: 1, unresolved: [] },
    options: { type: "undirected", multi: true, allowSelfLoops: false },
    nodes,
    edges,
  };
  return { graph: async () => json } as unknown as Backend;
}

const w = window as unknown as Record<string, unknown>;
w.backend = new WorkerBackend();
w.synthetic = synthetic;
w.mountGraph = (backend: Backend, model: string, onSelect: (id: string) => void) => {
  if (w.current) void unmount(w.current as ReturnType<typeof mount>);
  w.current = mount(Graph, { target: document.getElementById("app")!, props: { backend, model, onSelect } });
};

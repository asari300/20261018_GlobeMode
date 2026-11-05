<script lang="ts">
  import Graph from "graphology";
  import forceAtlas2 from "graphology-layout-forceatlas2";
  import Sigma, { DEFAULT_STYLES } from "sigma";
  import type { Backend } from "../backend";
  import { cssVar, theme } from "../../theme.svelte";

  let { backend, model, onSelect }: { backend: Backend; model: string; onSelect: (nodeId: string) => void } = $props();

  let host: HTMLDivElement;
  let depth = $state(1);
  let failure = $state("");
  let counts = $state({ nodes: 0, edges: 0 });
  let sigma: Sigma | undefined;

  /** 試験用: 描いている sigma のインスタンス。 */
  export function getSigma() {
    return sigma;
  }

  $effect(() => {
    const m = model;
    const d = depth;
    void theme.dark; // 明暗が変わったら色を取り直して作り直す（S152）
    let cancelled = false;
    let s: Sigma | undefined;
    failure = "";
    backend
      .graph(m, d)
      .then((json) => {
        if (cancelled) return;
        const g = Graph.from(json as never);
        const n = g.order;
        // 円に置いてから ForceAtlas2 を固定の反復で同期に回す（worker の layout は入れない）
        let i = 0;
        g.forEachNode((key, a) => {
          g.mergeNodeAttributes(key, {
            x: Math.cos((2 * Math.PI * i) / n),
            y: Math.sin((2 * Math.PI * i++) / n),
            size: a.kind === "connector" ? 5 : 9,
            color: a.kind === "connector" ? cssVar("--warn") : cssVar("--accent"),
          });
        });
        g.forEachEdge((key) => g.setEdgeAttribute(key, "color", cssVar("--border")));
        if (n > 1) forceAtlas2.assign(g, { iterations: 100, settings: forceAtlas2.inferSettings(g) });
        counts = { nodes: n, edges: g.size };
        // 動かしている間は辺とラベルを描かない（大きなグラフで fps を保つ）
        s = sigma = new Sigma(g, host, { settings: { hideEdgesOnMove: true, hideLabelsOnMove: true },
          // ラベルの字と、ホバーしたときの吹き出しの地（既定は黒字・白地）
          styles: { ...DEFAULT_STYLES, nodes: { ...DEFAULT_STYLES.nodes, labelColor: cssVar("--fg"), backdropColor: cssVar("--bg") } } });
        s.on("clickNode", ({ node }) => onSelect(node));
      })
      .catch((e) => {
        if (!cancelled) failure = String(e?.message ?? e);
      });
    return () => {
      cancelled = true;
      s?.kill();
      if (sigma === s) sigma = undefined;
    };
  });
</script>

<div class="wrap">
  <div class="bar">
    <label>深さ
      <select aria-label="深さ" bind:value={depth}>
        {#each [1, 2, 3] as d}<option value={d}>{d}</option>{/each}
      </select>
    </label>
    <span>ノード {counts.nodes}・辺 {counts.edges}</span>
  </div>
  {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  <div class="canvas" bind:this={host} data-nodes={counts.nodes} data-edges={counts.edges}></div>
</div>

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; min-height: 20rem; font: 14px/1.4 system-ui, sans-serif; }
  .bar { display: flex; gap: 1rem; margin-bottom: .5rem; }
  .canvas { flex: 1; border: 1px solid var(--border); min-height: 16rem; background: var(--bg); }
  .failure { color: var(--err); white-space: pre-wrap; }
</style>

<!-- S260: 線形化の結果（状態空間の行列 A・B・C・D）。行・列の見出しは変数名、数は有効数字 6 桁。JSON は `globe linearize --json` と同じ形で書き出す -->
<script lang="ts">
  import type { Backend } from "./backend";
  import { browserSave } from "./browser-file";
  import { exportName } from "./plot/signals";
  import { TauriBackend, inDesktop } from "./tauri-backend";
  import type { Linearization } from "./protocol";

  let { result, onclose }: { result: Linearization; onclose: () => void } = $props();
  const blocks = $derived([
    { key: "A", rows: result.x, cols: result.x, m: result.A },
    { key: "B", rows: result.x, cols: result.u, m: result.B },
    { key: "C", rows: result.y, cols: result.x, m: result.C },
    { key: "D", rows: result.y, cols: result.u, m: result.D },
  ]);
  const fmt = (v: number | null) => (v == null || !Number.isFinite(v) ? "なし" : String(Number(v.toPrecision(6))));
  let note = $state("");
  const out: Pick<Backend, "saveFile"> = inDesktop() ? new TauriBackend() : { saveFile: browserSave };
  const save = () =>
    out.saveFile(exportName(result.model, "json"), JSON.stringify(result, null, 2), true).then(
      (p) => void (p && (note = `書き出した: ${p}`)),
      (e) => (note = String((e as Error).message ?? e)),
    );
</script>

<section class="lin" aria-label="線形化" data-linearization>
  <p>
    <strong>{result.model}</strong> を t = {fmt(result.time)} で線形化（状態 {result.x.length}・入力 {result.u.length}・出力 {result.y.length}）。
    <button type="button" onclick={save}>JSON を書き出す</button>
    <button type="button" onclick={onclose}>閉じる</button>
    {#if note}<small role="status">{note}</small>{/if}
  </p>
  <div class="grid">
    {#each blocks as b (b.key)}
      <div data-matrix={b.key}>
        <h3>{b.key}（{b.key === "A" ? "dx/dt ← x" : b.key === "B" ? "dx/dt ← u" : b.key === "C" ? "y ← x" : "y ← u"}）</h3>
        {#if b.rows.length && b.cols.length}
          <table>
            <thead><tr><th></th>{#each b.cols as c}<th scope="col">{c}</th>{/each}</tr></thead>
            <tbody>
              {#each b.m as row, i}
                <tr><th scope="row">{b.rows[i]}</th>{#each row as v}<td>{fmt(v)}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        {:else}<p class="empty">（空）</p>{/if}
      </div>
    {/each}
  </div>
</section>

<style>
  .lin { margin-top: .5rem; }
  .grid { display: flex; flex-wrap: wrap; gap: 1rem; }
  h3 { margin: .25rem 0; font-size: 1em; }
  table { border-collapse: collapse; }
  th, td { border: 1px solid var(--line); padding: .1rem .4rem; text-align: right; font-variant-numeric: tabular-nums; }
  th[scope="row"] { text-align: left; }
  .empty { color: var(--muted); margin: 0; }
</style>

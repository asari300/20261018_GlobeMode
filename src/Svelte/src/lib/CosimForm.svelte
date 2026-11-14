<script lang="ts">
  import { onMount } from "svelte";

  // S277: コシミュレーション（.ssp / spec の .json）の実行の設定。終了時刻と刻みは空なら SSP・JSON の既定。
  // FMU のネイティブコードを別プロセスで実行する（FmuForm と同じ警告の趣旨）。数に読めない欄は実行を止めて理由を出す。
  let { name, onrun, oncancel }: { name: string; onrun: (r: { stopTime?: number; step?: number }) => void; oncancel: () => void } = $props();

  let text = $state({ stopTime: "", step: "" });
  const fields = [["stopTime", "終了時刻"], ["step", "刻み"]] as const;
  const bad = (t: string) => !!t.trim() && !Number.isFinite(Number(t.trim()));
  const problem = $derived(fields.filter(([k]) => bad(text[k])).map(([k, l]) => `${l}「${text[k].trim()}」は数でない`).join("、"));
  let first: HTMLButtonElement;
  onMount(() => first.focus());

  function submit(e: Event) {
    e.preventDefault();
    if (problem) return;
    const req: { stopTime?: number; step?: number } = {};
    for (const [k] of fields) if (text[k].trim()) req[k] = Number(text[k].trim());
    onrun(req);
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") oncancel(); }} />

<form class="cosim" data-cosim-form aria-label="コシミュレーションの実行の設定" onsubmit={submit}>
  <p class="warn">この操作は FMU のネイティブコードを別プロセスで実行する。信頼できる FMU だけを回すこと。</p>
  <p>{name} をコシミュレーションで実行する（結果の列は <code>&lt;FMU名&gt;.&lt;変数名&gt;</code>）。</p>
  {#each fields as [k, label]}
    <label>{label}
      <input bind:value={text[k]} aria-label={label} inputmode="decimal" size="8" placeholder="既定" aria-invalid={bad(text[k])}>
    </label>
  {/each}
  <button type="submit" bind:this={first} disabled={!!problem}>実行</button>
  <button type="button" onclick={oncancel}>やめる</button>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</form>

<style>
  .cosim { margin: .25rem 0; padding: .5rem; border: 1px solid var(--muted); }
  .cosim label { margin-right: 1rem; }
  .warn { margin: 0 0 .25rem; }
  .problem { color: var(--err); margin: .25rem 0; }
</style>

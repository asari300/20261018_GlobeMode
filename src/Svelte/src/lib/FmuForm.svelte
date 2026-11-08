<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { FmuInfo, FmuRunRequest } from "./backend";

  // S236: FMU の実行の設定。S229 の確認ダイアログの代わり（警告は残す）。終了時刻・刻み・予算（壁時計の秒）と kind（CS / ME）。
  // 設定は FMU のパスごとに localStorage（globe-mode.fmu）へ残す（S228 の実行の設定と同じ作り）。ME の積分器の選択は出さない（FMU の既定）。
  // 空の欄は送らない（FMU の既定の実験設定に任せる）。数に読めない欄は実行を止めて理由を出す。
  let { name, path, info, onrun, oncancel }: { name: string; path: string; info: FmuInfo; onrun: (r: FmuRunRequest) => void; oncancel: () => void } = $props();

  type Saved = { kind?: string; stop_time?: string; step?: string; timeout?: string };
  const KEY = "globe-mode.fmu", CAP = 50;
  const readAll = (): Record<string, Saved> => {
    try { const v = JSON.parse(localStorage.getItem(KEY) ?? "{}"); return v && typeof v === "object" && !Array.isArray(v) ? v : {}; } catch { return {}; } // 使えなくても動く（S102）
  };
  const save = (s: Saved | null) => {
    try {
      const all = readAll();
      delete all[path]; // 入れ直して新しい順にする。多すぎれば古いものから落とす
      if (s) all[path] = s;
      for (const k of Object.keys(all).slice(0, Math.max(0, Object.keys(all).length - CAP))) delete all[k];
      localStorage.setItem(KEY, JSON.stringify(all));
    } catch { /* 保存できなくても使える */ }
  };
  const str = (v: unknown) => (typeof v === "string" ? v : "");
  // 開くたびに作り直す（App の {#if}）ので、初期値だけ読む
  const init = untrack(() => readAll()[path]);
  const can = untrack(() => ({ cs: info.interfaces.some((i) => /^CoSimulation/.test(i.kind)), me: info.interfaces.some((i) => /^ModelExchange/.test(i.kind)) }));
  let kind = $state(str(init?.kind) === "cs" && can.cs || str(init?.kind) === "me" && can.me ? str(init?.kind) : "");
  let text = $state({ stop_time: str(init?.stop_time), step: str(init?.step), timeout: str(init?.timeout) });
  const fields = [["stop_time", "終了時刻"], ["step", "刻み"], ["timeout", "予算（秒）"]] as const;
  const bad = (t: string) => !!t.trim() && !Number.isFinite(Number(t.trim()));
  const problem = $derived(fields.filter(([k]) => bad(text[k])).map(([k, l]) => `${l}「${text[k].trim()}」は数でない`).join("、"));
  let first: HTMLButtonElement;
  onMount(() => first.focus());

  function submit(e: Event) {
    e.preventDefault();
    if (problem) return;
    const req: FmuRunRequest = {};
    if (kind === "cs" || kind === "me") req.kind = kind;
    for (const [k] of fields) if (text[k].trim()) req[k] = Number(text[k].trim());
    save({ kind, ...text });
    onrun(req);
  }
  const reset = () => { kind = ""; text = { stop_time: "", step: "", timeout: "" }; save(null); };
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") oncancel(); }} />

<form class="fmu" data-fmu-form aria-label="FMU の実行の設定" onsubmit={submit}>
  <p class="warn" data-fmu-warning>{info.warning}</p>
  <p>{name}（{info.model_name}、FMI {info.fmi_version}、{info.interfaces.map((i) => i.kind).join("・")}）を実行する。</p>
  <label>種類
    <select bind:value={kind} aria-label="FMU の種類">
      <option value="">既定</option>
      {#if can.cs}<option value="cs">Co-Simulation（CS）</option>{/if}
      {#if can.me}<option value="me">Model Exchange（ME）</option>{/if}
    </select>
  </label>
  {#each fields as [k, label]}
    <label>{label}
      <input bind:value={text[k]} aria-label={label} inputmode="decimal" size="8" placeholder="FMU の既定" aria-invalid={bad(text[k])}>
    </label>
  {/each}
  <button type="submit" bind:this={first} disabled={!!problem}>実行</button>
  <button type="button" onclick={oncancel}>やめる</button>
  <button type="button" onclick={reset} title="この FMU の設定を消して既定に戻す">設定を既定に戻す</button>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</form>

<style>
  .fmu { margin: .25rem 0; padding: .5rem; border: 1px solid var(--muted); }
  .fmu label { margin-right: 1rem; }
  .warn { margin: 0 0 .25rem; }
  .problem { color: var(--err); margin: .25rem 0; }
</style>

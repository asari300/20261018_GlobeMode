<script lang="ts">
  import { onMount } from "svelte";
  import type { Backend } from "./backend";
  import type { CheckResponse, ResolvedSettings, SimulateRequest } from "./protocol";

  // 実行の設定（S142）。空の欄は、検査で分かった experiment annotation の値か既定値に任せる（欄の横に出どころ）。
  // 触った欄だけ settings に入れる（出どころは「指定」）。数に読めない欄は problem に理由を入れ、settings には入れない。
  let { backend, experiment, last, settings = $bindable({}), problem = $bindable("") }: {
    backend: Backend;
    experiment?: CheckResponse["experiment"];
    last?: ResolvedSettings | null; // 直近の実行で実際に使われた値と出どころ
    settings?: NonNullable<SimulateRequest["settings"]>;
    problem?: string;
  } = $props();

  const numbers = [["stop_time", "終了時刻"], ["tolerance", "許容誤差"], ["interval", "出力の間隔"]] as const;
  type Key = (typeof numbers)[number][0];
  let solvers = $state<string[]>([]);
  let solver = $state("");
  let text = $state<Record<Key, string | undefined>>({ stop_time: undefined, tolerance: undefined, interval: undefined }); // undefined は触っていない
  let parameters = $state("");

  onMount(() => { void backend.solvers().then((all) => (solvers = all.filter((e) => e.status.kind === "available").map((e) => e.name))); });

  // 触っていない欄は experiment の値を見せる（送らない）
  const shown = (k: Key) => text[k] ?? (experiment?.[k] != null ? String(experiment[k]) : "");
  const origin = (k: Key) => (text[k] !== undefined ? "指定" : experiment?.[k] != null ? "experiment" : "既定");
  const names = { requested: "指定", experiment: "experiment", default: "既定" };

  $effect(() => {
    const s: NonNullable<SimulateRequest["settings"]> = {};
    const bad: string[] = [];
    if (solver) s.solver = solver;
    for (const [k, label] of numbers) {
      const t = text[k]?.trim();
      if (!t) continue;
      if (Number.isFinite(Number(t))) s[k] = Number(t);
      else bad.push(`${label}「${t}」は数でない`);
    }
    for (const line of parameters.split("\n").map((l) => l.trim()).filter(Boolean)) {
      const m = /^([^=\s]+)\s*=\s*(\S+)$/.exec(line);
      if (m && Number.isFinite(Number(m[2]))) (s.parameters ??= {})[m[1]] = Number(m[2]);
      else bad.push(`パラメータ「${line}」は 名前=値 でない`);
    }
    settings = s;
    problem = bad.join("、");
  });
</script>

<details class="settings" open>
  <summary>実行の設定</summary>
  <label>ソルバ
    <select bind:value={solver} aria-label="ソルバ">
      <option value="">既定</option>
      {#each solvers as s}<option value={s}>{s}</option>{/each}
    </select>
  </label>
  {#each numbers as [k, label]}
    <label>{label}
      <input
        value={shown(k)} aria-label={label} inputmode="decimal" size="8"
        oninput={(e) => (text[k] = e.currentTarget.value)}
        onchange={(e) => { if (!e.currentTarget.value.trim()) text[k] = undefined; }}
      >
      <small data-origin={k}>{origin(k)}</small>
    </label>
  {/each}
  <label>パラメータの上書き（1 行に 名前=値）
    <textarea bind:value={parameters} rows="2" cols="16" aria-label="パラメータの上書き"></textarea>
  </label>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
  {#if last}
    <p class="last" data-last>
      直近の実行: 開始 {last.start_time}、終了 {last.stop_time}（{names[last.origins.stop_time]}）、許容誤差 {last.tolerance}（{names[last.origins.tolerance]}）、出力の間隔 {last.interval}（{names[last.origins.interval]}）、ソルバ {last.solver}（{names[last.origins.solver]}）
    </p>
  {/if}
</details>

<style>
  .settings { margin: .25rem 0; }
  .settings label { margin-right: 1rem; }
  small { color: #555; }
  .problem { color: #b00020; margin: .25rem 0; }
  .last { color: #555; margin: .25rem 0; }
</style>

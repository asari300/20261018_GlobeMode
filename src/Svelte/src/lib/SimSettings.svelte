<script lang="ts">
  import { onMount } from "svelte";
  import type { Backend } from "./backend";
  import type { CheckResponse, ResolvedSettings, SimulateRequest, TunableParameter } from "./protocol";

  // 実行の設定（S142）。空の欄は、検査で分かった experiment annotation の値か既定値に任せる（欄の横に出どころ）。
  // 触った欄だけ settings に入れる（出どころは「指定」）。数に読めない欄は problem に理由を入れ、settings には入れない。
  // S170: パラメータの一覧は閉じて始める（開いていると結果を画面の下へ押し下げ、検査のあとに一覧が届くと結果の位置がずれる）。
  // S150: 検査のあと（checked が新しくなるたび）に上書きできるパラメータの一覧を取り、値を打った行だけ 名前=値 と同じ形で入れる（下の欄と同じ名前なら下の欄が優先）。
  let { backend, experiment, checked = null, last, settings = $bindable({}), problem = $bindable("") }: {
    backend: Backend;
    experiment?: CheckResponse["experiment"];
    checked?: { model: string } | null; // 直近に検査したモデル
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
  let listed = $state<TunableParameter[]>([]);
  let listedModel = $state("");
  let listNote = $state("");
  let filter = $state("");
  let picked = $state<Record<string, string>>({}); // 一覧の行に打った値（名前 → 字面）
  const rows = $derived(listed.filter((p) => `${p.name} ${p.description ?? ""}`.toLowerCase().includes(filter.trim().toLowerCase())));

  $effect(() => {
    const model = checked?.model;
    if (!model) return;
    void backend.tunableParameters(model).then(
      (all) => { listed = all; listedModel = model; listNote = all.length ? "" : "上書きできるパラメータは無い"; picked = Object.fromEntries(Object.entries(picked).filter(([n]) => all.some((p) => p.name === n))); },
      (e) => { listed = []; listedModel = model; listNote = `一覧を取れない: ${String((e as Error).message ?? e)}`; },
    );
  });

  onMount(() => { void backend.solvers().then((all) => (solvers = all.filter((e) => e.status.kind === "available").map((e) => e.name))); });

  // 触っていない欄は experiment の値を見せる（送らない）
  const shown = (k: Key) => text[k] ?? (experiment?.[k] != null ? String(experiment[k]) : "");
  const origin = (k: Key) => (text[k] !== undefined ? "指定" : experiment?.[k] != null ? "experiment" : "既定");
  const isBad = (t: string | undefined) => !!t?.trim() && !Number.isFinite(Number(t.trim())); // S173: 数に読めない欄に aria-invalid（理由は下の .problem）
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
    for (const [name, t] of Object.entries(picked)) {
      if (!t.trim()) continue;
      if (Number.isFinite(Number(t))) (s.parameters ??= {})[name] = Number(t);
      else bad.push(`パラメータ ${name}「${t}」は数でない`);
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
        value={shown(k)} aria-label={label} inputmode="decimal" size="8" aria-invalid={isBad(text[k])} aria-describedby={isBad(text[k]) ? "sim-problem" : undefined}
        oninput={(e) => (text[k] = e.currentTarget.value)}
        onchange={(e) => { if (!e.currentTarget.value.trim()) text[k] = undefined; }}
      >
      <small data-origin={k}>{origin(k)}</small>
    </label>
  {/each}
  {#if listedModel}
    <details class="params" data-params>
      <summary>パラメータ（{listedModel}。{listed.length} 本。値を打った行だけ上書き）{listNote ? ` ${listNote}` : ""}</summary>
      {#if listed.length > 12}<input bind:value={filter} aria-label="パラメータの絞り込み" placeholder="名前・説明で絞り込む" size="24">（{rows.length} 本）{/if}
      <div class="rows">
        <table>
          <tbody>
            {#each rows as p (p.name)}
              <tr data-param={p.name}>
                <th scope="row">{p.name}</th>
                <td><input value={picked[p.name] ?? ""} placeholder={String(p.default)} aria-label={`${p.name} の値`} inputmode="decimal" size="8" aria-invalid={isBad(picked[p.name])} oninput={(e) => (picked[p.name] = e.currentTarget.value)}></td>
                <td>{p.unit ?? ""}</td>
                <td><small>{p.description ?? ""}</small></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </details>
  {/if}
  <label>パラメータの上書き（1 行に 名前=値）
    <textarea bind:value={parameters} rows="2" cols="16" aria-label="パラメータの上書き"></textarea>
  </label>
  {#if problem}<p class="problem" id="sim-problem" role="alert">{problem}</p>{/if}
  {#if last}
    <p class="last" data-last>
      直近の実行: 開始 {last.start_time}、終了 {last.stop_time}（{names[last.origins.stop_time]}）、許容誤差 {last.tolerance}（{names[last.origins.tolerance]}）、出力の間隔 {last.interval}（{names[last.origins.interval]}）、ソルバ {last.solver}（{names[last.origins.solver]}）
    </p>
  {/if}
</details>

<style>
  .settings { margin: .25rem 0; }
  .settings label { margin-right: 1rem; }
  small { color: var(--muted); }
  .rows { max-height: 12rem; overflow-y: auto; }
  th { text-align: left; font-weight: normal; padding-right: .5rem; }
  .problem { color: var(--err); margin: .25rem 0; }
  .last { color: var(--muted); margin: .25rem 0; }
</style>

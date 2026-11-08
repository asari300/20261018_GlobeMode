<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Backend } from "./backend";
  import type { CheckResponse, Diagnostic, ResolvedSettings, SimulateRequest, TunableParameter } from "./protocol";

  // 実行の設定（S142）。空の欄は、検査で分かった experiment annotation の値か既定値に任せる（欄の横に出どころ）。
  // 触った欄だけ settings に入れる（出どころは「指定」）。数に読めない欄は problem に理由を入れ、settings には入れない。
  // S170: パラメータの一覧は閉じて始める（開いていると結果を画面の下へ押し下げ、検査のあとに一覧が届くと結果の位置がずれる）。
  // S228: 開始時刻・刻み幅（固定刻みのソルバのときだけ）・予算（壁時計の秒）の欄。設定はモデル名ごとに localStorage（globe-mode.sim）へ残し、モデルを切り替えるとそのモデルの設定に戻る（「既定に戻す」で消える）。
  //   GM-E10004（設定・experiment の値が妥当でない）は欄のすぐ下にも出す（errors）。
  // S150: 検査のあと（checked が新しくなるたび）に上書きできるパラメータの一覧を取り、値を打った行だけ 名前=値 と同じ形で入れる（下の欄と同じ名前なら下の欄が優先）。
  let { backend, model, experiment, checked = null, last, errors = [], settings = $bindable({}), problem = $bindable("") }: {
    backend: Backend;
    model: string; // いま編集しているモデル名（保存の鍵）
    errors?: Diagnostic[]; // GM-E10004 の診断
    experiment?: CheckResponse["experiment"];
    checked?: { model: string } | null; // 直近に検査したモデル
    last?: ResolvedSettings | null; // 直近の実行で実際に使われた値と出どころ
    settings?: NonNullable<SimulateRequest["settings"]>;
    problem?: string;
  } = $props();

  const numbers = [["start_time", "開始時刻"], ["stop_time", "終了時刻"], ["tolerance", "許容誤差"], ["interval", "出力の間隔"], ["step", "刻み幅"], ["max_wall_seconds", "予算（秒）"]] as const;
  type Key = (typeof numbers)[number][0];
  const blank = () => ({ start_time: undefined, stop_time: undefined, tolerance: undefined, interval: undefined, step: undefined, max_wall_seconds: undefined });
  // shortcut: 固定刻みのソルバの名前を Rust（globe_mode_sim::solvers）から写した。足すときはここも直す。SolverEntry に印が付いたら置き換える
  const FIXED = ["euler", "heun", "rk4", "impeuler", "trapezoid"];
  const hint: Partial<Record<Key, string>> = { step: "出力の間隔", max_wall_seconds: "無制限" }; // 空のときの既定
  let solvers = $state<string[]>([]);
  let solver = $state("");
  const fixed = $derived(FIXED.includes(solver));
  let text = $state<Record<Key, string | undefined>>(blank()); // undefined は触っていない
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
  const fromExperiment = (k: Key) => (k === "step" || k === "max_wall_seconds" ? undefined : experiment?.[k]); // experiment annotation にあるのは前の 4 つ
  const shown = (k: Key) => text[k] ?? (fromExperiment(k) != null ? String(fromExperiment(k)) : "");
  const origin = (k: Key) => (text[k] !== undefined ? "指定" : fromExperiment(k) != null ? "experiment" : "既定");
  const isBad = (t: string | undefined) => !!t?.trim() && !Number.isFinite(Number(t.trim())); // S173: 数に読めない欄に aria-invalid（理由は下の .problem）
  const names = { requested: "指定", experiment: "experiment", default: "既定" };

  $effect(() => {
    const s: NonNullable<SimulateRequest["settings"]> = {};
    const bad: string[] = [];
    if (solver) s.solver = solver;
    for (const [k, label] of numbers) {
      const t = text[k]?.trim();
      if (!t || (k === "step" && !fixed)) continue; // 刻み幅は固定刻みのソルバのときだけ送る
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

  // S228: モデル名ごとの保存。モデルが変わったらそのモデルの保存を読み（無ければ既定）、欄を直したら書く。loaded は「欄がどのモデルのものか」（読む前に書いて、別のモデルの欄を上書きしない）
  const KEY = "globe-mode.sim", CAP = 50;
  type Saved = { solver?: string; text?: Partial<Record<Key, string>>; parameters?: string; picked?: Record<string, string> };
  const readAll = (): Record<string, Saved> => {
    try { const v = JSON.parse(localStorage.getItem(KEY) ?? "{}"); return v && typeof v === "object" && !Array.isArray(v) ? v : {}; } catch { return {}; } // 使えなくても動く（S102）
  };
  const strings = (o: unknown) => Object.fromEntries(Object.entries(o && typeof o === "object" ? o : {}).filter(([, v]) => typeof v === "string"));
  let loaded = "";
  const snap = $derived.by((): Saved | null => {
    const t = Object.fromEntries(Object.entries(text).filter(([, v]) => v !== undefined));
    const pk = Object.fromEntries(Object.entries(picked).filter(([, v]) => v.trim()));
    return solver || Object.keys(t).length || parameters.trim() || Object.keys(pk).length ? { solver, text: t, parameters, picked: pk } : null;
  });
  $effect(() => {
    const m = model;
    untrack(() => {
      const s = readAll()[m];
      solver = typeof s?.solver === "string" ? s.solver : "";
      text = { ...blank(), ...strings(s?.text) };
      parameters = typeof s?.parameters === "string" ? s.parameters : "";
      picked = strings(s?.picked) as Record<string, string>;
      loaded = m;
    });
  });
  const save = (m: string, s: Saved | null) => {
    try {
      const all = readAll();
      delete all[m]; // 入れ直して新しい順にする。多すぎれば古いものから落とす
      if (s) all[m] = s;
      for (const k of Object.keys(all).slice(0, Math.max(0, Object.keys(all).length - CAP))) delete all[k];
      localStorage.setItem(KEY, JSON.stringify(all));
    } catch { /* 保存できなくても使える */ }
  };
  $effect(() => { const s = snap; untrack(() => { if (loaded === model) save(model, s); }); });
  const reset = () => { solver = ""; text = blank(); parameters = ""; picked = {}; save(model, null); };
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
        value={shown(k)} aria-label={label} inputmode="decimal" size="8" placeholder={hint[k]} disabled={k === "step" && !fixed} aria-invalid={isBad(text[k])} aria-describedby={isBad(text[k]) ? "sim-problem" : undefined}
        oninput={(e) => (text[k] = e.currentTarget.value)}
        onchange={(e) => { if (!e.currentTarget.value.trim()) text[k] = undefined; }}
      >
      <small data-origin={k}>{k === "step" && !fixed ? "固定刻みのソルバのみ" : origin(k)}</small>
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
  <button type="button" onclick={reset} disabled={!snap} title="このモデルの設定を消して既定に戻す">設定を既定に戻す</button>
  {#if errors.length}<p class="refusal" id="sim-refusal" role="alert" data-refusal>{#each errors as d}<span>{d.code} {d.message.split("\n")[0]}</span> {/each}</p>{/if}
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
  .problem, .refusal { color: var(--err); margin: .25rem 0; }
  .last { color: var(--muted); margin: .25rem 0; }
</style>

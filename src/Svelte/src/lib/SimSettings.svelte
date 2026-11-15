<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Backend } from "./backend";
  import ParameterDialog from "./ParameterDialog.svelte";
  import { inDesktop } from "./tauri-backend";
  import type { ManyConfig } from "./many";
  import type { CheckResponse, Diagnostic, ResolvedSettings, SimulateRequest, TunableParameter } from "./protocol";

  // 実行の設定（S142）。空の欄は、検査で分かった experiment annotation の値か既定値に任せる（欄の横に出どころ）。
  // 触った欄だけ settings に入れる（出どころは「指定」）。数に読めない欄は problem に理由を入れ、settings には入れない。
  // S170: パラメータの一覧は閉じて始める（開いていると結果を画面の下へ押し下げ、検査のあとに一覧が届くと結果の位置がずれる）。
  // S228: 開始時刻・刻み幅（固定刻みのソルバのときだけ）・予算（壁時計の秒）の欄。設定はモデル名ごとに localStorage（globe-mode.sim）へ残し、モデルを切り替えるとそのモデルの設定に戻る（「既定に戻す」で消える）。
  //   GM-E10004（設定・experiment の値が妥当でない）は欄のすぐ下にも出す（errors）。
  // S150: 検査のあと（checked が新しくなるたび）に上書きできるパラメータの一覧を取り、値を打った行だけ 名前=値 と同じ形で入れる（下の欄と同じ名前なら下の欄が優先）。
  // S323（ADR-0040）: ソルバの選び方（1 つ／複数）、実行の仕方（1 つずつ／並列）、全部終わったあとの比較（既定は off）。複数のときは many に依頼の分を出し、App が simulateMany を呼ぶ。最大積分次数・許容誤差は全部に同じ値で渡る。
  // S248: 「パラメータ…」のダイアログ（部品の名前つき・モデル全体）。picked（上の一覧と同じ名前 → 字面）を共有し、GM-E10006 は欄の近くに出す。
  let { backend, model, experiment, checked = null, last, errors = [], openParams = null, onwrite, settings = $bindable({}), many = $bindable(null), problem = $bindable("") }: {
    backend: Backend;
    model: string; // いま編集しているモデル名（保存の鍵）
    errors?: Diagnostic[]; // GM-E10004・GM-E10006 の診断
    openParams?: { part: string } | null; // 図の部品のダブルクリック。新しい値が来るたびダイアログを開く
    onwrite?: (cls: string, modifiers: { part: string; name: string; value: string }[]) => Promise<string | null>; // S294: ダイアログの「ソースに書く」（App がエディタに入れる）
    experiment?: CheckResponse["experiment"];
    checked?: { model: string } | null; // 直近に検査したモデル
    last?: ResolvedSettings | null; // 直近の実行で実際に使われた値と出どころ
    settings?: NonNullable<SimulateRequest["settings"]>;
    many?: ManyConfig | null;
    problem?: string;
  } = $props();

  const numbers = [["start_time", "開始時刻"], ["stop_time", "終了時刻"], ["tolerance", "許容誤差"], ["max_order", "最大積分次数"], ["interval", "出力の間隔"], ["step", "刻み幅"], ["max_wall_seconds", "予算（秒）"]] as const;
  type Key = (typeof numbers)[number][0];
  const blank = () => ({ start_time: undefined, stop_time: undefined, tolerance: undefined, max_order: undefined, interval: undefined, step: undefined, max_wall_seconds: undefined });
  // shortcut: 固定刻みのソルバの名前を Rust（globe_mode_sim::solvers）から写した。足すときはここも直す。SolverEntry に印が付いたら置き換える
  const FIXED = ["euler", "heun", "rk4", "impeuler", "trapezoid"];
  const hint: Partial<Record<Key, string>> = { max_order: "ソルバの既定", step: "出力の間隔", max_wall_seconds: "無制限" }; // 空のときの既定
  let solvers = $state<string[]>([]);
  let solver = $state("");
  let mode = $state<"single" | "multi">("single");
  let chosen = $state<string[]>([]); // 複数のとき選んだソルバ（実行の順は一覧の順）
  let parallel = $state(false);
  let compareOn = $state(false);
  let expText = $state("6"); // 比較の閾値 10^-n の n
  let reference = $state("");
  const MAX_SOLVERS = 16;
  const fixed = $derived(mode === "multi" ? chosen.some((c) => FIXED.includes(c)) : FIXED.includes(solver));
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
      (all) => { listed = all; listedModel = model; listNote = all.length ? "" : "上書きできるパラメータは無い"; },
      (e) => { listed = []; listedModel = model; listNote = `一覧を取れない: ${String((e as Error).message ?? e)}`; },
    );
  });

  let dialog = $state<{ prefix: string } | null>(null);
  $effect(() => { if (openParams) dialog = { prefix: openParams.part ? `${openParams.part}.` : "" }; });

  onMount(() => { void backend.solvers().then((all) => (solvers = all.filter((e) => e.status.kind === "available").map((e) => e.name))); });

  // 触っていない欄は experiment の値を見せる（送らない）
  const fromExperiment = (k: Key) => (k === "step" || k === "max_wall_seconds" || k === "max_order" ? undefined : experiment?.[k]); // experiment annotation にあるのは前の 4 つ
  const shown = (k: Key) => text[k] ?? (fromExperiment(k) != null ? String(fromExperiment(k)) : "");
  const origin = (k: Key) => (text[k] !== undefined ? "指定" : fromExperiment(k) != null ? "experiment" : "既定");
  const isBad = (t: string | undefined) => !!t?.trim() && !Number.isFinite(Number(t.trim())); // S173: 数に読めない欄に aria-invalid（理由は下の .problem）
  const names = { requested: "指定", experiment: "experiment", default: "既定" };

  $effect(() => {
    const s: NonNullable<SimulateRequest["settings"]> = {};
    const bad: string[] = [];
    const sel = solvers.filter((c) => chosen.includes(c));
    if (mode === "single" && solver) s.solver = solver;
    for (const [k, label] of numbers) {
      const t = text[k]?.trim();
      if (!t || (k === "step" && !fixed)) continue; // 刻み幅は固定刻みのソルバのときだけ送る
      if (!Number.isFinite(Number(t))) bad.push(`${label}「${t}」は数でない`);
      else if (k === "max_order" && !(Number.isInteger(Number(t)) && Number(t) >= 1 && Number(t) <= 5)) bad.push(`${label}「${t}」は 1〜5 の整数でない`);
      else s[k] = Number(t);
    }
    let cfg: ManyConfig | null = null;
    if (mode === "multi") {
      const n = Number(expText.trim());
      if (!sel.length) bad.push("比べるソルバを 1 つ以上選ぶ");
      if (compareOn && sel.length < 2) bad.push("比べるには 2 つ以上のソルバを選ぶ");
      if (compareOn && !(expText.trim() && Number.isInteger(n) && n >= 1 && n <= 15)) bad.push(`比較の閾値の n「${expText}」は 1〜15 の整数でない`);
      cfg = { solvers: sel, parallel, compare: compareOn ? { threshold_exp: n, ...(sel.includes(reference) ? { reference } : {}) } : undefined };
    }
    many = cfg;
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
  type Saved = { solver?: string; multi?: { chosen: string[]; parallel: boolean; compareOn: boolean; exp: string; reference: string }; text?: Partial<Record<Key, string>>; parameters?: string; picked?: Record<string, string> };
  const readAll = (): Record<string, Saved> => {
    try { const v = JSON.parse(localStorage.getItem(KEY) ?? "{}"); return v && typeof v === "object" && !Array.isArray(v) ? v : {}; } catch { return {}; } // 使えなくても動く（S102）
  };
  const strings = (o: unknown) => Object.fromEntries(Object.entries(o && typeof o === "object" ? o : {}).filter(([, v]) => typeof v === "string"));
  let loaded = "";
  const snap = $derived.by((): Saved | null => {
    const t = Object.fromEntries(Object.entries(text).filter(([, v]) => v !== undefined));
    const pk = Object.fromEntries(Object.entries(picked).filter(([, v]) => v.trim()));
    const multi = mode === "multi" ? { chosen: [...chosen], parallel, compareOn, exp: expText, reference } : undefined;
    return solver || multi || Object.keys(t).length || parameters.trim() || Object.keys(pk).length ? { solver, multi, text: t, parameters, picked: pk } : null;
  });
  $effect(() => {
    const m = model;
    untrack(() => {
      const s = readAll()[m];
      solver = typeof s?.solver === "string" ? s.solver : "";
      const mu = s?.multi;
      mode = mu && Array.isArray(mu.chosen) ? "multi" : "single";
      chosen = mu && Array.isArray(mu.chosen) ? mu.chosen.filter((x) => typeof x === "string").slice(0, MAX_SOLVERS) : [];
      parallel = mu?.parallel === true;
      compareOn = mu?.compareOn === true;
      expText = typeof mu?.exp === "string" ? mu.exp : "6";
      reference = typeof mu?.reference === "string" ? mu.reference : "";
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
  const reset = () => { solver = ""; mode = "single"; chosen = []; parallel = false; compareOn = false; expText = "6"; reference = ""; text = blank(); parameters = ""; picked = {}; save(model, null); };
</script>

<details class="settings" open>
  <summary>実行の設定</summary>
  <fieldset class="opt" data-solver-mode>
    <legend>ソルバの選び方</legend>
    <label><input type="radio" name="sim-mode" bind:group={mode} value="single"> 1 つ</label>
    <label><input type="radio" name="sim-mode" bind:group={mode} value="multi"> 複数</label>
  </fieldset>
  {#if mode === "single"}
    <label>ソルバ
      <select bind:value={solver} aria-label="ソルバ">
        <option value="">既定</option>
        {#each solvers as s}<option value={s}>{s}</option>{/each}
      </select>
    </label>
  {:else}
    <fieldset class="opt" data-many-solvers>
      <legend>比べるソルバ（{chosen.length} 個を選択。1〜{MAX_SOLVERS} 個）</legend>
      {#each solvers as s}
        <label><input type="checkbox" bind:group={chosen} value={s} disabled={!chosen.includes(s) && chosen.length >= MAX_SOLVERS}> {s}</label>
      {/each}
    </fieldset>
    <fieldset class="opt" data-many-parallel>
      <legend>実行の仕方</legend>
      <label><input type="radio" name="sim-par" bind:group={parallel} value={false}> 1 つずつ</label>
      <label><input type="radio" name="sim-par" bind:group={parallel} value={true}> 並列</label>
      {#if parallel && !inDesktop()}<small data-parallel-note>ブラウザでは 1 つずつ回す（並列は Desktop と CLI のみ）</small>{/if}
    </fieldset>
    <fieldset class="opt" data-many-compare>
      <legend>比較</legend>
      <label><input type="checkbox" bind:checked={compareOn}> 全部終わったら結果を比べる</label>
      {#if compareOn}
        <label>閾値 10<sup>-n</sup> の n
          <input bind:value={expText} aria-label="比較の閾値の n" inputmode="numeric" size="3" aria-invalid={!/^\d+$/.test(expText.trim()) || Number(expText) < 1 || Number(expText) > 15} aria-describedby="sim-problem">
        </label>
        <label>基準
          <select bind:value={reference} aria-label="基準のソルバ">
            <option value="">最初に成功したもの</option>
            {#each solvers.filter((c) => chosen.includes(c)) as s}<option value={s}>{s}</option>{/each}
          </select>
        </label>
        <small>許容誤差・最大積分次数は全部のソルバに同じ値で渡す</small>
      {/if}
    </fieldset>
  {/if}
  {#each numbers as [k, label]}
    <label>{label}
      <input
        value={shown(k)} aria-label={label} inputmode="decimal" size="8" placeholder={hint[k]} disabled={k === "step" && !fixed} aria-invalid={isBad(text[k])} aria-describedby={isBad(text[k]) ? "sim-problem" : undefined}
        oninput={(e) => (text[k] = e.currentTarget.value)}
        onchange={(e) => { if (!e.currentTarget.value.trim()) text[k] = undefined; }}
      >
      <small data-origin={k}>{k === "step" && !fixed ? "固定刻みのソルバのみ" : k === "max_order" ? `${origin(k)}。cvode・ida だけが使う` : origin(k)}</small>
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
  <button type="button" data-open-params onclick={() => (dialog = { prefix: "" })} title="パラメータの一覧を開いて値を入れる（モデル全体）">パラメータ…</button>
  {#if dialog}
    {#key dialog}<ParameterDialog {backend} {model} prefix={dialog.prefix} bind:picked errors={errors.filter((d) => d.code === "GM-E10006")} {onwrite} onclose={() => (dialog = null)} />{/key}
  {/if}
  <label>パラメータの上書き（1 行に 名前=値）
    <textarea bind:value={parameters} rows="2" cols="16" aria-label="パラメータの上書き"></textarea>
  </label>
  <button type="button" onclick={reset} disabled={!snap} title="このモデルの設定を消して既定に戻す">設定を既定に戻す</button>
  {#if errors.some((d) => d.code === "GM-E10004")}<p class="refusal" id="sim-refusal" role="alert" data-refusal>{#each errors.filter((d) => d.code === "GM-E10004") as d}<span>{d.code} {d.message.split("\n")[0]}</span> {/each}</p>{/if}
  {#if problem}<p class="problem" id="sim-problem" role="alert">{problem}</p>{/if}
  {#if last}
    <p class="last" data-last>
      直近の実行: 開始 {last.start_time}、終了 {last.stop_time}（{names[last.origins.stop_time]}）、許容誤差 {last.tolerance}（{names[last.origins.tolerance]}）、出力の間隔 {last.interval}（{names[last.origins.interval]}）、ソルバ {last.solver}（{names[last.origins.solver]}）
    </p>
  {/if}
</details>

<style>
  .settings { margin: .25rem 0; }
  .settings label { display: inline-block; margin-right: 1rem; } /* S348: 欄の名前と入力欄・チェックの名前と箱が行の端で泣き別れない */
  .opt { display: inline-block; vertical-align: top; margin: 0 .5rem .25rem 0; padding: .1rem .5rem; border: 1px solid var(--border); }
  small { color: var(--muted); }
  .rows { max-height: 12rem; overflow-y: auto; }
  th { text-align: left; font-weight: normal; padding-right: .5rem; }
  .problem, .refusal { color: var(--err); margin: .25rem 0; }
  .last { color: var(--muted); margin: .25rem 0; }
</style>

<!-- S248: パラメータのダイアログ。モデルのパラメータ宣言（部品ごとの入れ子の名前・型・既定値・単位）を出し、入れた値を picked（名前 → 字面）に入れる。
     picked は SimSettings の「実行の設定」の parameters になる（ソースは書き換えない）。変えた値だけが入り、既定と同じ値に戻せば消える。
     S294: 「ソースに書く」は、picked のうち `部品.名前` の形の行を、その部品の宣言の修飾 `(名前 = 値)` として onwrite（App）に渡す（エディタに 1 回の dispatch。保存しない）。
     S298: 列挙も書ける（`型.リテラル`。型の名前は enum_type）。行の「修飾を外す」は空の値を onwrite に渡す。書いたあと parameters を引き直して表を更新する。 -->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Backend } from "./backend";
  import type { Diagnostic, ModelParameter } from "./protocol";

  let { backend, model, prefix = "", picked = $bindable({}), errors = [], onwrite, onclose }: {
    backend: Backend;
    model: string;
    prefix?: string; // 開いたときの絞り込み（図の部品なら `c1.`）
    picked?: Record<string, string>;
    errors?: Diagnostic[]; // GM-E10006。メッセージの中の名前で欄を探す
    onwrite?: (cls: string, modifiers: { part: string; name: string; value: string }[]) => Promise<string | null>; // S294。null なら書けた、文字列なら断られた理由
    onclose: () => void;
  } = $props();

  const MAX = 200; // 行が多いモデルは先頭だけ描く（絞り込みで探す）
  let dlg: HTMLDialogElement;
  let all = $state<ModelParameter[]>([]);
  let note = $state("読み込み中…");
  let filter = $state(untrack(() => prefix));
  const q = $derived(filter.trim().toLowerCase());
  const matched = $derived(all.filter((p) => !q || `${p.name} ${p.description ?? ""}`.toLowerCase().includes(q)));
  const rows = $derived(matched.slice(0, MAX));
  const isArray = (p: ModelParameter) => !!p.dims?.length;
  const editable = (p: ModelParameter) => p.type !== "String" && !isArray(p);
  // 診断は名前を文中に書く（「パラメータ n は…」「Integer の c.n に 2.5 は…」）。識別子の一部（`c.n2`）には当てない
  const errorsOf = (name: string) => { const re = new RegExp(`(?<![\\w.])${name.replace(/[.[\]]/g, "\\$&")}(?![\\w.])`); return errors.filter((d) => re.test(d.message)); };
  const isBad = (t: string | undefined) => !!t?.trim() && !Number.isFinite(Number(t.trim()));

  const load = () => backend.parameters(model).then(
    (ps) => { all = ps; note = ps.length ? "" : "パラメータは無い"; },
    (e) => { note = `一覧を取れない: ${String((e as Error).message ?? e)}`; },
  );
  onMount(() => { dlg.showModal(); void load(); });

  const set = (p: ModelParameter, t: string, sameAsDefault = false) => {
    const next = { ...picked };
    if (!t.trim() || sameAsDefault) delete next[p.name];
    else next[p.name] = t;
    picked = next;
  };
  const clear = (name: string) => { const next = { ...picked }; delete next[name]; picked = next; };
  const effective = (p: ModelParameter) => picked[p.name] ?? (p.value != null ? String(p.value) : "");
  // S294: 書けるのは部品の直下の 1 段（`c.k`）。モデル自身の宣言・入れ子（`c.d.k`）・配列の部品は書かない
  const PART_NAME = /^([A-Za-z_]\w*)\.([A-Za-z_]\w*)$/;
  const ident = (l: string) => (/^[A-Za-z_]\w*$/.test(l) ? l : `'${l}'`);
  // 列挙は型の名前（enum_type）が無いと書けない。値は ordinal（1 始まり）なので、リテラル名を引いて `型.リテラル` にする
  const enumText = (p: ModelParameter) => { const l = p.literals?.[Number(picked[p.name]) - 1]; return p.enum_type && l ? `${p.enum_type}.${ident(l)}` : null; };
  const textOf = (p: ModelParameter) => (p.type === "Boolean" ? (picked[p.name] === "1" ? "true" : "false") : p.type === "Enumeration" ? enumText(p)! : picked[p.name].trim());
  const writable = $derived(all.filter((p) => p.name in picked && editable(p) && (p.type !== "Enumeration" || enumText(p)) && !isBad(picked[p.name]) && PART_NAME.test(p.name)));
  let writeNote = $state("");
  // 書いたら引き直す（ソースの既定が変わるので、既定値の列は再検査を待たずに更新する）。App の onwrite がエディタの本文を Workspace に入れ直してから返る
  async function apply(ps: ModelParameter[], value: (p: ModelParameter) => string, done: string) {
    const mods = ps.map((p) => { const [, part, name] = PART_NAME.exec(p.name)!; return { part, name, value: value(p) }; });
    const err = await onwrite!(model, mods);
    if (err) { writeNote = `書けなかった: ${err}`; return; }
    const next = { ...picked };
    for (const p of ps) delete next[p.name]; // ソースの既定になった（外した）ので、実行時の上書きからは外す
    picked = next;
    writeNote = done;
    await load();
  }
  const write = () => apply(writable, textOf, `${writable.length} 本をソースの修飾に書いた（Ctrl+Z で戻せる。保存はしていない）`);
  const unwrite = (p: ModelParameter) => apply([p], () => "", `${p.name} の修飾をソースから外した（Ctrl+Z で戻せる。保存はしていない）`);
  const changed = $derived(Object.keys(picked).filter((n) => all.some((p) => p.name === n)).length);
</script>

<dialog bind:this={dlg} class="params-dialog" data-param-dialog aria-label={`${model} のパラメータ`} onclose={onclose}>
  <header>
    <strong>{model} のパラメータ</strong>
    <small>値を入れると実行の設定の parameters に乗る（実行のときだけの上書き。ソースは変えない）。「ソースに書く」は部品の宣言の修飾 <code>(名前 = 値)</code> として書く</small>
  </header>
  <p>
    <input bind:value={filter} aria-label="パラメータの絞り込み" placeholder="名前・説明で絞り込む" size="24">
    （{matched.length} / {all.length} 本）
    <button type="button" onclick={() => (picked = {})} disabled={!Object.keys(picked).length} title="このモデルの上書きをすべて消す">すべて既定に戻す</button>
    {#if changed}<small data-changed>変更 {changed} 本</small>{/if}
    {#if onwrite}<button type="button" data-write-source onclick={write} disabled={!writable.length} title="入れた値のうち、部品の直下のパラメータ（c.k の形）を、その部品の宣言の修飾としてソースに書く">ソースに書く（{writable.length} 本）</button>{/if}
  </p>
  {#if writeNote}<p class="note" data-write-note role="status">{writeNote}</p>{/if}
  {#if note}<p class="note" data-param-note>{note}</p>{/if}
  <div class="rows">
    <table>
      <thead><tr><th>名前</th><th>型</th><th>値</th><th>既定値</th><th>単位</th><th>説明</th><th></th></tr></thead>
      <tbody>
        {#each rows as p (p.name)}
          {@const errs = errorsOf(p.name)}
          <tr data-param={p.name} class:changed={p.name in picked}>
            <th scope="row">{p.name}</th>
            <td><small>{p.type}{isArray(p) ? `[${p.dims!.join(",")}]` : ""}</small></td>
            <td>
              {#if !editable(p)}
                <small>{p.type === "String" ? "文字列は変えられない" : "配列は変えられない"}</small>
              {:else if p.type === "Boolean"}
                <input type="checkbox" aria-label={`${p.name} の値`} checked={effective(p) === "1"} onchange={(e) => set(p, e.currentTarget.checked ? "1" : "0", (e.currentTarget.checked ? 1 : 0) === p.value)}>
              {:else if p.type === "Enumeration"}
                <select aria-label={`${p.name} の値`} value={effective(p)} onchange={(e) => set(p, e.currentTarget.value, Number(e.currentTarget.value) === p.value)}>
                  {#each p.literals ?? [] as l, i}<option value={String(i + 1)}>{l}</option>{/each}
                </select>
              {:else}
                <input value={picked[p.name] ?? ""} placeholder={p.default ?? ""} aria-label={`${p.name} の値`} inputmode="decimal" size="10" aria-invalid={isBad(picked[p.name]) || errs.length > 0} oninput={(e) => set(p, e.currentTarget.value)}>
              {/if}
              {#each errs as d}<small class="err" role="alert">{d.code} {d.message.split("\n")[0]}</small>{/each}
            </td>
            <td><small>{p.default ?? ""}</small></td>
            <td><small>{p.unit ?? ""}</small></td>
            <td><small>{p.description ?? ""}</small></td>
            <td>
              <button type="button" onclick={() => clear(p.name)} disabled={!(p.name in picked)} aria-label={`${p.name} を既定に戻す`}>既定に戻す</button>
              {#if onwrite && PART_NAME.test(p.name)}<button type="button" data-unwrite onclick={() => unwrite(p)} aria-label={`${p.name} の修飾をソースから外す`} title="部品の宣言の修飾 (名前 = 値) を外して、型の既定に戻す（ソースを書き換える）">修飾を外す</button>{/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if matched.length > MAX}<p class="note">先頭の {MAX} 本だけ表示。絞り込みで探す</p>{/if}
  </div>
  <footer><button type="button" onclick={() => dlg.close()}>閉じる</button></footer>
</dialog>

<style>
  .params-dialog { max-width: min(60rem, 95vw); max-height: 85vh; flex-direction: column; background: var(--bg, canvas); color: inherit; border: 1px solid var(--border); }
  .params-dialog[open] { display: flex; }
  .rows { overflow: auto; flex: 1; }
  th { text-align: left; font-weight: normal; padding-right: .5rem; }
  thead th { font-weight: bold; position: sticky; top: 0; background: var(--bg, canvas); }
  td { padding-right: .5rem; vertical-align: top; }
  small, .note { color: var(--muted); }
  .err { color: var(--err); display: block; }
  tr.changed th { font-weight: bold; }
  header, footer { margin: .25rem 0; }
</style>

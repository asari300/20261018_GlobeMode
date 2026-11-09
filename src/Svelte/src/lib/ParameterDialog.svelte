<!-- S248: パラメータのダイアログ。モデルのパラメータ宣言（部品ごとの入れ子の名前・型・既定値・単位）を出し、入れた値を picked（名前 → 字面）に入れる。
     picked は SimSettings の「実行の設定」の parameters になる（ソースは書き換えない）。変えた値だけが入り、既定と同じ値に戻せば消える。 -->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Backend } from "./backend";
  import type { Diagnostic, ModelParameter } from "./protocol";

  let { backend, model, prefix = "", picked = $bindable({}), errors = [], onclose }: {
    backend: Backend;
    model: string;
    prefix?: string; // 開いたときの絞り込み（図の部品なら `c1.`）
    picked?: Record<string, string>;
    errors?: Diagnostic[]; // GM-E10006。メッセージの中の名前で欄を探す
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

  onMount(() => {
    dlg.showModal();
    backend.parameters(model).then(
      (ps) => { all = ps; note = ps.length ? "" : "パラメータは無い"; },
      (e) => { note = `一覧を取れない: ${String((e as Error).message ?? e)}`; },
    );
  });

  const set = (p: ModelParameter, t: string, sameAsDefault = false) => {
    const next = { ...picked };
    if (!t.trim() || sameAsDefault) delete next[p.name];
    else next[p.name] = t;
    picked = next;
  };
  const clear = (name: string) => { const next = { ...picked }; delete next[name]; picked = next; };
  const effective = (p: ModelParameter) => picked[p.name] ?? (p.value != null ? String(p.value) : "");
  const changed = $derived(Object.keys(picked).filter((n) => all.some((p) => p.name === n)).length);
</script>

<dialog bind:this={dlg} class="params-dialog" data-param-dialog aria-label={`${model} のパラメータ`} onclose={onclose}>
  <header>
    <strong>{model} のパラメータ</strong>
    <small>値を入れると実行の設定の parameters に乗る（ソースは変えない）</small>
  </header>
  <p>
    <input bind:value={filter} aria-label="パラメータの絞り込み" placeholder="名前・説明で絞り込む" size="24">
    （{matched.length} / {all.length} 本）
    <button type="button" onclick={() => (picked = {})} disabled={!Object.keys(picked).length} title="このモデルの上書きをすべて消す">すべて既定に戻す</button>
    {#if changed}<small data-changed>変更 {changed} 本</small>{/if}
  </p>
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
            <td><button type="button" onclick={() => clear(p.name)} disabled={!(p.name in picked)} aria-label={`${p.name} を既定に戻す`}>既定に戻す</button></td>
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

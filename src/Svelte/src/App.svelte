<script lang="ts">
  import { onMount } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, drawSelection } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { bracketMatching } from "@codemirror/language";
  import { WorkerBackend } from "./lib/worker-backend";
  import type { Backend } from "./lib/backend";
  import type { Diagnostic } from "./lib/protocol";

  const FILE = "Model.mo";
  const backend: Backend = new WorkerBackend();
  const severityRank = { error: 0, warning: 1, info: 2 };

  let host: HTMLDivElement;
  let view: EditorView;
  let model = $state("Decay");
  let busy = $state(false);
  let failure = $state("");
  let diagnostics = $state<Diagnostic[]>([]);

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n",
        extensions: [lineNumbers(), history(), drawSelection(), bracketMatching(), keymap.of([...defaultKeymap, ...historyKeymap])],
      }),
    });
    return () => view.destroy();
  });

  async function check() {
    busy = true;
    failure = "";
    try {
      await backend.setFile(FILE, view.state.doc.toString());
      const r = await backend.check(model);
      // S0…S10 の順、同じ段階では error > warning > info
      diagnostics = [...r.diagnostics].sort(
        (a, b) => Number(a.stage.slice(1)) - Number(b.stage.slice(1)) || severityRank[a.severity] - severityRank[b.severity],
      );
    } catch (e) {
      failure = String((e as Error).message ?? e);
    } finally {
      busy = false;
    }
  }

  // range は UTF-8 のバイト位置。CodeMirror は UTF-16 の位置なので変換する
  function jump(d: Diagnostic) {
    if (!d.range || (d.file && d.file !== FILE)) return;
    const bytes = new TextEncoder().encode(view.state.doc.toString());
    const at = (n: number) => new TextDecoder().decode(bytes.slice(0, n)).length;
    view.dispatch({ selection: { anchor: at(d.range.start), head: at(d.range.end) }, scrollIntoView: true });
    view.focus();
  }
</script>

<main>
  <div class="bar">
    <label>モデル名 <input bind:value={model} aria-label="モデル名"></label>
    <button onclick={check} disabled={busy}>検査</button>
  </div>
  <div class="editor" bind:this={host}></div>
  {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  <table class="diagnostics" aria-label="診断">
    <tbody>
      {#each diagnostics as d}
        <tr class={d.severity} onclick={() => jump(d)}>
          <td>{d.code}</td><td>{d.stage}</td><td>{d.severity}</td><td>{d.message}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</main>

<style>
  main { font: 14px/1.4 system-ui, sans-serif; max-width: 60rem; margin: 0 auto; padding: 1rem; }
  .bar { display: flex; gap: 1rem; margin-bottom: .5rem; }
  .editor { border: 1px solid #888; }
  .editor :global(.cm-editor) { height: 16rem; }
  .diagnostics { width: 100%; border-collapse: collapse; margin-top: .5rem; }
  td { border-bottom: 1px solid #ddd; padding: .2rem .5rem; vertical-align: top; }
  tr { cursor: pointer; }
  tr.error { color: #b00020; }
  tr.warning { color: #8a6100; }
  .failure { color: #b00020; white-space: pre-wrap; }
</style>

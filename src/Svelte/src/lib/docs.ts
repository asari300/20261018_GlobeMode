import type { Extension } from "@codemirror/state";
import { EditorView, ViewPlugin, type ViewUpdate } from "@codemirror/view";
import type { Backend, Documentation, Icon } from "./backend";

/**
 * 説明パネル（S232）。カーソルのあるいちばん内側のクラスの Documentation を Backend（Rust）に引き、show に渡す。
 * 折りたたみ（editor.ts の modelicaFolding）と同じく、入力かカーソルの移動が 300 ms 止まったら本文を送って取り直し、新しい要求が来たら古い応答は捨てる。
 * ライブラリのクラスは、木で選ぶとそのファイルがエディタに開いて宣言の行にカーソルが置かれるので、同じ道で引ける。
 */
export const modelicaDocs = (backend: Backend, file: () => string, show: (d: Documentation | null, icon: Icon | null) => void): Extension =>
  ViewPlugin.fromClass(
    class {
      timer: ReturnType<typeof setTimeout> | undefined;
      dead = false;
      seq = 0;
      constructor(readonly view: EditorView) { this.soon(); }
      update(u: ViewUpdate) { if (u.docChanged || u.selectionSet) this.soon(); }
      destroy() { this.dead = true; clearTimeout(this.timer); }
      soon() { clearTimeout(this.timer); this.timer = setTimeout(() => void this.fetch(), 300); }
      async fetch() {
        const { doc, selection } = this.view.state, f = file(), mine = ++this.seq;
        let d: Documentation | null = null, icon: Icon | null = null;
        try {
          if (doc.length) { // 空の文書（結果のタブ）は Backend に送らない
            const line = doc.lineAt(selection.main.head);
            await backend.setFile(f, doc.toString());
            const at = { file: f, line: line.number - 1, character: selection.main.head - line.from };
            [d, icon] = await Promise.all([backend.documentation("", at), backend.icon("", at).catch(() => null)]); // アイコンが引けなくても説明は出す（S244）
          }
        } catch { /* 引けなければ「無い」 */ }
        if (!this.dead && mine === this.seq) show(d, icon);
      }
    },
  );

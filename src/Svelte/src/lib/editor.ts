// CodeMirror の拡張（S053）: 予約語と本文の単語の補完、検査と lint の結果の波線。業務ロジックは置かない。
import { autocompletion, completeAnyWord, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { linter, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { Compartment, Prec, type Extension } from "@codemirror/state";
import { EditorView, hoverTooltip, keymap } from "@codemirror/view";
import type { Backend } from "./backend";
import type { Diagnostic } from "./protocol";

// globe-mode-fmt の KEYWORDS と同じ並び
export const KEYWORDS = [
  "algorithm", "and", "annotation", "assert", "block", "break", "class", "connect", "connector", "constant",
  "constrainedby", "der", "discrete", "each", "else", "elseif", "elsewhen", "encapsulated", "end", "enumeration",
  "equation", "expandable", "extends", "external", "false", "final", "flow", "for", "function", "if", "import",
  "impure", "in", "initial", "inner", "input", "loop", "model", "not", "operator", "or", "outer", "output", "package",
  "parameter", "partial", "protected", "public", "pure", "record", "redeclare", "replaceable", "return", "stream",
  "then", "true", "type", "when", "while", "within", "Real", "Integer", "Boolean", "String",
];

function keywords(ctx: CompletionContext): CompletionResult | null {
  const word = ctx.matchBefore(/\w+/);
  if (!word || (word.from === word.to && !ctx.explicit)) return null;
  return { from: word.from, options: KEYWORDS.map((label) => ({ label, type: "keyword" })) };
}

export const modelicaCompletion = (): Extension => autocompletion({ override: [keywords, completeAnyWord] });

/**
 * 定義ジャンプ（S138）: F12 か Ctrl/Cmd+クリックで、その位置の名前の定義へ。引いて動かすのは App（別のファイルなら開く）。
 * Alt+← で、ライブラリのファイルへ入る前の位置に戻る。
 */
export const modelicaDefinition = (
  jump: (line: number, character: number) => Promise<void>,
  back: () => void,
  hover?: { backend: Backend; file: () => string }, // S149: あれば、名前の上にマウスを置くと宣言の 1 行を出す
): Extension => {
  const at = (view: EditorView, pos: number) => {
    const line = view.state.doc.lineAt(pos);
    void jump(line.number - 1, pos - line.from); // CodeMirror の列は UTF-16 で LSP と同じ
  };
  return [
    hover ? modelicaHover(hover.backend, hover.file) : [],
    Prec.high(
      keymap.of([
        { key: "F12", run: (v) => (at(v, v.state.selection.main.head), true) },
        { key: "Alt-ArrowLeft", run: () => (back(), true) }, // defaultKeymap の cursorSyntaxLeft より先に
      ]),
    ),
    EditorView.domEventHandlers({
      mousedown(e, v) {
        const pos = e.ctrlKey || e.metaKey ? v.posAtCoords({ x: e.clientX, y: e.clientY }) : null;
        if (pos === null) return false;
        e.preventDefault();
        at(v, pos);
        return true;
      },
    }),
  ];
};

/** 参照の一覧（Shift+F12）とリネーム（F2）（S161）。キーを受けて、カーソルの位置を渡すだけ。計算は Backend（Rust）。 */
export const modelicaRefactor = (
  references: (line: number, character: number) => Promise<void>,
  rename: (line: number, character: number) => void,
): Extension => {
  const at = (view: EditorView, f: (line: number, character: number) => unknown) => {
    const pos = view.state.selection.main.head;
    const line = view.state.doc.lineAt(pos);
    f(line.number - 1, pos - line.from);
    return true;
  };
  return Prec.high(keymap.of([{ key: "Shift-F12", run: (v) => at(v, references) }, { key: "F2", run: (v) => at(v, rename) }]));
};

/** ホバー（S149）: 300 ms 置くと、その名前の宣言（部品は `Real x "説明"`、クラスは見出し）を素のテキストで出す。 */
const modelicaHover = (backend: Backend, file: () => string): Extension => [
  EditorView.baseTheme({ ".cm-hover": { padding: "2px 6px", maxWidth: "40em", whiteSpace: "pre-wrap", fontFamily: "monospace" } }),
  hoverTooltip(
    async (view, pos) => {
      const line = view.state.doc.lineAt(pos);
      try {
        await backend.setFile(file(), view.state.doc.toString());
        const h = await backend.hover(file(), line.number - 1, pos - line.from);
        if (!h) return null;
        const from = view.state.doc.line(h.range.start.line + 1).from + h.range.start.character;
        const to = view.state.doc.line(h.range.end.line + 1).from + h.range.end.character;
        return {
          pos: from,
          end: to,
          above: true,
          create: () => {
            const dom = document.createElement("div");
            dom.className = "cm-hover";
            dom.textContent = h.contents;
            return { dom };
          },
        };
      } catch {
        return null; // 文書が壊れている・名前が引けないときは何も出さない
      }
    },
    { hoverTime: 300 },
  ),
];

/** UTF-8 のバイト位置（protocol の range）→ UTF-16 の位置（CodeMirror） */
export function byteToChar(text: string): (n: number) => number {
  const bytes = new TextEncoder().encode(text);
  return (n) => new TextDecoder().decode(bytes.slice(0, n)).length;
}

/** 入力が止まったら lint（globe-mode-fmt）を回し、warning の波線を出す。検査（S0〜S10）はボタンのままで、結果は setDiagnostics で載せる。 */
export const modelicaLinter = (backend: Backend, file: string): Extension =>
  linter(
    async (view) => {
      const text = view.state.doc.toString();
      const out: CmDiagnostic[] = [];
      for (const m of await backend.lint(text)) {
        const line = view.state.doc.line(Math.min(Math.max(m.line, 1), view.state.doc.lines));
        const from = Math.min(line.from + Math.max(m.column - 1, 0), line.to);
        out.push({ from, to: line.to, severity: "warning", source: m.rule, message: m.message });
      }
      void file;
      return out;
    },
    { delay: 500 },
  );

/** 検査の診断を CodeMirror の波線にする */
export function toCmDiagnostics(text: string, ds: Diagnostic[], file: string): CmDiagnostic[] {
  const at = byteToChar(text);
  return ds
    .filter((d) => d.range && (!d.file || d.file === file))
    .map((d) => ({
      from: at(d.range!.start),
      to: Math.max(at(d.range!.end), at(d.range!.start) + 1),
      severity: d.severity === "error" ? "error" : d.severity === "warning" ? "warning" : "info",
      source: d.code,
      message: d.message,
    }));
}

/** 配色（S152）。色は theme.css の変数なので、明暗で書き換えない。`dark` だけ CodeMirror の既定の暗い部品（補完・lint の吹き出し）のために差し替える。 */
export const themeSlot = new Compartment();
export const darkSlot = (dark: boolean): Extension => themeSlot.of(EditorView.darkTheme.of(dark));
export const modelicaTheme: Extension = EditorView.theme({
  "&": { backgroundColor: "var(--bg)", color: "var(--fg)" },
  ".cm-gutters": { backgroundColor: "var(--bg)", color: "var(--muted)", borderRight: "1px solid var(--line)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "var(--hover)" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--fg)" },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "var(--sel)" },
  ".cm-tooltip": { backgroundColor: "var(--bg)", color: "var(--fg)", border: "1px solid var(--border)" },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { backgroundColor: "var(--sel)", color: "var(--fg)" },
});

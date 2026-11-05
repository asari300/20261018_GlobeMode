// CodeMirror の拡張（S053）: 予約語と本文の単語の補完、検査と lint の結果の波線。業務ロジックは置かない。
import { autocompletion, completeAnyWord, type CompletionContext, type CompletionResult, type CompletionSource } from "@codemirror/autocomplete";
import { linter, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { HighlightStyle, StreamLanguage, syntaxHighlighting, type StringStream } from "@codemirror/language";
import { Compartment, EditorState, Prec, type Extension } from "@codemirror/state";
import { EditorView, hoverTooltip, keymap } from "@codemirror/view";
import { tags } from "@lezer/highlight";
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

/**
 * 構文の色付け（S163）。StreamLanguage の小さなトークナイザ（予約語・組み込みの型・文字列・数値・コメント・演算子）。
 * 意味は見ない（semantic tokens は別）。予約語は上の KEYWORDS（Rust の KEYWORDS と同じことを test_ui が確かめる）。
 */
const TYPES = new Set(["Real", "Integer", "Boolean", "String"]);
const WORDS = new Set(KEYWORDS);
type Mode = { in: "" | "str" | "cmt" };
function token(stream: StringStream, st: Mode): string | null {
  if (st.in === "str") { // 文字列は複数行にまたがれる
    while (!stream.eol()) {
      const c = stream.next();
      if (c === "\\") stream.next();
      else if (c === '"') { st.in = ""; break; }
    }
    return "string";
  }
  if (st.in === "cmt") {
    if (stream.match(/^.*?\*\//)) st.in = ""; else stream.skipToEnd();
    return "blockComment";
  }
  if (stream.eatSpace()) return null;
  if (stream.match("//")) { stream.skipToEnd(); return "lineComment"; }
  if (stream.match("/*")) { st.in = "cmt"; return token(stream, st); }
  if (stream.next() === '"') { st.in = "str"; return token(stream, st); }
  stream.backUp(1);
  if (stream.match(/^'(?:\\.|[^'\\])*'/)) return "variableName"; // 引用符つきの識別子は予約語に見ない
  if (stream.match(/^[A-Za-z_]\w*/)) { // shortcut: ASCII の識別子だけ（Modelica の識別子は ASCII）。広げるなら Unicode のクラスに
    const w = stream.current();
    return TYPES.has(w) ? "typeName" : WORDS.has(w) ? "keyword" : null;
  }
  if (stream.match(/^(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/)) return "number";
  if (stream.match(/^(?::=|==|<>|<=|>=|\.[-+*\/^]|[-+*\/^<>=])/)) return "operator";
  stream.next();
  return null;
}
const modelicaLanguage = StreamLanguage.define<Mode>({
  startState: () => ({ in: "" }),
  token,
  languageData: { commentTokens: { line: "//", block: { open: "/*", close: "*/" } } },
});
// 色は modelicaTheme の .cm-mo-* が theme.css の --syn-* で付ける（明暗でここは変えない）
const modelicaStyle = HighlightStyle.define([
  { tag: tags.keyword, class: "cm-mo-kw" },
  { tag: tags.typeName, class: "cm-mo-type" },
  { tag: tags.string, class: "cm-mo-str" },
  { tag: tags.number, class: "cm-mo-num" },
  { tag: tags.comment, class: "cm-mo-cmt" },
  { tag: tags.operator, class: "cm-mo-op" },
]);
export const modelicaHighlight: Extension = [modelicaLanguage, syntaxHighlighting(modelicaStyle)];

function keywords(ctx: CompletionContext): CompletionResult | null {
  const word = ctx.matchBefore(/\w+/);
  if (!word || (word.from === word.to && !ctx.explicit)) return null;
  return { from: word.from, options: KEYWORDS.map((label) => ({ label, type: "keyword" })) };
}

// 予約語と本文の単語は即時。Rust 側の候補は modelicaDefinition に渡す backend から languageData で足される（S171）
const completionFrom = (source: CompletionSource): Extension => EditorState.languageData.of(() => [{ autocomplete: source }]); // source は同じ関数を渡し続ける（CodeMirror は関数で区別する）
export const modelicaCompletion = (): Extension => [autocompletion(), completionFrom(keywords), completionFrom(completeAnyWord)];

// LSP の CompletionItemKind → CodeMirror の type
const KIND: Record<number, string> = { 3: "function", 5: "property", 6: "variable", 7: "class", 9: "namespace", 14: "keyword" };

/**
 * Rust 側の補完（S171）。本文を送ってから位置を渡すだけで、候補の計算（型の位置のライブラリのクラス、`.` の後ろの部品、文字列・コメントの中は空）は Backend。
 * 語の頭が変わるまでは引き直さず（validFor）、古い応答は `ctx.aborted` で捨てる。予約語はこちらでは足さない（上の keywords が即時に出す）。
 */
const rustCompletion = (backend: Backend, file: () => string): CompletionSource => async (ctx) => {
  const word = ctx.matchBefore(/\w*/)!;
  const dotted = word.from > 0 && ctx.state.sliceDoc(word.from - 1, word.from) === ".";
  if (word.from === word.to && !ctx.explicit && !dotted) return null;
  const line = ctx.state.doc.lineAt(ctx.pos);
  ctx.addEventListener("abort", () => {}, { onDocChange: true }); // 打鍵で aborted が立つようにする（遅れて届いた古い応答を捨てる）
  try {
    await backend.setFile(file(), ctx.state.doc.toString());
    if (ctx.aborted) return null;
    const items = await backend.completion(file(), line.number - 1, ctx.pos - line.from);
    if (ctx.aborted) return null;
    const options = items
      .filter((i) => i.kind !== 14)
      .map((i) => ({ label: i.label, type: KIND[i.kind ?? 0], detail: i.detail, info: i.documentation }));
    return options.length ? { from: word.from, options, validFor: /^\w*$/ } : null; // 語を打ち足す間は CodeMirror が絞る（候補は打ちかけの語で絞り済みの上位 200 件）
  } catch {
    return null; // 候補が引けなくても、予約語と本文の単語は出ている
  }
};

/**
 * 定義ジャンプ（S138）: F12 か Ctrl/Cmd+クリックで、その位置の名前の定義へ。引いて動かすのは App（別のファイルなら開く）。
 * Alt+← で、ライブラリのファイルへ入る前の位置に戻る。
 */
export const modelicaDefinition = (
  jump: (line: number, character: number) => Promise<void>,
  back: () => void,
  hover?: { backend: Backend; file: () => string }, // S149: あれば、名前の上にマウスを置くと宣言の 1 行を出す（S171: Rust 側の補完もこれで足す）
): Extension => {
  const at = (view: EditorView, pos: number) => {
    const line = view.state.doc.lineAt(pos);
    void jump(line.number - 1, pos - line.from); // CodeMirror の列は UTF-16 で LSP と同じ
  };
  return [
    hover ? modelicaHover(hover.backend, hover.file) : [],
    hover ? completionFrom(rustCompletion(hover.backend, hover.file)) : [], // S171: 補完も同じ backend と file
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
  ".cm-mo-kw": { color: "var(--syn-kw)" },
  ".cm-mo-type": { color: "var(--syn-type)" },
  ".cm-mo-str": { color: "var(--syn-str)" },
  ".cm-mo-num": { color: "var(--syn-num)" },
  ".cm-mo-cmt": { color: "var(--syn-cmt)", fontStyle: "italic" },
  ".cm-mo-op": { color: "var(--muted)" },
  ".cm-tooltip": { backgroundColor: "var(--bg)", color: "var(--fg)", border: "1px solid var(--border)" },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { backgroundColor: "var(--sel)", color: "var(--fg)" },
});

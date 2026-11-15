// S316: マウスなしで結線を足す（部品を選んで `c` → コネクタを Tab・矢印で選んで Enter で始点 → 別の部品のコネクタを選んで Enter で終点）。
// Svelte の DiagramView と VS Code の webview が同じファイルを読む（diagram-select.ts と同じ置き方）。値は渡された図のデータと要素の API だけを使うので node の試験で動かせる。
import type { Diagram } from "./backend";
import type { Sel } from "./diagram-select.ts"; // 拡張子は node の型の除去で読む試験（VS Code の test/）のため

/** 結線を選んでいる最中。list は選べるコネクタ（`部品.コネクタ`）、i は今の候補、from は始点を決めたあと */
export type Wire = { list: string[]; i: number; from?: string } | null;
export interface WireStep {
  wire: Wire;
  /** 画面と読み上げ（`role="status"`）に出す文 */
  say: string;
  /** 終点まで決まったとき。onConnect へそのまま渡す（points は両端の図の座標） */
  connect?: { from: string; to: string; points: [number, number][] };
  /** true ならキーを使った（呼び出し側は preventDefault して他の処理へ回さない） */
  handled: boolean;
}

const endsOf = (d: Diagram, keep: (part: string) => boolean) => d.parts.filter((p) => keep(p.name)).flatMap((p) => (p.connectors ?? []).map((c) => `${p.name}.${c.name}`));
const partOf = (end: string) => end.split(".")[0];
const posOf = (d: Diagram, end: string): [number, number] => {
  const [p, ...c] = end.split(".");
  return d.parts.find((x) => x.name === p)?.connectors?.find((k) => k.name === c.join("."))?.pos ?? [0, 0];
};
const NEXT = ["ArrowRight", "ArrowDown", "Tab"];
const PREV = ["ArrowLeft", "ArrowUp"];

/** 1 回のキーで結線の選びを進める。入るのは、部品を選んでいるときの `c`。Esc で抜ける。Ctrl・Alt・Meta つきのキーは使わない */
export function wireKey(w: Wire, d: Diagram | null | undefined, sel: Sel, e: { key: string; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean }): WireStep {
  const none: WireStep = { wire: w, say: "", handled: false };
  if (e.ctrlKey || e.metaKey || e.altKey || !d) return none;
  const show = (x: NonNullable<Wire>, head: string) => ({ wire: x, say: `${head}: ${x.list[x.i]}（${x.i + 1}/${x.list.length}。Tab か矢印で切り替え、Enter で決める、Esc でやめる）`, handled: true });
  if (!w) {
    if (e.key !== "c" || !sel || !("part" in sel)) return none;
    const list = endsOf(d, (p) => p === sel.part);
    return list.length ? show({ list, i: 0 }, "結線の始点を選ぶ") : { wire: null, say: `${sel.part} にコネクタが無い`, handled: true };
  }
  const n = w.list.length;
  if (e.key === "Escape") return { wire: null, say: "結線をやめた", handled: true };
  if (e.key === "Enter") {
    const cur = w.list[w.i];
    if (w.from) return { wire: null, say: `${w.from} と ${cur} を結線した`, connect: { from: w.from, to: cur, points: [posOf(d, w.from), posOf(d, cur)] }, handled: true };
    const list = endsOf(d, (p) => p !== partOf(cur));
    return list.length ? show({ list, i: 0, from: cur }, `${cur} を始点に選んだ。終点を選ぶ`) : { wire: null, say: "終点にできるコネクタが無い", handled: true };
  }
  const back = PREV.includes(e.key) || (e.key === "Tab" && e.shiftKey);
  if (back || NEXT.includes(e.key)) return show({ ...w, i: (w.i + (back ? n - 1 : 1)) % n }, w.from ? `${w.from} から。終点を選ぶ` : "結線の始点を選ぶ");
  return { wire: w, say: "", handled: true }; // 抜けるまで他のキー（Delete・矢印での移動）は効かせない
}

/** 選び中のコネクタに `.cur`、始点に `.from`、`aria-label` を付け直す（見た目の枠は CSS）。図が作り直されたらもう一度呼ぶ */
export function markWire(svg: Element | null | undefined, w: Wire, say = ""): void {
  svg?.querySelectorAll(".cur, .from").forEach((e) => e.classList.remove("cur", "from"));
  for (const e of w ? [...(svg?.querySelectorAll("[data-end]") ?? [])] : []) {
    const end = e.getAttribute("data-end");
    if (end === w!.from) e.classList.add("from");
    if (end === w!.list[w!.i]) (e.classList.add("cur"), e.setAttribute("aria-label", say));
  }
}

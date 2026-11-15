// S293: 図の部品・結線の選びと削除の対象（S281 で DiagramView に埋まっていたものを、VS Code の webview と共有するために出した）。
// 値は渡された要素の API だけを呼ぶので、node の試験に偽の要素を渡せる（diagram-drag.ts と同じ）。
export type Sel = { part: string } | { key: string } | null;
export type Del = { part: string } | { from: string; to: string };

/** クリック・フォーカスの対象から選びを決める。部品か結線（`[data-hit]`）なら選び、何も無い所のクリックで外す */
export function pickSel(t: Element, type: string, prev: Sel): Sel {
  const part = t.closest("[data-part]")?.getAttribute("data-part");
  const hit = t.closest("[data-hit]")?.getAttribute("data-hit");
  return part != null ? { part } : hit != null ? { key: hit } : type === "click" ? (t.closest(".wp") ? prev : null) : prev; // S299: 途中の点（.wp）のクリックでは外さない
}

/** 選んだ要素に `.sel` を付け直す。図が読み直されて対象が無くなっていたら null を返す */
export function markSel(svg: Element | null | undefined, sel: Sel): Sel {
  svg?.querySelectorAll(".sel").forEach((e) => e.classList.remove("sel"));
  if (!sel) return sel;
  const found = [...(svg?.querySelectorAll("[data-part], [data-conn]") ?? [])].filter((e) => ("part" in sel ? e.getAttribute("data-part") === sel.part : e.getAttribute("data-conn") === sel.key));
  found.forEach((e) => e.classList.add("sel"));
  return found.length ? sel : null;
}

/** Delete / Backspace で消す対象。選びが無ければ null（結線の key は `from to`） */
export function deleteOf(sel: Sel, key: string): Del | null {
  if (!sel || (key !== "Delete" && key !== "Backspace")) return null;
  if ("part" in sel) return { part: sel.part };
  const [from, to] = sel.key.split(" ");
  return { from, to };
}

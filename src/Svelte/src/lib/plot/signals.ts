/** 信号の一覧と書き出しの名前の純粋な関数（S169）。 */

/**
 * コンパイラが作った名前（`$derivative.C1.v`・`$formal_derivative.1.PI.I.y`・`$state_coordinates[1]`）か。隠す判定はここ 1 か所。
 * Modelica の識別子は `$` で始められない（MLS 2.2.2 の NON-DIGIT に `$` が無い）ので、利用者の変数とぶつからない。Rust 側に同じ判定は無い。
 */
export const isInternal = (name: string) => name.startsWith("$");

/** 空白区切りの語がすべて名前に含まれるか（大文字小文字を区別しない）。空の問い合わせは全部に一致。 */
export function matches(name: string, query: string): boolean {
  const n = name.toLowerCase();
  return query.toLowerCase().split(/\s+/).every((w) => n.includes(w));
}

/** `<モデル名の末尾>-<YYYYMMDD-HHMMSS>.<ext>`。ファイル名に使えない文字は `_` にする。モデル名が空なら `result`。 */
export function exportName(model: string, ext: string, now = new Date()): string {
  const p = (v: number) => String(v).padStart(2, "0");
  const stamp = `${now.getFullYear()}${p(now.getMonth() + 1)}${p(now.getDate())}-${p(now.getHours())}${p(now.getMinutes())}${p(now.getSeconds())}`;
  const base = (model.split(".").pop() ?? "").replace(/[^\p{L}\p{N}_-]/gu, "_") || "result";
  return `${base}-${stamp}.${ext}`;
}

/**
 * プロットの代替テキスト（S175。canvas の aria-label）。表示の整形だけで、計算ではない。
 * モデル名、選んだ信号（先頭 5 本の名前、残りは「ほか n 本」）、x 軸の範囲、先頭 5 本の最後の値・最小・最大。
 */
export function plotLabel(model: string, x: string, xs: ArrayLike<number | null>, ys: { name: string; data: ArrayLike<number | null> }[], max = 5): string {
  const stat = (d: ArrayLike<number | null>) => {
    let lo = Infinity, hi = -Infinity, last: number | null = null;
    for (let k = 0; k < d.length; k++) {
      const v = d[k];
      if (v == null || !Number.isFinite(v)) continue;
      lo = Math.min(lo, v), hi = Math.max(hi, v), last = v;
    }
    return { lo, hi, last };
  };
  const f = (v: number | null) => (v == null || !Number.isFinite(v) ? "なし" : String(Number(v.toPrecision(4))));
  const shown = ys.slice(0, max);
  const more = ys.length > max ? `（ほか ${ys.length - max} 本）` : "";
  const sx = stat(xs);
  const head = `${model || "結果"} のプロット。` + (ys.length ? `信号 ${ys.length} 本: ${shown.map((y) => y.name).join("、")}${more}。` : "信号は選ばれていない。");
  const range = sx.last == null ? "" : `${x} の範囲は ${f(sx.lo)} から ${f(sx.hi)}。`;
  const vals = shown.map((y) => { const s = stat(y.data); return `${y.name} は最後 ${f(s.last)}、最小 ${f(s.lo)}、最大 ${f(s.hi)}`; });
  return head + range + (vals.length ? vals.join("。") + "。" : "");
}

/**
 * uPlot の AlignedData の形（1 本目が x、残りが系列）。Backend が返す列もこの形。
 * 有限でない値は null にして線を切る（uPlot は NaN を扱えない）。wasm の列は gapNonFinite で、sidecar の列は JSON で既に null（S076）。
 */
export interface Columns {
  names: string[]; // time を除く
  points: number;
  defaults?: string[]; // 既定で描く信号（Rust が決める。状態、足りなければ出力。S190）。無ければ内部名を除く先頭の 3 本
  data: ArrayLike<number | null>[]; // [time, ...系列]
}

/** 有限な値だけの列は型付きのまま、NaN / Inf があれば null に置き換えた配列にする（uPlot の欠損）。 */
export function gapNonFinite(col: Float64Array): Float64Array | (number | null)[] {
  for (let i = 0; i < col.length; i++) {
    if (!Number.isFinite(col[i])) return Array.from(col, (v) => (Number.isFinite(v) ? v : null));
  }
  return col;
}

/** 見出し行と列を CSV にする（S146）。null（欠損）は NaN と書く（空欄は globe-mode-sim の parse_csv が読めない。#93）。見出しにコンマ・引用符・改行があれば "..." で囲む。 */
export function toCsv(head: string[], cols: ArrayLike<number | null>[]): string {
  const q = (s: string) => (/[",\n]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s);
  const rows = [head.map(q).join(",")];
  for (let r = 0; r < (cols[0]?.length ?? 0); r++) rows.push(cols.map((c) => c[r] ?? "NaN").join(","));
  return rows.join("\n") + "\n";
}

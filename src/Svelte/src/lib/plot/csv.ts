/**
 * uPlot の AlignedData の形（1 本目が x、残りが系列）。Backend が返す列もこの形。
 * 有限でない値は null にして線を切る（uPlot は NaN を扱えない）。parseCsv は常に、列の道は gapNonFinite で。
 */
export interface Columns {
  names: string[]; // time を除く
  points: number;
  data: ArrayLike<number | null>[]; // [time, ...系列]
}

// write_csv の形だけ読む: 見出しの名前は二重引用符で囲み（中の `"` は `""`）、値は数か NaN / Inf / -Inf。
// shortcut: 値の欄に引用符は来ない前提。他のツールの CSV を読むときは globe-mode-sim の parse_csv に合わせる。
function headerNames(line: string): string[] {
  const names: string[] = [];
  for (const m of line.matchAll(/"((?:[^"]|"")*)"|([^,]+)/g)) names.push(m[1] !== undefined ? m[1].replaceAll('""', '"') : m[2]);
  return names;
}

/** 有限な値だけの列は型付きのまま、NaN / Inf があれば null に置き換えた配列にする（uPlot の欠損）。 */
export function gapNonFinite(col: Float64Array): Float64Array | (number | null)[] {
  for (let i = 0; i < col.length; i++) {
    if (!Number.isFinite(col[i])) return Array.from(col, (v) => (Number.isFinite(v) ? v : null));
  }
  return col;
}

function num(s: string): number | null {
  const v = Number(s);
  return Number.isFinite(v) ? v : null; // "NaN" / "Inf" は null、"Inf" は Number() で NaN になる
}

export function parseCsv(text: string): Columns {
  const lines = text.split("\n").filter((l) => l.trim() !== "");
  if (lines.length === 0) throw new Error("CSV が空");
  const names = headerNames(lines[0].replace(/\r$/, ""));
  const data: (number | null)[][] = names.map(() => []);
  for (let i = 1; i < lines.length; i++) {
    const f = lines[i].replace(/\r$/, "").split(",");
    for (let c = 0; c < names.length; c++) data[c].push(num(f[c]));
  }
  return { names: names.slice(1), points: data[0].length, data };
}

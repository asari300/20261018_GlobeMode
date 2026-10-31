/** uPlot の AlignedData の形（1 本目が x、残りが系列。有限でない値は null にして線を切る）。 */
export interface Columns {
  names: string[]; // time を除く
  data: (number | null)[][]; // [time, ...系列]
}

// write_csv の形だけ読む: 見出しの名前は二重引用符で囲み（中の `"` は `""`）、値は数か NaN / Inf / -Inf。
// shortcut: 値の欄に引用符は来ない前提。他のツールの CSV を読むときは globe-mode-sim の parse_csv に合わせる。
function headerNames(line: string): string[] {
  const names: string[] = [];
  for (const m of line.matchAll(/"((?:[^"]|"")*)"|([^,]+)/g)) names.push(m[1] !== undefined ? m[1].replaceAll('""', '"') : m[2]);
  return names;
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
  return { names: names.slice(1), data };
}

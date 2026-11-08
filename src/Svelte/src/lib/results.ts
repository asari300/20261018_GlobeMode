import type { Columns } from "./plot/csv";

/** 「開く」で受け付ける結果のファイル（S222。MAT v4 と CSV。読み分けは Rust の `read_results_bytes`）。 */
export const isResultsFile = (name: string): boolean => /\.(mat|csv)$/i.test(name);

/** `globe/readResults`・`read_results` の {layout, columns}（layout.names は time を含む）を、プロットが使う Columns にする。点が無ければ reject 相当の Error。 */
export function toColumns(layout: { names: string[]; points: number; defaults?: string[] }, data: Columns["data"]): Columns {
  if (layout.points < 1) throw new Error("結果に時系列の点が無い");
  return { names: layout.names.slice(1), points: layout.points, defaults: layout.defaults, data };
}

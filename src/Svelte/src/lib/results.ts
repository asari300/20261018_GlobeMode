import type { Columns } from "./plot/csv";

/** 「開く」で受け付ける結果のファイル（S222。MAT v4 と CSV。読み分けは Rust の `read_results_bytes`）。 */
export const isResultsFile = (name: string): boolean => /\.(mat|csv)$/i.test(name);

/** 「開く」で受け付ける FMU（S229。Desktop だけが回せる。ブラウザ・Android は理由を出す）。 */
export const isFmuFile = (name: string): boolean => /\.fmu$/i.test(name);

/** MAT の `data_1` のパラメータ 1 つ（S236。CSV と FMU の結果には無い）。 */
export interface ResultParam { name: string; value: number; unit?: string; description?: string }
/** 開いた結果の列。MAT のパラメータがあれば parameters に付く（プロットは読まない）。 */
export type ResultColumns = Columns & { parameters?: ResultParam[] };

/** `globe/readResults`・`read_results` の {layout, columns}（layout.names は time を含む）を、プロットが使う Columns にする。点が無ければ reject 相当の Error。 */
export function toColumns(layout: { names: string[]; points: number; defaults?: string[] }, data: Columns["data"], parameters?: ResultParam[]): ResultColumns {
  if (layout.points < 1) throw new Error("結果に時系列の点が無い");
  return { names: layout.names.slice(1), points: layout.points, defaults: layout.defaults, data, ...(parameters?.length ? { parameters } : {}) };
}

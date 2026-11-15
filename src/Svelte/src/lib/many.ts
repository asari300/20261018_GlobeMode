import type { Columns } from "./plot/csv";
import type { ManyComparison, ManyRun, SimulateManyRequest } from "./protocol";

/** 実行の設定で「複数」を選んだときの依頼の分（S323。SimSettings が作り、App が simulateMany に渡す）。 */
export interface ManyConfig {
  solvers: string[];
  parallel: boolean;
  compare?: SimulateManyRequest["compare"];
}

/** 比較の 1 行の要約: 外れた信号の数と信号名の先頭、最大のはみ出し量、最初に外れた時刻（外れていなければ null）。 */
export function pairSummary(p: ManyComparison["pairs"][number]): { excess: number; first: number } | null {
  if (!p.failures.length) return null;
  const firsts = p.failures.flatMap((f) => (f.first_error_time == null ? [] : [f.first_error_time]));
  return { excess: Math.max(...p.failures.map((f) => f.max_excess)), first: firsts.length ? Math.min(...firsts) : NaN };
}

// 時刻 u の値を (t, y) の折れ線から読む。範囲の外は null（欠損）
function resample(t: ArrayLike<number | null>, y: ArrayLike<number | null>, u: Float64Array): (number | null)[] {
  const n = t.length;
  const out: (number | null)[] = new Array(u.length);
  let j = 0;
  for (let k = 0; k < u.length; k++) {
    const x = u[k];
    while (j + 1 < n && (t[j + 1] as number) <= x) j++;
    const x0 = t[j] as number;
    if (x < (t[0] as number) || x > (t[n - 1] as number)) out[k] = null;
    else if (x === x0 || j + 1 >= n) out[k] = y[j];
    else {
      const [x1, y0, y1] = [t[j + 1] as number, y[j], y[j + 1]];
      out[k] = y0 == null || y1 == null ? null : y0 + ((y1 - y0) * (x - x0)) / (x1 - x0);
    }
  }
  return out;
}

/**
 * ソルバごとの列を 1 つの Columns に重ねる（プロットのタブにそのまま載せる）。信号名は `x [bdf]`（信号ごとにソルバが並ぶ）。
 * 時刻は全部の和集合で、その時刻に点が無い実行は折れ線で補う（出力の間隔が同じなら補いは起きない）。点の無い実行（回せなかった）は入れない。
 */
export function mergeRuns(runs: Pick<ManyRun, "solver">[], cols: Columns[]): Columns | null {
  const ok = cols.map((c, i) => ({ c, s: runs[i]?.solver ?? String(i) })).filter(({ c }) => c.points > 0);
  if (!ok.length) return null;
  const u = Float64Array.from(new Set(ok.flatMap(({ c }) => Array.from(c.data[0] as ArrayLike<number>)))).sort();
  const sigs = ok[0].c.names;
  const label = (n: string, s: string) => `${n} [${s}]`;
  const data: Columns["data"] = [u];
  for (const n of sigs) {
    for (const { c } of ok) {
      const at = c.names.indexOf(n);
      data.push(at < 0 ? new Array(u.length).fill(null) : resample(c.data[0], c.data[at + 1], u));
    }
  }
  const d = ok[0].c.defaults;
  return {
    names: sigs.flatMap((n) => ok.map(({ s }) => label(n, s))),
    points: u.length,
    defaults: d?.flatMap((n) => (sigs.includes(n) ? ok.map(({ s }) => label(n, s)) : [])),
    data,
  };
}

/** 最大の相対差の表示（有効数字 3 桁の指数表記）と、閾値 10^-n との大小を文字で（S328）。null は有限でない（NaN など）、undefined は比べた点が無い。 */
export function relText(v: number | null | undefined, thresholdExp: number): string {
  if (v === undefined) return "-";
  if (v === null || !Number.isFinite(v)) return "有限でない";
  return `${v.toExponential(2)}（${v < 10 ** -thresholdExp ? "<" : "≥"} 1e-${thresholdExp}）`;
}

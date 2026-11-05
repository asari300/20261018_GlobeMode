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

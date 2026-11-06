// wasm とネイティブの結果の CSV の突き合わせ（S012。compare.mjs から。S203 で分けた）。

/** 2 つの f64 の間の ulp の数（符号が違えば Infinity）。 */
export function ulps(a, b) {
  if (a === b) return 0;
  if (Math.sign(a) !== Math.sign(b)) return Infinity;
  const buf = new DataView(new ArrayBuffer(16));
  buf.setFloat64(0, Math.abs(a));
  buf.setFloat64(8, Math.abs(b));
  return Number(buf.getBigUint64(0) > buf.getBigUint64(8) ? buf.getBigUint64(0) - buf.getBigUint64(8) : buf.getBigUint64(8) - buf.getBigUint64(0));
}

/**
 * 2 つの CSV を比べる。`rowDiff` は行数の差（行数が違えば値は比べない）、`maxRel` は相対の差の最大。
 * 有限でない値（NaN・Inf・-Inf。`Number("Inf")` は NaN なので数にできない）は、セルの文字列が同じときだけ一致（#11）。
 */
export function compareCsv(a, b, rtol) {
  if (a === null || b === null) return a === b ? { same: true, rows: 0, rowDiff: 0, maxRel: 0, maxUlps: 0 } : { same: false, why: "片方だけ CSV がある" };
  const ra = a.trim().split("\n");
  const rb = b.trim().split("\n");
  if (ra[0] !== rb[0]) return { same: false, why: "列の名前が違う" };
  if (ra.length !== rb.length) return { same: false, rowDiff: Math.abs(ra.length - rb.length), why: `行数が違う（${ra.length - 1} と ${rb.length - 1}）` };
  let maxRel = 0;
  let maxUlps = 0;
  for (let i = 1; i < ra.length; i++) {
    const ca = ra[i].split(",");
    const cb = rb[i].split(",");
    for (let j = 0; j < ca.length; j++) {
      const x = Number(ca[j]);
      const y = Number(cb[j]);
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        if (ca[j] !== cb[j]) {
          maxRel = Infinity;
          maxUlps = Infinity;
        }
        continue;
      }
      const d = Math.abs(x - y);
      const rel = d === 0 ? 0 : d / Math.max(Math.abs(x), Math.abs(y), 1e-12);
      maxRel = Math.max(maxRel, rel);
      maxUlps = Math.max(maxUlps, ulps(x, y));
    }
  }
  return { same: maxRel <= rtol, rows: ra.length - 1, rowDiff: 0, maxRel, maxUlps };
}

/**
 * 既知の差として許すか（#8）。行数の差が 1 以下で、行数が同じなら相対の差が `bound` 以下のときだけ。
 * `c` は [`compareCsv`] の結果。それ以外（列の名前の違い、行数の差が 2 以上、`bound` を超える差）は許さない。
 */
export function withinKnownBound(c, bound) {
  return c.rowDiff !== undefined && c.rowDiff <= 1 && (c.rowDiff > 0 || c.maxRel <= bound);
}

// gapNonFinite（src/lib/plot/csv.ts）の確認。node の型の除去でそのまま読む: node tests/test_columns.mjs
import assert from "node:assert/strict";
import { gapNonFinite } from "../src/lib/plot/csv.ts";

const ok = new Float64Array([1, 2, 3]);
assert.equal(gapNonFinite(ok), ok, "有限だけの列は型付きのまま");
assert.deepEqual(gapNonFinite(new Float64Array([1, NaN, Infinity, -Infinity, 5])), [1, null, null, null, 5]);
console.log("ok");

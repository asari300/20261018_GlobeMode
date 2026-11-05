// gapNonFinite（src/lib/plot/csv.ts）の確認。node の型の除去でそのまま読む: node tests/test_columns.mjs
import assert from "node:assert/strict";
import { gapNonFinite, toCsv } from "../src/lib/plot/csv.ts";
import { exportName, isInternal, matches } from "../src/lib/plot/signals.ts";

const ok = new Float64Array([1, 2, 3]);
assert.equal(gapNonFinite(ok), ok, "有限だけの列は型付きのまま");
assert.deepEqual(gapNonFinite(new Float64Array([1, NaN, Infinity, -Infinity, 5])), [1, null, null, null, 5]);
assert.equal(toCsv(["time", "a,b"], [[0, 1], [2, null]]), 'time,"a,b"\n0,2\n1,\n', "CSV: null は空欄、見出しは必要なら引用");

// 信号の絞り込み・内部名・書き出しの名前（S169）
assert.ok(matches("PI.I.y", "") && matches("PI.I.y", "pi Y") && !matches("PI.I.y", "pi z"), "部分一致は大文字小文字を区別せず、空白区切りは AND");
assert.deepEqual(["$derivative.C1.v", "$formal_derivative.1.PI.I.y", "$state_coordinates[1]", "C1.v", "a$b"].map(isInternal), [true, true, true, false, false]);
const t = new Date(2026, 10, 5, 9, 3, 7);
assert.equal(exportName("Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum", "csv", t), "DoublePendulum-20261105-090307.csv");
assert.equal(exportName("Decay", "png", t), "Decay-20261105-090307.png");
assert.equal(exportName("'a/b:c'", "csv", t), "_a_b_c_-20261105-090307.csv", "使えない文字は _");
assert.equal(exportName("", "csv", t), "result-20261105-090307.csv");
console.log("ok");

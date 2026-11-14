// gapNonFinite（src/lib/plot/csv.ts）の確認。node の型の除去でそのまま読む: node tests/test_columns.mjs
import assert from "node:assert/strict";
import { gapNonFinite, toCsv } from "../src/lib/plot/csv.ts";
import { diagramNodes, snap } from "../src/lib/shape-svg.ts";
import { exportName, isInternal, matches } from "../src/lib/plot/signals.ts";

const ok = new Float64Array([1, 2, 3]);
assert.equal(gapNonFinite(ok), ok, "有限だけの列は型付きのまま");
assert.deepEqual(gapNonFinite(new Float64Array([1, NaN, Infinity, -Infinity, 5])), [1, null, null, null, 5]);
assert.equal(toCsv(["time", "a,b"], [[0, 1], [2, null]]), 'time,"a,b"\n0,2\n1,NaN\n', "CSV: null は NaN（空欄だと「結果を開く」が読めない）、見出しは必要なら引用");

// 信号の絞り込み・内部名・書き出しの名前（S169）
assert.ok(matches("PI.I.y", "") && matches("PI.I.y", "pi Y") && !matches("PI.I.y", "pi z"), "部分一致は大文字小文字を区別せず、空白区切りは AND");
assert.deepEqual(["$derivative.C1.v", "$formal_derivative.1.PI.I.y", "$state_coordinates[1]", "C1.v", "a$b"].map(isInternal), [true, true, true, false, false]);
const t = new Date(2026, 10, 5, 9, 3, 7);
assert.equal(exportName("Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum", "csv", t), "DoublePendulum-20261105-090307.csv");
assert.equal(exportName("Decay", "png", t), "Decay-20261105-090307.png");
assert.equal(exportName("'a/b:c'", "csv", t), "_a_b_c_-20261105-090307.csv", "使えない文字は _");
assert.equal(exportName("", "csv", t), "result-20261105-090307.csv");
// 図の部品のドラッグの吸着（S250）: 2 単位の格子。-0 は 0
assert.deepEqual([0.9, 1.1, -3.2, -0.4, 5].map((v) => snap(v)), [0, 2, -4, 0, 6]);
assert.ok(Object.is(snap(-0.4), 0));
// 図のコネクタのヒット領域（S261）: 部品の <g> の中に circle.conn。data-end は 部品.コネクタ、data-x・data-y は図の座標、cy は y 反転
const part = { name: "r", class: "R", line: 0, transform: [1, 0, 0, 1, 0, 0], bbox: [0, 0, 20, 10], shapes: [], connectors: [{ name: "p", pos: [3, 4] }] };
const conn = diagramNodes({ class: "T", extent: [-50, -50, 50, 50], parts: [part], connections: [], shapes: [] })[0].children.find((n) => n.tag === "circle");
assert.deepEqual([conn.attrs["data-end"], conn.attrs["data-x"], conn.attrs["data-y"], conn.attrs.cy], ["r.p", 3, 4, -4]);
console.log("ok");

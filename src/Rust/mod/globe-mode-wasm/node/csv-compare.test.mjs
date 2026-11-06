// csv-compare.mjs の試験（node --test。mise run wasm:compare が先に回す）。
import test from "node:test";
import assert from "node:assert/strict";
import { compareCsv, withinKnownBound } from "./csv-compare.mjs";

const csv = (...rows) => ["time,x", ...rows].join("\n");

test("同じ値は一致し、小さい差は rtol で分かれる", () => {
  assert.equal(compareCsv(csv("0,1", "1,2"), csv("0,1", "1,2"), 1e-9).same, true);
  const c = compareCsv(csv("0,1", "1,2"), csv("0,1", "1,2.001"), 1e-9);
  assert.equal(c.same, false);
  assert.ok(c.maxRel > 4e-4 && c.maxRel < 5e-4, c.maxRel);
});

test("#11: Inf・-Inf・NaN はセルの文字列が同じときだけ一致する", () => {
  for (const [a, b] of [["Inf", "NaN"], ["Inf", "-Inf"], ["NaN", "-Inf"], ["Inf", "1"], ["NaN", "0"]]) {
    const c = compareCsv(csv(`0,${a}`), csv(`0,${b}`), 1e-9);
    assert.equal(c.same, false, `${a} vs ${b}`);
    assert.equal(c.maxRel, Infinity);
  }
  for (const v of ["Inf", "-Inf", "NaN"]) assert.equal(compareCsv(csv(`0,${v}`), csv(`0,${v}`), 1e-9).same, true, v);
});

test("#8: 既知の差でも、行数の差は 1 まで・同じ行数なら上限まで", () => {
  const base = csv("0,1", "1,2", "2,3");
  const bound = 1e-2;
  // 行数の差 1: 許す
  assert.equal(withinKnownBound(compareCsv(base, csv("0,1", "1,2"), 1e-9), bound), true);
  // 行数の差 2 以上: 許さない
  assert.equal(withinKnownBound(compareCsv(base, csv("0,1"), 1e-9), bound), false);
  // 同じ行数で上限の内側 / 外側
  assert.equal(withinKnownBound(compareCsv(base, csv("0,1", "1,2.001", "2,3"), 1e-9), bound), true);
  assert.equal(withinKnownBound(compareCsv(base, csv("0,1", "1,2.5", "2,3"), 1e-9), bound), false);
  // 列の名前が違う・片方だけ CSV: 許さない
  assert.equal(withinKnownBound(compareCsv(base, base.replace("x", "y"), 1e-9), bound), false);
  assert.equal(withinKnownBound(compareCsv(base, null, 1e-9), bound), false);
});

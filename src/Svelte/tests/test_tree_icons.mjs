// treeIconLoader（src/lib/tree-icons.ts）の確認: memo・同時 4 本・失敗は null。node tests/test_tree_icons.mjs
import assert from "node:assert/strict";
import { treeIconLoader } from "../src/lib/tree-icons.ts";

let running = 0, peak = 0;
const calls = [];
const backend = { icon: async (c) => { calls.push(c); running++; peak = Math.max(peak, running); await new Promise((r) => setTimeout(r, 5)); running--; if (c === "bad") throw new Error("x"); return { class: c, extent: [0, 0, 1, 1], shapes: [] }; } };
const load = treeIconLoader(backend);
const r = await Promise.all(["a", "b", "c", "d", "e", "f", "g", "a", "bad"].map(load));
assert.equal(peak, 4, "同時に引くのは 4 本まで");
assert.equal(calls.length, 8, "同じクラスは 1 回だけ引く");
assert.equal(r[0], r[7], "memo");
assert.equal(r[8], null, "失敗は null");
console.log("tree-icons: ok");

// html-sanitize.ts（S232）の確認: node --test src/Svelte/tests/test_html_sanitize.mjs（node の型の除去で .ts を直接読む）。
// node に DOM は無いので、使う分だけの小さな木（el / txt）を渡す。実際の DOMParser を通した表示は test_ui.py が確かめる。
import assert from "node:assert/strict";
import { test } from "node:test";
import { sanitize } from "../src/lib/html-sanitize.ts";

const txt = (s) => ({ nodeType: 3, nodeName: "#text", nodeValue: s, childNodes: [] });
const el = (name, attrs = {}, ...kids) => ({ nodeType: 1, nodeName: name.toUpperCase(), nodeValue: null, childNodes: kids, getAttribute: (k) => attrs[k] ?? null });
const run = (...kids) => sanitize("", () => ({ nodeType: 1, nodeName: "BODY", nodeValue: null, childNodes: kids }));

test("許可したタグは残り、入れ子も保つ", () => {
  assert.deepEqual(run(el("p", {}, txt("a "), el("b", {}, txt("b")), el("br"))), [
    { tag: "p", href: undefined, children: ["a ", { tag: "b", href: undefined, children: ["b"] }, { tag: "br", href: undefined, children: [] }] },
  ]);
});

test("script・style は中身ごと落とす。onerror などの属性は写さない", () => {
  assert.deepEqual(run(el("script", {}, txt("alert(1)")), el("style", {}, txt("p{}")), el("iframe", { src: "http://x" }, txt("f"))), []);
  assert.deepEqual(run(el("p", { onclick: "x()", style: "color:red", class: "c" }, txt("t"))), [{ tag: "p", href: undefined, children: ["t"] }]);
  const out = JSON.stringify(run(el("img", { src: "x", onerror: "alert(1)", alt: "図" })));
  assert.equal(out, '["図"]', "img は alt の文字列だけ。src も onerror も出ない");
  assert.deepEqual(run(el("img", { src: "modelica://P/a.png" })), [], "alt が無ければ何も出ない");
});

test("知らないタグは外して中身を残す（html・font・div）", () => {
  assert.deepEqual(run(el("html", {}, el("font", { color: "red" }, txt("x")), el("div", {}, txt("y")))), ["x", "y"]);
});

test("a の href は http・https・modelica だけ。それ以外は外して文字だけ残す", () => {
  const a = (href) => run(el("a", { href, target: "_top", onclick: "x()" }, txt("l")));
  assert.deepEqual(a("https://example.com/x"), [{ tag: "a", href: "https://example.com/x", children: ["l"] }]);
  assert.deepEqual(a("http://example.com/"), [{ tag: "a", href: "http://example.com/", children: ["l"] }]);
  assert.equal(a("modelica://Modelica.Blocks")[0].href, "modelica://Modelica.Blocks");
  for (const bad of ["javascript:alert(1)", " JavaScript:alert(1)", "jav\tascript:alert(1)", "data:text/html,<script>1</script>", "vbscript:x", "relative/path.html", "//evil.example", "", undefined]) {
    assert.deepEqual(a(bad), ["l"], String(bad));
  }
});

test("コメントなどの節は無視する", () => {
  assert.deepEqual(run({ nodeType: 8, nodeName: "#comment", nodeValue: "<script>", childNodes: [] }, txt("ok")), ["ok"]);
});

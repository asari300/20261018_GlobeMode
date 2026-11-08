// language-configuration.json の字下げの規則（S220）: 正規表現がコンパイルでき、Modelica の典型の行で増減が当たる。
// 折りたたみは LSP（textDocument/foldingRange）に任せるので folding.markers は持たない。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const conf = JSON.parse(readFileSync(new URL("../language-configuration.json", import.meta.url), "utf8"));
const inc = new RegExp(conf.indentationRules.increaseIndentPattern);
const dec = new RegExp(conf.indentationRules.decreaseIndentPattern);

test("字下げを増やす行", () => {
  for (const l of [
    "model M", "  partial model M", "encapsulated package P", "  function f", "block B", "connector C", "record R", "class K",
    "operator function plus", "  expandable connector EC \"d\"",
    "equation", "  initial equation", "algorithm", "  initial algorithm", "public", "protected",
    "  if x > 0 then", "  elseif y then", "elsewhen z then", "when time > 1 then", "for i in 1:n loop", "while a < b loop", "else",
  ]) assert.match(l, inc, l);
});

test("字下げを増やさない行", () => {
  for (const l of [
    "type Q = Real;", "  model M = Other;", "connector RealInput = input Real \"x\";", "// model M", "  Real x;",
    "y = if a then 1 else 2;", "  extends Base;", "  end M;", "  end for;", "  annotation(", "  der(x) = -x;",
  ]) assert.doesNotMatch(l, inc, l);
});

test("字下げを戻す行", () => {
  for (const l of ["end M;", "  end for;", "end if;", "else", "elseif x then", "elsewhen y then", "equation", "initial equation", "algorithm", "public", "protected"])
    assert.match(l, dec, l);
  for (const l of ["  x = 1;", "model M", "  else_flag = 1;", "endpoint = 1;", "  equationX = 2;"]) assert.doesNotMatch(l, dec, l);
});

test("// の行の Enter は字下げしない", () => {
  const rule = conf.onEnterRules[0];
  assert.match("  // a", new RegExp(rule.beforeText));
  assert.equal(rule.action.indent, "none");
  assert.equal(conf.folding, undefined);
});

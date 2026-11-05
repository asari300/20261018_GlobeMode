// S165: grammar.test.mjs は JS の正規表現で当てるだけなので、VS Code と同じ Oniguruma（vscode-oniguruma の wasm）で同じ文法を当てて確かめる。
// VS Code の API には TextMate の scope を取るものが無い（semantic tokens は別物）ので、vscode-textmate は試験の中だけで使う。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import vscodeOniguruma from "vscode-oniguruma";
import vscodeTextmate from "vscode-textmate";

const require = createRequire(import.meta.url);
const { loadWASM, OnigScanner, OnigString } = vscodeOniguruma;
const { Registry, parseRawGrammar } = vscodeTextmate;

const wasm = readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
const path = new URL("../syntaxes/modelica.tmLanguage.json", import.meta.url);

test("Oniguruma で、予約語・型・数値・文字列・コメントに scope が付く", async () => {
  await loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const registry = new Registry({
    onigLib: Promise.resolve({ createOnigScanner: (p) => new OnigScanner(p), createOnigString: (s) => new OnigString(s) }),
    loadGrammar: async (scope) =>
      scope === "source.modelica" ? parseRawGrammar(readFileSync(path, "utf8"), path.pathname) : null,
  });
  const g = await registry.loadGrammar("source.modelica");
  assert.ok(g, "文法が読めない");
  const lines = ['model Decay "d\\"q" // c', "  Real x = 1.5e-3; /* b", " k */ end if_x x1;"];
  const toks = [];
  let state = null;
  for (const line of lines) {
    const r = g.tokenizeLine(line, state);
    state = r.ruleStack;
    for (const t of r.tokens) toks.push([t.scopes.at(-1), line.slice(t.startIndex, t.endIndex)]);
  }
  const has = (scope, text) => assert.ok(toks.some(([s, t]) => s === scope && t === text), `${scope} ${text} in ${JSON.stringify(toks)}`);
  has("storage.type.class.modelica", "model");
  has("support.type.modelica", "Real");
  has("constant.numeric.modelica", "1.5e-3");
  has("keyword.other.modelica", "end");
  has("keyword.operator.modelica", "=");
  has("string.quoted.double.modelica", "d");
  has("constant.character.escape.modelica", '\\"');
  assert.ok(toks.some(([s, t]) => s === "comment.line.double-slash.modelica" && t.startsWith("//")), "行コメント");
  assert.ok(toks.some(([s, t]) => s === "comment.block.modelica" && t.includes("k")), "行をまたぐ /* */");
  assert.ok(!toks.some(([s, t]) => s.startsWith("keyword") && t === "if"), "識別子 if_x の中の if は予約語にしない");
});

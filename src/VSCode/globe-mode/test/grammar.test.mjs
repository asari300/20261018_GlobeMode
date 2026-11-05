// TextMate 文法（S163）: JSON として読め、正規表現がコンパイルでき、予約語の集合が Rust の KEYWORDS と一致し、小さな例文で各分類が当たる。
// vscode-textmate は使わない。上位の patterns を前から順に当てる小さな走査（begin / end と include だけ）で足りる。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(p, import.meta.url), "utf8");
const grammar = JSON.parse(read("../syntaxes/modelica.tmLanguage.json"));
const pkg = JSON.parse(read("../package.json"));

const rules = (node, out = []) => {
  if (Array.isArray(node)) node.forEach((n) => rules(n, out));
  else if (node && typeof node === "object") {
    for (const k of ["match", "begin", "end"]) if (typeof node[k] === "string") out.push(node[k]);
    Object.values(node).forEach((v) => rules(v, out));
  }
  return out;
};

// patterns を展開して { name, match | begin, end } の並びにする
const flat = (ps) =>
  ps.flatMap((p) => {
    const r = p.include ? grammar.repository[p.include.slice(1)] : p;
    return r.match || r.begin ? [r] : flat(r.patterns);
  });

function scan(text) {
  const top = flat(grammar.patterns);
  const out = [];
  for (let i = 0; i < text.length; ) {
    let best = null;
    for (const r of top) {
      const re = new RegExp(r.match ?? r.begin, "ym");
      re.lastIndex = i;
      const m = re.exec(text);
      if (m && m[0] !== "") { best = { r, m }; break; } // 前にある規則が先（TextMate は位置が同じなら先の規則）
    }
    if (!best) { i++; continue; }
    let len = best.m[0].length;
    if (best.r.begin) { // end まで（無ければ末尾まで）。内側の escape は \\. で飛ばす
      const esc = best.r.patterns?.[0] ? new RegExp(best.r.patterns[0].match, "ym") : null;
      const end = new RegExp(best.r.end, "ym");
      let j = i + len;
      for (; j < text.length; j++) {
        if (esc) { esc.lastIndex = j; const e = esc.exec(text); if (e) { j += e[0].length - 1; continue; } }
        end.lastIndex = j;
        const e = end.exec(text);
        if (e) { j += e[0].length; break; }
      }
      len = j - i;
    }
    out.push([best.r.name, text.slice(i, i + len)]);
    i += len;
  }
  return out;
}

test("package.json の grammars が文法を指し、言語の id が合う", () => {
  const g = pkg.contributes.grammars[0];
  assert.equal(g.language, "modelica");
  assert.equal(g.scopeName, grammar.scopeName);
  assert.equal(JSON.parse(read(`../${g.path}`)).scopeName, "source.modelica");
  assert.ok(pkg.contributes.languages.some((l) => l.id === g.language));
});

test("正規表現がすべてコンパイルできる", () => {
  const all = rules(grammar);
  assert.ok(all.length > 10);
  for (const r of all) assert.doesNotThrow(() => new RegExp(r), r);
});

test("予約語の集合が globe-mode-fmt の KEYWORDS と一致する", () => {
  const rust = read("../../../Rust/mod/globe-mode-fmt/src/lib.rs");
  const want = [...rust.match(/pub const KEYWORDS.*?\];/s)[0].matchAll(/"(\w+)"/g)].map((m) => m[1]).sort();
  const got = grammar.repository.keyword.patterns.flatMap((p) => p.match.match(/\(\?:(.*)\)/)[1].split("|")).sort();
  assert.deepEqual(got, want);
});

test("小さな例文で予約語・型・数値・文字列・コメントが当たる", () => {
  const toks = scan('model Decay "d\\"q" // c\n  Real x = 1.5e-3; /* b\n k */ end if_x x1;');
  const has = (name, text) => assert.ok(toks.some(([n, t]) => n === name && t === text), `${name} ${text} in ${JSON.stringify(toks)}`);
  has("storage.type.class.modelica", "model");
  has("string.quoted.double.modelica", '"d\\"q"');
  has("comment.line.double-slash.modelica", "// c");
  has("support.type.modelica", "Real");
  has("constant.numeric.modelica", "1.5e-3");
  has("comment.block.modelica", "/* b\n k */");
  has("keyword.other.modelica", "end");
  has("keyword.operator.modelica", "=");
  assert.ok(!toks.some(([n, t]) => n.startsWith("keyword") && t === "if"), "識別子 if_x の中の if は予約語にしない");
  assert.ok(!toks.some(([n, t]) => n === "constant.numeric.modelica" && t === "1"), "識別子 x1 の数字は数値にしない");
});

// flat の関数の評価器（S370。`globe-mode-flat` の `fneval`）が、wasm のスタックで溢れないことと、wasm の native の検査で FFT の大きさが決まることの試験。
//
//   node fneval.mjs [--pkg DIR] [--msl DIR]       （wasm:node が回す。--msl の下に MSL があれば、その `realFFTsamplePoints` の本文もそのまま使う）
//
// native は作業中のファイルだけを見る（ブラウザの MSL は見えない。S347）ので、MSL の関数の本文を試験のファイルに入れる。
// 1. FFT の例題（RealFFT1 と同じ大きさの式）: `ns = realFFTsamplePoints(…)` → `nf`・`nfi` → `fi[nfi]` が決まり、native の検査（S0〜S6）が通る
// 2. 最も深く使う形: 再帰 16 段（`MAX_CALL_DEPTH`）× 1 段ごとに括弧 16 段と `if` 8 段の入れ子（`MAX_NEST` 32 の内）。終わる再帰は通り、終わらない再帰は誤りで返る（溢れない）
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
import { defaultPkg } from "./common.mjs";

const args = process.argv.slice(2);
const arg = (name) => (args.includes(name) ? args[args.indexOf(name) + 1] : undefined);
const gm = createRequire(import.meta.url)(join(resolve(arg("--pkg") ?? defaultPkg("node")), "globe_mode_wasm.js"));
const check = (src, model) => {
  const ws = new gm.Workspace();
  ws.set_file("T.mo", src);
  return JSON.parse(ws.check_with(model, "native"));
};

// 1. FFT
const fnInline = `function realFFTsamplePoints
  input SI.Frequency f_max; input SI.Frequency f_resolution; input Integer f_max_factor(min=1)=5; output Integer ns;
protected Integer ns1;
algorithm
  assert(f_resolution > 0, "f_resolution > 0 required");
  assert(f_max > f_resolution, "f_max > f_resolution required");
  ns1 :=2*integer(ceil(f_max*f_max_factor/f_resolution));
  ns :=if mod(ns1, 2) == 0 then ns1 else ns1 + 1;
  while true loop
    ns1 :=ns;
    while mod(ns1,2) == 0 loop ns1 :=div(ns1, 2);end while;
    while mod(ns1,3) == 0 loop ns1 :=div(ns1, 3);end while;
    while mod(ns1,5) == 0 loop ns1 :=div(ns1, 5);end while;
    if ns1 <= 1 then break; end if;
    ns :=ns + 2;
  end while;
end realFFTsamplePoints;`;
const msl = arg("--msl");
const mslFile = msl && join(msl, "Modelica 4.1.0/Math/FastFourierTransform.mo");
const fnMsl = mslFile && existsSync(mslFile) ? readFileSync(mslFile, "utf8").match(/ {2}function realFFTsamplePoints[\s\S]*?end realFFTsamplePoints;/)?.[0] : undefined;
if (mslFile && existsSync(mslFile)) assert.ok(fnMsl, "MSL の realFFTsamplePoints が見つからない");
const fft = (fn) => `package T
  package SI type Frequency = Real(unit = "Hz"); end SI;
  package Modelica package Icons partial function Function end Function; end Icons; end Modelica;
  ${fn}
  model RealFFT1
    parameter SI.Frequency f_max = 4;
    parameter SI.Frequency f_resolution = 0.2;
    final parameter Integer nfi = max(1,min(integer(ceil(f_max/f_resolution))+1,nf));
    final parameter SI.Frequency fi[nfi](each fixed=false);
    Real y(start=0, fixed=true);
  protected
    parameter Integer ns = realFFTsamplePoints(f_max, f_resolution, f_max_factor=5);
    parameter Integer nf = div(ns,2) + 1;
  equation
    der(y) = 1;
  end RealFFT1;
end T;
`;
for (const [what, fn] of [["試験の中の本文", fnInline], ["MSL の本文", fnMsl]]) {
  if (!fn) continue;
  const r = check(fft(fn), "T.RealFFT1");
  assert.ok(r.passed && /native: S0〜S6/.test(r.summary), `${what}: ${JSON.stringify(r)}`);
  console.log(`FFT（${what}）: ${r.summary}`);
}

// 2. 深い再帰。1 段ごとに括弧 16 段と `if` 8 段の入れ子（文と式を合わせた入れ子の上限 32 に近い）
const open = "(".repeat(16), close = ")".repeat(16);
const ifs = (inner) => "if n > -1000 then ".repeat(8) + inner + " end if;".repeat(8);
const rec = (name, body) => `package T
  function ${name} input Integer n; output Integer r; algorithm
    ${ifs(`r := ${open}${body}${close};`)}
  end ${name};
  model M Real x[${name}(${name === "ends" ? 14 : 1})]; equation x = zeros(size(x, 1)); end M;
end T;
`;
// 終わる再帰（15 段）: ends(n) = ends(n - 1) + 1、ends(0) = 1 から ends(14) = 15
const ok = check(rec("ends", "if n <= 0 then 1 else ends(n - 1) + 1"), "T.M");
assert.ok(ok.passed, JSON.stringify(ok));
// 終わらない再帰: 16 段で断る（大きさが決まらず native は GM-U で断る。例外を投げず、溢れない）
const bad = check(rec("endless", "endless(n + 1) + 1"), "T.M");
assert.ok(!bad.passed && bad.diagnostics.some((d) => d.code === "GM-U0003" || /non-constant-dim/.test(JSON.stringify(d))), JSON.stringify(bad));
console.log("深い再帰（16 段 × 括弧 16 段 × if 8 段）: 終わる再帰は通り、終わらない再帰は誤りで返る");

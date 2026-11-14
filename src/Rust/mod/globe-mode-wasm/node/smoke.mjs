// globe-mode-wasm の Node での試験（S012。CI の wasm-node と mise run wasm:node が回す）。
//
//   node smoke.mjs [--pkg DIR] [--msl DIR]
//
// 1. 版とソルバの登録表（JSON）が読める
// 2. GlobeModeValidation.Simulation.Decay（der(x) = -x、x(0) = 1）を bdf・dopri45・auto で回し、
//    t = 1 の x が exp(-1) に合う（bdf は Rumoca の CLI と同じ道。P3 の「bdf が wasm で動く」）
// 3. --msl があれば、MSL を in-memory のライブラリとして一度だけ読み（S037 から分割読み込み: 使うファイルだけを
//    検査のときに読む）、ChuaCircuit（電気）と Rotational.Examples.First（機械）を bdf で回す（無ければ飛ばす）。
//    読み込み後と各モデルの後のメモリ（MiB）と時間を出す
//    （S133）Decay の進捗と、performance.now() の費用（Decay と ChuaCircuit）
// 4. panic の文言が last_panic() で読める（最後に回す。panic の後のモジュールは使えない）
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
import { defaultPkg, readMo, repo } from "./common.mjs";

const args = process.argv.slice(2);
const arg = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const pkg = resolve(arg("--pkg") ?? defaultPkg("node"));
const msl = arg("--msl");

const t0 = performance.now();
const gm = createRequire(import.meta.url)(join(pkg, "globe_mode_wasm.js"));
const ms = (t) => +(performance.now() - t).toFixed(1);
const mb = () => +(gm.memory_bytes() / 2 ** 20).toFixed(0);
console.log(`globe-mode-wasm ${gm.version()}（読み込み ${ms(t0)} ms、Node ${process.version}）`);

// 1. ソルバの登録表
const solvers = JSON.parse(gm.solvers());
const available = solvers.filter((s) => s.status.kind === "available").map((s) => s.name);
assert.ok(available.includes("bdf") && available.includes("dopri45") && available.includes("auto"), available.join());
console.log(`ソルバ: ${solvers.length} 件（実装あり ${available.join(", ")}）`);

// 1b. 結果のファイルを開く（S222）: omc の MAT v4 と CSV が、simulate_columns と同じ列の形になる。読めなければ投げる
const opened = gm.read_results("omc_decay_res.mat", readFileSync(join(repo, "src/Rust/mod/globe-mode-sim/tests/data/omc_decay_res.mat")));
const layout = JSON.parse(opened.layout());
assert.equal(layout.names[0], "time");
assert.ok(layout.points > 1 && opened.column(0).length === layout.points && opened.column(layout.names.length).length === 0);
assert.ok(JSON.parse(opened.parameters()).every((p) => typeof p.name === "string" && typeof p.value === "number")); // S236
opened.free();
const csv = gm.read_results("r.csv", new TextEncoder().encode("time,x\n0,1\n1,0.5\n"));
assert.deepEqual(JSON.parse(csv.layout()), { names: ["time", "x"], points: 2, defaults: ["x"] });
assert.deepEqual([...csv.column(1)], [1, 0.5]);
csv.free();
assert.throws(() => gm.read_results("bad.csv", new TextEncoder().encode("x,y\n1,2\n")), /bad\.csv/);
console.log(`結果を開く: MAT ${layout.points} 点 × ${layout.names.length} 列、CSV、読めないファイルは投げる`);

// 1c. 折りたたみの範囲（S225）: LSP の textDocument/foldingRange と同じ関数。クラスは end の行を残し、equation の節と複数行の annotation も出る
{
  const fw = new gm.Workspace();
  const doc = "model M\n  Real x;\nequation\n  der(x) = -x;\n  annotation(\n    experiment(StopTime = 1));\nend M;\n";
  fw.set_file("F.mo", doc);
  const lines = JSON.parse(fw.folding_ranges("F.mo", doc)).map((r) => [r.startLine, r.endLine]);
  assert.ok([[0, 5], [2, 5]].every(([a, b]) => lines.some((r) => r[0] === a && r[1] === b)), JSON.stringify(lines));
  console.log(`折りたたみ: ${JSON.stringify(lines)}`);
  // 説明（S232）: カーソルのクラスの Documentation（生の HTML。エスケープは戻す）。クラスの外とライブラリに無い名前は null
  const withDoc = 'model D\n  Real x;\n  annotation(Documentation(info = "<p>\\"hi\\"</p>", revisions = "<ul/>"));\nend D;\n';
  const d = JSON.parse(fw.documentation("", withDoc, 1, 3));
  assert.deepEqual(d, { class: "D", info: '<p>"hi"</p>', revisions: "<ul/>" });
  assert.equal(fw.documentation("", withDoc, 9, 0), "null");
  assert.equal(fw.documentation("Nope", "", 0, 0), "null");
  // アイコン（S244）: 形のデータ。%name は置き換わり、クラスの外は null
  const withIcon = 'model I\n  Real x;\n  annotation(Icon(graphics = {Rectangle(extent = {{-100, -100}, {100, 100}}, fillColor = {255, 255, 255}, fillPattern = FillPattern.Solid), Text(extent = {{-100, 100}, {100, 140}}, textString = "%name")}));\nend I;\n';
  const ic = JSON.parse(fw.icon("", withIcon, 1, 3));
  assert.deepEqual(ic.shapes.map((s) => s.kind), ["rect", "text"]);
  assert.equal(ic.shapes[1].text, "I");
  assert.equal(fw.icon("", withIcon, 9, 0), "null");
  // 図（S245）: 部品と connect。クラスの外は null
  const withDiagram = 'model D\n  Box a annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));\n  Box b annotation(Placement(transformation(extent = {{20, -10}, {40, 10}})));\nequation\n  connect(a.p, b.p);\nend D;\nmodel Box\n  annotation(Icon(graphics = {Rectangle(extent = {{-100, -100}, {100, 100}})}));\nend Box;\n';
  const dg = JSON.parse(fw.diagram("", withDiagram, 4, 3));
  assert.deepEqual(dg.parts.map((p) => p.name), ["a", "b"]);
  assert.equal(dg.connections.length, 1);
  assert.equal(fw.diagram("", withDiagram, 99, 0), "null");
  const mv = JSON.parse(fw.move_part("D", withDiagram, "b", 2, 0)); // S250: origin が無ければ足す
  assert.match(mv[0].newText, /^origin = \{2, 0\}, $/);
  // パラメータのダイアログ（S248）: 部品ごとの入れ子の名前・型・修飾で変えた既定値・単位。変換できなければ例外
  const withParams = 'model P\n  Part c(k = 5, on = false);\nend P;\nmodel Part\n  parameter Real k(unit = "1/s") = 1 "rate";\n  parameter Boolean on = true;\n  parameter Integer n = 3;\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -k * x;\nend Part;\n';
  fw.set_file("P.mo", withParams);
  const pr = Object.fromEntries(JSON.parse(fw.parameters("P")).parameters.map((p) => [p.name, p]));
  assert.deepEqual([pr["c.k"].type, pr["c.k"].value, pr["c.k"].unit, pr["c.on"].type, pr["c.on"].value, pr["c.n"].value], ["Real", 5, "1/s", "Boolean", 0, 3], JSON.stringify(pr));
  assert.throws(() => fw.parameters("Nope"));
  fw.free();
}

// 2. Decay
const ws = new gm.Workspace();
for (const [name, text] of Object.entries(readMo(join(repo, "src/Modelica/GlobeModeValidation"), repo))) {
  ws.set_file(name, text);
}
const decay = "GlobeModeValidation.Simulation.Decay";
for (const solver of ["bdf", "dopri45", "auto"]) {
  const t = performance.now();
  const r = JSON.parse(ws.simulate(JSON.stringify({ model: decay, settings: { solver } })));
  const wall = ms(t);
  assert.ok(r.ok, `${solver}: ${JSON.stringify(r.diagnostics)}`);
  assert.equal(r.settings.solver, solver);
  assert.equal(r.termination.kind, "stop_time");
  const x = r.final.x;
  const err = Math.abs(x - Math.exp(-1));
  assert.ok(err < 1e-6, `${solver}: x(1) = ${x}`);
  const last = r.csv.trim().split("\n").at(-1).split(",");
  assert.equal(Number(last[0]), 1);
  assert.equal(Number(last[1]), x);
  const p = Object.entries(r.timings_ms).map(([k, v]) => `${k} ${v.toFixed(1)}`).join(" / ");
  console.log(`${decay} ${solver}: x(1) = ${x}（誤差 ${err.toExponential(1)}、${wall} ms: ${p} ms、${r.stats.points} 行）`);
}
// 2a. 読み取りの口と summary（S191。Python の outline・deps・flat、S184 の summary と同じ形）
{
  const o = JSON.parse(ws.outline("src/Modelica/GlobeModeValidation/Simulation/Decay.mo"));
  assert.equal(o.within, "GlobeModeValidation.Simulation");
  assert.throws(() => ws.outline("No/Such.mo"), /ワークスペースに無い/);
  assert.ok(JSON.parse(ws.deps(decay)).some((p) => p.endsWith("Decay.mo")));
  assert.ok(JSON.parse(ws.flat(decay)).variables >= 1);
  assert.equal(JSON.parse(ws.flat("No.Such.Model")), null);
  assert.equal(JSON.parse(ws.check(decay)).summary, `${decay}: OK（S0〜S10 を通過）`);
  console.log("outline・deps・flat・summary: 読めた");
}
// 2b. 進捗（S133、ADR-0033）: simulate_columns に関数を渡すと段と t を受け、列は渡さないときと同じ
{
  const req = JSON.stringify({ model: decay, settings: { solver: "esdirk34" } });
  const seen = [];
  const cols = (c) => { try { return [c.column(0), c.column(1)]; } finally { c.free(); } };
  const plain = cols(ws.simulate_columns(req));
  const observed = cols(ws.simulate_columns(req, (stage, time, start, stop) => seen.push({ stage, time, start, stop })));
  assert.deepEqual(observed, plain);
  // S190: layout.defaults は既定で描く信号（状態）。names（time が先頭）の部分集合
  const dc = ws.simulate_columns(req);
  const lay = JSON.parse(dc.layout());
  dc.free();
  assert.deepEqual(lay.defaults, ["x"]);
  assert.ok(lay.defaults.every((n) => lay.names.includes(n)));
  assert.equal(seen[0].stage, "lowering");
  assert.deepEqual(seen.at(-1), { stage: "integration", time: 1, start: 0, stop: 1 });
  // 関数が true を返すと止まり、そこまでの列と GM-I11004 を返す（S157）。push の戻り値（数）のような真の値では止まらない
  ws.set_file("Long.mo", "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation (experiment(StopTime = 1000));\nend Long;\n");
  const cut = ws.simulate_columns(JSON.stringify({ model: "Long", settings: { solver: "esdirk34" } }), (stage, time) => time !== undefined);
  try {
    const r = JSON.parse(cut.response);
    const t = cut.column(0).at(-1);
    assert.ok(r.ok && r.diagnostics.some((d) => d.code === "GM-I11004") && r.termination.kind === "cancelled");
    assert.ok(t > 0 && t < 1000 && t === r.termination.time, `止めた時刻 ${t}`);
    console.log(`中断: 進捗の関数が true を返して t = ${t} で止まり、${cut.column(0).length} 点と GM-I11004 を受けた`);
  } finally {
    cut.free();
  }
  console.log(`進捗: ${seen.length} 回（${[...new Set(seen.map((p) => p.stage))].join(" → ")}、最後の t = ${seen.at(-1).time}）、列は渡さないときと同じ`);
}

// performance.now() の費用（ADR-0033 の持ち越し）: 実行系は積分器の 1 歩ごとに now を呼ぶ。呼ぶ回数を数え、
// 本物の now と、定数を返す now（間引きの判定だけが変わる）で同じモデルを回して中央値の差を見る
function nowCost(target, request, runs = 7) {
  const real = performance.now.bind(performance);
  let calls = 0;
  const time = (now) => {
    const walls = [];
    for (let i = 0; i <= runs; i++) {
      performance.now = now;
      const t = real();
      target.simulate_columns(request).free();
      if (i > 0) walls.push(real() - t);
      performance.now = real;
    }
    return walls.sort((a, b) => a - b)[walls.length >> 1];
  };
  time(() => (calls++, real()));
  const n = Math.round(calls / (runs + 1));
  const withNow = time(real), constant = time(() => 0);
  const per = (() => { const t = real(); for (let i = 0; i < 1e6; i++) performance.now(); return (real() - t) / 1e6 * 1e6; })(); // ns
  return `now ${n} 回/実行、中央値 ${withNow.toFixed(1)} ms（定数の now では ${constant.toFixed(1)} ms、差 ${(withNow - constant).toFixed(1)} ms）、JS の performance.now() 1 回 ${per.toFixed(0)} ns`;
}
console.log(`performance.now() の費用 Decay esdirk34: ${nowCost(ws, JSON.stringify({ model: decay, settings: { solver: "esdirk34" } }))}`);
console.log(`wasm の線形メモリ ${mb()} MiB`);

// 3. MSL
if (msl && existsSync(msl)) {
  let t = performance.now();
  const files = readMo(resolve(msl), repo);
  const read = ms(t);
  t = performance.now();
  const lib = new gm.Workspace(JSON.stringify(files), "MSL");
  const load = ms(t);
  console.log(`MSL: ${Object.keys(files).length} ファイル（読み ${read} ms、in-memory のライブラリ ${load} ms、メモリ ${mb()} MiB）`);
  // S167: ファイルの中の入れ子のクラス（Blocks の package.mo の Examples）が木の子になり、名前の部分一致で見つかる
  const kids = JSON.parse(lib.library_tree("Modelica.Blocks")).map((e) => e.name);
  assert.ok(kids.includes("Examples") && kids.includes("Sources"), kids.join());
  t = performance.now();
  const hits = JSON.parse(lib.library_search("PID_Con", 50));
  const q1 = ms(t);
  t = performance.now();
  const many = JSON.parse(lib.library_search("s", 50));
  assert.equal(hits[0].path, "Modelica.Blocks.Examples.PID_Controller");
  assert.equal(JSON.parse(lib.library_search("modelica.blocks.sources.sine", 5))[0].path, "Modelica.Blocks.Sources.Sine");
  // S186: 入れ子のクラスを開くとき、宣言の行（0 始まり）が range で分かる（Blocks/package.mo の BusUsage は 990 行目）
  const src = JSON.parse(lib.library_source("Modelica.Blocks.Examples.BusUsage"));
  assert.ok(src.range.start.line > 900 && src.text.split("\n")[src.range.start.line].includes("model BusUsage"), JSON.stringify(src.range));
  // S232: 説明。MSL の PID の Documentation の生の HTML（リンクの引用符のエスケープは戻っている）
  const pidDoc = JSON.parse(lib.documentation("Modelica.Blocks.Continuous.PID", "", 0, 0));
  assert.ok(pidDoc.info.includes("text-book version of a PID controller") && pidDoc.info.includes('<a href="modelica://Modelica.Blocks.Types.Init">'), pidDoc.info.slice(0, 120));
  assert.equal(lib.documentation("Modelica.Nope", "", 0, 0), "null");
  // S244: アイコン。Gain は三角形（polygon）を自分で持ち、基底（Interfaces.SISO）を 1 段重ねても壊れない
  const gain = JSON.parse(lib.icon("Modelica.Blocks.Math.Gain", "", 0, 0));
  assert.ok(gain.shapes.some((s) => s.kind === "polygon" && s.points.length >= 3), JSON.stringify(gain).slice(0, 200));
  assert.equal(lib.icon("Modelica.Nope", "", 0, 0), "null");
  // S245: 図。ChuaCircuit は部品 7・connect 9（Line つき）で、部品の型の Icon が引ける
  const chuaDg = JSON.parse(lib.diagram("Modelica.Electrical.Analog.Examples.ChuaCircuit", "", 0, 0));
  assert.equal(chuaDg.parts.length, 7, JSON.stringify(chuaDg.parts.map((p) => p.name)));
  assert.equal(chuaDg.connections.length, 9);
  assert.ok(chuaDg.parts.every((p) => p.shapes.length > 0), "全部品のアイコンが引けた");
  assert.equal(lib.diagram("Modelica.Nope", "", 0, 0), "null");
  // S248: パラメータの一覧。ChuaCircuit は入れ子の名前（C1.C など）で出て、時間を測る
  let tp = performance.now();
  const chuaPs = JSON.parse(lib.parameters("Modelica.Electrical.Analog.Examples.ChuaCircuit")).parameters;
  console.log(`パラメータの一覧 ChuaCircuit: ${chuaPs.length} 本、${(performance.now() - tp).toFixed(0)} ms`);
  assert.ok(chuaPs.some((p) => p.name === "C1.C" && p.type === "Real" && p.unit === "F" && p.value > 0), JSON.stringify(chuaPs.slice(0, 3)));
  console.log(`MSL の木と検索: Blocks の子 ${kids.length} 件、検索 PID_Con ${q1} ms、s（${many.length} 件）${ms(t)} ms`);
  // S171: 補完。型の位置の `Modelica.Blocks.Sources.` で Sine が出る・`.` の後ろで部品名が出る・文字列の中は空。応答時間も測る。
  // どれも打ちかけで文書は壊れている（カーソルの行を除いて読む）
  const complete = (doc, line, ch) => {
    lib.set_file("C.mo", doc);
    const t0 = performance.now();
    const items = JSON.parse(lib.completion("C.mo", doc, line, ch));
    return [items.map((i) => i.label), performance.now() - t0];
  };
  const [sources, c1] = complete("model C\n  Modelica.Blocks.Sources.\n  Real x;\nend C;\n", 1, 26);
  assert.ok(sources.includes("Sine") && sources.includes("Step"), sources.join());
  const [members, c2] = complete("model C\n  Modelica.Blocks.Sources.Sine s;\nequation\n  s.\nend C;\n", 3, 4);
  assert.ok(members.includes("y"), members.join()); // Sine が継承した出力 y
  const c2b = complete("model C\n  Modelica.Blocks.Sources.Sine s;\nequation\n  s.\nend C;\n", 3, 4)[1]; // 読んだファイルのアウトラインは持つので 2 回目は速い
  assert.deepEqual(complete("model C // Sin\nend C;\n", 0, 13)[0], []);
  const [top, c3] = complete("model C\n  Real x;\n  \nend C;\n", 2, 2);
  assert.ok(top.includes("Modelica") && top.includes("model") && top.includes("x") && top.length <= 200, top.length);
  console.log(`MSL の補完: Sources. ${sources.length} 件 ${c1.toFixed(1)} ms、s. ${members.length} 件 ${c2.toFixed(1)} ms（2 回目 ${c2b.toFixed(1)} ms）、最上位 ${top.length} 件 ${c3.toFixed(1)} ms`);
  for (const [model, check, shown] of [
    ["Modelica.Electrical.Analog.Examples.ChuaCircuit", (f) => Number.isFinite(f["C1.v"]), "C1.v"],
    ["Modelica.Mechanics.Rotational.Examples.First", (f) => Number.isFinite(f["inertia1.w"]), "inertia1.w"],
  ]) {
    t = performance.now();
    const r = JSON.parse(lib.simulate(JSON.stringify({ model, settings: { solver: "bdf" }, no_csv: true })));
    const wall = ms(t);
    assert.ok(r.ok, `${model}: ${JSON.stringify(r.diagnostics.filter((d) => d.severity === "error"))}`);
    assert.ok(check(r.final), JSON.stringify(r.final));
    const p = Object.entries(r.timings_ms).map(([k, v]) => `${k} ${v.toFixed(0)}`).join(" / ");
    console.log(`${model} bdf: ${wall} ms（${p} ms）、${shown}(T) = ${r.final[shown]}、メモリ ${mb()} MiB`);
  }
  const chua = JSON.stringify({ model: "Modelica.Electrical.Analog.Examples.ChuaCircuit", settings: { solver: "bdf" } });
  console.log(`performance.now() の費用 ChuaCircuit bdf: ${nowCost(lib, chua, 3)}`);
} else if (msl) {
  console.log(`MSL（${msl}）が無いので MSL の例題は飛ばす`);
}

// 4. panic の文言
assert.equal(gm.last_panic(), undefined);
assert.throws(() => gm.panic_for_test("試験の panic"), WebAssembly.RuntimeError);
assert.match(gm.last_panic(), /試験の panic/);
console.log(`panic: ${gm.last_panic().split("\n")[0]} …（last_panic で読めた）`);
console.log("ok");

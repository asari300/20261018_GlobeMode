// tauri-backend.ts の純粋な部分（S196）。node の型の除去で .ts を直接読む。実機（Tauri）は要らない。
import assert from "node:assert/strict";
import { TauriBackend, dirOf, tabKey } from "../src/lib/tauri-backend.ts";

// Windows でドライブの根に書いても outDir は "C:\"（"C:" はドライブ相対）
for (const [path, want] of [["C:\\Foo.fmu", "C:\\"], ["C:\\a\\Foo.fmu", "C:\\a"], ["/a/Foo.fmu", "/a"], ["\\\\server\\share\\Foo.fmu", "\\\\server\\share"], ["/Foo.fmu", "/"]]) {
  assert.equal(dirOf(path), want, path);
}

// タブの key: 同じ名前の別のパスは別のタブ、同じパスは同じタブ
assert.equal(tabKey("a.mo", undefined, []), "a.mo");
const p1 = { file: "package.mo", path: "/d1/package.mo" };
assert.equal(tabKey("package.mo", "/d2/package.mo", [p1]), "d2/package.mo");
assert.equal(tabKey("package.mo", "/d1/package.mo", [p1]), "package.mo");
const p2 = { file: "d2/package.mo", path: "/d2/package.mo" };
assert.equal(tabKey("package.mo", "/d2/package.mo", [p2]), "d2/package.mo"); // d1 を閉じたあとに d2 を開き直しても二重にならない
assert.equal(tabKey("package.mo", "C:\\x\\d2\\package.mo", [p1, { file: "d2/package.mo", path: "/other/d2/package.mo" }]), "package.mo (2)");
assert.equal(tabKey("M.mo", "/d/M.mo", [{ file: "M.mo" }]), "M.mo"); // 保存前のタブ（パス無し）とは重ねる

// TauriBackend: パスを名前で覚えない（名前が同じ別のファイルの保存先を取り違えない）
const writes = [];
let saveTo = "/picked/x.mo";
globalThis.window = { __TAURI__: { dialog: { open: async () => null, save: async () => saveTo }, fs: { writeTextFile: async (p, t) => writes.push([p, t]), readTextFile: async () => "" }, core: { invoke: async (cmd, a) => (cmd === "write_recent" ? writes.push([a.path, a.text]) && null : "") } } }; // #67: path があるときの上書きは Rust の write_recent
const b = new TauriBackend();
assert.equal(await b.saveFile("package.mo", "A", false, "/d1/package.mo"), "/d1/package.mo");
assert.equal(await b.saveFile("package.mo", "B", false), "/picked/x.mo"); // パスが無ければ、前に保存した /d1 には書かずダイアログ
assert.equal(await b.saveFile("package.mo", "C", true, "/d1/package.mo"), "/picked/x.mo"); // 名前を付けて保存は選び直す
saveTo = null;
assert.equal(await b.saveFile("package.mo", "D"), null);
assert.deepEqual(writes, [["/d1/package.mo", "A"], ["/picked/x.mo", "B"], ["/picked/x.mo", "C"]]);
// 結果のファイルを開く（S222）: .csv は sidecar の globe/readResults に path を渡し、列は simulateColumns と同じ形（time を除いた names）で返る
const calls = [];
globalThis.window.__TAURI__.dialog.open = async () => "/r/out.csv";
globalThis.window.__TAURI__.core.invoke = async (cmd, a) => (calls.push([cmd, a]), { layout: { names: ["time", "x"], points: 2, defaults: ["x"] }, columns: [[0, 1], [1, null]] });
const opened = await b.openFile();
assert.deepEqual(calls, [["rpc", { method: "globe/readResults", params: { path: "/r/out.csv" } }]]);
assert.deepEqual(opened, { name: "out.csv", text: "", path: "/r/out.csv", columns: { names: ["x"], points: 2, defaults: ["x"], data: [[0, 1], [1, null]] } });
// MAT のパラメータ（S236）と、パスからの読み直し（最近使ったファイル・再読み込み）
globalThis.window.__TAURI__.core.invoke = async () => ({ layout: { names: ["time", "x"], points: 2 }, columns: [[0, 1], [1, 0.5]], parameters: [{ name: "k", value: 2, unit: "1/s" }] });
assert.deepEqual((await b.readResultsAt("/r/out.mat")).parameters, [{ name: "k", value: 2, unit: "1/s" }]);
globalThis.window.__TAURI__.core.invoke = async () => ({ layout: { names: ["time", "x"], points: 2 }, columns: [[0, 1], [1, 0.5]], parameters: [] });
assert.equal("parameters" in (await b.readResultsAt("/r/out.csv")), false);
// FMU（S229）: 開くときは読まず（text は空）、確認のあと fmuInfo・runFmu が sidecar の globe/fmuInfo・globe/fmuRun を呼ぶ。止めた呼び出しは cancelled で reject する
calls.length = 0;
globalThis.window.__TAURI__.dialog.open = async () => "/r/D.fmu";
assert.deepEqual(await b.openFile(), { name: "D.fmu", text: "", path: "/r/D.fmu" });
assert.deepEqual(calls, []);
globalThis.window.__TAURI__.event = { listen: async () => () => {} };
globalThis.window.__TAURI__.core.invoke = async (cmd, a) => (calls.push([cmd, a]), cmd === "rpc" && a.method === "globe/fmuRun" ? { layout: { names: ["time", "x"], points: 2 }, columns: [[0, 1], [1, 0.5]] } : { model_name: "D" });
assert.equal((await b.fmuInfo("/r/D.fmu")).model_name, "D");
assert.deepEqual((await b.runFmu("/r/D.fmu")).names, ["x"]);
assert.deepEqual(calls.map(([, a]) => a.method), ["globe/fmuInfo", "globe/fmuRun"]);
// 実行の設定（S236）: request がそのまま globe/fmuRun の params.request に乗る（省けば {}）
const sent = calls.filter(([, a]) => a.method === "globe/fmuRun").map(([, a]) => a.params.request);
await b.runFmu("/r/D.fmu", { kind: "me", stop_time: 2, step: 0.1, timeout: 30 });
assert.deepEqual(sent, [{}]);
assert.deepEqual(calls.at(-1)[1].params.request, { kind: "me", stop_time: 2, step: 0.1, timeout: 30 });
// 図の部品の移動（S250）: globe/movePart に class・part・dx・dy と file を渡す（RPC_METHODS にある method）
calls.length = 0;
globalThis.window.__TAURI__.core.invoke = async (cmd, a) => (calls.push([cmd, a]), null);
assert.equal(await b.movePart("T", "a", 2, -4, { file: "T.mo" }), null);
assert.deepEqual(calls, [["rpc", { method: "globe/movePart", params: { file: "T.mo", class: "T", part: "a", dx: 2, dy: -4 } }]]);
// 図の結線の追加（S261）: globe/addConnect に class・from・to・points と file を渡す
calls.length = 0;
assert.equal(await b.addConnect("T", "a.p", "b.n", [[0, 0], [10, 5]], { file: "T.mo" }), null);
assert.deepEqual(calls, [["rpc", { method: "globe/addConnect", params: { file: "T.mo", class: "T", from: "a.p", to: "b.n", points: [[0, 0], [10, 5]] } }]]);
// 図の部品の追加（S265）: globe/addPart に class・type・x・y と file を渡す
calls.length = 0;
assert.equal(await b.addPart("T", "L.Res", 12, -4, { file: "T.mo" }), null);
assert.deepEqual(calls, [["rpc", { method: "globe/addPart", params: { file: "T.mo", class: "T", type: "L.Res", x: 12, y: -4 } }]]);
globalThis.window.__TAURI__.core.invoke = async (cmd) => { if (cmd === "cancel_run") return; throw "中断した"; };
const running = b.runFmu("/r/D.fmu");
b.cancel();
await assert.rejects(running, (e) => e.cancelled === true);
console.log("test_tauri ok");

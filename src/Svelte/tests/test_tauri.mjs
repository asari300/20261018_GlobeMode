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
console.log("test_tauri ok");

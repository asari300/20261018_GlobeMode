// パラメータの入力の途中で target が変わっても、開始時のクラスに書く（変わった文書には書かない）。#117
import { test } from "node:test";
import assert from "node:assert/strict";
import Module, { createRequire } from "node:module";

const vscode = { Uri: { joinPath() {} }, window: {}, workspace: { applyEdit: async () => true }, WorkspaceEdit: class { set() {} }, Position: class {}, Range: class {} };
const load = Module._load;
Module._load = function (r, ...a) { return r === "vscode" ? vscode : load.call(this, r, ...a); };
const { Panel } = createRequire(import.meta.url)("../panel.js");
Module._load = load;

const doc = (name) => ({ version: 1, uri: { toString: () => `file:///${name}.mo` } });
const setup = () => {
  const calls = [];
  let release;
  const gate = new Promise((r) => (release = r));
  const client = {
    protocol2CodeConverter: { asTextEdits: async (e) => e },
    async sendRequest(m, a) {
      if (m === "globe/parameters") { await gate; return { parameters: [{ name: "R1.R", type: "Real", default: "1" }] }; }
      calls.push([m, a.file, a.class]);
      return [{ range: 0, newText: "x" }];
    },
  };
  const p = new Panel({}, () => client);
  p.ui = { showErrorMessage() {}, showInformationMessage() {}, showQuickPick: async (items) => items[0], showInputBox: async () => "99" };
  return { p, calls, release };
};

test("応答待ちの間に target が差し替わっても、書くのは開始時のクラス", async () => {
  const { p, calls, release } = setup();
  const a = doc("A");
  p.target = { doc: a, class: "ModelA" };
  const run = p.editParams("R1");
  p.target = { doc: doc("B"), class: "ModelB" }; // refresh が差し替える
  release();
  assert.equal(await run, true);
  assert.deepEqual(calls, [["globe/setModifier", "file:///A.mo", "ModelA"]]);
});

test("開始時の文書の版が変わっていれば入れない", async () => {
  const { p, calls, release } = setup();
  const a = doc("A");
  p.target = { doc: a, class: "ModelA" };
  const run = p.editParams("R1");
  a.version++;
  release();
  assert.equal(await run, false);
  assert.deepEqual(calls, []);
  assert.match(p.why, /文書が変わった/);
});

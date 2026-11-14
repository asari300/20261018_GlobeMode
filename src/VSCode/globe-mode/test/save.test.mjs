// 保存時の整形（S060）: globeMode.formatOnSave が true の .mo だけ、整形を頼む。vscode と LSP クライアントは差し替える。
import { test } from "node:test";
import assert from "node:assert/strict";
import Module, { createRequire } from "node:module";

test("保存時の整形と、globe/format への振り替え", async () => {
  let onWillSave;
  let ext;
  const calls = [];
  const settings = {
    globeMode: { formatOnSave: false, blankAroundSections: true, formatHtml: true, libraries: [], path: "globe" },
    editor: { tabSize: 2 },
  };
  const vscode = {
    TextEdit: { replace: (range, text) => ({ range, text }) },
    Range: class {
      constructor(a, b) {
        Object.assign(this, { a, b });
      }
    },
    workspace: {
      getConfiguration: (section) => ({ get: (k, d) => settings[section]?.[k] ?? d }),
      onWillSaveTextDocument: (f) => ((onWillSave = f), { dispose() {} }),
    },
    commands: { executeCommand: async (...a) => (calls.push(a), []) },
    // 側のパネル（S247）が activate で登録するぶんだけ
    window: { registerWebviewViewProvider: () => ({ dispose() {} }), onDidChangeActiveTextEditor: () => ({}), onDidChangeTextEditorSelection: () => ({}) },
    Disposable: { from: () => ({ dispose() {} }) },
  };
  vscode.workspace.onDidChangeTextDocument = () => ({});
  const requests = [];
  class LanguageClient {
    start() {}
    sendRequest(method, params) {
      requests.push([method, params]);
      return method === "globe/format" && params.text.includes("(") ? Promise.reject(new Error("壊れている")) : Promise.resolve("整形後\n");
    }
  }
  const load = Module._load;
  Module._load = function (req, ...rest) {
    if (req === "vscode") return vscode;
    if (req === "vscode-languageclient/node") return { LanguageClient };
    return load.call(this, req, ...rest);
  };
  try {
    ext = createRequire(import.meta.url)("../extension.js");
    await ext.activate({ subscriptions: [] });
  } finally {
    Module._load = load;
  }
  const save = (languageId) => {
    const waited = [];
    onWillSave({ document: { languageId, uri: `file:///x.${languageId}` }, waitUntil: (p) => waited.push(p) });
    return waited.length;
  };
  assert.equal(save("modelica"), 0); // 既定はオフ
  settings.globeMode.formatOnSave = true;
  assert.equal(save("python"), 0);
  assert.equal(save("modelica"), 1);
  assert.deepEqual(calls, [["vscode.executeFormatDocumentProvider", "file:///x.modelica", { tabSize: 2, insertSpaces: true }]]);

  // 整形は globe/format に振り替え、設定 blankAroundSections を整形のたびに渡す（S061）
  const doc = (text) => ({ getText: () => text, positionAt: (n) => n });
  settings.globeMode.blankAroundSections = false;
  const edits = await ext.formatEdits(doc("model M end M;"), { tabSize: 2 });
  assert.deepEqual(requests.at(-1), ["globe/format", { text: "model M end M;", indent: 2, blankAroundSections: false, formatHtml: true }]);
  assert.equal(edits[0].text, "整形後\n");
  assert.deepEqual(await ext.formatEdits(doc("model M (end M;"), { tabSize: 2 }), []); // 壊れていれば変えない
  assert.deepEqual(await ext.formatEdits(doc("整形後\n"), { tabSize: 2 }), []); // 変わらなければ空
});

test("globeMode.path はワークスペースの設定から変えられない（clone したリポジトリのプログラムが走る。#94）", async () => {
  const { readFileSync } = await import("node:fs");
  const props = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")).contributes.configuration.properties;
  assert.equal(props["globeMode.path"].scope, "machine");
});

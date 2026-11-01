// 保存時の整形（S060）: globeMode.formatOnSave が true の .mo だけ、整形を頼む。vscode と LSP クライアントは差し替える。
import { test } from "node:test";
import assert from "node:assert/strict";
import Module, { createRequire } from "node:module";

test("formatOnSave が true の .mo だけ整形を頼む", async () => {
  let onWillSave;
  const calls = [];
  const settings = { globeMode: { formatOnSave: false, libraries: [], path: "globe" }, editor: { tabSize: 2 } };
  const vscode = {
    workspace: {
      getConfiguration: (section) => ({ get: (k, d) => settings[section]?.[k] ?? d }),
      onWillSaveTextDocument: (f) => ((onWillSave = f), { dispose() {} }),
    },
    commands: { executeCommand: async (...a) => (calls.push(a), []) },
  };
  class LanguageClient {
    start() {}
  }
  const load = Module._load;
  Module._load = function (req, ...rest) {
    if (req === "vscode") return vscode;
    if (req === "vscode-languageclient/node") return { LanguageClient };
    return load.call(this, req, ...rest);
  };
  try {
    const ext = createRequire(import.meta.url)("../extension.js");
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
});

// 拡張ホストの中で回す（S165）。mocha は使わず、順に回して最初の失敗で投げる。
const vscode = require("vscode");
const assert = require("node:assert/strict");
const path = require("node:path");

const fixture = (name) => vscode.Uri.file(path.join(__dirname, "fixtures", name));
const until = async (what, f, ms = 90_000) => {
  for (const end = Date.now() + ms; Date.now() < end; await new Promise((r) => setTimeout(r, 500))) {
    const v = await f();
    if (v) return v;
  }
  throw new Error(`時間切れ: ${what}`);
};

const cases = {
  async "言語 ID が modelica になり、拡張が activate する"() {
    const doc = await vscode.workspace.openTextDocument(fixture("Good.mo"));
    await vscode.window.showTextDocument(doc);
    assert.equal(doc.languageId, "modelica");
    const ext = vscode.extensions.getExtension("globe-mode.globe-mode");
    assert.ok(ext, "拡張が見つからない");
    await ext.activate(); // client.start() まで待つ（globe lsp の initialize が返る）
    assert.ok(ext.isActive);
  },
  async "構文の誤りのある .mo に診断が出る"() {
    const uri = fixture("Broken.mo");
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
    const ds = await until("診断", () => {
      const d = vscode.languages.getDiagnostics(uri);
      return d.length > 0 && d;
    });
    assert.ok(ds[0].message, JSON.stringify(ds));
  },
  async "正しい .mo でアウトライン（documentSymbol）が返る"() {
    const syms = await until("documentSymbol", async () => {
      const s = await vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", fixture("Good.mo"));
      return s?.length > 0 && s;
    });
    assert.equal(syms[0].name, "R");
  },
  async "textDocument/rename が WorkspaceEdit を返す"() {
    const uri = fixture("Good.mo");
    // 1 行目の `Real x;` の x（0 始まりの行 1、桁 7）。部品 x の宣言・der(x)・-x の 3 箇所が変わる（xx は変わらない）
    const we = await vscode.commands.executeCommand("vscode.executeDocumentRenameProvider", uri, new vscode.Position(1, 7), "pos");
    assert.ok(we, "WorkspaceEdit が null");
    const edits = we.get(uri);
    assert.equal(edits.length, 3, JSON.stringify(edits));
    assert.ok(edits.every((e) => e.newText === "pos"));
  },
};

exports.run = async () => {
  for (const [name, f] of Object.entries(cases)) {
    await f();
    console.log(`ok - ${name}`);
  }
};

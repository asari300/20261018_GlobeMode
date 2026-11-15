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
  async "クラスの rename が、開いていない .mo も WorkspaceEdit に入れる（S230）"() {
    const defs = fixture("Defs.mo");
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(defs));
    // Pairs.mo は開かない。Defs.mo の `model Pair`（行 0、桁 7）を変えると、Pairs.mo の `Pair p;` も入る
    const we = await until("rename", () => vscode.commands.executeCommand("vscode.executeDocumentRenameProvider", defs, new vscode.Position(0, 7), "Duo"));
    const files = we.entries().map(([u]) => path.basename(u.fsPath)).sort();
    assert.deepEqual(files, ["Defs.mo", "Pairs.mo"], JSON.stringify(files));
    assert.ok(we.entries().every(([, es]) => es.every((e) => e.newText === "Duo")));
  },
  async "側のパネル（GlobeMode ビュー）が開き、ChuaCircuit の図に部品が 7 個出る（S247）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("ChuaCircuit.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    // クラス ChuaCircuit の中（部品 Ro の行）にカーソルを置く。入れ子の Part の中ではない
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Part Ro"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const ext = vscode.extensions.getExtension("globe-mode.globe-mode");
    const { panel } = await ext.activate();
    await vscode.commands.executeCommand("globeMode.panel.focus"); // ビューを開く（webview が resolve される）
    // webview の DOM は拡張ホストから見えないので、描き終えた数を webview が送ってくる（panel.state）
    const st = await until("パネルの描画", () => panel.state?.parts === 7 && panel.state);
    assert.equal(st.class, "ChuaCircuit", JSON.stringify(st));
    assert.ok(st.icon === false, "ChuaCircuit にアイコンは無い");
    // 部品のクリックは reveal(宣言の行) になる。カーソルがその行へ動く
    await panel.reveal(row - 1); // 部品 L の行
    assert.equal(vscode.window.activeTextEditor.selection.active.line, row - 1);
  },
  async "パネルの部品の移動と結線が文書に入り、undo で戻る（S264）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("ChuaCircuit.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Part Ro"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の対象", () => panel.target?.class === "ChuaCircuit" && panel.target.doc === doc);
    const before = doc.getText();
    // webview のドラッグは拡張ホストから起こせないので、webview が送るメッセージをそのまま渡す（ドラッグの処理は pnpm test の diagram-edit.test.mjs）
    panel.state = null;
    assert.equal(await panel.handle({ type: "move", id: 1, part: "Ro", dx: 4, dy: -2 }), true, panel.why);
    assert.ok(doc.getText().includes("origin = {-71, -19}"), doc.getText());
    assert.ok(doc.isDirty, "保存はしない");
    await until("移動のあとの図の引き直し", () => panel.state?.parts === 7);
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), before, "undo 1 回で戻る");
    assert.equal(await panel.handle({ type: "connect", id: 2, from: "L.p", to: "Ro.n", points: [[0, 0], [1, 1]] }), true, panel.why);
    assert.ok(doc.getText().includes("connect(L.p, Ro.n)"), doc.getText());
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), before);
    // 検査を通らないメッセージは何もしない
    assert.equal(await panel.handle({ type: "move", id: 3, part: "R o", dx: 1, dy: 1 }), false);
    assert.equal(doc.getText(), before);
  },
  async "パネルの部品・結線の削除が文書に入り、undo で戻る（S293）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("ChuaCircuit.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Part Ro"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の対象", () => panel.target?.class === "ChuaCircuit" && panel.target.doc === doc);
    const before = doc.getText();
    assert.equal(await panel.handle({ type: "connect", id: 1, from: "L.p", to: "Ro.n", points: [[0, 0], [1, 1]] }), true, panel.why);
    const connected = doc.getText();
    // 結線だけ消す（端の逆順でも同じ結線）。undo で戻る
    assert.equal(await panel.handle({ type: "deleteConnect", id: 2, from: "Ro.n", to: "L.p" }), true, panel.why);
    assert.ok(!doc.getText().includes("connect(") && doc.getText().includes("Part Ro"), doc.getText()); // 空になった equation の見出しは残る（S281）
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), connected);
    // 部品を消すと、宣言とつながる connect がまとめて消え、undo 1 回で戻る
    assert.equal(await panel.handle({ type: "deletePart", id: 3, part: "Ro" }), true, panel.why);
    assert.ok(!doc.getText().includes("Part Ro") && !doc.getText().includes("connect("), doc.getText());
    assert.ok(doc.isDirty, "保存はしない");
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), connected, "undo 1 回で戻る");
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), before);
    // 検査を通らない・無い部品は何もしない
    assert.equal(await panel.handle({ type: "deletePart", id: 4, part: "R o" }), false);
    assert.equal(await panel.handle({ type: "deletePart", id: 5, part: "Nothing" }), false);
    assert.equal(doc.getText(), before);
  },
  async "パネルの結線の途中の点を足して動かすと Line(points) が書き換わり、undo で戻る（S302）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("ChuaCircuit.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Part Ro"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の対象", () => panel.target?.class === "ChuaCircuit" && panel.target.doc === doc);
    const before = doc.getText();
    assert.equal(await panel.handle({ type: "connect", id: 1, from: "L.p", to: "Ro.n", points: [[0, 0], [10, 10]] }), true, panel.why);
    const connected = doc.getText();
    // 点を足す（index 1 = 始めと終わりの間）。webview のダブルクリックが送るメッセージをそのまま渡す
    assert.equal(await panel.handle({ type: "addWaypoint", id: 2, from: "L.p", to: "Ro.n", index: 1, x: 4, y: 6, ends: [[0, 0], [10, 10]] }), true, panel.why);
    assert.ok(doc.getText().includes("{4, 6}"), doc.getText());
    const added = doc.getText();
    // 動かす（つまみのドラッグ）。端の点は動かせない
    assert.equal(await panel.handle({ type: "moveWaypoint", id: 3, from: "Ro.n", to: "L.p", index: 1, dx: 2, dy: -2 }), true, panel.why);
    assert.ok(doc.getText().includes("{6, 4}") && !doc.getText().includes("{4, 6}"), doc.getText());
    assert.ok(doc.isDirty, "保存はしない");
    assert.equal(await panel.handle({ type: "moveWaypoint", id: 4, from: "L.p", to: "Ro.n", index: 0, dx: 2, dy: 2 }), false, "端は動かせない");
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), added);
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), connected);
    await vscode.commands.executeCommand("undo");
    assert.equal(doc.getText(), before);
    // 検査を通らないメッセージは何もしない
    assert.equal(await panel.handle({ type: "addWaypoint", id: 5, from: "L.p", to: "Ro.n", index: 1.5, x: 1, y: 1, ends: [[0, 0], [1, 1]] }), false);
    assert.equal(doc.getText(), before);
  },
};

exports.run = async () => {
  for (const [name, f] of Object.entries(cases)) {
    await f();
    console.log(`ok - ${name}`);
  }
};

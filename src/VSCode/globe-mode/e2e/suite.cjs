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
  async "キー操作の流れ（c → Tab・Enter で始点・終点）のメッセージで結線が足され、undo で戻る（S316）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("ChuaCircuit.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Part Ro"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の対象", () => panel.target?.class === "ChuaCircuit" && panel.target.doc === doc);
    const before = doc.getText();
    // webview のキーは拡張ホストから起こせないので、共有の diagram-connect.ts（esbuild で JS にする）にキーを渡し、出てきた connect をそのままパネルへ渡す
    const m = { exports: {} };
    new Function("module", "exports", require("esbuild").buildSync({ entryPoints: [path.join(__dirname, "../../../Svelte/src/lib/diagram-connect.ts")], bundle: true, format: "cjs", write: false }).outputFiles[0].text)(m, m.exports);
    // フィクスチャの Part はアイコンのコネクタを持たず、図にコネクタが出ない。部品の名前（panel.parts）に p・n を付けた図で流れを確かめる
    assert.ok(panel.parts.some((p) => p.name === "L") && panel.parts.some((p) => p.name === "Ro"));
    const d = { parts: ["L", "Ro"].map((name) => ({ name, connectors: [{ name: "p", pos: [-10, 0] }, { name: "n", pos: [10, 0] }] })), connections: [] };
    const press = (w, sel, key, shiftKey = false) => m.exports.wireKey(w, d, sel, { key, shiftKey, ctrlKey: false, metaKey: false, altKey: false });
    const choose = (s, end) => {
      for (let i = 0; i < 20 && s.wire.list[s.wire.i] !== end; i++) s = press(s.wire, null, "Tab");
      return press(s.wire, null, "Enter");
    };
    let s = choose(press(null, { part: "L" }, "c"), "L.p");
    assert.equal(s.wire?.from, "L.p", s.say);
    s = choose(s, "Ro.n");
    assert.deepEqual([s.wire, s.connect.from, s.connect.to], [null, "L.p", "Ro.n"], s.say);
    assert.equal(await panel.handle({ type: "connect", id: 1, ...s.connect }), true, panel.why);
    assert.ok(doc.getText().includes("connect(L.p, Ro.n)"), doc.getText());
    await vscode.commands.executeCommand("undo");
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
  async "パネルの部品の R を 100 に書くと宣言に (R = 100) が入り、undo で戻る。空の入力で外れる（S306）"() {
    // 構文の壊れた Broken.mo が開いたままだと、LSP の Workspace 全体の変換が通らない（閉じても VS Code が文書を手放すまで didClose は来ない）。保存せずに直しておく
    const broken = await vscode.workspace.openTextDocument(fixture("Broken.mo"));
    const fix = new vscode.WorkspaceEdit();
    fix.replace(broken.uri, new vscode.Range(0, 0, broken.lineCount, 0), "model Broken\nend Broken;\n");
    assert.ok(await vscode.workspace.applyEdit(fix));
    const doc = await vscode.workspace.openTextDocument(fixture("Params.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Res r1"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の対象", () => panel.target?.class === "Params" && panel.target.doc === doc);
    await until("パラメータの一覧", () => panel.getClient().sendRequest("globe/parameters", { model: "Params" }).then(() => true, () => false));
    const before = doc.getText();
    // QuickPick・InputBox は値を渡せる偽物に差し替える（webview のダブルクリックは拡張ホストから起こせないので、メッセージをそのまま渡す）
    const shown = [];
    const answer = { pick: "R", input: "100" };
    panel.ui = {
      showQuickPick: async (items) => (shown.push(items.map((i) => i.label)), items.find((i) => i.label === answer.pick)),
      showInputBox: async () => answer.input,
      showErrorMessage: (m) => shown.push(m),
      showInformationMessage: (m) => shown.push(m),
    };
    try {
      assert.equal(await panel.handle({ type: "params", part: "r1" }), true, JSON.stringify(shown));
      assert.deepEqual(shown[0], ["R", "mode"]);
      assert.ok(doc.getText().includes("Res r1(R = 100)"), doc.getText());
      assert.ok(doc.isDirty, "保存はしない");
      await vscode.commands.executeCommand("undo");
      assert.equal(doc.getText(), before, "undo 1 回で戻る");
      // 空の入力で修飾を外す
      assert.equal(await panel.handle({ type: "params", part: "r1" }), true, JSON.stringify(shown));
      answer.input = "";
      assert.equal(await panel.handle({ type: "params", part: "r1" }), true, JSON.stringify(shown));
      assert.equal(doc.getText(), before, "(R = 100) が外れる");
      await vscode.commands.executeCommand("undo");
      await vscode.commands.executeCommand("undo");
      assert.equal(doc.getText(), before);
      // 列挙は QuickPick で選ぶ（型つきの名前で書く）
      const want = ["mode", "high"];
      panel.ui.showQuickPick = async (items) => { const w = want.shift(); return items.find((i) => i.label === w); };
      assert.equal(await panel.handle({ type: "params", part: "r1" }), true, JSON.stringify(shown));
      assert.ok(doc.getText().includes("Res r1(mode = Params.Mode.high)"), doc.getText());
      await vscode.commands.executeCommand("undo");
      assert.equal(doc.getText(), before);
      // LSP が断る式は通知に理由が出て、文書は変わらない
      panel.ui.showQuickPick = async (items) => items.find((i) => i.label === "R");
      panel.ui.showInputBox = async () => "1;2";
      shown.length = 0;
      assert.equal(await panel.handle({ type: "params", part: "r1" }), false);
      assert.ok(shown.some((m) => typeof m === "string" && m.includes("壊れている")), JSON.stringify(shown));
      assert.equal(doc.getText(), before);
      // 検査を通らないメッセージは何もしない
      assert.equal(await panel.handle({ type: "params", part: "r 1" }), false);
    } finally {
      panel.ui = vscode.window;
    }
  },
  async "コマンド「部品のパラメータを編集」で、カーソルの部品か QuickPick で選んだ部品の R を書ける（S309）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("Params.mo"));
    const ed = await vscode.window.showTextDocument(doc);
    const row = doc.getText().split("\n").findIndex((l) => l.startsWith("  Res r1"));
    ed.selection = new vscode.Selection(row, 4, row, 4);
    const { panel } = await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    await until("図の部品", () => panel.target?.class === "Params" && panel.target.doc === doc && panel.parts?.length);
    await until("パラメータの一覧", () => panel.getClient().sendRequest("globe/parameters", { model: "Params" }).then(() => true, () => false));
    const before = doc.getText();
    const shown = [];
    const want = ["R"]; // カーソルが r1 の行なので、部品は聞かれずパラメータから
    panel.ui = {
      showQuickPick: async (items) => (shown.push(items.map((i) => i.label)), items.find((i) => i.label === want.shift())),
      showInputBox: async () => "42",
      showErrorMessage: (m) => shown.push(m),
      showInformationMessage: (m) => shown.push(m),
    };
    try {
      assert.equal(await vscode.commands.executeCommand("globeMode.editParams"), true, JSON.stringify(shown));
      assert.deepEqual(shown[0], ["R", "mode"]);
      assert.ok(doc.getText().includes("Res r1(R = 42)"), doc.getText());
      await vscode.commands.executeCommand("undo");
      assert.equal(doc.getText(), before);
      // カーソルが部品の行に無ければ、先に部品を QuickPick で選ぶ
      ed.selection = new vscode.Selection(0, 0, 0, 0);
      shown.length = 0;
      want.push("r1", "R");
      assert.equal(await vscode.commands.executeCommand("globeMode.editParams"), true, JSON.stringify(shown));
      assert.deepEqual(shown[0], ["r1"]);
      assert.ok(doc.getText().includes("Res r1(R = 42)"), doc.getText());
      await vscode.commands.executeCommand("undo");
      assert.equal(doc.getText(), before);
    } finally {
      panel.ui = vscode.window;
    }
  },
  async "関数の呼び出しの位置引数の前に、引数の名前の inlay hint が出る（S353）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("Inlay.mo"));
    await vscode.window.showTextDocument(doc);
    await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    // 範囲はモデルの行だけ（Twice の中の `Scale(v, v)` にもヒントは出るが、範囲の外）
    const row = doc.getText().split("\n").findIndex((l) => l.includes("Scale(2, 3)"));
    const range = new vscode.Range(row, 0, row + 1, 0);
    const hints = await until("inlayHint", async () => {
      const h = await vscode.commands.executeCommand("vscode.executeInlayHintProvider", doc.uri, range);
      return h?.length > 0 && h;
    });
    // `Scale(2, 3)` の 2 と 3 の前に input の名前。組み込みの `sin(time)` には出ない
    const at = (c) => new vscode.Position(row, doc.lineAt(row).text.indexOf(c));
    const got = hints.map((h) => [h.position.line, h.position.character, typeof h.label === "string" ? h.label : h.label.map((p) => p.value).join(""), h.kind]);
    const P = vscode.InlayHintKind.Parameter;
    assert.deepEqual(got, [[row, at("2, 3").character, "gain:", P], [row, at("3)").character, "offset:", P]], JSON.stringify(got));
  },
  async "関数の call hierarchy（呼び元・呼び先）が返る（S353）"() {
    const doc = await vscode.workspace.openTextDocument(fixture("Inlay.mo"));
    await vscode.window.showTextDocument(doc);
    await vscode.extensions.getExtension("globe-mode.globe-mode").activate();
    const at = (decl) => { const row = doc.getText().split("\n").findIndex((l) => l.startsWith(decl)); return new vscode.Position(row, doc.lineAt(row).text.indexOf(decl.split(" ")[1])); };
    const prepare = (decl) => until("prepareCallHierarchy", async () => {
      const items = await vscode.commands.executeCommand("vscode.prepareCallHierarchy", doc.uri, at(decl));
      return items?.length > 0 && items;
    });
    // Scale を呼ぶのは関数 Twice とモデル Inlay（呼び元は関数とは限らない）。呼び出しの範囲は 1 つずつ
    const [scale] = await prepare("function Scale");
    assert.deepEqual([scale.name, scale.kind], ["Scale", vscode.SymbolKind.Function]);
    const incoming = await vscode.commands.executeCommand("vscode.provideIncomingCalls", scale);
    const callers = incoming.map((c) => [c.from.name, c.fromRanges.length]).sort();
    assert.deepEqual(callers, [["Inlay", 1], ["Twice", 1]], JSON.stringify(callers));
    // Twice の本体が呼ぶのは Scale だけ。Scale の本体 `gain + offset` は何も呼ばない
    const [twice] = await prepare("function Twice");
    const outgoing = await vscode.commands.executeCommand("vscode.provideOutgoingCalls", twice);
    assert.deepEqual(outgoing.map((c) => [c.to.name, c.fromRanges.length]), [["Scale", 1]], JSON.stringify(outgoing));
    assert.ok(!(await vscode.commands.executeCommand("vscode.provideOutgoingCalls", scale))?.length);
  },
};

exports.run = async () => {
  for (const [name, f] of Object.entries(cases)) {
    await f();
    console.log(`ok - ${name}`);
  }
};

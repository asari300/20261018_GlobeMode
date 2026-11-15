// 側のパネル（S247）。Svelte の UI の「説明 / アイコン / 図」と同じものを、Explorer のビュー「GlobeMode」に出す。
// 引くのは `globe lsp` の globe/documentation・globe/icon・globe/diagram だけ（業務ロジックは Rust 側）。
// 描くのは webview.js（dist/webview.js に束ねる）。ここは「カーソルが 300 ms 止まったら 3 つを引いて webview へ渡す」と「部品のクリックで宣言の行へ」と、S264 の図の編集（部品の移動・結線を LSP に頼んで文書に入れる）だけ。
const vscode = require("vscode");
const crypto = require("node:crypto");
const { parseEdit, parseParams, partParams, enumValue } = require("./edit");

class Panel {
  /** @param {vscode.Uri} extensionUri @param {() => import("vscode-languageclient/node").LanguageClient | undefined} getClient */
  constructor(extensionUri, getClient) {
    this.extensionUri = extensionUri;
    this.getClient = getClient;
    this.seq = 0;
    this.ui = vscode.window; // QuickPick・InputBox・通知（e2e が差し替える）
    this.state = null; // webview が描き終えたあとの数（e2e が見る。webview の DOM は拡張ホストから見えない）
  }

  /** エディタの動きを見て、止まったら取り直す。dispose するものを返す */
  watch() {
    const w = vscode.window, ws = vscode.workspace;
    return vscode.Disposable.from(
      w.onDidChangeActiveTextEditor(() => this.soon()),
      w.onDidChangeTextEditorSelection((e) => e.textEditor === w.activeTextEditor && this.soon()),
      ws.onDidChangeTextDocument((e) => e.document === w.activeTextEditor?.document && this.soon()),
    );
  }

  resolveWebviewView(view) {
    this.view = view;
    const dist = vscode.Uri.joinPath(this.extensionUri, "dist");
    const web = view.webview;
    web.options = { enableScripts: true, localResourceRoots: [dist] };
    const nonce = crypto.randomBytes(16).toString("base64");
    const script = web.asWebviewUri(vscode.Uri.joinPath(dist, "webview.js"));
    // 外への通信も、nonce の無いスクリプトも許さない
    web.html = `<!doctype html><html lang="ja"><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${web.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'">
<meta name="viewport" content="width=device-width, initial-scale=1"><title>GlobeMode</title>
<style>
/* 地は常に明るい（Modelica の色は白地を前提にしている。Svelte の UI と同じ） */
#icon, #diagram { background: #fff; color: #555; border: 1px solid var(--vscode-panel-border, #888); margin: 4px 0; }
#icon svg { height: 120px; } #diagram svg { height: 240px; }
svg { width: 100%; display: block; } svg .part { cursor: grab; touch-action: none; } svg .part:focus-visible { outline: 2px solid #06c; }
svg .conn { fill: #06c; fill-opacity: 0; cursor: crosshair; } svg .conn:hover { fill-opacity: .35; } svg .hit { cursor: pointer; } svg .wp { fill: #fff; stroke: #06c; stroke-width: 2; vector-effect: non-scaling-stroke; cursor: move; touch-action: none; } svg .hit:focus-visible { outline: 2px solid #06c; }
svg .part.sel > path { stroke: #06c; stroke-width: 2; stroke-dasharray: 4 2; vector-effect: non-scaling-stroke; fill: #06c; fill-opacity: .08; } svg [data-conn].sel polyline { stroke: #06c; stroke-width: 3; }
.none { margin: 4px; font-size: .85em; } #docs { overflow-wrap: anywhere; } pre { white-space: pre-wrap; }
table { border-collapse: collapse; } td, th { border: 1px solid var(--vscode-panel-border, #888); padding: 1px 4px; }
</style></head>
<body><div id="icon"></div><div id="diagram"></div><p id="cls"></p><div id="docs"></div><details id="revisions" hidden><summary>改訂履歴</summary><div id="rev"></div></details>
<script nonce="${nonce}" src="${script}"></script></body></html>`;
    web.onDidReceiveMessage((m) => this.handle(m));
    view.onDidChangeVisibility(() => view.visible && this.soon());
    this.soon();
  }

  /** webview からのメッセージ。型と値を検査する（webview は信頼の境界の外）。編集なら適用できたかを返す */
  handle(m) {
    if (m?.type === "reveal" && Number.isInteger(m.line)) void this.reveal(m.line);
    else if (m?.type === "rendered") this.state = m;
    else if (m?.type === "params") return this.editParams(parseParams(m));
    else if (["move", "connect", "deletePart", "deleteConnect", "moveWaypoint", "addWaypoint"].includes(m?.type)) return this.edit(parseEdit(m));
  }

  /** 図の編集（S264）。LSP の globe/movePart・globe/addConnect が返す TextEdit を WorkspaceEdit で文書に入れる（undo 1 回。保存しない）。
   * 入れたあとの図は、文書の変更で動く `soon()` が引き直す。結果は webview に返し、失敗なら見た目を戻させる */
  async edit(req) {
    const ok = req ? await this.apply(req).catch((e) => ((this.why = String(e)), false)) : false;
    if (req) void this.view?.webview.postMessage({ type: "edited", id: req.id, ok });
    return ok;
  }

  /** 部品のパラメータの入力（S306）。globe/parameters からその部品の分を QuickPick で選び、値を InputBox（列挙・Boolean は QuickPick）で受けて
   * globe/setModifier で `(名前 = 値)` を書く。空の入力は修飾を外す。断られたら理由を通知に出す。書けたかを返す。
   * shortcut: 修飾の検査は Rust 側に任せる。小さい方として webview 内の表でなく QuickPick にした（CSP・入力部品を足さずに済む。多くの値を一度に直すなら表へ） */
  async editParams(part) {
    const t = this.target, client = this.getClient(), ui = this.ui;
    if (!part || !t || !client) return false;
    const v = t.doc.version; // 開始時の対象と版を使い続ける（応答待ちと入力の間に target が変わる。#117）
    const fail = (m) => (ui.showErrorMessage(`GlobeMode: ${m}`), false);
    let list;
    try {
      list = partParams((await client.sendRequest("globe/parameters", { model: t.class })).parameters, part);
    } catch (e) {
      return fail(`パラメータを取れない: ${e.message ?? e}`);
    }
    if (!list.length) return ui.showInformationMessage(`GlobeMode: ${part} に書けるパラメータは無い`), false;
    const pick = await ui.showQuickPick(
      list.map((p) => ({ label: p.key, description: [p.default, p.unit].filter(Boolean).join(" "), detail: p.description, p })),
      { title: `${part} のパラメータ`, placeHolder: "修飾に書くパラメータ" },
    );
    if (!pick) return false;
    const p = pick.p;
    let value; // undefined は取り消し、"" は修飾を外す
    if (p.type === "Real" || p.type === "Integer") {
      value = await ui.showInputBox({ title: `${part}.${p.key}（${p.type}）`, prompt: "値か式。空で修飾をソースから外す", value: p.default ?? "" });
    } else {
      const items = (p.type === "Boolean" ? ["true", "false"] : p.literals ?? []).map((l) => ({ label: l, value: p.type === "Boolean" ? l : enumValue(p, l) }));
      items.push({ label: "修飾を外す", value: "" });
      value = (await ui.showQuickPick(items, { title: `${part}.${p.key}（${p.type}）` }))?.value;
      if (value === null) return fail("列挙の型の名前が無く書けない");
    }
    if (value === undefined) return false;
    const ok = await this.apply({ method: "globe/setModifier", args: { modifiers: [{ part, name: p.key, value: value.trim() }] } }, t, v).catch((e) => ((this.why = e.message ?? String(e)), false));
    return ok || fail(`書けなかった: ${this.why}`);
  }

  /** コマンド「部品のパラメータを編集」（S309。キーボードだけで開く口）。カーソルが部品の宣言の行にあればその部品、なければ部品の QuickPick で選ぶ */
  async editParamsCommand() {
    const ed = vscode.window.activeTextEditor, parts = this.parts ?? [], ui = this.ui;
    if (!this.target || !parts.length || ed?.document !== this.target.doc) return ui.showInformationMessage("GlobeMode: パネルの図に部品が無い（Modelica のクラスの中にカーソルを置く）"), false;
    const at = parts.find((p) => p.line === ed.selection.active.line);
    const pick = at ?? (await ui.showQuickPick(parts.map((p) => ({ label: p.name, description: p.class, p })), { title: "パラメータを編集する部品" }))?.p;
    return pick ? this.editParams(pick.name) : false;
  }

  async apply({ method, args }, t = this.target, v = t?.doc.version) {
    const client = this.getClient();
    this.why = ""; // 失敗の理由（e2e が見る）
    if (!t || !client) return (this.why = "対象が無い"), false;
    if (t.doc.version !== v) return (this.why = "文書が変わったので入れなかった"), false; // 呼び手が控えた版から変わっている
    const edits = await client.sendRequest(method, { file: t.doc.uri.toString(), class: t.class, ...args });
    // 待つあいだに文書が変わったら、古い本文に対する編集は入れない
    if (!edits?.length || t.doc.version !== v) return (this.why = t.doc.version !== v ? "文書が変わったので入れなかった" : "ソースは変わらなかった（変えられない）"), false;
    const we = new vscode.WorkspaceEdit();
    we.set(t.doc.uri, await client.protocol2CodeConverter.asTextEdits(edits)); // v9 では Promise
    return (await vscode.workspace.applyEdit(we)) || ((this.why = "applyEdit が false"), false);
  }

  soon() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.refresh(), 300);
  }

  async refresh() {
    const ed = vscode.window.activeTextEditor;
    const client = this.getClient();
    if (!this.view?.visible || !client || !ed || ed.document.languageId !== "modelica") return;
    this.editor = ed;
    const at = { file: ed.document.uri.toString(), line: ed.selection.active.line, character: ed.selection.active.character };
    const mine = ++this.seq;
    // アイコン・図が引けなくても説明は出す。新しい要求が来たら古い応答は捨てる
    const ask = (m) => client.sendRequest(m, at).catch(() => null);
    const [doc, icon, diagram] = await Promise.all([ask("globe/documentation"), ask("globe/icon"), ask("globe/diagram")]);
    if (mine !== this.seq) return;
    this.parts = diagram?.parts ?? []; // コマンド（editParamsCommand）がカーソルの部品を探すのに使う
    this.target = diagram?.class ? { doc: ed.document, class: diagram.class } : null; // 編集（edit）の対象は、いま webview が描いている図
    void this.view?.webview.postMessage({ type: "show", doc, icon, diagram });
  }

  /** 部品の宣言の行（0 始まり）へ。カーソルを置いたエディタに戻す */
  async reveal(line) {
    const ed = this.editor;
    if (!ed) return;
    const p = new vscode.Position(line, 0);
    await vscode.window.showTextDocument(ed.document, { viewColumn: ed.viewColumn, selection: new vscode.Range(p, p), preserveFocus: true }); // フォーカスは webview に残す（選んだ部品を Delete で消せるように。S293）
  }
}

module.exports = { Panel };

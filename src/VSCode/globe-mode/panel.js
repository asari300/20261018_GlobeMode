// 側のパネル（S247）。Svelte の UI の「説明 / アイコン / 図」と同じものを、Explorer のビュー「GlobeMode」に出す。
// 引くのは `globe lsp` の globe/documentation・globe/icon・globe/diagram だけ（業務ロジックは Rust 側）。
// 描くのは webview.js（dist/webview.js に束ねる）。ここは「カーソルが 300 ms 止まったら 3 つを引いて webview へ渡す」と「部品のクリックで宣言の行へ」と、S264 の図の編集（部品の移動・結線を LSP に頼んで文書に入れる）だけ。
const vscode = require("vscode");
const crypto = require("node:crypto");
const { parseEdit } = require("./edit");

class Panel {
  /** @param {vscode.Uri} extensionUri @param {() => import("vscode-languageclient/node").LanguageClient | undefined} getClient */
  constructor(extensionUri, getClient) {
    this.extensionUri = extensionUri;
    this.getClient = getClient;
    this.seq = 0;
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
svg .conn { fill: #06c; fill-opacity: 0; cursor: crosshair; } svg .conn:hover { fill-opacity: .35; }
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
    else if (m?.type === "move" || m?.type === "connect") return this.edit(parseEdit(m));
  }

  /** 図の編集（S264）。LSP の globe/movePart・globe/addConnect が返す TextEdit を WorkspaceEdit で文書に入れる（undo 1 回。保存しない）。
   * 入れたあとの図は、文書の変更で動く `soon()` が引き直す。結果は webview に返し、失敗なら見た目を戻させる */
  async edit(req) {
    const ok = req ? await this.apply(req).catch((e) => ((this.why = String(e)), false)) : false;
    if (req) void this.view?.webview.postMessage({ type: "edited", id: req.id, ok });
    return ok;
  }

  async apply({ method, args }) {
    const t = this.target, client = this.getClient();
    this.why = ""; // 失敗の理由（e2e が見る）
    if (!t || !client) return (this.why = "対象が無い"), false;
    const v = t.doc.version;
    const edits = await client.sendRequest(method, { file: t.doc.uri.toString(), class: t.class, ...args });
    // 待つあいだに文書が変わったら、古い本文に対する編集は入れない
    if (!edits?.length || t.doc.version !== v) return (this.why = `edits=${JSON.stringify(edits)} version ${v} → ${t.doc.version}`), false;
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
    this.target = diagram?.class ? { doc: ed.document, class: diagram.class } : null; // 編集（edit）の対象は、いま webview が描いている図
    void this.view?.webview.postMessage({ type: "show", doc, icon, diagram });
  }

  /** 部品の宣言の行（0 始まり）へ。カーソルを置いたエディタに戻す */
  async reveal(line) {
    const ed = this.editor;
    if (!ed) return;
    const p = new vscode.Position(line, 0);
    await vscode.window.showTextDocument(ed.document, { viewColumn: ed.viewColumn, selection: new vscode.Range(p, p) });
  }
}

module.exports = { Panel };

// 側のパネル（S247）。Svelte の UI の「説明 / アイコン / 図」と同じものを、Explorer のビュー「GlobeMode」に出す。
// 引くのは `globe lsp` の globe/documentation・globe/icon・globe/diagram だけ（業務ロジックは Rust 側）。
// 描くのは webview.js（dist/webview.js に束ねる）。ここは「カーソルが 300 ms 止まったら 3 つを引いて webview へ渡す」と「部品のクリックで宣言の行へ」だけ。
const vscode = require("vscode");
const crypto = require("node:crypto");

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
svg { width: 100%; display: block; } svg .part { cursor: pointer; } svg .part:focus-visible { outline: 2px solid #06c; }
.none { margin: 4px; font-size: .85em; } #docs { overflow-wrap: anywhere; } pre { white-space: pre-wrap; }
table { border-collapse: collapse; } td, th { border: 1px solid var(--vscode-panel-border, #888); padding: 1px 4px; }
</style></head>
<body><div id="icon"></div><div id="diagram"></div><p id="cls"></p><div id="docs"></div><details id="revisions" hidden><summary>改訂履歴</summary><div id="rev"></div></details>
<script nonce="${nonce}" src="${script}"></script></body></html>`;
    web.onDidReceiveMessage((m) => {
      if (m?.type === "reveal" && Number.isInteger(m.line)) void this.reveal(m.line);
      else if (m?.type === "rendered") this.state = m;
    });
    view.onDidChangeVisibility(() => view.visible && this.soon());
    this.soon();
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
    if (mine === this.seq) void this.view?.webview.postMessage({ type: "show", doc, icon, diagram });
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

// GlobeMode の VS Code 拡張（S059）。`globe lsp` を起動してつなぐだけで、整形・lint・補完・検査は Rust 側が受け持つ。
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

// shortcut: 設定のうち path・libraries は起動時に 1 回だけ読む。変えたら「ウィンドウの再読み込み」。頻繁に変えるなら client.restart で作り直す

exports.activate = async (context) => {
  const cfg = vscode.workspace.getConfiguration("globeMode");
  const args = ["lsp", ...cfg.get("libraries", []).flatMap((d) => ["--library", d])];
  client = new LanguageClient(
    "globeMode",
    "GlobeMode",
    { command: cfg.get("path", "globe"), args },
    {
      documentSelector: [{ scheme: "file", language: "modelica" }],
      middleware: { provideDocumentFormattingEdits: formatEdits },
    },
  );
  // 保存時の整形（S060）。設定は保存のたびに読むので、切り替えに再読み込みは要らない
  context.subscriptions.push(
    vscode.workspace.onWillSaveTextDocument((e) => {
      const doc = e.document;
      if (doc.languageId !== "modelica" || !vscode.workspace.getConfiguration("globeMode", doc).get("formatOnSave")) return;
      const editor = vscode.workspace.getConfiguration("editor", doc);
      e.waitUntil(
        vscode.commands.executeCommand("vscode.executeFormatDocumentProvider", doc.uri, {
          tabSize: editor.get("tabSize", 2),
          insertSpaces: true,
        }),
      );
    }),
  );
  await client.start();
};

// 整形は独自の要求 globe/format で頼む（S061）。vscode-languageclient は FormattingOptions の追加のキーを捨てるので、
// 標準の textDocument/formatting では blankAroundSections を渡せない。設定は整形のたびに読む
async function formatEdits(doc, options) {
  const text = doc.getText();
  const cfg = vscode.workspace.getConfiguration("globeMode", doc);
  const out = await client
    .sendRequest("globe/format", {
      text,
      indent: options.tabSize,
      blankAroundSections: cfg.get("blankAroundSections", true),
    })
    .catch(() => null); // 文法が壊れていれば変えない
  if (out == null || out === text) return [];
  return [vscode.TextEdit.replace(new vscode.Range(doc.positionAt(0), doc.positionAt(text.length)), out)];
}

exports.formatEdits = formatEdits;
exports.deactivate = () => client?.stop();

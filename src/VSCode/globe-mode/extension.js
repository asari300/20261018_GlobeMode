// GlobeMode の VS Code 拡張（S059）。`globe lsp` を起動してつなぐだけで、整形・lint・補完・検査は Rust 側が受け持つ。
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

// shortcut: 設定（path・libraries）は起動時に 1 回だけ読む。変えたら「ウィンドウの再読み込み」。頻繁に変えるなら client.restart で作り直す

exports.activate = async () => {
  const cfg = vscode.workspace.getConfiguration("globeMode");
  const args = ["lsp", ...cfg.get("libraries", []).flatMap((d) => ["--library", d])];
  client = new LanguageClient(
    "globeMode",
    "GlobeMode",
    { command: cfg.get("path", "globe"), args },
    { documentSelector: [{ scheme: "file", language: "modelica" }] },
  );
  await client.start();
};

exports.deactivate = () => client?.stop();

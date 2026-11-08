// S165: 本物の VS Code（stable）を起動して拡張を読み込み、e2e/suite.cjs を拡張ホストの中で回す。
// `globe` は PATH から探す（拡張の既定の globeMode.path）。ディスプレイが無ければ `xvfb-run -a node e2e/run.mjs`。
import { runTests } from "@vscode/test-electron";
import { fileURLToPath } from "node:url";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
await runTests({
  extensionDevelopmentPath: here(".."),
  extensionTestsPath: here("suite.cjs"),
  // user-data-dir の既定は .vscode-test/ の下で長い。macOS の Unix ソケット（IPC ハンドル）は 103 文字までなので、短い一時フォルダにする（S233）
  launchArgs: [here("fixtures"), "--user-data-dir", mkdtempSync(join(tmpdir(), "gm-")), "--disable-extensions", "--disable-workspace-trust", "--no-sandbox"],
});

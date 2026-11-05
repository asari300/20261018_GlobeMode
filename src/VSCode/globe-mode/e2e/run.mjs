// S165: 本物の VS Code（stable）を起動して拡張を読み込み、e2e/suite.cjs を拡張ホストの中で回す。
// `globe` は PATH から探す（拡張の既定の globeMode.path）。ディスプレイが無ければ `xvfb-run -a node e2e/run.mjs`。
import { runTests } from "@vscode/test-electron";
import { fileURLToPath } from "node:url";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
await runTests({
  extensionDevelopmentPath: here(".."),
  extensionTestsPath: here("suite.cjs"),
  launchArgs: [here("fixtures"), "--disable-extensions", "--disable-workspace-trust", "--no-sandbox"],
});

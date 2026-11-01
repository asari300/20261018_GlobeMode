// 拡張が VS Code から送るのと同じ形（file:// の URI、package.json の既定の tabSize）で `globe lsp` と話す（S059）。
// GLOBE_BIN が無ければ PATH の globe。VS Code の本体は起動しない。
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url)));
const tabSize = pkg.contributes.configurationDefaults["[modelica]"]["editor.tabSize"];

function lsp() {
  const p = spawn(process.env.GLOBE_BIN ?? "globe", ["lsp"], { stdio: ["pipe", "pipe", "inherit"] });
  let buf = Buffer.alloc(0);
  const waiting = [];
  const seen = [];
  p.stdout.on("data", (d) => {
    buf = Buffer.concat([buf, d]);
    for (;;) {
      const head = buf.indexOf("\r\n\r\n");
      if (head < 0) return;
      const len = Number(/Content-Length: (\d+)/i.exec(buf.subarray(0, head).toString())[1]);
      if (buf.length < head + 4 + len) return;
      const msg = JSON.parse(buf.subarray(head + 4, head + 4 + len).toString());
      buf = buf.subarray(head + 4 + len);
      seen.push(msg);
      for (const w of [...waiting]) if (w.match(msg)) waiting.splice(waiting.indexOf(w), 1) && w.resolve(msg);
    }
  });
  const send = (m) => {
    const body = JSON.stringify({ jsonrpc: "2.0", ...m });
    p.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  };
  const until = (match) =>
    new Promise((resolve) => {
      const hit = seen.find(match);
      hit ? resolve(hit) : waiting.push({ match, resolve });
    });
  return { p, send, until };
}

test("標準の整形・lint の診断・補完が file:// の文書で返る", { timeout: 60_000 }, async (t) => {
  assert.equal(tabSize, 2);
  const s = lsp();
  t.after(() => s.p.kill()); // assert が落ちても子プロセスを残さない
  const uri = pathToFileURL("/tmp/globe-vscode/Small_model.mo").href;
  const text = "model small_model\nReal x(start=1);\nequation\nder(x)=-x;\nend small_model;\n";
  s.send({ id: 1, method: "initialize", params: { processId: null, rootUri: null, capabilities: {} } });
  const init = await s.until((m) => m.id === 1);
  assert.equal(init.result.capabilities.documentFormattingProvider, true);
  s.send({ method: "initialized", params: {} });
  s.send({ method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "modelica", version: 1, text } } });

  const diags = await s.until((m) => m.method === "textDocument/publishDiagnostics");
  assert.equal(diags.params.uri, uri);
  assert.ok(diags.params.diagnostics.some((d) => d.code === "GM-L0002"), JSON.stringify(diags.params.diagnostics));

  s.send({ id: 2, method: "textDocument/formatting", params: { textDocument: { uri }, options: { tabSize, insertSpaces: true } } });
  const [edit] = (await s.until((m) => m.id === 2)).result;
  assert.equal(
    edit.newText,
    "model small_model\n\n  Real x(start = 1);\n\nequation\n\n  der(x) = -x;\n\nend small_model;\n",
  );

  s.send({ id: 3, method: "textDocument/completion", params: { textDocument: { uri }, position: { line: 3, character: 0 } } });
  const labels = (await s.until((m) => m.id === 3)).result.map((i) => i.label);
  assert.ok(labels.includes("equation") && labels.includes("x"), labels.join(","));

  s.send({ id: 4, method: "shutdown" });
  await s.until((m) => m.id === 4);
  s.send({ method: "exit" });
  await new Promise((r) => s.p.on("exit", r));
});

// 実行には GLOBE_BIN（globe CLI の場所）が要る。例: GLOBE_BIN=…/target/debug/globe pnpm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { chmodSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MAX_BODY, format, lint, readBody } from '../src/lib/globe.ts';

const sample = readFileSync(
  fileURLToPath(new URL('../../../Rust/mod/globe-mode-fmt/tests/data/objects_before.mo', import.meta.url)),
  'utf8',
);

test('format が例題に origin を足す', () => {
  const r = format(sample);
  assert.equal(r.ok, true, r.error ?? '');
  assert.ok(r.text.includes('origin = {0, 0}'), 'origin = {0, 0} が無い');
});

test('format は文法が壊れた入力を ok:false で返す', () => {
  const r = format('model A\n');
  assert.equal(r.ok, false);
  assert.equal(r.text, null);
  assert.equal(typeof r.error, 'string');
});

test('lint は messages の配列を返す', () => {
  const r = lint(sample);
  assert.ok(Array.isArray(r.messages));
  for (const m of r.messages) {
    assert.deepEqual(Object.keys(m).sort(), ['column', 'line', 'message', 'rule']);
  }
});

test('readBody は 1 MiB を超える本文を null にする', async () => {
  const post = (body) => new Request('http://x/api/format', { method: 'POST', body });
  assert.equal(await readBody(post('a'.repeat(MAX_BODY))), 'a'.repeat(MAX_BODY));
  assert.equal(await readBody(post('a'.repeat(MAX_BODY + 1))), null);
});

test('遅い globe は時間切れで投げる', () => {
  const slow = join(mkdtempSync(join(tmpdir(), 'globe-slow-')), 'slow.sh');
  writeFileSync(slow, '#!/bin/sh\nexec sleep 30\n');
  chmodSync(slow, 0o755);
  const saved = { bin: process.env.GLOBE_BIN, timeout: process.env.GLOBE_TIMEOUT_MS };
  process.env.GLOBE_BIN = slow;
  process.env.GLOBE_TIMEOUT_MS = '200';
  try {
    assert.throws(() => format('model A end A;'), /時間内/);
  } finally {
    process.env.GLOBE_BIN = saved.bin;
    if (saved.timeout === undefined) delete process.env.GLOBE_TIMEOUT_MS;
    else process.env.GLOBE_TIMEOUT_MS = saved.timeout;
  }
});

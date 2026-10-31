// 実行には GLOBE_BIN（globe CLI の場所）が要る。例: GLOBE_BIN=…/target/debug/globe pnpm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { format, lint } from '../src/lib/globe.ts';

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

// globe CLI（doc/web-apps.md §1）を子プロセスで呼ぶ。計算は Rust 側だけ。
import { spawnSync } from 'node:child_process';

export type FormatResult = { ok: boolean; text: string | null; error: string | null };
export type LintMessage = { rule: string; line: number; column: number; message: string };
export type LintResult = { messages: LintMessage[] };
export type FormatOptions = { indent?: number; width?: number; insert_origin?: boolean };

export const MAX_BODY = 1 << 20;

// 本文の上限（doc/web-apps.md §3.1）。超えたら null を返す。
// shortcut: 本文を読み終えてから測る（Content-Length が嘘でも測れる）。巨大な本文を読む前に断るならサーバの前段で切る
export async function readBody(request: Request): Promise<string | null> {
  if (Number(request.headers.get('content-length') ?? 0) > MAX_BODY) return null;
  const raw = await request.text();
  return Buffer.byteLength(raw) > MAX_BODY ? null : raw;
}

// 環境変数は呼ぶたびに読む（テストで GLOBE_BIN を変えられるように）
const bin = (): string => process.env.GLOBE_BIN ?? 'globe';

// 終了コード 0 と 1 は正常な結果（1 は文法エラーや lint の指摘）。それ以外は失敗
function run(args: string[], input: string): string {
  const timeout = Number(process.env.GLOBE_TIMEOUT_MS ?? 10000);
  const r = spawnSync(bin(), args, { input, encoding: 'utf8', timeout, maxBuffer: 16 << 20 });
  if (r.error?.code === 'ETIMEDOUT') throw new Error('globe が時間内に終わらなかった');
  if (r.error) throw new Error(`globe を起動できない（${bin()}）: ${r.error.message}`);
  if (r.status !== 0 && r.status !== 1) {
    throw new Error(`globe が異常終了した（終了コード ${r.status}）: ${r.stderr}`);
  }
  return r.stdout;
}

export function format(text: string, opts: FormatOptions = {}): FormatResult {
  const args = ['fmt', '--json', '-'];
  if (opts.indent !== undefined) args.push('--indent', String(opts.indent));
  if (opts.width !== undefined) args.push('--width', String(opts.width));
  if (opts.insert_origin === false) args.push('--no-insert-origin');
  return JSON.parse(run(args, text)) as FormatResult;
}

// API の shape に合わせ、CLI の file 欄は落とす（入力は標準入力だけなので無意味）
export function lint(text: string, disable: string[] = []): LintResult {
  const args = ['lint', '--json', '-'];
  for (const rule of disable) args.push('--disable', rule);
  const parsed = JSON.parse(run(args, text)) as { messages: (LintMessage & { file?: string })[] };
  return {
    messages: parsed.messages.map(({ rule, line, column, message }) => ({ rule, line, column, message })),
  };
}

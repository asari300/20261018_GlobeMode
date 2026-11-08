// globe CLI（doc/web-apps.md §1）を子プロセスで呼ぶ。計算は Rust 側だけ。
import { spawn } from 'node:child_process';

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
// 非同期の spawn（#78。spawnSync は 1 要求の間 Node のイベントループを止め、ほかの要求を全部待たせる）
function run(args: string[], input: string): Promise<string> {
  const timeout = Number(process.env.GLOBE_TIMEOUT_MS ?? 10000);
  return new Promise((resolve, reject) => {
    const p = spawn(bin(), args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let out = '';
    let err = '';
    let done = false;
    const end = (f: () => void) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      f();
    };
    const kill = (e: Error) => end(() => { p.kill('SIGKILL'); reject(e); });
    const timer = setTimeout(() => kill(new Error('globe が時間内に終わらなかった')), timeout);
    p.stdout.setEncoding('utf8').on('data', (d: string) => {
      out += d;
      if (out.length > 16 << 20) kill(new Error('globe の出力が大きすぎる'));
    });
    p.stderr.setEncoding('utf8').on('data', (d: string) => { err += d; });
    p.on('error', (e) => end(() => reject(new Error(`globe を起動できない（${bin()}）: ${e.message}`))));
    p.on('close', (code) => end(() => {
      if (code === 0 || code === 1) resolve(out);
      else reject(new Error(`globe が異常終了した（終了コード ${code}）: ${err}`));
    }));
    p.stdin.on('error', () => {}); // 子が先に終わったときの EPIPE は close で拾う
    p.stdin.end(input);
  });
}

export async function format(text: string, opts: FormatOptions = {}): Promise<FormatResult> {
  const args = ['fmt', '--json', '-'];
  if (opts.indent !== undefined) args.push('--indent', String(opts.indent));
  if (opts.width !== undefined) args.push('--width', String(opts.width));
  if (opts.insert_origin === false) args.push('--no-insert-origin');
  return JSON.parse(await run(args, text)) as FormatResult;
}

// API の shape に合わせ、CLI の file 欄は落とす（入力は標準入力だけなので無意味）
export async function lint(text: string, disable: string[] = []): Promise<LintResult> {
  const args = ['lint', '--json', '-', '--disable', 'connect-type-mismatch']; // #73: 型検査は 1 MiB で 2.6 GB になる
  for (const rule of disable) args.push('--disable', rule);
  const parsed = JSON.parse(await run(args, text)) as { messages: (LintMessage & { file?: string })[] };
  return {
    messages: parsed.messages.map(({ rule, line, column, message }) => ({ rule, line, column, message })),
  };
}

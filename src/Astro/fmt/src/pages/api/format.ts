import type { APIRoute } from 'astro';
import { format, readBody } from '../../lib/globe';

const bad = (error: string) =>
  Response.json({ ok: false, text: null, error }, { status: 400 });

// 整形の結果は CLI の ok/error で返す。HTTP は入力が不正なときだけ 400
export const POST: APIRoute = async ({ request }) => {
  const raw = await readBody(request);
  if (raw === null) return Response.json({ ok: false, text: null, error: '本文が 1 MiB を超えている' }, { status: 413 });
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(raw);
  } catch {
    return bad('JSON を読めない');
  }
  if (typeof body.text !== 'string') return bad('text が文字列ではない');
  const { indent, width, insert_origin } = body;
  if (indent !== undefined && !(Number.isInteger(indent) && (indent as number) >= 0 && (indent as number) <= 16)) {
    return bad('indent が 0〜16 の整数ではない');
  }
  if (width !== undefined && !Number.isInteger(width)) return bad('width が整数ではない');
  if (insert_origin !== undefined && typeof insert_origin !== 'boolean') {
    return bad('insert_origin が真偽値ではない');
  }
  const result = await format(body.text, {
    indent: indent as number | undefined,
    width: width as number | undefined,
    insert_origin: insert_origin as boolean | undefined,
  });
  return Response.json(result);
};

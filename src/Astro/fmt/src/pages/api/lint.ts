import type { APIRoute } from 'astro';
import { lint, readBody } from '../../lib/globe';

const bad = (error: string) => Response.json({ error }, { status: 400 });

export const POST: APIRoute = async ({ request }) => {
  const raw = await readBody(request);
  if (raw === null) return Response.json({ error: '本文が 1 MiB を超えている' }, { status: 413 });
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(raw);
  } catch {
    return bad('JSON を読めない');
  }
  if (typeof body.text !== 'string') return bad('text が文字列ではない');
  const disable = body.disable ?? [];
  // 規則名は小文字とハイフンだけ（CLI に渡す引数を制限する）
  if (!Array.isArray(disable) || !disable.every((r) => typeof r === 'string' && /^[a-z][a-z-]*$/.test(r))) {
    return bad('disable が規則名の配列ではない');
  }
  return Response.json(await lint(body.text, disable));
};

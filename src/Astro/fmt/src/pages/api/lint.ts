import type { APIRoute } from 'astro';
import { lint } from '../../lib/globe';

const bad = (error: string) => Response.json({ error }, { status: 400 });

export const POST: APIRoute = async ({ request }) => {
  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return bad('JSON を読めない');
  }
  if (typeof body.text !== 'string') return bad('text が文字列ではない');
  const disable = body.disable ?? [];
  // 規則名は小文字とハイフンだけ（CLI に渡す引数を制限する）
  if (!Array.isArray(disable) || !disable.every((r) => typeof r === 'string' && /^[a-z][a-z-]*$/.test(r))) {
    return bad('disable が規則名の配列ではない');
  }
  return Response.json(lint(body.text, disable));
};

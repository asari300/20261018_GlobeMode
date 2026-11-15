import type { Backend } from "./backend";

/**
 * S368: 参照・リネームを開いているタブの全部に広げる。離れたタブは Workspace では空（同じクラスの二重定義を避ける。S141）なので、
 * 今の文書と `others`（ほかの自分のタブ）の本文を Workspace に入れて `f` を回し、終わったら（失敗しても）`others` を空に戻す。
 */
export async function acrossDocs<T>(
  backend: Pick<Backend, "setFile">,
  current: { file: string; text: string },
  others: { file: string; text: string }[],
  f: () => Promise<T>,
): Promise<T> {
  await backend.setFile(current.file, current.text);
  try {
    for (const t of others) await backend.setFile(t.file, t.text);
    return await f();
  } finally {
    for (const t of others) await backend.setFile(t.file, "").catch(() => {});
  }
}

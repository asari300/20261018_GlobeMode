// S257: ライブラリの木の行のアイコンを遅延で引く。クラス名で memo し、同時に引くのは limit 本まで（MSL のような大きな木で RPC を撃ち続けない）。
// memo は木のインスタンスごとに作る（ライブラリを読み直すと木が作り直され、捨てられる）。
import type { Backend, Icon } from "./backend";

export function treeIconLoader(backend: Pick<Backend, "icon">, limit = 4): (cls: string) => Promise<Icon | null> {
  const memo = new Map<string, Promise<Icon | null>>();
  const waiting: (() => void)[] = [];
  let running = 0;
  const pump = () => {
    while (running < limit && waiting.length) { running++; waiting.shift()!(); }
  };
  return (cls) => {
    let p = memo.get(cls);
    if (!p) {
      p = new Promise<Icon | null>((resolve) => {
        waiting.push(() => backend.icon(cls).then(resolve, () => resolve(null)).finally(() => { running--; pump(); }));
        pump();
      });
      memo.set(cls, p);
    }
    return p;
  };
}

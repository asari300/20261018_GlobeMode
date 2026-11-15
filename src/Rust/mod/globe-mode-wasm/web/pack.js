// ライブラリの分割配信（S301）の作り方。pack-msl.mjs（UI の配布物）と node/split.mjs（試験）が共有する。
//   const { index, chunks } = pack(files, gm.tree_stub, gm.file_deps);   // files: 名前 → 本文
// index = { chunks: 塊の数, chunk: 名前 → 塊の番号, stubs: 名前 → 見出し, real: 名前 → 本文（見出しにできないファイル）,
//           deps: 塊の番号 → その塊のファイルが直接参照する他の塊の番号の配列（S308。Worker の先読み）, prefetch: 先読みの深さ }
// chunks[i] = 名前 → 本文。同じディレクトリのファイルを cap 文字以内でまとめる（Worker は閉包に要るファイルを含む塊を取る）。
// 先読みの深さ（S308）: 閉包に要る塊から deps を何段たどって、まとめて取るか。測った前後は ADR-0023 の S308。
export const PREFETCH = 2;

export function pack(files, treeStub, fileDeps, cap = 16 * 1024, prefetch = PREFETCH) {
  const stubs = {}, real = {}, chunk = {};
  const chunks = [{}];
  let size = 0, lastDir = null;
  for (const n of Object.keys(files).sort()) {
    const stub = treeStub(files[n]);
    if (stub === undefined) real[n] = files[n];
    else stubs[n] = stub;
    const dir = n.slice(0, n.lastIndexOf("/"));
    if (size > 0 && (dir !== lastDir || size + files[n].length > cap)) { chunks.push({}); size = 0; }
    chunks.at(-1)[n] = files[n];
    chunk[n] = chunks.length - 1;
    size += files[n].length;
    lastDir = dir;
  }
  const deps = chunks.map(() => new Set());
  for (const [n, ds] of Object.entries(JSON.parse(fileDeps(JSON.stringify(files)))))
    for (const d of ds) if (chunk[d] !== chunk[n]) deps[chunk[n]].add(chunk[d]);
  return { index: { chunks: chunks.length, chunk, stubs, real, deps: deps.map((d) => [...d].sort((a, b) => a - b)), prefetch }, chunks };
}

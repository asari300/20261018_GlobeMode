# ADR-0002 Rumoca をアダプタ経由の git 依存で使う

- 状態: Accepted（2026-10-18、S001）

## 背景

- Rumoca（Apache-2.0）は想定以上に成熟している。
  - MSL 4.1.0 の例題では parse 566、simulate 153 まで通る。
  - LSP、WASM、FMI 2 / 3 のテンプレートも持っている。
- 一方で、使うには難点がある。
  - nightly を固定している。
  - crates.io 版は 0.7.28 で止まっている。
  - 開発が速く、API が頻繁に変わる。
  - salsa のようなインクリメンタル計算の基盤が無い。

## 決定

- **Phase A**: Rumoca を git の `rev` 固定で依存させ、`globe-mode-frontend` と `globe-mode-exec` の中に閉じ込める。
  - umbrella crate `rumoca` は使わない。zenoh などを引き込むため。
  - 依存するのは `rumoca-compile`、`rumoca-phase-codegen`、`rumoca-sim`（必要な feature だけ）。
- Rumoca の型は中立 IR（`globe-mode-ir`）に変換してから外に出す。
- **Track B**: 余裕ができたら salsa と rowan で自前のコアを作り、`Frontend` の feature `native` として置き換える。Rumoca は差分テストの基準として残す。
- Rumoca の不具合や不足は、できるだけ上流に PR を出して直す。

## 結果

- P1 で stable でビルドできるか確かめる必要がある（[ADR-0003](0003-toolchain.md)）。
- 月に1回 `xtask bump-rumoca` で更新し、MSL のファネルの差分を記録する。
- 配布物には Rumoca のライセンス文を同梱する。改変した場合は Apache-2.0 §4b に従って明記する。製品名に「Rumoca」を使わない。

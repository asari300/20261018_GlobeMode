---
name: add-dependency
description: GlobeMode に依存ライブラリ（Rust の crate、npm パッケージ、Python パッケージ、Hex パッケージ、Julia パッケージなど）を足す前に、ライセンスを確かめ、cargo deny check を通し、doc/licenses.md を更新し、ADR が要るかを判断する。「crate を足して」「ライブラリを入れて」「依存を追加」と言われたとき、または Cargo.toml / package.json / pyproject.toml / mix.exs / Project.toml の依存を書き換える前に使う。
---

# 依存を足す

GlobeMode は **MIT OR Apache-2.0** で公開する。依存を1つ足すたびに、配布物のライセンスが変わりうる。判断基準は [doc/licenses.md](../../../doc/licenses.md)。

## 1. ライセンスを一次資料で確かめる

- crates.io の API、またはリポジトリの `LICENSE` と `Cargo.toml` の `license` を見る。
  - README の記載だけで決めない。
  - デュアルライセンスなら、どれを選ぶかも書く。
- 推移的依存も見る。

  ```bash
  cargo tree -e normal -i <crate>          # どこから入るか
  cargo deny check licenses bans           # P1 以降
  ```

- npm は `pnpm licenses list`、Python は `uv tree` とパッケージのメタデータで確かめる。

## 2. 区分に当てはめる

| 区分 | 扱い |
| --- | --- |
| 許可（MIT、Apache-2.0、BSD、ISC、Zlib など） | 足してよい |
| 条件付き（MPL-2.0、EPL-2.0） | 改変しないで使う理由を licenses.md に書く。`deny.toml` の `exceptions` に個別に足す |
| LGPL | 任意の feature に限る。既定の配布物には入れない |
| GPL / AGPL / OSMC-PL / NC / source-available | **足さない**。代わりを探すか、ユーザーに相談する |

- 特に注意するもの:
  - Rumoca の umbrella crate `rumoca`（zenoh を引き込む）
  - russell_sparse（GPL の UMFPACK）
  - tree-sitter-modelica（AGPL）
  - Cosmograph（NC）
  - fdg-sim の 0.4 系（GPL）
- `rumoca-*` を足してよいのは `globe-mode-frontend` と `globe-mode-exec` だけ（cargo-deny の `wrappers`）。

## 3. wasm と Android で動くか確かめる

- ブラウザ版に入る crate なら `cargo check --target wasm32-unknown-unknown -p <crate>` を通す。
- C のライブラリやスレッドを前提にするものは、feature で分ける。

## 4. 記録する

- [doc/licenses.md](../../../doc/licenses.md) の「主な依存の判定」の表に1行足す（対象、ライセンス、判定、メモ）。依存を足すのと同じコミットで行う。
- 次のどれかに当てはまるなら ADR を書く（[doc/adr/README.md](../../../doc/adr/README.md)）。
  - 設計の選択肢を減らす依存（UI フレームワーク、ソルバ、パーサなど）
  - 条件付きのライセンス
  - nightly を要求する
- 作業記録の「決めたこと」に、選ばなかった候補と理由を書く。

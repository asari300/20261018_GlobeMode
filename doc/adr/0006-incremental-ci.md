# ADR-0006 CI は差分だけを検査し、大きな変更のときだけ全体を検査する

- 状態: Accepted（2026-10-18）

## 背景

- CI は main への push で回す（ユーザーの決定。PR は使わない）。
- Rumoca を含む Rust のビルドは、キャッシュが無いと 1 ジョブあたり数分かかる。
- Windows のランナーは、非公開リポジトリの無料枠では 2 倍の時間として数えられる。
- ユーザーの要望は 3 つ。
  - CI は更新した部分だけを検査する。
  - 検証済みの依存は省く。
  - 大規模な変更のときだけ全体を検査する。

## 決定

最初の `plan` ジョブが [.github/scripts/ci_plan.py](../../.github/scripts/ci_plan.py) で、何を検査するかを決める。ほかのジョブは、その結果で実行するか省くかを決める。

| ジョブ | 実行する条件 |
| --- | --- |
| `rust-linux` | 変更した crate と、それに依存する crate がある。clippy と test は `-p` で絞る。fmt は安いので全体にかける |
| （wasm32 の check） | core / ir / frontend のどれかが影響を受ける |
| `deny` | 依存の集合を変えた（`Cargo.lock`、`Cargo.toml`、`deny.toml`）。変えていなければ、依存は前回の CI で検証済みとして省く |
| `rust-windows` | 全体を検査するときだけ |
| `python` | `pyproject.toml`、`uv.lock`、`src/Python/` の変更 |
| `docs` | 文書の変更（リンクの検査）。スライドや共通 CSS / JS の変更なら Playwright も |

**全体を検査する（full）条件**:

- 依存・ツール・CI の定義を変えた: `Cargo.toml`、`Cargo.lock`、`mise.toml`、`mise.lock`、`deny.toml`、`.github/`
- 変更したファイルが 40 件を超えた（大規模な変更）
- コミットメッセージに `[full-ci]` がある
- 毎週月曜 03:00 JST の定期実行
- Actions の画面からの手動実行（既定は full）
- 比較元のコミットが無い（新しいブランチ、履歴の書き換え）

**影響範囲の求め方**:

- 変更したファイルが入っている crate を、直接の影響とする。
- `src/Modelica/` はテストが読むので、frontend と cli に影響するとみなす。
- そこから、workspace 内の依存を逆にたどった閉包をとる。
- 各 crate の `Cargo.toml` を直接読むので、cargo が無くても判定できる。

**依存のコンパイル結果**は `Swatinem/rust-cache` で使い回す。`Cargo.lock` が変わらなければ、依存は再ビルドしない。

## 結果

- 文書だけの変更では、Rust のジョブも deny も走らない。
- 葉の crate（cli）だけの変更なら、テストは cli だけになる。
- core を変えたときは全 crate をテストするが、Windows と deny は省く。
- 取りこぼしの危険は、週 1 回の全体検査と `[full-ci]` で抑える。
  - 例: 判定に入っていない種類のファイルが、実はビルドに影響する場合。
- 判定ロジックは [test_ci_plan.py](../../.github/scripts/test_ci_plan.py)（12 件）で守り、plan ジョブの最初に実行する。
- 手元で計画を見るには `mise run ci:plan`（origin/main との差分）。
- しきい値（40 ファイル）や判定の表を変えたら、この ADR と `ci_plan.py` とテストを同じコミットで直す。

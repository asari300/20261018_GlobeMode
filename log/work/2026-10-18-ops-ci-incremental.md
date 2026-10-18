# ops ci-incremental

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/ops-ci-push-main`、`feature/ops-ci-incremental` |
| worktree | `.git-worktree/ops-ci-push-main`、`.git-worktree/ops-ci-incremental` |
| Phase | P1（運用） |
| セッション文書 | S002 の解説書とスライド（S12）に追記した（[解説書](../../doc/sessions/S002-workspace-skeleton/解説書.md#追記-ci-の運用s002-の後同じセッションで)） |
| セッション記録 | [log/sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md](../sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md) |

## 依頼

1. 「今後は方法1で行う」: CI は main を直接 push して回す。
2. 「CI 結果が通ったら 2 つ以上前のワークツリーを掃除。マージまで実行」
3. 「CI は更新部分のみとしたい。依存関係を検証済みの部分はスキップし、大規模な変更時のみ全体テストで効率化することは可能か。可能ならその旨を更新し、マージまで進める」

## 決めたこと

- 1 は、メモリと CLAUDE.md、finish-session スキルに書いた（`feature/ops-ci-push-main`。main にマージ済み、`c514fef`）。
  - 作業の途中で「現在 push しているのでコミットまでにして」と指示があったので、そのときはコミットで止めた。
  - マージは 2 の依頼を受けて行った。
- 2 の worktree の掃除は**保留した**。
  - リポジトリが非公開で、Actions の API が認証なしでは Not Found を返す。そのため「CI が通ったら」の条件を確かめられない。
  - 「2 つ以上前」は「最新から数えて 2 つ以上前」と解釈した。対象は `s001-research-plan`。ユーザーに確認中。
- 3 は可能と判断して実装した（ADR-0006）。
  - 判定のスクリプトは標準ライブラリだけで書いた。各 crate の `Cargo.toml` を tomllib で読む。plan ジョブで Rust を入れずに済み、手元でも同じ判定を見られるため。
  - 全体を検査する条件:
    - 依存・ツール・CI の定義の変更
    - 40 ファイルを超える変更
    - `[full-ci]`
    - 毎週月曜 03:00 JST の定期実行
    - 手動実行
    - 比較元が無いとき
  - しきい値の 40 は、これまでのコミットのうち S001（47 ファイル）を「大規模」、S002（40 ファイル。ただし依存の変更で full になる）以下を「通常」とみなして決めた。
  - Windows は時間がかかる（無料枠では 2 倍で数える）ので、全体を検査するときだけ実行する。
  - `src/Modelica/` はテストが読むので、frontend と cli に影響するとみなした。
  - 「依存関係を検証済みの部分はスキップ」は 2 段で実現した。
    - cargo deny を依存の変更時だけ実行する。
    - コンパイル済みの依存を rust-cache で使い回す。

## やったこと

- `.github/scripts/ci_plan.py`（判定）と `.github/scripts/test_ci_plan.py`（単体テスト 12 件）
- `.github/workflows/ci.yml`
  - 構成: plan → rust-linux / deny / rust-windows / python / docs
  - 起動: schedule と workflow_dispatch を追加し、concurrency で古い実行を取り消す
- `doc/adr/0006-incremental-ci.md`、ADR の索引、architecture の検証基盤の表
- CLAUDE.md、finish-session スキル、`mise.toml` のタスク `ci:plan`
- S002 の解説書の追記とスライド S12

## 検証

- `test_ci_plan.py`: 12 件すべて通過。
- 実際の履歴での判定:

  | 範囲 | 判定 |
  | --- | --- |
  | 文書だけ（`c22e1e2..c514fef`） | docs だけ |
  | `.gitkeep` だけ（`e6788d1..c22e1e2`） | 何も実行しない |
  | S002（`00dba82..36dea44`） | 全体（依存・CI の定義の変更） |
  | S001（`fba8059..e4e00bc`） | 全体（47 ファイル） |

- plan ステップのシェルを、push・新しいブランチ・schedule・workflow_dispatch の 4 通りで模擬した。期待どおりの判定だった。
- actionlint: 指摘 0 件。YAML の構文解析も通過（6 ジョブ、4 種類の起動）。
- **GitHub 上での実行は確かめていない**（この変更は `.github/` を含むので、次の push では全体が検査される）。

## 残したこと

- 2 の worktree の掃除。CI の結果の確認を待っている。
- CI の結果をこちらで確かめる手段。`gh` の導入か、読み取り用のトークンが要る。

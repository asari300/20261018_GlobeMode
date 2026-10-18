# S002 workspace-skeleton

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/s002-workspace-skeleton` |
| worktree | `.git-worktree/s002-workspace-skeleton` |
| Phase | P1 骨格（1 回目） |
| セッション文書 | [doc/sessions/S002-workspace-skeleton/](../../doc/sessions/S002-workspace-skeleton/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md](../sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md)（S001 と同じセッションで続けた） |

## 依頼

「ワークツリー内でコミット、main の変更内容を確認しマージして続けてください。ブラウザ環境は playwright で確認してください。言語管理は mise, uv, cargo となる」

## 決めたこと

### ユーザーの指示に従ったこと

- S001 の取り込みの経緯は、S001 の作業記録の「追記」に書いた。
  - main の `db789c4` を worktree に取り込んだ。
  - 誤って入った gitlink を外した。
  - 衝突2件は、main の内容を含む S001 の版を採った。
  - そのうえで、main に `--no-ff` でマージした（`25f9dff`）。
- 言語の管理は mise・uv・cargo で分担する（ADR-0003 を Accepted にした）。
  - `rust-toolchain.toml` は置かない。
  - Playwright は uv の開発依存として入れた。
  - cargo-deny は mise で入れた。

### 既定のまま進めたこと

- **Rumoca の rev は最新のリリースタグ v0.10.2**（`276e82bd`、2026-10-04）にした。
  - main の HEAD（`74d93b6`）ではなくタグを選んだ。リリースされた版の方が、後で比べるときの基準として分かりやすいため。
- crate は P1 に要る 4 つだけにした（core / ir / frontend / cli）。db・analysis・graph などは P2 以降で作る。
- 検証用のサンプルは MSL を使わない自前の RC 回路にした。MSL の取得は S003 で行う。
- `rumoca_compile` の `ClassType` は公開されていないので、`as_str()` のキーワードで `ClassKind` に対応づけた。
- Rumoca の ToDae の失敗のうち ED001（数が合わない）は、GM-E5002 にした。最初は汎用の GM-E5001 になっていたのを、テストで見つけて直した。
- 保守終了の勧告 2 件（bincode、paste）は、どちらも Rumoca 経由で脆弱性ではないので、理由を書いて例外にした。
- **`publish = false`** にした。git 依存を持つ crate は crates.io に公開できず、cargo-deny が wildcard と判定するため。
- **`uv_build` の範囲を `>=0.13.1,<0.14.0` に直した。**
  - main の `>=0.12.6,<0.13.1` は、mise の uv 0.13.1 を含んでいなかった。そのため、ビルドのたびに uv_build 0.13.0 が PyPI から取得されていた。
  - `mise.toml` のコメント「pyproject.toml の uv_build に合わせる」の意図に沿うと判断した。
- Python のプロジェクト名を `globe-mode` に、スクリプト名を `globe-mode` に変えた。ライセンス（MIT OR Apache-2.0）も追記した。
- workspace の members を `src/Rust/mod/globe-mode-*` に絞った。main の `.gitkeep` を crate と誤認させないため。

## やったこと

- **workspace**:
  - `Cargo.toml`: virtual workspace。lints を共有し、`publish = false`。
  - `src/Rust/mod/globe-mode-{core,ir,frontend,cli}`
  - `src/main.rs` は cli に移した。
- **Rumoca アダプタ**（`globe-mode-frontend/src/rumoca.rs`）:
  - `outline`: AST からクラス・コンポーネント・extends・connect を取り出す。
  - `check`: S0 の構文 → モデル名の確認 → `compile_model_phases` → 方程式数と変数数。
- **CLI `globe`**: `outline` / `check`、`--json`。終了コードは 0 / 1 / 2。
- **サンプル**: `src/Modelica/GlobeModeExamples/package.mo`（RC、Unbalanced）、`src/Modelica/SyntaxError.mo`。
- **テスト**: core 3、ir 2、frontend 5（サンプルを使った結合テスト）。
- **mise**: rust の targets（wasm32-unknown-unknown、wasm32-wasip2、aarch64-linux-android）、cargo-deny 0.20.2、タスク `check` と `docs:check`。`mise.lock` を更新した。
- **deny.toml**、**`.github/workflows/ci.yml`**（3 ジョブ）。
- **Python**: `src/20261018_globemode` を `src/Python/mod/globe_mode` に改名した。pyproject を合わせた。
- **文書**:
  - ADR-0002 / 0003 に結果を書いた。0003 は Accepted にした。
  - architecture・roadmap・licenses・CLAUDE.md・README・`src/*/README.md`・スキルを更新した。
- **セッション文書**: スライド 11 枚、解説書、索引。

## 検証

- `mise run check`: すべて通過。
  - fmt の差分 0
  - clippy の警告 0
  - テスト 10 件すべて通過
  - cargo-deny 4 項目 ok
  - `uv sync --locked` 通過
- Rumoca の stable ビルド（rustc 1.99.1）:
  - ネイティブの `cargo check` が通過（初回 2 分 59 秒）。
  - wasm32-unknown-unknown の `cargo check`（core・ir・frontend）が通過（3 分 18 秒）。
- cargo-deny の wrappers の確認: cli から直接 `rumoca-compile` に依存させると `error[banned]` になった。元に戻すと bans ok に戻った。
- `globe check` の結果:
  - RC: OK。方程式 20 / 変数 20（状態 1・代数 19）。終了コード 0。
  - Unbalanced: S5 で停止（GM-E5002）。終了コード 1。
  - SyntaxError: S0 で停止（GM-E0001、3:8）。終了コード 1。
  - 存在しないモデル: S1 で停止（GM-E1002）。候補を表示した。
- Python: `uv run globe-mode` が動き、`import globe_mode` が `src/Python/mod/` から読み込まれた。
- CI の YAML: 構文の解析が通った（3 ジョブ）。**GitHub 上での実行は確かめていない**（未 push）。
- `check_docs.py`、Playwright、セッション記録の `--check`、ステージした差分の漏洩検査: コミット直前に実行した（結果はコミットメッセージと最終報告にある）。

## 残したこと

- MSL の取得と、例題 20 本での Rumoca との照合（P1 の残りの完了条件）は S003 で行う。
- CI を GitHub で実行していない。
- 個人のメールアドレスが、git の作者と pyproject の authors に残っている。公開前に判断する。
- outline の connect は equation 節の直下だけを拾う（for や if の中は P2）。

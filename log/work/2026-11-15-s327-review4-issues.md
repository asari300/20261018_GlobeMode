# S327 `review4-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s327-review4-issues`（origin/main から切った） |
| worktree | `.git-worktree/s327-review4-issues` |
| Phase | 監査 |
| セッション文書 | `doc/sessions/S327-review4-issues/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

4 回目のレビューの issue #118（LSP の相対 `--library` の URI）・#119（`--solvers -o DIR` の出力先の検査）・#120（ソルバ名の重複の正規化）を 1 件ずつ監査して直す。issue は閉じない。

## 決めたこと

- 3 件とも妥当と判断し、issue の「直し方の案」のとおり最小で直した。表は解説書にある。
- #119 は `prepare_out_dir` を使わず、フォルダを作らない小さな検査（`check_out_dir`）にした。FMU の作り直しの規則が合わないのと、断られた依頼で空のフォルダを残さないため。
- #120 は、1 つだけの `compare` を断る（issue の「断るか明示」のうち断る側）。`CompareRequest` を受ける UI は 2 つ以上を要求済みで、影響しない。

## やったこと

- `globe-mode-lsp/src/lib.rs`: `file_uri` が相対パスに cwd を足す。試験 1 本。
- `globe-mode-cli/src/main.rs`・`simulate.rs`: `check_out_dir`、`--solvers` の `-o` を計算の前に検査、ファイル名の正規化。試験 1 本。
- `globe-mode/src/lib.rs`: `solver_key`、重複・基準の照合の正規化、1 つで比較を断る。`tests/simulate_many.rs` に追加。
- 文書: ADR-0039 に付記（#118）、ADR-0040 に付記（#119・#120）、解説書・スライド、索引。

## 検証

- `cargo test -p globe-mode -p globe-mode-cli -p globe-mode-lsp`: 通った（lsp 71 本、simulate_many 3 本、CLI の simulate 試験を含む）。
- `cargo clippy -p globe-mode -p globe-mode-cli -p globe-mode-lsp --all-targets -- -D warnings`: 0 件。`cargo fmt --check`: 差分なし。
- `cargo fmt --all --check`: 差分なし。`check_docs.py`: 全デッキ OK・リンク切れ 0 件。`render_slides.py`（S327 のデッキだけ）: 問題 0 件。ステージ差分の漏洩検査: 0 件。`mise run check` 全体（workspace の test・deny）は回していない。

## 残したこと

- 読み取り専用の既存フォルダ、大文字小文字を区別しない OS（Windows・macOS）での `-o DIR` の確認。

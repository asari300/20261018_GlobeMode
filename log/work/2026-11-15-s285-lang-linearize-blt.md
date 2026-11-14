# S285 `lang-linearize-blt`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s285-lang-linearize-blt` |
| worktree | `.git-worktree/s285-lang-linearize-blt` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S285-lang-linearize-blt/` |
| セッション記録 | なし（`log/local/` に無い） |

## 依頼

facade の `Workspace::linearize`（S254）・`blt`（S256）を C ABI・Zig・Julia、NIF・Elixir・Gleam から呼べるようにする。

## 決めたこと

- facade に `blt_json`・`linearize_json(model, options)` を置き、C ABI と NIF は呼ぶだけ（`outline_json` と同じ流儀。重複を避ける）。誤りは最初の診断の `コード: 文`（Python のような `.diagnostics` は返さない）。
- `options_json` は `{"time", "parameters"}`、未知のキーは誤り。NULL 可。

## やったこと

- `globe-mode`（`blt_json`・`linearize_json`・`first_diagnostic`）、`globe-mode-c`（`gm_blt`・`gm_linearize`・ヘッダ・README・Rust の試験）、`globe-mode-nif`（`blt`・`linearize`）。
- Zig・Julia・Elixir・Gleam の包みと試験、各 README、features・roadmap の P7・lang の対応表・索引。

## 検証

- `cargo fmt --check` OK、`cargo clippy -p globe-mode -p globe-mode-c -p globe-mode-nif --all-targets -- -D warnings` 0 件。`cargo test -p globe-mode-c` 7 + ヘッダの署名 2 本（新しい `linearize_and_blt_return_json` を含む）、`-p globe-mode` 通過。
- `mise run c:zig`・`c:julia` 成功（release の C ABI。`analysis: A=-2.99999999996, blt=144 bytes`）、`mise run elixir:test` 8 本 0 失敗、`mise run gleam:test` 4 本 0 失敗。
- `cargo deny` は依存を足していないので回していない（`mise run check` 全体は回していない）。
- `check_docs.py`: スライド OK、リンク切れ 0 件。Playwright の `render_slides.py` は回していない。
- セッション記録のコピーと漏洩検査は省いた（記録が無い）。

## 残したこと

- 診断の配列は返さない。固有値は未。

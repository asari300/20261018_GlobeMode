# S220 `lsp-folding-indent`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s220-lsp-folding-indent` |
| worktree | `.git-worktree/s220-lsp-folding-indent` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S220-lsp-folding-indent/` |
| セッション記録 | （親が写す） |

## 依頼

roadmap 末尾の表「機能の穴（P4）エディタの言語サービス」から、折りたたみ（`textDocument/foldingRange`）と VS Code の字下げの規則。Svelte は触らない。

## 決めたこと

- `folding.rs` を新設し、`lib.rs` は能力の宣言（`foldingRangeProvider`）と 1 つの分岐だけ。字句は `globe_mode_fmt::lex`、クラスの範囲は `ws.outline`。桁は出さない（行だけ）。
- クラス・節・annotation は閉じの行を残す。コメントは行コメントの連続と複数行のブロックコメント。
- 字下げは `indentationRules` と `onEnterRules`（`//` の行は字下げしない）。`folding.markers` は入れない。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/folding.rs`（新）、`src/lib.rs`（3 か所）、`tests/folding.rs`（新、1 本）。
- `src/VSCode/globe-mode/language-configuration.json`、`test/indent.test.mjs`（新）、`README.md`。
- `doc/features.md`、`doc/roadmap.md`、`CHANGELOG.md`、セッション文書と索引。

## 検証

- コードの検査（fmt / clippy / test / deny）: `mise run check` が通った（試験 796 本・失敗 0。1 回目は `fmu_export` の `exports_a_fmi3_source_fmu` が単独で回すと通る揺れで、2 回目は共有 target の古いビルドで folding の試験が落ち、`touch` して通った）
- `pnpm test`（`GLOBE_BIN` に自分でビルドした `globe`）: 11 本通過（`indent.test.mjs` の 4 本を含む）
- `check_docs.py`: 相対リンク切れ 0 件、`render_slides.py`（自分のデッキ）: 問題 0 件

## 結果

- `cargo test -p globe-mode-lsp --test folding`: 1 本通過（package 0〜12、model 3〜11、initial equation 5〜6、equation 7〜11、annotation 10〜11、コメント 1〜2。CRLF と全角の説明文字列を含む。壊れた構文ではコメントだけ出る）。
- CI: （親が確かめる）

## 持ち越し

- UI（CodeMirror）の折りたたみ。構文が壊れている間はクラスと節の範囲が出ない。

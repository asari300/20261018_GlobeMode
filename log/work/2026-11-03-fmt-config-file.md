# S130 `fmt-config-file`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s130-fmt-config-file` |
| worktree | `.git-worktree/s130-fmt-config-file` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S130-fmt-config-file/` |
| セッション記録 | （親が写す） |

## 依頼

`doc/features.md` の「整形・lint」の行の「設定ファイルは未」。プロジェクトの設定ファイルで整形と lint の設定を決められるようにする（ruff / rustfmt の流儀）。形は `globe-mode.toml`（対象から上へたどって最初のもの）、`[fmt]` の indent・width・insert_origin・blank_around_sections、`[lint]` の disable。`globe-mode-fmt` の依存 0 を保つか、崩すなら ADR-0024 に理由。新しい外部の依存は足さない。CLI は対象から探して読み、引数が優先。LSP は根の設定を起動時と `didChangeConfiguration` で読む（dispatch の近くは触らない）。wasm は対象外。親（並列の WP の 1 つ）から渡された。

## 決めたこと

- 読む処理は `globe-mode-fmt` の **optional な feature `config`** に置いた（`LintOptions::from_toml`・`LintOptions::find`、`CONFIG_FILE`、規則のコードと名前の表 `RULES`）。CLI と LSP に別々に書くと規則が 2 か所に分かれるため。既定の feature は空なので、既定のビルドと wasm は依存 0 のまま（ADR-0024 に追記）。
- TOML は `toml` 0.8（`Cargo.lock` に rumoca-core の依存として既にある 0.8.23。新しい crate 0）。serde の derive は使わず `toml::Table` から手で取り出し、知らないキーを誤りにする。
- `disable` は名前でもコード（`GM-L0001`〜`GM-L0005`）でもよい。依頼の例はコードだったが、fmt の lint は名前で規則を持っている（コードは LSP の対応だけ）ので、両方を受けて名前にそろえた。
- `format_html` も `[fmt]` に入れた（`Options` の欄と 1 対 1 にするため。依頼の 4 つに 1 つ足しただけ）。
- CLI はファイルごとに探す（ruff と同じ）。`--indent`・`--width` は `Option` にして、指定したときだけ設定ファイルを上書き。`--no-…` は設定ファイルの値を false にするだけ。`--disable` は設定ファイルに足す。
- LSP は `initialize` の `rootUri`（無ければ `rootPath`）から上へたどる。整形の `indent` は設定ファイルがあれば `tabSize` より設定ファイル（VS Code は `tabSize` を必ず送るため）。誤りは `window/showMessage` と既定。`didChangeConfiguration` で読み直して開いている文書の診断を出し直す。`connect-type-mismatch`（GM-L0005）も `disable` で止まる。
- `globe/format`・`globe/lint` の要求は触らなかった（wasm と同じく params だけ。S127 の dispatch とぶつけない）。

## やったこと

- `src/Rust/mod/globe-mode-fmt/`: `Cargo.toml`（feature `config`、optional の `toml`）、`src/config.rs`（新規。`from_toml`・`find`・試験 2 本）、`src/lib.rs`（mod と再公開）、`src/lint.rs`（`RULES`）。
- `src/Rust/mod/globe-mode-cli/`: `Cargo.toml`（feature `config`）、`src/main.rs`（`config()`、fmt・lint のファイルごとの設定）、`tests/fmt_dirs.rs`（試験 1 本）。
- `src/Rust/mod/globe-mode-lsp/`: `Cargo.toml`、`src/lib.rs`（`root`・`config`、`load_config`、`initialize`・`workspace/didChangeConfiguration`、整形と lint の診断、試験 1 本）。
- 文書: `doc/format-style.md`（5 章）、`doc/features.md`、`doc/adr/0024-own-formatter-and-lint.md`、`doc/licenses.md`（toml の行）、`doc/roadmap.md`、索引、セッション文書。

## 検証

- `cargo test -p globe-mode-fmt --features config -p globe-mode-cli -p globe-mode-lsp`: 通過（新しい試験 4 本: fmt `config::tests` 2 本、CLI `config_file_changes_fmt_and_lint`、LSP `config_file_at_the_root_drives_lint_and_formatting`）。`globe-mode-fmt` を feature なしでも clippy 通過（wasm と同じ既定のビルド）
- `mise run check`: fmt・clippy（`-D warnings`）通過、`cargo test --workspace` は 630 本通過・失敗 0。1 回目の doctest `globe-mode` が共有 target の rlib の差し替え（並行の WP のビルド）で E0463 になったので `cargo test --workspace --doc` を回し直して通過。`cargo deny check` は advisories・bans・licenses・sources ok、`uv sync --locked` ok。1 回目の前に共有 target の古いビルド（E0599 `Termination::Cancelled`）で落ちたので core・ir・exec などを touch した
- `check_docs.py`: デッキ NG 0 件、文書 535 件 / 相対リンク 2694 件 / 切れ 0 件
- `render_slides.py`: S130 のデッキ 7 枚 × desktop-light・desktop-dark・mobile で問題 0 件（1 回目と 2 回目は並行の Chromium と重なり 30 s で時間切れ、load 9.5 のときに通過）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- VS Code 拡張の設定 UI と、エディタのオプションとの同期（依頼どおり持ち越し）。
- ブラウザ UI・wasm の設定（ファイルが無いので引数のまま）。
- LSP は根の 1 つだけ（複数のワークスペースフォルダ、サブディレクトリの設定は見ない）。設定ファイルの監視（`didChangeWatchedFiles`）も無い。

# S276 `lsp-tolerant-syntax`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s276-lsp-tolerant-syntax` |
| worktree | `.git-worktree/s276-lsp-tolerant-syntax` |
| Phase | Track B（初めて製品につないだ） |
| セッション文書 | `doc/sessions/S276-lsp-tolerant-syntax/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

構文の誤りがあるファイルでも、LSP の `documentSymbol` と `foldingRange` が出るようにする。Rumoca のアウトラインが引けないときは `globe-mode-syntax` の CST で補う。正常なファイルの出力は変えない。新しい RPC は足さない。

## 決めたこと

- Rumoca のアウトラインが `Err` のときだけ `tolerant::outline`（`globe-mode-lsp/src/tolerant.rs`）を使う。作るのはクラス（名前・種類・`partial`・範囲）と部品（名前・型名・範囲）だけ。範囲は最初と最後の字句。
- `operator` だけのクラスは `class` として出す（正常なファイルでは使わない）。
- ADR は新設せず ADR-0039 に付記。

## やったこと

- `globe-mode-lsp`: `tolerant.rs` 新規、`lib.rs` の `documentSymbol`・`foldingRange` が共通の `outline_or_tolerant` を使う、`Cargo.toml` に `globe-mode-syntax`。
- 試験: `tests/tolerant_outline.rs`（誤りのあるファイルと途中まで書いた入れ子でクラスと部品が出る、正常なファイルは同じ）、`tests/folding.rs` に誤りのあるファイルでクラスと equation 節が折りたためる 1 本。既存の 1 本は、誤りのあるファイルでクラスの範囲が増えた分だけ期待値を直した（正常なファイルの期待値は変えていない）。
- doc: ADR-0039 の付記、roadmap（2 か所）、features、architecture の LSP の行、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-lsp`: 全 pass（lib 55・cross_file 5・folding 2・tolerant_outline 2、ignore 2）。`cargo clippy -p globe-mode-lsp --all-targets -- -D warnings` 警告 0、fmt ok。

## 残したこと

- 補完・定義ジャンプの `globe-mode-resolve` へのつなぎ。誤りのあるファイルの `extends`・`import`・connect。

# S305 `check-native`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s305-check-native` |
| worktree | `.git-worktree/s305-check-native` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S305-check-native/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

自前の段を試しに検査につなぐ。`globe check ... --frontend native`（既定は Rumoca）、診断の番号、`Frontend` の feature `native`、試験、文書。

## 決めたこと

- `Frontend` の trait に feature `native` の実装を足す形が小さく済んだので、その形にした（CLI だけの経路にはしていない）。
- CLI のオプション `--frontend`。未対応は `GM-U0006`・終了コード 3 で、Rumoca に切り替えない。
- 新しい診断の範囲は要らず、`GM-U0006` だけを足した。
- 親の片付けで worktree が一度消えたが、コミット前だったので作り直した（変更は消えていない）。

## やったこと

- `globe-mode-core`: `codes::NATIVE_UNSUPPORTED`（`GM-U0006`）。
- `globe-mode-flat`: `Project::from_dirs_with`（ディレクトリに加えてメモリ上のファイルを読む）。
- `globe-mode-frontend`: feature `native`、`src/native.rs`（`NativeFrontend`）。
- `globe-mode-cli`: `--frontend`、`check_native`、`tests/check_native.rs`（3 本）。
- 文書: ADR-0039 の付記、roadmap・features・architecture、セッションのスライドと解説書、索引。

## 検証

- `cargo clippy -D warnings`（cli・frontend・flat・core、native 有効）: 警告 0。
- `cargo test -p globe-mode-cli -p globe-mode-flat -p globe-mode-core`: 全部 pass（check_native は 3 本）。
- `mise run msl:diff-frontend` は自前の段を変えていない（`from_dirs_with` を足しただけ）ので回していない。

## 残したこと

位置の付与（`GM-E6001` に行・列）、S7 以降、閉包の読み込み、`redeclare`、native を選べる LSP・wasm・Python・UI。

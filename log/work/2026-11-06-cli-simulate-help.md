# S182 `cli-simulate-help`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s182-cli-simulate-help` |
| worktree | `.git-worktree/s182-cli-simulate-help` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S182-cli-simulate-help/` |
| セッション記録 | （親が写す） |

## 依頼

S178 の監査の穴 4〜8（WP の B1・B2）。`simulate` の `-o` なし、書けない出力先を計算の前に、`| head`、負の数、ヘルプ、`--force` の重複と最後の行の理由、info の畳み。やらない: `fmt --check` の標準入力、GM-U の終了コード、`msl fetch`、ヘルプの言語、frontend の診断、Python、Svelte、workflows。

## 決めたこと

- `-o` なしは捨てるが黙らない（stderr に 1 行）。`--json` は stderr にも足さない。
- 出力先の事前検査は追記で開いて、無かったら消す（既存の結果を壊さない）。fmu の `--out` は `create_dir_all` を先にする。
- `| head` は unsafe 禁止・libc なしのため、`println!`・`print!` を置き換えるマクロで BrokenPipe なら `exit(0)`。
- 重複は CLI で除く（facade を直すと wasm・UI に波及し wasm:node・wasm:compare が要る）。`--json` の `refused` の値は変えない。
- info は既定で件数とコードだけ、global の `-v`。人間向けの出力を読む所は 0 件（grep と workflows・xtask・各言語のラッパを確認）。
- `graph` に `-L`。`deps` は `Workspace` がライブラリを別に持ち「0 / 0 ファイル」になるので足さない。

## やったこと

- `globe-mode-cli/src/main.rs`: マクロ、`io_reason`・`io_error`・`check_writable_file`・`prepare_out_dir`、`-v`、`shown`・`hidden_text`、`after_help`、ヘルプから内部番号を除去、`graph` の `-L`。
- `globe-mode-cli/src/simulate.rs`: 事前検査、書いた・書いていない、重複除去、`first_error`。
- 試験: `globe-mode-cli/tests/cli_flow.rs`（新規 7 本）。
- 文書: README の CLI の節、索引、features・roadmap・CHANGELOG、スライドと解説書。

## 検証

- 再現: 直す前の debug の `globe` で S178 の記述どおり（`-o` なし、`-o /nonexistent`、`| head` が終了コード 2、`--stop-time -1`、`--force` の重複）を再現した。
- README の CLI の例を 1 つずつ回した（ChuaCircuit の検査 5.7 s・実行 10.2 s は debug）。
- コードの検査（`mise run check`、fmt / clippy / test / deny / uv sync）: 通った。テスト 685 本 pass、0 fail、13 ignored（CLI の `cli_flow` 7 本を含む）。
- `check_docs.py`: スライド全部 OK、リンク 3734 件中の切れ 0 件。
- `render_slides.py`（S182 のデッキだけ）: 8 枚、問題 0 件（1 回目で通った）。
- セッション記録のコピーと `--check`、漏洩検査、main へのマージ・push は親が行う。
- 回していない: `msl:compare`（CLI の人間向けの出力を読む所が無かった）、`msl:funnel`・`msl:sim`・`wasm:*`（CLI だけの変更）。

## 残したこと

- facade の診断の重複、`fmt` のパス省略、GM-U の終了コード、`msl fetch`、ヘルプの言語。

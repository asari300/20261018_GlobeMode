# S178 `cli-python-flow-audit`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s178-cli-python-flow-audit` |
| worktree | `.git-worktree/s178-cli-python-flow-audit` |
| Phase | 点検 |
| セッション文書 | `doc/sessions/S178-cli-python-flow-audit/` |
| セッション記録 | （親が写す） |

## 依頼

S140・S164 と同じ方法で、UI 以外の面（CLI `globe`、Python `globe_mode` と marimo、他言語）を利用者の目で通しで使い、穴を順位付きで出す。0.2.0 を配る前の点検。コードは変えない。

## 決めたこと

- 共有 target の `globe` が `0.0.1`（2026-11-05 19:01）で古かったので、main の先頭（`b14345f`）で release をビルドし直した（58 s）。`globe --version` は `globe 0.2.0`。
- Python は scratch の venv に `uv pip install ".[widget]"`（共有 target、`CARGO_BUILD_JOBS=2`）。main の `.venv` と worktree の `.venv` は作っていない。worktree に `target/` は出なかった。
- 誤った使い方は 21 通りを表にし、終了コードと出力先を実測した。
- 見つけた不具合は直さず、調査の 8 章に 9 件を記録した（`src/`・`.github/` は変えていない）。
- ユーザーの判断に回したもの: `fmt` のパス省略と非 TTY、未対応の終了コード、FMI のヘッダの同梱、Python の例外の階層、CLI の `msl fetch`、公開、ヘルプの言語。

## やったこと

- `doc/research/cli-python-flow-2026-11-06.md`: 方法、通った流れ、誤った使い方 21 通り、つまずき P1〜P18、順位（上位 13）、WP の案（A〜E）、判断、不具合 9 件、再現手順。
- `doc/sessions/S178-cli-python-flow-audit/`: スライド 9 枚と解説書。
- `doc/sessions/README.md` の索引、`doc/roadmap.md` の「いま残っているもの」に 1 行。

## 検証

- CLI: MSL 5 本の検査 1.19〜1.58 s・実行 1.29〜1.87 s、すべて終了コード 0。誤った使い方 21 通りで panic 0 件。
- Python: README の 5 行が通った。ChuaCircuit を `stop_time=2e6` で回すと bdf・auto・dopri45 の進捗は 5 個（時刻は `None`）で、`True` を返しても 8.67 s で完走。esdirk34 は 22 個中 17 個に時刻。
- 他言語: `c:zig` 12 s、`c:julia` 4 s、`elixir:test` 8 s、`gleam:test` 3 s、すべて成功（共有 target が温まっていた）。
- Rust・`src/` の変更なし。`mise run check` は不要。
- `check_docs.py`: デッキ全件 OK / 文書 728 件・相対リンク 3680 件中の切れ 0 件
- `render_slides.py`（S178 のデッキだけ、cargo が止まってから）: 9 枚・3 表示（desktop-light・desktop-dark・mobile）、問題 0 件。S06・S08 の下の余白は 400 px 以上
- セッション記録の `--check` と漏洩検査: 親がセッション記録を写すときに行う。

## 残したこと

- Desktop・Android・VS Code の拡張は対象外。marimo の HTML をブラウザで開いての見え方は【未確認】。
- 冷えた状態の `uv pip install`・`mise run c:zig` の時間は測っていない。
- 非 UTF-8 の端末（`LC_ALL=C` かつ端末が UTF-8 でない）での読めるかは【未確認】。
- scratchpad の一時ファイル（venv、`w/`）はリポジトリに入れていない。

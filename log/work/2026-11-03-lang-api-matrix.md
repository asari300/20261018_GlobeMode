# S110 `lang-api-matrix`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s110-lang-api-matrix` |
| worktree | `.git-worktree/s110-lang-api-matrix` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S110-lang-api-matrix/` |
| セッション記録 | （親が写す） |

## 依頼

P7「言語 API と UI 比較」の 2 回目。完了条件「参照シナリオが 6 言語で通る」の前段として、各言語の API が出している機能の表を作り、参照シナリオを 1 つ決め、穴を埋める順を決める。文書だけ。cargo のビルドは回さない。置き場は `doc/research/lang-api-matrix-2026-11-03.md`、`doc/features.md` に 1 行、`doc/roadmap.md` は P7 の行と「S110 で済んだこと」だけ。main へのマージ・push・worktree の削除は親がやる。

## 決めたこと

- 表は 13 行（機能）× 10 列（Rust facade・C・Zig・Julia・Elixir・Gleam・Python・wasm・LSP・CLI）。中断は予算（`max_wall_seconds`）を「あり」に数え、途中で止める口が無いことは注に書いた。
- 参照シナリオは S039 の形に format を足した 5 段（open → check → simulate → x(1) → format）。MSL は使わない。graph は Julia 以外にあるので外した。
- 埋める順は facade に `format` / `lint` を足すのが先（束ねは facade だけに依存する方針を保つ）。次に C・NIF・py と Julia の包み、各言語の試験、列の道、FMU、進捗と中断。
- Zig は C のヘッダを translate-c するので、C にある関数はすべて「あり」とした（例が呼ぶのは check と simulate だけ）。Julia は包みのある関数だけを「あり」とした。

## やったこと

- `doc/research/lang-api-matrix-2026-11-03.md`: 表、根拠のファイル、穴の数、参照シナリオ、手元の結果、埋める順。
- `doc/features.md` に言語 API の機能表の行、`doc/roadmap.md` の P7 の行と「S110 で済んだこと」。
- `doc/sessions/S110-lang-api-matrix/`（スライド 7 枚と解説書）、索引の行。

## 検証

- 参照シナリオ（既にある build で）: wasm 5/5 段（x = 0.367879468116207、815 ms）、LSP 5/5、CLI 5/5、Python 4/5（相対誤差 7.3e-8、format が無い）。C・Zig・Julia・Elixir・Gleam はソースで判定（1〜4 はある、format が無い）。
- コードの検査: Rust のコードを触っていないので回していない。
- `check_docs.py`: デッキ OK（S110 を含む）/ 文書 446 件・相対リンク 2185 件中の切れ 0 件
- `render_slides.py`（main の `.venv/bin/python -I`、load average 8.95 で）: 7 枚 × 3 表示で問題 0 件（1 回目は load 8.5 で Chromium の screenshot が 30 s で時間切れ、`pkill` して回し直した）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）

## 残したこと

- 穴を埋めるのは次のセッション（埋める順の 1〜3）。
- main の `.venv` の `_native.abi3.so` は 2026-11-01 の build で `simulate_columns` が無い。`.git-worktree/target-shared/release/globe` は 2026-11-02 の build で `fmu` のサブコマンドが無い。どちらも作り直していない。

# S185 `app-flow-audit-3`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s185-app-flow-audit-3` |
| worktree | `.git-worktree/s185-app-flow-audit-3` |
| Phase | 点検 |
| セッション文書 | `doc/sessions/S185-app-flow-audit-3/` |
| セッション記録 | （親が写す） |

## 依頼

S164 のブラウザ版の穴 1〜10 と S178 の CLI・Python の穴を S166〜S184 の 16 本が手当てした。通しで使ったときに噛み合っているかを、利用者の目でもう一度確かめる（調査だけ。コードは変えない）。(1) 各穴を同じ操作で再現して「直った / 一部 / 直っていない / 新しい問題」の表に、(2) 取り込みの組み合わせの退行・食い違いを見つける。

## 決めたこと

- ブラウザ版は main `2ec6105` を wasm（release、`wasm:build` と同じ環境変数）から作り直し、毎回 localStorage が空の新しい context から Playwright で操作した。MSL の例題は S164 の 7 本と新しい 3 本を、S167 の検索から開いた。
- `/tmp`（tmpfs）が 98% 埋まっていた（親の以前の scratch が約 3.7 GB）ので、Python の venv と Zig の作業ディレクトリは `~/.cache` に作り、終わりに消した。親の scratch には触っていない。
- 見つけた不具合は直さず、調査の 5 章に 10 件（R1〜R10）として記録した。
- ユーザーの判断は roadmap の「いま残っているもの」と重ならない 4 件（Tab キー、検証用ライブラリの整形、FMU のファイル名、プロットの既定の信号）だけを調査の 7 章に書いた。

## やったこと

- `doc/research/app-flow-2026-11-06.md`、`doc/research/app-flow-2026-11-06/`（写真 6 枚、最大 131 KB）
- `doc/sessions/S185-app-flow-audit-3/`（スライド 8 枚、解説書）、`doc/sessions/README.md` の索引
- `doc/roadmap.md` の「いま残っているもの」に 1 行

## 検証

- ブラウザ: 例題 10 本のうち 9 本が通り `Friction` は未対応。検査 0.88〜2.36 s、実行 0.06〜2.38 s、検索から開くまで 0.01〜0.04 s。console の error は全シナリオで 0 件。load average 0.4〜0.8
- CLI: 誤った使い方 21 通りで panic 0 件。9 通りが直り、E4・E5・E10b が直っていない
- Python: README の例が通る。他言語の README の最初の例 5 本が書いてあるとおり動いた
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない
- `check_docs.py`: デッキ 1 件（S185）OK / リンク 3796 件中の切れ 0 件。`render_slides.py`（S185 のデッキだけ）: 8 枚 × 3 画面（desktop-light・desktop-dark・mobile）、問題 0 件
- セッション記録の `--check`: （親が写すので、親が確かめる）
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークンらしき文字列 0 件（`grep` で確認）

## 残したこと

- 見つけた R1〜R10 は直していない。WP の案は調査の 6.2 章（A 文書、B App を順に、C CLI・frontend、D FMU）
- 【未確認】: Desktop・Android・VS Code の実機、スクリーンリーダー、`marimo run` の画面、`.txt` を開いたときの案内、N14（worktree の `uv sync`）
- 検査の秒数が 1.86〜1.87 s に揃う理由は調べていない（推測: ライブラリのタブを切り替えるたびの索引作り）

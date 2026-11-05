# S164 `app-flow-audit-2`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s164-app-flow-audit-2` |
| worktree | `.git-worktree/s164-app-flow-audit-2` |
| Phase | P4・P6 |
| セッション文書 | `doc/sessions/S164-app-flow-audit-2/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S140 の穴 1〜10 が S141〜S161 で埋まったので、同じ方法でいまの main を利用者の目で通しで使い、次に埋める穴を順位付きで出す。ブラウザ版を Playwright で操作し、MSL の例題を 5 本ほど通す。「v0.1 として配れる」までを (a) 最初の 10 分、(b) 機能の穴、(c) ユーザーの判断に分ける。コードは変えない。

## 決めたこと

- ブラウザ版は操作し、Desktop・Android・VS Code は CI とコードから読んで【未確認】を付けた（WSL2 で起動できない）。
- 待ち時間は load 約 3 の値を採った（load 12〜14 では 2〜4 倍遅かった）。
- wasm は共有 target の既存の release（17:49 のビルド）を使い、再ビルドしなかった。S161 の最後のコミット `e1db04a`（18:02）より前のため、参照・リネームの範囲の細部だけ main と違いうるが、今回は使っていない。
- 「v0.1 の範囲」はブラウザ版・CLI・Desktop の 3 つとする案（推測）を置き、決めるのはユーザーとした。
- 整形の Ctrl+Z は自動操作が速すぎて履歴のまとめに当たり、不具合かどうか確かめられなかったので、不具合には数えず【未確認】とした。

## やったこと

- `doc/research/app-flow-2026-11-05.md` と写真 6 枚（`doc/research/app-flow-2026-11-05/`、最大 187 KB）。
- `doc/sessions/S164-app-flow-audit-2/`（slides.html・解説書.md）、`doc/sessions/README.md` の索引、`doc/roadmap.md` の節「S164 の再点検と、次の候補」。
- 見つけた不具合・つまずきは直さず記録した（N1〜N14）。
- 副産物: worktree の `uv sync` が maturin で worktree の中に `target/`（1.6 GB）を作った。使ったあとに消した（N14）。

## 検証

- コードの検査: Rust を触っていないので `mise run check` は回していない。
- `check_docs.py`: デッキ S164 を含め全 OK / リンク 3322 件中の切れ 0 件。`render_slides.py`（S164 のデッキだけ）は 1 回目で問題 0 件（9 枚、明・暗・モバイル）
- セッション記録の `--check`: 親が写すので未実施
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークン 0 件

## 残したこと

- Desktop・Android・VS Code の利用者の操作、整形の undo。
- main に未追跡の `doc/research/upstream/*_jp.md`（他のセッション。触っていない）。

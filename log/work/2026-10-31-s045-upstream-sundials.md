# S045 `upstream-sundials`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s045-upstream-sundials` |
| worktree | `.git-worktree/s045-upstream-sundials` |
| Phase | 運用（[計画書](../../doc/plan/2026-10-30-release-estimate-and-parallel-plan.md)の WP-H） |
| セッション文書 | `doc/sessions/S045-upstream-sundials/` |
| セッション記録 | `log/sessions/2026-10-30-0e301cfb-c9ff-440e-b48e-89148a3cfe93.md`（S033・S035・S038〜S041・S044 と同じセッション） |

## 依頼

計画書の WP-H。(1) msl-sim-failures.md と roadmap 候補 8 から、Rumoca と diffsol への issue の下書きを 1 件 1 ファイルで `doc/research/upstream/` に 9 件以上（英語、再現つき）。(2) 毎週の全体の検査に `--features sundials` のジョブを 1 つ足す。実装はサブエージェント 1 人。

## 決めたこと

- 出すのはユーザー。エージェントは出さない（README の冒頭に書いた）。
- 再現の `.mo` は `doc/research/upstream/` に置き、`src/Modelica/` には置かない。既存の検証モデル（InitRelationBranch・InlinedParameters・ArrayParameters・OuterModification・ModifierScope・DiodeSwitching・TinyStateRestart）を英語のコメントの最上位のモデルにして写した。
- Rumoca の CLI がこの環境に無く、cargo も回さないので、再現は新しく確かめていない。msl-sim-failures.md の数字を出典の S 番号つきで引用し、確かめていない所は [not verified]、diffsol は "not verified against upstream diffsol main" と書いた。
- 下書きは目標の 9 件を超えて 14 件。再現の最小化が済んでいない候補（4・5・7・8・9・20）は書かなかった。
- ジョブ `rust-sundials` は `full == 'true'` のときだけ。`-p` は 3 つの crate を明示（feature は CLI から伝わるが、exec と sim の試験も sundials つきで回すため）。cmake は ubuntu-24.04 のランナーに入っている見込みで、`apt-get` は足さず、初回の run で確かめる。

## やったこと

- `doc/research/upstream/`: README.md、`01`〜`14` の `.md`、`.mo` 7 本（04・05・06・07・08・10・12）。
- `.github/workflows/ci.yml`: ジョブ `rust-sundials`、冒頭のコメントに 1 行。
- `doc/roadmap.md`（候補 8 の末尾、S045 の節）、計画書の WP-H に「済」、`doc/research/msl-sim-failures.md` の冒頭に下書きへのリンク、`doc/sessions/README.md` に S045 の行。
- `doc/sessions/S045-upstream-sundials/`（スライド 7 枚と解説書）。

## 検証

- コード（Rust）は変えていない。cargo は回していない（WSL の RAM 3 GB、別の cargo が並行で動いている）。sundials のビルドは CI の初回の run で確かめる。
- `python3 -I .github/scripts/test_ci_plan.py`: 23 本 OK（ci_plan.py は変えていない）。
- `check_docs.py`: デッキ 48 件 OK / リンク 1097 件中の切れ 0 件（文書 211 件）
- `render_slides.py`: 問題 0 件（S045 のデッキ 1 件。並行する別の Playwright と競合して screenshot が 30 s で時間切れになり、Chromium が静かになってから回し直して通った。S04 の図は目で確かめた）
- ステージした差分の漏洩検査: 0 件
- CI: {{CI}}

## 残したこと

- ユーザーが Rumoca の最新で再現を確かめてから出す。出したら README の表に URL と日付を足す。
- diffsol の 3 件（10〜12）は単体の diffsol のプログラムに縮める。
- 下書きにしなかった候補（4・5・7・8・9・20）と、公開の範囲の依頼（`admitted_native_execution_backend`、`ClosureNoJac`。14 の Notes に 1 行）。
- 初回の `rust-sundials` で cmake の有無、所要時間、rust-cache の大きさを確かめる。

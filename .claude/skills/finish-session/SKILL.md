---
name: finish-session
description: GlobeMode のセッションを締める。コードの検査、スライドと解説書の検査、伏せ字にしたセッション記録のコピーと漏洩チェック、作業記録の仕上げ、feature ブランチへのコミットまでを順に行う。「作業を終わる」「まとめて」「コミットして」と言われたとき、または一区切りついたときに使う。
---

# セッションの締め

順番に通す。**どれかが落ちたら、そこで止めて報告する。** 検査を通さずにコミットしない。

以下はすべて worktree（`$WT`）で実行する。`MAIN` は main のチェックアウト。

```bash
MAIN=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')
```

## 1. コードの検査（Rust・Python・JS を触ったときだけ）

```bash
mise run check              # 下の 4 つと uv sync --locked をまとめて実行する
# cargo fmt --all --check
# cargo clippy --workspace --all-targets -- -D warnings
# cargo test --workspace
# cargo deny check          # ライセンスと bans。doc/licenses.md と食い違ったら止める
```

- workspace ができる前（P0）は不要。
- Python を触ったら `uv run python -m pytest`、Svelte を触ったら `pnpm -C src/Svelte check` も通す（P1 以降に整える）。

## 2. スライドと解説書を検査する

`/session-docs` の手順でスライドと解説書を書き上げてから、検査する。

```bash
python3 -I .claude/skills/session-docs/check_docs.py
uv run python .claude/skills/session-docs/render_slides.py     # Playwright でブラウザ表示を確かめる
```

期待する結果:

- 全デッキが OK（連番、外部参照 0、`<style>` 0、`style=` 0、共通 CSS と JS を読んでいる）
- 相対リンク切れ 0 件（見出しの anchor も含む）
- `render_slides.py` の問題 0 件。スクリーンショットを目で見て、図の文字のはみ出しが無い

## 3. セッション記録をコピーして伏せ字を検査する

フックは main の `log/local/` に整形版を書いている。最新の状態に書き直してから、worktree の `log/sessions/` にコピーする。

```bash
BRANCH=$(git branch --show-current)
MD=$(CLAUDE_PROJECT_DIR="$MAIN" python3 .claude/hooks/save_session_log.py --latest --match "$BRANCH" --out "$MAIN/log/local")
cp "$MD" log/sessions/
python3 .claude/hooks/save_session_log.py --check log/sessions/*.md
```

- 同じプロジェクトで別のセッションが並行して動いていることがある。`--match "$BRANCH"` で、このブランチ名を含む transcript に絞る。セッション ID が分かっているなら `--session <ID>` の方が確実。

- `--check` の取りこぼしが **0 件**であること。1 件でもあれば、フックの `REDACTIONS` に規則を足して作り直す。手で直さない。
- コミットするのはそのセッションの時点の写し。セッションが続く間、main の `log/local/` 側は更新され続ける。
- `log/local/raw/` の生ログは**絶対にコミットしない**（`.gitignore` 済み）。

## 4. 作業記録と索引を仕上げる

- `log/work/<日付>-sNNN-<topic>.md` の「決めたこと / やったこと / 検証 / 残したこと」を埋める。
  - 検証は**数字で**書く。通っていない項目は「通っていない」と書く。
- [doc/sessions/README.md](../../../doc/sessions/README.md) の索引の行を仕上げる（状態、セッション ID、成果物）。
- 計画を変えたなら、[doc/roadmap.md](../../../doc/roadmap.md) と該当する ADR を同じコミットで直す。

## 5. 漏洩を検査してからコミットする

```bash
git add -A
git status --porcelain                                     # 意図していない差分が無いか読む
LEAK="$(whoami)|$(git config user.email)|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9_-]{20,}"
git diff --cached -U0 | grep '^+' | grep -nE "$LEAK" \
  && echo "漏洩の疑いあり。止める" || echo "漏洩の疑い 0 件"
git commit
```

- **feature ブランチにコミットする。`main` へ直接コミットしない。**
- 見るのは、手元のログイン名、git のメールアドレス、長さまで指定したトークンの形。
  - 「/home/」や「@」のような汎用の形では探さない。フックやスキルに書いた正規表現の文字列そのものに当たってしまうため。
- 何か出たら、それが意図した記述かを1件ずつ確かめる。疑わしければ止める。
- コミットメッセージは「何を・なぜ」。末尾に、システムが指示する Co-Authored-By の行を付ける。

## 6. main にマージして push する（CI を回す）

CI は **main を直接 push して回す**（ユーザーの決定。PR は使わない）。

```bash
MAIN=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')
BRANCH=$(git branch --show-current)
git -C "$MAIN" status --porcelain        # main に未コミットの変更が無いか。あれば止めて伝える
git -C "$MAIN" merge --no-ff -m "Merge $BRANCH: <要約>" "$BRANCH"
git -C "$MAIN" push origin main
```

- push の前に、5 の漏洩検査を **main と origin/main の差分**に対しても実行する。
  - `git -C "$MAIN" diff origin/main..main -U0 | grep '^+' | grep -nE "$LEAK"` が 0 件であることを確かめる。
- feature ブランチは push しない。
- CI は差分だけを検査する（ADR-0006）。push の前に `mise run ci:plan` で計画を見る。
  - 大きな作り直しや、判定に入っていない種類のファイルを触ったときは、マージのコミットメッセージに `[full-ci]` を入れて全体を検査する。
- push したら Actions の結果を確かめる（`gh` は認証済み）。

  ```bash
  gh run list --branch main --limit 3
  gh run watch <run-id> --exit-status     # 完了まで待つ。長いのでバックグラウンドで実行する
  gh run view <run-id> --log-failed       # 落ちたジョブのログ
  ```

- CI が落ちたら、新しいセッション（worktree）で直して、同じ手順でもう一度 push する。

## 7. CI が通ったら、2 つ以上前の worktree を片付ける

**CI が成功したときだけ**行う（ユーザーの決定）。失敗したら片付けずに、直すセッションに進む。

- 対象: 作った順に並べて、**最新から数えて 2 つ以上前**の worktree（最新と 1 つ前は残す）。
- 消す前に、それぞれ次の 2 つを確かめる。どちらかを満たさないものは消さずに報告する。
  - 未コミットの変更が無い
  - ブランチが main に取り込み済み

```bash
cd "$MAIN"
git worktree list                                         # 作った順を確かめる（ブランチの作成日でもよい）
for t in <2 つ以上前の topic>; do
  test -z "$(git -C .git-worktree/$t status --porcelain)" || { echo "$t に未コミットの変更"; continue; }
  git branch --merged main --format='%(refname:short)' | grep -qx "feature/$t" || { echo "$t は未マージ"; continue; }
  git worktree remove ".git-worktree/$t" && git branch -d "feature/$t"
done
git worktree prune
```

- ブランチは `-d`（取り込み済みのものだけを消す）で消す。`-D` や `--force` は使わない。
- 作業記録には触らない（`log/work/` は main に残っている）。

## 8. 報告する

ユーザーには次を伝える。

- 何を作ったか・変えたか
- 検査の結果（数字）
- ブランチ名と worktree のパス、main のコミット、CI の結果（run の ID とジョブごとの成否）
- 片付けた worktree と、残した worktree
- 積み残しと、次のセッションでやること

worktree の片付けは 7 の条件（CI が成功、2 つ以上前、未コミットの変更が無い、取り込み済み）を満たすものだけを行う。それ以外は消さない。

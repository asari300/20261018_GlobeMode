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
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check            # ライセンスと bans。doc/licenses.md と食い違ったら止める
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

## 6. 報告する

ユーザーには次を伝える。

- 何を作ったか・変えたか
- 検査の結果（数字）
- ブランチ名と worktree のパス、`main` への取り込み方（`git merge --no-ff feature/sNNN-<topic>`）
- 積み残しと、次のセッションでやること

worktree の片付け（`git worktree remove` / `git branch -d`）は、**取り込みが済んでからユーザーが判断する**。勝手に消さない。

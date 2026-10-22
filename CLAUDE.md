# このリポジトリでの作業ルール

GlobeMode は Rust 製の OSS Modelica IDE。全体像は [README.md](README.md)、設計は [doc/architecture.md](doc/architecture.md)、順序は [doc/roadmap.md](doc/roadmap.md) にある。

以下は**このリポジトリで作業するとき必ず守ること**。

---

## 1. 作業は git worktree の feature ブランチで行う

`main` の作業ツリーを直接編集しない。作業（＝1セッション）ごとに worktree を切る。手順は `/start-session` スキルにある。

```bash
MAIN=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')   # main のチェックアウト
cd "$MAIN"
git worktree add -b feature/<topic> .git-worktree/<topic> main
cd .git-worktree/<topic>
mise trust && mise install --locked   # worktree のパスごとに信頼が要る
uv sync                      # worktree ごとに必須
```

- ブランチ名は `feature/<topic>`。`<topic>` は `sNNN-<英小文字とハイフン>`（例 `s002-workspace-skeleton`）。
- worktree は `.git-worktree/<topic>` に置く（`.gitignore` 済み）。
- **`.venv` を worktree 間で共有・symlink しない。** editable install はそのチェックアウトの `src/` を指すため。
- コミットは feature ブランチに行う。
- **CI は main を直接 push して回す**（2026-10-18 にユーザーが決めた。PR は使わない）。
  - 手順: 検査を通した feature ブランチを main に `--no-ff` でマージし、`git push origin main` する。そのあと Actions の結果を確かめる（`/finish-session` の 6）。
  - feature ブランチ自体は push しない。
  - CI が成功したら、最新から数えて 2 つ以上前の worktree を片付ける（`/finish-session` の 7）。
  - **CI は差分だけを検査する**（[ADR-0006](doc/adr/0006-incremental-ci.md)）。
    - 変更した crate とその依存元だけをテストし、依存を変えていなければ cargo deny を省く。
    - 依存・ツール・CI の定義の変更、40 ファイルを超える変更、毎週の定期実行のときは全体を検査する。
    - 全体を検査したいときは、コミットメッセージに `[full-ci]` を入れる。
    - push の前に `mise run ci:plan` で、何が検査されるかを確かめられる。
- main に未コミットの変更があったら、そこで止めてユーザーに伝える。勝手にコミットも stash もしない。

---

## 2. ディレクトリ構造の意味

| 場所 | 意味 | 編集してよいか |
| --- | --- | --- |
| `src/Rust/mod/<crate>/` | Rust のコア（workspace の library crate） | ○ |
| `src/Rust/apps/` | Tauri のシェル、Leptos / Dioxus の試験版 | ○ |
| `src/Svelte/` | UI（Tauri とブラウザで共用）。**業務ロジックを置かない** | ○ |
| `src/Python/mod/` `marimo/` `streamlit/` | Python パッケージ、ノートブック、ダッシュボード | ○ |
| `src/{Elixir,Gleam,Julia,Zig}/` | 各言語の薄いラッパ。ロジックは Rust 側に置く | ○ |
| `src/Modelica/` | 検証用の Modelica ライブラリ。**MODELICAPATH として開けるパッケージだけを置く**（わざと構文を誤ったファイルは `globe-mode-frontend/tests/data/` へ）。説明は [src/Modelica/README.md](src/Modelica/README.md) | ○ |
| `data/msl/` `data/rumoca/` | MSL 4.1.0 と Rumoca の CLI の取得物（`cargo xtask fetch-msl` / `fetch-rumoca`） | ×（コミットしない） |
| `src/Rust/xtask/` | 開発用のタスク（取得、MSL での突き合わせ、Rumoca の更新） | ○ |
| `doc/architecture.md` `roadmap.md` `features.md` `licenses.md` | **正典**。計画を変えたら同じコミットで直す | ○ |
| `doc/adr/` | 設計判断の記録。覆すときは消さずに Superseded にする | ○（追記） |
| `doc/research/` | 調査の記録。事実ごとに URL と確認日を付ける | ○ |
| `doc/sessions/SNNN-<topic>/` | セッションごとのスライドと解説書 | ○（そのセッションの間だけ） |
| `doc/assets/` | スライドの共通 CSS と JS | ○ |
| `log/work/` | 作業記録（手動、1セッション1枚） | ○ |
| `log/sessions/` | 伏せ字にしたセッション記録（コミットする） | ×（finish-session がコピーする） |
| `log/local/` | フックの出力。生ログを含む | ×（**コミットしない**） |

- Python パッケージは `src/Python/mod/globe_mode`（S002 で `src/20261018_globemode` から改名した）。
- 検査はまとめて `mise run check`（Rust の fmt / clippy / test / deny と `uv sync --locked`）と `mise run docs:check`（スライドとリンク、Playwright）で回せる。
- フロントエンド（Rumoca アダプタ）や Rumoca の rev を変えたら、`mise run msl:compare`（MSL の例題 20 本で Rumoca の CLI と突き合わせ）も通す。
- 検査の段階（S0〜S10）、GM-U の振り分けや一覧（`src/Modelica/msl-known-unsupported.txt`）、Rumoca の rev を変えたら、`mise run msl:funnel`（MSL の全例題 566 本に判定を出し、報告を `doc/msl-funnel.md` に書く。S005 では release で検査だけ約 18 分、S009 では約 16 分、最大 RSS 7.2 GiB）も回し、OK・GM-U・panic の本数の変化を作業記録に書く。CI には入れていない。
- `msl:funnel` のあと、omc（OpenModelica）が入っていれば `mise run msl:omc`（`target/msl-funnel.json` を omc の `checkModel` / `translateModel` と突き合わせ、報告を `doc/omc-compare.md` に書く。S008 では約 7 分）も回し、S5 の一致率を作業記録に書く（[ADR-0010](doc/adr/0010-omc-comparison.md)）。omc は別プロセスで呼ぶだけで、CI には入れていない。
- 実行系（`globe-mode-exec`・`globe-mode-sim`）、Rumoca の rev、diffsol の patch を変えたら、`msl:funnel` のあと `mise run msl:sim`（`msl:funnel` で OK の例題を別プロセスのワーカーで実行し、MAP-LIB の参照結果 v4.1.0 と矩形の管で比べ、報告を `doc/msl-sim.md` に書く。初回は参照結果を約 1 GB 取得する。[ADR-0013](doc/adr/0013-exec-backend.md)）も回し、実行できた本数と管に収まった本数の変化を作業記録に書く。S012 から既定のソルバは `auto`（BDF が失敗したら dopri45 で回し直す）で、時間切れ（`GM-E11005`）と外から止めたものを予算 180 s で回し直し（lowering の段は別の上限 150 s。S015、[ADR-0017](doc/adr/0017-lowering-limit-and-hard-timeout-retry.md)）、前回の `target/msl-sim.json` との差と既知の外れ（`src/Modelica/msl-sim-known-deviations.txt`）を報告に書く。S013 から注釈の Tolerance を参照結果と同じ 0.1 倍にして回す（`--tolerance-scale 0.1`、[ADR-0015](doc/adr/0015-auto-and-reference-tolerance.md)）。CI には入れていない。
- 実行系・`globe-mode-sim`・`globe-mode-wasm`（とその依存の frontend・db）を変えたら、`mise run wasm:node`（release の wasm を Node で回す）と `mise run wasm:compare`（`GlobeModeValidation.Simulation` を実装のあるソルバすべてで wasm とネイティブの両方で回して突き合わせる）も通す（[ADR-0014](doc/adr/0014-session-route-wasm.md)）。CI はジョブ `wasm-node` で Decay だけを回す。
- **言語の管理は mise・uv・cargo で分担する**（[ADR-0003](doc/adr/0003-toolchain.md)、手順は [doc/mise-setup.html](doc/mise-setup.html)）。

  | 道具 | 受け持つもの | ロックファイル |
  | --- | --- | --- |
  | mise | ツールの版（rust / uv / zig / julia / erlang / elixir / gleam）とタスク | `mise.lock` |
  | uv | Python 本体（`.python-version`）と Python の依存 | `uv.lock` |
  | cargo | Rust の依存と workspace | `Cargo.lock` |

  - `rust-toolchain.toml` は作らない（mise の `RUSTUP_TOOLCHAIN` が優先され、無視されるため）。
  - Python を mise に入れない。ツールは `mise.toml` を、依存は各言語の道具を使って足す。
  - **RPi（aarch64、S017 から）では elixir が入らない**（erlang の arm64 ビルドが glibc 2.38 を要求する。[ADR-0019](doc/adr/0019-rpi-aarch64-environment.md)）。`MISE_DISABLE_TOOLS=elixir` を付けて mise を使う。
  - ロックファイルはすべてコミットする。

---

## 3. ライセンス規則

GlobeMode は **MIT OR Apache-2.0**。判断基準は [doc/licenses.md](doc/licenses.md)。依存を足す手順は `/add-dependency` スキルにある。

- **許可**されたライセンスの依存だけを足す。条件付きのもの（MPL / EPL）は licenses.md に理由を書いてから足す。
- **GPL / AGPL / LGPL を静的リンクしない。** KLU（LGPL）は任意の feature に限る。russell_sparse（GPL の UMFPACK）は使わない。
- **Rumoca** は `globe-mode-frontend` と `globe-mode-exec` からだけ使う。
  - umbrella crate `rumoca` は使わない（zenoh などを引き込むため）。
  - 改変したら、そのファイルに明記する。製品名に「Rumoca」を使わない。
- **OpenModelica** は読むだけにする。リンクもコードの複製もしない。`omc` を別プロセスで呼ぶ照合用途だけはよい。
  - `tree-sitter-modelica` と `modelica-language-server` も AGPL / OSMC なので使わない。
- Cosmograph（NC）、Slint（GPL）、symbolica（source-available）は使わない。
- 依存を足したら、同じコミットで `doc/licenses.md` の表を更新する。

---

## 4. 文書の書き方

- 文書は日本語で書く。README は英語の要約を付ける。
- 調査した事実には出典 URL と確認日を付ける。確認できなかったものには【未確認】と書く。推測を事実のように書かない。
- 設計の判断は ADR（`doc/adr/NNNN-*.md`、章は「状態 / 背景 / 決定 / 結果」）に書く。正典の文書からリンクする。
- Markdown は markdownlint の警告を出さない。見出し・リスト・表・コードブロックの前後は空行にする。

---

## 5. セッションごとにスライドと解説書を作る

毎セッション `doc/sessions/SNNN-<topic>/` に次の2つを作る。書き方は `/session-docs` スキルにある。

| ファイル | 内容 |
| --- | --- |
| `slides.html` | HTML スライド。`../../assets/slides.css` と `slides.js` だけを使う。**外部 CDN・`<style>`・`style=` を使わない**（オフラインで開けるように） |
| `解説書.md` | Markdown の解説書。章立ては「目的 / 決定と理由 / 図 / 再現手順 / 次回」。正典の文書へリンクする |

索引は [doc/sessions/README.md](doc/sessions/README.md)。S 番号・日付・ブランチ・セッション ID・成果物を1行ずつ足す。

---

## 6. ログはすべて残す（公開用は伏せ字）

### セッション記録（自動）

`.claude/settings.json` の `Stop` / `SessionEnd` フックが [.claude/hooks/save_session_log.py](.claude/hooks/save_session_log.py) を呼ぶ。

```text
log/local/<日付>-<セッションID>.md           整形版（伏せ字済み）   … コミットしない
log/local/raw/<日付>-<セッションID>.jsonl.gz 完全な生ログ（伏せ字なし）… 絶対にコミットしない
log/sessions/<日付>-<セッションID>.md        finish-session がコピーした公開用 … コミットする
```

- 伏せ字にするもの: メールアドレス、ローカルのユーザー名とホームのパス、トークンらしき文字列。
- コピーした後に `python3 .claude/hooks/save_session_log.py --check log/sessions/*.md` が取りこぼし 0 件であることを確かめてからコミットする。
- フックは main のチェックアウトの `log/local/` に書く。worktree に記録を置くのは finish-session のコピーだけ。こうしておくと、main へのマージで未追跡ファイルとぶつからない。

### 作業記録（手動・1セッション1枚）

`log/work/<日付>-<topic>.md` を [log/work/_TEMPLATE.md](log/work/_TEMPLATE.md) から作る。セッション記録は「何が起きたか」の全量、作業記録は「何を決めたか」を残す。

---

## 7. 締めのチェックリスト

`/finish-session` スキルの手順に従う。**どれかが落ちたらそこで止めて報告する。**

1. Rust のコードを触ったら: `mise run check`（`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、`cargo deny check`、`uv sync --locked`）
2. スライドと解説書がある。`check_docs.py`（外部参照・`<style>`・`style=` が 0 件）と、Playwright の `render_slides.py`（ブラウザでの表示）を通す
3. `doc/` と `log/` の相対リンク切れが 0 件
4. セッション記録をコピーし、`--check` で取りこぼしが 0 件
5. ステージした差分に、メールアドレス・ホームのパス・トークンが 0 件
6. 作業記録を数字で仕上げてから、feature ブランチにコミットする
7. main に `--no-ff` でマージし、`git push origin main` で CI を回して、結果を `gh run watch` で確かめる
8. CI が成功したら、最新から数えて 2 つ以上前の worktree を片付ける（未コミットの変更が無く、main に取り込み済みのものだけ。`git worktree remove` と `git branch -d`）

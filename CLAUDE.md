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
  - **feature ブランチも push してよい**（2026-10-31 にユーザーが決めた）。CI の試行は `git push -u origin feature/<topic>` のあと `gh workflow run <wf>.yml --ref feature/<topic>` で回す（`workflow_dispatch` のあるワークフローだけ）。push の `ci.yml` は main だけで起動する。main に取り込んだら remote の feature ブランチは `git push origin --delete` で消す。
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
| `src/Rust/apps/` | `desktop/`（Tauri のシェル。別の Cargo workspace）、`experimental/{leptos,dioxus}`（UI の試験版）、`web-{actix,axum,loco,rocket}`（整形・lint のウェブアプリ。[doc/web-apps.md](doc/web-apps.md)） | ○ |
| `src/Svelte/` | UI（Tauri〔Desktop・Android〕とブラウザで共用）。**業務ロジックを置かない** | ○ |
| `src/VSCode/globe-mode/` | VS Code の拡張（`globe lsp` を使う。業務ロジックを置かない） | ○ |
| `src/Astro/fmt/` | 整形・lint のウェブアプリ（Astro 版。[doc/web-apps.md](doc/web-apps.md)） | ○ |
| `src/Python/mod/` `marimo/` `streamlit/` `fastapi_fmt/` | Python パッケージ、ノートブック、ダッシュボード、整形・lint のウェブアプリ（FastAPI 版） | ○ |
| `src/{Elixir,Gleam,Julia,Zig}/` | 各言語の薄いラッパ。ロジックは Rust 側に置く。Elixir と Gleam には整形・lint のウェブアプリ（`phoenix_fmt`・`wisp_fmt`）もある | ○ |
| `src/Modelica/` | 検証用の Modelica ライブラリ。**MODELICAPATH として開けるパッケージだけを置く**（わざと構文を誤ったファイルは `globe-mode-frontend/tests/data/` へ）。説明は [src/Modelica/README.md](src/Modelica/README.md)。**このフォルダの `.mo` は `globe fmt` で整形してある。`globe fmt --check src/Modelica` が CI と `mise run check` で通ること。新しい `.mo` は `globe fmt src/Modelica` で整形してからコミットする（位置に依存する試験と snapshot が動く）。** | ○ |
| `data/msl/` `data/rumoca/` `data/reference-fmus/` | MSL 4.1.0、Rumoca の CLI、Reference-FMUs（FMI 2.0 / 3.0）の取得物（`cargo xtask fetch-msl [--version 4.0.0]`＝既定 4.1.0、4.0.0 は別の `--dest` へ。S067 / `fetch-rumoca` / `fetch-reference-fmus`） | ×（コミットしない） |
| `data/libs/` | MSL 以外の公開ライブラリ（Buildings・AixLib・IBPSA・ClaRa・ThermofluidStream・DeltaRobot・Chemical）の取得物（`cargo xtask fetch-library <名前>`。S058） | ×（コミットしない） |
| `src/Rust/xtask/` | 開発用のタスク（取得、MSL での突き合わせ、Rumoca の更新） | ○ |
| `doc/architecture.md` `roadmap.md` `features.md` `licenses.md` `upstream.md` | **正典**。計画を変えたら同じコミットで直す | ○ |
| `doc/format-style.md` `web-apps.md` | 準正典。整形の規則（`globe-mode-fmt`）と、整形・lint のウェブアプリ群の共通設計 | ○ |
| `doc/msl-funnel.md` `msl-sim.md` `msl-solvers.md` `omc-compare.md` `libraries.md` | 検査・実行の報告。`mise run msl:*` と workflow が書く | ×（手で編集しない） |
| `doc/bench/` `doc/plan/` | UI の比較の表（`bench_ui.py report` が書く）、リリースまでの見積もりと並行作業のプラン（計画は変えない） | ○ |
| `doc/adr/` | 設計判断の記録。覆すときは消さずに Superseded にする | ○（追記） |
| `doc/yomiyasu/` | yomiyasu で文書を推敲したときのビフォー/アフター（記録ごとに冒頭へ yomiyasu の版・モデル名・effort。2026-11-08） | ○ |
| `doc/research/` | 調査の記録。事実ごとに URL と確認日を付ける | ○ |
| `doc/sessions/SNNN-<topic>/` | セッションごとのスライドと解説書 | ○（そのセッションの間だけ） |
| `doc/assets/` | スライドの共通 CSS と JS | ○ |
| `log/work/` | 作業記録（手動、1セッション1枚） | ○ |
| `log/sessions/` | 伏せ字にしたセッション記録（コミットする） | ×（finish-session がコピーする） |
| `log/review/` | 監査役（Codex ほか）のレビュー履歴（指摘と採否だけ。1 レビュー 1 枚） | ○ |
| `log/local/` | フックの出力。生ログを含む | ×（**コミットしない**） |

- Python パッケージは `src/Python/mod/globe_mode`（S002 で `src/20261018_globemode` から改名した）。
- 検査はまとめて `mise run check`（Rust の fmt / clippy / test / deny と `uv sync --locked`）と `mise run docs:check`（スライドとリンク、Playwright）で回せる。
- フロントエンド（Rumoca アダプタ）や Rumoca の rev を変えたら、`mise run msl:compare`（MSL の例題 20 本で Rumoca の CLI と突き合わせ）も通す。
- 検査の段階（S0〜S10）、GM-U の振り分けや一覧（`src/Modelica/msl-known-unsupported.txt`）、Rumoca の rev を変えたら、`mise run msl:funnel`（MSL の全例題 566 本に判定を出し、報告を `doc/msl-funnel.md` に書く。S005 では release で検査だけ約 18 分、S009 では約 16 分、最大 RSS 7.2 GiB）も回し、OK・GM-U・panic の本数の変化を作業記録に書く。CI には入れていない。
- `msl:funnel` のあと、omc（OpenModelica）が入っていれば `mise run msl:omc`（`target/msl-funnel.json` を omc の `checkModel` / `translateModel` と突き合わせ、報告を `doc/omc-compare.md` に書く。S008 では約 7 分）も回し、S5 の一致率を作業記録に書く（[ADR-0010](doc/adr/0010-omc-comparison.md)）。omc は別プロセスで呼ぶだけで、CI には入れていない。
- 実行系（`globe-mode-exec`・`globe-mode-sim`）、Rumoca の rev、diffsol の patch を変えたら、`msl:funnel` のあと `mise run msl:sim`（`msl:funnel` で OK の例題を別プロセスのワーカーで実行し、MAP-LIB の参照結果 v4.1.0 と矩形の管で比べ、報告を `doc/msl-sim.md` に書く。初回は参照結果を約 1 GB 取得する。[ADR-0013](doc/adr/0013-exec-backend.md)）も回し、実行できた本数と管に収まった本数の変化を作業記録に書く。S012 から既定のソルバは `auto`（BDF が失敗したら dopri45 で回し直す）で、時間切れ（`GM-E11005`）と外から止めたものを予算 180 s で回し直し（lowering の段は別の上限 150 s。S015、[ADR-0017](doc/adr/0017-lowering-limit-and-hard-timeout-retry.md)）、前回の `target/msl-sim.json` との差と既知の外れ（`src/Modelica/msl-sim-known-deviations.txt`）を報告に書く。S013 から注釈の Tolerance を参照結果と同じ 0.1 倍にして回す（`--tolerance-scale 0.1`、[ADR-0015](doc/adr/0015-auto-and-reference-tolerance.md)）。push の CI には入れていない。手元の RAM が足りないときは `gh workflow run msl-sim.yml`（S037、手動だけ。GitHub Actions のランナーで `msl:funnel` と `msl:sim` を回し、報告と JSON を artifact `msl-sim` に残す。取得は `gh run download <run-id> -n msl-sim`）で回す。同じワークフローのジョブ `split-all`（S043）は、wasm の分割読み込みが全部読みと同じ検査結果になるかを全例題で確かめる（S047 で 566/566 一致、約 1 時間）。
- MSL 以外の公開ライブラリの検査 `libraries.yml`（S058、手動と毎週月曜 06:00 JST）: ライブラリごとに `fetch-library` → `msl-funnel --library data/libs/<名前> --package <根> [--examples-in <段>] --shard i/n` → `msl-sim --library … --package …`（参照結果は無いので実行できたかを数えるだけ）。ジョブの最初に ci.yml が動いていれば待ち（上限 30 分）、funnel・sim は `continue-on-error`。ジョブ `libraries-report`（`cargo xtask libraries-report`）が表 `doc/libraries.md` を artifact `libraries` に出す。取得は `gh run download <run-id> -n libraries`。出典・版・ライセンスは [doc/research/libraries-2026-11-01.md](doc/research/libraries-2026-11-01.md)。
- ソルバの表 `doc/msl-solvers.md`（S050）は `msl-sim.yml` のジョブ `solvers`（`msl-sim` の後、ソルバごとに `msl-sim --solver <名前>`。cvode・ida は `--features globe-mode-exec/sundials`）とジョブ `solvers-report`（`cargo xtask msl-solvers`）が作る。取得は `gh run download <run-id> -n msl-solvers`。ソルバを足す・直したら `gh workflow run msl-sim.yml` で回し直して表を更新する（手元では回さない）。
- Android の APK `android.yml`（S092、手動と main の `src/Rust/apps/desktop/**`・`src/Svelte/**` の変更時）: wasm の UI を作り、`cargo tauri android build --apk --debug`（署名なし、aarch64・x86_64）で artifact `globe-android` を出し、API 34 のエミュレータで起動を確かめる。Android は sidecar を使わず `WorkerBackend`（[ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md)）。取得は `gh run download <run-id> -n globe-android`。
- VS Code の拡張の起動試験 `vscode.yml`（S165、手動と main の `src/VSCode/**` の変更時。S233 から Linux・Windows・macOS の 3 OS の matrix）: `globe` を OS ごとに release でビルドして PATH に置き、拡張を束ね、`pnpm test`（Oniguruma での文法の試験を含む）のあと（Linux は `xvfb-run` で）本物の VS Code（stable）を `@vscode/test-electron` で起動し、言語 ID と activate・構文の誤りの診断・アウトライン・rename を確かめる（`src/VSCode/globe-mode/e2e/`）。手元では VS Code を起動しない。回すのは `gh workflow run vscode.yml`。
- UI の 3 OS 測定 `ui-bench.yml`（S156、手動だけ）: Linux・Windows・macOS のランナーで Svelte・Leptos・Dioxus のブラウザ版（wasm）をビルドし、`bench_ui.py browser --ui <UI>` を 3 回ずつ回して、JSON を artifact `ui-bench-<os>` に出す。取得は `gh run download <run-id> -D ui-bench`、表にするのは `bench_ui.py merge ui-bench/*/ui-bench.json` のあと `report`（`doc/bench/ui.md`。[ADR-0034](doc/adr/0034-ui-bench-method.md) の付記）。ランナーは共有で、値は目安。
- Desktop の bundle `desktop.yml`（S049、手動だけ）: Tauri の .deb・.msi・.dmg を作り、起動の smoke と UI の測定を回して artifact に残す（署名なし）。
- ブラウザ版の配布物 `web.yml`（S145、手動だけ）: 静的ファイルの束を artifact `globe-web` に残す。公開（Pages の有効化・URL の配布）はユーザーの判断。
- FMU の確認 `fmu-export.yml`（S098、手動だけ）: Export した FMU を FMPy と fmusim で確かめる（FMI 2.0・3.0。[ADR-0031](doc/adr/0031-fmi-export-p5a-first.md)）。
- FMI Import の確認 `fmu-import.yml`（S204、手動だけ）: Reference-FMUs の参照の CSV・解析解との突き合わせと `globe fmu info` / `run` の試験を Linux・Windows・macOS で回す（[ADR-0026](doc/adr/0026-fmi-import-placement.md)）。
- リソースの計測 `resources.yml`（S051、手動だけ）: release の `globe` の最大 RSS と、wasm の線形メモリの伸びを測る。
- リリース `release.yml`（S044）: タグ `v*` の push で `globe`・Python の wheel・Desktop の bundle を作って Release を作る。**手動実行は dry run**（artifact に残すだけで、タグも Release も作らない）。**マイナーバージョンを変えるときは必ず dry run を回す**（2026-11-05 にユーザーが決めた）。公開（タグを push する）はユーザーの判断。
- 各 workflow の一覧は [doc/architecture.md](doc/architecture.md) の 2.5 章。
- 実行系・`globe-mode-sim`・`globe-mode-wasm`（とその依存の frontend・db）を変えたら、`mise run wasm:node`（release の wasm を Node で回す）と `mise run wasm:compare`（`GlobeModeValidation.Simulation` を実装のあるソルバすべてで wasm とネイティブの両方で回して突き合わせる）も通す（[ADR-0014](doc/adr/0014-session-route-wasm.md)）。CI はジョブ `wasm-node` で Decay だけを回す。
- C ABI（`globe-mode-c`）とその依存、`src/Zig`・`src/Julia` を変えたら、`mise run c:zig`・`mise run c:julia` も通す（S075）。Elixir の NIF（`globe-mode-nif`）とその依存、`src/Elixir`・`src/Gleam` を変えたら `mise run elixir:test` と `mise run gleam:test` も通す（S082・S087）。CI はジョブ `lang`（S077、`ci_plan.py` の出力 `lang`）で同じものを回す。
- **言語の管理は mise・uv・cargo で分担する**（[ADR-0003](doc/adr/0003-toolchain.md)、手順は [doc/mise-setup.html](doc/mise-setup.html)）。

  | 道具 | 受け持つもの | ロックファイル |
  | --- | --- | --- |
  | mise | ツールの版（rust / uv / cargo-deny / wasm-bindgen / node / pnpm / zig / julia / erlang / elixir / gleam）とタスク | `mise.lock` |
  | uv | Python 本体（`.python-version`）と Python の依存 | `uv.lock` |
  | cargo | Rust の依存と workspace | `Cargo.lock` |

  - `rust-toolchain.toml` は作らない（mise の `RUSTUP_TOOLCHAIN` が優先され、無視されるため）。
  - Python を mise に入れない。ツールは `mise.toml` に、依存は各言語の道具で追加する。
  - **RPi（aarch64、S017 から）では elixir が入らない**（erlang の arm64 ビルドが glibc 2.38 を要求する。[ADR-0019](doc/adr/0019-rpi-aarch64-environment.md)）。`MISE_DISABLE_TOOLS=elixir` を付けて mise を使う。
  - ロックファイルはすべてコミットする。
  - **worktree のビルドは共有の target を使う。** `CARGO_TARGET_DIR=<main>/.git-worktree/target-shared` と `CARGO_BUILD_JOBS=2` を付ける（`uv sync` も。付けないと worktree かリポジトリの根に `target/` ができる）。共有 target の `debug` は消してよい。

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

### レビュー履歴（手動・1レビュー1枚）

`log/review/<日付>-sNNN-<topic>-<レビュアー>.md` を [log/review/_TEMPLATE.md](log/review/_TEMPLATE.md) から作る。監査役（Codex）の指摘を原文のまま貼り、採否と理由を書く。コードの変更は書かない。

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

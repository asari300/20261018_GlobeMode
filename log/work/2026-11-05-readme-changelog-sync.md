# S168 `readme-changelog-sync`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s168-readme-changelog-sync` |
| worktree | `.git-worktree/s168-readme-changelog-sync` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S168-readme-changelog-sync/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S164 の監査の穴 4（N11）。README を S012 の状態から、いまの main の事実（できること・面ごとの使い始め方・既知の限界・開発の入口・ライセンス）に書き直す。CHANGELOG に S048 以降を足す。書くコマンドは手元で 1 回動かして確かめる。版の番号は変えない。`src/`・`.github/` は触らない。

## 決めたこと

- README は英語の要約（状態・できること・始め方の表・限界）を先頭、日本語の本文をその後ろにする（CLAUDE.md 4 章）。日本語の「できること」は features.md の ○ だけ。FMI の読み込み・SSP・Diagram ビュー・署名・公開は「できていないもの」。
- Release・タグ・Pages・PyPI・Marketplace は 0 件（`gh release list` が空、Pages の API が 404、`git tag` が空）なので「入手できる」と書かず、チェックアウトのビルドか Actions の artifact の道だけを書いた。旧 README の Releases の案内は削除した。
- 動かせないもの（Desktop・Android の実機、`gh run download`、`adb install`）は、workflow のコメントと artifact の名前（`gh api .../artifacts` で desktop-linux・desktop-windows・desktop-macos・globe-android・globe-android-release・globe-web を確認）から事実だけを書いた。署名なしで OS が警告を出す点は features.md の記述と Apple の案内に拠る。
- 版の番号は README に書かず、どこも変えない。Cargo・Tauri は 0.0.1、pyproject は 0.1.0、VS Code の拡張は 0.0.5（決めるのはユーザー）。CHANGELOG の「version はまだ 0.0.1」は「Cargo の version」に直しただけ。
- CHANGELOG は `[Unreleased]` のまま。P5 FMI と P6 Web・Android・ノートブックの節を新設し、既存の節（P3・P4・P8・P7）に S048 以降を足した。索引は `doc/sessions/README.md`、確認は `git log --merges --first-parent main`。作業の途中で main に S165 が入ったので、main を取り込んで S165 も足した。
- features.md は、衝突の合成で重なっていた古い行を整理した。新しい行に無い古い行の文（S141 の開く・保存、S159 の Android の中断）は新しい行に戻した。

## やったこと

- `README.md`（全面）、`CHANGELOG.md`、`doc/features.md`（重なった 6 行の整理 2 か所、marimo と Leptos / Dioxus の 2 行の更新）、`doc/roadmap.md`（S168 の節）、`doc/sessions/README.md`（索引）、`doc/sessions/S168-readme-changelog-sync/`（スライドと解説書）、この作業記録。
- README のコマンドの確認は下の「検証」。

## 検証

共有 target（`CARGO_TARGET_DIR=.git-worktree/target-shared`、`CARGO_BUILD_JOBS=2`、load 4 前後）。

- 動かして確かめた: `cargo build --release -p globe-mode-cli`（25 s）、`globe check`（Decay、ChuaCircuit 約 2.8 s）、`globe simulate`（ChuaCircuit 約 3.9 s、50219 行）、`globe simulate --list-solvers`（22 ソルバ）、`globe fmt --check` / `globe fmt` / `globe lint`、`globe fmu export`（2.0・3.0）、`globe lsp`（initialize の応答に rename まで）、`globe outline`・`graph`。`cargo xtask fetch-msl --dest <空のフォルダ>`（sha256 一致、4213 ファイル）。
- ブラウザ: `pnpm install --frozen-lockfile`、`pnpm build`、`python3 -m http.server -d dist`。Playwright の Chromium で `?smoke=1` が `GM-SMOKE check=ok simulate=ok … tree=ok fmt=ok graph=ok cancel=ok settings=ok plot=ok def=ok isolated=0 cancel_path=restart`、検査と実行でプロットの canvas。wasm は同じ日に別のセッションが作った `wasm-pkg`（17:49）を使い、`mise run wasm:build` は回していない。
- VS Code: `mise exec -- pnpm install --frozen-lockfile` と `pnpm package` で `globe-mode-0.0.5.vsix`（main を取り込んだ後にもう一度）。
- Python: 5 行の例が `True` と `(50219, 45) ['time', 'L.i', 'L.v']`。main の `.venv` の native（11 月 1 日）は `simulate_columns` が無く古いので、この checkout から `cargo build --release -p globe-mode-py`（pyo3 の abi3-py312 の設定ファイルを scratch に置いて）作った native を scratch のパッケージに置いて回した。最初に設定なしでビルドした共有 target の `lib_native.so` は Python 3.12 と合わず（`unknown slot ID 4`）、作り直した。maturin はフィンガープリントの違いで次回に作り直す。
- 他の言語: `mise run c:zig`、`c:julia`、`elixir:test`、`gleam:test` が通った。
- 動かさなかった: `uv sync`（worktree に `target/` を作るため）、Desktop・Android の実機とインストール、`gh workflow run`・`gh run download`、`adb install`、`apt install`、`code --install-extension`、`globe flat`・`deps`。
- `check_docs.py`: リンク 661 件、切れ 0 件（README・CHANGELOG・S168 の追加後は下で再確認）。
- MSL の数字は `doc/msl-funnel.md`（566 本、OK 369・NG 13・GM-U 184）と `doc/msl-sim.md`（2026-11-03、356 本のうち実行 312、管 303 / 311）。
- セッション記録の `--check`: 親が写す。

## 残したこと

- 版の番号の決定（Cargo・Tauri 0.0.1、pyproject 0.1.0、拡張 0.0.5）と、公開（Release・Pages・PyPI・Marketplace）はユーザー。公開したら README の「公開前」を書き換える。
- README の Desktop・Android の入れ方は、実機で確かめていない。
- features.md の「01〜16 の再確認は未」は S162 の担当なので触っていない。
- markdownlint は手元に無く、見出し・表・リスト・コードブロックの前後の空行は目で確かめた。

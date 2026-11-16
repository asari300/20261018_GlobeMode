# S380 `release-licenses`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s380-release-licenses` |
| worktree | `.git-worktree/s380-release-licenses` |
| Phase | P8（公開に向けた持ち越し） |
| セッション文書 | `doc/sessions/S380-release-licenses/` |
| セッション記録 | なし（専用の記録が無い。親のセッション `log/sessions/2026-11-02-a2580a13-…` に含まれる。そのファイルは親が更新するので、ここでは触らない） |

## 依頼

S380。release.yml が作る配布物（`globe`・Python の wheel・Desktop の bundle・web）に、サードパーティのライセンス表と CycloneDX の SBOM を artifact として添える。足りない分だけを最小で足し、既にあるものは再利用する。ツールを足すなら mise.toml に版を固定し、`/add-dependency` の基準でライセンスを確かめ、doc/licenses.md を同じコミットで更新する。npm 側は `src/Svelte` から。GPL などの禁止ライセンスが混じっていたら止めて報告。手元で試し、release.yml は feature ブランチを push して dry run で確かめる。タグ・Release・公開・鍵は触らない。

## 決めたこと

- すでにあったもの: `globe` の `THIRD-PARTY.md`（S052）、wheel の `THIRD-PARTY-wheel.md`（S074）。無かったもの: Desktop・wasm・npm の表と、SBOM 全部。web は release.yml ではなく web.yml が作るので、web.yml にも同じ道具で入れた。
- Rust の SBOM は cargo-cyclonedx 0.5.9。**0.5.10 は 2026-11-06 公開で 2 週間たっていないので避けた**。mise の `github:` の道（wasm-bindgen と同じ）で入れ、mise.lock に checksum と provenance。`cargo install` の 2 分のコンパイルが要らない。cargo-about の入れ方（`cargo install --version`）は今までのまま。
- SBOM は CycloneDX 1.5 の JSON。Rust は配布する target ごと（`about.toml` の targets と同じ。`globe` と wheel が 4、Desktop が 3、wasm が 1）。npm は `pnpm sbom`（pnpm 12.14.0 に入っている）、Python は `uv export --format cyclonedx1.5`（uv 0.13.1）。新しい依存は cargo-cyclonedx だけ。
- npm の表は、dev 依存も含める（Svelte の実行時が dist に入るため。prod だけだと 21 個で足りない）。許可外の判定は deny.toml の allow と同じ集合に、ビルド時だけの `lightningcss` 用の MPL-2.0 を足したもの。
- Desktop の bundle・wheel の中への同梱はしない（Tauri の `bundle.resources` が要る。持ち越し）。artifact と Release の資産に付けるだけ。
- `about.hbs` の前書きを `globe` のバイナリに限らない書き方にした（wheel・Desktop・wasm に同じテンプレートを使うため）。

## やったこと

- `.github/scripts/make_sbom.sh`（新）、`.github/scripts/npm_licenses.py`（新。`--test` を `mise run check` に追加）、`src/Rust/apps/desktop/about.toml`（新）、`about.hbs`。
- `.github/workflows/release.yml`: ジョブ `third-party` に Desktop・wasm・npm の表、新ジョブ `sbom`、`release` ジョブが `third-party`・`sbom` を取り込んで SHA256SUMS の対象に入れる。`.github/workflows/web.yml`: `globe-web/` に wasm・npm の表と SBOM。
- `mise.toml`・`mise.lock`（cargo-cyclonedx）、`doc/licenses.md`（ツールの行、3 の監査、4 の続きの節）、`doc/release.md`（7 と 8）、`doc/roadmap.md`（S052・S135 の持ち越しに打ち消し線、S380 の節）、`doc/architecture.md`、`CHANGELOG.md`、`CLAUDE.md`、セッション文書、索引。
- 最初の dry run（run 43870042990）は、npm の表で `pnpm install` の出力が JSON に混ざって失敗した。出力を stderr へ回して直した。

## 検証

- 手元: `cargo about` で Desktop・wasm の表、`npm_licenses.py` で npm の表、`make_sbom.sh` で SBOM 14 個（Rust 12・npm 1・Python 1。どれも CycloneDX 1.5 の JSON として読める）。`npm_licenses.py --test`・`test_ci_plan.py` が通る。cargo-cyclonedx は `mise install` で入れた実行ファイルと、`cargo install` で作ったものの結果が同じ（wasm の components 数）。
- 許可外のライセンス（ライセンス表）: **0 件**。cargo-about は `--fail` つきで 5 つとも成功（`THIRD-PARTY.md` 359 KB・wheel 357 KB・Desktop 574 KB・wasm 302 KB・npm 240 KB）。内訳: globe は Apache-2.0 252・MIT 77・Unicode-3.0 19・Zlib 3・ISC 2・BSD-2-Clause 1、Desktop は Apache-2.0 236・MIT 111・Unicode-3.0 19・ISC 18・MPL-2.0 5（改変せずに使う 5 crate）・BSD-3-Clause 4・Zlib 2、wasm は Apache-2.0 212・MIT 63・Unicode-3.0 19・Zlib 3・BSD-3-Clause 1・ISC 1、npm（66 パッケージ）は MIT 58・Apache-2.0 4・MPL-2.0 2（`lightningcss`、ビルド時だけ）・ISC 1・BSD-3-Clause 1。
- SBOM の license 欄を全部読んだ（14 個・約 5.4 MB）: GPL 系は `self_cell` の `Apache-2.0 OR GPL-2.0-only` の 1 つだけ（Apache-2.0 を選ぶ。cargo-about は通っている）。Python の SBOM は uv の出力に license が無い。手元で入れて調べた 46 配布物は GPL 系なし。例外は docutils（分類子に GPL・BSD・Public Domain が並ぶ。wheel には入らず、marimo 経由で pip が入れる。一次資料の確認は未）。
- dry run（Release は作らない）:
  - run 43870042990: `third-party` が失敗（`pnpm install` の出力が npm の表の JSON に混ざった）。直した。
  - run 43870626993（feature ブランチ）: `third-party`・`sbom`・Desktop 3 つ・`globe` の build 3 つが success。**`build (aarch64-unknown-linux-gnu)` が failure**。原因は wheel の pytest `test_export_fmu_p5b`（`--target aarch64-linux: FMI 2.0 に置き場の名前が無い`）で、**この作業の前から main でも落ちている**（main の run 43868055458 も同じ。最後に成功した dry run は 11-08）。`release` ジョブは needs の失敗で skipped。
  - run 43875849826（一時のブランチ `tmp/s380-release-check`。wheel の pytest の step だけ `continue-on-error: true` にしたもの。main には入れず、ブランチは消した）: **全ジョブ success**（`release` も）。`SHA256SUMS` は 30 行（`globe` 4・wheel 4・Desktop 3・`THIRD-PARTY*.md` 5・`sbom-*.cdx.json` 14）。Release の step は skipped（手動）。
  - web.yml（run 43870083855、feature ブランチ）: success。`globe-web/` に `THIRD-PARTY-wasm.md`・`THIRD-PARTY-npm.md`・`sbom-wasm-wasm32-unknown-unknown.cdx.json`・`sbom-npm.cdx.json` が入った。
- コードの検査（`mise run check`）: Rust のコードは触っていないので回していない。main への push の CI は全体の検査（`.github/`・`mise.toml` の変更）になる。
- `check_docs.py`: デッキ（全部）OK / リンク 5678 件中の切れ 0 件。`render_slides.py`（S380 のデッキ）: 問題 0 件（図の箱の文字がはみ出していたので広げた）。
- セッション記録: 専用の記録が無いので、コピーしない（親の記録を触ると、並行のセッションとぶつかる）。`save_session_log.py --check log/sessions/*.md`: 取りこぼし 0 件。
- ステージした差分の漏洩検査（ログイン名・メールアドレス・トークンの形）: 0 件。

## 残したこと

- Desktop の bundle・wheel の中へのライセンス表の同梱、`sundials` ビルドの表、SBOM の署名・検証。
- Python の依存（`sbom-python`）には、uv の出力の仕様でライセンスの欄が無い。表は作っていない（wheel に入らない）。調べた結果は「検証」。
- **既存の不具合（S380 とは別）**: aarch64 Linux の wheel の pytest `test_export_fmu_p5b`（`ws.export_fmu(..., runtime="p5b", runtime_lib=…)`）が `FMI 2.0 に置き場の名前が無い` で落ちる（`globe-mode-fmi/src/export.rs` の `platform` が `(aarch64, linux)` の FMI 2.0 を断る）。release.yml の dry run の `build (aarch64-unknown-linux-gnu)` が落ち、`release` ジョブが skipped になるので、タグを打つ前に直す必要がある。FMI 2.0 には aarch64 Linux の置き場の名前が無いので、aarch64 Linux の host で FMI 2.0 の p5b を書くときの扱い（断る・FMI 3.0 にする・試験を host で分ける）を決める。

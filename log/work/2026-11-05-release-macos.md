# S176 `release-macos`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s176-release-macos` |
| worktree | `.git-worktree/s176-release-macos` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S176-release-macos/` |
| セッション記録 | （親が写す） |

## 依頼

S160 の持ち越し。`release.yml` に macOS（arm64）の `globe` の CLI・wheel と Desktop の .dmg を足す。既存の matrix に行を足す形。署名・公証・鍵、x86_64 の macOS、タグ・Release、`desktop.yml`・`ci.yml`、UI や Rust の本体は対象外。確かめ方は feature を push して `gh workflow run release.yml --ref feature/s176-release-macos`（dry run。ユーザーが 2026-11-05 に許可）。

## 決めたこと

- matrix に 1 行ずつ（共通化の作り替えはしない）。Rust は全 OS で rustup、macOS の uv は mise（brew 管理の python3 では pip が使えない。`mise.lock` に macos-arm64 の uv がある）。
- wheel は maturin の既定の tag で足りた（`macosx_11_0_arm64`）。zig と `--compatibility` は Linux だけ。
- タグの版の置換を GNU sed の `0,/re/` から perl にした（BSD sed で動かないため。3 OS に perl がある）。
- `about.toml` の targets に `aarch64-apple-darwin` を足した（macOS にだけ入る crate の表からの漏れを防ぐ。`third-party` は `--fail` 付きで通った）。
- Desktop は `--bundles app,dmg`（S160）。macOS の updater は `.app.tar.gz` を読むので、鍵があるときの成果物を `GlobeMode_<版>_aarch64.app.tar.gz`（と .sig）に直して `dmg/` に置き、`latest.json` に `darwin-aarch64` を足せるようにした。鍵が無い今は署名なしの .dmg だけ。
- x86_64 の macOS は足さない（ランナーと費用）。

## やったこと

- `.github/workflows/release.yml`（冒頭のコメント、`build` と `desktop` の matrix、rustup と uv の step、perl、`.app.tar.gz` のリネーム、upload の path、`latest.json` の key）、`about.toml`。
- 文書: `doc/features.md`（Desktop と Python の行）、`doc/roadmap.md`（S176 の節）、`doc/adr/0023-*.md` の付記、`README.md`、`CHANGELOG.md`、`doc/sessions/S176-release-macos/`、`doc/sessions/README.md`。
- 試走: 1 回目 run 42042349155 は、変更をコミットする前に feature ブランチを push して dispatch したため main と同じ workflow が動いた（macOS のジョブが無い）。cancel して出し直した。2 回目 run 42043702702 は全ジョブ success だったが、desktop の artifact に `dmg/`・`deb/`・`msi/` の階層が付き、`release` ジョブの `sha256sum ... GlobeMode_*` が「No such file」になっていた（`| tee` に隠れて step は success）。upload の path に `bundle/macos/` を足したため、upload-artifact が共通の親（`bundle/`）を根にしたのが原因。`.app.tar.gz` を `dmg/` に移し、path を `bundle/${{ matrix.bundle }}/` の下だけにして直した。3 回目 run 42048206686 で確かめた。

## 検証

- `python3 .github/scripts/test_ci_plan.py`: 25 本 OK。workflow の YAML は PyYAML で読めた。
- dry run 42048206686: 全 10 ジョブ success（macOS の build・desktop を含む）。artifact は `globe-desktop-*` が階層なしで、`SHA256SUMS` は 11 行（`globe` 4・wheel 4・`GlobeMode_*` 3）。Release の step は skipped
- macOS の成果物: `globe-0.2.0-dryrun-aarch64-apple-darwin.tar.gz` 18,175,599 バイト、`globe_mode-0.2.0-cp312-abi3-macosx_11_0_arm64.whl` 17,598,897 バイト、`GlobeMode_0.2.0_aarch64.dmg` 21,945,857 バイト（3 回目の run の値。2 回目とは数バイト違う）。
- `gh release list` と `git ls-remote --tags origin`: 0 件のまま。
- `check_docs.py`: デッキ 1 件 OK（S176 を含む全デッキ OK）/ リンク 724 件の文書・相対リンク 3657 件中の切れ 0 件
- `render_slides.py`: S176 のデッキ 1 件 / 問題 0 件（1 回目で通った）
- セッション記録の `--check`: （親が写すので、このセッションではコピーしない）
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークンが 0 件

## 残したこと

- 署名・公証の鍵（ユーザーの判断）。登録後に dry run で `.app.tar.gz`・`.sig`・`latest.json` の `darwin-aarch64` を確かめる。
- タグの経路（perl の版の置換、リネーム、latest.json）は dry run で通っていない【未確認】。
- x86_64 の macOS、Finder から開く経路と自動更新の実機確認、macOS の wheel の PyPI（非公開のまま）。
- `release` ジョブの `sha256sum ... | tee` は pipefail が無く、失敗が隠れる。今回は直さなかった（足すなら別の変更）。

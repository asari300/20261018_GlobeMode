# S195 `release-tag-path`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s195-release-tag-path` |
| worktree | `.git-worktree/s195-release-tag-path` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S195-release-tag-path/` |
| セッション記録 | （親が写す） |

## 依頼

`release.yml` のタグの経路を、タグを打たずに確かめられるようにし（入力 `tag`）、Opus のレビュー（指摘 1〜9）を 1 件ずつ確かめて直す。タグ・Release・公開はしない。

## 決めたこと

- 版は書き換えず、ファイルが決める。タグの版（接尾辞を除く）と 4 か所が違えば最初のジョブで止める。理由: 指摘 1（rc と msi）が run で再現した。書き換えを消せば試験の無いコードが減り、2.4 の表を揃えてから打つ運用と合う。
- pre-release のタグは `v0.2.0-rc1` の形のまま。接尾辞は名前にだけ出て、版は 0.2.0。数字に直す案（msi 用の変換）は書き換えのコードが増えるので採らなかった。
- 手動実行は Release を作らない（`github.event_name == 'push'` だけ）。
- softprops は v2.6.2 のコミットで固定（v3 には上げない）。

## やったこと

- `.github/workflows/release.yml`: 入力 `tag`、env `TAG`、版の突き合わせの step、perl と `--config` の version の削除、鍵の env を bundle の step へ、SHA256SUMS に `shell: bash`、softprops の固定、tauri-cli 2.12.4・cargo-zigbuild 0.24.1、`desktop-ui` を `third-party` の後に。
- `.github/scripts/check_release_tag.py`（`--test` つき）。
- `doc/release.md`（新規）、`doc/architecture.md`・`README.md`・`CHANGELOG.md` のリンクと文面。
- レビュー: `log/review/2026-11-06-s195-release-tag-path-opus.md`。

## 検証

- 直す前（途中のコミット d078cf9、入力 `tag` だけ入れた版）: `tag=v0.2.0` は run 42157306409 が全 10 ジョブ success（windows の wheel の perl `-0pi` も通った＝指摘 3 は再現せず）。`tag=v0.2.0-rc1` は run 42157318748 で desktop(windows) が `failed to bundle project: optional pre-release identifier in app version must be numeric-only and cannot be greater than 65535 for msi target` で落ち、release ジョブは skipped（指摘 1 を再現）。
- 直した後（コミット 25ec267）: `tag=v0.2.0` は run 42160739628、`tag=v0.2.0-rc1` は run 42160753960 が、どちらも全 10 ジョブ success。`tag=v0.2.1` は run 42160725471 で最初のジョブ `third-party` が 4 か所の版の違いで落ち、`desktop-ui`・`desktop`・`build`・`release` は skipped。
- 成果物の名前: `tag=v0.2.0` は `globe-0.2.0-<target>.tar.gz|zip` 4 つ、`globe_mode-0.2.0-…whl` 4 つ、`GlobeMode_0.2.0_aarch64.dmg`・`_amd64.deb`・`_x64_en-US.msi`。`tag=v0.2.0-rc1` は `globe-0.2.0-rc1-<target>…` 4 つで、wheel と Desktop は `0.2.0` のまま。artifact 名（`globe-<target>` など）にタグは入らない。
- 「Release に付ける（タグの push のときだけ）」の step は 2 つの run とも skipped。`gh release list` 0 件、`git ls-remote --tags origin` 0 件のまま。
- `python3 .github/scripts/check_release_tag.py --test` ok、`python3 .github/scripts/test_ci_plan.py` 25 件 OK、`check_docs.py` 切れ 0 件。

## 残したこと

- タグ・Release・鍵は作っていない。鍵が無いので署名・`latest.json`・`.app.tar.gz` の名前直しの道は空振りのまま（【未確認】）。
- 打ち直しの softprops の動き【未確認】。

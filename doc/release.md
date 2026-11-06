# リリースの手順（S195）

タグ `v*` を push すると [`.github/workflows/release.yml`](../.github/workflows/release.yml) が `globe`・Python の wheel・Desktop の bundle（.deb・.msi・.dmg）を作り、GitHub の Release に付ける。**タグを打つか、公開するかはユーザーの判断**（エージェントは打たない）。まだタグは 1 つも無い（2026-11-06）。

## 1. 版を揃える

Cargo・Tauri・Python・npm などの版は [architecture.md の 2.4（版を書いている場所）](architecture.md)の表を全部同じに直し、lock を更新してコミットする。`release.yml` は版を書き換えない。

タグの版（`v` と `-` 以降を除いたもの）が `Cargo.toml`・`src/Rust/apps/desktop/Cargo.toml`・`tauri.conf.json`・`pyproject.toml` の version と違えば、最初のジョブ `third-party` が止まる（`python3 .github/scripts/check_release_tag.py v0.2.0` を手元でも回せる）。

## 2. dry run（タグの経路の予行）

マイナーバージョン（0.x.0）を変えるときと `release.yml` を変えたときは、**必ず**タグを打つ前に feature ブランチか main で回す。

```bash
gh workflow run release.yml --ref <ブランチ> -f tag=v0.3.0        # タグの経路（Release は作らない）
gh workflow run release.yml --ref <ブランチ> -f tag=v0.3.0-rc1    # pre-release のタグの形
gh workflow run release.yml --ref <ブランチ>                      # 入力なし。アーカイブ名が <版>-dryrun
```

入力 `tag` は workflow の env `TAG` に入り、push のタグと同じ道（版の突き合わせ、アーカイブ名にタグの版）を通る。**手動の実行は、どの ref・入力でも Release を作らない**（Release の step は `github.event_name == 'push'` のときだけ）。成果物は run の artifact（`gh run download <run-id>`）。

## 3. タグを打つ

```bash
git tag v0.3.0 <main のコミット> && git push origin v0.3.0
```

pre-release は `v0.3.0-rc1` の形（`-` を含むタグは Release が pre-release になり、`releases/latest` に出ない）。接尾辞はアーカイブ名と Release の名前にだけ出て、wheel・Desktop の版は Cargo などのまま（0.3.0）。MSI は pre-release に数字しか許さないので、版には入れない。

## 4. 鍵（secret）

| 名前 | 使う所 | 無いとき |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Desktop の bundle を作る step だけ（ほかの step には渡さない）。updater の `.sig` と `latest.json` | 署名なしの bundle だけ。`latest.json` は作らない。鍵はあるが `tauri.conf.json` の `pubkey` が空なら警告 |

2026-11-06 は secret が 0 件、`pubkey` は空。macOS の署名・公証の鍵は使わない。鍵の作成と登録はユーザー。

## 5. 打ち直す

同じタグを打ち直すとき（失敗した、作り直したい）は、**先に Release とタグを消す**。

```bash
gh release delete v0.3.0 --cleanup-tag --yes     # Release と remote のタグ
git tag -d v0.3.0
```

そのあとコミットを直して打つ。消さずに同じ名前の資産を足すと、softprops/action-gh-release の既定（`overwrite_files: true`）の動きに頼ることになる【未確認。試していない】。

## 6. 公開

Release は作られると公開される（draft にしていない）。PyPI・crates.io・Marketplace への公開は別で、いまはしない（[CHANGELOG](../CHANGELOG.md)）。失敗したら Release を消して直す。

## 7. 固定している外部の版（2026-11-06）

| もの | 固定 | 出典 |
| --- | --- | --- |
| `softprops/action-gh-release` | v2.6.2 のコミット `3bb12739c298aeb8a4eeaf626c5b8d85266b0e65`（`v2` が指していた。2026-04-12 公開） | <https://github.com/softprops/action-gh-release/releases/tag/v2.6.2>（確認日 2026-11-06） |
| `tauri-cli` 2.12.4、`cargo-zigbuild` 0.24.1、`cargo-about` 0.9.2 | `release.yml` の `cargo install --version`（前の 2 つは 2026-11-06 の dry run で入った版） | 上げるときは dry run を回す |

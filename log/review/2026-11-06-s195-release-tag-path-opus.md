# S195 `release-tag-path` のレビュー（Opus/medium）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s195-release-tag-path` |
| 対象 | `.github/workflows/release.yml`（main cc846ba、読むだけのレビュー） |
| レビュアー | Opus（medium） |
| 依頼の文面 | release.yml の「タグの push のときだけ通る経路」を、dry run で一度も動いていない所を中心に読む |
| 作業記録 | `log/work/2026-11-06-s195-release-tag-path.md` |

## 指摘（原文のまま）

前提: gh secret list は空、pubkey も ""。v0.2.0 では .sig・latest.json・.app.tar.gz の名前直しは空振り。v0.2.0 そのものは成功の見込みが高い（1 と 3 が当たらなければ）。

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 1 | 可能性（高） | release.yml:266,270 | rc タグ（v0.2.0-rc1）をそのまま Tauri の version に渡す。WiX（msi）は pre-release に数字しか許さない → desktop(windows) が落ち、release ジョブが飛び Release 0 件。直し方: msi のときだけ数字の形にするか、rc では msi を作らないか、「rc は -<数字>」と書く。tauri-bundler の規則は未確認。 | 採用 | **再現した**。`tag=v0.2.0-rc1` の run 42157318748 で windows の desktop が `failed to bundle project: optional pre-release identifier in app version must be numeric-only and cannot be greater than 65535 for msi target` で落ち、release は skipped。直し: 版を書き換えるのをやめ（2 と一緒）、wheel・Desktop の版はファイルのまま（0.2.0）。接尾辞は名前にだけ出る。 |
| 2 | 確認済み | :143,165,266 | タグの版と Cargo・pyproject・tauri.conf.json の版が違っても止まらない（v0.2.1 を版を上げ忘れて打つと食い違ったまま出る）。直し方: third-party ジョブの頭で push のときだけ突き合わせて止める。版を揃えて打つ運用なら 165 行の perl と 266 行の上書きは消せる。 | 採用 | 読んで確かめた（直す前は 4 か所が 0.2.0 のままタグの版だけがアーカイブ名に入る。直す前の `v0.2.1` の run は流していない）。`check_release_tag.py` を最初のジョブに足し、perl と `--config` の version を消した。直したあとの `tag=v0.2.1` の run は最初のジョブ `third-party` で落ち、build・desktop は skipped。 |
| 3 | 可能性 | :165 | Windows での `perl -0pi`（バックアップなし）は一度も動いていない。Strawberry Perl が先に来ると "Can't do inplace edit without backup"。直し方: `-0pi.bak`、または 2 の運用で行ごと消す。 | 再現せず（行は 2 で消した） | `tag=v0.2.0` の run 42157306409 で windows-2025 の wheel の step は Git for Windows の bash と perl で通った（wheel は 0.2.0）。落ちる環境は今は無い。2 の運用で行ごと消えたので `.bak` の直しは不要。 |
| 4 | 確認済み | :225-229 | TAURI_SIGNING_PRIVATE_KEY と _PASSWORD がジョブ全体の env（サードパーティの action、build.rs・proc-macro に見える）。直し方: 「bundle を作る」step の env に移す。 | 採用 | 読んで確かめた（ジョブの env にあり、全 step に渡っていた）。bundle を作る step の env に移した。鍵は 0 件なので run では動作の差が出ない（署名の道は未確認のまま）。 |
| 5 | 確認済み | :140,162,263,334 | 判定が startsWith(github.ref,'refs/tags/v') だけ。`gh workflow run release.yml --ref v0.2.0` の手動実行でも本物の Release が作られる。直し方: github.event_name == 'push' && … に。 | 採用 | 読んで確かめた。Release の step を `github.event_name == 'push'` のときだけにした。`tag` 入力の 2 つの run で step が skipped（Release 0 件のまま）。 |
| 6 | 確認済み | :302-304 | SHA256SUMS の step は shell 未指定（bash -e、pipefail なし）。※ S176 で `set -o pipefail` を足した所と同じか要確認。 | 採用 | `git log -S pipefail` で release.yml に S176 が足した `pipefail` は無い（desktop の `defaults.run.shell: bash` とは別）。release ジョブの SHA256SUMS は shell 未指定で `sha256sum … | tee` の失敗が隠れるので `shell: bash` を付けた（確認は読んだだけ）。 |
| 7 | 可能性 | :335 | softprops/action-gh-release@v2 をタグで指す（contents: write）。直し方: コミット SHA で固定。 | 採用 | `v2` は v2.6.2（コミット `3bb12739c298aeb8a4eeaf626c5b8d85266b0e65`、2026-04-12）を指していた。そのコミットで固定。最新は v3.0.4 だが、メジャーを上げる判断はしない。 |
| 8 | 可能性 | :259,116 | cargo install tauri-cli --version "^2"・cargo-zigbuild の版が固定でない。直し方: --version 2.x.y。 | 採用 | dry run で入った版に固定: tauri-cli 2.12.4、cargo-zigbuild 0.24.1。 |
| 9 | 未確認 | — | 同じタグを打ち直したときの挙動（softprops の overwrite_files）。手順に「打ち直すときは Release を消してから」。 | 採用（文書だけ） | 試せない（Release を作らない）ので動きは【未確認】のまま、`doc/release.md` の 5 に「先に Release とタグを消す」手順を書いた。 |

提案（タグを打たずに確かめる案）: 採用。`workflow_dispatch` の入力 `tag`、env `TAG`、Release の step は push のときだけ。

## 結論

- 採用 8 件（1・2・4〜9。9 は文書だけ）・再現せず 1 件（3。行ごと消えた）。
- 1 は「読んで論証」だった指摘が実際に当たっていた。3 は当たらなかった（Windows の Git の perl は `-0pi` を受け付けた）。

# S160 `desktop-macos`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s160-desktop-macos` |
| worktree | `.git-worktree/s160-desktop-macos` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S160-desktop-macos/` |
| セッション記録 | （親が写す） |

## 依頼

`desktop.yml` に macOS（`macos-latest`、arm64）を足し、署名なしの .app / .dmg のビルド、smoke（MSL の取得と ChuaCircuit の検査・実行）、UI の測定（起動・Decay・MSL+ChuaCircuit・RSS）を Linux・Windows と同じ合格条件（`GM-SMOKE` 12 項目 ok）で回す。結果を `doc/bench/ui.md` の「Svelte Tauri macOS」に入れ、署名・公証なしと Gatekeeper の注意を事実として書く。対象外: 署名・公証・鍵、`release.yml`、`android.yml`、`tauri.conf.json` の security、`App.svelte`、experimental、UI のコード。

## 決めたこと

- 新しいジョブにせず、既存の `build` の matrix に 1 行足した（既存の step を再利用。共通化はしない）。macOS 専用の step は「UI の測定（macOS）」1 つ。
- Rust は Windows と同じランナーの rustup（`if: runner.os != 'Linux'`）。
- `--bundles app,dmg`（1 回目の `dmg` だけの run で .app が片付けられ、測定が落ちた）。
- RSS は `ps` の 0.2 s ごとのサンプリング（`time -l` は WebKit の XPC プロセスを含まないと考えた【試していない】）。
- Rust のコードは変えなかった（`fix_c_locale` は Linux だけ、sidecar の探し方はそのまま動いた）。
- `target/ui-bench.json` が無かったので、S156・S125 の artifact と `ui.md` の中央値から作り直し、`report` が元の `ui.md` と一致することを確かめてから足した（WSL2 の 3 列の生の値は中央値 ×3 で代用）。

## やったこと

- `.github/workflows/desktop.yml`: matrix に macOS、rustup の条件、`--bundles` の上書き、UI の測定（macOS）、artifact `desktop-macos-app`、ヘッダのコメント。
- `src/Svelte/tests/bench_ui.py`: `report` の文言の 2 か所（「未」の説明、Tauri の RSS の測り方）。
- `doc/bench/ui.md`（再生成）、`doc/features.md`、`doc/roadmap.md`（P7 の行と S160 の節）、`doc/architecture.md`、ADR-0023・ADR-0034 の付記。
- `doc/sessions/S160-desktop-macos/`（slides.html・解説書.md）、`doc/sessions/README.md` の索引。

## 検証

- run 41990732519（1 回目）: macOS が `--bundles dmg` で .app が無く落ちた（dmg は 20.88 MiB で作れた）。ほかの 2 OS は、macOS の修正の push で cancelled。
- run 41994245407（2 回目）: ubuntu・windows・macos のジョブとも success。macOS のジョブは約 27 分（08:26〜08:53 UTC）。回し直しは 1 回（上限 3 回）。
- macOS: `GM-SMOKE` 3 回とも 12 項目 ok。ready_ms 282 / 190 / 166、decay_ms 827 / 256 / 228、msl_ms 2,576 / 2,200 / 1,844、rss_kib 168,352 / 167,872 / 168,576、GM-BUNDLE 21,379 KiB。取得 smoke は MSL の取得 8.7 s、`smoke: ok`・`smoke: simulate ok=true`。署名は `adhoc`（linker-signed）、`TeamIdentifier=not set`。
- `python3 .github/scripts/test_ci_plan.py` OK。YAML は構文を確かめた（actionlint は mise に版が無く未実行）。`bench_ui.py` の自己検査と `report` OK。
- Rust は触っていない（`mise run check` は不要）。
- `check_docs.py`: スライド OK（S160 を含む）/ リンク 3,243 件中の切れ 0 件。`render_slides.py`（S160 のデッキだけ）: 7 枚、問題 0 件（1 回目で通った）
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件（メール・ホームのパス・トークンを grep）

## 残したこと

`release.yml` への macOS（署名・公証の鍵はユーザーの判断）、.dmg を Finder から開く経路・隔離属性・updater の確認【未確認】、x64 の macOS、RSS のピーク、`desktop.yml` を main から dispatch すること。

# S147 `ui-smoke-ops`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s147-ui-smoke-ops` |
| worktree | `.git-worktree/s147-ui-smoke-ops` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S147-ui-smoke-ops/` |
| セッション記録 | （親が写す） |

## 依頼

S140 の点検の順位 6: Desktop と Android で、ブラウザ版には試験のある UI の操作（ライブラリの木、整形、グラフ、進捗と止める、実行の設定、プロットの系列の選択、定義ジャンプ）を smoke で回す。`GM-SMOKE` の行に項目を足し、`desktop.yml` と `android.yml` の合格条件にする。cargo は回さない。

## 決めたこと

- Playwright は WebView に届かないので、`smoke()` の中から UI のコードの関数を呼ぶ（`openLibrary`・`format`・`graph`・`simulate`・`definition`・`goBack`）。関数にならない操作（実行の設定の欄、「止める」のボタン、プロットの信号のチェック）は要素に `input` / `click()` を送る。新しい依存も `App.svelte` の切り出しも無し。
- 順序は tree → graph → def → fmt → settings → plot → cancel。ChuaCircuit を開いたまま graph と定義ジャンプを回し、cancel は最後（Worker を作り直すとライブラリが消えるため）。各項目の最後に Decay に戻す（localStorage に残るのも Decay）。
- cancel は 1e6 秒の振動 `Long` を実行し、進捗が `integration` になってから「止める」を押す。Desktop は sidecar が止めれば診断 `GM-I11004`、作り直しなら `cancelled`。どちらも合格にした。
- MSL が無いブラウザは tree・graph を `skip`（def は Decay の中だけ）。
- Windows の測定にも項目を足した（`cfg=\w+` の後）。Android の待ちは 120 → 180 回（15 分）。

## やったこと

- `src/Svelte/src/App.svelte`（`smoke()` と `LONG` の定数だけ）、`src/Svelte/tests/test_ui.py`（smoke の行の項目）、`.github/workflows/desktop.yml`・`android.yml`（合格条件）。
- 文書: `doc/features.md`（Desktop・Android の行）、`doc/roadmap.md`（P6 の行と S147 の節）、ADR-0023、セッション文書、索引。

## 検証

- `pnpm check`: 0 errors / 0 warnings。
- `test_ui.py`（main の `.venv/bin/python -I`）: 通過。`GM-SMOKE check=ok simulate=ok … msl=ok tree=ok fmt=ok graph=ok cancel=ok settings=ok plot=ok def=ok ready_ms=332 decay_ms=191 msl_ms=2072`。
- desktop.yml run 41706643110（feature を push して `gh workflow run --ref`）: 成功。Linux は `msl=ok cfg=ok tree=ok … def=ok` を 3 回、Windows は `cfg=none` で同じ 7 項目 ok を 3 回。
- android.yml run 41706648182: 成功。debug・release の emulator とも 7 項目 ok（release の行: `decay_ms=2181 msl_ms=22461`）。
- 見つかった不具合: 0 件。
- `render_slides.py`: 手元では 2 回とも通らず（スクリーンショットで 30 s の時間切れ、2 回目は EPIPE）、CI の docs で確認
- `check_docs.py`: デッキ OK（S147 を含む）、リンク 2977 件中の切れ 0 件
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件（メール・ホームのパス・ユーザー名・トークンを grep）

## 残したこと

- `cancel` の通った道（`cancelled` か `GM-I11004`）を行に出す。プロットとグラフの描画の中身の確認。実機の Android。ダークモード・PWA は持ち越し。

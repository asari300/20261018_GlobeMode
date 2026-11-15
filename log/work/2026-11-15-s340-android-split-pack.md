# S340 `android-split-pack`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s340-android-split-pack`（main から切った） |
| worktree | `.git-worktree/s340-android-split-pack` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S340-android-split-pack/` |
| セッション記録 | なし（このセッション専用の記録は log/local/ に無い） |

## 依頼

S308 で Android の APK から `MSL.json` を外して分割配信の塊 838 本にしたら、APK が arm64 で +1.24 MB（+9.1%）増えた。APK の大きさを S308 の前（arm64 release 13,616,490 B）程度に戻しつつ、実行時のメモリの利点（読み込み後の線形メモリ 62 → 14 MiB）を残す方法を作る。候補は (A) Android の束だけ塊を 1 本のパックにして slice、(B) Android だけ塊の cap を上げる、(C) その他。

途中のユーザーの追加指示（2026-11-15）: **案 A と案 B を両方実装して試し、APK の大きさ・JS ヒープと wasm の線形メモリ・往復・smoke を同じ表で比べ、優れた方を採る。採らなかった方は外す。比べた表と理由を ADR-0023 の付記と解説書に書く。**

## 決めたこと

- **案 A を採り、案 B を外した**（ユーザーの指示の方式で、数字で決めた）。
  - APK（arm64 release、CI）: A 13,847,114 B（`MSL.json` 時代 13,616,490 B の +1.7%）、B（cap 256 KiB・先読み 0）14,432,522 B（+6.0%）、S308 14,859,242 B（+9.1%）。
  - 実行時: A は読む塊が S308 と同じで、往復（ChuaCircuit 3 往復・179 塊、20 本 17 往復・258 塊）と wasm の線形メモリ（13.7 / 80.6 / 289.3 MiB）が変わらない。B は取りすぎて 20 本の最大が 294.6〜341.2 MiB。
  - B は cap を上げても塊が 446 本で頭打ち（ディレクトリが変わるたびに分けるため）。brotli の見積もりで、B は最良でも `MSL.json` より 0.53 MB 大きい。
- 原因は Tauri の asset ごとの brotli（`tauri-codegen` 2.7.4。quality 9、debug は 2）。見積もり（brotli q9、窓 4 MiB）の差 1.17 MB が APK の実測の差 1.24 MB とほぼ合った。
- パックは Worker の JS ヒープに載せないよう Blob で持つ（`fetch().blob()` と `blob.slice().text()`。Node は `fs.openAsBlob`）。ArrayBuffer を握ると +15.2 MiB。
- ブラウザ版の分割配信は変えない（`GM_MSL_PACK` を付けるのは `android.yml` だけ）。
- 測るための `split.mjs --cap` は残した（B を再現できるように）。pack-msl.mjs の環境変数と `android.yml` の `variant` は外した。

## やったこと

- `globe-mode-wasm/web/pack.js`: `packFile(index, chunks)`（塊の JSON を UTF-8 で連結し、索引に `pack: { url, at }`）。
- `globe-mode-wasm/web/worker.js`: 索引に `pack` があれば、塊の取得を `packText`（Blob を 1 回取って slice）に替える。取れなければ誤りで、次の呼び出しで取り直す。閉包・先読み・壊れた塊の検査は変えていない。
- `src/Svelte/scripts/pack-msl.mjs`: `GM_MSL_PACK=1` のとき `MSL.json` と `c/` を作らず、`index.json` と `pack.bin` だけを作る。
- `.github/workflows/android.yml`: `ui` ジョブが `GM_MSL_PACK=1` で pnpm build し、`pack.bin` があり `MSL.json`・`c/` が無いことを確かめる。
- `globe-mode-wasm/node/split.mjs`: `--pack`、`--cap N`、各モデルの後に Worker の JS ヒープ（`worker.getHeapStatistics` の使用中 + external から wasm の線形メモリを引く）。`node/broken-chunks.mjs`: パックの束の試験（パックが無ければ誤り、置けば通る。MSL 不要で `mise run wasm:node` に入っている）。
- 文書: ADR-0023 の S340、roadmap（S340 の節・P6 の行）、CHANGELOG、README、セッションの索引・スライド・解説書。

## 検証

Node（release の wasm、`split.mjs`。20 本、全部読みと比べる。他の作業が同時に走っていたので時間は載せない）:

| 設定 | 塊 | Chua の往復・塊 | Chua の後の線形メモリ | 20 本の往復・塊・取る量 | 20 本の線形メモリ（最大） | JS ヒープ（最大） | 一致 |
| --- | ---: | --- | ---: | --- | ---: | ---: | --- |
| S308（cap 16 KiB・先読み 2） | 838 | 3・179 | 80.6 MiB | 17・258・9.4 MiB | 289.3 MiB | 41.8 MiB | 20 / 20 |
| 案 A（`--pack`） | 838 | 3・179 | 80.6 MiB | 17・258・9.4 MiB | 289.3 MiB | 43.2 MiB | 20 / 20 |
| 案 B（cap 256 KiB・先読み 0） | 461 | 8・95 | 81.1 MiB | 25・133・10.2 MiB | 294.6 MiB | 44.0 MiB | 20 / 20 |
| 案 B（cap 256 KiB・先読み 2） | 461 | 3・105 | 82.4 MiB | 9・160・11.0 MiB | 341.2 MiB | 42.8 MiB | 20 / 20 |
| 案 B（cap 64 KiB・先読み 2） | 514 | 3・126 | 82.1 MiB | 11・193・10.5 MiB | 296.7 MiB | 44.9 MiB | 20 / 20 |

読み込み直後の線形メモリは全部 13.7 MiB（`MSL.json` 全部は 62.1 MiB）。

APK（`android.yml`。基準は main の run 43663027170 = S301 の前の `MSL.json` 時代、S308 は run 43665628896）:

| APK | `MSL.json` | S308 | 案 A（run 43699981144） | 案 B（run 43699990294） |
| --- | ---: | ---: | ---: | ---: |
| arm64 release | 13,616,490 | 14,859,242 | **13,847,114** | 14,432,522 |
| x86_64 release | 14,202,127 | 15,444,927 | 14,432,687 | 15,018,143 |
| universal debug | 259,532,967 | 261,050,263 | 260,116,999 | 260,184,183 |

エミュレータの smoke（debug・release とも）は A・B とも `msl=ok` を含む全項目 ok。`msl_ms` は A 11.3 / 8.2 s、B 13.1 / 7.7 s、S308 10.6 / 11.2 s（共有ランナーの目安）。

最終の状態（案 B を外したコミット）の `android.yml`（run 43701068806）も success。APK は arm64 release 13,847,114 B・x86_64 release 14,432,687 B・universal debug 260,116,999 B（案 A の run と同じ）、エミュレータの smoke は debug・release とも `msl=ok` を含む全項目 ok、`msl_ms` は 12.9 / 12.0 s（共有ランナーのばらつき。run ごとに 7.7〜13.1 s）。

- ブラウザ版の回帰: `split.mjs`（既定、20 本）20 / 20 一致（S308 と同じ数字）。`pack-msl.mjs`（`GM_MSL_PACK` なし）の出力は `MSL.json` と `c/` 838 本で、`index.json` に `pack` が付かない。`web.yml`（run 43700035156、`test_ui.py` を含む）success。束は S308 の run 43668008748 と同じ（`index.json` 1549 KiB、`c/*.json` 838 本 15.2 MiB、`MSL.json` なし、ChuaCircuit の取得は塊 191 本・5.4 MiB、smoke ok）。
- `broken-chunks.mjs`: 壊れた塊の試験とパックの束の試験が通った。
- コードの検査: Rust は触っていない（`cargo` の検査は対象外）。JS のみ。
- `check_docs.py`: スライド全部 OK / リンク 5175 件中の切れ 0 件。`render_slides.py`（S340 のデッキ）: 問題 0 件。
- セッション記録の `--check`: セッション記録を作っていないので対象なし。
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークン 0 件

## 残したこと

- 実機の Android（WebView）での Blob の置き場・メモリ・速度。今回はエミュレータの smoke まで。
- 索引（約 0.2 MB）を縮めれば APK がさらに戻る。
- ブラウザ版も 1 本のパックにするかは、回線から塊だけ取れる利点とのトレードで、今回は変えていない。
- 別の worktree で動いている S335・S336・S337・S339（Rust の flat・dae）とは触るファイルが重ならない。索引 `doc/sessions/README.md`・`CHANGELOG.md`・`README.md`・`doc/roadmap.md` は他と同じ場所を触るので、マージの時に衝突しうる。

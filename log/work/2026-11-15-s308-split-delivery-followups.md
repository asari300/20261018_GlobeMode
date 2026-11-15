# S308 `split-delivery-followups`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s308-split-delivery-followups`（S301 のブランチから切った） |
| worktree | `.git-worktree/s308-split-delivery-followups` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S308-split-delivery-followups/` |
| セッション記録 | なし（このセッション専用の記録は log/local/ に無い） |

## 依頼

S301 の持ち越し 2 つ。(1) 塊を取る往復の回数を、索引に塊ごとの依存を持たせて減らす（取りすぎの量を測り、前後を作業記録と ADR に）。閉包は今の繰り返しで正しさを保つ。(2) Android の APK から `MSL.json` を外し、分割の束にして APK を軽くする（前後を測る）。

## 決めたこと

- 塊の依存は pack の時に、閉包と同じ走査（`SplitLibrary::deps_of`）の 1 段分を wasm の `file_deps` で求めた（新しい走査を作らない）。索引の `deps`（塊の番号 → 参照する他の塊）は約 50 KB。
- Worker は閉包に要るファイルの塊に加え、`deps` を `prefetch` 段たどった塊をまとめて `Promise.all` で取る。足りなければ今までの繰り返しが追加で取る（正しさは変えない）。
- 深さは測って決めた（下の表）。**既定は 2**。1 は往復が 5 で、3 以上は往復があまり減らない（3 で 3、4 で 2、6 で 1）のに取りすぎが増える。深さは索引の `prefetch` に書き、`split.mjs --prefetch N` で変えて測れる。
- Android は `android.yml` の `ui` ジョブで `dist/msl/MSL.json` を消す（`pack-msl.mjs` は変えない。Leptos・Dioxus・marimo が使うため）。

## やったこと

- `globe-mode-frontend`: `SplitLibrary::direct_deps`、`LibraryClosure::direct_deps`（試験 `direct_deps_are_one_step`）。`globe-mode-wasm`: `file_deps`。
- `web/pack.js`（`deps`・`prefetch`・`PREFETCH`）、`web/worker.js`（`fetchChunks` の先読み、`library_stats` に `rounds`）、`scripts/pack-msl.mjs`、`node/split.mjs`（`--prefetch`、往復の表示、ChuaCircuit の往復 4 以内・塊 190 以内の確認）。
- `android.yml`（MSL.json を消し、塊の有無と dist/msl の大きさを出す）。
- 文書: ADR-0023 の S308、roadmap（P6 の行・機能の穴・S308 の節）、CHANGELOG、セッションの索引・スライド・解説書。

## 検証

往復と取る量（Node、release の wasm、ChuaCircuit の初回。`split.mjs --prefetch N`）:

| 深さ | 往復 | 塊 | 取る量（索引込み） |
| ---: | ---: | ---: | ---: |
| 0（S301） | 11 | 163 | 4.64 MiB |
| 1 | 5 | 174 | 4.78 MiB |
| 2（既定） | 3 | 179 | 5.16 MiB |
| 3 | 3 | 182 | 5.19 MiB |
| 4 | 2 | 189 | 5.26 MiB |
| 6 | 1 | 192 | 5.29 MiB |

20 本を続けて回した合計: 深さ 0 は 39 往復・235 塊・8.8 MiB、深さ 2 は 17 往復・258 塊・9.4 MiB、深さ 3 は 17・261・9.5、深さ 4 は 16・265・9.5。**全部で 20 / 20 が全部読みと一致**（不一致 0）。

ブラウザ（`?smoke=1` の ChuaCircuit まで、Chromium、`/msl/` の要求の content-length の合計。索引を含む）: S301 は 174 要求・4.8 MiB。先読みを木のアイコンにも効かせた最初の版（web.yml run 43665632267 の `test_ui.py` も 217 本・6.9 MiB）は 218 要求・6.91 MiB に膨らんだので、基底だけの閉包（アイコン）では先読みしないようにした（`worker.js` の `ensure`）。直した版は **192 要求・5.35 MiB**（2 回とも同じ）。

ブラウザ（`bench_ui.py browser`、Chromium、手元）の `msl_ms` は 11.4〜12.0 s、RSS 356 MiB（S301 は 4.7 s、346 MiB）。このとき `test_ui.py` を同時に回しており（`decay_ms` が 54〜90 s）、機械の負荷で時間は比べられない。往復の時間への効きは測っていない（往復 × 往復時間で、100 ms なら 1.1 s → 0.3 s という計算上の見積もり）。

Android（`android.yml` run 43665628896、main の run 43663027170 と比べる）:

| APK | 前（B） | 後（B） | 差 |
| --- | ---: | ---: | ---: |
| arm64 release | 13,616,490 | 14,859,242 | +1,242,752（+9.1%） |
| x86_64 release | 14,202,127 | 15,444,927 | +1,242,800 |
| universal debug | 259,532,967 | 261,050,263 | +1,517,296 |

**APK は軽くならず、増えた。** 塊 838 本は別々に圧縮されるため（zlib -9 で `MSL.json` 2.65 MB に対し、塊の合計 3.30 MB と索引 0.22 MB）。`dist/msl` は 31.9 MiB（`MSL.json` を含む。pack-msl が塊と MSL.json の両方を作る）→ 16.7 MiB だが、APK の中では圧縮されて逆になる。エミュレータの smoke は debug・release とも全項目 ok（`msl=ok` を含む）。`msl_ms` は前 9.3 / 11.0 s、後 10.6 / 11.2 s で差なし。利点は読み込み時の線形メモリ（62 → 14 MiB）。APK の大きさを優先するなら、Android だけ `MSL.json` に戻せる（ユーザーの判断）。

- コードの検査: `cargo fmt --check` 0、`cargo clippy -p globe-mode-fmt -p globe-mode-frontend -p globe-mode -p globe-mode-wasm --all-targets -D warnings` 0、`cargo test -p globe-mode-frontend --lib` 49 通過（`direct_deps_are_one_step` を含む）。`split.mjs`（既定の深さ 2、20 本）は 20 / 20 一致で、往復の確認（ChuaCircuit が 4 往復以内・190 塊以内）を通った（実測 3 往復・179 塊）。`web.yml`（run 43665632267）success（束に MSL.json が無く、838 塊が全部ある。`test_ui.py` を含む）。`android.yml`（run 43665628896）success（ui・apk・emulator の debug と release）。
- `check_docs.py`: スライド全部 OK / リンク 4913 件中の切れ 0 件。`render_slides.py`（S308 のデッキ）: 問題 0 件。
- 回していない: 全体の `mise run check`（Rust は `file_deps` と `direct_deps` の追加だけ）、`wasm:compare`、`test_ui.py` の手元での通し（フォールドの印のクリックで時間切れになる既存のばらつき。CI の `web.yml` では通った）。先読みを基底だけの閉包で止めた直しのあとの `android.yml` は回し直していない（APK の大きさは変わらない）。web.yml は直したあとで回し直す（下）。
- 並行の作業の負荷（load 12〜20）でブラウザの時間は比べられず、`render_slides` は 2 回時間切れになり、`pkill -f '[c]hromium'` で他の作業のブラウザも止めた可能性がある。

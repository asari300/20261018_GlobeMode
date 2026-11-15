# issue #42（ponytail-audit 2026-11-08 の 49 件）の現状（S357）

- 確認日: 2026-11-15（main `cd15cccf`、S354 の取り込み直後）。
- 目的: 親 issue [#42](https://github.com/asari300/20261018_GlobeMode/issues/42) の 49 件を、今の main のコードで 1 件ずつ確かめ、済・未・見送りに分ける。
- 49 件の内訳は、issue 46 件（#43〜#88）と、既存 issue へのコメントにした 3 件（範囲 A-5 は #37、D-3 と E-2 は #41。B-1 は #33 にも知らせたが、数えるのは #43）。
- 根拠の書き方: 「済」は、(1) 直しのコミットが main の祖先、(2) 直しの本体が main の今のコードにある（`file:line`）、(3) 回帰試験が main にある、の 3 つを見た。試験は変えた crate に絞って今の main で回した（結果は 3 章）。JS・Svelte・CI・Zig・Julia・Elixir・Gleam は手元で回さなかったので、コードの読みだけで「済」にしたものに【コードのみ】と付ける。
- 判定は 3 つ。**済**（直った）、**未**（今も再現する）、**見送り**（直さないと決めた。理由つき）。

## 1. 結果

| 判定 | 件数 | 内訳 |
| --- | --- | --- |
| 済（S357 の前に） | 45 | S218 が 13（#72 は S241 で追い直した）、S223 が 18（#85 は S320 で読み方を変えた）、S226 が 12、S343 が 1（#63。S226 では見送りにした）、S230 が 1（E-2） |
| 未 → S357 で直した | 1 | D-3（CLI の `expand_inputs` が隠しディレクトリを読む。#41 のコメントの 1 つ）。親の集計では「既存 issue へのコメント」で、直しの記録が無かった |
| 見送り | 3 | #45（上流の口が要る）、#46（lowering の回数は上流の口が要る。予算とキャンセルの部分は S344 で直した）、A-5（#37 のコメント。見立てが【未確認】） |

合計 45 + 1 + 3 = 49。

## 2. 1 件ずつ

根拠の略号: 「S218」「S223」「S226」は親 issue のコメントにある修正のセッション。コミットは短縮ハッシュ。`file:line` は main `cd15cccf` の行。

### 2.1 必ず直す（13 件。すべて済）

| issue | 範囲 | 判定 | 根拠 |
| --- | --- | --- | --- |
| #83 | A-1 | 済 | S218 `ba2f2120`。`globe-mode-ir/src/sim.rs:13` に `deny_unknown_fields`。試験 `globe-mode-protocol/src/lib.rs` の `simulate_request_rejects_unknown_fields`（入れ子の `settings.stoptime`・`parameter` が断られる） |
| #43 | B-1 | 済 | S218 `06518c7c` → `c4f2344a`（issue の案は実測で取り消し、別スレッドで回して即 Cancelled に変えた。[ADR-0013](../adr/0013-exec-backend.md) 付記）。`globe-mode-exec/src/rumoca.rs:625-631`（`checked_detached`）。試験 `globe-mode/tests/observed.rs:148` `cancel_from_another_thread_stops_the_default_solvers`。放置したスレッドは終わるか予算まで走る（ADR に書いた設計） |
| #48 | C-1 | 済 | S218 `1058a2f9`。`globe-mode-fmi/src/export.rs:61-77` が `<DefaultExperiment>` を挿す。試験 `globe-mode-cli/tests/fmu_export.rs:47` |
| #49 | C-2 | 済 | S218 `28066004`。展開先を親が作る `globe-mode-fmi-import/src/worker.rs:62`（`work_dir`）。試験 `globe-mode-cli/tests/fmu_worker.rs:254`（`leaked_unpacked`） |
| #54 | D-1 | 済 | S218 `0bd2d09a`。`globe-mode-fmt/src/html.rs:22`（`MAX_DEPTH = 256`）・`:94`。試験 `html.rs:285` |
| #55 | D-2 | 済 | S218 `e66a8470`。`globe-mode-cli/src/fmu.rs:177`（実行の前に `.mo` と書けない出力先を断る）。試験 `globe-mode-cli/tests/fmu_import.rs:286` |
| #64 | E-1 | 済 | S218 `ea7b53f5`。`globe-mode-lsp/src/lib.rs:1177`（`republish_others`）。試験 `lib.rs:2040` |
| #67 | F-1 | 済【コードのみ】 | S218 `c5c93ab4`。`src/Rust/apps/desktop/src/main.rs:119`（`write_recent`）、`src/Svelte/src/lib/tauri-backend.ts:248-249`。試験は `src/Svelte/tests/test_ui.py`（Playwright。回していない） |
| #68 | F-2 | 済【コードのみ】 | S218 `83c998c5`。`src/Svelte/src/lib/plot/Plot.svelte:75`・`tauri-backend.ts:343`（`inDesktop` は Android を除く）。試験 `test_ui.py`（回していない） |
| #72 | G-1 | 済【コードのみ】 | S218 `732df498`、S241 `bc022768`。`.github/workflows/ci.yml:30-36`（`cancel-in-progress` は push 以外だけ、group は event ごと）。**残り**: 待ちが 2 本以上並ぶと GitHub が古い待ちを捨てる。注釈のとおり、続けて push するときは `full=true` で回す運用 |
| #73 | G-2 | 済【コードのみ】 | S218 `461bc90c`。6 アプリが `--disable connect-type-mismatch` を常に付ける（`src/Python/fastapi_fmt/app.py:98`、`src/Astro/fmt/src/lib/globe.ts:64`、`src/Python/marimo/globe_cli.py:51`、`src/Python/streamlit/globe_cli.py:47`、`src/Elixir/phoenix_fmt/lib/phoenix_fmt/globe.ex:16`、`src/Gleam/wisp_fmt/src/wisp_fmt/engine.gleam:69`）。CLI の lint の既定（型検査あり）は変えていない。issue の「直し方」どおり |
| #74 | G-3 | 済【コードのみ】 | S218 `c54ec8da`。`.github/workflows/android.yml:128-145`（`secrets.ANDROID_*` は keystore.properties を書く step の `env` だけ） |
| #75 | G-4 | 済 | S218 `81ae9db3`。`.claude/hooks/save_session_log.py:77`（`gh[pousr]_`）。試験 `.claude/hooks/test_save_session_log.py`（3 本） |

### 2.2 直した方がよい（18 件。すべて済）

| issue | 範囲 | 判定 | 根拠 |
| --- | --- | --- | --- |
| #84 | A-2 | 済 | S223 `4d2e1f3a`。BOM を空白 3 個にする `globe-mode-frontend/src/lib.rs:39-42`。試験 `globe-mode/tests/library_closure.rs` の `a_bom_library_file_is_read_from_memory_sources` ほか |
| #85 | A-3 | 済 | S223 `4d2e1f3a`。のち S320 `345d6f83`（#113）で、UTF-8 でないファイルは「名前を出して読まない」から「`from_utf8_lossy` で読む」に変えた（`globe-mode/src/lib.rs:485`）。無関係なモデルが GM-E1003 で止まらない。試験 `library_closure.rs` の `a_bom_and_a_non_utf8_library_file_do_not_break_other_models`、`globe-mode-db/tests/checks.rs` の `load_dir_names_the_unreadable_file_and_adds_nothing` |
| #86 | A-4 | 済 | S223 `86df2636`。`globe-mode-graph/src/lib.rs:205`（`walk`）。試験 `lib.rs:535` |
| #44 | B-2 | 済 | S223 `48dc3188`。`globe-mode-sim/src/settings.rs:51`（`grid_problem`。`floor(区間 / 刻み) + 1` が 1e7 以上、または刻みが時刻の ulp より細かい）。試験 `settings.rs:963-965` |
| #50 | C-3 | 済 | S223 `df52b5e5`。配列変数は「スカラーとして数えて止まる」でなく**断る**（`globe-mode-fmi-import/src/lib.rs:356`、`globe-mode-fmi/src/lib.rs:111-113,404`）。配列の対応そのものは未（[ADR-0026](../adr/0026-fmi-import-placement.md) の表）。試験 `globe-mode-fmi-import/tests/reference.rs:178`（Reference-FMUs が `data/` に要る） |
| #51 | C-4 | 済 | S223 `85ee82dd`。`globe-mode-fmu/src/lib.rs:252-258`。試験 `lib.rs:575` |
| #52 | C-5・D-9 | 済 | S223 `1ba2f60f`。`globe-mode-cli/src/fmu.rs:402`（`csv_header`）。試験 `fmu.rs:422`（`csv_header_quotes_names_with_commas`） |
| #56 | D-4 | 済 | S223 `1e968e11`。`globe-mode-cli/src/main.rs:1034-1038`。試験 `globe-mode-cli/tests/review_fixes.rs:433` |
| #57 | D-5 | 済 | S223 `4b8a0a06`。`globe-mode-fmt/src/lint.rs:102`。試験 `globe-mode-fmt/tests/format.rs:271` |
| #58 | D-6 | 済 | S223 `4b8a0a06`。`globe-mode-fmt/tests/format.rs:256`（`line_too_long_points_at_the_source_line_even_when_the_format_changes_it`） |
| #59 | D-7 | 済 | S223 `24bde2a2`。`globe-mode-fmt/src/tree.rs:43,157`（BOM を外す）。試験 `tree.rs:344` |
| #60 | D-8 | 済 | S223 `1e968e11`、`8120e1bf`。試験 `globe-mode-cli/tests/review_fixes.rs:408`（標準入力の構文の誤りは 2） |
| #69 | F-3 | 済【コードのみ】 | S223 `b43dd3be`。`globe-mode-wasm/web/client.js:13,113`（2 GiB を越えたら次の要求の前に Worker を作り直す）。試験は `web/worker-node.mjs:178`（`mise run wasm:node`。wasm のビルドが要るので回していない） |
| #70 | F-4 | 済【コードのみ】 | S223 `396c2fdf`。`src/Svelte/src/App.svelte:862`（保存の前に `confirm`）。試験 `test_ui.py`（回していない） |
| #76 | G-5 | 済【コードのみ】 | S223 `2838a303`。`release.yml`・`android.yml` の第三者 action（mise-action・rust-cache・setup-android・android-emulator-runner・softprops）はコミットの SHA 固定。残るタグ指定は GitHub 公式の `actions/*` だけ |
| #77 | G-6 | 済【コードのみ】 | S223 `1350884d` ほか。`.github/workflows/ci.yml:283-334`（marimo・Streamlit・Phoenix・Wisp）・ジョブ `astro`（`:413`）、`.github/scripts/ci_plan.py:148-152` |
| #78 | G-7 | 済【コードのみ】 | S223 `c6792885`。`src/Astro/fmt/src/lib/globe.ts:23-27`（非同期の `spawn`）。試験 `src/Astro/fmt/test/api.test.mjs`（回していない） |
| #79 | G-8 | 済 | S223 `e92a6435`。`globe-mode-c`・`-py` に残る `serde_json` は使っている（`src/lib.rs` に `serde_json::`）。nif の `serde`・`serde_json`、c・py の `serde` は無い |

### 2.3 あれば良い（15 件。済 13、見送り 2）

| issue | 範囲 | 判定 | 根拠 |
| --- | --- | --- | --- |
| #87 | A-6 | 済 | S226 `4b2da80a`。`globe-mode-db/src/lib.rs` の `record_executed`。試験 `globe-mode-db/tests/checks.rs:223` |
| #88 | A-7 | 済 | S226 `b72ef66f`。`globe-mode/src/lib.rs:406,659,1024` が `FullReport::failed_stage()`（`globe-mode-db/src/lib.rs:520`）を呼ぶ。「最初の Error の段」を求める式はここ 1 つ |
| #45 | B-3 | 見送り | issue は open のまま。S226・S344 に直さない判断。3 回目の初期化の計算（`check_prepared_initialization`）は「初期化」の段の目印で、外すと BDF の始動の失敗が積分の失敗に混ざり `auto` が dopri45 に落ちる。Rumoca に、初期化済みの状態を受け取る口が要る（上流の候補）。単体試験 `a_bdf_start_failure_is_an_initialization_failure_on_the_session_route` が分類を守る |
| #46 | B-4 | 見送り（予算とキャンセルの部分は S344 で済） | issue は open のまま。S344 の issue コメント（2026-11-15）のとおり、`auto` の回し直しに予算の残りを渡し、放置したスレッドは段の境目でトークンを見る（[ADR-0041](../adr/0041-auto-remaining-budget.md)）。lowering の回数（dopri45 は 2 回、`auto` は最大 3 回）は直さない: `rk45::simulate_artifact` が `pub(crate)` で、lowering 済みを渡す口が Rumoca に無い |
| #47 | B-5 | 済 | S226 `a7083a66`。`globe-mode-sim/src/store.rs:7`・`mat.rs:19`（イベントの 2 行は数 ulp ずれる） |
| #53 | C-6 | 済 | S226 `7fd21af4`。`globe-mode-fmi-import/src/abi2.rs:263`・`abi3.rs:86`（`initialized` のときだけ `Terminate`）。試験 `abi3.rs:642` |
| #61 | D-10 | 済 | S226 `d1027db1`。`Element::first` は `globe-mode-fmt/src/structure.rs` に無い |
| #62 | D-11 | 済 | S226 `6ad073a9`。`globe-mode-fmt/src/lint.rs:88`。試験 `globe-mode-fmt/tests/format.rs:245` |
| #63 | D-12 | 済 | S226 では見送りにしたが、**S343** `ef7bbc16` で直した（issue は 2026-11-15 に close）。`globe-mode-fmt/src/layout.rs:33`（`take_trail`）。試験 `format.rs:179` |
| #65 | E-3 | 済【コードのみ】 | S226 `6ed56619`。`src/Julia/globe_mode.jl:20-22`（`cconvert` / `unsafe_convert`）。`mise run c:julia` は回していない |
| #66 | E-4 | 済【コードのみ】 | S226 `a749ebbc`。`src/Zig/src/main.zig:36-37`（`std.json.fmt`）。`mise run c:zig` は回していない |
| #71 | F-5 | 済【コードのみ】 | S226 `f6222b17`・`a4c30987`。`src/Svelte/tests/test_ui.py:75` の `LONG` が `StopTime = 1e6`・`Interval = 1`、`App.svelte:100` も同じ |
| #80 | G-9 | 済【コードのみ】 | S226 `882c55c4`。`.github/workflows/ci.yml:84-87`（手動実行の比較元は `origin/main`）、`.claude/skills/finish-session/SKILL.md:102` |
| #81 | G-10 | 済 | S226 `d8429434`。`src/Rust/xtask/src/main.rs:629-647`（`fetch_zip` が前の版の展開を消す）。試験 `main.rs:1036` |
| #82 | G-11 | 済 | S226 `c47fa526`。`src/Python/fastapi_fmt/app.py:62-64,79`。試験 `src/Python/fastapi_fmt/tests/test_api.py:70,79` |

### 2.4 既存 issue へのコメントにした 3 件

| 範囲 | issue | 判定 | 根拠 |
| --- | --- | --- | --- |
| E-2 | #41 | 済 | S230 `ad74b844`。LSP の自前の走査 `collect_mo` を消し、frontend の `collect_mo_files`（隠しフォルダを飛ばす）に替えた（`globe-mode-lsp/src/lib.rs:1619-1621`）。試験 `globe-mode-lsp/tests/cross_file.rs:17,31`（`.hidden/Hid.mo` が出ない）。S230 の記録に #42 の E-2 とは書いていないが、直す対象と結果は一致する |
| D-3 | #41 | **未 → S357 で済** | 親の集計（2026-11-08）で触れられていなかった。`globe-mode-cli/src/main.rs` の `expand_inputs`（check・deps・graph・simulate・`check --frontend native` の入力）は自前の `walk` で隠しディレクトリを読み、`proj/.history/M_x.mo` があると `GM-E1001`（rumoca EM001、Duplicate class）で NG になった。今の main（S357 の前）で `globe check <dir> -m Decay` を再現（終了コード 1。`fmt --check` は `.history` を飛ばす）。**直した**: `walk` を消し、`globe_mode::collect_mo_files` を呼ぶ（-10 行）。試験 `globe-mode-cli/tests/review_fixes.rs` の `check_dir_input_skips_hidden_directories`。副作用: `X.mo` と `X/`（`package.mo` 無し）が並ぶ配置は `X.mo` だけを読む（S068。ライブラリの読み込みと同じ規則） |
| A-5 | #37 | 見送り | コメントの見立て（`SplitState.loaded` が増えるだけで、分割読み込みのメモリが増え続ける）は、コメント自身が【未確認】。今の main の `ensure_library`（`globe-mode-frontend/src/rumoca.rs:1655-1685`）は、足りないファイルを足して Session を作り直し、古い Session は捨てる。`loaded` は読んだファイルの番号の集合で、増えても上限はライブラリのファイル数。測っていない（コメントの「確かめ方」＝分割読み込み 100 本で `loaded.len()` と RSS を並べる、は未実施）。計測は `mise run msl:*`・wasm の計測が要るので S357 では回さない。`loaded` を閾値で空にする案は、分割読み込みの再読み込みの費用とのつり合いなので、測ってから決める |

## 3. 回した試験

`CARGO_TARGET_DIR=<main>/.git-worktree/target-shared CARGO_BUILD_JOBS=2`。debug。release・`msl:*`・wasm は回していない。`data/reference-fmus` は main のチェックアウトへの symlink（コミットしない）。

| 対象 | 結果 | 確かめた issue |
| --- | --- | --- |
| `cargo test -p globe-mode-ir -p globe-mode-sim -p globe-mode-fmt -p globe-mode-graph` | すべて pass（fmt の lib 34・`format.rs` 24、graph 2、ir 9、sim 116+1） | #44・#47・#54・#57・#58・#59・#61・#62・#63・#86 |
| `cargo test -p globe-mode-db -- executed_queries a_bom_and load_dir_names` | 2 本 pass | #85・#87 |
| `cargo test -p globe-mode --test observed --test library_closure` | observed 4・library_closure 6 pass | #43・#84・#85 |
| `cargo test -p globe-mode-fmi -p globe-mode-fmu` | pass（fmpy 21 本を含む） | #48・#51 |
| `cargo test -p globe-mode-fmi-import`、`GLOBE_REQUIRE_REFERENCE_FMUS=1 … --test reference` | 18 + 7 + 4 + 6 pass（`reference.rs` は Reference-FMUs ありで 4 本） | #50・#53 |
| `cargo test -p globe-mode-cli --bin globe --test fmu_export --test fmu_worker --test fmu_import --test review_fixes` | 8 + 12 + 5 + 7 + 15 pass | #48・#49・#52・#55・#56・#60 |
| `cargo test -p globe-mode-lsp` | lib 75（1 ignored）・call_hierarchy 1・cross_file 5・folding 2・tolerant_outline 2 pass | #64・E-2 |
| `cargo test -p xtask -- fetch_zip` | 1 本 pass | #81 |
| `python3 .claude/hooks/test_save_session_log.py` | 3 本 pass | #75 |
| `uv run pytest -q tests`（`src/Python/fastapi_fmt`、`GLOBE_BIN` を debug の `globe` に） | 11 本 pass（`GLOBE_BIN` が無いと 503 で 3 本落ちる。#82 の 503 の試験は `GLOBE_BIN` を壊して確かめる） | #73・#82 |
| `cargo test -p globe-mode-protocol`（単体、直す前） | **1 本落ちる**: `check_response_keys_follow_the_cli_json`（`preserve_order` が単体では有効にならず、鍵が辞書順） | 新規 |

共有 target の古いビルドで、`globe-mode-lsp` が `is_function` を引けず 1 回コンパイルに失敗した（別の worktree のビルド）。resolve・lsp の `lib.rs` を touch して再ビルドしたら通った。

## 4. 回さなかったもの（【コードのみ】の理由）

- Svelte の `test_ui.py`・`test_tauri.mjs`（Playwright。他の担当の Chromium と RAM を取り合う）: #67・#68・#70・#71。
- Astro の `api.test.mjs`・Elixir・Gleam・marimo・Streamlit の試験（node・BEAM の環境が要る）: #73・#77・#78。
- `mise run wasm:node`（wasm のビルドが要る）: #69。`worker-node.mjs:178` に試験がある。
- `mise run c:zig`・`c:julia`: #65・#66。
- GitHub Actions の定義（`ci.yml`・`android.yml`・`release.yml`）: #72・#74・#76・#77・#80。YAML を読んだだけで、実行の結果は見ていない。

## 5. S357 で直したもの

| 項目 | 直し | 試験 |
| --- | --- | --- |
| D-3 | `expand_inputs` の自前の走査を `globe_mode::collect_mo_files` に替える（-10 行）。`--help` の 3 か所に「隠しディレクトリを除いて」と足した | `review_fixes.rs` の `check_dir_input_skips_hidden_directories`。直す前の `globe check <dir> -m Decay`（`.history/Decay_old.mo` あり）は終了コード 1（GM-E1001 Duplicate class）、直したあとは 0 |
| 新規（#42 の外） | `globe-mode-protocol` の dev-dependency に `serde_json` の `preserve_order` を明示 | 既存の `check_response_keys_follow_the_cli_json`（直す前は `-p` 単体で落ち、直したあとは 3 本とも pass） |

### 直さなかった近い所

- `globe-mode-resolve/src/build.rs:73`（`mo_files`）と `globe-mode-flat/src/lib.rs:181`（`mo_files`）にも、隠しディレクトリを飛ばさない自前の走査がある（`Library::from_dirs`・`Project::read_dirs`。`--library` のディレクトリの走査）。ライブラリの中に隠しディレクトリの `.mo` が入る例は今は無く、flat は S355 が並行で触るので、S357 では触らなかった。**S358 で直した**（resolve の `mo_files` が `.` で始まる名前を飛ばし、flat はこれを呼ぶ。試験は各 1 本）。

## 6. 親 issue を閉じてよいか

- #42 の 49 件は、**直さないと決めたもの 3 件（#45・#46 の lowering の回数・A-5）を除いて済**。#45・#46 は issue が open のまま、A-5 は #37（close 済み）のコメントで、どれも直さない判断が記録されている。
- 閉じるかどうかはユーザーの判断。

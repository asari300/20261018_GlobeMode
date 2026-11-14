# S270 `flaky-timing-tests`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s270-flaky-timing-tests` |
| worktree | `.git-worktree/s270-flaky-timing-tests` |
| Phase | 保守 |
| セッション文書 | `doc/sessions/S270-flaky-timing-tests/` |
| セッション記録 | 省略（この WP のセッションの記録は `log/local/` に無い） |

## 依頼

共有の CI ランナーで時々落ちる時間を測る試験を、守りたい性質（計算量が 2 乗にならない等）を保ったまま安定させる。
`lint_time_grows_linearly`（S207）が feature の CI run 43628763459 の rust-linux で落ちた。S255 の深い入れ子の試験も同じ種類。

## 決めたこと

- 内部に仕事量のカウンタが無いので足さず、時間で測る。比の試験は入力を大きく・最小値を 5 回から・しきい値を線形と 2 乗の間に置く。
- 深い入れ子の試験は、直したあとも深さに比例する仕事が残る（深さ 10 → 200 で約 9 倍）ので、深さ 1 との比にした（直したあと約 11〜16、直す前は約 100〜200、しきい値 50）。
- ほかの時間の上限は、余裕が大きいものを残した。

## やったこと

洗い出した試験と判断:

| 試験 | 場所 | 測るもの | 判断 |
| --- | --- | --- | --- |
| `lint_time_grows_linearly` | `globe-mode-fmt/tests/format.rs` | 本文 2 倍の時間比 < 3（3 回の最小値） | **直した**。本文 4 倍（2000 → 8000）・5 回の最小値・しきい値 8。線形で約 4（実測 3.4〜4.7）、2 乗で約 16（わざと戻して 15） |
| `deeply_nested_wide_input_is_not_quadratic` | 同上 | 絶対 5 s 未満（debug で約 0.6 s） | **直した**。同じ入力を深さ 1 で整形した時間との比（5 回の最小値）< 50。直したあと約 11〜16、直す前は約 100〜200。絶対時間をやめ、ランナーの速さが打ち消される |
| `a_very_long_model_name_skips_the_spelling_distance` | `globe-mode-frontend/tests/library_hints.rs` | 絶対 5 s 未満（debug で約 12 ms） | 残す。余裕が約 400 倍で、守る性質（数十秒かかる編集距離を飛ばす）との差が大きい |
| `cancel_from_another_thread_stops_the_default_solvers` | `globe-mode/tests/observed.rs` | 中断から 5 s 未満（止まらなければ分単位） | 残す。中断が効かなければ 10 s 以上かかり、差が大きい |
| `progress_is_throttled_and_does_not_change_the_result` | 同上 | 進捗の回数 ≤ 段数 + 壁時計 / 0.1 s + 1 | 残す。上限は同じ試験の中で測った壁時計から出すので、ランナーの速さに依らない |
| `crashing_fmus_are_reported_and_leave_no_process`（中断） | `globe-mode-cli/tests/fmu_worker.rs` | 中断から 1.4 s 未満 | **直した**。5 s 未満に。FMU の全部を回すと約 1.5 s 以上かかるので、旗を無視すれば「中断した」の誤りでなく成功になり、そちらの確認が本体。時間は保険 |
| 同上（無限ループ・`--timeout 1`） | 同上 | 10 s 未満（上限 1.5 s・1 s） | 残す。余裕が 6 倍以上で、止まらなければ永久に終わらない |
| 要求の中断（`worker.rs`） | `apps/desktop/src/worker.rs` | 登録 5 s・中断 5 s 未満 | 残す。`sleep` 相当の子を kill するだけで、余裕が大きい |
| `request` のタイムアウト 2 つ（`sim.rs`） | `xtask/src/sim.rs` | 5 s 未満（`sleep 30` の子を 0.3 s で止める） | 残す。止まらなければ 30 s |
| `measure_*`・`msl_*`・`graph_library` の `eprintln` | `globe-mode-db`・`globe-mode-exec`・`globe-mode-wasm`・`globe-mode-frontend/src/structure.rs` | 表示だけ、または `#[ignore]` | 対象外。CI で回らない、または比較しない |

変更: `globe-mode-fmt/tests/format.rs`（`best_of_5` を足し、2 本を比に）、`globe-mode-cli/tests/fmu_worker.rs`（1.4 s → 5 s）。

過去の CI: 失敗 14 件のうち約 10 本のログを見て、時間の試験の失敗はほかに無かった。run 43628763459 のログは `gh run view --log` で取得できなかった。

## 検証

- `lint_time_grows_linearly` + `deeply_nested_wide_input_is_not_quadratic` を 20 回: 0 回失敗。lint の比 3.43〜4.68、深さの比 11.0〜16.0。
- わざと壊した（コミットしない）: lint の行・桁を数え直しに戻す → 比 15.08 で失敗、`flat` の打ち切りを外す → 比 101.8 で失敗。
- `cargo test -p globe-mode-cli --test fmu_worker`: 5 本 pass。
- コードの検査（fmt / clippy / test / deny）: finish で記す。
- `check_docs.py`: finish で記す。

## 残したこと

- run 43628763459 の落ちた比の値は不明（ログが取れない）。しきい値は手元の揺れ（3.4〜4.7）から、2 倍の余裕で置いた。

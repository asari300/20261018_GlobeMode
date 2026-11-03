# S109 `fmu-events`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s109-fmu-events` |
| worktree | `.git-worktree/s109-fmu-events` |
| Phase | P5（P5b の 3 回目） |
| セッション文書 | `doc/sessions/S109-fmu-events/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S099 の持ち越しの 1 つ目。事象のあるモデルを FMU で回す。`ModelInstance` に事象指示関数（root の条件の値）と事象の処理（Event Mode → `fmi2NewDiscreteStates` で離散の状態を更新し、`reinit` があれば `valuesOfContinuousStatesChanged`）を足し、`fmi2GetEventIndicators` が `n_event_indicators` 本を返し、`numberOfEventIndicators` を生成に反映する。時刻事象（`sample`）は `nextEventTime`。Rumoca の公開 API だけで出来ない部分は ADR-0032 に書いて持ち越す（Rumoca は改変しない）。BouncingBall と SampleCounter を FMPy の ME で回し、exec の `run` の最後の値と事象の回数を突き合わせる試験を各 1 本。出力の値参照・MSL・CS・FMI 3.0・Windows、`globe-mode-fmi`・CLI・`src/Python`・upstream・workflows は触らない。main へのマージ・push は親。

## 決めたこと

- **host の部品の公開の関数を host と同じ順に呼ぶ**（既定のまま進めた）: `SolveMeKernel` の `get_event_indicators`・`enter_event_mode`・`update_discrete_states` は `pub(crate)`。`SolveRuntime` の `eval_root_conditions`・`full_solver_y`・`apply_projected_event_update`・`refresh_event_dependency_slots_certified`・`next_runtime_event_stop` と、crate の公開の関数 `commit_pre_params_after_event_at`・`root_crossings_with_relation_memory`・`orient_typed_root_zeros`・`SolveStopSchedule` で足りた。
- **越えた根の上書き**: 直前の事象の後（初期化の後）の事象指示関数の値を覚えておき、Event Mode の値と比べる。
- **時刻事象の端は `f64::MAX`**: 1 回目は `f64::INFINITY` にして SampleCounter の nextEventTime が出なかった（`next_periodic_event_time` は有限の端でないと何も返さない）。
- **事象の回数は FMPy 側で数える**: n（Integer）に値参照が無い（持ち越し）。`fmpy_run.py` が `FMU2Model.enterEventMode`・`newDiscreteStates` を包んで数え、`events=`・`reinit=` を出す。BouncingBall は reinit の回数を、SampleCounter は Event Mode の回数を exec の n と比べる。
- **`initial()` の事象は回さない**（`shortcut:`）。今回の 2 本は使っていない。
- `tests/fmpy.rs` の `fmpy()` は出力の文字列を返すようにし、exec の最後の値を取る `exec_last` を足した（HarmonicOscillator の試験もこれを使う）。`events_are_refused` は消した。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/instance.rs`: 事象の 4 つの関数と `DiscreteUpdate`、GM-U0004 の拒否を外した。`lib.rs`: `DiscreteUpdate` を再輸出。
- `src/Rust/mod/globe-mode-fmu/src/lib.rs`: `numberOfEventIndicators`、`fmi2GetEventIndicators`・`fmi2EnterEventMode`・`fmi2NewDiscreteStates`。
- `src/Rust/mod/globe-mode-fmu/tests/fmpy.rs`・`fmpy_run.py`: BouncingBall・SampleCounter の試験、事象の回数。
- `doc/adr/0032-p5b-own-runtime.md`（追記）、`doc/architecture.md`、`doc/features.md`、`doc/roadmap.md`、`doc/sessions/S109-fmu-events/`、`doc/sessions/README.md`。
- 外部の依存の追加 0。

## 検証

- FMPy 0.3.32 の ME: BouncingBall `reinit=3 events=6`、h(2) = 0.26073984246749315、v(2) = -0.16588584207712853（exec の run: n = 3、0.26074169044319206、-0.1658694813220666。差 1.8e-6・1.6e-5）。SampleCounter `events=10`、x(1) = 1.0（exec: n = 10、0.9999999999999998）。Decay・HarmonicOscillator は S099 と同じ数字。
- `cargo clippy -p globe-mode-exec -p globe-mode-fmu --all-targets -D warnings`: 0 件。
- `cargo test -p globe-mode-exec -p globe-mode-fmu`: 成功（exec 21・13・31 件、fmu 6 件、失敗 0）。
- `mise run check`（`CARGO_BUILD_JOBS=2`）: 成功。fmt・clippy 0 件、`cargo test --workspace` 618 件成功・失敗 0、deny（advisories・bans・licenses・sources ok）・`uv sync --locked` 成功。
- `check_docs.py`: 全デッキ OK、相対リンク 2,174 件で切れ 0。`render_slides.py`（S109 のデッキ）: 6 枚 × 3 の表示で問題 0（1 回目は load 23 で Chromium が 30 s で時間切れ、load が 10 未満に下がってから回し直した）。
- セッション記録の `--check`: 親が写す（このセッションでは写していない）。
- ステージした差分の漏洩検査: 0 件。
- `wasm:node`・`wasm:compare`・`msl:sim` は回していない（exec は `ModelInstance` の追加だけで、`simulate` / `run` / session の道と wasm の道は変えていない）。

## 残したこと

- 離散・出力の変数の値参照（n・xs を FMU の外から読めない）。SampleCounter の試験は Event Mode の回数が exec の n と合うことまでで、FMU の中で n が 10 になったことは直接は見ていない（BouncingBall は reinit で `when` の発火を確かめている）。
- `initial()` の事象（`shortcut:`）、host の多様体の射影、状態事象と時刻事象が同時のとき。
- MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows。

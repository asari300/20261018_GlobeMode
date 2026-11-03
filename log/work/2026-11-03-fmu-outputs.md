# S112 `fmu-outputs`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s112-fmu-outputs` |
| worktree | `.git-worktree/s112-fmu-outputs` |
| Phase | P5（P5b の 4 回目） |
| セッション文書 | `doc/sessions/S112-fmu-outputs/` |
| セッション記録 | 親が写す |

## 依頼

S109 の持ち越しの 1 つ目。離散・出力（代数）変数を FMU の外から読めるようにする。`ModelInstance` に代数・離散の変数の値を返す公開 API を Rumoca の `SolveRuntime` の公開関数だけで足し（Rumoca は改変しない）、`globe-mode-fmu` の `modelDescription.xml` に代数・離散の `ScalarVariable` を足して `fmi2GetReal`・`fmi2GetInteger`・`fmi2GetBoolean` で読めるようにする（値参照 `2n+p..`）。FMPy の試験で SampleCounter の n が 10 になること、代数変数 1 つが exec の `run` の最後の値と一致することを各 1 本。S109 の Event Mode の回数だけの試験は n の値で強める。`initial()` の事象・MSL・P5a との共有・CS・FMI 3.0・Windows、facade・C・NIF・py・各言語・workflows は触らない。main へのマージ・push は親。

## 決めたこと

- **値は `visible_values`**（既定のまま進めた）: exec の `run` の結果の列は Rumoca の host の `record_outputs` → `SolveRuntime::record_visible_sample` → `visible_values_into`、FMI ME の host の `get_outputs` は `visible_values`。どちらも公開なので、`full_solver_y` で求めた y の上でそのまま呼ぶ。足りない部品は無かった。
- **役割と型は `variable_meta`**: `convert_variable_meta`（公開）で `SimVariableMeta` に直し、`rumoca.rs` の `role`（結果の `Trajectory` の振り分け）をそのまま使う。名前で引く（`visible_names` と `variable_meta` の順がそろう保証を読めなかったので、`trajectory` と同じく名前の表にした）。
- **API は 1 つの表**: 依頼の例（`algebraics()`・`algebraic_names()`・型ごとの離散）ではなく `variables()`（名前・`VariableRole`・`ValueType`）と `variable_values()` にした。FMU 側で型ごとに引き分けられれば足り、関数を増やさずに済む。`VariableRole` は exec から再輸出（fmu の Cargo.toml を変えない）。
- **代数変数の試験は NonlinearLoop**: 依頼の HarmonicOscillator・BouncingBall には代数変数が無い。モデルを書き換えると wasm:compare などの他の試験に響くので、既にある NonlinearLoop（y・z が代数ループ）を使った。
- **Enumeration・String は載せない**（`shortcut:`）、**値参照ごとに解き直す**（`shortcut:`）。
- BouncingBall の試験にも n を足した（依頼は SampleCounter だけだが、同じ 1 行で強められる）。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/instance.rs`: `Variable`・`ValueType`、`variables`・`variable_values`。`lib.rs`: 再輸出（`ValueType`・`Variable`・`VariableRole`）。
- `src/Rust/mod/globe-mode-fmu/src/lib.rs`: `modelDescription.xml` に代数・離散の変数と `Outputs`、`fmi2GetReal` の `2n+p..`、`fmi2GetInteger`・`fmi2GetBoolean`。
- `src/Rust/mod/globe-mode-fmu/tests/fmpy.rs`: `model_description_lists_discrete_variables`、`fmpy_matches_exec_on_algebraic_variables`（NonlinearLoop）、SampleCounter・BouncingBall の n。
- `doc/adr/0032-p5b-own-runtime.md`（追記）、`doc/architecture.md`、`doc/features.md`、`doc/roadmap.md`、`doc/sessions/S112-fmu-outputs/`、`doc/sessions/README.md`。
- 外部の依存の追加 0。

## 検証

- FMPy 0.3.32 の ME と exec の `run`（既定の bdf）: SampleCounter n = 10・10、xs = 0.9500000000000001・0.9499999999999997（差 4.4e-16）、x(1) = 1.0・0.9999999999999998。BouncingBall n = 3・3、h(2)・v(2) は S109 と同じ（差 1.8e-6・1.6e-5）。NonlinearLoop y(1) = 1.9999999999999991・2.0、z(1) = 4.000000000000001・4.0（差 8.9e-16 ずつ）、x(1) = 1.5000000094734762・1.5000000000931208（差 9.4e-9）。Decay・HarmonicOscillator は S099 と同じ数字。
- `cargo test -p globe-mode-fmu --test fmpy`: 8 件成功・失敗 0（BouncingBall の n を足した後にその 1 本を回し直して成功）。1 回目の回し直しは共有 target の古い exec（別 worktree のビルド）で `n_event_indicators` が見つからず落ちた。exec・fmu・core・fmi・fmt を touch して通った。
- `mise run check`（`CARGO_BUILD_JOBS=2`、共有 target）: 成功。fmt・clippy 0 件、`cargo test --workspace` 620 件成功・失敗 0、deny（advisories・bans・licenses・sources ok。警告 2 件は既存の deny.toml の未使用の許可）・`uv sync --locked` 成功。
- `check_docs.py`: 全デッキ OK、文書 457 件・相対リンク 2,246 件で切れ 0。
- `render_slides.py`（S112 のデッキ）: 6 枚 × 3 の表示で問題 0（1〜4 回目は他の WP のビルドで load が上がり Chromium が 30 s で時間切れ。5 回目で通った）。
- セッション記録の `--check`: 親が写す（このセッションでは写していない）。
- ステージした差分の漏洩検査: 0 件。
- `wasm:node`・`wasm:compare`・`msl:sim` は回していない（exec は `ModelInstance` の追加だけで、`simulate`・`run`・session の道と wasm の道は変えていない）。

## 残したこと

- 入力（`causality="input"`）と、代数・離散の変数の書き込み。Enumeration・String の変数（`shortcut:`）。値参照ごとに解き直す（`shortcut:`）。
- `Outputs` の `dependencies` を書いていない。今回の検証モデルには `output` の変数が無く、`Outputs` の節は試験で通っていない。
- `initial()` の事象、MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows。

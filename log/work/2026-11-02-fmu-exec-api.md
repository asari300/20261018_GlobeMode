# S099 `fmu-exec-api`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s099-fmu-exec-api` |
| worktree | `.git-worktree/s099-fmu-exec-api` |
| Phase | P5（P5b の 2 回目） |
| セッション文書 | `doc/sessions/S099-fmu-exec-api/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md)、[2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S095 の持ち越しの 1 つ目。`globe-mode-exec` に「状態・パラメータ・時刻 → 微分」の公開 API（例 `ModelInstance`）を足し、`globe-mode-fmu` の手書きの Decay をそれで置き換える。既存の `simulate` / `run` / session の道は変えない。載せ替えは最小で確かめられる道を選ぶ。FMPy の Decay の試験を通し、事象の無い 2 状態のモデルを 1 本足して exec の `simulate` と x(1) を突き合わせる。ADR-0032 に追記。`globe-mode-fmi`・CLI の `fmu export`・`fmi_export.rs`・workflows・Svelte は触らない。main へのマージ・push は親。

## 決めたこと

- **host でなく `SolveRuntime` を包む**（既定のまま進めた）: 依頼は「session の道の host の公開の関数を薄く包む」だったが、Rumoca の rev 276e82bd では host の `SolveMeKernel` の set time / set states / get derivatives は `pub(crate)`、積分器に渡る `MeDerivativeHandle` は host の呼び出しの間しか使えない。host と同じ部品の公開の `rumoca_solver::SolveRuntime` を持つことにした（lowering は `tunable_parameters` と同じ `lower_for_simulation_with_overrides`、初期化は `settle_initialization_system`、微分は `eval_state_derivatives`）。
- **事象のあるモデルは断る**（`GM-U0004`）: 根の条件か `sample` があれば `ModelInstance::new` が失敗する。FMU は event indicator 0 で出す。
- **道 2（実行時に compile）**: `fmi2Instantiate` が `resources/<完全修飾名>.mo`（1 ファイル、MSL なし）を読む。build.rs で埋め込む案より、1 つの cdylib で任意のモデルを回せ、試験も 2 本を同じ cdylib で回せる。
- **パラメータの変更は lowering からやり直す**: `ScalarSlot`（`rumoca-ir-solve`）は exec の直接の依存に無く、名前から P の枠を引けない。`fmi2ExitInitializationMode` で `param_overrides` を付けて作り直す（依存するパラメータにも届く）。
- **`modelDescription.xml` を生成に変えた**: `model_description(名前, 文面)`。guid は名前と文面の FNV-1a。`globe-mode-fmi` の型は使っていない（S098 が触るため）。
- 比べる相手は `simulate` でなく `run`（experiment annotation から設定を決める。中身は `simulate` と同じ道）にした。設定を手で組む手間を省くため。
- `.github/scripts/test_ci_plan.py` の期待を直した（fmu が exec と frontend に依存したので、ir・core・Modelica の変更で fmu も回る）。workflows は触っていない。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/instance.rs`（新規）、`rumoca.rs`（mod 宣言）、`lib.rs`（再輸出と文書）。
- `src/Rust/mod/globe-mode-fmu/`: `Cargo.toml`（exec・frontend への依存）、`src/lib.rs`（`guid`・`compile`・`model_description`、resources の読み込み、`Instance`）、`tests/fmpy.rs`（5 試験）、`tests/fmpy_decay.py` → `tests/fmpy_run.py`（validate と任意の変数）、`modelDescription.xml` を削除。
- `doc/adr/0032-p5b-own-runtime.md`（追記）、`doc/architecture.md`、`doc/roadmap.md`、`doc/sessions/S099-fmu-exec-api/`、`doc/sessions/README.md`、`.github/scripts/test_ci_plan.py`。
- 外部の依存の追加 0（`Cargo.lock` は fmu の依存の行だけ）。

## 検証

- FMPy 0.3.32: `validate_fmu` 問題 0。Decay x(1) = 0.3678794150955906（e⁻¹ と 2.6e-8）。HarmonicOscillator x(1) = -0.41614683955697307、v(1) = -1.818594856574809（exec の run の -0.4161468121809279・-1.8185947093271577 と 2.7e-8・1.5e-7、解析解と 3.0e-9・2.9e-9）。
- `cargo test -p globe-mode-fmu`: 5 件成功。`cargo clippy -p globe-mode-exec -p globe-mode-fmu --all-targets -D warnings`: 0 件。`test_ci_plan.py`: 25 件 OK。
- cdylib: debug 702 MB、release 44,128,856 B（44.1 MB）、strip 後 32,796,024 B、gzip 12,407,167 B。
- `mise run check`（main を merge した後）: 成功。fmt・clippy 0 件、`cargo test --workspace` 615 件成功・失敗 0、deny・`uv sync --locked` 成功。1 回目は `globe-mode-fmt` の 2 件が共有 target の古いビルド（別の worktree の `CARGO_MANIFEST_DIR`）で NotFound になり、fmt・core・fmi を touch して回し直した
- `check_docs.py`: 全デッキ OK、相対リンク 2,113 件で切れ 0。`render_slides.py`（S099 のデッキ）: 7 枚 × 3 の表示で問題 0
- ステージした差分の漏洩検査: 0 件
- 前任のエージェントが 機械の load 100 超えで落ち、未コミットの作業を引き継いで試験・merge・検査を仕上げた（`main` の S098・S100〜S103 を merge。roadmap と索引の衝突は両方を残した）

## 残したこと

- 事象のあるモデル、出力（代数変数）の値参照、MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows。
- `resourceLocation` の `%xx` を戻していない（`shortcut:`）。`derivatives` の中の panic は `extern "C"` で abort する（`ModelInstance::new` の lowering だけ `catch_unwind`）。
- `wasm:node`・`wasm:compare` は回していない（exec は追加だけで、wasm の道は変えていない。`globe-mode-wasm` のビルドは `mise run check` で確かめる）。

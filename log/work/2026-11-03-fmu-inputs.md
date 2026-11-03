# S114 `fmu-inputs`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s114-fmu-inputs` |
| worktree | `.git-worktree/s114-fmu-inputs` |
| Phase | P5（P5b の 5 回目） |
| セッション文書 | `doc/sessions/S114-fmu-inputs/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S112 の持ち越しの 1 つ目。最上位に `input Real u` を持つモデルを `ModelInstance` で扱い（`variables()` に Input、`set_input`）、FMU では `causality="input"`（start 必須）として `fmi2SetReal` で書けるようにする。同じモデルの `output` で S112 で未検証の `ModelStructure/Outputs` を FMPy の validate で通す。FMPy の `input=` で入力を与えて x(1) を exec の `run` か解析解と突き合わせる試験を 1 本。Rumoca は改変しない。facade・C・NIF・py・各言語・upstream・workflows は触らない。main へのマージ・push は親。

## 決めたこと

- **入力の置き場は `input_parameter_index`**: Rumoca の host の `set_float64` は入力をパラメータの枠の `SolveLayout::input_parameter_index(name)` に書くだけ（ほかは計算の控えの消去と `delay` の点の記録）。どちらも公開なので `set_input` は枠に書くだけにした。
- **lowering の前の値は `start`**: `lower_for_simulation_with_overrides` は駆動されない入力を断る。S094 の記録どおり `host_driven_input_seeds` は crate の外から呼べないが、公開の `SimOptions::initial_inputs` と `rumoca_phase_solve::fmi::numeric_attribute_values` で同じ処理を exec の `input_starts` に書けた。Rumoca の改変も ADR への「足りないもの」の記録も要らなかった。
- **API は名前で引く `set_input(name, value)`**: FMU 側は値参照から `variables()` の名前を引いて呼ぶ。index 版は足していない。
- **出力を `InitialUnknowns` にも並べる**: 最初の FMPy の実行で validate が `ModelStructure/InitialUnknowns does not contain the expected set of variables. Expected {der(x), y} but was {der(x)}.` で落ちた。S112 の `Outputs` は試験に通っていなかったための漏れ。
- **作り直しの後に入力を置き直す**: パラメータの変更で `ModelInstance` を作り直すと入力が start に戻るので、`Instance` が与えた入力を控える。初期化そのものは start で解く（`shortcut:`）。FMPy は入力を Initialization Mode の中で与えるので、`InputDecay` のように初期値が入力に依らないモデルでは差が出ない。
- **突き合わせは解析解**: exec の `run` は入力を与える道が無く（`ResolvedSettings` に入力が無い）、駆動されない入力を断る。依頼の「または解析解」を採った。入力は階段（t = 0.5 で 0 → 2）にし、途中の `fmi2SetReal` が効くことも確かめた。
- **試験のモデルは新しい `Simulation/InputDecay`**: 既存の GlobeModeValidation に入力を持つモデルが無い（`input Real` は関数の引数だけ）。`package.order` の最後に足した。`globe simulate` では断られる（コメントに書いた）。

## やったこと

- `src/Modelica/GlobeModeValidation/Simulation/InputDecay.mo`（新規）、`package.order`。
- `src/Rust/mod/globe-mode-exec/src/rumoca/instance.rs`: `set_input`、`variables()` に Input、`input_starts`（`SimOptions::initial_inputs`）。
- `src/Rust/mod/globe-mode-fmu/src/lib.rs`: `causality="input"`（start）、出力を `InitialUnknowns` にも、`fmi2SetReal` の入力、作り直しの後に入力を置き直す、`fmi2Reset` で入力を消す。
- `src/Rust/mod/globe-mode-fmu/tests/fmpy.rs`: `inputs_and_outputs_through_the_c_abi`、`fmpy_drives_input_step`、`fmpy()` に入力の引数。`tests/fmpy_run.py`: 7 番目の引数（入力の JSON）を FMPy の `input=` に渡す。
- `doc/adr/0032-p5b-own-runtime.md`（追記）、`doc/architecture.md`、`doc/features.md`、`doc/roadmap.md`、`doc/sessions/S114-fmu-inputs/`、`doc/sessions/README.md`。
- 外部の依存の追加 0（numpy は FMPy の依存で、`uv run --with fmpy` の中だけで使う）。

## 検証

- FMPy 0.3.32 の ME（CVode）で InputDecay、u は t = 0.5 に 0 → 2: x(1) = 1.1548180654142353（解析解 1.1548181217461755、差 5.6e-8）、y(1) = 2.3096361308284705（解析解 2.309636243492351、差 1.1e-7）、u(1) = 2.0、Event Mode に入った回数 1（階段の時刻事象）。`validate_fmu` は問題なし。
- C ABI: `fmi2SetReal(u = 3)` で der(x) = 2.0、u = -1・x = 0.5 で y = 1.0・der(x) = -1.5（ちょうど一致）、出力 y への書き込みは `fmi2Error`。
- `cargo test -p globe-mode-fmu --test fmpy`: 10 件成功・失敗 0（213.7 s）。1 回目は BouncingBall・SampleCounter の FMPy が `GM-U0004`（S109 より前の文言）で落ちた。試験の中の `cargo build --lib` が共有 target の古い exec（別 worktree のビルド）を拾ったため。exec・fmu・core・fmi・fmt を touch して通った。
- `mise run check`（`CARGO_BUILD_JOBS=2`、共有 target、load 3 のときに開始）: 成功。fmt・clippy 0 件（1 回目は clippy の `useless_format` 1 件で落ち、直した）、`cargo test --workspace` 622 件成功・失敗 0、deny（advisories・bans・licenses・sources ok。警告は既存の deny.toml の未使用の許可）・`uv sync --locked` 成功。
- `check_docs.py`: 全デッキ OK、文書 476 件・相対リンク 2,355 件で切れ 0。
- `render_slides.py`: **通っていない**。Chromium の `Page.screenshot` が 30 s で時間切れになる（S112 のデッキも、`set_content('<p>hi</p>')` だけの頁も、`--disable-gpu` でも同じ。load 1.6 でも再現）。環境の不調で、デッキの問題ではない。
- セッション記録の `--check`: 親が写す（このセッションでは写していない）。
- `mise run wasm:node`: 成功（Decay の bdf・dopri45・auto、panic の文言。data/msl が無いので MSL は飛ばした）。
- `mise run wasm:compare`: 387 / 400 件が一致、既知の差 13 件、一致しない 0 件。新しい InputDecay は 20 ソルバとも wasm・ネイティブがそろって `GM-U0004`（Rumoca の `EX002: input u has neither a checked default nor a runtime value`。`globe simulate` は入力を与える道が無い）。
- ステージした差分の漏洩検査: 0 件。

## 残したこと

- 初期化が入力に依るモデル（今は start で初期化し、作り直しの後に置き直す。`shortcut:`）。代数・離散の変数の書き込み、Integer・Boolean の入力。
- exec の `run`（`globe simulate`）に入力を与える道。今は InputDecay を `GM-U0004` で断る。
- `Outputs` の `dependencies`、Enumeration・String、`initial()` の事象、MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows。
- `render_slides.py` は環境の不調で通っていない（上の検証）。

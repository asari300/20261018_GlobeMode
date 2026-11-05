# S137 `fmu-runtime-fmi3`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s137-fmu-runtime-fmi3` |
| worktree | `.git-worktree/s137-fmu-runtime-fmi3` |
| Phase | P5（P5b の 6 回目） |
| セッション文書 | `doc/sessions/S137-fmu-runtime-fmi3/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

`globe-mode-fmu`（S095〜S114、FMI 2.0 ME）と `write_model_description_3`（S124、EventIndicator は `Err`）の持ち越し。同じ cdylib で FMI 3.0 の ME も出す（2.0 と同じ `ModelInstance`、2.0 の関数を形だけ変えて呼ぶ）。`EventIndicator` の valueReference を `ModelStructure` に足して S124 の `Err` を外す（2.0 の読み書きは変えない）。`tests/fmpy.rs` の道に FMI 版の引数を足し、Decay・HarmonicOscillator・BouncingBall・SampleCounter・InputDecay を 3.0 でも回して 2.0 と同じ値、FMPy の validate が 3.0 で通る。CS・Windows・macOS・MSL・Enumeration/String・`dependencies` は持ち越し。exec・lsp・Svelte・desktop・`doc/msl-*.md`・workflows は触らない。Rumoca は改変しない。新しい外部の依存は足さない。main へのマージ・push・worktree の削除は親。

## 決めたこと

- **3.0 は 2.0 の関数の包み**: `fmi3InstantiateModelExchange` と `fmi2Instantiate` は中身（`instantiate`）を共有、`fmi3EnterInitializationMode` は `fmi2SetupExperiment`、`fmi3GetFloat64`・`SetFloat64`・`GetInt32` は `fmi2GetReal`・`SetReal`・`GetInteger`（`nValues != nvr` は `fmi3Error`）、`fmi3GetBoolean` は `each::<bool>`、`fmi3UpdateDiscreteStates` は `fmi2NewDiscreteStates` の `EventInfo` を `bool` に移す。`fmi3Boolean` は C の `bool`。
- **75 関数をすべて出す**: `fmi3FunctionTypes.h`（FMPy 同梱の版）の名前と `lib.rs` の名前を `comm` で突き合わせて欠け 0。ME に要らない 49 関数は引数を宣言しない `fmi3Error` の stub をマクロ `unsupported!` で並べた（呼び手が引数を片付ける C の呼び出し規約）。CS・SE の Instantiate は NULL。今の FMPy は関数を使うときに引くが、S098 で関数を引けずに落ちた経緯があるので全部出した。
- **`FMI3_OVERRIDE_FUNCTION_PREFIX` は関係しない**: C のヘッダで関数名に接頭辞を付けるかの切り替え（S098 の記録）。Rust の `#[unsafe(no_mangle)]` は `fmi3*` をそのまま出す。
- **`resourcePath`**: 3.0 は URI でなくそのままのパス。`read_resource` は `file:` があれば外し、無ければそのまま使う（2.0 の道は変わらない）。
- **EventIndicator の変数**: 3.0 の `EventIndicator` は変数の valueReference を指すので、値参照 `2n+p+q..` に Float64・local・continuous・calculated の `_eventIndicator1..` を足した（3.0 の XML だけ）。`variable()` がその値参照で `event_indicators()` を返すので `fmi3GetFloat64`（と `fmi2GetReal`）で読める。
- **independent の `time`**: 最初の FMPy の実行で 3.0 の 6 本とも validate が `Exactly one independent variable must be defined.` で落ちた。EventIndicator の次の値参照に `time`（`causality="independent"`、initial・start なし）を足し、`ModelInstance::time()` を返す。
- **`globe-mode-fmi`**: `ModelStructure` に `event_indicators: Vec<u32>`。3.0 の読む側は `<EventIndicator>` の valueReference を入れ、`write_model_description_3` はそれを書く。数が `number_of_event_indicators` と違うときだけ `Err`。2.0 の読み書きは触っていない（2.0 は空のまま）。
- **試験**: FMPy の 6 本（依頼の 5 本と NonlinearLoop）を `for v3 in [false, true]` で 2.0・3.0 の両方に回す。`fmpy_run.py` の 7 番目の引数を FMI の主版にし（入力の JSON は 8 番目へ）、3.0 なら `binaries/x86_64-linux/` に置き、事象の数は `fmi3._FMU3.updateDiscreteStates` を数える。3.0 の XML の形を見る `model_description_3_lists_event_indicators` を 1 本足した。

## やったこと

- `src/Rust/mod/globe-mode-fmi/src/lib.rs`: `ModelStructure::event_indicators`、3.0 の読み書き。`tests/parse.rs`: EventIndicator の読み→書き→読みと数違いの `Err`。
- `src/Rust/mod/globe-mode-fmu/src/lib.rs`: `model_description_3`（`describe` を 2.0 と共有）、`instantiate` の切り出し、`read_resource` のパス、`variable()` の EventIndicator と `time`、`fmi3*` の 75 関数。`Cargo.toml` の description。
- `src/Rust/mod/globe-mode-fmu/tests/fmpy.rs`・`tests/fmpy_run.py`: FMI 版の引数、2.0・3.0 の両方で回す。
- `doc/adr/0032-p5b-own-runtime.md`・`doc/adr/0031-fmi-export-p5a-first.md`（追記）、`doc/architecture.md`、`doc/features.md`、`doc/roadmap.md`、`doc/sessions/S137-fmu-runtime-fmi3/`、`doc/sessions/README.md`。
- 外部の依存の追加 0。

## 検証

- FMPy 0.3.32 の ME（CVode）、3.0 の終了時刻の値は 2.0 と全桁で同じ:
  - Decay x(1) = 0.3678794150955906（e^-1 と 2.6e-8）
  - HarmonicOscillator x(1) = -0.41614683955697307、v(1) = -1.818594856574809（exec と 2.7e-8・1.5e-7）
  - BouncingBall h(2) = 0.26073984246749315、v(2) = -0.16588584207712853、n = 3、Event Mode 6 回・reinit 3 回（exec と 1.8e-6・1.6e-5、n は一致）
  - SampleCounter x(1) = 1.0、n = 10、xs = 0.9500000000000001、Event Mode 10 回（exec と 4e-16 以内）
  - NonlinearLoop y = 1.9999999999999991、z = 4.000000000000001（exec 2.0・4.0）
  - InputDecay x(1) = 1.1548180654142353、y(1) = 2.3096361308284705（解析解と 5.6e-8・1.1e-7）
- `validate_fmu`: 3.0 の 6 本とも問題なし（`time` を足す前は 6 本とも上の 1 件）。2.0 も問題なし。
- `cargo test -p globe-mode-fmu --test fmpy`: 11 件成功・失敗 0（738.6 s、`--test-threads 3`）。`cargo test -p globe-mode-fmi`: 6 件成功。1 回目の `cargo test -p globe-mode-fmi -p globe-mode-fmu` は自分で付けた `timeout 590` で FMPy の途中で切れた（失敗ではない）。
- main（S150 まで）を取り込んだ後、`cargo test -p globe-mode-fmi` 6 件、`cargo test -p globe-mode-fmu -- --test-threads 3` 11 件（29.4 s、ビルド済み）が成功。`mise run check` 通過（テスト 653 件成功・失敗 0、cargo deny ok、`uv sync --locked`）
- `check_docs.py`: 切れ 0 件、スライド OK
- `render_slides.py`（S137 のデッキだけ）: 7 枚、問題 0 件
- ステージした差分の漏洩検査: 0 件
- セッション記録の `--check`: 親が写す。

## 残したこと

- CS（2.0・3.0）、Windows・macOS の FMU、MSL を使うモデル、Enumeration・String、`Outputs` の `dependencies`、Integer・Boolean の書き込み、初期化が入力に依るモデル。
- 3.0 の `InitialUnknown` に EventIndicator の変数を並べるかは確かめていない（FMPy の validate は求めなかった）。

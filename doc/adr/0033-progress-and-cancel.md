# ADR-0033 進捗は段と積分の時刻を 100 ms に 1 回まで、中断は呼び手の持つトークンを実行系が区切りごとに見る

- 状態: Accepted（2026-11-03、S136。全 5 段を実装した。Proposed は 2026-11-03 の S121）
- 関係: [ADR-0014](0014-session-route-wasm.md)（session の道）、[ADR-0016](0016-preflight-on-build-lowering.md)（`run_with_progress` の段）、[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)（lowering の上限と外から止めたものの回し直し）、[ADR-0023](0023-desktop-and-web-process-layout.md)（ワーカーを作り直して止める）。どれも覆さない。S110 の [言語 API の機能表](../research/lang-api-matrix-2026-11-03.md) の埋める順の 6

## 背景

### 今あるもの（2026-11-03、main `d41e130` のソースで確かめた）

| 口 | 進捗 | 中断 | 置き場所 |
| --- | --- | --- | --- |
| `globe-mode-exec` | `run_with_progress(model, settings, &mut dyn FnMut(ExecStage))`。段（Preflight・Lowering・Build・Initialization・Integration）に入るたびに呼ぶ（S014）。積分の時刻は出さない | `SimSettings.max_wall_seconds` を Rumoca の `SimOptions` に渡すだけ。時間切れは `GM-E11005` | `src/Rust/mod/globe-mode-exec/src/lib.rs`、`rumoca.rs`、`rumoca/session.rs` |
| Rumoca の型付きの道（ネイティブの bdf・dopri45・auto） | 中から知らせる口は無い | 期限（`Instant`）を中で見る。期限切れは panic（`SolverTimeoutPanic`）で戻り、`panic_diagnostic` が `GM-E11005` にする | `rumoca.rs` の `run_typed` |
| Rumoca の session の道（wasm32 と自前の積分器） | 同上（`run_to_stop` を 1 回呼ぶ） | `run_to_stop` は `advance_to(stop)` を呼ぶだけで、`advance_to` は受理した 1 歩ごとに `TimeoutBudget::check` を見る。区切りの時刻は呼び手が選べ、止めたあとも `finish()` でそれまでの trace を取れる | `rumoca-solver/src/fmi_me/session.rs` の 462〜513 行（rev `276e82bd`） |
| facade `globe-mode` | 無い（`simulate_with` は `exec::run` を呼ぶ。壁時計 `now` は差し込める） | 予算だけ | `src/Rust/mod/globe-mode/src/lib.rs` の 358〜421 行 |
| C ABI・Zig・Julia | 無い | 予算だけ（`settings_json`） | `globe-mode-c` |
| NIF・Elixir・Gleam | 無い。`simulate` は `DirtyCpu` | 予算だけ | `globe-mode-nif/src/lib.rs` |
| Python | 無い。`simulate`・`simulate_columns`（S085）は GIL を持ったまま回る（`detach` を呼んでいない）ので、Ctrl-C は戻るまで届かない | 予算だけ | `globe-mode-py/src/lib.rs` |
| wasm | 無い | 予算と、`GlobeWorker.terminate()`・`#die` による Worker の作り直し（S037・S104） | `globe-mode-wasm/web/client.js` |
| LSP（`globe lsp`、Desktop の sidecar） | `$/progress` は使っていない | `$/cancelRequest` は扱っていない。stdin を 1 本のループで読み、要求を順に処理する | `globe-mode-lsp/src/lib.rs` |
| Desktop | MSL の取得だけ: イベント `msl-progress`（`{received, total}`）と command `cancel_msl`（`Arc<AtomicBool>`。S084）。実行には無い | sidecar の作り直し（ADR-0023） | `src/Rust/apps/desktop/src/main.rs`・`msl.rs` |
| CLI | 無い | `--timeout`（`max_wall_seconds`） | `globe-mode-cli/src/main.rs` |
| xtask `msl-sim` | `run_with_progress` の段を親に送る | 親がワーカーのプロセスを止める（ADR-0017 の 120 / 150 / 240 s） | `src/Rust/xtask/src/sim.rs` |

- ADR-0014 で、session の道と型付きの道は `Simulation.*` 12 本の bdf でビット単位で同じと確かめた。ただし session の道はネイティブでも Cranelift の JIT を使わない（インタプリタ）。速さの差は測っていない【未確認】。
- ADR-0017: lowering は Rumoca の協調型の予算の外にあり、止めるには外からプロセスを止めるしかない。製品の実行系には lowering の上限が無い。
- 呼び手が途中で止める口は、Desktop の MSL の取得（`cancel_msl`）にしか無い。その形（呼び手が `Arc<AtomicBool>` を持ち、仕事の側が区切りごとに見る）は既にこのリポジトリにある。

### 外の仕様（確認日 2026-11-03）

- LSP 3.17（<https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>）: 取り消された要求にも応答を返す。取り消しのときに途中までの結果を返してよい。誤りで返すなら `RequestCancelled`（-32800）。進捗は要求の params の `workDoneToken` と `$/progress`（`begin` の `title` は必須、`percentage` は 0〜100 で増えていく）。
- SharedArrayBuffer（<https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer>）: 使うには secure context で cross-origin isolated であること（ヘッダを 2 つ付ける）。付けないと `postMessage` が SharedArrayBuffer を投げられない。

## 決定

### (1) 進捗の単位: 段と、積分の時刻 t。時刻は 100 ms に 1 回まで

- 進捗は `Progress { stage: ExecStage, time: Option<f64>, start_time: f64, stop_time: f64 }` の 1 つの形。
- 段が変わったら必ず 1 回出す（今の `run_with_progress` と同じ）。積分の段では、t を出してから壁時計で 100 ms 以上たったときだけ次の t を出す。積分が終わったら t = 終了時刻（または止めた時刻）を 1 回出す。
- 壁時計は facade の `simulate_with` に渡す `now`（ミリ秒。wasm は `performance.now()`）を使う。実行系は `Instant` を呼ばない（wasm32 で落ちるため、ADR-0014）。
- t を出せるのは session の道（wasm と自前の積分器）だけ。ネイティブの型付きの道（bdf・dopri45・auto）は段だけ（Rumoca に口が無い）。

却下した案:

- 段だけ: wasm と自前の積分器で積分が長いモデル（Lightning の類）の動きが見えない。区切りは session の道にもうあるので費用は小さい。
- 出力の点ごとに毎回: 10 万点で 10 万回呼ぶ。NIF のメッセージや LSP の通知があふれる。
- 割合（%）や刻みの数で出す: 割合は t と開始・終了時刻から呼び手が出せる（LSP の `percentage` は glue で計算する）。刻みの数は終わりが分からない。

### (2) 中断の仕組み: 呼び手が持つトークン（`Arc<AtomicBool>`）を、実行系が区切りごとに見る

- facade に `CancelToken`（`Arc<AtomicBool>` の newtype、`Clone + Send + Sync`、`cancel()`・`is_cancelled()`）を置く。`cancel_msl` と同じ形。
- 実行系は次の区切りでトークンを見る。
  - 段に入る前（ネイティブの型付きの道でも効く）。
  - `auto` の回し直しの前（ADR-0028 の esdirk34・dopri45 に進む前）。
  - session の道の積分の中: `run_to_stop` を、出力の点を区切りにした `advance_to` の繰り返しに替え、区切りごとに見る。Rumoca は変えない（公開の `advance_to` だけ）。
- 見られないところ: lowering と組み立て（Rumoca の中）、ネイティブの型付きの道の積分の中。ここは今まで通り `max_wall_seconds` と、外からプロセスやワーカーを止める道（ADR-0017・ADR-0023）が受け持つ。外から止める道は消さない。
- 各口で同じ形になるか:
  - wasm: Worker の中は 1 本のスレッドで、計算の間は `onmessage` が回らないので、外からトークンを立てられない。止めるのは今の `terminate()` と作り直しのまま（途中の列は返らない）。トークンは使わない。
  - NIF: `DirtyCpu` のスケジューラで回し、トークンを resource（`ResourceArc`）に包んで返す。別の Erlang プロセスから NIF `cancel(ref)` で立てる。同じ形。
  - C: コールバックの戻り値が 0 以外なら、glue がトークンを立てる。別のスレッドから止めたい呼び手は、`void *user` の先のフラグをコールバックで読んで返す。同じ形（呼ばれる頻度は進捗と同じ 100 ms に 1 回）。
  - Python: GIL を放して回し（pyo3 0.30 の `Python::detach`。旧名 `allow_threads`【名前は実装のときに確かめる】）、進捗のたびに GIL を取り直してコールバックと `py.check_signals()` を呼ぶ。コールバックが真を返せばトークンを立てる。Ctrl-C ならトークンを立て、戻ったあと `KeyboardInterrupt` を上げ直す。同じ形。

却下した案:

- `max_wall_seconds` を拡げる: Rumoca の `TimeoutBudget` は作ったときに期限を決め、後から縮める口が無い。「今止めて」を秒で表せない。
- スレッドを落とす: Rust にスレッドを外から止める手段は無く、Rumoca の `PreparedSimulation`（`Rc`・`RefCell`）を壊れたまま残す。プロセスを落とす道は ADR-0023 で既に外側にある。
- wasm で SharedArrayBuffer のフラグを Worker と共有する: cross-origin isolated（ヘッダ 2 つ）が要り、静的な配信と marimo・Tauri の WebView のそれぞれで付けられるかを確かめていない【未確認】。`terminate()` で足りている。
- 進捗のコールバックの戻り値だけで止める（トークンを持たない）: NIF と LSP は止める合図が計算と別のスレッド・プロセスから来るので、共有のフラグが要る。C だけは戻り値で受けて glue がトークンに写す。

### (3) 各口の形: facade に 1 つの型、各口はそれを写す

| 口 | 進捗 | 中断 |
| --- | --- | --- |
| `globe-mode-exec` | `run_observed(model, settings, &mut Observer)`（`Observer` は進捗の `FnMut(&Progress)` と `Option<CancelToken>` と `now`）。`run_with_progress` は段だけを写す薄い包みとして残す（xtask が使う） | `CancelToken` |
| facade `globe-mode` | `SimulateHandle<'a> { on_progress: Option<&'a mut dyn FnMut(&Progress)>, cancel: Option<CancelToken> }` を受ける `Workspace::simulate_observed(request, now, handle) -> Simulated`。`Progress`・`ExecStage`・`CancelToken` を再公開する | 同左 |
| C ABI | `typedef int (*gm_progress_fn)(void *user, int stage, double t, double start, double stop)`（t が無いときは NaN）と `gm_simulate_observed(ws, request_json, cb, user)`。列の版も同じ引数を足す | 戻り値 0 以外 |
| NIF | `simulate_async(ws, request, pid)` が `{ref, token}` を返し、進捗は `{:globe_progress, ref, stage, t, start, stop}`、結果は `{:globe_result, ref, json}` を pid に送る | `cancel(token)` |
| Python | `simulate(..., progress=None)`・`simulate_columns(..., progress=None)`。`progress(stage: str, t: float | None, start, stop) -> bool | None` | 真を返す、または Ctrl-C（`KeyboardInterrupt`） |
| wasm | `simulate_columns(request, onProgress)`（JS の関数を同期で呼ぶ）。`GlobeWorker` は Worker の中で `postMessage({type: "progress", …})` に写し、呼び手の `onProgress` に渡す | `terminate()` と作り直し（今のまま） |
| LSP | 要求の params に `workDoneToken` があれば `$/progress`（`begin`・`report` の `percentage` は `(t − start) / (stop − start) × 100`・`end`）。`initialize` の応答で知らせる | `$/cancelRequest`。読むループを計算のスレッドと分け、計算は別の 1 本のスレッドで 1 度に 1 要求（ADR-0023 の直列は変えない） |
| Desktop | LSP のものを使う。UI の「止める」は `$/cancelRequest` を送り、応答が来なければ今の sidecar の作り直し | 同左 |
| CLI | 変えない（`--timeout` のまま。Ctrl-C はプロセスを終える） | — |

### (4) 中断後の結果: 積分に入っていれば止めた時刻までの列を返す。診断は情報の `GM-I11004`

- 新しいコード `GM-I11004`（`RUN_CANCELLED`、情報、「呼び手が t = … で止めた」）。`GM-I11001` はモデルの `terminate()` なので分ける。
- 積分の段で止めたら `Termination::Cancelled { time }` を足し、それまでの列を返す（`ok: true`、`terminate()` と同じ扱い）。session の道は `advance_to` の区切りで止めるので、trace は壊れていない。
- 積分の前（段の境目）で止めたら、列は無く、`ok: false` と `GM-I11004` だけを返す。
- LSP は誤りでなく普通の応答で返す（LSP 3.17 は取り消しのときに途中の結果を返すのを認めている）。

却下した案:

- 診断だけを返す: session の道は止めた時刻までの trace をそのまま持っているので、捨てる理由が無い。UI は途中まで描ける。
- 誤り（`GM-E…`）にする: 呼び手が自分で止めたのは失敗ではない。`max_wall_seconds` の時間切れ（`GM-E11005`）とは分ける。

## 結果

### 実装の順と各段の試験の案

| 順 | 何を | 試験の案 |
| --- | --- | --- |
| 1 | `globe-mode-core` に `GM-I11004`、`globe-mode-ir` に `Termination::Cancelled`、exec に `Observer`・`run_observed`、session の道を `advance_to` の繰り返しに、facade に `SimulateHandle`・`simulate_observed` | (a) トークンを止めずに回すと `run` と同じ（`Simulation.*` 12 本の session の道の全変数の最後の値がビット単位で同じ。`wasm:compare` の 400 件で不一致 0）。(b) Decay（esdirk34）で t ≥ 0.5 の進捗のときにトークンを立てると `Cancelled`、最後の時刻が 0.5 以上 0.5 + 出力の間隔以下。(c) 先にトークンを立てておくと、ネイティブの bdf でも lowering に入らず `GM-I11004` だけ。(d) 進捗の t が増えていき、呼ばれた回数が「段の数 + 壁時計 / 0.1 s + 1」以下 |
| 2 | wasm（`globe-mode-wasm` と `GlobeWorker`、Svelte の進捗の表示） | `wasm:node` で Decay の進捗が 1 回以上、最後が Integration の t = 1。`terminate()` の作り直しの試験は今のまま |
| 3 | Python | `test_api.py` で、コールバックが t > 0.5 で真を返すと `Cancelled` と途中の列。別のスレッドの `_thread.interrupt_main()` で `KeyboardInterrupt` が上がる |
| 4 | C・Zig・Julia、NIF・Elixir・Gleam | C: Zig・Julia の試験でコールバックが 1 を返すと `Cancelled`。NIF: 別のプロセスから `cancel` を送ると `{:globe_result, …}` の termination が `cancelled` |
| 5 | LSP と Desktop | `globe lsp` に `workDoneToken` 付きの `globe/simulate` を送ると `$/progress` の `begin`・`end` が来る。続けて `$/cancelRequest` を送ると、応答が `GM-I11004` を含む |

- 機能表の「進捗」「中断」の行は、各段が終わるたびに埋める。
- 速さ: session の道を `advance_to` の繰り返しにすると、区切りごとの呼び出しが増える。1 の試験で Decay と MSL の `Electrical.Analog.Examples` の壁時計を前と比べ、5% を超えて遅くなるなら区切りを出力の点でなく 100 ms の進捗の間隔に揃える。

### 1 段目の結果（S123、2026-11-03）

- 状態は Proposed のまま（2 段目以降が残る）。core・IR・exec・facade は入った: `GM-I11004`（`RUN_CANCELLED`）、`Termination::Cancelled { time }`、exec の `Progress`・`CancelToken`・`Observer`・`run_observed`（`run_with_progress` は段だけを写す包み）、facade の `SimulateHandle`・`Workspace::simulate_observed`（`simulate_with` はその包み）。
- **session の道の区切りは `advance_to` でなく、積分器（`MeIntegratorBackend`）を包んで 1 歩ごとに見る形にした。** `advance_to(t)` の t は刻みの上限（Rumoca の `reachable_bound` が `yield_time` で刻みを切る）なので、出力の点で区切ると刻みの列が変わり、数値が変わる。出力の点で区切った版で `globe-mode-exec` の `tests/integrators.rs` の 6 本が落ちた（euler の x(1) が 0.348678 から 0.367511 に、trbdf2 の HarmonicOscillator が積分に失敗）。100 ms の進捗の間隔で区切る案も、区切りが壁時計で決まるので同じ入力で数値が変わる。包む形は刻みの要求を変えないので、`the_session_route_matches_the_typed_route`（`Simulation.*` の bdf、全変数の最後の値がビット単位で同じ）を含む既存の試験がそのまま通る。止めるときは包みが次の 1 歩の前に誤りを返し、`finish()` でそれまでの列を取る（止めた時刻は列の最後の出力の点）。
- 速さ（release、`Simulation.*` 20 本 × 実装のあるソルバすべて（sundials なしのビルド）を session の道で 3 回ずつ回し、モデルごとの最小の `run_seconds` の和、load average 18〜25 の WSL）: `run_to_stop` 22.368 s、包む形 22.630 s（**+1.2%**。モデルごとに −3.1%〜+4.6%）、出力の点ごとの `advance_to` 22.107 s（和は DiodeSwitching の 22 s が占める。小さなモデルでは Decay +186%、BouncingBall +182%、TinyStateRestart +366%）。包む形を採った（5% 未満）。
- 試験（`globe-mode/tests/observed.rs` と exec の単体試験）: (a) 止めないときの最後の値が `simulate_with` とビット単位で同じ、(b) Decay（esdirk34、出力の間隔 0.01）で t ≥ 0.5 の進捗のときにトークンを立てると `Cancelled { time }`（0.5 ≤ time < 1、列の最後の時刻と同じ。包む形では「次の 1 歩の前」で止まるので、上限は「0.5 + 出力の間隔」でなく「0.5 + 1 歩」に替えた）と `GM-I11004`、(c) 先に立てたトークンなら bdf・dopri45・esdirk34 のどれも段を 1 つも知らせずに止まり、`ok: false` と `GM-I11004` だけ（Error 無し）、(d) t が単調に増え、最後は 1、回数は「段の数 + 壁時計 / 0.1 s + 1」以下。`auto` は立っていれば回し直さない（単体試験）。
- `wasm:compare`: 400 件中 387 件が一致、既知の差 13、一致しない 0（減っていない）。`wasm:node` ok。
- 状態の無いモデル（積分器が無い）は積分の中で止められない。積分器の 1 歩ごとに `now` を呼ぶので、wasm（`performance.now()`）での費用は 2 段目で測る。

### 2 段目の結果（S133、wasm と Svelte、2026-11-03）

- exec・facade は変えていない。`globe-mode-wasm` の `simulate_columns(request_json, on_progress?)` に省略できる 2 つ目の引数を足した（(3) の表の形。別の `simulate_observed` は作らなかった。UI が使うのは列の道だけで、省略すると前と同じ `run_columns` の道なので `run`・`wasm:compare` は変わらない）。JS の関数は js-sys を足さず、`#[wasm_bindgen] extern` で `Function.prototype.call` を宣言して呼ぶ（`on_progress(stage, time, start_time, stop_time)`。`stage` は Python と同じ小文字、`time` は無ければ `undefined`。関数の例外は無視する）。
- `worker.js` は `{id, type: "progress", stage, time, start_time, stop_time}` を親に送り、`GlobeWorker.simulateColumns(req, {onProgress})` が受ける。`GlobeWorker.cancel()` は実行中の呼び出しがあるときだけ、Worker を `terminate()` して作り直し（ライブラリとファイルは既存の再生で送り直す）、止めた呼び出しを `cancelled: true` の Error で reject する。途中の列は返らない（決定のとおり）。
- Svelte: `Backend` に `simulateColumns(req, onProgress?)` と省略できる `cancel?()`。`WorkerBackend` は両方を通し、`TauriBackend` は `cancel` を持たない（「止める」ボタンを出さない。LSP・Desktop は 5 段目）。`App.svelte` は実行中に「実行中: integration、t = … / stop（n%）」の 1 行（`role="status"`）と「止める」ボタンを出し、止めたら `data-run="cancelled"` と「止めた」を出して、ボタンを戻す。
- **`performance.now()` の費用**（release の wasm、Node 24、load average 10〜20 の WSL）: 実行系は積分器の 1 歩ごとに `now` を呼ぶ（進捗を受けないときも。`simulate_with` も同じ `Hooks::tick` を通る）。呼んだ回数は Decay（esdirk34）で 1 回の実行あたり 55 回、`ChuaCircuit`（bdf）で 12,074 回。JS から `performance.now()` を 100 万回呼んだ 1 回あたりは 60〜160 ns なので、ChuaCircuit で約 0.7〜1.9 ms、実行の中央値 363 ms の 0.5% 未満。定数を返す `now` に差し替えた実行との中央値の差（Decay +2.1 ms、ChuaCircuit −40.7 ms）は揺れの範囲で、費用は見分けられなかった。区切りを変える必要は無い。
- 試験: `globe-mode-wasm/tests/decay.rs` の `observed_columns_report_progress_and_match`（ネイティブで、Decay の esdirk34 の進捗が lowering で始まり integration の t = 1 で終わり、列が進捗なしと同じ）、`node/smoke.mjs`（`wasm:node`。wasm で同じことを確かめ、6 回の進捗。上の費用もここで出す）、`web/worker-node.mjs`（`wasm:web`。Worker 越しの進捗と、`der(x) = sin(100 * time)` を 1000 s の実行を最初の t の進捗で `cancel()` すると `cancelled` で reject し、作り直した Worker で Decay が回る）、`src/Svelte/tests/test_ui.py`（同じモデルで進捗の行が出て、「止める」で `data-run="cancelled"` になり「実行」が押せるに戻る）。

### 3 段目の結果（S129、Python、2026-11-03）

- 2 段目（wasm）より先に入れた（wasm は S127 が `globe-mode-wasm` を触っているので独立に進めた）。exec・facade は変えていない。
- `globe-mode-py` の `Workspace.simulate`・`simulate_columns` に keyword 引数 `on_progress` を足した（`None` なら前と同じ道）。名前は (3) の表の `progress` でなく `on_progress` にし、引数は位置引数 4 つでなく `{"stage", "time", "start_time", "stop_time"}` の dict にした（`stage` は `"lowering"`・`"integration"` などの小文字）。真の値を返すとトークンを立てる。
- GIL: pyo3 0.30.1 の `Python::detach`（旧名 `allow_threads`）で離して `Workspace::simulate_observed` を回し、進捗のたびに `Python::attach` で取り直して `py.check_signals()` とコールバックを呼ぶ（名前は crate のソース `pyo3-0.30.1/src/marker.rs` と <https://docs.rs/pyo3/0.30.1/pyo3/marker/struct.Python.html> で確かめた。確認日 2026-11-03）。`check_signals` かコールバックが例外を出したらトークンを立て、実行が戻ってから投げ直す（Ctrl-C は `KeyboardInterrupt`）。facade の `Workspace` は `Send` なので `unsafe` は要らなかった。
- 試験（`src/Python/tests/test_api.py`）: (1) Decay（esdirk34）で段が `lowering` から `integration` まで届き、最後の t が 1、列・CSV が `on_progress` なしと同じ、(2) `der(x) = sin(100 * time)` を 1000 s（手元で約 0.9 s）で最初の t の進捗で `True` を返すと `ok`・`termination` が `cancelled`（0 < time < 1000、列の最後の時刻と同じ）・`GM-I11004`、(3) 別のスレッドの `threading.Timer(0.2, _thread.interrupt_main)` で `KeyboardInterrupt`。7 本通る（1 本は前からの skip）。
- Ctrl-C も `True` も、知らせが来るのは段の変わり目と積分の t（100 ms に 1 回まで）だけなので、lowering・組み立て・ネイティブの型付きの道（bdf・dopri45・auto）の積分の中では次の段まで届かない（1 段目の持ち越しと同じ）。`on_progress` を渡さないときは前のまま GIL を持って回る。

### 4 段目の結果（S131、C・Zig・Julia と NIF・Elixir・Gleam、2026-11-03）

- exec・facade は変えていない。(3) の表の形から次のように変えた（呼び手の言語で読みやすい形にし、Python の dict と同じ 4 つの値をそろえた）。
- C ABI（`globe-mode-c`）: 位置引数 5 つの `gm_progress_fn` でなく、構造体 `GmProgress { const char *stage; double time; int has_time; double start_time; double stop_time; }` を渡す `typedef int (*GmProgressCallback)(const GmProgress *progress, void *user_data)`。`stage` は Python と同じ小文字で、コールバックの間だけ有効。`time` が無いときは NaN と `has_time = 0`。依頼は `request_json` でなく既存の `gm_simulate` と同じ `(model, settings_json)`。口は `gm_simulate_observed(ws, model, settings_json, callback, user_data)` と、列の道の `gm_simulate_columns_observed(ws, model, settings_json, data, len, callback, user_data)`（`gm_simulate_columns` はその `callback = NULL` の包み）。`callback` が NULL なら `SimulateHandle::default()` で前と同じ道。
- Zig は translate-c のヘッダの型をそのまま使い、`callconv(.c)` の関数を渡す。Julia は `simulate_observed(ws, model, on_progress, settings)` で、`@cfunction` の固定の trampoline に Julia の関数を `Any` の `user_data` として渡す（閉包の `@cfunction` を使わない）。`on_progress` は `(stage, time, start_time, stop_time)` の NamedTuple を受け、`true` で止める。
- NIF（`globe-mode-nif`）: `simulate_async` が `{ref, token}` を返す形でなく、`cancel_token()` でトークン（`ResourceArc`）を先に作り、`simulate_observed(ws, model, settings_json, pid, token)`（DirtyCpu）が結果の JSON を返すまで待つ形にした。DirtyCpu の NIF は戻るまで値を返せないので、トークンを先に渡すのが最小。進捗は `pid` へ `{:globe_progress, %{stage, time, start_time, stop_time}}`（Rustler の `NifMap`、`time` は `nil` か float）を呼び出し元の env の `send` で送る。中断は別のプロセスから `cancel(token)`（`:ok`）。Elixir は `GlobeMode.simulate_observed(ws, model, settings, token, pid \\ self())`、Gleam は `@external` と不透明な型 `CancelToken`・`Pid`（`erlang:self/0`。`gleam_erlang` は足していない）。
- 試験（どれも S129 と同じ `der(x) = sin(100 * time)` を esdirk34・1000 s・間隔 1 で回し、最初の t の進捗で止める）: `globe-mode-c` の単体試験（JSON と列の両方。`cancelled`、`GM-I11004`、列の最後の時刻が `termination.time`）と `tests/header.rs`（`typedef` の行を飛ばし、試験のモジュールの `extern "C" fn` を数えない）、`mise run c:zig`（呼ばれた回数 7）、`c:julia`（段は lowering・preflight・build・initialization・integration）、`elixir:test`（`Task` で回し、`assert_receive` で t の進捗を受けて `cancel`）、`gleam:test`（spawn と受信は `test/globe_mode_test_ffi.erl`）。
- NIF の列の道（`simulate_columns`）には進捗を付けていない（要るときに同じ形で足す）。

### 5 段目の結果（S136、LSP と Desktop、2026-11-03）

- 状態を Accepted にした（5 段がすべて入った）。exec・facade は変えていない。
- LSP（`globe-mode-lsp`）: `run_stdio` は `serve(server, input, out)` を呼ぶだけにし、`serve` で入力の読みを別のスレッドにした。読みのスレッドは枠を読んで `mpsc` のキューに積み、`globe/simulate`・`globe/simulateColumns` の要求には `CancelToken` を作って id（JSON の文字列）をキーにした表（`Arc<Mutex<HashMap>>`）に置く。`$/cancelRequest` はキューに積まず、表のトークンを立てる（キューで待っている要求にも効く）。`exit` か入力の終わりで読みが抜け、キューが閉じる。計算と書き出しは元のスレッドだけで 1 度に 1 つ（ADR-0023 の直列のまま）。`Server::handle` は `handle_with(msg, cancel, notify)` の包みにし、同期の試験は変えていない。
- 進捗は決定 (3) のとおり、要求の params に `workDoneToken` があるときだけ出す（クライアントが作るトークンなので `window/workDoneProgress/create` は要らない）。`begin`（`title` は `simulate <model>`、`cancellable: true`）→ facade の進捗ごとに `report`（`message` は段の小文字、t があれば `percentage` = (t − start) / (stop − start) × 100 の整数）→ `end`。`report` の value には Desktop の UI がそのまま使えるよう `stage`・`time`・`start_time`・`stop_time` も添えた。`initialize` の capability は足していない（独自の要求なので知らせる標準の欄が無い）。
- 中断の応答は決定 (4) のとおり普通の応答（`termination` が `cancelled`、途中までの列、`GM-I11004`）。
- Desktop: `Worker` の `Mutex` は実行中ずっと握られているので、UI からの `notify` では `$/cancelRequest` を送れない。`worker.rs` に `Canceller`（実行中の要求の id と sidecar の stdin の共有。`Mutex` の外）と command `cancel_run` を足した。sidecar が 2 s（`CANCEL_GRACE`）のうちに応えなければ（lowering・組み立て・ネイティブの型付きの道の積分の中）、決定 (3) の表のとおり kill して次の要求で作り直す。UI の `TauriBackend` は `workDoneToken` を付けて `globe-notification` の `$/progress` を `onProgress` に流し、`cancel()` で `cancel_run` を呼ぶ（作り直しで rpc が誤りになったら `cancelled: true` で reject。サーバが止めたときは途中の列の普通の応答）。`App.svelte` は変えていない。
- 試験: `globe-mode-lsp` の `cancel_request_stops_a_long_run_with_partial_columns`（`std::io::pipe` で `serve` を回し、`der(x) = sin(100 * time)` を 1000 s（esdirk34）の `globe/simulateColumns` の最初の t の `report` を待って `$/cancelRequest`。`begin`…`end` の後に `cancelled`・`GM-I11004`・最後の時刻が 0 と 1000 の間の列）。Desktop の `worker.rs` の `canceller_stops_a_long_simulate`（本物の `globe`、`desktop.yml` の `GLOBE_BIN`）。
- Desktop の既定のソルバ（`auto`、ネイティブの型付きの道）では積分の中で t が出ず、止めるのは段の境目か 2 s 後の作り直しになる（1 段目の持ち越しと同じ）。

### 持ち越し

- ネイティブの型付きの道（bdf・dopri45・auto）の積分の中では t も中断も出せない。session の道をネイティブの既定にする案は、JIT を使わない分の遅さを測ってから（ADR-0014 で数値は同じ。速さは【未確認】）。
- lowering と組み立ての中で止めること（上流の Rumoca に口が要る。ADR-0017 の外から止める道のまま）。
- CLI の Ctrl-C で途中の結果を書き出すこと。
- wasm で途中の列を返すこと（SharedArrayBuffer か、Worker の中で計算を区切って返す形。要るときに）。

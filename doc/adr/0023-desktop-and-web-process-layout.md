# ADR-0023 Desktop と Web のプロセス構成: 検査と実行は UI のプロセスから分け、Desktop は `globe` を sidecar に、Web は Web Worker に置く

- 状態: Accepted（2026-10-31、S040）
- 関係: [ADR-0014](0014-session-route-wasm.md)（wasm は session の道、Node で確かめる）、[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)（`msl-sim` のワーカーと時間切れ）、[ADR-0019](0019-rpi-aarch64-environment.md)（検査の RSS と swap）。[architecture.md §4](../architecture.md#4-ターゲットごとの実行方法) の「Tauri のプロセス内でネイティブ実行」をこの ADR で置き換える。計画書は [plan/2026-10-30-release-estimate-and-parallel-plan.md](../plan/2026-10-30-release-estimate-and-parallel-plan.md) の WP-J

## 背景

- P4（Desktop MVP）の Tauri のシェル（計画書の WP-C）を作る前に、検査（S0〜S10）と実行をどのプロセスで回すかを決めておく必要がある。[architecture.md §4](../architecture.md#4-ターゲットごとの実行方法) は「Tauri のプロセス内でネイティブ実行」と書いていたが、S005 以降の測定で次が分かった。
- **メモリ**: 検査は 1 モデルでも GiB の単位になる。`msl:funnel`（1 プロセスで 566 本、50 本ごとに Session を戻す）は x86_64 で最大の常駐メモリ（VmHWM）7.1〜7.2 GiB（[ADR-0019](0019-rpi-aarch64-environment.md)、[roadmap.md](../roadmap.md) の持ち越し 9）、RPi で 6.5 GiB、最新の [msl-funnel.md](../msl-funnel.md) で 2.0 GiB。`msl:sim` のワーカー（1 本ごとに別プロセス）は最大 3.4 GiB（[msl-sim.md](../msl-sim.md)）。wasm は MSL を読んだ後の線形メモリ 262 MiB、`ChuaCircuit` の検査と実行の後 513 MiB（CSV を返すと 634 MiB）、Node の RSS 886 MiB（[ADR-0014 の結果](0014-session-route-wasm.md)）。`Checker` のメモリの上限は「P4 のプロセス構成が決まってから」として持ち越していた。
- **止まり方**: Rumoca の lowering・組み立てが終わらないモデルがある（`DrumBoiler`、[ADR-0018](0018-no-preflight-limit.md)）。`msl-sim` はワーカーを別プロセスにして、時間切れ（180 s、lowering は 150 s）で**外から止めて回し直す**（[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)）。同じプロセスの中では、終わらない計算を安全に止められない。
- **panic**: wasm は panic=abort で、S036 は panic の後に Worker を捨てて作り直し、ライブラリとファイルを再生する道を作った（[roadmap.md](../roadmap.md) の S036）。ネイティブでも Rumoca の内部の `unwrap` などで panic する道は残っている（`msl:funnel` の panic は 0 本だが、未知の入力に対しては保証できない）。
- **入口はもうそろっている**: S035 の facade `globe-mode` と `globe-mode-protocol`（`CheckResponse`・`SimulateRequest`・`SimulateResponse` の JSON）、S038 の `globe lsp`（stdio の JSON-RPC、`Content-Length` の枠、1 スレッドで 1 要求ずつ）、S036 の `GlobeWorker`（呼び出しを直列にし、panic の後に作り直す）。Desktop の UI が同じ JSON を同じ枠で別プロセスへ送れば、新しい計算の道は要らない。
- 測定（S040、x86_64 の WSL2、RAM 3 GB + swap 16 GB、release ビルド。手順は [sessions/S040-adr-process-layout/解説書.md](../sessions/S040-adr-process-layout/解説書.md)）は次のとおり。

  | 項目 | 値 |
  | --- | --- |
  | `globe check --model Modelica.Electrical.Analog.Examples.ChuaCircuit`（MSL 4.1.0 を `--library`）の最大の常駐メモリ | **1.48 GiB**（VmHWM 1,554,836 kB。2 回で差 0.1%） |
  | 同じ検査の壁時計 | 5.2〜6.6 s（2 回。MSL の読み込みを含む） |
  | `globe lsp` の起動（プロセスの開始から `initialize` の応答まで。ライブラリなし） | 3 ms（3 回とも 2〜3 ms） |
  | `globe lsp --library <MSL>` の起動（同上。MSL を読む） | 2.3〜2.8 s（5 回） |
  | `globe --help` の起動から終了まで | 10 ms 未満 |

## 決定

### (a) 検査と実行は UI のプロセスから分ける

UI（Svelte）のプロセスでは検査も実行もしない。計算は 1 つの「ワーカー」に送り、ワーカーは **1 度に 1 要求**を処理する（S036 の `GlobeWorker`、S038 の `globe lsp` と同じ直列）。ワーカーが落ちた（panic・OOM・時間切れで外から止めた）ときは、UI が**ワーカーを作り直してライブラリと開いているファイルを再生する**（S036 の作り直しと同じ手順）。UI の状態（エディタの本文、開いているモデル、プロットの設定）はワーカーの外に置く。

### (b) Desktop（Tauri）: `globe` の CLI を sidecar にし、stdio の JSON-RPC で話す

- Tauri のシェルは `globe` を Tauri の sidecar（`tauri.conf.json` の `externalBin`。同じ配布物に同梱）として起動し、**S038 の `globe lsp` と同じ stdio の枠**で話す。エディタの LSP（診断・documentSymbol・hover）はそのまま使い、検査パネル・実行・グラフは **同じ接続に独自の要求を足す**（`globe/check`・`globe/simulate`・`globe/graph`。中身は `globe-mode-protocol` の JSON）。独自の要求の追加は WP-C（Tauri のシェル）で行い、この ADR では形だけを決める。
- Tauri の Rust 側（`src/Rust/apps/`）はシェルに徹し、facade `globe-mode` をリンクしない。コアをリンクするプロセスは `globe` の 1 つで、Desktop の配布物にあるコアの実体も 1 つになる。
- 時間切れは `msl-sim` と同じ考えで**外から止める**。UI は要求ごとに予算（既定は実行 180 s・検査は無し。[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)・[ADR-0018](0018-no-preflight-limit.md)）を持ち、超えたらプロセスを kill して (a) の作り直しに入る。
- ワーカーは 1 つだけ。並列に 2 つ以上のモデルを検査・実行したいときは、ワーカーの数を増やすのではなく、要求を待たせる（RSS の上限が人の環境の RAM で決まるため。[ADR-0019](0019-rpi-aarch64-environment.md) の swap と同じ判断）。

### (c) Web: Web Worker の wasm のまま。線形メモリの目標と上限を決める

- ブラウザは S036 の構成（module Worker の wasm、`GlobeWorker` で直列、panic の後は作り直し）のまま。サーバーでは計算しない（[features.md](../features.md)）。
- 線形メモリの**目標**: ライブラリ（MSL）を読んだ後 **128 MiB 以下**、1 つのモデルの検査と実行の後 **1 GiB 以下**。この ADR を書いた時点は 262 MiB / 513〜634 MiB だったが、同じ日に別のセッションの S037（[roadmap.md](../roadmap.md) の S037）が分割読み込みで **62 MiB / 113 MiB** にし、どちらも目標の内側になった。目標は上限の見張りとして残す。P6 の完了条件「marimo の WASM で 2 GB 未満」はこの目標の外側にある。
- 線形メモリの**上限**: `memory_bytes()` が **2 GiB** を超えたら、UI は次の要求の前に Worker を作り直す（wasm32 の上限 4 GiB の手前で、スマートフォンのブラウザでも落ちにくい値。実測の根拠は無く【未確認】、P6 で測って直す）。
- **実装（S223、#69）**: `worker.js` が応答に `mem`（`memory_bytes()`）を付け、`client.js` の `GlobeWorker` が次の呼び出しの前に上限と比べて作り直す（ライブラリとファイルは再生する。再生のあとは数え直す）。それまでは決めたまま実装が無く、Spice3 の 1 本で 668 MiB から 3802 MiB に伸びた後の大きなモデルで 4 GiB を超えて abort した。上限の値は `memoryLimit` で変えられる（試験）。
- ネイティブの Desktop には線形メモリの上限に当たるものを置かない。RSS は人の環境の RAM と swap で決まり、OOM で落ちたら (a) の作り直しで戻る。

### (d) 採らなかった案

| 案 | 採らなかった理由 |
| --- | --- |
| A. Tauri のプロセス内で facade を呼ぶ（architecture の元の記述） | 終わらない計算を止められない。panic・OOM で UI ごと落ちる。コアの実体が `globe` と Tauri の 2 つになる |
| B. Tauri のプロセス内のスレッドで回し、`catch_unwind` で受ける | 止められない計算は同じ。スレッドの kill は無い。OOM はプロセス全体に効く |
| C. 専用のワーカー crate（`globe-mode-worker`）を作り、独自の枠で話す | `globe lsp` の枠と `GlobeWorker` の直列化で足りる。crate と枠が 1 つ増えるだけで得るものが無い |
| D. ワーカーを複数持ち、モデルごとに並列に検査する | 1 本で GiB の単位の RSS なので、RAM 8 GiB の機械で 2 本は危ない。要るときに (b) の数を増やせばよい |
| E. Desktop でも wasm を使う（Tauri の WebView で Worker） | ネイティブより遅く（[ADR-0014 の結果](0014-session-route-wasm.md): 読み込み 10.5〜11.9 s 対 6.9 s）、線形メモリの上限に縛られる。Cranelift の JIT も使えない |

## 結果

- `globe` の CLI が Desktop の計算の実体になる。WP-C（Tauri のシェル）は sidecar の起動・再起動と、`globe lsp` への独自の要求の追加から始める。WP-I（リリース工程）の成果物の `globe` のバイナリが、そのまま Desktop の sidecar になる。
- `Checker` のメモリの上限（持ち越し 9）は、プロセスの外からの kill と作り直しで扱う。`Checker` の内側に上限を入れない。
- [architecture.md §4](../architecture.md#4-ターゲットごとの実行方法) の Desktop と Android の行を「`globe` の sidecar（stdio の JSON-RPC）」に書き換えた。Android は Tauri の sidecar が使えない可能性があり【未確認】、P6 で決める。
- 測定（背景の表）の数字を、P4 の完了条件「RSS の基準値を記録する」の最初の値にする。
- 持ち越し: sidecar の独自の要求の名前と形（WP-C）、Android のプロセス構成（P6）、線形メモリの上限 2 GiB の実測（P6）。

### S049 で決まったこと（WP-C、2026-10-31）

- **Tauri のシェルは別の Cargo workspace**（`src/Rust/apps/desktop`）。自分の `[workspace]` と `Cargo.lock`（406 パッケージ）を持ち、ルートの workspace に入れない。webkit2gtk を要求するので、手元と差分 CI の `cargo build --workspace` に巻き込まないため。依存は `tauri` 2 と `serde_json` だけで、facade `globe-mode` はリンクしない（desktop の `deny.toml` の `bans` で守る）。ビルドは [desktop.yml](../../.github/workflows/desktop.yml)（手動だけ）。
- **独自の要求の名前と形**（`globe lsp` の同じ接続。実装は `globe-mode-lsp`）は次のとおり。

  | 要求 | params | result |
  | --- | --- | --- |
  | `globe/check` | `{model}` | `CheckResponse` の JSON |
  | `globe/simulate` | `{request}`（`SimulateRequest` の JSON） | `SimulateResponse` の JSON |
  | `globe/simulateColumns` | `{request}` | `{response, layout, columns}`（S076。`csv` の無い `SimulateResponse`、`ColumnsLayout`、列ごとの数の配列で NaN / Inf は null。予算は `globe/simulate` と同じ） |
  | `globe/graph` | `{model, depth}` | graphology の JSON（`ConnectGraph::to_graphology`） |
  | `globe/format` | `{text, indent?, insertOrigin?}` | 整形後の本文（S053、[ADR-0024](0024-own-formatter-and-lint.md)。壊れていれば -32000） |
  | `globe/lint` | `{text}` | `[{rule, line, column, message}]`（S053） |

  params が読めなければ -32602、モデルが無いなどの失敗は -32000（`message` に理由）。ファイルは標準の `textDocument/didOpen`（UI の `setFile`）で渡す。
- **UI から sidecar まで**: Svelte の `TauriBackend`（`src/Svelte/src/lib/tauri-backend.ts`）→ `window.__TAURI__.core.invoke("rpc" | "notify", {method, params})` → Rust の `Worker`（`src/worker.rs`。stdio の枠、1 度に 1 要求）→ `globe lsp`。サーバの通知（`publishDiagnostics`）はイベント `globe-notification`。`@tauri-apps/api` は入れない。
- **sidecar の起動と作り直し**: `tauri.conf.json` の `externalBin` が `binaries/globe`（CI が target triple 付きの名前で置く）。実行ファイルと同じ場所の `globe` を `globe lsp --library <DIR>...` で起動し、`--library` の引数と環境変数 `GLOBE_LIBRARY` で MSL を渡す。**予算の kill も入れた**: `globe/simulate` は 180 s を超えたらプロセスを kill し、次の要求で作り直す。開いているファイル（`didOpen`・`didChange`、`globe/simulate` の `files`）は作り直しで再生する。落ちた（panic・OOM）ときも同じ。検査には予算を付けない（ADR-0018）。
- **CI の成果物**: artifact `desktop-linux`（.deb、ubuntu-24.04）と `desktop-windows`（.msi、windows-2025）。大きさと run の ID は [作業記録](../../log/work/2026-10-31-s049-desktop-tauri.md)。Linux では .deb を入れて xvfb でアプリを起動する smoke（sidecar に 1 モデルを検査させる）と、最大の常駐メモリの記録も回す。
- **持ち越し**: MSL を同梱するか取得するか（今は `--library` か `GLOBE_LIBRARY`）、署名と自動更新、MSL を読んだ後の Desktop 全体の RSS（smoke は MSL なし）、`didOpen` が毎回全モデルの検査（診断）を回す点（静かな `globe/setFile` を足す案）。

### S056 で決まったこと（2026-11-01）

- **Desktop で MSL が読める**: 起動時の `--library` / `GLOBE_LIBRARY` を command `libraries` で UI に返し、`TauriBackend.library()` は「sidecar が既にライブラリを持っていれば resolve（JSON は捨てる）、無ければ reject」にした。「MSL を読む」ボタン（App.svelte）は変えずに動く。
- **MSL を読んだ後の RSS の基準値**（`desktop.yml` run 41244465666、ubuntu-24.04 の .deb、xvfb、`GLOBE_LIBRARY=data/msl`、`Modelica.Electrical.Analog.Examples.ChuaCircuit`）: sidecar の VmHWM は検査の後 **1,565,032 kB（1.49 GiB）**、実行の後 **1,598,636 kB（1.52 GiB）**。アプリ（Tauri のシェル + WebKit）の VmHWM は **139,508 kB（136 MiB）**。背景の表の `globe check ChuaCircuit` 1.48 GiB と同じ桁で、Desktop 全体でも約 1.65 GiB。壁時計は WebKit の起動と終了を含めて 37.6 s。

- **持ち越し**: App.svelte の `WorkerBackend` と `TauriBackend` の切り替え（`inTauri()`）、MSL を同梱するか取得するか（今は `--library` か `GLOBE_LIBRARY`）、署名と自動更新、MSL を読んだ後の Desktop 全体の RSS（smoke は MSL なし）、`didOpen` が毎回全モデルの検査（診断）を回す点（静かな `globe/setFile` を足す案）。

### S054 で決まったこと（2026-11-01）

- **ネイティブの library も閉包だけ読む**（[ADR-0025](0025-native-library-closure.md)）: sidecar が使う `Workspace::with_libraries` を wasm の分割読み込みと同じ道にした。背景の測定の `ChuaCircuit` の最大 RSS 1.48 GiB は、この変更の前の値（全部読み）。変更後の値は [research/resources-2026-10-31.md](../research/resources-2026-10-31.md) の §6。

### S069 で決まったこと（2026-11-01）

- **MSL の置き場**: 同梱せず、初回に app data dir の `msl/4.1.0/` へ取得する。`--library`・`GLOBE_LIBRARY` があればそれを優先し（この ADR の (a) の道はそのまま）、無ければ UI のボタンが command `fetch_msl` を呼ぶ。取得したら sidecar を新しい `--library` で作り直す。理由と比べた案は [ADR-0030](0030-desktop-msl-placement.md)。

### S092 で決まったこと（Android、2026-11-02）

- **Android では sidecar を使わない**: Android のアプリは別プロセスの実行ファイル（`globe`）を同梱して起動できない（Tauri の `externalBin` は Desktop だけ）。だから (b) の道は Android に効かない。
- **Android は Web と同じ (c) の道**: Tauri の mobile の入口（`src/Rust/apps/desktop/src/lib.rs` の `run`、`#[tauri::mobile_entry_point]`）は command を出さず、UI を表示するだけ。UI（`App.svelte`）は `__TAURI__` があっても User-Agent が Android なら `WorkerBackend`（wasm の Web Worker）を使う。`tauri.android.conf.json` が `externalBin` を空にする。APK は `android.yml`（署名なしの debug）が作る。
- **持ち越し（P6）**: Rust のコア（facade）を Tauri の mobile の lib に直接 link して command で呼ぶ道（線形メモリの上限が無くなる代わりに、UI のプロセスで計算する。(a) を Android でどう守るかを決めてから）、~~Android での MSL の置き場~~（S103 で済）、~~エミュレータで検査と実行を UI から自動で回すこと~~（S100 で済）。

### S100 で確かめたこと（Android、2026-11-02）

- **Android のエミュレータで UI から検査と実行が通る**（P6 の Android の完了条件）: (c) の道（`WorkerBackend`、wasm の Web Worker）が Tauri の Android の WebView でそのまま動く。`App.svelte` はビルド時の `VITE_GM_SMOKE=1`（Android はクエリを渡せない）か `?smoke=1` のときだけ、起動時に Decay を検査・実行して `GM-SMOKE check=ok simulate=ok x1=…` を console に出す。`android.yml` はこの設定で UI を作り、API 34 のエミュレータの logcat でこの行を待つ（run 41444392188、x1 = 0.3678797279080248。console のタグは `Tauri/Console`）。既定のビルドの挙動は変えていない。[解説書](../sessions/S100-android-emulator-smoke/解説書.md)。

### S103 で決まったこと（Android の MSL、2026-11-03）

- **Android の APK は MSL.json を同梱する**（S092 の持ち越しの「Android での MSL の置き場」）。比べた 2 つの道は次のとおり。
  - (a) Desktop と同じ `fetch_msl`（S069・S084）を mobile でも動かす: `#[cfg(mobile)]` の入口に command と `msl.rs`（取得・展開）を載せ、さらに app のデータディレクトリの `.mo` を WebView の `WorkerBackend` に渡す新しい道（command で JSON を返す等）が要る。Rust と TS の両方に新しいコード、初回は回線が要る。
  - (b) ビルド時に同梱する: `android.yml` のジョブ `ui` で `cargo xtask fetch-msl` してから `pnpm build` するだけで、既存の `pack-msl.mjs` が `MSL.json` を `dist`（Tauri が APK に埋める）に入れ、UI の「MSL を読む」はブラウザと同じ `fetch("msl/MSL.json")` の道でそのまま動く。アプリのコードは変えない。
  - → (b) を採った（変更は workflow の 1 段だけ。回線も要らない）。形はブラウザと同じ全部読みの `MSL.json`（15.2 MiB、2556 ファイル）。S054 の分割読み込みは UI がまだ使っていないので、使うときにブラウザと一緒に替える。
- **APK の大きさ**: debug の universal APK（aarch64 と x86_64）が 251,629,679 → 257,983,183 bytes（+6,353,504 bytes、+6.1 MiB。Tauri が frontend を ABI ごとの lib に圧縮して埋めるので、gzip で約 2.6 MB の MSL.json が 2 つ分入る）。
- **確かめ方**: S100 の smoke が Decay の後に「MSL を読む」と同じ `loadMsl()` を呼び、`Modelica.Electrical.Analog.Examples.ChuaCircuit` を検査して error 0 なら `msl=ok` を同じ行に足す。`android.yml` は `msl=ok` が無ければ失敗（run 41521456071、`Displayed` の 13.6 s 後に `GM-SMOKE check=ok simulate=ok x1=0.3678797279080248 msl=ok`）。[解説書](../sessions/S103-android-msl/解説書.md)。
- 持ち越し: ~~release の署名つき APK~~（S106 で済）、Rust を mobile の lib に直接 link する道、MSL 4.0.0 の切り替え（Desktop だけ。Android は 4.1.0 のみ）。

### S106 で決まったこと（Android の release APK、2026-11-03）

- **release の APK は `android.yml` の `apk` が ABI ごとに作る**（S092・S103 の持ち越し）。`cargo tauri android build --apk --split-per-abi --target aarch64 --target x86_64`。配るのは aarch64（artifact `globe-android-release`）、x86_64 はエミュレータの smoke 用。debug の universal APK（artifact `globe-android`）は変えない。大きさは Step Summary に並べる（run 41544046811: debug の universal 257,983,951 bytes（246.0 MiB）、release の aarch64 12,706,158 bytes（12.1 MiB）、x86_64 13,290,251 bytes（12.7 MiB）。debug は lib のデバッグ情報を残すので約 20 倍）。
- **署名は secret があるときだけ**: job の `env` に `ANDROID_KEYSTORE_BASE64`・`ANDROID_KEYSTORE_PASSWORD`・`ANDROID_KEY_ALIAS`・`ANDROID_KEY_PASSWORD` を写し（secret は `if:` に直接書けない）、`ANDROID_KEYSTORE_BASE64` が空でなければ `gen/android/keystore.properties`（`.gitignore` 済み）を書く。`build.gradle.kts` はこのファイルがあるときだけ `signingConfigs.release` を作る（Tauri の手引きの形。パスワードは `storePassword`・`keyPassword` に分けた）。無ければ `app-*-release-unsigned.apk`。鍵はリポジトリに置かない。
- **smoke**: Tauri（wry の `Logger.kt`）は `BuildConfig.DEBUG` のときだけ WebView の console を logcat に出すので、配布物を upload したあと x86_64 だけを `GM_SMOKE_DEBUGGABLE=1` で作り直し（`build.gradle.kts` が release に `isDebuggable = true` を立てる。R8 と release の Rust はそのまま）、署名なしなら runner の中で作る使い捨ての鍵（`keytool`、有効 1 日）で `apksigner` 署名して入れる（runner に `~/.android/debug.keystore` は無い）。`emulator` は matrix で debug と release を回し、どちらも `GM-SMOKE … msl=ok` を待つ（run 41544046811）。UI は debug と同じ `VITE_GM_SMOKE=1` のビルド。
- **鍵の作り方と secret の登録**（ユーザーがやる）: `keytool -genkey -v -keystore ~/globemode-upload.jks -keyalg RSA -keysize 2048 -validity 10000 -alias upload`、`base64 -w0 ~/globemode-upload.jks | gh secret set ANDROID_KEYSTORE_BASE64`、残り 3 つは `gh secret set <名前>`（対話で値を入れる）。出典: [Tauri: Android Code Signing](https://v2.tauri.app/distribute/sign/android/)・[GitHub: Using secrets in GitHub Actions](https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions)・[Android Developers: Sign your app](https://developer.android.com/studio/publish/app-signing)（いずれも確認日 2026-11-03）。[解説書](../sessions/S106-android-release-apk/解説書.md)。
- 持ち越し: secret を登録した run での署名つき APK の実機確認、配布用の UI を smoke なしで作るか、Rust を mobile の lib に直接 link する道、MSL 4.0.0 の切り替え。

### S145 で決まったこと（ブラウザ版の配布物、2026-11-04）

- **ブラウザ版は、サーバーで計算しない静的ファイルの束として配る。** 手動の `web.yml` が wasm と UI（vite の `base: "./"`。どのサブパスでも動く）、`MSL.json`、CSP の header の断片（`_headers`・`nginx.csp.conf`。正は `index.html` の meta で、Worker に効かせるため配布先の header に写す）を artifact `globe-web` に出す。`Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` は要らない（`SharedArrayBuffer` を使わない）。**公開（GitHub Pages の有効化、URL の配布）はユーザーの判断で、このセッションでは行わない。** 持ち越し: PWA、ダークモード、~~`MSL.json` の分割配信~~（S301 で済。下の付記）。

### S154 で決まったこと（最近使ったファイルの読み込み、2026-11-05）

- **dialog を通さないパス（S151 の「最近使ったファイル」）は、Tauri の command `read_recent { path }` で Rust が読む。** fs プラグインは dialog で選んだパスだけを許可に入れるので（S141）、保存しておいたパスを起動後に開くには別の道が要る。選択肢のうち、`tauri-plugin-persisted-scope` は新しい依存になるので採らず、fs の scope を起動時に一覧で広げる道は一覧（UI の `localStorage`）を信用して許可を広げることになるので採らず、command を 1 つ足した（`generate_handler!` に `read_recent`、依存 0）。
- **任意のパスを読ませないための検証**: パスは UI（WebView の JS）から来るので、`read_mo` が(1) 拡張子が `.mo`、(2) `std::fs::metadata` で存在する通常のファイル（ディレクトリ・無いパスは Err）の 2 つを満たすものだけを `read_to_string` する。`/etc/passwd` のような拡張子の違うものは読まない。相対パスは作業ディレクトリから（CI の smoke はこれで `Recent.mo` を開く）。「開く」の dialog は従来どおり fs の許可（`TauriBackend.openFile`）。
- shortcut: 検査は拡張子と種類だけで、`.mo` という名前なら任意の場所のものを読める（シンボリックリンクも辿る）。UI の JS が任意のコードに乗っ取られた場合の防御ではなく、誤ったパスと意図しない種類のファイルを渡さないための検査。サイズの上限は無い。
- 確かめ方: `main.rs` の単体試験（`.mo` の通常のファイルを読み、`.txt`・ディレクトリ・無いパスは Err）と、`desktop.yml` の測定の `GM-SMOKE … recent=ok`（作業ディレクトリの `Recent.mo` を最近使ったファイルとして開き、本文が入る）。

### S141 で決まったこと（ファイルを開く・保存する、2026-11-04）

- **自分の `.mo` を持ち込み・持ち帰る道は、環境ごとに別の実装を `Backend.openFile` / `saveFile` の後ろに置く。** ブラウザと Android（sidecar が無く `WorkerBackend` を使う）は `<input type=file>` と Blob のダウンロード（保存先はブラウザが決める）。Desktop は公式プラグイン `tauri-plugin-dialog` と `tauri-plugin-fs`（Desktop だけの依存で、Android の lib には入れない）で、UI は npm の `@tauri-apps/plugin-*` を入れず `window.__TAURI__.dialog` / `fs` を使う（S135 と同じ）。権限は `capabilities/desktop.json` に `dialog:allow-open`・`dialog:allow-save`・`fs:allow-read-text-file`・`fs:allow-write-text-file` だけ。dialog で選んだパスだけが fs の許可に入るので、静的なスコープは置かない。command（`generate_handler!`）は足していない。
- 本文は sidecar にも渡さずに UI の中で読み書きする（`setFile` へ通すのは今まで通り検査・実行のとき）。

### S135 で決まったこと（Desktop の自動更新、2026-11-03）

- **updater は Tauri 2 の公式プラグイン**（`tauri-plugin-updater` 2.14、Desktop だけ。`Cargo.toml` の `cfg(not(any(target_os = "android", target_os = "ios")))`、権限は `capabilities/desktop.json` の `updater:default`（`platforms` を Linux・Windows・macOS に限る））。2.10 から .deb・.rpm・AppImage・.msi・NSIS のどれでも入れ直せ、`latest.json` のキーは `<os>-<arch>-<installer>` を先に、無ければ `<os>-<arch>` を引く（.deb は `pkexec dpkg -i`）。出典: [Tauri: Updater](https://v2.tauri.app/plugin/updater/)・[plugins-workspace の updater の CHANGELOG](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/CHANGELOG.md)・[updater.rs](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs)（いずれも確認日 2026-11-03）。
- **設定**: `tauri.conf.json` の `plugins.updater` に `endpoints`（`https://github.com/asari300/20261018_GlobeMode/releases/latest/download/latest.json`。prerelease は対象外）と `pubkey`（今は空）、Windows は `installMode: passive`。`pubkey` が空のビルドは `main.rs` がプラグインを入れず、command `updater_configured` が偽を返し、UI（`TauriBackend.checkUpdate()`、ボタン「更新を確認」。Desktop だけ）は「未設定」と出す。JS は npm の `@tauri-apps/plugin-updater` を入れず `window.__TAURI__.updater`（`withGlobalTauri`）を使う。
- **配布と署名は `release.yml`**（`desktop.yml` は smoke と試験のまま）: ジョブ `desktop-ui`（vite build）→ `desktop`（Linux の .deb・Windows の .msi。sidecar の `globe` を作って置き、`cargo tauri build --config` でタグの版と `createUpdaterArtifacts` を渡す）→ `release`（SHA256SUMS と `latest.json`）。**署名は secret があるときだけ**（S106 と同じく `env` に写して分岐）: `TAURI_SIGNING_PRIVATE_KEY` が空でなければ `createUpdaterArtifacts: true` で `.deb.sig`・`.msi.sig` を作り、`release` が `.sig` の中身を `signature` に、Release の資産の URL を `url` にした `latest.json`（`linux-x86_64-deb`・`windows-x86_64-msi`）を作る。無ければ署名なしの bundle だけで `latest.json` は作らない。手動実行（`workflow_dispatch`）は従来どおり dry-run で、Release には付けず artifact（`globe-desktop-linux`・`globe-desktop-windows`、`SHA256SUMS`）に残す。
- **鍵の作り方と secret の登録**（ユーザーがやる。鍵はリポジトリに置かない）: `cargo tauri signer generate -w ~/.tauri/globemode.key`（パスワードを決める）、`gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/globemode.key`、`gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（対話で入れる）、公開鍵 `~/.tauri/globemode.key.pub` の中身を `tauri.conf.json` の `plugins.updater.pubkey` に書いてコミットする。秘密鍵を失うと、それまでに配ったアプリへ更新を届けられない（Tauri の手引き）。
- 持ち越し: 署名つきの Release からの実機での更新の確認（Linux は pkexec の確認、Windows は passive の msi）、Desktop の THIRD-PARTY.md、macOS。

### S143 で決まったこと（Desktop の設定ファイルの根、2026-11-04）

- **Desktop は、起動時の作業ディレクトリ（`--root <DIR>`、無ければ `GLOBE_ROOT`、無ければカレントディレクトリ）を sidecar の `initialize` に `rootPath` で渡す。** sidecar（`globe lsp`）は根から上へ `globe-mode.toml` を探し（S130、[ADR-0024](0024-own-formatter-and-lint.md)）、整形と lint に使う。`rootUri` でなく `rootPath` にしたのは LSP が両方を読むためで、URI の組み立てを Desktop に持たせない。作り直した sidecar にも同じ根を渡す。UI は整形・lint の引数を渡さず設定に任せる（ブラウザ版の wasm は設定ファイルを持たないので、`WorkerBackend` が既定の 2・true を渡す）。設定の読み直しは command `reload_config`（`workspace/didChangeConfiguration`）。根を実行中に替える（フォルダを開く）ことは持ち越し。

### S134 で決まったこと（Linux の locale、2026-11-03）

- **Linux の Desktop は、locale が `C.UTF-8` などのとき起動時に `en_US.UTF-8` に替えてから WebView を作る**（S125 で見つけた白画面の直し）。WebKitGTK は既定の言語を `setlocale(LC_CTYPE, nullptr)` から決め、`C`・`POSIX` そのものなら `en-US` にするが、`C.UTF-8` は `_` を `-` に替えて `.` 以降を落とすので `C` になる（[WebKit: Source/WTF/wtf/unix/LanguageUnix.cpp](https://github.com/WebKit/WebKit/blob/main/Source/WTF/wtf/unix/LanguageUnix.cpp)、確認日 2026-11-03）。`C` は BCP 47 の言語タグではないので、既定の locale を使う `Intl` が `RangeError: invalid language tag: C` を投げる。比べた 2 つの道: (a) UI で `Intl.Segmenter` の前に既定の言語を確かめて `en` を渡す。ただし module の評価のときに既定の locale で `Intl` を作るのは CodeMirror の `@codemirror/commands`（`new Intl.Segmenter(undefined, …)`）だけでなく uPlot（`new Intl.NumberFormat(navigator.language)`・`new Intl.DateTimeFormat()`）もあり、全部を包むことになり、依存が増えるたびに漏れうる。(b) Rust の `main` の最初（他のスレッドと GTK の `setlocale(LC_ALL, "")` の前）で、効いている変数（`LC_ALL`、無ければ `LC_CTYPE`、無ければ `LANG`）が `C.`・`POSIX.` で始まるときだけ、その変数を `en_US.UTF-8` にする。→ (b) を採った（`main.rs` の `fix_c_locale`、20 行。言語の根で直すので `Intl` の使い手によらない）。`en_US.UTF-8` が入っていない環境では `setlocale` が失敗して `C` のままになり、WebKit はやはり `en-US` を使う。確かめ方は `desktop.yml` の UI の測定の段から S125 の `LANG: en_US.UTF-8` を外し、ランナーの既定の locale のまま `GM-SMOKE … msl=ok` が 3 回出ること。[解説書](../sessions/S134-desktop-lang-c-fix/解説書.md)。

### S147 で決まったこと（smoke に UI の操作、2026-11-04）

- **Desktop と Android の UI の操作は、WebView の外から叩かず、UI の中の `smoke()` が回す。** Tauri の WebView（WebKitGTK・WebView2・Android の WebView）には Playwright をつなげないので、`smoke()` が `openLibrary`・`format`・`graph`・`simulate`・`definition` を呼び、実行の設定の欄、「止める」のボタン、プロットの信号のチェックは要素を操作する。結果は `GM-SMOKE` の行に `tree= fmt= graph= cancel= settings= plot= def=`（`=ok` / `=ng` / MSL が無いブラウザで `=skip`）で出し、`desktop.yml`（Linux・Windows）と `android.yml` が全部 ok を合格にする。ブラウザの `test_ui.py` も同じ行を確かめる。
- 結果: sidecar（`TauriBackend`）、Worker（`WorkerBackend`、Android）のどちらでも 7 項目が最初の run から ok だった（desktop.yml run 41706643110、android.yml run 41706648182）。バックエンドの違い（sidecar の「止める」は `$/cancelRequest` か sidecar の作り直し、Worker は `terminate()`）は `cancel` 1 項目で両方を通っている。どちらの道かは行に出していない（持ち越し）。

### S160 で決まったこと（macOS、2026-11-05）

- **Desktop は macOS（arm64）でも `desktop.yml` でビルドし、smoke と測定が通る。** Rust のコードの変更は要らなかった（`fix_c_locale` は Linux だけ。sidecar は `.app` の `Contents/MacOS/globe` に実行ファイルと並んで入り、`sidecar()` の既定の場所で見つかった）。署名・公証はしていない。測り方と結果は [ADR-0034](0034-ui-bench-method.md) の付記と [bench/ui.md](../bench/ui.md)。
- 持ち越し: `release.yml` への macOS（署名・公証の鍵はユーザーの判断）、.dmg を Finder から開く経路と自動更新の確認【未確認】、x64 の macOS。

### S176 で決まったこと（release.yml の macOS、2026-11-05）

- **`release.yml` が macOS（arm64、`macos-latest`）の Desktop の .dmg も作る**（`--bundles app,dmg`。署名・公証なし）。鍵（`TAURI_SIGNING_PRIVATE_KEY`）があるときだけ updater 用の `GlobeMode_<版>_aarch64.app.tar.gz` と `.sig` も付け、`latest.json` に `darwin-aarch64` を足す。鍵が無い今は署名なしの bundle だけ。あわせて `globe` の tar.gz と wheel も macOS で作る。経緯と dry run の結果は [S176 の解説書](../sessions/S176-release-macos/解説書.md)。
- 持ち越し: x86_64 の macOS、署名・公証の鍵（ユーザーの判断）、鍵を登録したあとの macOS の更新の確認【未確認】。

### S197 で決まったこと（Desktop のレビューの指摘、2026-11-06）

- **中断は予算の無い要求にも効く。** `globe/check` などは予算を付けない決まり（[ADR-0018](0018-no-preflight-limit.md)）のまま。ただし `Worker` は予算の有無にかかわらず実行中の要求を `Running` に登録し、`wait` は予算が無くても 0.2 秒ごとに止める合図を見る。`cancel_run` から 2 秒（`CANCEL_GRACE`）で応えなければ sidecar を kill して作り直す。以前は応えない検査で `Worker` の `Mutex` を握ったままアプリ全体が止まり、中断も効かなかった。CI で回る試験は `cancel_kills_a_silent_check_without_budget`（initialize だけ答えて黙る sh のスクリプト）。
- **MSL の切り替えで sidecar を作り直せなかったら、ライブラリの設定を元に戻す。** `fetch_msl` は同時に 1 つだけ（取得中の旗。2 つ目は「取得中」の誤り）。HEAD は `--max-time 10`、展開のループも取り消しの旗を見る。
- **UI から中継する method を限る（多層防御）。** `rpc` は `globe/*` の 18 個、`notify` は `textDocument/didOpen` だけ。それ以外（`shutdown`、`$/cancelRequest`、`textDocument/didChange` など）は誤りで返す。一覧は `src/shell.rs` の `RPC_METHODS`・`NOTIFY_METHODS`。試験が `tauri-backend.ts` の `method: "…"` を拾って一覧と一致を確かめるので、UI が口を足すと試験が落ちて一覧の更新に気づく。注入の道は見つかっていない（`{@html}`・`innerHTML` は 0 件）。
  - **付記（S227、2026-11-08）**: 上の「18 個」は S197 の時点。いまの `RPC_METHODS`（`src/Rust/apps/desktop/src/shell.rs`）は **20 個**で、`globe/*` が 19 個、`textDocument/foldingRange` が 1 個。S222 が `globe/readResults`（結果を開く）、S225 が `textDocument/foldingRange`（折りたたみ）を足した。以後も一覧と数は `shell.rs` を正とする。
- **CSP を入れた。** `tauri.conf.json` の `app.security.csp` は、ブラウザ版が使う `index.html` の meta の CSP と同じものに、IPC の `connect-src` だけを足したもの。`default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self' ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'; img-src 'self' data:`。`'wasm-unsafe-eval'` は Android の `WorkerBackend`（wasm）、`'unsafe-inline'` は Svelte と CodeMirror が注入する style のため。meta の `connect-src 'self'` は元のままなので、実際の IPC は以前と同じ道を通る（meta と header は両方が効く）。両者が食い違わないことを試験（`tauri_csp_is_the_index_html_csp_plus_ipc`）が確かめる。Tauri は CSP の書式に `ipc: http://ipc.localhost` を示している（出典: <https://v2.tauri.app/security/csp/>、確認日 2026-11-06【文書を直接は取得できず、tauri-utils 2.10.2 のソースの例示で確認】）。Tauri は HTML に `<style>` や外部 http の script があるときだけ nonce・hash を足す。Vite の出力には無いので CSP は変わらない。Desktop（Linux・Windows・macOS）と Android の smoke が全項目 ok のままであることは run の ID を作業記録に書いた。
- **限界（変えていない）:** `globe/exportFmu` の `outDir` は dialog で選んだパスに限られない。sidecar の `outDir` は任意のディレクトリで、UI の JS を乗っ取れる者は `outDir` を好きに言える（FMU と同名のフォルダを作る）。UI は `dialog.save` の親ディレクトリを渡している。限るには Rust 側で dialog を呼ぶ必要があり、UI の構成を変えるので持ち越し。`read_recent`（`.mo` の通常のファイルだけ）は S154 のまま。

### S301 で決まったこと（MSL.json の分割配信、2026-11-15）

- **背景**: S145 の持ち越し。分割読み込み（S037）は Worker の線形メモリを減らしただけで、UI は「MSL を読む」で `MSL.json`（15.2 MiB、gzip で 2.7 MB）を全部取り、JSON の文字列を Worker に渡して読み込んでいた。ChuaCircuit の閉包は 295 / 2556 ファイル（2.4 MB）しか使わない。
- **形**（`scripts/pack-msl.mjs` と `globe-mode-wasm/web/pack.js` が作る。依存の追加なし）:
  - `msl/index.json`: ファイルごとの**見出し**（`globe_mode_fmt::tree_stub`。`within` と、入れ子を含むクラスの種類・名前・説明文字列だけ。`LibraryTree` の結果と分割読み込みの修飾名の索引が本文のときと同じになる）と、ファイル → 塊の番号。見出しにできないファイル（クラス名を引用符で囲んだ入れ子がある `ModelicaReference/package.mo`）は本文のまま `real` に入れる。
  - `msl/c/<番号>.json`: 同じディレクトリのファイルを 16 KiB までまとめた塊 `{ 名前: 本文 }`（838 本）。
- **Worker の動き**（`worker.js` の `library_index`・`ensure`）: 見出しだけの `Workspace` を作る。モデルや文書を使う操作（check・simulate・graph・flat・definition・hover・completion・diagram など）の前に、`LibraryClosure`（frontend。S238 の `library_closure` と同じ走査で、ファイルごとの走査の記憶を持ち越す）で閉包を求め、答えに見出しのままのファイルがあれば、その塊を `fetch` して `set_text` で本文に替え、閉包を求め直す。本文だけが参照を持つので、見出しのファイルが無くなるまで繰り返す（全部読みと同じ閉包に落ち着く）。取ったら `Workspace` を作り直し、`set_file` した文書を足し直す。木から開くファイルは、そのファイルの塊だけを取る。**木の行ごとに引くアイコンと説明は、クラスの `extends` の基底だけをたどる**（`bases_only`。そうしないと、`ObsoleteModelica4` のように多くを参照するクラスの行を描くだけで塊の 7 割を取った）。
- **変えなかったこと**: Service Worker・独自のキャッシュは足さない（ブラウザの HTTP キャッシュに任せる）。塊も同じ origin なので CSP の `connect-src 'self'` のまま。`MSL.json` は pack が引き続き作る（Leptos・Dioxus・marimo と、`index.json` が無い配信への代わり。UI は `index.json` が無い・JSON でないときだけ `MSL.json` に戻る）。`web.yml` の束 `globe-web` には入れない。Desktop（sidecar）は影響なし。
- **確かめ方**: `node globe-mode-wasm/node/split.mjs`（`mise run wasm:node` が 5 本、全 20 本は手で）が、`msl-compare.txt` の 20 本で全部読みと検査結果（passed・段階・診断のコード）が一致すること、作業中の文書が MSL を参照するとき、木と開くファイルが一致することを確かめる（20/20 一致）。`test_ui.py` の既存の MSL の試験はすべて分割配信で通り、新しい試験が `MSL.json` を取らず索引と塊だけで ChuaCircuit の検査が通ること・取る量が `MSL.json` の半分未満であることを確かめる。
- **限界**: (1) 塊は閉包の深さだけ順番に取る（ChuaCircuit で 11 回、Pendulum 8 回、PID_Controller 6 回。各回の中は並列）ので、往復の遅い回線では回数ぶん待つ。(2) 索引（1.5 MiB、gzip で 0.2 MB）は必ず取る。ブラウザ（`?smoke=1`）では、検査のほかに木の行のアイコンなどで塊を足し、ChuaCircuit までに 174 要求、4.8 MiB。(3) 閉包の外のファイルは見出しのままなので、そこの型のメンバーを引く補完などは本文を取るまで出ない。(4) Android の APK は dist に `MSL.json` も入る（S308 で外した）。(5) `bench_ui.py` の Svelte の `msl_ms` は S301 から分割配信の値で、Leptos・Dioxus（`MSL.json` 全部）とは条件が違う。

### S308 で決まったこと（塊の先読みと Android の APK の分割配信、2026-11-15）

S301 の持ち越し 2 つ。

- **塊の先読み**（往復の回数）: 索引 `index.json` に、塊ごとの「その塊のファイルが直接参照する他の塊」`deps`（塊の番号の配列。`pack.js` が作る）と、先読みの深さ `prefetch`（既定 2）を足した。直接の参照は閉包と同じ走査（`SplitLibrary::deps_of`。`extends`・型・`import`・外側のパッケージ）で、wasm の `file_deps` が全部の本文から求める（pack の時だけ。実行時の閉包は変えない）。Worker の `fetchChunks` は、閉包に要るファイルの塊に加えて、そこから `deps` を `prefetch` 段たどった塊をまとめて 1 回の `Promise.all` で取る。**閉包の正しさは今までの繰り返しのまま**（先読みは目安。足りなければ見出しのファイルが残り、次の回で追加の塊を取る。先読みが取りすぎても、本文が増えるだけで閉包は変わらない）。
- **測り方**: `node globe-mode-wasm/node/split.mjs --prefetch N`（Node、release の wasm、全部読みと同じ検査結果になることは毎回確かめる）。ChuaCircuit の初回（索引は先読み無しの 1.5 MiB に deps の約 50 KB が足される）:

  | 深さ | 往復 | 塊 | 取る量（索引 1.5 MiB を含む） |
  | ---: | ---: | ---: | ---: |
  | 0（S301） | 11 | 163 | 4.64 MiB |
  | 1 | 5 | 174 | 4.78 MiB |
  | **2（既定）** | **3** | **179** | **5.16 MiB** |
  | 3 | 3 | 182 | 5.19 MiB |
  | 4 | 2 | 189 | 5.26 MiB |
  | 6 | 1 | 192 | 5.29 MiB |

  20 本を続けて回した合計は、先読み無し 39 往復・235 塊・8.8 MiB、深さ 2 で 17 往復・258 塊・9.4 MiB、深さ 4 で 16 往復・265 塊・9.5 MiB。**深さ 2 を既定にした**（往復が ChuaCircuit で 11 → 3、20 本の合計で 39 → 17 に減り、取りすぎは 1 割ほど（ChuaCircuit は +0.5 MiB、20 本は +0.6 MiB）。3 より先は往復があまり減らず、取りすぎだけが増える）。往復あたりの待ちは回線しだいで、ここでは測っていない（往復 × 往復時間で、たとえば 100 ms なら ChuaCircuit の塊の待ちは約 1.1 s → 約 0.3 s という計算上の見積もりで、実測ではない）。
- **木のアイコンは先読みしない**: 基底だけの閉包（`bases_only`。木の行ごとのアイコンと説明）は `deps`（全部の参照）をたどらない。たどると、ブラウザ（`?smoke=1`）の ChuaCircuit までが 218 要求・6.9 MiB に膨らんだ（S301 は 174 要求・4.8 MiB）。たどらないと 192 要求・5.35 MiB（Chromium、`/msl/` の content-length の合計。索引を含む）で、Node の増え方（+0.5 MiB）と合う。
- **試験**: `split.mjs` が既定の先読みのとき、ChuaCircuit の塊の取得が 4 往復以内・190 塊以内であることを確かめる（20 本の一致は従来どおり）。`globe-mode-frontend` に `direct_deps_are_one_step`。
- **Android の APK**: `android.yml` の `ui` ジョブが、`pnpm build` のあとに `dist/msl/MSL.json` を消し、`index.json` の塊が全部あることを確かめてから APK に渡す。Android は sidecar を使わず WorkerBackend なので、UI は `index.json` を見つければ分割配信で読む（ブラウザ版と同じ道）。エミュレータの smoke（`msl=ok`）が、塊だけで ChuaCircuit を検査できることを確かめる。**APK は軽くならなかった**（run 43665628896 と main の run 43663027170 の比べ: arm64 release 13,616,490 → 14,859,242 B、x86_64 release 14,202,127 → 15,444,927 B、universal debug 259,532,967 → 261,050,263 B。塊 838 本は別々に圧縮され、重複が効かないため。zlib -9 で `MSL.json` 2.65 MB に対し塊の合計 3.30 MB と索引 0.22 MB）。分割配信の利点は実行時の線形メモリ（62 → 14 MiB）で、smoke の `msl_ms` は前後で変わらない（9〜11 s）。APK の大きさを優先するなら、Android だけ `MSL.json` に戻す選択肢がある。（**S340 で、塊を 1 本のパックにして戻した**。下の S340。）
- **限界**: (1) 先読みは塊の依存の目安。塊の粒度（16 KiB）のぶん、使わないファイルの参照も追うので、深さを上げると取りすぎが増える。(2) 往復の回数は Node で数えた値。ブラウザの実際の回線での時間は測っていない。(3) `MSL.json` は Leptos・Dioxus・marimo と、索引が無い配信のために pack が作り続ける。

**S321 の付記（#110）**: 分割配信の塊が壊れている・索引と合わないときに Worker が固まらないようにした。`fetchChunks` は全部の塊を取って読めてから本文を入れ、索引が「その塊にある」と言うファイルが見出しのままなら誤り（「塊に無いファイル」）にして、その塊を取得済みに入れない（配信が直れば次の呼び出しで取り直す）。壊れた塊は「壊れている」の誤り。`ensure` の繰り返しは塊の数 + 1 回で打ち切る。試験は `globe-mode-wasm/node/broken-chunks.mjs`（MSL 不要。`mise run wasm:node` に入れた）。

### S340 で決まったこと（Android の束を 1 本のパックにした、2026-11-15）

S308 で APK が +1.24 MB（arm64 で +9.1%）増えた件。ユーザーの指示（2026-11-15）で、**案 A（パック 1 本 + slice）と案 B（Android だけ塊の cap を上げる）の両方を実装して同じ表で比べ、優れた方を採った**。**案 A を採り、案 B は外した**。

- **原因**: Tauri は frontend の asset を 1 つずつ brotli（quality 9。debug は 2。`tauri-codegen` 2.7.4 の `embedded_assets.rs` の `compression_settings`、確認日 2026-11-15）で圧縮して Rust の binary に埋める。塊（16 KiB）は 1 本ずつ圧縮されて辞書が持ち越されないうえ、838 本ぶんの名前の表の項目もある。brotli（q9、窓 4 MiB）で見積もると、`MSL.json` 15.95 MB は 2.08 MB、塊 838 本の合計は 3.06 MB + 索引 0.19 MB。この見積もりの差 1.17 MB は、APK の実測の差 1.24 MB とほぼ合う。
- **案 A（採用）**: Android の束だけ、塊の JSON を UTF-8 のまま 1 本に連結した `msl/pack.bin` と、索引 `index.json` の `pack: { url, at }`（`at[i]〜at[i+1]` が塊 i のバイト範囲）にする。`pack-msl.mjs` は環境変数 `GM_MSL_PACK=1`（`android.yml` だけが付ける）のとき `MSL.json` と `c/` を作らない。Worker（`worker.js`）は、索引に `pack` があれば、`fetchChunks` の塊の取得を「パックを `fetch().blob()` で 1 回だけ取り、塊を `blob.slice(a, b).text()` で切り出す」に替える（`packText`。取れなければ誤りで、取れるまで次の呼び出しで取り直す。Node は `fs.openAsBlob`）。**Blob は Worker の JS ヒープに載せない**（ArrayBuffer を握ると 15.2 MiB の外部メモリ。ブラウザの Blob の置き場は Chromium の blob 管理で、Worker のヒープの外）。閉包の繰り返し・先読み・塊の壊れの検査は変えない（塊の本文の取り方だけが違う）。ブラウザ版（`web.yml` の束）は `GM_MSL_PACK` を付けないので、`MSL.json`・`c/*.json` のまま。
- **案 B（外した）**: 塊の cap を上げる。`pack()` の cap は引数にあり、`split.mjs --cap` で測れる（測るための口として残した）。cap 64 KiB で塊 514 本、256 KiB で 461 本、それ以上は **446 本で頭打ち**（ディレクトリが変わるたびに塊を分けるため）。
- **比べた表**（同じ 20 本、Node、release の wasm。全部の設定で全部読みと 20 / 20 一致。APK は CI の arm64 release、x86_64・debug は解説書）:

  | | MSL.json（S301 の前） | S308（塊 838 本） | **案 A（pack.bin）** | 案 B（cap 256 KiB・先読み 0） |
  | --- | ---: | ---: | ---: | ---: |
  | APK arm64 release | 13,616,490 B | 14,859,242 B（+9.1%） | **13,847,114 B（+1.7%）** | 14,432,522 B（+6.0%） |
  | 見積もり（brotli q9 の合計） | 2.08 MB | 3.25 MB | 2.28 MB（パック 2.08 + 索引 0.20） | 2.84 MB（塊 2.65 + 索引 0.19） |
  | `dist/msl` の asset 数 | 1 | 839 | **2** | 462 |
  | 実行時メモリ: 読み込み直後の線形メモリ | 62.1 MiB | 13.7 MiB | **13.7 MiB** | 13.7 MiB |
  | ChuaCircuit の後の線形メモリ | 95.9 MiB | 80.6 MiB | **80.6 MiB** | 81.1 MiB |
  | 20 本の後の線形メモリ（最大） | — | 289.3 MiB | **289.3 MiB** | 294.6 MiB |
  | 20 本の後の JS ヒープ（Worker。wasm 除く） | — | 41.8 MiB | 43.2 MiB（パックは Blob でヒープ外） | 44.0 MiB |
  | ChuaCircuit の往復・塊・読んだ文字 | — | 3 往復・179 塊・5.16 MiB | 3 往復・179 塊・5.16 MiB（取得は 1 回 + 切り出し） | 8 往復・95 塊・5.29 MiB |
  | 20 本の往復・塊・読んだ文字 | — | 17 往復・258 塊・9.4 MiB | 17 往復・258 塊・9.4 MiB | 25 往復・133 塊・10.2 MiB |
  | エミュレータの smoke（`msl=ok` を含む全項目） | ok | ok | ok | ok |
  | smoke の `msl_ms`（release / debug、共有ランナーの目安） | — | 10.6 / 11.2 s | 11.3 / 8.2 s | 13.1 / 7.7 s |

  案 B で先読みを 2 にした場合（cap 256 KiB）は、ChuaCircuit が 3 往復・105 塊・5.79 MiB、20 本が 9 往復・160 塊・11.0 MiB・線形メモリ最大 341.2 MiB（塊が大きく、deps が広がって取りすぎる）。cap 64 KiB（514 本）・先読み 2 は、ChuaCircuit が 3 往復・126 塊・5.66 MiB、20 本が 11 往復・193 塊・10.5 MiB・線形メモリ最大 296.7 MiB（往復は保てるが、A より取りすぎ、塊の数は 838 → 514 までしか減らない）。
- **選んだ理由**: (1) **APK の大きさ**: A は +1.7%（索引の約 0.2 MB だけが `MSL.json` に足される）。B は cap をいくら上げても塊がディレクトリごとに 446 本残り、塊ごとの圧縮の損が 0.5〜0.6 MB 残るので、最良（見積もり）でも +5% 余り（cap 256 KiB の実測は +6.0%）。(2) **実行時のメモリ**: A は読む塊が S308 と同じなので、wasm の線形メモリ（13.7 / 80.6 / 289.3 MiB）も往復も変わらない。B は取る塊が大きく、往復を保つには先読みが要り、取りすぎて線形メモリが増える（先読み 2 で 20 本の最大 341 MiB）。(3) **変更の小ささ**: A は `pack.js` に `packFile`（6 行）、`worker.js` に `packText`・`getBlob`（約 10 行）。B は pack の引数だけで済むが、APK を戻しきれない。
- **A の費用**: パック（15.2 MiB）を全部読む。Android では APK 内のローカルの資源なので回線は使わない（smoke の `msl_ms` は S308 と同程度で、共有ランナーのばらつきの内）。ブラウザで Blob を握る間、ブラウザプロセスのメモリ（blob の置き場。Chromium は大きいと一時ファイルに逃がす【未確認】）を使うが、Worker のヒープと wasm の線形メモリには載らない。Worker を作り直すと（線形メモリ 2 GiB 超など）パックをもう一度読む。
- **確かめ方**: `node globe-mode-wasm/node/split.mjs --msl data/msl --pack`（20 本が全部読みと一致し、ヒープと線形メモリを出す。`--cap N` で B を測れる）、MSL 不要の `broken-chunks.mjs`（`mise run wasm:node`。パックが無ければ誤りで返り、置けば通る）。実機に近い確認は `android.yml`（emulator の smoke）。ブラウザ版（`web.yml`）は変えていない。
- **限界**: (1) 実機の Android（WebView）の Blob のメモリの置き場・速度は、エミュレータの smoke でしか見ていない。(2) 往復の時間・JS ヒープは Node の `worker.getHeapStatistics`（ArrayBuffer は wasm の線形メモリも含むので引いている）。ブラウザ・WebView のメモリの測定ではない。(3) APK は MSL.json 時代より +230,624 B（+1.7%）。索引（`stubs` は木の見出しに要る）を縮めれば戻せるが、していない。


### S368 で決まったこと（UI の参照・リネームが開いているタブの全部に届く、2026-11-16）

S230・S235 で LSP（`textDocument/references`・`rename`）は開いていないファイル・ライブラリ・継承した部品まで届いたが、UI は開いている 1 文書の中だけだった（S366 で確かめた。S230 は UI の `globe/*` を広げなかった）。UI（ブラウザ・Desktop・Android）が開いているタブの全部を見るようにした。

- **口の形を変えた（互換を切る）**: S230 が広げなかった理由の 1 つは「配列の応答が複数ファイルを表せない」ことだった。`globe/references`（と wasm の `references`）は `[{uri, range}]`（LSP の Location）、`globe/rename`（と wasm の `rename`）は `{changes: {uri: [TextEdit]}}`（WorkspaceEdit）に変えた。`textDocument/references`・`rename` と同じ形で、`uri` は UI が `setFile`（`didOpen`）に渡した名前（タブの key）。利用者は UI の `Backend` だけなので旧形は残さない。新しい名前の口は作らなかった。
- **規則は新しく作らない**: `definition::references_in`・`rename_in`（`globe-mode-lsp`）が、`textDocument/*` と同じ部品（`component_name_at`・`component_hits`・`check_component_name`・`class_name_at`・`class_hits`・`rename`）を文書の組に回す。クラス名は同じクラスに行き着くほかの文書の識別子（S166）、部品は継承した先と `b.x`（S235）、import の別名はその文書の中。クラスのリネームは定義した文書の中から（別のファイルで定義したクラスは使う側から断る。S166 のまま）。宣言したファイルが組の外（ライブラリ）の部品は断る。wasm は `Workspace::text`（facade に足した小さな口）で組を作り、sidecar は `didOpen` 済みの文書（`texts`）で作る。
- **ディスクは見ない（S230 の「UI はメモリ上の文書」を保つ）**: sidecar の根（`initialize` の `rootPath`。Desktop は作業フォルダ）の下の開いていない `.mo` は、UI のタブの本文と食い違い、UI には適用先のタブが無いので、`globe/*` は読まない。**ライブラリ（`-L`・wasm の読み込んだ木）のクラスの参照は一覧に足す**（`library: true`。見るだけ。`disk_class_hits`・`definition::library_references`）。リネームはライブラリのファイル・開いていないファイルを変えない。
- **離れたタブの本文は呼ぶ間だけ Workspace に入れる**: UI は S141 以来、タブを離れるとそのファイルを Workspace から外す（同じクラスの二重定義を避ける）。参照・リネームの間だけほかの自分のタブを `setFile` で入れて、終わったら（断られても）空に戻す（`src/Svelte/src/lib/cross-file.ts`。Svelte に業務ロジックは置かず、編集の計算は Rust）。リネームは今のタブを 1 回の dispatch（undo 1 回）で、ほかの自分のタブは本文に当てる（そのタブの undo で戻る。1 回の Ctrl+Z で全タブは戻らない）。ライブラリのタブは変えず、変えなかった箇所の数を知らせる。
- **限界**: (1) 同じクラスを 2 つのタブが定義していると（二重定義）、`Resolver` はどちらかに寄せるので参照が片方に偏る。(2) 参照・リネームの間は Workspace を書き換えるので、実行・検査の最中は入らない（`busy` なら何もしない）。(3) 1 回の Ctrl+Z で全タブは戻らない。(4) ライブラリの参照は、名前を含むファイルを全部字句に分けるので、短い名前（`R` など）の上の初回は遅い（LSP の S230 と同じ。wasm は読み込んだ木の本文を走る）。(5) 部品の参照は開いているタブの中だけ（ライブラリのファイルの継承の鎖の先は一覧に出さない。LSP の `textDocument/references` は出す）。

### S372 で決まったこと（8 回目のレビューの #141〜#143。UI の参照・リネームの端、2026-11-16）

S368 の `references_in`・`rename_in` を 8 回目のレビューが 3 つの端で破った。再現は debug の `globe lsp`（Desktop の sidecar と同じ道）で確かめ、wasm も同じ構造（`Workspace::docs`）なので同じ直しを入れた。

- **#141 ライブラリのタブ（自分の文書でない）**: UI はライブラリのタブ（読み取り専用）を開くたびに `setFile` し、離れても Workspace から外さない（外すのは自分のタブだけ。S141）。S368 の「ライブラリで宣言した部品は断る」は「宣言したファイルが `docs` の中にあるか」で決めていたので、そのタブを一度開くと断られず、自分のタブだけが書き換わって宣言と食い違った。`definition::Doc` に `own`（編集してよい文書）を足した。wasm は読み込んだ木のファイル名（`library_source` の `file`）を、LSP は `-L` の下で根の下でないファイル（`component_rename_blocked` と同じ判定を `in_library` に切り出した）を `own: false` にする。`rename_in` は、宣言した文書が `own` でない部品を断り、`own` でない文書から始めるリネームも断り、`changes` に `own` でない文書を入れない。参照の一覧には出る。UI の `skipped`（ライブラリの編集の数）は残した（別のクライアントでも守りが Rust 側にある）。
- **#142 同じ名前のクラスを複数のタブが定義するとき**: `Workspace` は 1 つの名前空間で、`Resolver::locate` はファイル名順の先の定義に寄る。コピーして試す 2 つのタブが `Base` を定義すると、後ろのタブの `Base` への参照が先のタブのものとして数えられ、リネームが後ろのタブを壊した。`at_home`（`definition.rs`）が、文書を引く間だけ `Resolver::overlay`（S171。文書自身のアウトラインを先に引く）にその文書を立て、各文書の名前はまず自分の定義に解決する。参照・リネームのカーソル側と、文書ごとの `class_hits`・`component_hits` に掛ける。**区別できない場合の断りは足さなかった**（shortcut）: 定義しない 3 つ目の文書が `Base` を使うときは、定義ジャンプと同じくファイル名順の先に寄る。
- **#143 読めないタブ**: 構文の誤りで読めない自分のタブは outline が無く、`class_hits` は 0 件を返し、リネームはそのタブを黙って飛ばして成功した。S230 のディスクの守り（`disk_class_hits` の解決できなかった候補）に合わせ、読めない自分のタブ（`own`）に識別子 `name`（字句に分けられなければ部分文字列）があれば、そのタブのファイル名を挙げて断る（`-32803`。UI は `failure` に出す）。部品も同じ。`textDocument/rename` が断る「同名の部品を持つだけのファイル」と同じ shortcut（読めないタブに同じ名前の別の部品があっても断る）。**参照の一覧は変えない**（読めないタブの行は出ない）。
- **限界**: (1) 上の「区別できない場合」。(2) 参照の一覧が読めないタブを黙って飛ばす。(3) `at_home` は `Workspace` に入っている文書にだけ掛かる（読めない文書の寛容な名前解決 `tolerant_definition` は、今までどおり）。

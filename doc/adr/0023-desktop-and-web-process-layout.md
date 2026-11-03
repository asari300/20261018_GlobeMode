# ADR-0023 Desktop と Web のプロセス構成: 検査と実行は UI のプロセスから分け、Desktop は `globe` を sidecar に、Web は Web Worker に置く

- 状態: Accepted（2026-10-31、S040）
- 関係: [ADR-0014](0014-session-route-wasm.md)（wasm は session の道、Node で確かめる）、[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)（`msl-sim` のワーカーと時間切れ）、[ADR-0019](0019-rpi-aarch64-environment.md)（検査の RSS と swap）。[architecture.md §4](../architecture.md#4-ターゲットごとの実行方法) の「Tauri のプロセス内でネイティブ実行」をこの ADR で置き換える。計画書は [plan/2026-10-30-release-estimate-and-parallel-plan.md](../plan/2026-10-30-release-estimate-and-parallel-plan.md) の WP-J

## 背景

- P4（Desktop MVP）の Tauri のシェル（計画書の WP-C）を作る前に、検査（S0〜S10）と実行をどのプロセスで回すかを決めておく必要がある。[architecture.md §4](../architecture.md#4-ターゲットごとの実行方法) は「Tauri のプロセス内でネイティブ実行」と書いていたが、S005 以降の測定で次が分かった。
- **メモリ**: 検査は 1 モデルでも GiB の単位になる。`msl:funnel`（1 プロセスで 566 本、50 本ごとに Session を戻す）は x86_64 で最大の常駐メモリ（VmHWM）7.1〜7.2 GiB（[ADR-0019](0019-rpi-aarch64-environment.md)、[roadmap.md](../roadmap.md) の持ち越し 9）、RPi で 6.5 GiB、最新の [msl-funnel.md](../msl-funnel.md) で 2.0 GiB。`msl:sim` のワーカー（1 本ごとに別プロセス）は最大 3.4 GiB（[msl-sim.md](../msl-sim.md)）。wasm は MSL を読んだ後の線形メモリ 262 MiB、`ChuaCircuit` の検査と実行の後 513 MiB（CSV を返すと 634 MiB）、Node の RSS 886 MiB（[ADR-0014 の結果](0014-session-route-wasm.md)）。`Checker` のメモリの上限は「P4 のプロセス構成が決まってから」として持ち越していた。
- **止まり方**: Rumoca の lowering・組み立てが終わらないモデルがある（`DrumBoiler`、[ADR-0018](0018-no-preflight-limit.md)）。`msl-sim` はワーカーを別プロセスにして、時間切れ（180 s、lowering は 150 s）で**外から止めて回し直す**（[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)）。同じプロセスの中では、終わらない計算を安全に止められない。
- **panic**: wasm は panic=abort で、S036 は panic の後に Worker を捨てて作り直し、ライブラリとファイルを再生する道を作った（[roadmap.md](../roadmap.md) の S036）。ネイティブでも Rumoca の内部の `unwrap` などで panic する道は残っている（`msl:funnel` の panic は 0 本だが、未知の入力に対しては保証できない）。
- **入口はもうそろっている**: S035 の facade `globe-mode` と `globe-mode-protocol`（`CheckResponse`・`SimulateRequest`・`SimulateResponse` の JSON）、S038 の `globe lsp`（stdio の JSON-RPC、`Content-Length` の枠、1 スレッドで 1 要求ずつ）、S036 の `GlobeWorker`（呼び出しを直列にし、panic の後に作り直す）。Desktop の UI が同じ JSON を同じ枠で別プロセスへ送れば、新しい計算の道は要らない。
- 測定（S040、x86_64 の WSL2、RAM 3 GB + swap 16 GB、release ビルド。手順は [sessions/S040-adr-process-layout/解説書.md](../sessions/S040-adr-process-layout/解説書.md)）:

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
- **独自の要求の名前と形**（`globe lsp` の同じ接続。実装は `globe-mode-lsp`）:

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

- **Android の APK は MSL.json を同梱する**（S092 の持ち越しの「Android での MSL の置き場」）。比べた 2 つの道:
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

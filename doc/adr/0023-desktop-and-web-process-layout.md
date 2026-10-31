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

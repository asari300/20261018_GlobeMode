# リソースの計測（S051）: wasm の線形メモリ、列ごとの結果、`globe` の RSS

- 確認日: 2026-10-31
- 計測の道具: [.github/workflows/resources.yml](../../.github/workflows/resources.yml)（手動）、[node/memory.mjs](../../src/Rust/mod/globe-mode-wasm/node/memory.mjs)、[node/columns.mjs](../../src/Rust/mod/globe-mode-wasm/node/columns.mjs)
- 環境:
  - 「ランナー」: GitHub Actions の ubuntu-24.04（実測は `free -m` で 15,982 MiB、`nproc` 4、swap 3 GiB）。run 41196216961（feature ブランチの push で起動）。
  - 「手元」: WSL2（x86_64、`free` で 7.9 GiB と出る。CLAUDE.md の言う RAM 3 GB 制限の機械）。他のセッションが同じ機械でビルドしているので、時間は参考値。
- 推測は「推測」と書いた。確認できなかったものは【未確認】と書いた。

## 1. 結論

| 問い | 答え |
| --- | --- |
| wasm の線形メモリは ADR-0023 (c) の上限 2 GiB に達するか | **達する。** 566 本を順に `check` すると、336 本目の `Spice3BenchmarkFourBitBinaryAdder`（1 本の `check` が 259〜289 s）が **1 回で 668 MiB → 3,802 MiB（3.7 GiB）** に伸ばし、wasm32 の上限 4 GiB の 294 MiB 手前まで来る。この 1 本を除くと 566 本で 868 MiB（1 本あたり約 1.4 MiB の伸び）。2 GiB を超えたあとの作り直しは 1 回（609 ms）で足りるが、1 回の `check` の中の伸びは止められない |
| `globe check` / `globe simulate` の最大 RSS | 1.5〜1.9 GiB（3 モデル。下の表）。ADR-0023 の `ChuaCircuit` 1.48 GiB と同じ |
| 列ごとの結果は CSV よりどれだけ速いか | 10 万点 × 7 列で wasm の呼び出しから JS の配列まで 452 ms → 137 ms（手元。3.3 倍）、ランナーで 548 → 159 ms（3.4 倍）。Worker 越しは手元で 399 → 114 ms |
| RSS の大半は何か | 推測: MSL 全体を読んで持つこと（閉包のファイルだけを渡すと 461 MiB に落ちる。実測） |

## 2. 数字

### 2.1 `globe` の最大 RSS（ランナー、release、`/usr/bin/time -v`）

`globe check --library data/msl -m <モデル>` と `globe simulate --library data/msl -m <モデル> --timeout 300 -o /dev/null`。`data/msl` は MSL 4.1.0（2,556 ファイル、約 15.3 MB）。閉包は `globe deps data/msl -m <モデル>` の本数。

| モデル | 閉包 | check の最大 RSS | check の壁時計 | simulate の最大 RSS | simulate の壁時計（終了コード） |
| --- | ---: | ---: | ---: | ---: | ---: |
| `Modelica.Electrical.Analog.Examples.ChuaCircuit` | 1,076 | 1,493 MiB | 8.1 s | 1,533 MiB | 3.9 s（0） |
| `Modelica.Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 1,103 | 1,653 MiB | 6.4 s | 1,929 MiB | 59.0 s（1） |
| `Modelica.Fluid.Examples.HeatingSystem` | 1,072 | 1,618 MiB | 10.1 s | 1,589 MiB | 13.4 s（1） |

- `FullRobot` と `HeatingSystem` の simulate の終了コード 1 は、実行系が止まったため。`HeatingSystem` は手元で同じ出力を確かめた: `GM-U0004`（Rumoca の構造解析 EL005）。`FullRobot` はランナーのログに停止の理由が残っていない（ワークフローは標準エラーの末尾を出す設計だったが、ログに出なかった）。手元でも確かめていない。どちらも止まるまでの RSS なので、実行が最後まで進んだ場合の値ではない。
- 手元（release、2026-10-30 のビルド）の `ChuaCircuit` の `check` は 1,499 MiB（6.3 s）。ランナーの 1,493 MiB と同じ。
- 3 つの check の差（1,493〜1,653 MiB）は小さい。モデルの大きさでなく、ライブラリを読む固定の費用が大半である。

手元の追加の実験（`ChuaCircuit` の `check`、同じ release の `globe`）:

| 条件 | 最大 RSS | 壁時計 |
| --- | ---: | ---: |
| `--library data/msl`（MSL の 2,556 ファイル、15.3 MB を全部読む） | 1,499 MiB | 6.3 s |
| 同じ、`RAYON_NUM_THREADS=1` | 1,501 MiB | 6.4 s |
| 同じ、`MALLOC_ARENA_MAX=1` | 1,518 MiB | 7.9 s |
| 閉包の 1,075 ファイル（8.1 MB）だけをファイルとして渡す（`--library` なし） | **461 MiB** | 10.9 s |

- スレッド数と malloc の arena の数は RSS に効かない（どちらも ±1%）。並列の作業領域は主因ではない。
- ファイルを 15.3 MB から 8.1 MB（閉包）に絞ると RSS は 1/3 になる。ただし後者は `--library` と別の道（文書として渡す）で、壁時計は遅い（10.9 s）。読む量だけでなく道も違うので、1/3 になる分がすべて「読む量」の効果とは言えない。

### 2.2 wasm の線形メモリ（Node の Worker、MSL は分割読み込み）

`memory.mjs`: MSL を 1 回読み（S037 の分割読み込み）、`msl-funnel --list` の 566 本を順に `check` し、1 本ごとに `memory_bytes()` を記録する。線形メモリは縮まないので、値は単調に増える。

| 条件 | 本数 | 読んだ直後 | 最大 | 2 GiB 到達 | 作り直し | 失敗 |
| --- | ---: | ---: | ---: | --- | ---: | ---: |
| 手元、先頭 30 本（`Blocks.Examples`） | 30 | 60.9 MiB | 127.8 MiB | なし | 0 | 0 |
| 手元、重い 3 本（`ChuaCircuit`、`FullRobot`、`HeatingSystem`） | 3 | 60.9 MiB | 315.6 MiB | なし | 0 | 0 |
| ランナー、566 本全部、2 GiB で作り直し | 566 | 60.9 MiB | 3,801.9 MiB | **336 本目** | 1（609 ms） | 0 |
| ランナー、566 本全部、作り直しなし | 566 | 60.9 MiB | 3,801.9 MiB | **336 本目** | 0 | 0（落ちない） |

伸びの様子（作り直しなしの線形メモリ、MiB）: 10 本目 91、50 本目 159、200 本目 307、300 本目 520、**336 本目 3,802**、以後 566 本目まで 3,802（縮まない）。作り直しありは 336 本目のあと新しい Worker で 339 本目 537、400 本目 570、500 本目 793、566 本目 868。検査が通ったのは 369 本で、funnel の OK 369 と同じ。全体の時間は 3,398 s（作り直しあり）と 3,810 s（なし）。

- 336 本目より前は 300 本で 520 MiB と、ADR-0023 の目標（検査と実行のあと 1 GiB 以下）の内側である。この 1 本だけが外れる。`Spice3BenchmarkFourBitBinaryAdder` はネイティブでも重い例題で、ADR-0023 の背景の `msl:funnel` の 7.2 GiB の主因の候補（推測。ネイティブの RSS は測っていない）。
- 作り直しの判定は `check` の後に見る（`memory.mjs` も UI の想定も同じ）ので、336 本目の伸びそのものは防げない。4 GiB を超えた場合の挙動は、この run では起きなかったので未確認である【未確認】。

- 重い 3 本の check の後の線形メモリ: `ChuaCircuit` 82 MiB → `FullRobot` 205 MiB → `HeatingSystem` 331 MiB（累積。`HeatingSystem` の check は 16.1 s）。ネイティブの RSS（1.5〜1.7 GiB）の 1/5 以下である。
- 検査だけでなく実行を含めた手元の値（`smoke.mjs --msl`）: 読み込み後 62 MiB、`ChuaCircuit` の bdf の後 113 MiB。S037 の値と同じ。

### 2.3 列ごとの結果（`simulate_columns`）

`columns.mjs`: 同じ wasm の上で、CSV の道（`simulate` → `JSON.parse` → `src/Svelte/src/lib/plot/csv.ts` と同じ `parseCsv`）と、列ごとの道（`simulate_columns` → `layout()` の `JSON.parse` → `column(i)`）を交互に回した中央値。どちらも 100,001 点。値は全点で一致する（CSV の非有限は `null`、列は `NaN`/`Inf`）。

| モデル | CSV の道（wasm + JSON.parse / parseCsv / 合計） | 列ごとの道（wasm / 受け取り / 合計） | 比 |
| --- | --- | --- | ---: |
| `Decay`（`Interval = 1e-5`、2 列） | 93 / 63 / 151 ms | 61 / 0 / 62 ms | 2.4 倍 |
| `Wide`（4 状態 + 代数 2 本、7 列） | 288 / 164 / 452 ms | 136 / 1 / 137 ms | 3.3 倍 |

Web Worker（`worker_threads`、`postMessage`。列の `buffer` は転送）越しの往復:

| モデル | CSV の道 | 列ごとの道 |
| --- | ---: | ---: |
| `Decay` | 144 ms | 60 ms |
| `Wide` | 399 ms | 114 ms |

ランナーの値（`columns.mjs --runs 9` の中央値）: `Decay` は CSV の道 199 ms → 列ごと 83 ms（2.4 倍）、`Wide` は 548 ms → 159 ms（3.4 倍）。Worker 越しは `Decay` 193 → 84 ms、`Wide` 556 → 158 ms。手元と同じ比である。

- S042 は「CSV の解析が 10 万点で約 0.3 s」と書いた（4 状態のモデル、ブラウザ）。Node の `parseCsv` は 7 列で 164 ms で、同じ桁である。
- 列ごとの道で wasm の線形メモリに残るのは軌跡（`Vec<f64>`）だけで、CSV の文字列を作らない。`Wide` の `simulate` 1 回後の線形メモリ（Node、新しいプロセス）: CSV あり 40 MiB（CSV の文字列は 13.7 MiB）、`no_csv` 9 MiB、`simulate_columns` 9 MiB。つまり CSV は文字列そのものの約 2 倍の線形メモリを、実行の間だけ余分に使う（書き出しのバッファが倍々に伸びる。推測）。
- 形: `Workspace.simulate_columns(request_json)` は `SimColumns` を返す。`response`（`csv` を除いた `SimulateResponse` の JSON）、`layout()`（`ColumnsLayout { names, points }` の JSON。`names[0]` は `time`）、`column(i)`（`Float64Array`）。`column(i)` は wasm-bindgen の生成物で `getArrayF64FromWasm0(...).slice()`（JS 側に 1 回コピー）になり、新しい `ArrayBuffer` なので Worker から転送できる（`web/worker.js` が `postMessage` の転送リストに入れる）。wasm の中でも 1 列を `clone` するので、一時的に 1 列分（10 万点で 0.8 MB）が余る。ゼロコピーの `Float64Array::view` は線形メモリが伸びると無効になるため使わない。

## 3. メモリの使われ方の見立て

事実と推測を分ける。

事実（実測）:

- ネイティブの `check` の RSS 1.5 GiB に対し、wasm の線形メモリは同じ `ChuaCircuit` で 82 MiB（分割読み込み。実行を含めると 113 MiB）。差は約 18 倍。
- ネイティブで MSL の全ファイル（15.3 MB）を読むと 1.5 GiB、閉包の 1,075 ファイル（8.1 MB）だけを渡すと 461 MiB。
- スレッド数と malloc の arena の数を変えても RSS は変わらない。
- 3 つのモデルの check の RSS の差は 1,493〜1,653 MiB。モデル固有の分は小さく、固定の費用が大きい。

推測（未検証）:

- **Rumoca の `Session`**: MSL の 2,556 ファイルを構文木にして持つ。ソース 15.3 MB に対し約 1 GiB（約 65 倍）は、構文木のノードと文字列の複製、`Session` の中の索引による。ADR-0023 の背景にある `msl:funnel` の 7.2 GiB（566 本、50 本ごとに `Session` を戻す）も、同じ構造が積み上がったものと読める。
- **salsa のキャッシュ**: 検査のたびに増える。wasm の 566 本の伸び（上の表）が、1 本あたりの増分の目安になる。`Checker` の内側に上限が無い（ADR-0023 (b)）ので、長く開いた UI では伸び続ける。
- **CSV**: 上で測った通り、wasm では結果の約 2 倍の線形メモリを一時的に使い、JS の側にも同じ大きさの文字列と、`parseCsv` の `(number|null)[]`（1 要素 8 バイトより大きい。V8 の表現による。推測）が残る。ネイティブの RSS には、結果が大きいモデルでだけ効く。
- wasm とネイティブの差（約 18 倍）の原因は特定していない。分割読み込み、`dlmalloc` と glibc の違い、ネイティブだけが持つ並列の作業領域のうち、どれがどれだけかは測っていない。上のスレッド数の実験では並列の作業領域は主因でなかった。

## 4. 削減の案（3 つ）

| # | 案 | 費用 | 効果（見込み） | 根拠・注意 |
| --- | --- | --- | --- | --- |
| 1 | `globe`（CLI・LSP）の `--library` にも分割読み込み（S037）を使う。facade に `with_library_sources_split` があるので、ファイルを読む `with_libraries` に同じ分割を足す | 小〜中（facade と CLI の数十行。S043 の `split-all` が同じ検査結果を保証する道具として使える） | `check` の RSS 1.5 GiB → 0.5 GiB 前後（推測。閉包だけを渡した実験が 461 MiB）。Desktop の sidecar（ADR-0023 (b)）の常駐が 1/3 | 実験の 461 MiB は `--library` と別の道。実装して測るまで確かでない |
| 2 | UI の `Plot` に `simulateColumns` を接続する（`Backend.simulate` が CSV でなく列を受ける）。次回のセッション | 中（Svelte と `Backend` の型。`uPlot` は `Float64Array` を受けられるが、`null` でなく `NaN` が来る点の確認が要る【未確認】） | 10 万点 × 7 列で 400 ms 台 → 約 110 ms（Worker 越し）。S042 の「30 fps に届かない」の初回の待ちが縮む | 描画の fps は別の問題（間引き）。ここで測ったのは取り込みまで |
| 3 | wasm の線形メモリが大きくなったら UI が `memory()` を見て Worker を作り直す（ADR-0023 (c) の通り）。作り直しは `GlobeWorker` の既存の仕組みで足りる（ランナーで 609 ms）。ただし **336 本目のように 1 回の検査で 3 GiB 伸びるモデルがある**ので、「検査の後に見て作り直す」だけでは 4 GiB を超えうる。検査の前に閉包の大きさなどで予告する、wasm の側で割り当てに上限を付けて診断にする、が要る（どちらも未設計）。2 GiB を下げるか（例 512 MiB）は、スマートフォンのブラウザで測ってから決める | 作り直しは小。予告か上限は中〜大（Rumoca の割り当てを wasm の側から止められるかが【未確認】） | 通しで 2 GiB に達する（336 本目）のを、ブラウザが落ちる前に捕まえる。作り直すとキャッシュが消えて次の検査が遅くなる | スマートフォンの上限は【未確認】 |

## 5. 出典と確認日

- ADR-0023 (c) の上限 2 GiB と目標 128 MiB / 1 GiB: [adr/0023-desktop-and-web-process-layout.md](../adr/0023-desktop-and-web-process-layout.md)（確認日 2026-10-31）。
- 分割読み込みの値（62 MiB / 113 MiB）: [sessions/S037-msl-split-load/解説書.md](../sessions/S037-msl-split-load/解説書.md)。
- CSV の解析が 10 万点で約 0.3 s: [sessions/S042-plot/解説書.md](../sessions/S042-plot/解説書.md)。
- `msl:funnel` の最大 RSS 7.2 GiB: [msl-funnel.md](../msl-funnel.md)。
- `column(i)` の生成コード（`.slice()`）: `wasm-bindgen --target web` の出力 `globe_mode_wasm.js`（wasm-bindgen 0.2.129、2026-10-31 に手元で確認）。
- GitHub Actions のランナーの仕様（4 vCPU、16 GB）は、`free -m` と `nproc` の実測（run 41196216961 の `rss-env.txt`）。公式文書の URL は確認していない【未確認】。

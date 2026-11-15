# ADR-0040 複数のソルバで 1 つのモデルを回して比べる: 同じコンパイル結果を共有するスレッドと、矩形の管での比較

- 状態: Accepted（2026-11-15、S322）
- 関係: [ADR-0013](0013-exec-backend.md)（実行系）、[ADR-0023](0023-desktop-and-web-process-layout.md)（プロセス構成。ワーカーは 1 つ）、[ADR-0033](0033-progress-and-cancel.md)（進捗と中断）、[ADR-0015](0015-auto-and-reference-tolerance.md)（許容誤差）。UI は次の S323。

## 背景

ユーザーの要望（2026-11-15）: 実行のときに複数のソルバを選んで結果を比べる手段が欲しい。(1) 3 つ以上を選べる（既定は 1 つ、複数のときはチェックボックス）、(2) 1 つずつ回すか並列かを選べる（既定は 1 つずつ）、(3) 全部が終わった後に自動で比べるオプション（既定は比べない）で、最大積分次数・許容値・比較の閾値（10^-n）を指定する。参考は OpenModelica の OMEdit の「シミュレーションのセットアップ」（手法 dassl、許容値 1e-06、最大積分次数 5、初期・最大ステップサイズ）。

調べたこと（コードで確認。2026-11-15）:

| ソルバ | 最大次数 | 根拠 |
| --- | --- | --- |
| `bdf`・`auto`（diffsol の BDF） | **設定する口が無い** | diffsol 0.13.2 の `BdfState::MAX_ORDER` は定数 5（`src/ode_solver/bdf_state.rs`）。Rumoca の `rumoca-solver-diffsol`（rev 276e82b）にも次数の設定は無い |
| `cvode`・`ida`（SUNDIALS の BDF。feature `sundials`） | **1〜5 を設定できる** | `CVodeSetMaxOrd`・`IDASetMaxOrd`（`sundials-sys` 0.6.2 の同梱ソースの `cvode.h`・`ida.h`）。上限は同じソースの `cvode_impl.h` の `BDF_Q_MAX 5`、`ida_impl.h` の `MAXORD_DEFAULT 5` |
| `esdirk*`・`trbdf2`・`sdirk34hw`・`tsit45`・`dopri45`・`dopri853`・`cerk*`・`radau5`・`qss`・固定刻みの 5 つ | 次数は手法で決まっていて、設定する意味が無い | 係数表で次数が固定（`dopri853` は 8、`radau5` は 5 など） |

許容値（`tolerance`）は既存の `SimSettings.tolerance`（相対と絶対に同じ値。下限 1e-10、固定刻みは使わない。GM-I10004）で、そのまま全部の実行に渡せる。

## 決定

1. **`SimSettings.max_order: Option<u32>` を足す**（`deny_unknown_fields` のまま。Python の `Settings` と `_native.settings_sample` も合わせた）。次数を持つソルバ（`SolverId::has_max_order` ＝ `cvode`・`ida`）では 1〜5 を渡し（外なら GM-E10004）、持たないソルバでは**無視して情報 `GM-I10005`**（「指定した最大次数（3）はソルバ bdf では使わない」）を出す。黙って無視しない。`ResolvedSettings.max_order` に写す。
2. **`Workspace::simulate_many(&SimulateManyRequest, now, ManyHandle) -> ManyResult`**（facade）。依頼は `{files?, model, settings, solvers[], parallel?, compare?{threshold_exp, reference?}, force?}`、応答 `SimulateManyResponse` は `{ok, refused, diagnostics（検査・コンパイルの分）, mode, workers, mode_note?, runs[], comparison?, elapsed_ms}` で、`runs[i]` は `{solver, status: ok|failed|cancelled, response（その実行の SimulateResponse。csv 無し）, elapsed_ms, notes[]}`。時系列は応答に含めず `ManyResult.runs[i]`（列）に持つ。検査とコンパイルは**1 回だけ**行い、実行ごとに `run_observed` を呼ぶ。`settings.solver` は使わない（誤り）。
3. **並列は、同じコンパイル結果（`CompiledModel`）を共有するスレッドで、最大 `min(CPU 数, 4, ソルバ数)` 本**（`MAX_PARALLEL` = 4）。要望は別プロセスのワーカーを想定していたが、スレッドにした。理由: (a) ADR-0023 は検査と実行の RSS が 1 本で GiB の単位なので同時のプロセスを増やさないと決めた。コンパイルを共有すれば、増えるのは実行ごとの組み立てだけ。(b) facade は wasm・Python・C・NIF からも呼ばれ、`globe` の実行ファイルの場所を知らない。(c) 中断は `CancelToken` が全部の実行に効く（既存の道。ネイティブの型付きの道は別のスレッドで回し、トークンが立てば結果を捨てて返す）。**代わりに、硬い実行を外から確実に止めること（kill）はできない**（実行ごとの `max_wall_seconds`、既定 180 s〔LSP〕は協調型）。Desktop は sidecar の kill の予算を「ソルバの数 × 180 s」にした。逐次は 1 つずつ回す（スレッドを作らない）。**wasm はスレッドが無いので `parallel` は逐次に落とし、`mode_note` に書く**。進捗は、並列のときも呼ぶスレッドを 1 つにして（チャネルで集めて）呼ぶ。
4. **比較は `globe-mode-sim::compare` の矩形の管（csv-compare の既定）をそのまま使う**。基準（`reference`、無ければ最初に成功した実行）の信号を折れ線とみなし、幅 `2·δt`・高さ `2·δy`（`δ = 10^-n × 範囲`、時間と値の両方向）の矩形が掃く領域を管とし、ほかの成功した実行の点が管の外に出たら不合格。要望の「共通の時刻に線形補間して `max |a−b| / max(|b|, atol)` を閾値と比べる」案は採らなかった。理由: (a) 同じ数え方を 2 か所に書かない（MSL の参照結果との比較と同じ判定）。(b) 出力の格子が違う実行（固定刻み、イベントでの 2 行）を補間なしに比べられ、イベントの縦の跳びや時刻のずれに強い。(c) 補間と相対差では、ゼロ交差の近くで分母が小さくなり、閾値が安定しない。**欠点**: 変数ごとの「最大の相対差・絶対差」は出ず、出るのは管の外の点の数・最大のはみ出し量（値の単位）・最初の時刻と、信号ごとの合否。閾値 `10^-n`（1〜15）は管の幅の比率で、相対差そのものではない。比べる信号は基準の `Parameter` と内部名（`$`）を除いた全部。全体の合否 `comparison.passed` は、比べた実行がすべて合格（相手が 1 つも無ければ false）。基準が失敗していたら全部を `skipped`。
5. **選べる数は 1〜16**（`MAX_MANY_SOLVERS`。登録表は 22 個あるが、列を JSON で返す大きさ〔1 実行 = 全列〕が 16 を超えるとメッセージが数十 MB になりやすい。UI の表も 16 行で収まる）。同じ名前の重複は誤り。
6. **口**: CLI `globe simulate --solvers a,b,c [--parallel] [--compare N [--reference NAME]] [--max-order N] [--tolerance X] [-o DIR] [--json]`（終了コードは全部成功かつ比較が合格なら 0、そうでなければ 1）、LSP `globe/simulateMany`（`{request}` → `{response, columns: [{layout, columns}|null]}`。進捗の report に `solver`・`index`・`count`。`$/cancelRequest` は全部に効く）、Desktop の `RPC_METHODS`、wasm `simulate_many_columns`（JS: `GlobeWorker.simulateMany`）、`Backend.simulateMany`（UI はまだ作らない）、Python `Workspace.simulate_many(model, solvers, parallel=, compare=, **settings)`。

## 結果

- 良い: 1 回のコンパイルで何本でも回せる。比較の判定が MSL の参照結果の比較と同じ（試験済みの実装）。最大次数を持たないソルバは注記で分かる。
- 悪い: (a) 並列の実行は硬いモデルを外から kill できない（スレッドを放置する。既存の `checked_detached` と同じ）。(b) 実行ごとの時系列を全部メモリに持つので、1 実行 × 16 本の分のメモリが要る。(c) 管の比較は「何 % ずれたか」の数字を出さない。(d) `bdf` の最大次数は diffsol に口が無いので無視される。必要なら上流への依頼。(e) feature `sundials` を立てたビルドの `cvode`・`ida` の最大次数は、C の API を呼ぶだけで、MSL での挙動は未確認。
- 持ち越し: 別プロセスでの並列（確実に止めたい場合）、最大の相対差の表示（S328 で足した。末尾の付記）。

## 付記（S323、2026-11-15）: UI

実行の設定（`SimSettings.svelte`）に、ソルバの選び方（1 つ＝既定の選択欄／複数＝チェックボックス 1〜16）、実行の仕方（1 つずつ＝既定／並列）、「全部終わったら結果を比べる」（既定 off。n は既定 6、基準は既定で最初に成功したもの）、最大積分次数の欄を足した。決めたこと。

- 許容誤差・最大積分次数は 1 つの欄を全部の実行に同じ値で渡す（ソルバごとの値は持たない）。最大次数の欄は 1〜5 の整数だけ受け、「cvode・ida だけが使う」と注記する。ほかのソルバの注記（GM-I10005）は実行の表に出る。
- ブラウザ（`WorkerBackend`）は並列を選べるが、欄に「ブラウザでは 1 つずつ回す」と出し、応答の `mode_note` も表に出す。Desktop は `inDesktop()` のとき注記を出さない。
- 結果のプロットは、ソルバごとの列を時刻の和集合に折れ線で補って 1 つの `Columns` に重ね、信号名を `x [bdf]` にする（凡例がソルバ名）。出力の間隔が同じなら補いは起きない。補った点は計算の値ではなく表示のためだけである。
- 比較を頼んだときは、実行の表（状態・時間・注記）と比較の表（合否・外れた信号・最大のはみ出し量・最初の時刻）を出す。合否・状態は色だけに頼らず文字で出す。
- 設定はモデルごとの保存（S228、`globe-mode.sim`）に乗る。
- 持ち越し: 複数のときの「それでも実行する」（強行。応答に `can_force` が無い）、ソルバごとの許容誤差、実行の表からの個別の列の表示切り替え。

## 付記（S327）: 出力先の検査とソルバ名の正規化（#119・#120）

- **#119**: `--solvers` の `-o DIR` は計算の前に `check_out_dir` で確かめる。`-` は誤り（標準出力へ複数の結果は出せない。`-` というフォルダは作らない）、既存のファイルを指す・親がファイルは誤り、ライブラリの中は誤り。終了コードは単独の `-o` と同じ 2。フォルダを作るのは結果を書くときのまま（断られた依頼で空のフォルダを残さない）。
- **#120**: 重複の検査と `compare.reference` の照合は、ソルバ名の解決と同じ正規化（前後の空白を除き、小文字にする。`solver_key`）で比べる。別名（`dassl` と `bdf`）は別の名前のまま。CLI の `-o DIR` のファイル名も正規化した名前にする。`compare` は `solvers` が 2 つ未満なら「比べる相手が無い」で断る（UI は 2 つ以上を要求している）。
- 詳細と試験は [S327 の解説書](../sessions/S327-review4-issues/解説書.md)。

## 付記（S328、2026-11-15）: 最大の相対差と絶対差

ユーザーの要望（2026-11-15）「比較閾値（10E-n）を指定してシミュレーションし比較」に近い数字も出す。**管の判定（`passed`・`errors`・`max_excess`）はそのまま**で、比較の結果に次を足した（既存のキーは変えない）。

- `globe-mode-sim::compare::deviations(reference, actual, signals)`: 基準の出力点の時刻に、試験側を線形補間して信号ごとに最大の相対差 `max_rel` と最大の絶対差 `max_abs`、相対差が最大の点の時刻を求める。試験側の時刻の範囲の外の点は比べない。比べる前の正規化は `compare` と同じ（同じ瞬間の 3 行以上は両端だけ、最後の瞬間は 1 行）。時間は信号 1 つあたり O(基準の点数 + 試験側の点数)（両方が時刻の順なので、試験側を指す添字は後ろへだけ進む）。
- 式: `max_rel = max_i |a_i − b_i| / max(|b_i|, range × 1e-12, MIN_POSITIVE)`（a は試験側の補間値、b は基準、`range` は基準の有限な値の最大 − 最小）。下駄の `range × 1e-12` は 0 割りを避ける最小の大きさにとどめ（定数の 0 の信号は `MIN_POSITIVE`）、管の `δy` のような大きな下駄は使わない。その代わり、**基準が 0 を通る点の近くでは、小さな絶対差でも相対差は大きく出る**（例: 範囲 2 の信号が 0 の点で 1e-6 ずれると 5e5）。管の判定と矛盾するときは、管のほうを合否とし、相対差は参考の数字とする。NaN の点は無限大（JSON では `null`）。
- イベントの縦の跳び（同じ時刻の行）は、基準の m 番目の行を試験側の同じ時刻の m 番目（足りなければ最後）と比べる。試験側に同じ時刻の行が無ければ、前後の行の間を補間する。
- 出力: `comparison.pairs[i].max_rel`・`max_abs`（その組の全信号での最大。単位の違う信号をまたぐので `max_abs` は目安）・`worst: {name, time}`（相対差が最大の信号と時刻）、`failures[].max_rel`・`max_abs`、`comparison.max_rel`・`comparison.worst: {solver, name, time}`（全部の組での最大）。比べた点が無ければキーを出さない。CLI の人が読む出力は、全体の 1 行、組ごとの 1 行（閾値 10^-n との大小を `< 1e-6`／`>= 1e-6` で）、外れた信号の行に最大の相対差。UI（`ManyResult.svelte`）は比較の表に「最大の相対差」の列（有効数字 3 桁の指数表記、`（< 1e-6）` か `（≥ 1e-6）`）と全体の 1 行。LSP・Python は JSON・dict にそのまま出る。
- 計算量は、点数 × 信号 × ソルバに比例する。debug ビルドでの測定（2026-11-15、1 信号・基準 100 万点・試験側 100 万点で時刻をずらしたもの）で、`deviations` は 0.24 s、同じ入力の `compare`（管）は 0.90 s だった（管の約 4 分の 1。release は測っていない）。管との重複は、正規化を 2 回（変えるところが無ければ借りるだけ）することだけ。
- 決定 4 の「採らなかった」補間と相対差を、**判定には使わず、数字として足した**。判定を相対差にしない理由（ゼロ交差で閾値が安定しない）は変わらない。

## 付記（S348、2026-11-15）: 実ブラウザでの確認

レビュー第 5 回の「`ManyResult` を実ブラウザで見ていない」に応じ、ブラウザ版（wasm の `WorkerBackend`）で設定から結果の表までを Playwright の Chromium で通した（`src/Svelte/tests/test_ui.py` の `many_result_view`。Decay を bdf・dopri45・rk4、比較 on・閾値 6）。決めたこと。

- 要望の既定は画面でも満たされていた（実行の仕方は 1 つずつ、比較は off、n は 6、3 つ以上選べる）。何も選ばない既定は変えない。
- 並列を頼むと、欄に「ブラウザでは 1 つずつ回す」、結果に「1 つずつ」と応答の `mode_note` が出る。
- 実行の表は選んだ全部（3 行）、比較の表は基準を除く行（2 行）。全体の合否は行の合否の AND、全体の最大の相対差は行の最大と等しいことを試験で確かめる。
- 不具合を 1 つ直した: `SimSettings.svelte` の `label` がインラインで、欄の名前と入力欄・チェックの箱と名前が行の端で別の行に泣き別れていた。`display: inline-block` を足した 1 行（詳細は [S348 の解説書](../sessions/S348-many-ui-e2e/解説書.md)）。
- 持ち越し: Desktop（sidecar の並列）の実機の画面。

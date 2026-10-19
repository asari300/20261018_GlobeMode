# ADR-0012 S9 は BLT と貪欲な tearing で非線形の代数ループの反復変数を見る。S10 は静的な部分（パラメータと start 値の評価、experiment annotation）から始める

- 状態: Accepted（2026-10-19、S010）

## 背景

- 検査の段階 S0〜S10（[architecture.md](../architecture.md) §3.2）のうち、S9「代数ループ: 非線形の反復変数に start 値が無い」と S10「プリフライト: パラメータの評価、t0 で残差が有限か、ソルバの能力が足りるか、experiment annotation が妥当か」が残っていた。P2 の完了条件は S008 で満たした（[roadmap.md](../roadmap.md)）。
- S9 は、BLT（block lower triangular）に分けたときの大きさ 2 以上のブロック（代数ループ）のうち、非線形のものの反復変数（tearing 変数）に start 値が無いと、ソルバは既定の 0 から Newton 法を始める。omc 1.27.1 は `-d=initialization` のとき標準出力に「Nonlinear iteration variables with default zero start attribute in NLSJacN」を出す（S010 で確かめた。[research/openmodelica-dymola.md](../research/openmodelica-dymola.md)）。
- Rumoca v0.10.2 について S010 で確かめた事実（[research/rumoca.md](../research/rumoca.md#s010-で調べた-apiv0102)）:
  - `phase_structural::analyze` は `StructuralDiagnostics::algebraic_loops` を埋めず、ループを ES002 の警告（"algebraic loop detected: {n} equations must be solved simultaneously"）として積むだけ。BLT とループは `phase_structural::sort` の `BltBlock::AlgebraicLoop` からも取れる。
  - `rumoca_eval_dae::NumericEvaluator` は静的な値（パラメータ・定数の束縛と、変数の start）だけを評価する。状態・代数などの実行時の座標は拒む（「never invents a runtime value」）。値は式ごとに有限か確かめ、非有限は `InvalidValue` になる。
  - `CompilationResult::experiment_*` は `sanitize_experiment_settings` を通した後の値で、非有限・StopTime < 0・Tolerance ≤ 0・Interval ≤ 0 を黙って捨て、StopTime < StartTime なら StopTime を捨てる。`CompilationResult` と flat のモデルには annotation が残らない。

## 決定

### S9（`globe-mode-analysis/src/loops.rs`、`analyze_loops`）

analysis crate に自前で書く（Rumoca に依存しない。wasm32 と Track B でも使う）。Rumoca の構造解析は照合にだけ使う。

| 手順 | 中身 |
| --- | --- |
| 解く系 | S7 で微分が要らない（インデックス 1 以下）なら der の形の `incidence`（未知数は `der(x)` と代数・出力）。要るなら **Pantelides 法の最高階の系**: 行は方程式 e の `differentiations[e]` 回目の微分、列は各族の最高階 `orders[u]`。行 e^(c) は、元の参照（階数 o）が c 回の微分で o+c になり、それが `orders[u]` と等しいものだけを未知数として参照する（[ADR-0011](0011-init-system-matching.md) の「j 回の微分は族の階数 o..=o+j を参照する」の最高階だけ）。この場合は `reduced`（状態選択の前の見込み）の印を付ける |
| BLT | 完全マッチング（Hopcroft–Karp）→ 依存グラフ（方程式 e が、別の方程式 e' にマッチした未知数を参照するなら e は e' に依存）→ 強連結成分（再帰を使わない Tarjan 法、O(E)）。出てきた順が解く順。ブロックの分け方はマッチングの選び方によらない（Duff 1981。並べ替えの乱択テストで確かめた） |
| 線形・非線形 | ブロックのどれかの方程式が、ブロックの未知数を非線形に参照していれば非線形のループ。参照が非線形かは `DaeStructure::nonlinear`（下）。最高階の系では、c ≥ 1 の行は最高階について線形（連鎖律）、c = 0 の行は `nonlinear` に従う |
| tearing | 貪欲法（最適な tearing は NP 困難）。残りの未知数が 1 つだけで、それが線形に現れる方程式があれば解く。無ければ tearing 変数を 1 つ選ぶ（start 値のある代数・出力 → start の無い代数・出力 → `der(x)` と階数 1 以上の列の順。同じ組の中では出現の多い順）。最後に残った方程式が残差で、本数は tearing 変数の数と同じ。O(E + k log k) |
| start 値 | 非線形のループの tearing 変数のうち、階数 0 の代数・出力で start の無いもの。`der(x)` と高階の列は start を書けないので除く。初期化系の構造（`InitStructure`）が無ければ判定しない |

| 診断 | 重さ | 中身 |
| --- | --- | --- |
| `GM-I9001` | Info | ループがあれば 1 つ。「代数ループが N 個（線形 a・非線形 b、最大 k 本の方程式）」。位置は一番大きい非線形のループの最初の方程式 |
| `GM-W9001` | Warning | start の無い反復変数を持つ非線形のループごとに 1 つ。位置はその変数の宣言 |

- S9 は Error を出さない（実行は止めない）。解析できない（S7 の打ち切り、長さの不一致、完全マッチングが無い）ときは理由を `LoopReport::skipped` に残し、診断は出さない。
- **線形・非線形の判定**（`globe-mode-frontend/src/structure.rs`、`DaeStructure::nonlinear`）: 残差の式木を族（変数）の単位で歩き、行の incidence のうち非線形の族に属するスカラーを非線形にする（広めに見積もる）。加減・符号・配列の構築・添字は和、積は両辺に未知数があれば両辺とも非線形、商は分母に未知数があれば全部、冪は底に未知数があり指数がリテラル 1 でないか指数に未知数があれば全部、組み込み・利用者の関数は引数の全部（`abs`・`min`・`max` も広めに非線形）。if 式は分岐の和で、条件の関係演算はイベントで凍結されるので非線形にしない。`noEvent` の中の条件は凍結しないので、その if 式は全部を非線形にする。`homotopy(actual, simplified)` は両方の和。状態 `x`・パラメータ・離散・`pre`・`time` は既知。
- db のクエリ `loop_report` は `dae_shape`・`index_report`・`init_shape` を読む（どれも位置を落とした写し）。`check_model` は S0〜S8 に Error が無く S7 の結果があるときだけ回す。

### S10（静的な部分。`globe-mode-frontend/src/preflight.rs`、`globe-mode-analysis/src/preflight.rs`）

| 対象 | 取り出し（frontend） | 判定（analysis） |
| --- | --- | --- |
| パラメータ | パラメータと定数を `NumericEvaluator::initial_value` で変数ごとに評価する。`fixed = false` のパラメータと、束縛や start がそれに（推移的に）依存するものは評価しない（初期化で決まる。仮の start で偽の 0 除算を出さないため）。String は飛ばす。参照先の失敗を受け継いだだけの失敗は落とす | 非有限（NaN、無限大、0 除算）→ `GM-E10001`。関数の中の `assert` の失敗 → `GM-E10002`。外部関数 → `GM-I10002`（1 つにまとめる）。値が無い（S8 の `GM-W8003` と重なる）・評価器の対応していない演算は出さない |
| start 値 | 状態・代数・出力・離散変数の start を `initial_value` で評価する。値が無いのは既定の 0 なので失敗にしない | 非有限 → `GM-E10003` |
| experiment annotation | Rumoca の値は不正な値を捨てた後なので、根のクラスを定義するファイルを `Dae::source_map()` から探し、Rumoca の parser で読み直して `annotation` の生の値と位置を取る（見つからなければ Rumoca の値で代える） | 無い → `GM-I10001`。StopTime < StartTime・Tolerance ≤ 0・Interval ≤ 0 → `GM-E10004`。Interval が区間より長い・Tolerance ≥ 0.1・値が数の字面でない → `GM-W10001`。StopTime = StartTime は出さない（MSL の `Inverse_sine` が `StopTime = 0` を使う）。StartTime が無ければ 0 |

- db のクエリは `preflight_shape`（位置を落とした写し）→ `preflight_report`。`check_model` は S0〜S9 に Error が無いときだけ回す。
- **残差が t0 で有限か、初期化のヤコビ行列の階数（S009 の `AdvancedSolenoid`・`Invalid.UngroundedLoop` の数値的な特異）、ソルバの能力は、P3 の実行系（`globe-mode-exec`）ができてから**にする。Rumoca の評価器は実行時の座標を評価しないので、残差を評価するには実行系と同じ評価の道具が要る。

### 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| Rumoca の `phase_structural::sort` と `tear_algebraic_loop` をそのまま使う | analysis crate は Rumoca に依存しない約束（wasm32、Track B の自前コアでも同じ検査を使う）。Rumoca は照合（ループの大きさ）にだけ使う |
| 最小の tearing を探す | NP 困難。S9 の目的は start 値の無い反復変数を見つけることで、tearing 変数の数そのものではない。start 値のある変数を先に選ぶので、警告は「避けられないもの」に近い |
| 線形の判定をスカラーの単位でする | 配列の式を要素ごとに展開すると、MSL の大きなモデルで時間が増える。族の単位で広めに見積もっても、誤るのは「線形なのに非線形」の側だけ（警告が多めに出る側） |
| S10 で t0 の残差を自前の評価器で評価する | P3 の実行系で作る評価と重なる。二重に作らない |
| experiment を Rumoca の値で判定する | StopTime < StartTime などの誤りが値ごと消えて見つけられない |
| 字面の 0 除算（`parameter Real T = 1 / a`、a = 0）を S10 で見る | Rumoca が DAE への変換で `ED019`（"cannot be evaluated: division by zero"）で止め、[ADR-0007](0007-unsupported-classification.md) のコードの規則で `GM-U0001`（未対応）になる。モデルの誤りなので振り分けを見直す候補にした（roadmap） |

## 結果

（S010 の実行結果。数字は [msl-funnel.md](../msl-funnel.md)、[omc-compare.md](../omc-compare.md)、[作業記録](../../log/work/2026-10-19-s010-loops-preflight.md)）

- **実装**（2026-10-19）: `globe-mode-analysis/src/loops.rs`（`analyze_loops`、`LoopReport`、`AlgebraicLoop`）と `preflight.rs`（`analyze_preflight`、`PreflightReport`）、`globe-mode-ir`（`DaeStructure::nonlinear`、`PreflightStructure`）、`globe-mode-frontend`（`structure.rs` の線形・非線形の判定、`preflight.rs`）、`globe-mode-db`（`loop_report`、`preflight_shape`、`preflight_report`、`check_model` の S9・S10）、CLI（`globe check` の S9・S10 の行と `--json` の `s9` / `s10`）、`msl-funnel`（S9・S10 の列と集計、Rumoca のループの大きさとの照合）、`omc-compare`（S9 の照合。`-d=initialization,iterationVars`）。
- **検証用のモデル**: `Loops.LinearLoop` / `NonlinearLoopWithStart` / `NonlinearLoopWithoutStart` / `NoLoop`、`Preflight.*` 10 本（[src/Modelica/README.md](../../src/Modelica/README.md)）。S10 の 9 本は omc の `checkModel` がすべて通し、`simulate` で初めて失敗するか黙って進むもの。
- **MSL の全例題**（[msl-funnel.md](../msl-funnel.md)）: OK 369・NG 13・GM-U 184・panic 0 は S009 と同じ（S9・S10 で止まったものは 0）。S9 は 369 本すべてで解析した（解析しなかったもの 0、最高階の系で数えたもの 154）。代数ループのあるもの 226（線形 254 個・非線形 260 個）、非線形のループのあるもの 142、`GM-W9001` のあるもの 57（反復変数 188 個）。一番大きいループは `MultiBody.Examples.Loops.EngineV6` の 4584 本。**ループの大きさの一覧は、微分の要らない 215 本すべてで Rumoca の構造解析（ES002）と一致**した。S10 は Error 0、`GM-I10001`（experiment が無い）が 16 本。
- **omc との照合**（[omc-compare.md](../omc-compare.md)、[ADR-0010](0010-omc-comparison.md) の S010 の追記）: 非線形の代数ループの有無は 355 本中 282 本（79.4%）、start 値の無い非線形の反復変数の有無は 312 本（87.9%）が一致。omc は別名の消去と式の変形を先にするので、数は比べない。食い違いの主な見込みは、線形の判定が「方程式ごとの非線形に現れる未知数の集合」しか持たないこと（`v = R*i` の `R` がループの外で先に決まっても、`i` について非線形とみなす）。積の項ごとの未知数の組を持てば、ループの未知数だけで線形かを判定できる（S011 の候補）。
- **Rumoca の `algebraic_loops` は空**: 照合を作る途中で、`phase_structural::analyze` が `algebraic_loops` を埋めないことが分かった（上の背景）。GlobeMode の照合は ES002 の文言から大きさを読む。上流に報告する候補。
- **字面の 0 除算は S10 に届かない**: Rumoca が DAE への変換で `ED019` で止め、`GM-U0001` になる（選ばなかった案の表）。検証用のモデルは `sqrt(-1)` で NaN になるパラメータにした。

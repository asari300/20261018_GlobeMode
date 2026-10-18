# ADR-0010 omc との照合は別プロセスで行い、checkModel の数と translateModel の文言で突き合わせる

- 状態: Accepted（2026-10-19、S008）

## 背景

- [roadmap.md](../roadmap.md) の P2 の完了条件の 3 つ目は「S5 が omc と 95% 以上一致する」。S005・S007 では omc（OpenModelica のコンパイラ）が入っていなかったので測れず、ユーザーの指示で後回しにしていた。2026-10-19 に OpenModelica 1.27.1 が入った（`/usr/bin/omc`、`omc --version` は `OpenModelica 1.27.1~2-g6db4671`）。
- [ADR-0007](0007-unsupported-classification.md) は、GM-U（コンパイラが未対応）の振り分けの C「omc と照合する」を任意にしていた。既知の制約の一覧（`GM-U0003`、50 行）が本当に Rumoca の制約か（omc では通るか）は未確認だった。
- S005 で S6（構造特異）にした MSL の例題 12 本（`OpAmpCircuits.*`、`FluxTubes…Components.*`）が、単体で本当に特異かも未確認だった。
- OpenModelica は OSMC-PL / AGPL なので、**リンクもコードの複製もしない。別プロセスで呼ぶ照合用途だけ**にする（[CLAUDE.md](../../CLAUDE.md) の 3、[licenses.md](../licenses.md)）。
- omc 1.27.1 のスクリプト API を手元で確かめた事実は [research/openmodelica-dymola.md](../research/openmodelica-dymola.md#s008-で確かめた-omc-1271-のスクリプト-api) にある。要点:
  - `checkModel(M)` は方程式数と変数数（スカラー）を返す。**partial や数が合わないモデルでも「completed successfully」**で、数だけが違う。合う / 合わないは数で判断する。
  - `translateModel(M)` はバックエンド（マッチング、インデックス低減、初期化の解析、コード生成）まで走る。構造特異は「Model is structurally singular」「IndexReduction.pantelidesIndexReduction failed」「An independent subset of the model has imbalanced number of equations」の Error で `false` になる。
  - 初期条件の過剰は Warning「The initial conditions are over specified」（`-d=initialization` で余分な初期方程式の本数も出る）。**不足は何も出ない**（start 値で黙って埋める）。
  - `.mos` の中で `print(String(translateModel(M)))` のように式の中で呼ぶと、失敗したときの `getErrorString()` が空になる。文として呼べば残る。
  - omc 同梱の MSL は `Modelica 4.1.0+maint.om`（OpenModelica 向けの修正入り）。`setModelicaPath` と `loadModel(Modelica, {"4.1.0"})` で `data/msl` の素の 4.1.0 も読める（「ModelicaServices が patch されていない」という警告と、`Complex` を読み飛ばす通知が出るが、例題の検査には差し支えない）。
  - 速さ: MSL の読み込み約 4.6 s、`checkModel` 1 本 0.1 s 前後、`translateModel` も多くは数秒以内。

## 決定

- **`cargo xtask omc-compare` が omc を別プロセスで呼ぶ**（`src/Rust/xtask/src/omc.rs`）。入力は `msl-funnel --json` の結果（globe check の S0〜S8 の判定）で、ファネル自体は omc に依存しない。omc が無い環境では `omc-compare` だけが動かない。
- **omc に読ませる MSL は `data/msl` の素の 4.1.0**（globe check と同じもの）。omc 同梱の `+maint.om` は、中身が違うと照合にならないので使わない。
- **N 本ごとに 1 つの `.mos` を書いて `omc script.mos` を実行し、標準出力を印（`@@BEGIN` / `@@ERR` / `@@TRANSLATE` / `@@END`）で切って読む。** `checkModel` と `translateModel` は文として呼び、印だけを `print` する。omc が落ちたり時間切れになったりしたら、印の無いモデルを 1 本ずつ再実行し、それでも駄目なら「結果なし」にして続ける（既定: 50 本ごと、制限 900 s / プロセス）。
- **照合の規則**:

  | 段階 | globe check 側 | omc 側 | 比べ方 |
  | --- | --- | --- | --- |
  | S5 | 判定があるもの（OK、または S5 以降で止まったもの）。GM-U と S4 以前で止まったものは判定なし | `checkModel` の方程式数 = 変数数 | **「合う / 合わない」の判定が同じなら一致**。一致率が P2 の完了条件。方程式数と未知数（どちらもスカラーの総数）が同じ数かは副次的に数える |
  | GM-U | GM-U にしたモデル | `checkModel` の数が合う、`translateModel` が通る | omc で通れば Rumoca の制約（ADR-0007 の C）。`GM-U0003` の一覧は 1 行ずつ表にする |
  | S6 | S6 で止めた（特異）か、通ったか | `translateModel` の失敗の種類（特異 / 部分系が不均衡 / その他） | 特異 ↔ 特異、通る ↔ 通る なら一致。omc が別の理由で失敗したものは比べない |
  | S7 | — | — | omc は構造インデックスを出さないので比べない |
  | S8 | 与えられた初期条件 > 要る数（過剰）か | 「over specified」の警告 | **過剰だけ**比べる。不足は omc が報告しないので比べない。omc の translateModel が失敗したものは比べない |

- **報告の置き場は [doc/omc-compare.md](../omc-compare.md)**（`mise run msl:omc`）。`msl:funnel` は JSON も `target/msl-funnel.json` に残す（`counts` に `unknowns` を足した）。CI には入れない（omc が要る。ランナーには無い）。

## 結果

（S008 の実行結果。数字は [omc-compare.md](../omc-compare.md) と [作業記録](../../log/work/2026-10-19-s008-omc-compare.md)）

- omc 1.27.1 で MSL 4.1.0 の全例題 566 本を照合した。omc は 12 プロセス（50 本ごと）で 391 s。時間切れ 0・異常終了 0・再実行 0・結果の無いモデル 0。`checkModel` は 566 本すべてで数が出た。
- **S5: 判定のある 382 本すべてで omc と一致（100%）。P2 の完了条件「95% 以上」を満たした。** 方程式数と未知数（スカラーの総数）が omc と同じ数なのは 307 / 382（80.4%）。違う 75 本のうち 72 本は `Clocked`（omc の方程式数・変数数が 2〜30 多い。クロックの変数などを数えているとみられるが内訳は未調査）、残りは `Fluid.Examples.InverseParameterization`（+8）、`…ExampleUtilities.SpacePhasorLimiter`（+2）、`Media.Examples.TwoPhaseWater.BaseProperties`（+2）。差はどれも方程式数と変数数の両方に同じだけ付くので、合う / 合わないの判定は変わらない。
- **GM-U 184 本は omc の `checkModel` で全部数が合い（100%）、`translateModel` も 180 本（97.8%）が通る。既知の制約の一覧（`GM-U0003`、50 本）は全部 omc で通った。** 一覧は Rumoca の制約として正しい（ADR-0007 の C を実施）。通らない 4 本は、接続の無い部品モデル 2 本（`Hysteresis.Components.Transformer1PhaseWithHysteresis` / `Transformer3PhaseYyWithHysteresis`。omc も特異）、`FieldWeakeningController`（omc 側の別の失敗）、`Utilities.Examples.ReadRealMatrixFromFile`（omc が `modelica://` のファイルを開けない）。
- **S6: 比べた 363 本中 362 本が一致（99.7%）。** globe が S6 で止めた 12 本のうち 7 本は omc も特異（接続の無いピンの `n.i = 0.0` を「Found equation without time-dependent variables」とし、Pantelides が失敗）。5 本（`OpAmpCircuits.Der` / `Derivative` / `FirstOrder` / `Integrator` / `PI`）は omc が手前の「Parameter f（T）has neither value nor start value, and is fixed」で止まるので比べられない。不一致は `SolenoidActuator.Components.AdvancedSolenoid` の 1 本: globe は S6 を通し（まとめた二部グラフで 290 / 290）S7 でインデックス 2（方程式 175 本を微分、動的な状態 5 / 8）、omc は「Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equations」。どちらが正しいかは**未確認**（S009 の候補）。
- **S8（過剰だけ）: 比べた 355 本中 354 本が一致（99.7%）。** globe が `GM-W8004`（過剰の見込み）にした 3 本（`Electrical.Analog.Examples.GenerationOfFMUs` 8 / 6、`Lines.CompareLineTrunks` 60 / 57、`Thermal.HeatTransfer.Examples.GenerationOfFMUs` 4 / 3）は omc も「over specified」で、余分な初期方程式の本数（2・3・1）も globe の差と同じ。不一致は `Fluid.Examples.NonCircularPipes` の 1 本: globe は「不足 4 / 40」、omc は「冗長な初期方程式 2 本」。数だけで見る S8 は、与えられた初期条件の中の冗長さを見られない（S009 の候補）。
- globe が OK なのに omc の `translateModel` が失敗した 13 本は、partial の `PowerConverters.Examples.*.ExampleTemplates.*` 12 本（omc は「Illegal to instantiate partial class」。globe は Rumoca のハーネスと同じく partial も数えて判定する）と、`SpacePhasorLimiter`（omc は「Parameter yMax has neither value nor start value」を Error。globe は S8 の警告 `GM-W8003`）。
- 最初の全例題の実行では、`-d=initialization` の反復変数の一覧が `true` の前に出るのを読み違え、52 本を「失敗」と数えた。結果を区間の最後の行で読むように直し、単体テストを足した。
- 検証用モデルでも確かめた: `Invalid.StructurallySingular` は omc でも `translateModel` が「imbalanced number of equations (1) and variables (2)」で失敗し、`Invalid.HighIndexDuplicate` は「Model is structurally singular, error found sorting equations」と「over specified」が出る。
- 残り: `AdvancedSolenoid` の真偽、S8 の冗長な初期条件、omc が手前で止めるモデル（`GM-W8003` 相当）の扱い。omc は照合にだけ使い、検査の判定には使わない（ライセンス、速さ、依存の方針は変えない）。

# OpenModelica と Dymola の調査

- 確認日: 2026-10-18
- 確認できなかった項目には【未確認】と書いた。
- 目的: IDE に要る機能とソルバの品ぞろえを洗い出すこと。**OpenModelica のコードは読むだけにし、リンクも複製もしない**（[licenses.md](../licenses.md)）。

## OpenModelica

### 版と構成

- 版: 最新は **1.27.1**（2026-09-08）。1.27.0 は 2026-07 初旬、1.26.0 は 2025-12-18。1.28.0-dev の文書も公開されている。
- **OMC（コンパイラ）**: MetaModelica で書かれている。
  - 既定のフロントエンドは New Frontend（NF）。
  - New Backend（NB）はライブラリテストの約30%を通る（2026 年のワークショップ発表）。
- **OMEdit**: GUI。
- **OMSimulator**: FMI と SSP のコシミュレーション。Python バインディング付きで再設計中。FMI 3 への対応は部分的【未確認】。
- **OMPython / OMJulia / OMMatlab**: スクリプトから使う API。
- **OMShell / OMNotebook / OMWebbook**: 対話シェルとノートブック。廃止の告知は見当たらない【未確認】。
- **Base Modelica の出力**: `--baseModelica` 系のフラグがある。正確な綴りは【未確認】。

### OMEdit の機能（GlobeMode の機能表の元）

- Icon / Diagram / Text / Documentation の4ビュー（Documentation は WYSIWYG で編集できる）
- Libraries / Variables / Messages の各ブラウザ。接続線と図形の描画
- Check Model / Check All、Instantiate Model
- Simulation Setup（フラグを annotation に保存する）。再コンパイルせずに再実行できる
- プロット: 時系列、パラメトリック、配列
- 変換デバッガ、アルゴリズムデバッガ、state machine のデバッガ。3D アニメーション
- パッケージマネージャ: インストール、更新、索引、変換スクリプト
- FMU の Import と Export（FMI 1 / 2 / 3、ME / CS）、SSP エディタ
- データ照合、感度解析と最適化、暗号化パッケージの出力
- LSP 風のホバーと定義ジャンプ

### コンパイラのパイプライン

1. parse
2. NF による instantiate と flatten（接続集合、redeclare、構造パラメータの評価）
3. 記号的な簡約
4. マッチング（PFPlusExt）
5. インデックス低減（Pantelides と、動的な状態選択またはダミー微分）
6. BLT と Tarjan
7. tearing（omcTearing / cellier）
8. C / C++ のコード生成
9. 実行時ランタイム（イベント処理、初期化、線形・非線形ソルバ）

既定の手法は記憶に頼った部分がある。`omc --help` で確かめること【未確認】。

### ソルバ（`-s` で選ぶ）

- 一覧:
  - `dassl`（既定）、`ida`、`cvode`
  - `gbode`、`euler`、`heun`、`rungekutta`（非推奨。gbode を使う）
  - `impeuler`、`trapezoid`、`imprungekutta`、`irksco`、`rungekuttaSsc`
  - `symSolver`、`symSolverSsc`、`qss`、`optimization`
- **gbode** は汎用の RK 枠組み。
  - 既定は ESDIRK4。
  - ほかに SDIRK / ESDIRK / Gauss / Radau / Lobatto / Fehlberg / DOPRI45 / Merson の方式と bi-rate モードがある。
- 今回取得した文書で確かめられたのは dassl、ida、cvode、gbode、euler、rungekutta、symSolver(Ssc)、qss。
- heun、impeuler、trapezoid、imprungekutta、irksco、rungekuttaSsc は古い文書と記憶による【未確認】。

### ライセンス

- OMC は **OSMC-PL 1.8**。誰でも **AGPL-3.0** か OSMC-PL のどちらかを選べる。
  - EPL-1.0 ベースの Internal / External モードは、OSMC の Level 1 / 2 の会員だけが使える。
  - 以前の GPL-3 の文言はもう使われていない。
- 実行時ランタイムのファイルは BSD-New / AGPL-3 / OSMC-PL のいずれか。そのため生成されたシミュレーションコードは寛容なライセンスで扱える。
- **GlobeMode の扱い**:
  - 文書やソースを読んで振る舞いを学び、自分で実装し直すのはよい。
  - `omc` を**別プロセス**で呼ぶ（CLI・スクリプト API・ZMQ）のは集合著作物の扱いで、照合用に限って使う。
  - **静的リンク・埋め込み・コードの複製はしない**。いずれも AGPL の義務を負う。
  - `tree-sitter-modelica` と `modelica-language-server`（OpenModelica 製）も AGPL / OSMC-PL なので使わない。

### S008 で確かめた omc 1.27.1 のスクリプト API

照合（[ADR-0010](../adr/0010-omc-comparison.md)、`cargo xtask omc-compare`）のために、手元の omc で確かめた（確認日 2026-10-19、`omc --version` は `OpenModelica 1.27.1~2-g6db4671`、WSL2 の Ubuntu に apt で導入）。関数の説明は User's Guide の Scripting API（出典）にある。

| 項目 | 確かめたこと |
| --- | --- |
| 同梱のライブラリ | `~/.openmodelica/libraries/` に `Modelica 4.1.0+maint.om`、`3.2.3+maint.om` など（`getAvailableLibraries()`）。`+maint.om` は OpenModelica 向けの修正入り |
| 素の MSL を読む | `setModelicaPath("<data/msl の絶対パス>"); loadModel(Modelica, {"4.1.0"});` で `Modelica 4.1.0/package.mo` を読む（`getSourceFile(Modelica)` で確認）。「ModelicaServices が OpenModelica 向けに patch されていない」という警告と、「`Complex` (4.1.0) を読み飛ばした（uses-annotation）」という通知が出るが、例題の検査には差し支えなかった |
| `checkModel(M)` | 文字列を返す: `Check of M completed successfully.` / `Class M has N equation(s) and M variable(s).` / `K of these are trivial equation(s).`。**partial のクラスや数が合わないモデルでも「completed successfully」**で、数だけが違う（`PartialOpAmp` は 23 / 27、手元の `x = 1;` だけの 2 変数モデルは 1 / 2、過剰決定の 1 変数モデルは 2 / 1）。構造特異（`GlobeModeValidation.Invalid.StructurallySingular`）も 3 / 3 で通る |
| `translateModel(M)` | バックエンド（マッチング、インデックス低減、初期化の解析、C コード生成）まで走って `true` / `false` を返す。生成物（`M.c`、`M.makefile` など十数ファイル）を**カレントディレクトリに書く** |
| 構造特異の文言 | `Error: Model is structurally singular, error found sorting equations`、`Error: Internal error IndexReduction.pantelidesIndexReduction failed! System is structurally singular and cannot be reduced`（または `Found empty set of continuous equations`）、`Error: An independent subset of the model has imbalanced number of equations (1) and variables (2).`（`pre-optimization module clockPartitioning (simulation) failed`）。MSL の `OpAmpCircuits.*` と `FluxTubes…Components.*` の 12 本はいずれかで `false` |
| 初期条件 | 過剰: `Warning: The initial conditions are over specified.`。`setCommandLineOptions("-d=initialization")` を付けると `The following N initial equations are redundant, so they are removed from the initialization system:` と本数が出る。**不足は何も出ない**（`-d=initialization` でも。start 値で埋める） |
| 式の中で呼ぶと誤りが消える | `print(String(translateModel(M)))` のように**式の中で呼ぶと、失敗したときの `getErrorString()` が空**になる。`translateModel(M); getErrorString();` と文として呼べば残る。`print` は印（区切り）だけに使う |
| 速さ | MSL 4.1.0 の読み込み約 4.6 s（`loadModel` だけのスクリプト）。`checkModel` は 1 本 0.1 s 前後（例題 20 本で 2 s）。`translateModel` は多くが数秒以内 |
| 計時 | スクリプトの中では `time()` も `timerTick()` も無い（`Class time not found`）。時間は外から測る |

### S009 で確かめた omc 1.27.1 の診断の文言

S008 で食い違った 2 本と、検証用のモデルで確かめた（確認日 2026-10-19、`omc --version` は `OpenModelica 1.27.1~2-g6db4671`。GlobeMode の [ADR-0011](../adr/0011-init-system-matching.md)）。

| 項目 | 事実 |
| --- | --- |
| 数値的な特異 | 接地の無い回路の部品を単体で `translateModel` すると、記号的な簡約（alias の除去）の後に `0.0 = 0.0` の方程式が残り、「Model is structurally singular, error found sorting equations」と、対応づかない変数（例: `r_par1.v`）が出る（`-d=bltdump` で確かめた。`AdvancedSolenoid`）。小さな部品では「Internal error Circular Equalities Detected for Variables」と「System is structurally singular and cannot be handled because the number of unassigned equations is larger than the number of states」（`GlobeModeValidation.Invalid.UngroundedLoop`）。どちらも二部グラフの完全マッチングはあるので、omc の「structurally」は記号的な簡約の後の構造を指す |
| 不足の文言 | 初期条件が足りないとき、`-d=initialization` では「Assuming fixed start value for the following N variables: …」の Warning が出る（`GlobeModeValidation.Init.RedundantAndMissing` の `y`）。S008 では「不足は何も出ない」としていたが、`-d=initialization` を付ければこの文言で分かる。ただし何本足りないかではなく、omc がどの変数を fixed と仮定したかの一覧 |
| 冗長の判定の条件 | 拘束で決まる状態に `fixed = true` が付いていても、拘束の相手（パラメータ）がコンパイル時に評価できて start と整合すれば、警告は出ない（`RedundantAndMissing` の `x = pb`、`pb = 1`、`start = 1`）。評価できないとき（`NonCircularPipes` の `boundary.p`）は「It was not possible to determine if the initialization problem is consistent」と「The initial conditions are over specified. The following N initial equations are redundant」が出て、冗長な初期方程式を外して進む |
| `dumpInitialSystem` | 1.27.1 には無い（「Unknown debug flag dumpInitialSystem」） |

### S010 で確かめた omc 1.27.1 の代数ループとプリフライトの文言

GlobeMode の S9・S10（[ADR-0012](../adr/0012-loops-and-preflight.md)）の照合のために、小さなモデルと MSL の数本で確かめた（確認日 2026-10-19、`OpenModelica 1.27.1~2-g6db4671`）。

| 項目 | 事実 |
| --- | --- |
| 反復変数の一覧 | `setCommandLineOptions("-d=iterationVars")` で、`translateModel` の後の `getErrorString()` に「Notification: List of all iteration variables (DAE kind: initialization)」と「… (DAE kind: simulation)」が出る。系ごとに「Iteration variables of torn nonlinear equation system:」「… of torn linear equation system:」「… of equation system with analytic Jacobian:」の見出しと、変数の行（`  x:VARIABLE(start = 1.0 )  type: Real`。属性の括弧に `start = …` があれば start 値がある）が並ぶ。系が 1 つも無くても 2 つの見出しは出る |
| start の無い非線形の反復変数 | `-d=initialization` を付けないと、`getErrorString()` に「Warning: There are nonlinear iteration variables with default zero start attribute found in NLSJac0. For more information set -d=initialization.」が出る。付けると、警告の代わりに**標準出力**に「Nonlinear iteration variables with default zero start attribute in NLSJac0. (1)」と番号付きの変数の一覧、「Info: Only non-linear iteration variables in non-linear eqation systems require start values.」が出る（`getErrorString()` には出ない）。2 元の `x*y = 1 + time; x + y = 3`（tearing で 1 元に縮む）では何も言わず、3 元の非線形の系で出た |
| 線形のループ | 線形の系の反復変数は「torn linear equation system」として一覧に出るが、start 値の警告の対象ではない |
| S10 の検証用のモデル | `checkModel` は `GlobeModeValidation.Preflight` の 9 本すべてで「completed successfully」（静的には誤りを見つけない）。`simulate` では、`sqrt(-1)` のパラメータは「Model error: Argument of sqrt(area) was -1 should be >= 0」と「simulation terminated by an assertion at initialization」、start の `log(0)` は「Argument of log(x0) was 0 should be > 0」、関数の中の `assert` は初期化で当たる。`experiment(StartTime = 1, StopTime = 0.5)` は警告（「Integrator step size greater than length of experiment … start time: 1.000000, stop time: 0.500000, integrator step size: -0.001000」）だけで進み、`Tolerance = 0` は「DASKR--  ALL ELEMENTS OF RTOL AND ATOL ARE ZERO」で失敗する |

## Dymola（Dymosim の積分器）

| 分類 | ソルバ |
| --- | --- |
| 可変刻み | Lsodar, Dassl, Radau IIa, Esdirk23a, Esdirk34a, Esdirk45a, Dopri45, Dopri853, Sdirk34hw, Cerk23, Cerk34, Cerk45, Cvode |
| 固定刻み | Euler, Rkfix2, Rkfix3, Rkfix4 |

GlobeMode のソルバ名との対応は [architecture.md](../architecture.md#33-ソルバ) の表にある。

## IDE に要る機能（両者から抽出）

1. エラーに強いパーサと LSP（診断、補完、定義ジャンプ、参照、リネーム、ホバー、アウトライン、整形、lint）
2. ライブラリとパッケージのブラウザ。MSL の読み込み、`uses` によるバージョン管理
3. Icon / Diagram ビュー（Placement、Line、connect の経路）。テキストと双方向に編集できる
4. Documentation ビュー（HTML annotation）
5. パラメータダイアログ（modifier、replaceable / redeclare の選択）
6. Check Model（方程式数と変数数の釣り合い）、Instantiate / flatten の表示
7. シミュレーションの設定（experiment annotation、ソルバ）、実行、パラメータを変えた再実行
8. 結果のプロットと変数ブラウザ
9. 変換デバッガ（BLT、インデックス低減、tearing）
10. FMU の Export と Import、SSP
11. コード生成ターゲット
12. WASM と Web 版のビルド、Python API
13. OMC の参照波形との照合テスト

これを GlobeMode の優先度と Phase に割り当てた表が [features.md](../features.md)。

## 出典

- <https://github.com/OpenModelica/OpenModelica/releases>
- <https://openmodelica.org/doc/OpenModelicaUsersGuide/latest/omedit.html>
- <https://openmodelica.org/doc/OpenModelicaUsersGuide/latest/solving.html>
- <https://openmodelica.org/doc/OpenModelicaUsersGuide/latest/scripting_api.html>（Scripting API: `checkModel`、`translateModel`、`setModelicaPath`、`setCommandLineOptions`。確認日 2026-10-19）
- <https://raw.githubusercontent.com/OpenModelica/OpenModelica/master/OSMC-License.txt>
- <https://openmodelica.org/osmc-pl/osmc-pl-1.8-runtime.txt>
- <https://openmodelica.org/images/M_images/OpenModelicaWorkshop_2026/OpenModelica2026-talk01-FrancescoCasella-OpenModelica-Workshop-StatusDirections-v1.pdf>
- <https://github.com/OpenModelica/tree-sitter-modelica>
- <https://github.com/OpenModelica/modelica-language-server>
- Dymola: <https://help-3dexperience.aesvietnam.com/English/SemUserMap/svecommon-c-DymosimIntegrators.htm>

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
- <https://raw.githubusercontent.com/OpenModelica/OpenModelica/master/OSMC-License.txt>
- <https://openmodelica.org/osmc-pl/osmc-pl-1.8-runtime.txt>
- <https://openmodelica.org/images/M_images/OpenModelicaWorkshop_2026/OpenModelica2026-talk01-FrancescoCasella-OpenModelica-Workshop-StatusDirections-v1.pdf>
- <https://github.com/OpenModelica/tree-sitter-modelica>
- <https://github.com/OpenModelica/modelica-language-server>
- Dymola: <https://help-3dexperience.aesvietnam.com/English/SemUserMap/svecommon-c-DymosimIntegrators.htm>

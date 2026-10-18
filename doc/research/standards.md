# 規格の調査（Modelica / FMI / SSP / eFMI）

- 確認日: 2026-10-18
- 確認できなかった項目には【未確認】と書いた。

## 一覧

| 規格 | 現行版 | ライセンス | GlobeMode での扱い |
| --- | --- | --- | --- |
| Modelica Language Specification | **3.7**（2026-05）。master は 3.8-dev | 文書 | 対象とする言語版 |
| Modelica Standard Library | **4.1.0**（2025-05-23） | BSD-3-Clause | 検証用のコーパス。`data/msl/` に取得する（コミットしない） |
| FMI | **3.0.2** と **2.0.5**（2024-11-27。どちらも不具合修正のみ） | 文書は CC-BY-SA-4.0、ヘッダと XSD は **BSD-2-Clause** | 2.0 と 3.0 の ME / CS / SE（SE は 3.0 のみ）の Import と Export |
| FMI Layered Standards | LS-XCP 1.0.0、LS-BUS 1.0（1.3.0-beta.1）、LS-STRUCT 1.0.0-beta.1、LS-REF 1.0.0-beta.1、**LS-DAE 1.0.0-alpha.1**（2026-09-11） | 同上 | LS-DAE は Modelica から出力する経路として注視する |
| SSP | **2.0.1**（2026-05-05）。2.0 は 2024-12-20 | スキーマは BSD-2、文書は CC-BY-SA-4.0 | 系の構成の入出力（P8） |
| Base Modelica（MCP-0031） | 策定中（0.1 のロードマップ） | — | Rumoca のテンプレートで出力できる。正式化を待つ |
| eFMI | 1.0.0 Beta 2（2026-10-07） | — | 対象外（将来の検討事項） |

- LS-STRUCT の版は、トップページに「rc.1」と書かれていて食い違いがある【未確認】。
- Base Modelica のページは古い可能性がある【未確認】。
- 「Modelica 4」「Modelica Base Library」「LinkModelica」は一次資料が見つからなかった。計画には入れない。

## FMI 2.0 と 3.0 の違い

| 観点 | FMI 2.0 | FMI 3.0 |
| --- | --- | --- |
| インターフェース | Model Exchange、Co-Simulation | ME、CS、**Scheduled Execution** |
| クロック | 無い | 時間クロックとトリガークロック。クロック付きの変数 |
| CS の拡張 | — | イベントモード、intermediate update（通信点の間で出力する）、`doStep` からの early return |
| 型 | Real / Integer / Boolean / String / Enumeration | Float32/64、Int8〜UInt64、**Binary**、Clock |
| 配列 | スカラーのみ | 多次元配列と構造パラメータ |
| 付属ファイル | `modelDescription.xml` | `terminalsAndIcons.xml`、ソース FMU 用の `buildDescription.xml`、`extra/` 以下の階層規格 |
| ME の離散更新 | `fmi2NewDiscreteStates` | `fmi3UpdateDiscreteStates` |

FMI 2 と 3 の違いは仕様の知識に基づく。仕様書（<https://fmi-standard.org/docs/3.0/>）と突き合わせて確かめること【未確認】。

## 検証に使う資料

- **Reference-FMUs**（BSD-2-Clause）: FMI 2 と 3 の BouncingBall、Dahlquist、Feedthrough、Resource、Stair、StateSpace、VanDerPol、Clocks。README には Roberts（DAE）も載っている。
- **FMPy**（BSD-2-Clause）: `validate_fmu` とシミュレーション。SUNDIALS（BSD-3）を同梱している。
- **FMI Library（FMIL）**（BSD-3-Clause）、**fmusdk**（BSD-2-Clause、2021 年にアーカイブ済み）: どちらも参照実装として扱う。

## 出典

- <https://modelica.org/news/release-modelica-specification-3.7/>
- <https://github.com/modelica/ModelicaStandardLibrary/releases>
- <https://fmi-standard.org/news/2024-11-27-fmi-2.0.5-and-3.0.2>
- <https://fmi-standard.org/docs/3.0/>
- <https://fmi-standard.org/news/>
- <https://github.com/modelica/fmi-standard/blob/main/LICENSE.txt>
- <https://github.com/modelica/Reference-FMUs>
- <https://ssp-standard.org/news/>
- <https://github.com/modelica/ssp-standard>
- <https://modelica.org/news/2025-01-08-ssp-20-released/>
- <https://github.com/modelica/ModelicaSpecification/tree/MCP/0031/RationaleMCP/0031>
- <https://www.efmi-standard.org/>
- <https://github.com/CATIA-Systems/FMPy>
- <https://github.com/modelon-community/fmi-library>
- <https://github.com/qtronic/fmusdk>

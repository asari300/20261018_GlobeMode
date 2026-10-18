# src/Modelica

GlobeMode の検証用の Modelica ライブラリ。**このディレクトリは MODELICAPATH の 1 項目として使える**。直下には Modelica のパッケージだけを置く。

- わざと構文を誤ったファイルは置かない。ほかの IDE がライブラリを開けなくなるため。
  - 構文の誤りのテストに使うファイルは `src/Rust/mod/globe-mode-frontend/tests/data/` に置く。
- MSL 4.1.0 は同梱せず、`cargo xtask fetch-msl` で `data/msl/` に取得する。

## パッケージ

| パッケージ | 構成 | 内容 |
| --- | --- | --- |
| `GlobeModeExamples` | 1 ファイル（`package.mo`） | MSL を使わない最小の例。`RC` は検査をすべて通り、`Unbalanced` は S5 で止まる |
| `GlobeModeValidation` | ディレクトリ（`package.mo` と `package.order`） | connect の型ごとの検証モデル（S004）。下の表 |

### GlobeModeValidation

```text
GlobeModeValidation/
├── package.mo          uses(Modelica(version="4.1.0"))
├── package.order       Connectors, Components, Connect, Index, Init, Invalid
├── Connectors.mo       Pin（Real の potential / flow）、RealInput / RealOutput、BooleanInput / BooleanOutput、FluidPort（Medium と stream）
├── Components.mo       抵抗・コンデンサ・電源・接地、Real と Boolean のブロック、流体の境界と抵抗
├── Connect/            connect の型ごとのテストモデル（1 モデル 1 ファイル）
├── Index/              微分インデックスが 1 より大きい正しいモデル（S7 の検証、S007）
├── Init/               初期化系の検証モデル（S8、S007）
└── Invalid/            わざと誤ったモデル
```

| モデル | 試す connect | connect グラフ（depth 1） | 検査（`globe check`） |
| --- | --- | --- | --- |
| `Connect.RealCircuit` | Real の potential / flow（電気のピン） | ノード 4・辺 4（flow） | OK。方程式 20 = 変数 20。S8 は警告（fixed が無いので GM-W8001 と、`capacitor.v` に start が無い GM-W8002） |
| `Connect.RealSignals` | Real の信号（output → input） | ノード 3・辺 2（real） | OK。5 = 5 |
| `Connect.BooleanSignals` | Boolean の信号と Real の混在 | ノード 4・辺 3（boolean 2、real 1） | OK。7 = 7（Boolean の離散 4）。S8 は柔らかい警告（GM-W8001「足りない見込み」。離散変数の分） |
| `Connect.MediumFlow` | Medium を持つ流体ポート（flow と stream、`inStream`） | ノード 3・辺 2（stream） | **MSL が要る**。`-L data/msl` で OK。12 = 12 |
| `Connect.Subsystem` | 外に 2 つのピンを出す部分回路 | — | OK |
| `Connect.Hierarchical` | 部分回路を 2 つ直列（depth のテスト） | depth 1 はノード 4・辺 4、depth 2 はノード 10・辺 10 | OK。40 = 40。S8 は警告（GM-W8001 と GM-W8002 × 2） |
| `Invalid.SignalTypeMismatch` | Real の出力を Boolean の入力に接続 | ノード 2・辺 1 | S4（GM-E4001）で止まる |
| `Invalid.StructurallySingular` | なし（x だけを含む方程式が 2 本あり、y と z を 1 本の方程式で決めている） | — | S6（GM-E6001）で止まる。方程式 3 = 変数 3 なので S5 は通る。omc 1.27.1 でも `checkModel` は 3 / 3 で通り、`translateModel` が「An independent subset of the model has imbalanced number of equations (1) and variables (2)」で失敗する（S008 で確認） |
| `Invalid.HighIndexDuplicate` | なし（RC 回路を式だけで書き、電圧則を 2 回書いてオームの法則を書き忘れた） | — | S6 は通る。S7 の情報 GM-I7001（インデックス 2）と GM-I7002。構造的には動的な状態が 0 なので `fixed = true` が 1 つ余り、S8 は警告 GM-W8004（インデックスの低減で状態が減るため確定できない）。OK（警告だけ）。omc 1.27.1 では `translateModel` が「Model is structurally singular, error found sorting equations」（`der(vC)` の dummy derivative）を出し、「The initial conditions are over specified」の警告も出る（S008 で確認） |
| `Index.Pendulum` | なし（直交座標の平面の振り子を式だけで書いた。状態 x, y, vx, vy と張力 lambda） | — | OK。方程式 5 = 変数 5。S7 の情報 GM-I7001（構造インデックス 3: 拘束 `x ^ 2 + y ^ 2 = L ^ 2` を 2 回、`der(x) = vx` と `der(y) = vy` を 1 回ずつ微分）と GM-I7002（状態選択の候補 x, y, vx, vy。動的な状態の見込み 2）。fixed は x と vx の 2 つ |
| `Index.ParallelCapacitors` | なし（電流源と 2 つのコンデンサを理想的に並列にした回路を式だけで書いた） | — | OK。方程式 5 = 変数 5。S7 の情報 GM-I7001（構造インデックス 2: `v1 = v2` を 1 回微分）と GM-I7002（状態選択の候補 v1, v2。動的な状態の見込み 1）。fixed は v1 だけ |
| `Init.Balanced` | なし（ばね・質量・ダンパを式だけで書いた。状態 x, v の 2 つ） | — | OK。方程式 2 = 変数 2。S8 は初期条件 2 / 2（x の `fixed = true` と `initial equation` の `v = 0`） |
| `Init.OverDetermined` | なし（1 次遅れ。`fixed = true` の状態 x に `initial equation` の `x = 2` も書いた） | — | S8（GM-E8001）で止まる。初期条件 2 / 要る数 1。位置は `x = 2`。方程式 1 = 変数 1 で S5〜S7 は通る |
| `Init.UnderDetermined` | なし（2 つのタンクの水位 h1, h2。fixed も `initial equation` も無く、h2 は start も無い） | — | OK（警告）。方程式 2 = 変数 2。S8 の GM-W8001（初期条件 0 / 2）と、h2 の宣言に GM-W8002 |
| `Init.ParameterWithoutValue` | なし（1 次遅れの時定数 `parameter Real T;` に値も start も無い） | — | OK（警告）。方程式 1 = 変数 1。T の宣言に S8 の GM-W8003。初期条件は 1 / 1 |
| `Init.FreeParameter` | なし（`parameter Real u0(fixed = false)` を、定常状態の `initial equation` の `der(x) = 0` で決める） | — | OK。方程式 1 = 変数 1。S8 は初期条件 2 / 2（状態 1 + `fixed = false` のパラメータ 1） |

## 使い方

```bash
# 検査（作業中のパッケージは、ディレクトリを渡して文書として読む）
cargo run -p globe-mode-cli -- check src/Modelica/GlobeModeValidation -m GlobeModeValidation.Connect.RealCircuit
cargo run -p globe-mode-cli -- check src/Modelica/GlobeModeValidation -L data/msl -m GlobeModeValidation.Connect.MediumFlow

# connect グラフ（--json は graphology の形式）
cargo run -p globe-mode-cli -- graph src/Modelica/GlobeModeValidation -m GlobeModeValidation.Connect.Hierarchical -d 2
```

- 作業中のパッケージを `--library` ではなく**文書**として渡すのには理由がある。
  - Rumoca v0.10.2 は、ライブラリとして読んだクラスから、別のライブラリ（MSL）のクラスを extends できない（S004 で確かめた）。
  - 文書として渡せば、`Modelica.Media` を使う `MediumFlow` も検査できる。

## ほかの Modelica の IDE で開く

- **OMEdit（OpenModelica）**:
  - 「File → Open Model/Library File(s)」で `src/Modelica/GlobeModeValidation/package.mo` を開く。
  - または、`src/Modelica` を MODELICAPATH に足して「Load Library」で `GlobeModeValidation` を選ぶ。
  - `uses(Modelica(version="4.1.0"))` があるので、MSL 4.1.0 も読み込まれる。
- **Dymola**: 「File → Open」で `GlobeModeValidation/package.mo` を開く。
- アイコンを付けるための `extends Modelica.Icons.*` は、わざと書いていない。書くと、Real や Boolean だけのモデルでも検査に MSL が要るようになるため。

## そのほかのファイル

| ファイル | 内容 |
| --- | --- |
| `msl-compare.txt` | `cargo xtask msl-compare` が Rumoca の CLI と突き合わせる MSL 4.1.0 の例題 20 本（S003） |

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
├── package.order       Connectors, Components, Connect, Invalid
├── Connectors.mo       Pin（Real の potential / flow）、RealInput / RealOutput、BooleanInput / BooleanOutput、FluidPort（Medium と stream）
├── Components.mo       抵抗・コンデンサ・電源・接地、Real と Boolean のブロック、流体の境界と抵抗
├── Connect/            connect の型ごとのテストモデル（1 モデル 1 ファイル）
└── Invalid/            わざと誤ったモデル
```

| モデル | 試す connect | connect グラフ（depth 1） | 検査（`globe check`） |
| --- | --- | --- | --- |
| `Connect.RealCircuit` | Real の potential / flow（電気のピン） | ノード 4・辺 4（flow） | OK。方程式 20 = 変数 20 |
| `Connect.RealSignals` | Real の信号（output → input） | ノード 3・辺 2（real） | OK。5 = 5 |
| `Connect.BooleanSignals` | Boolean の信号と Real の混在 | ノード 4・辺 3（boolean 2、real 1） | OK。7 = 7（Boolean の離散 4） |
| `Connect.MediumFlow` | Medium を持つ流体ポート（flow と stream、`inStream`） | ノード 3・辺 2（stream） | **MSL が要る**。`-L data/msl` で OK。12 = 12 |
| `Connect.Subsystem` | 外に 2 つのピンを出す部分回路 | — | OK |
| `Connect.Hierarchical` | 部分回路を 2 つ直列（depth のテスト） | depth 1 はノード 4・辺 4、depth 2 はノード 10・辺 10 | OK。40 = 40 |
| `Invalid.SignalTypeMismatch` | Real の出力を Boolean の入力に接続 | ノード 2・辺 1 | S4（GM-E4001）で止まる |

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

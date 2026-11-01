# MSL 以外の公開ライブラリの調査（S058）

- 確認日: 2026-11-01（GitHub の API とアーカイブの中身を読んだ）。
- 確認できなかった項目には【未確認】と書いた。
- 目的: `msl-funnel`（S0〜S10 の判定）と `msl-sim`（実行）を MSL 以外の公開ライブラリにも回す（`.github/workflows/libraries.yml`、[S058](../sessions/S058-extra-libs-ci/解説書.md)）。
- 取得は `cargo xtask fetch-library <名前>`（[src/Rust/xtask/src/libs.rs](../../src/Rust/xtask/src/libs.rs)）。GitHub の `archive/<commit>.zip` を commit で固定し、sha256 で確かめて `data/libs/<名前>/<パッケージ>/package.mo` の形に置く（コミットしない）。
- ライブラリは読むだけ（コードは複製もリンクもしない）ので、静的リンクの規則（[licenses.md](../licenses.md)）の対象外。ライセンスは表に書く。

## 一覧

| 名前 | 出典 | 版（commit） | アーカイブの sha256 | ライセンス | `uses` の MSL |
| --- | --- | --- | --- | --- | --- |
| Buildings | [lbl-srg/modelica-buildings](https://github.com/lbl-srg/modelica-buildings) | 14.0.0（tag v14.0.0、2026-10-05。`ad48324b74e9751e2e49dac5fe3c4dcb546755a2`） | `895e6498…9834` | 3-clause BSD（README） | **4.1.0** |
| AixLib | [RWTH-EBC/AixLib](https://github.com/RWTH-EBC/AixLib) | 3.0.1（tag v3.0.1、2026-06-23。`e5df9a8da27833ee391c2f7a0adbcd80278f47d2`） | `9067935c…7995` | 3-clause BSD（README） | **4.0.0** |
| IBPSA | [ibpsa/modelica-ibpsa](https://github.com/ibpsa/modelica-ibpsa) | 4.0.0（master の `2adc9194f762dbee92dc0f3d0d139e5e7dbdfc71`、2026-10-29） | `1ea5ae80…5a00` | 3-clause BSD（README） | **4.1.0** |
| ClaRa | [ClaRaLibrary/ClaRa](https://github.com/ClaRaLibrary/ClaRa) | 1.9.0（master の `573a02b1194e3eb2fb289e560b717f66de4b0f65`、2026-03-04。tag なし） | `78f9c5af…edf5` | 3-clause BSD（`UsersGuide/package.mo` の見出し） | **4.0.0** |
| TILMedia（ClaRa の依存） | [ClaRaLibrary/TILMedia](https://github.com/ClaRaLibrary/TILMedia) | `1.9.0 ClaRa`（master の `1f3380f91f8ab0d746da7c45590fe55f16f046f3`） | `d31661d9…bca0` | 【未確認】（LICENSE ファイルが無い） | - |
| ThermofluidStream | [DLR-SR/ThermofluidStream](https://github.com/DLR-SR/ThermofluidStream) | 1.4.1（tag v1.4.1、2026-10-19。`a2faa411bc7ac7de9b46c5c30b6850505cc05dac`） | `671c9fa0…d56c` | 3-clause BSD（`LICENSE`。GitHub の API は NOASSERTION） | **4.1.0** と Complex 4.1.0 |
| DeltaRobot | [looms-polimi/Parallel_manipulators](https://github.com/looms-polimi/Parallel_manipulators)（`Modelica_models/DeltaRobot`） | 1.0.2（master の `da73b5d43284db6941426b16ffadad11a6dfa965`、2023-03-10） | `7de61807…1ebd` | BSD-3-Clause（`LICENSE`） | **4.0.0** |
| Chemical | [MarekMatejak/Chemical](https://github.com/MarekMatejak/Chemical) | 2.0.0（tag v2.0.0、2025-09-06。`2da36279db8adf3effa142a8dfa270c40b0e517d`） | `bb4892eb…d709` | BSD-3-Clause（`LICENSE`） | **4.0.0** |

- sha256 の全文は `libs.rs` の表にある。
- 各アーカイブの最上位ディレクトリ名（`<repo>-<commit>/`）は捨て、パッケージのディレクトリだけを `data/libs/<名前>/<パッケージ>/` に置く。Buildings・AixLib・IBPSA・Chemical は `<repo>/<パッケージ>/package.mo`、ThermofluidStream は `ThermofluidStream/` と `TILMediaWrapper/`、DeltaRobot は `Modelica_models/DeltaRobot/`、ClaRa と TILMedia は **リポジトリの根がパッケージ**（根に `package.mo`）。
- release の zip がある Buildings（`Buildings-v14.0.0.zip`）も、ほかと同じ書き方にそろえるため、tag の commit のアーカイブを使う。IBPSA は最後の tag が v3.0.0（2018）で、MSL 4.1.0 を使う 4.0.0 は master にしか無い。ClaRa は tag も release も無い。この 2 つは commit で固定した。

## 依存

| ライブラリ | 依存 | 取得したか |
| --- | --- | --- |
| Buildings 14.0.0 | MSL 4.1.0 だけ（`uses`）。IBPSA は `uses` に無い（IBPSA のコードは Buildings の中に取り込まれていて、IBPSA を名前で参照するのは文書の中だけだった） | MSL のみ |
| AixLib 3.0.1 | `SDF` 0.4.4、`Modelica_DeviceDrivers` 2.1.1（`uses`）。IBPSA を名前で参照するのは文書だけ（AixLib は IBPSA を含まない） | **取得しない**。これらを使うモデルは名前が解決できず GM-E で止まる。数えるだけ |
| IBPSA | MSL 4.1.0 だけ | MSL のみ |
| ClaRa 1.9.0 | `TILMedia`（`1.9.0 ClaRa`）、`SMArtInt` 0.5.1、MSL 4.0.0 | TILMedia は取得する。SMArtInt は取得しない（[xrg-simulation/SMArtInt](https://github.com/xrg-simulation/SMArtInt) があるが、C のライブラリを要る外部関数の包みで、未確認【未確認】） |
| ThermofluidStream 1.4.1 | MSL 4.1.0、Complex。TILMediaWrapper はリポジトリに同梱（`TILMedia` 本体は無い。ThermofluidStream からの参照は 1 ファイル） | 同梱のものを取得 |
| DeltaRobot / Chemical | MSL 4.0.0 だけ | MSL のみ |

## MSL の版

GlobeMode は MSL 4.1.0 しか持たない。`uses` が 4.0.0 のライブラリ（AixLib・ClaRa・DeltaRobot・Chemical）も、まず 4.1.0 で回す。4.0.0 と 4.1.0 の差による失敗は、結果の報告の中で数える（別に直さない）。

## 例題の集め方

`msl-funnel` は MSL では「`Examples` を含む名前で、`Utilities`・`BaseClasses`・`Internal`・`Interfaces` を挟まない model / block / class」を例題とする（Rumoca の規則）。同じ規則を根のパッケージ名を変えて他のライブラリに当てた（`--package <根> --examples-in <段の名前>`）。

- Buildings・AixLib・IBPSA・ClaRa・ThermofluidStream・Chemical は `Examples`。
- DeltaRobot には `Examples` が無く、`Scenarios`（2 本ずつの完成した系）と `Tests`（部品の試験）に `experiment` 付きのモデルがある。この 2 つの段を例題とした（21 本）。

## Rumoca の読み込みの検査に通らないファイル

`Checker::new` は、ライブラリに構文の誤り（EP001）か配置の誤り（PKG-006: `package.mo` の無いディレクトリ、PKG-010: `within` とディレクトリ名が合わない）が 1 つでもあると、ライブラリ全体を断る（`GM-E1003`）。2026-11-01 に手元で見つけたもの:

| ライブラリ | 見つけたもの |
| --- | --- |
| ThermofluidStream 1.4.1 | `TILMediaWrapper/UsersGuide/Contact.mo`: `revisions` の文字列の中の `<img src="…"/>` の引用符が閉じていない（構文の誤り）。`ThermofluidStream/Processes/Pipes/Internal/BaseClasses`・`Interfaces`: `package.mo` の無いディレクトリ |
| ClaRa 1.9.0 | `.mo` 1483 ファイルが UTF-8 の BOM で始まり、BOM つきだと構文の誤りになって例題が 0 本。`Components/Mills/physicalMills/…` のファイルの `within` が `PhysicalMills`（大文字小文字が合わない。Windows の名前で書かれたものと思われる【未確認】） |
| AixLib 3.0.1 | BOM つきの `.mo` が 2 ファイル |

GlobeMode は次のように読む。`fetch-library` が `.mo` の BOM を外して置く（ほかは変えない）。`msl-funnel`・`msl-sim` は、MSL とライブラリを 1 つのルート（`target/library-roots/<名前>/`、symlink の木）にまとめ、1 ファイルずつ読んで構文の誤りのもの、`package.mo` の無いディレクトリ（`.mo` を含むもの）、診断（PKG-0xx）に出たファイルを除く。除いた一覧は `target/library-roots/<名前>.skipped.txt`。除いたクラスを使うモデルは名前が解決できず `GM-E1001` で止まる（数えるだけ）。

## 読んだ範囲と限界

- アーカイブの中の `package.mo` の `uses(...)` と `version`、README と LICENSE の見出しを読んだ。ライセンス全文の突き合わせまではしていない。
- Buildings・AixLib の Resources の中の C のソースやデータ（FMI Library など）は、取得はするが読まない（Modelica のソースだけを読む）。それぞれのライセンスは各ライブラリの文書に従う【未確認】。

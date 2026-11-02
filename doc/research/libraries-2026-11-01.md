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

GlobeMode は次のように読む。`fetch-library` が `.mo` の BOM を外して置く（ほかは変えない）。`msl-funnel`・`msl-sim` は、ライブラリの木を `target/library-roots/<名前>/`（symlink の木）に写し（MSL は別のルートとして渡す。S058 は MSL も同じ木に symlink していたが、S063 で `Checker::new` を直して外した）、1 ファイルずつ読んで構文の誤りのもの、`package.mo` の無いディレクトリ（`.mo` を含むもの）、診断（PKG-0xx）に出たファイルを除く。除いた一覧は `target/library-roots/<名前>.skipped.txt`。除いたクラスを使うモデルは名前が解決できず `GM-E1001` で止まる（数えるだけ）。

## ルートの間で名前が引けない原因（S063）

S058 は `Checker::new(&[msl, lib])` のようにルートを 2 つ渡すと、ライブラリの `import Modelica.Units.SI` が `GM-E1001`（ER002）になると見つけ、xtask で symlink の木に迂回した。S063 で原因を突き止めて直した（確認日 2026-11-01、Rumoca rev `276e82bd`、`~/.cargo/git/checkouts/rumoca-*/276e82b/crates/rumoca-compile/src/` のソースと実験）。

- **再現**: ルートを 2 つ渡した `Checker::new` で `Chemical.Obsolete.Examples.HeatingOfWater` が `unresolved import: 'Modelica.Units.SI'`（ER002）。順序を入れ替えても同じ。MSL を使わない最小の 2 ルート（`A/Units.mo` の型を `B/M.mo` が `import A.Units.T`）でも同じ（試験 `globe-mode-frontend/tests/multi_root.rs`）。
- **切り分け**（同じモデルを `compile_model_strict` で）: ルートごとに `load_source_root_tolerant`（ディスクを読む）して 2 組にする → 失敗。ファイルを読んで `replace_in_memory_source_set` で 2 組にする → **成功**。1 組にまとめる（メモリ・`parse_source_root_with_cache` + `replace_parsed_source_set` のどちらも）→ 成功。つまり組が 2 つあること自体ではなく、**ディスクのルートごとに呼ばれる `load_source_root_tolerant` の中身**が原因。
- **原因**: `load_source_root_tolerant` は文書を入れた後、`sync_source_root_semantic_summary_cache(source_set_id, root, …)` でそのルートだけの意味の要約（`SourceRootSemanticSummary`。宣言の索引・パッケージ定義の表・解決済みの集計）を作って Session に載せる（`session_impl_queries.rs:435`）。要約はそのルートの中だけで名前を引いた結果なので、別のルートの `Modelica.Units.SI` を import するライブラリのルートは未解決のまま載る。メモリから読む道（`replace_in_memory_source_set`）はこの要約を作らない。なお CLI・LSP・wasm は S054 から `Workspace::with_libraries` がファイルを 1 組のメモリの集合にして読むので、この問題は xtask と `Checker::new` を直接呼ぶ場合だけに出ていた（`globe check --library data/msl --library data/libs/Chemical` は S058 の前から通っていた）。
- **修正**: `Checker::new` は、ルートが 2 つ以上のときだけ、各ルートを `parse_source_root_with_cache`（構文木のキャッシュと 1 ルートごとのパッケージ配置の検査 PKG-0xx はそのまま）で読み、全部の文書を **1 つの組** `library:<a>,<b>` として `replace_parsed_source_set` で入れる。ルートが 1 つのときは従来の `load_source_root_tolerant`（要約あり）のまま。xtask は MSL の symlink をやめ、ミラーはライブラリだけにした（読めないファイルを除くため。`target/library-roots/<名前>/`）。
- **確かめたこと**: DeltaRobot の `msl-funnel`（21 本）は OK 9・NG 7・GM-U 5・panic 0 で、S058 の run（41248143483）と 21 本とも判定・段・コードが同じ。Chemical（76 本: OK 12・NG 21・GM-U 43）と ClaRa（5 本: NG 5）も、S058 の run の `*-funnel.json` と 76 本・5 本とも判定・段・コードが同じ（差 0。手元は debug ビルドで、Chemical の検査 892 s・最大 RSS 2.0 GiB）。
- **限界**: ルートが 1 つのときの要約（宣言の索引など）をルートが 2 つ以上のときは作らないので、そのぶんの初回の速さは失う（Session の問い合わせは文書から作り直す）。Rumoca 側（要約を別ルートの名前で解決し直す）は直していない。

## 失敗の分類（S063）

run 41250636800（Buildings 4・AixLib 2・IBPSA・ClaRa・ThermofluidStream）と 41248143483（Chemical・DeltaRobot）の artifact `library-<名前>-<i>` の `*-funnel.json` を集計した（2026-11-01。全 2,132 本、OK 351、失敗 1,781）。集計は `rumoca_code`（Rumoca の診断コード）とメッセージの形で分けた。GlobeMode のコード（`GM-E1001` など）は段で決まり、`GM-E1001`（S1）は「名前が解決できない」だけでなく Rumoca の解決の段の検査すべてを含む（ER070・ER132 など）。

| 区分 | 本数 |
| --- | ---: |
| NG S1 `GM-E1001` | 617 |
| NG S3 `GM-E3001` | 442 |
| GM-U S4 `GM-U0001`（未対応） | 232 |
| GM-U S4 `GM-U0002`（内部エラー） | 170 |
| GM-U S5 `GM-U0001` | 142 |
| GM-U S5 `GM-U0002` | 108 |
| NG S2 `GM-E2001` | 31 |
| NG S4 `GM-E4001` | 26 |
| panic | 8（Buildings 4・IBPSA 4。`inheritance must be validated before resolving components for eval: RedeclareConstraintViolation`） |
| NG S5 `GM-E5001` | 3 |
| GM-U S3 `GM-U0001` | 2 |

段ごとのライブラリ別:

| ライブラリ | 例題 | OK | NG S1 | NG S2 | NG S3 | NG S4 | NG S5 | GM-U | panic |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| AixLib | 642 | 119 | 225 | 8 | 124 | 2 | 1 | 163 | 0 |
| Buildings | 809 | 119 | 274 | 14 | 201 | 3 | 1 | 193 | 4 |
| Chemical | 76 | 12 | 0 | 0 | 0 | 21 | 0 | 43 | 0 |
| ClaRa | 5 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 0 |
| DeltaRobot | 21 | 9 | 0 | 0 | 7 | 0 | 0 | 5 | 0 |
| IBPSA | 457 | 88 | 113 | 7 | 109 | 0 | 1 | 135 | 4 |
| ThermofluidStream | 122 | 4 | 0 | 2 | 1 | 0 | 0 | 115 | 0 |

### ライブラリごとの上位 5 つの原因

「割合」は例題に対する割合。1 本の例題は 1 つの原因に数える（最初の診断）。

##### AixLib（例題 642・OK 119・NG 360・GM-U 163）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 85 | 13% | S1 | GM-E1001 | ER132 | ExternalObject の寿命の検査で基底の DefId が無い |
| 2 | 80 | 12% | S3 | GM-E3001 | EI032 | stateSelect の値が定数でない（`if … then StateSelect.prefer else …` / パラメータ） |
| 3 | 58 | 9% | S1 | GM-E1001 | ER070 | Evaluate 注釈が parameter / constant 以外の成分にある（MLS §18.6） |
| 4 | 51 | 8% | S4 | GM-U0002 | EF025 | 関数選択の同一性が決まらない（`Modelica.Utilities.Files.loadResource` / replaceable な Medium の関数） |
| 5 | 43 | 7% | S5 | GM-U0001 | ED019 | 関数の意味（文・条件分岐・形の証明）が未対応 |

上位 5 つで失敗 523 本のうち 317 本（61%）。

##### Buildings（例題 809・OK 119・NG 493・GM-U 197）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 173 | 21% | S1 | GM-E1001 | ER070 | Evaluate 注釈が parameter / constant 以外の成分にある（MLS §18.6） |
| 2 | 97 | 12% | S1 | GM-E1001 | ER132 | ExternalObject の寿命の検査で基底の DefId が無い |
| 3 | 87 | 11% | S3 | GM-E3001 | EI032 | stateSelect の値が定数でない（`if … then StateSelect.prefer else …` / パラメータ） |
| 4 | 54 | 7% | S3 | GM-E3001 | EI006 | 条件付き成分の条件がパラメータ式でない |
| 5 | 50 | 6% | S4 | GM-U0002 | EF025 | 関数選択の同一性が決まらない（`Modelica.Utilities.Files.loadResource` / replaceable な Medium の関数） |

上位 5 つで失敗 690 本のうち 461 本（67%）。

##### Chemical（例題 76・OK 12・NG 21・GM-U 43）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 41 | 54% | S4 | GM-U0001 | EF004 | if 方程式の枝で方程式の本数が違う |
| 2 | 21 | 28% | S4 | GM-E4001 | EF016 | 関数呼び出しの名前付き引数が入力に無い |
| 3 | 2 | 3% | S5 | GM-U0001 | ED020 | DAE の構築が拒んだ（式の形・関数値） |

上位 5 つで失敗 64 本のうち 64 本（100%）。

##### ClaRa（例題 5・OK 0・NG 5・GM-U 0）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 3 | 60% | S1 | GM-E1001 | ER023 | record の成分の型がクラス（ExternalObject。MLS §4.7） |
| 2 | 2 | 40% | S1 | GM-E1001 | ER003 | 基底クラスが見つからない |

上位 5 つで失敗 5 本のうち 5 本（100%）。

##### DeltaRobot（例題 21・OK 9・NG 7・GM-U 5）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 7 | 33% | S3 | GM-E3001 | EI032 | stateSelect の値が定数でない（`if … then StateSelect.prefer else …` / パラメータ） |
| 2 | 4 | 19% | S4 | GM-U0002 | EF015 | 関数値のクラス情報が無い（stateSelect の TypeId） |
| 3 | 1 | 5% | S5 | GM-U0001 | ED020 | DAE の構築が拒んだ（式の形・関数値） |

上位 5 つで失敗 12 本のうち 12 本（100%）。

##### IBPSA（例題 457・OK 88・NG 230・GM-U 139）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 85 | 19% | S1 | GM-E1001 | ER132 | ExternalObject の寿命の検査で基底の DefId が無い |
| 2 | 61 | 13% | S3 | GM-E3001 | EI032 | stateSelect の値が定数でない（`if … then StateSelect.prefer else …` / パラメータ） |
| 3 | 36 | 8% | S4 | GM-U0002 | EF025 | 関数選択の同一性が決まらない（`Modelica.Utilities.Files.loadResource` / replaceable な Medium の関数） |
| 4 | 33 | 7% | S5 | GM-U0001 | ED019 | 関数の意味（文・条件分岐・形の証明）が未対応 |
| 5 | 30 | 7% | S3 | GM-E3001 | EI006 | 条件付き成分の条件がパラメータ式でない |

上位 5 つで失敗 369 本のうち 245 本（66%）。

##### ThermofluidStream（例題 122・OK 4・NG 3・GM-U 115）

| 順位 | 本数 | 割合 | 段 | 区分 | Rumoca のコード | 原因 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 93 | 76% | S4 | GM-U0001 | EF004 | getInstanceName() が未対応 |
| 2 | 11 | 9% | S5 | GM-U0001 | ED019 | 関数の意味（文・条件分岐・形の証明）が未対応 |
| 3 | 10 | 8% | S5 | GM-U0002 | ED008 | flat の参照が解決できない |
| 4 | 2 | 2% | S2 | GM-E2001 | ET000 | 型の検査（コンテキスト・variability） |
| 5 | 1 | 1% | S3 | GM-E3001 | EI027 | redeclare が constrainedby に反する |

上位 5 つで失敗 118 本のうち 117 本（99%）。

### 原因の中身（3 つ以上のライブラリに共通するもの）

- **ER132（267 本、Buildings 97・AixLib 85・IBPSA 85）**: 267 本すべてがメッセージの名前が `product`。`IBPSA/Electrical/PhaseSystems/OnePhase.mo` の `redeclare function extends product`（`PartialPhaseSystem` の関数を継承して書き換える）を Rumoca の ExternalObject の寿命の検査が「解決済みの基底の DefId が無い」と断る。電気系の部品を 1 つでも使う例題が全部止まる。S070 で最小の再現（`product` を `prod` に変えると通る。`sum`・`max`・`min`・`abs`・`sin`・`ceil` でも同じ）を作り、判定した（確認日 2026-11-01）: **ライブラリの誤りでなく Rumoca の穴**。`extends product` が継承した関数でなく組み込みの `product` に引かれるらしい（MLS §7.3.1・§5.3.1。omc 1.27.2 は通す）。上流に出す下書きは書いた（出していない）。→ 上流 [17](upstream/17-er132-redeclare-function-extends-product.md)
- **ER070（258 本）**: 250 本が `Dzero`。`IBPSA/Controls/Continuous/LimPID.mo` の `Modelica.Blocks.Sources.Constant Dzero(final k=0) if not with_D annotation(Evaluate=true, …)`。Evaluate 注釈を parameter / constant 以外の成分（ブロックのインスタンス）に付けたのを Rumoca が MLS §18.6 で断る。PID を使う例題が全部止まる。S070 の判定: **Rumoca の穴**。MLS §18.3（Annotation 18.1）は Evaluate を「component declaration に付けてよい」「効くのは parameter だけ」とするだけで、誤りとは言わない（§18.6 は Graphical Objects で、Rumoca のメッセージの節番号も誤り）。関数の局所変数には同じ理由で警告（WR006）にしてあり、成分だけが硬いエラー。omc は通す。→ 上流 [18](upstream/18-er070-evaluate-on-non-parameter-component.md)
- **EI032（235 本）**: 228 本が `stateSelect = if medium.preferredMediumStates then StateSelect.prefer else StateSelect.default`（Media の体積）、7 本（DeltaRobot）が MSL `Mechanics/MultiBody/Joints/Revolute.mo` の `stateSelect` を親から渡す形。`stateSelect` の値を定数でないと断る（Rumoca の型属性の検査）。S070 の判定: **Rumoca の穴**。MLS §4.8.1 は `parameter StateSelect stateSelect` と宣言し、parameter 式なら何でもよい。兄弟成分の parameter を指す if 式（A）と、親から渡す parameter（B）を自己完結の 2 モデルで再現した。omc は通す。→ 上流 [19](upstream/19-ei032-statselect-parameter-expression.md)
- **EF025（138 本）**: 136 本が `Modelica.Utilities.Files.loadResource`（51 本）や replaceable な Medium の関数（`setState_pTX` など）の「正確な関数の選択」が決まらない。S070 の判定: `loadResource`（extends が 2 つの関数。MLS §7.1.1 で合法）は **Rumoca の穴**で、MSL だけと自己完結の 2 通りで再現した。replaceable な Medium の関数の方は最小の再現を作れなかった（単純な `Medium.setState_pTX` は通る）ので不明。omc は通す。→ 上流 [20](upstream/20-ef025-function-selection-identity.md)
- **EF004（217 本）**: ThermofluidStream の 93 本と Chemical の 41 本が上位。前者は `getInstanceName()`（全体で 119 本: ThermofluidStream 93・Buildings 11・IBPSA 8・AixLib 7）、後者は「if 方程式の枝で方程式の本数が違う」。S070 の判定: どちらも **Rumoca の穴**。`getInstanceName()` は parameter の束縛に書くと断る（equation の assert では通る。MLS §3.7.4.6）。後者の実体は「枝の方程式の本数」でなく、成分配列の項（`substrates.n_flow*ones(nS)`）を含む式の行数の数え違い（要素 1 個なら通り、2 個か 0 個で断る。MLS §10.6.4）。omc は通す。→ 上流 [21](upstream/21-ef004-getinstancename-and-if-equation-branches.md)

### 2 回目の判定（S072）

S070 の「次回」の 5 つのコードを同じ手順で判定した（確認日 2026-11-02。本数は run 41250636800 の artifact の `*-funnel.json` を数え直したもの）。最小の再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードになり、対照は両方で通る。omc 1.27.2 の `checkModel` は ER003 の再現だけを断る。詳しくは [upstream/README.md](upstream/README.md) の「公開ライブラリの失敗から（22〜26、S072）」。

| コード | 本数 | 実際の箇所 | 判定 |
| --- | ---: | --- | --- |
| ER023 | 3（ClaRa） | `ClaRa/Basics/Records/IComVLE_L2.mo` ほかの `TILMedia.Internals.TILMediaExternalObject fluidPointer_bulk` | **正当**。MLS 3.6 §4.6（[class-predefined-types-and-declarations](https://specification.modelica.org/maint/3.6/class-predefined-types-and-declarations.html)、2026-11-02）は record の成分を record か type に限る。ExternalObject は class。ClaRa の書き方は規定の外で、omc は緩く通す。Rumoca のメッセージの「§4.7」は 3.6 では §4.6。再現 [22](upstream/22-er023-record-component-externalobject.mo) |
| ER003 | 3（ClaRa 2・AixLib 1） | ClaRa の `SMArtInt.Internal.ClaRaDelay`、AixLib の `Modelica.Media.Examples.Tests.Components.PartialTestModel2`（`AixLib.Media.Refrigerants.Examples.RefrigerantTestImplementation`） | **正当**。ClaRa は取得しない依存（取得すれば避けられる）。AixLib はライブラリの誤り（MSL 4.0.0・4.1.0 とも `Modelica.Media.Examples.Utilities.PartialTestModel2` にある）。omc も同じ再現を断る。再現 [23](upstream/23-er003-base-class-not-found.mo) |
| EI006 | 117（Buildings 54・AixLib 33・IBPSA 30） | `IBPSA/Fluid/FMI/Interfaces/FluidProperties.mo` の `X_w if Medium.nXi > 0`（40 本）、`MassFlowSource_WeatherData` の `x_pTphi if (not singleSubstance)`（27 本） | **Rumoca の穴**。別の package の定数（直接でも、それを束縛にした parameter 経由でも）を読む条件を断る。モデルの定数・parameter なら通る。MLS §4.4.5 は parameter 式を求めるだけ。→ 上流 [24](upstream/24-ei006-condition-on-package-constant.md) |
| ED019 | 125（AixLib 43・Buildings 38・IBPSA 33・ThermofluidStream 11） | `IBPSA/Utilities/Math/Functions/isMonotonic.mo`（`function conditional`、同じメッセージ 44 本） | **Rumoca の未対応**。Boolean の出力の関数で、if の枝が代入のあと for を回すと断る（Real の出力なら通る）。→ 上流 [25](upstream/25-ed019-function-conditional-boolean-loop.md)。`function shape proof`（12 本。MSL 4.0.0 の `SensorVerification` は `Modelica/Media/package.mo` を指す）と `splineDerivatives` の形は最小にできず【未確認】 |
| ED008 | 108（Buildings 40・AixLib 29・IBPSA 29・ThermofluidStream 10） | `IBPSA/Media/Air.mo` の `constant GasProperties dryair(...)` を `dryair.cp` で読む（`<ライブラリ>.Media.Air.dryair.cp` が 47 本） | **Rumoca の穴**（内部エラー）。package の record 定数の成分が flat に無い。スカラの package 定数・モデルの record 定数なら通る。→ 上流 [26](upstream/26-ed008-field-of-package-record-constant.md)。ループ変数 `i` の 18 本は別の形で【未確認】 |

- ライブラリ側の誤りは ER003 の AixLib の 1 本と、規定の外の書き方の ER023（ClaRa 3 本）。S070 の 5 つと合わせると、10 のコードのうち 8 つが Rumoca の側。
- 実際の例題でも同じコードになることを確かめた: `IBPSA.Fluid.FMI.ExportContainers.Examples.FMUs.Fan`（EI006）、`IBPSA.Airflow.Multizone.BaseClasses.Examples.WindPressureProfile`（ED019）、`IBPSA.Airflow.Multizone.Examples.Orifice`（ED008）、`ClaRa.Examples.SteamCycle_01`（ER023）。

### MSL 4.0.0 と 4.1.0 の差・取得しない依存

- **取得しない依存を名指すもの**（メッセージに `Modelica_DeviceDrivers`・`SDF`・`SMArtInt` が出る失敗）: **AixLib 10 本**（`Modelica_DeviceDrivers.Communication.UDPSocket` の import 9、`TCPIPSocketClient` の型 1）、**ClaRa 2 本**（`SMArtInt.Internal.ClaRaDelay` が基底クラスに無い。`SteamPowerPlant_01`・`SteamPowerPlant_CombinedComponents_01`）。SDF は名指す失敗が 0 本。名前を直接出さずに、これらを使うクラスを使うために止まったものは分けられない【未確認】。
- **MSL 4.1.0 で消えた・移った名前が原因のもの**（`uses` が 4.0.0 の AixLib・ClaRa・DeltaRobot・Chemical で、失敗メッセージに `Modelica.` の名前が出るもの）: 確かめられたのは **AixLib 1 本**（`base class not found: Modelica.Media.Examples.Tests.Components.PartialTestModel2`。MSL 4.1.0 には無く、`UsersGuide/ReleaseNotes.mo` の記述だけに出る）。そのほかの `Modelica.` の名前が出る失敗は、4.1.0 にある関数の扱い（`Modelica.Utilities.Files.loadResource` 51 本、`fullPathName` 2、`Math.Vectors.sort` 1、`SingleGasesData.Air.R_s` 2 など、すべて Rumoca の内部の失敗）で、版の差ではない。名前を出さない失敗の中に版の差が混ざるかは、S067 で MSL 4.0.0 で回し直して確かめた（判定の差は 1 本だけ。下の「MSL 4.0.0 で回し直した差（S067）」）。
- **S1 の ER002（68 本、AixLib 67）**: `unresolved function call: 'specificEntropyOfpTX'` が 39 本（ライブラリの replaceable な Medium の関数）、成分の参照 14 本、import 10 本（9 本が Modelica_DeviceDrivers）。

### ClaRa（例題 5 本・OK 0）

`ClaRa.Examples` の 5 本はすべて S1 で止まる。**BOM・大文字小文字の迂回で読めていないファイルが原因ではない**。

| 本数 | 例題 | 原因 |
| ---: | --- | --- |
| 3 | `SteamCycle_01`・`SteamCycle_02`・`VapourCycle_01` | ER023: `record component 'fluidPointer_bulk' has type 'TILMedia.Internals.TILMediaExternalObject' which is a class, but only record or type components are allowed (MLS §4.7)`。ClaRa の `iCom` の record が TILMedia の ExternalObject の型（`class … extends ExternalObject`）を成分に持つ。MLS は record の成分を record か型に限るが、ClaRa（Dymola 向け）は書いている。Rumoca が断る |
| 2 | `SteamPowerPlant_01`・`SteamPowerPlant_CombinedComponents_01` | ER003: 基底クラス `SMArtInt.Internal.ClaRaDelay` が無い（取得しない依存 SMArtInt） |

- 読み飛ばしたファイルは 53 本（`target/library-roots/ClaRa.skipped.txt`）で、すべて `ClaRa/Components/Mills/physicalMills/` の下（`within …PhysicalMills`、PKG-010 で `physicalMills` ごと除いた）。ClaRa 全体の 2,044 ファイルの約 2.6%。`ClaRa.Examples` の 5 本はミルを使わない。BOM は `fetch-library` が外した後なので 0 本。
- 例題の数が 5 本なのは `Examples` を含む名前だけを例題とする規則のため。ClaRa の試験は `ClaRa.*.Check` などにあり、例題としては数えていない（`experiment` を持つモデルの数は調べていない【未確認】）。
- 先に直すと効くもの: ER023（Rumoca で record の成分の型を ExternalObject に緩める）が 3/5、SMArtInt の取得が 2/5。両方で S1 は抜けるが、その先（TILMedia の外部関数は実行系が未対応）で止まる見込み【未確認】。

## 読めるファイルだけでライブラリを読む（S066）

S058・S063 の持ち越し。`Checker::new` は、ライブラリの `.mo` が 1 つでも構文の誤りだと、またパッケージ配置の検査（PKG-0xx）に合わないと、ライブラリ全体を `GM-E1003` で断った（確認日 2026-11-01、Rumoca rev `276e82bd`、`~/.cargo/git/checkouts/rumoca-*/276e82b/crates/rumoca-compile/src/` のソース）。xtask は `mirror_libraries`（symlink の木で構文の誤りと `package.mo` の無いディレクトリを除く）と、診断から PKG-0xx のファイルを割り出して除いて読み直す最大 20 回の繰り返しで迂回していた。

- **Rumoca の側**: `parse_source_root_with_cache`（`source_root_cache.rs`）は、ファイル一覧（`collect_source_root_source_files`、非公開）を rayon で `collect::<Result<Vec<_>>>()` するので、1 ファイルの構文の誤りで全体が Err になる。続けて `validate_source_root_package_layout`（非公開）が PKG-006（`package.mo` の無いディレクトリ）・PKG-010（`within` の綴りが合わない）などでルート全体を Err にする。`Session::load_source_root_tolerant` の `tolerant` は「読み込み前の失敗を `diagnostics` で返す」だけで、ファイル単位では寛容でない。ファイル単位で寛容な公開 API は `rumoca_compile::parsing::parse_files_parallel_lenient(&[path])`（成功と失敗の組を別々に返す。構文木のキャッシュも使う）で、`Session::replace_parsed_source_set` は自前で組んだ `documents` を受け取る。Rumoca は改変しない。
- **採った方法**: `Checker::new` は従来の道（ルート 1 つは `load_source_root_tolerant`、2 つ以上は `parse_source_root_with_cache`）を先に試し、**失敗したルートだけ**、`.mo` を自前で集め（`.` で始まる名前は飛ばす）、`parse_files_parallel_lenient` で 1 ファイルずつ読む。読めたものを `replace_parsed_source_set` に渡し、読めなかったものは `Checker::skipped_library_files()`（パスと構文の誤りの最初の 1 件）と、検査の結果の診断（S1 の警告 `GM-W1001`、ルートごとに 1 件。ファイル名は先頭の 5 件を notes に）で知らせる。全部読めるルートの道は変わらない（MSL の初回の速さは変わらない。要約を作らない 2 ルート以上の制約も S063 のまま）。
- **配置の検査は飛ばす**: 配置の検査は非公開で呼べないので、失敗したルートは検査せずに読む。`package.mo` の無いディレクトリや `within` の綴りの合わないファイルも文書として入る（Rumoca の名前は `within` で決まるので解決には効かない）。構文は全部読めて配置だけが断っていたルートも、読んだうえで同じ警告（「配置の検査を飛ばして読んだ」、元の PKG-0xx を notes に）を出す。PKG-006 は Rumoca の仕様のままで、GlobeMode は断らず警告にする。
- **xtask**: `mirror_libraries`・`mirror_tree`・`link_entry`・`contains_mo`・`layout_offenders`（と繰り返し）・`libs::unparsable_files` を外した。`make_checker` は `Checker::new(&[msl, lib…])` を呼ぶだけで、読み飛ばしを `target/library-roots/<名前>.skipped.txt`（`パス<TAB>理由`。libraries.yml が artifact に残す）に書く。
- **試験**: `globe-mode-frontend/tests/tolerant_roots.rs`（fixture は `tests/data/tolerant/`）。ルート 1 つ・2 つのどちらでも、構文の誤りのファイルを 1 つ含むライブラリで `Checker::new` が成り立ち、警告が 1 件出て、残りのモデルが検査でき、読み飛ばしたクラスを使うモデルだけが失敗する。`package.mo` の無いディレクトリだけのルートは、読み飛ばしなしで警告が出る。

- **確かめたこと**（手元、release、2026-11-01）: 読み飛ばしが起きるのは S058 の `skipped.txt` が空でなかった **ClaRa（53 ファイル、PKG-010）と ThermofluidStream（構文の誤り 1 ファイル + `package.mo` の無いディレクトリ 2 つ）**。Chemical・DeltaRobot は一覧が空で、従来の道のまま（コードは通らない）。`msl-funnel` を S063 の run の `*-funnel.json`（41250636800）と比べて、**ThermofluidStream 122 本と ClaRa 5 本とも判定・段・GM コード・Rumoca コードが全部同じ（差 0）**。OK 4 / GM-U 115 / panic 0（ThermofluidStream）、OK 0・S1 で NG 5（ClaRa）も同じ。ClaRa は、元の 53 ファイルを除く代わりに全部読み（`physicalMills` の `within` の綴りの誤りは名前に効かない）、構文の誤りは 0 ファイルで「配置の検査を飛ばして読んだ」警告 1 件。ThermofluidStream は `Contact.mo` を読み飛ばした警告 1 件（`unexpected \`modelica\``）。時間は ThermofluidStream 134.2 秒（S063 の run は 135.3 秒）、ClaRa 6.5 秒（5.0 秒）で、変わらない。
- **MSL の初回の速さ**: MSL は従来の道で全部読めるので、失敗の道は通らない（`mise run msl:compare` は 20/20 一致）。失敗の道の費用は、ClaRa（2,044 ファイル）で追加の 1 回の構文解析（構文木のキャッシュが効く）程度で、検査の秒数に差は出なかった。
- **限界**: 失敗したルートでは配置の検査を飛ばすので、配置の誤り（PKG-0xx）は警告になる。`package.mo` の無いディレクトリのファイルは、これまで除いていたが、今は読む（ThermofluidStream で判定が変わらないことを確かめた）。
- Actions（`libraries.yml`、run 41295588356、Chemical・ClaRa・ThermofluidStream・DeltaRobot）の結果は成功（親が確認）。表の 4 行は S063（run 41250636800）と例題・OK・NG・GM-U・panic・実行できた のすべてが同じ: Chemical 76 / 12 / 21 / 43 / 0 / 0、ClaRa 5 / 0 / 5 / 0 / 0 / 0、DeltaRobot 21 / 9 / 7 / 5 / 0 / 5、ThermofluidStream 122 / 4 / 3 / 115 / 0 / 2。
## MSL 4.0.0 で回し直した差（S067）

S063 の持ち越し「`uses` が 4.0.0 のライブラリを 4.1.0 で回したことによる版の差の数」を測った（確認日 2026-11-01）。

### `uses(Modelica(version=…))`

各ライブラリの `package.mo` を、取得に使う commit の `raw.githubusercontent.com` から読んだ（`fetch-library` と同じ commit）。

| ライブラリ | `package.mo` の `uses` |
| --- | --- |
| Buildings 14.0.0 | `Modelica 4.1.0` |
| IBPSA 4.0.0 | `Modelica 4.1.0` |
| ThermofluidStream 1.4.1 | `Modelica 4.1.0`、`Complex 4.1.0` |
| TILMediaWrapper（ThermofluidStream に同梱） | `Modelica 4.1.0`、`ThermofluidStream 1.4.1`、`TILMedia 3.15.0` |
| AixLib 3.0.1 | `Modelica 4.0.0`、`SDF 0.4.4`、`Modelica_DeviceDrivers 2.1.1` |
| ClaRa 1.9.0 | `Modelica 4.0.0`、`TILMedia 1.9.0 ClaRa`、`SMArtInt 0.5.1` |
| TILMedia 1.9.0 ClaRa | `Modelica 4.0.0` |
| DeltaRobot 1.0.2 | `Modelica 4.0.0` |
| Chemical 2.0.0 | `Modelica 4.0.0` |

つまり 4.1.0 が前提なのは Buildings・IBPSA・ThermofluidStream、4.0.0 が前提なのは AixLib・ClaRa・DeltaRobot・Chemical。

### 取得物

- `ModelicaStandardLibrary_v4.0.0.zip`: <https://github.com/modelica/ModelicaStandardLibrary/releases/tag/v4.0.0> の資産（29,117,960 バイト、GitHub の API は digest を返さない）。2026-11-01 に取得して計算した sha256 は `98d8e785d3383699008282b01728c2dad794317076eb31d341456e9ed053a783`。`cargo xtask fetch-msl --version 4.0.0 --dest data/msl-4.0.0` で取れる（12,403 ファイル。`Modelica 4.0.0`・`ModelicaServices 4.0.0`・`ModelicaReference 4.0.0` のディレクトリ名は 4.1.0 の配布物と同じ並び）。
- `libraries.yml` は入力 `msl`（`4.1.0` 既定 / `4.0.0`）を取り、`fetch-msl --version` と `libraries-report --msl` に渡す。funnel の JSON の `msl_version` は xtask の定数（4.1.0）のままなので、版の表示は `--msl` だけが正。

### 4.0.0 で回した結果（run 41295445820）と 4.1.0（S058 の run）の差

同じ commit のライブラリ 7 つを、`libraries.yml -f msl=4.0.0`（run 41295445820、2026-11-01、全ジョブ success）で回し、4.1.0 の結果（run 41250636800 と 41248143483 の artifact の `*-funnel.json`・`*-sim.json`。表は [libraries.md](../libraries.md)）と例題ごとに突き合わせた。

| ライブラリ | 例題 4.1.0 / 4.0.0 | OK | NG | GM-U | panic | 判定の違う例題 | 実行できた（OK のうち） |
| --- | --- | --- | --- | --- | --- | ---: | --- |
| AixLib | 642 / 642 | 119 / 118 | 360 / 360 | 163 / 164 | 0 / 0 | 1 | 98 / 71（**-27**） |
| Buildings | 809 / 809 | 119 / 119 | 493 / 493 | 193 / 193 | 4 / 4 | 0 | 88 / 61（**-27**） |
| IBPSA | 457 / 457 | 88 / 88 | 230 / 230 | 135 / 135 | 4 / 4 | 0 | 66 / 47（**-19**） |
| DeltaRobot | 21 / 21 | 9 / 9 | 7 / 7 | 5 / 5 | 0 / 0 | 0 | 5 / 2（**-3**） |
| Chemical | 76 / 76 | 12 / 12 | 21 / 21 | 43 / 43 | 0 / 0 | 0 | 0 / 0 |
| ThermofluidStream | 122 / 122 | 4 / 4 | 3 / 3 | 115 / 115 | 0 / 0 | 0 | 2 / 2 |
| ClaRa | 5 / 5 | 0 / 0 | 5 / 5 | 0 / 0 | 0 / 0 | 0 | 0 / 0 |
| 計 | 2,132 / 2,132 | 351 / 350 | 1,119 / 1,119 | 654 / 655 | 8 / 8 | 1 | 259 / 183（**-76**） |

- **判定（S0〜S10）の差はほぼ無い**: 2,132 本のうち、判定・段・GlobeMode のコード・Rumoca のコードのどれかが違うのは **1 本**だけ。`AixLib.Obsolete.YearIndependent.FastHVAC.Examples.Sensors.SensorVerification` が OK（4.1.0）→ GM-U S5 の ED019（`function shape proof`: dependent extent is not an exact Integer expression over proven shape axes、4.0.0）になった。ほかの 2,131 本は両方の版で同じ段・同じコード（ER132・ER070・EI032・EF004・EF025 の本数も同じ）。つまり S063 の失敗の分類に MSL の版の差は混ざっていない（上の「失敗の分類（S063）」は 4.1.0 でも 4.0.0 でも同じ）。4.0.0 が前提の AixLib・ClaRa・DeltaRobot・Chemical を 4.1.0 で読んでも、4.1.0 が前提の Buildings・IBPSA・ThermofluidStream を 4.0.0 で読んでも同じ。
- **実行の差は 76 本（OK の例題が実行できなくなった）**: funnel が OK の例題のうち、4.1.0 で終わりまで実行できて 4.0.0 では `GM-U0004`（実行系が未対応）になったものが 75 本（Buildings 27・AixLib 26・IBPSA 19・DeltaRobot 3）と、判定が変わった AixLib の 1 本。逆（4.0.0 だけ実行できる）は 0 本。
  - **70 本は同じ原因**: `Modelica.Math.sin`（28）・`exp`（27）・`log`（10）・`cos`（4）・`asin`（1）。メッセージは `external builtin function 'Modelica.Math.log' calls 'log', which this runtime cannot execute`（EL005 / EX002）。MSL 4.0.0 の `Modelica/Math/package.mo` はこれらの関数を `external "builtin" y = log(u);` で書く（13 か所）。MSL 4.1.0 は同じ関数を `algorithm y := .log(u);` に書き直した（`external "builtin"` は 0 か所）。Rumoca の実行系は `external "builtin"` の呼び出しを実行できず、ライブラリ側が `Modelica.Math.*` を呼ぶ例題が 4.0.0 で全部止まる。**ライブラリではなく MSL の版の差で、実行系の穴**（4.0.0 の `external "builtin"` を実行系が解く道が無い）。funnel の段では見えない。
  - 残り 5 本: Buildings の `HeatTransfer.Convection.Examples.Interior`・`HeatTransfer.Windows.BaseClasses.Examples.InteriorConvection`（`invalid Solve IR contract: pure-call argument or slot interface is invalid`）と DeltaRobot の 3 本（`Disc_test`・`Functions_jointsCoordinates_actuatorsJointsRotationAngles_test`・`modelicaTransformations_test`。`Modelica.Mechanics.MultiBody.Frames.axesRotations` の record の戻り値を呼べない）。4.0.0 の MSL の関数の書き方（関数の中身）に依存する実行系の失敗と思われるが、MSL の差分は読んでいない【未確認】。
- 4.0.0 でも `external "builtin"` で止まるのに 4.1.0 でも止まっていたものが 23 本あり（別の原因が先に出ていた例題）、これは差に数えていない。
- **比較の土台の確認**: 4.1.0 の側は S058 の run なので、この branch の xtask・Rumoca で 4.1.0 を回し直した（run 41302566710、`-f msl=4.1.0`、Buildings を除く 6 つ、全ジョブ success）。S058 の run と 2,132 - 809 = 1,323 本すべてで判定・段・コードが同じで、実行できた本数も同じ（AixLib 98・IBPSA 66・DeltaRobot 5・ThermofluidStream 2・Chemical 0・ClaRa 0）。つまり上の差は S058 以降のコードの変化ではなく MSL の版の差。Buildings は回し直していない（S058 の 4.1.0 の run のまま。Buildings だけ分割 4 の 1 本が約 70 分かかるため）。
- 時間（funnel の足し算）は 4.1.0 / 4.0.0 で AixLib 732 / 772 s、Buildings 4,361 / 5,529 s、IBPSA 314 / 305 s と同程度（Buildings は分割 4 の 1 本が 3,769 / 4,748 s。Actions のランナーの差もある）。

## 読んだ範囲と限界

- アーカイブの中の `package.mo` の `uses(...)` と `version`、README と LICENSE の見出しを読んだ。ライセンス全文の突き合わせまではしていない。
- Buildings・AixLib の Resources の中の C のソースやデータ（FMI Library など）は、取得はするが読まない（Modelica のソースだけを読む）。それぞれのライセンスは各ライブラリの文書に従う【未確認】。

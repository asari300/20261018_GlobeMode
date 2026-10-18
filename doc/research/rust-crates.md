# Rust の crate 調査（FMI・数値計算・遅延評価・LSP）

- 確認日: 2026-10-18
- 情報源: crates.io API、GitHub の README と LICENSE、fmi-standard.org。
- 確認できなかった項目には【未確認】と書いた。

## FMI

jondo2010/rust-fmi（<https://github.com/jondo2010/rust-fmi>）。ライセンスは **MIT OR Apache-2.0**。star は約41で、1.0 前の小さなプロジェクト。

| crate | 版 | 内容 |
| --- | --- | --- |
| `fmi` | 0.8.0（2026-07-28） | FMI 2.0 / 3.0 の FMU の **Import**。`fmi-sys`、`fmi-schema`（XML）、`fmi-test-data`、`fmi-ls-bus` を同梱 |
| `fmi-sim` | 0.7.0 | 純 Rust の FMI シミュレータ。README では作業中とされている |
| `fmi-export` | 0.3.0（初公開 2026-02-01） | **FMI 3.0 の Export のみ**。`cdylib` と `#[derive(FmuModel)]` と `export_fmu!` で作る |
| `cargo-fmi` | — | `modelDescription.xml` を生成し、`.fmu` に zip する |

- `fmi-export` は既定で ME。CS と SE はフラグで有効にする。イベント指標とクロックに対応する。
  - 任意の `rumoca` feature を使うと `.mo` を Rust に変換できる。
- **FMI 2 を Export できる crate は無い**。GlobeMode が BSD-2 のヘッダの上に自前で作る（[architecture.md](../architecture.md#34-fmi-20--30)）。
- 「fmi-rs」「rust-fmi」という名前の別 crate は見つからなかった【未確認】。

## 数値計算（ODE / DAE）

| crate | 用途 | ライセンス | wasm | 成熟度 |
| --- | --- | --- | --- | --- |
| **diffsol** | BDF（NDF）、SDIRK / ESDIRK（TR-BDF2、ESDIRK34）、Tsit45。質量行列 DAE、根探索、順方向と随伴の感度 | MIT | JIT と KLU を使わなければ可能らしい【未確認】 | 0.17.1（2026-09-28）。活発 |
| diffsl | diffsol の DSL と JIT（既定は Cranelift、LLVM は任意） | MIT | ×（JIT のため） | 小規模。対応 LLVM の版が README ごとに食い違う【未確認】 |
| sundials-sys / sundials | SUNDIALS（CVODE / IDA / KINSOL）のバインディング | BSD-3-Clause | ×（emscripten のみ） | 0.6.2 / 0.4.1 |
| russell_ode | Radau5、DoPri5/8 | MIT。ただし **russell_sparse が GPL の UMFPACK を必ずリンクする** | × | 3.3.1。**使用禁止** |
| ode_solvers | 陽的 RK のみ。イベントは無い | Apache-2.0（古い版は BSD-3） | ○ | 0.6.2 |
| differential-equations | ODE / DAE / DDE / SDE、イベント | Apache-2.0 | おそらく可能【未確認】 | 0.7.0（2026-10-11） |
| peroxide | 科学計算。RK と GL4（BDF は無い） | MIT OR Apache-2.0 | ○ | 0.43.1 |
| **faer** | 密行列と疎行列の線形代数（純 Rust） | MIT | ○ | 0.24.4。成熟 |
| nalgebra | 密行列の線形代数 | Apache-2.0（古い版は BSD-3） | ○ | 0.35.0 |
| ndarray | n 次元配列 | MIT OR Apache-2.0 | ○ | 0.17.2 |
| **egg** | e-graph（記号的な簡約） | MIT | ○ | 0.11.0 |
| symbolica | 数式処理 | **OSI 承認ではない独自の source-available ライセンス**（実行時のライセンスが要り、改変物を再配布できない） | — | **使用禁止** |

### SuiteSparse のライセンス（成分ごとに違う）

| 成分 | ライセンス | GlobeMode での扱い |
| --- | --- | --- |
| AMD, CAMD, COLAMD, CCOLAMD | BSD-3-Clause | 可 |
| **KLU, BTF, LDL, CSparse / CXSparse** | LGPL-2.1-or-later | 同梱しない。必要なら任意の feature にし、利用者がビルドする |
| **UMFPACK, SPQR, CHOLMOD の Supernodal / Modify / MatrixOps** | GPL-2.0-or-later | **使用禁止** |

CHOLMOD Core のライセンス文は、読んだ版では空欄だった【未確認】。

## 遅延評価・構文木・LSP・JIT

| crate | 用途 | ライセンス | wasm | 備考 |
| --- | --- | --- | --- | --- |
| **salsa** | インクリメンタルなクエリ | Apache-2.0 OR MIT | ○（ty のプレイグラウンドで実績がある） | 0.28.5。rust-analyzer と ty が使う。wasm32 の `catch_unwind` の問題は解消済みらしい【未確認】 |
| **rowan** | 情報を落とさない CST | MIT OR Apache-2.0 | ○ | 0.17.0。Track B の自前パーサで使う |
| tree-sitter | C で書かれたインクリメンタルパーサ | MIT | ブラウザでは web-tree-sitter を使う | 0.27.1 |
| tree-sitter-modelica | Modelica の文法 | **AGPL-3.0 OR OSMC-PL-1.8** | — | **使用禁止**。Modelica 3.5 が対象で、まだ未成熟 |
| tower-lsp | LSP の枠組み | MIT OR Apache-2.0 | × | **2023-08 以降リリースが無い** |
| tower-lsp-server | tower-lsp のコミュニティ fork | MIT OR Apache-2.0 | 【未確認】 | 0.23 |
| lsp-server | rust-analyzer の LSP 通信層 | MIT OR Apache-2.0 | ×（stdio とスレッド） | 0.10.0 |
| lsp-types | LSP の型 | MIT | ○ | GlobeMode では Transport を抽象化して自前でディスパッチする |
| cranelift-codegen | JIT / AOT のコード生成 | Apache-2.0 WITH LLVM-exception | JIT は×（ブラウザでは実行可能メモリを確保できない） | 0.136.3 |
| inkwell | LLVM のバインディング | Apache-2.0 | × | 0.10.0 |

## 結論

- **コアは寛容なライセンスの crate だけで組む**: faer、diffsol、salsa、rowan、egg、lsp-types。
- LGPL（KLU）と GPL（UMFPACK）は任意の feature の外に出す。russell_sparse は cargo-deny で禁止する。
- symbolica と tree-sitter-modelica は使わない。
- ブラウザでは JIT が使えないので、解釈実行か wasm へのコード生成で動かす。

## 出典

- <https://github.com/jondo2010/rust-fmi>
- <https://docs.rs/fmi-export>
- <https://docs.rs/cargo-fmi>
- <https://github.com/martinjrobins/diffsol>
- <https://github.com/martinjrobins/diffsl>
- <https://github.com/DrTimothyAldenDavis/SuiteSparse/blob/dev/LICENSE.txt>
- <https://github.com/benruijl/symbolica/blob/master/License.md>
- <https://github.com/OpenModelica/tree-sitter-modelica>
- crates.io API（`https://crates.io/api/v1/crates/<name>`）で各 crate の版とライセンスを確認した

# Rumoca 調査

- 確認日: 2026-10-18
- 確認できなかった項目には【未確認】と書いた。
- 結論: GlobeMode の Phase A では、Rumoca を**コンパイラのフロントエンドとして git 依存で使う**（[ADR-0002](../adr/0002-rumoca-adapter.md)）。

## 概要

| 項目 | 内容 |
| --- | --- |
| リポジトリ | <https://github.com/CogniPilot/rumoca>（中心的なコミッタは jgoppert） |
| ライセンス | **Apache-2.0**（ルートの `LICENSE` を確認）。**NOTICE ファイルは無い** |
| 版 | 最新の GitHub Release は v0.10.2（2026-10-04）。main のワークスペースは 0.10.3 |
| crates.io | **0.7.28（2025-12-27）で止まっている**。0.8 以降は GitHub Releases（Python wheel と `rumoca` / `rumoca-lsp` のバイナリ）だけで配布 |
| 活動 | 非常に活発。2026-10-08〜16 の間に約15コミット。star は約140 |
| ツールチェーン | `rust-toolchain.toml` が `nightly-2026-02-27` を固定している（wasm のスレッド対応、wasm-bindgen-rayon のため）。target は `wasm32-unknown-unknown` と `wasm32-wasip2` |
| 論文 | arXiv 2606.14998（2026-06）、Modelica Conference の論文（<https://ecp.ep.liu.se/index.php/modelica/article/view/1388>） |

## ワークスペースの構成（約57 crate）

```text
IR:        rumoca-ir-ast → rumoca-ir-flat → rumoca-ir-dae → rumoca-ir-solve  (+ ir-galec)
フェーズ:  phase-parse / resolve / typecheck / instantiate / flatten / dae /
           structural / solve / autodiff / codegen
集約:      rumoca-compile（純粋なパイプライン。CLI や通信は含まない）
シミュ:    rumoca-sim, rumoca-solver(-diffsol | -rk45), exec-cranelift / exec-mlir / exec-wasm
ツール:    tool-lsp（tower-lsp 0.20）, tool-fmt, tool-lint
バインド:  bind-python（pyo3 0.22）, bind-wasm
実行時:    transport-udp / websocket / zenoh, input-gamepad / keyboard
テスト:    rumoca-test-msl, rumoca-contracts（仕様テスト）
```

- **パーサ**: `parol =4.2.2` と `scnr2`。打鍵ごとに走らせられる程度にはエラーに強いとされる。
  - 文法が LL(k) か LALR(1) かは【未確認】。
- **パイプライン**: parse → resolve（安定した `DefId`）→ typecheck → instantiate → flatten → DAE → 構造解析 → Solve IR。
  - instantiate では modifier、extends、**redeclare**、条件付きコンポーネントを処理する。
  - 構造解析ではマッチング、BLT/Tarjan、Pantelides、tearing、初期化の計画を行う。
- **インクリメンタル計算**: salsa のようなクエリ基盤は**見当たらない**。GlobeMode がここを補う（[ADR-0005](../adr/0005-lazy-query-db.md)）。
- **コード生成**: MiniJinja のテンプレート。`target.toml` と Jinja で独自ターゲットを足せる。組み込みのターゲットは次のとおり。
  - base-modelica, flat-modelica, dae-modelica
  - c-ode, casadi-ode, jax-ode, cuda-ode, wgsl-ode, rust-ode, rust-fixed-ode, mlir
  - **fmi2, fmi3**, fmi-ls-wasm
  - galec 系
- **シミュレーション**: ソルバは `auto` / `bdf`（diffsol 0.13、MIT）/ `rk-like`。
  - 対応: ゼロ交差の検出、`sample` の正確なスケジューリング、AD による Jacobian、感度解析と随伴、A/B/C/D 線形化。
  - イベントの多いモデルでは止まることがある。

## 言語の対応状況（MSL 4.1.0 の例題。2026-06-05 時点）

| 段階 | 通過したモデル数 |
| --- | --- |
| parse | 566 |
| flatten | 562 |
| DAE | 477 |
| balanced | 464 |
| 初期化に成功 | 224 |
| シミュレーションに成功 | 153 |
| OMC と高い一致 | 比較した 143 本のうち 89 本 |

- **対応済み**: extends、modifier、connect/flow、配列、関数、when/pre/edge/reinit/sample。
- **0.7.28 時点で未実装**だったもの: stream / expandable / overconstrained connector、演算子オーバーロード、state machine。
  - **S003・S004 で確かめたこと（v0.10.2）**: stream connector は使える。MSL の `Fluid.Examples.Tanks.EmptyTanks` と、自作の流体ポート（`inStream`）のモデルが DAE まで通った。
  - 現在の状況は【未確認】。v0.10.1 でも Fluid は作業中だった。
- GlobeMode は、これらを「コンパイラが未対応」の **GM-U コード**として「モデルの誤り」と分けて報告する（[architecture.md](../architecture.md#32-遅延評価とシミュレーション前のアサーション)）。

## S004 で見つけた制約（v0.10.2）

- ライブラリ（`--source-root`）として読んだクラスが、別のライブラリのクラスを extends すると、ER003（base class not found）になる。
  - 例: ライブラリの中の `replaceable package Medium = Modelica.Media.Interfaces.PartialMedium`。
  - コンパイルする文書の中に書けば解決できる。
- GlobeMode は、作業中のパッケージを文書として渡して回避している（[ADR-0002](../adr/0002-rumoca-adapter.md)）。

## S005 で調べた API（v0.10.2）

- 確認日: 2026-10-18。
- 読んだのは、GlobeMode が固定している rev `276e82bd` のソース。cargo の git の取得物（`~/.cargo/git/checkouts/rumoca-*/276e82b/`）を読んだ。
  - 同じ rev は GitHub の <https://github.com/CogniPilot/rumoca/tree/276e82bd0fd48960ac201e782918d28bf19cf7ce> にある。
  - 下の表のパスは、リポジトリのルートからのもの。
- S005 の設計（[ADR-0005](../adr/0005-lazy-query-db.md) の結果、[ADR-0007](../adr/0007-unsupported-classification.md)）は、この調査に基づく。

### Session（文書とキャッシュ）

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 複製 | `Session` は `#[derive(Debug, Clone)]`。GlobeMode の `Checker` は、ライブラリを読んだ直後の写しを取っておき、panic の後や `reset()` で戻す | `crates/rumoca-compile/src/session.rs`（`pub struct Session`） |
| 文書 | `add_document(uri, text)` は追加と差し替え（parse に失敗したら Err）。`update_document` は parse に失敗しても文書を残す（LSP 向け）。`remove_document(uri)` で消す。いずれも `&mut self` なので、共有するなら `Mutex` が要る | `crates/rumoca-compile/src/session/session_impl.rs` |
| 無効化 | 文書を足す・替えると `invalidate_resolved_state(DocumentMutation)` が走り、resolve 済みの状態を捨てる（MSL も resolve し直す）。parse の結果は残る | 同上 |
| キャッシュ | モデルごとの LRU。コンパイルの結果は 256 件、意味の診断・ナビゲーション・モデルの問い合わせは 64 件。鍵は依存の fingerprint（blake3） | `crates/rumoca-compile/src/session.rs`（`MAX_SESSION_COMPILE_CACHE_ENTRIES` など）、`crates/rumoca-compile/src/session/dependency_fingerprint.rs` |
| flat | `compile_model_flat_strict_reachable_uncached_with_recovery(name)` は、ToDae で失敗するモデルでも flat モデルを返す | `crates/rumoca-compile/src/session/session_impl.rs` |

### DAE の構造と構造解析

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| incidence | **公開されていない**。`build_incidence` は `pub(crate)`。GlobeMode は `rumoca_eval_dae::for_each_scalar_coordinate`（公開）で、方程式ごとに参照するスカラーの座標を列挙して自前で作る | `crates/rumoca-phase-structural/src/incidence.rs`、`crates/rumoca-eval-dae/src/projection.rs` |
| 構造解析 | `phase_structural::analyze(view)` は公開。`StructuralDiagnostics{diagnostics, matching_size, n_equations, n_unknowns, unmatched_unknowns, unmatched_equations, algebraic_loops}` を返す | `crates/rumoca-phase-structural/src/lib.rs`、`src/diagnostics.rs` |
| マッチング | ファイル冒頭の説明は「増加路（Kuhn 法）」だが、`grow_maximum_matching` は Hopcroft–Karp の段階（幅優先の層分けと深さ優先の増加）で探す。過剰・不足を分ける Dulmage–Mendelsohn 分解は無い | `crates/rumoca-phase-structural/src/matching.rs` |
| 未知数の約束 | 状態ごとの `der(x)` と、代数・出力のスカラー。状態・パラメータ・入力・離散変数・`pre()`・`time` は既知（der の形）。GlobeMode の `DaeStructure` も同じにした | `crates/rumoca-phase-structural/src/incidence.rs`、`src/incidence/projection.rs` |
| 釣り合い | `balance_detail`（`BalanceDetail`）: `state_unknowns`・`algebraic_unknowns`・`output_unknowns`・`discrete_real_unknowns`・`discrete_value_unknowns`・`continuous_equations`・`discrete_real_equations`・`discrete_value_definitions` | `crates/rumoca-phase-dae/src/balance.rs` |
| 部品ごとの釣り合い | 部品ごと（MLS 4.7 のローカルバランス）の数は見当たらない。GlobeMode は flat モデルの方程式の由来から、部品ごとの方程式数を数える | `crates/` の `.rs` を `local balance` などで検索して 0 件 |
| flat モデル | `FlatModel`（`rumoca_ir_flat::Model`）は公開。方程式ごとに `EquationOrigin`（`ComponentEquation{component}`、`Connection`、`FlowSum`、`UnconnectedFlow` など）を持つ | `crates/rumoca-ir-flat/src/lib.rs` |
| 位置 | 方程式・変数・式に `provenance().span()` があり、`source_span_location` でファイルと範囲に直せる | `crates/rumoca-compile/src/lib.rs` |
| annotation | AST の `ClassDef` などは `annotation: Vec<Expression>` を持つ（experiment annotation を読むときに使える） | `crates/rumoca-ir-ast/src/nodes.rs` |

### エラーコード

- コードの範囲は SPEC_0008 にある。一覧表（コードごとの意味の表）は無く、phase ごとの `errors.rs` などに散らばっている。
  - 出典: `spec/SPEC_0008_PHASE_ERRORS.md`、`crates/rumoca-phase-*/src/errors.rs`。
  - そのほかの置き場: `crates/rumoca-phase-resolve/src/semantic_checks/builtin_calls.rs`（ER073）、`crates/rumoca-compile/src/session/strict_compile_diagnostics.rs`（EI000）、`crates/rumoca-phase-structural/src/diagnostic_codes.rs`（ES014）。

  | 範囲 | phase |
  | --- | --- |
  | EP0xx | parse |
  | ER0xx | resolve |
  | ET0xx / WT0xx | typecheck |
  | EI0xx / WI0xx | instantiate |
  | EF0xx | flatten |
  | ED0xx | ToDAE |
  | EC0xx | codegen |
  | EM0xx | class merge |
  | ES0xx | structural |
  | EL0xx | solve lowering |
  | EX0xx / WX0xx | 実行時 |

- 名前とコメントから「未対応の言語機能・実装の制限」と読めるもの（ED019 など）と、「コンパイラの内部の不変条件が崩れた」と読めるもの（ED008 など）がある。GlobeMode の振り分けの表は [ADR-0007](../adr/0007-unsupported-classification.md) と `globe-mode-frontend/src/unsupported.rs`。
- SPEC_0008 は、内部の不変条件の崩れは `panic!` / `expect` にすると定めている。GlobeMode はモデルごとに `catch_unwind` で囲む。

### MSL のハーネス（例題の選び方と基準値）

- **例題は名前の規則で選ぶ**（`ROOT_MSL_EXAMPLE_SELECTION_PATTERN = "Modelica.*.Examples.*"`）。
  - 出典: `crates/rumoca-test-msl/tests/balance_pipeline/balance_pipeline_example_targets.rs`。
  - `Examples` の下の途中のパッケージに `Utilities`・`BaseClasses`・`Internal`・`Interfaces`（`EXAMPLE_SUPPORT_SEGMENTS`）があれば除く。途中はすべて package。
  - 「partial でない」は、DAE の検査に回すモデルを選ぶ別の関数（`is_root_standalone_msl_example_dae_model`）の条件。名前の規則で選んだ 566 本には、partial のモデルが 13 本含まれる。
- **基準値**: `crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json`（rev `276e82bd` に同梱）。

  | 項目 | 値 |
  | --- | ---: |
  | 例題（`simulatable_attempted`） | 566 |
  | parse | 566 |
  | flatten | 494 |
  | DAE | 305 |
  | balanced | 293 |
  | simulate に成功 | 216 |
  | partial（`partial_models`） | 13 |

  - 記録した commit は `b92932c0`（`git_commit`）で、固定している rev とは違う。`msl_version` は v4.1.0、`quality_gate_version` は 11。
  - 上の「言語の対応状況」の値（2026-06-05 時点、flatten 562 / DAE 477 / balanced 464）とは数え方か版が違う。どちらが今の Rumoca に当てはまるかは【未確認】。
  - モデルごとの結果の一覧は、リポジトリに入っていない（ハーネスの実行時に作る）。
- GlobeMode の `cargo xtask msl-funnel` は同じ名前の規則で選び、**566 本（うち partial 13 本）で一致した**（[報告](../sessions/S005-check-queries/msl-funnel.md)）。

## 依存として取り込むときの注意

`rumoca`（umbrella crate）を**使わない**。理由はデフォルト feature にある。

```toml
default = ["scheduled-sim", "scenario-config", "input-keyboard", "input-gamepad",
           "transport-udp", "transport-zenoh", "viewer-web", "process-control",
           "native-allocator", "fmu-packaging", "component-build"]
```

- `transport-zenoh` は zenoh（EPL-2.0 OR Apache-2.0）を引き込む。gamepad・UDP・web viewer も IDE のコアには要らない。
- 依存してよいのは次の crate だけ。
  - `rumoca-compile`
  - `rumoca-phase-codegen`
  - 必要な feature だけを有効にした `rumoca-sim`
  - `rumoca-solver-diffsol`
- ワークスペース側の主な外部依存とライセンス:
  - faer 0.24（MIT）、minijinja（Apache-2.0）、parol（MIT OR Apache-2.0）、diffsol 0.13（MIT）
  - cranelift 0.125（Apache-2.0 WITH LLVM-exception）、tower-lsp 0.20、pyo3 0.22、wasm-bindgen
- 推移的依存を含めた確認は P1 で `cargo deny check` を回して行う（[licenses.md](../licenses.md)）。

## 参考にする設計

- IR を層に分け、フェーズを crate ごとに分ける構成（GlobeMode の中立 IR の境界の参考にする）
- 構造解析（マッチング・BLT・Pantelides・tearing）の実装
- テンプレートでコード生成する方式と `target.toml`
- LSP と WASM プレイグラウンドの構成、MSL のファネル計測（`rumoca-test-msl`）

## 出典

- <https://github.com/CogniPilot/rumoca>
- <https://github.com/CogniPilot/rumoca/releases>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/Cargo.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/rust-toolchain.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/crates/rumoca/Cargo.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/user-guide/src/simulation/solvers.md>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/user-guide/src/language/support-status.md>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/dev-guide/src/tooling/msl-baseline-promotion-analysis-2026-06-05.md>
- <https://docs.rs/crate/rumoca/latest>
- <https://crates.io/api/v1/crates/rumoca/versions>
- <https://github.com/CogniPilot/rumoca/tree/276e82bd0fd48960ac201e782918d28bf19cf7ce>（S005。rev `276e82bd` のソース）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/spec/SPEC_0008_PHASE_ERRORS.md>
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-phase-structural/src/incidence.rs>
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json>

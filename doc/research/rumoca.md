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

## S007 で調べた API（v0.10.2）

- 確認日: 2026-10-18。
- 読んだのは S005 と同じ rev `276e82bd` のソース（cargo の git の取得物。GitHub では <https://github.com/CogniPilot/rumoca/tree/276e82bd0fd48960ac201e782918d28bf19cf7ce>）。下の表のパスは、リポジトリのルートからのもの。
- 「出典」が GlobeMode のテストのものは、ソースを読んだのではなく、Rumoca に小さなモデルを通して確かめた事実（テストは `src/Rust/mod/globe-mode-frontend/src/init.rs` の単体テストと `tests/init.rs`）。
- S007 の設計（[ADR-0005](../adr/0005-lazy-query-db.md) の結果の `deps_closure`、[ADR-0009](../adr/0009-structural-index-and-init.md)）は、この調査に基づく。

### import の AST

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 型 | `rumoca_ir_ast::Import` は 4 つの形の enum: `Qualified { path }`（`import A.B.C;`）、`Renamed { alias, path }`（`import D = A.B.C;`）、`Unqualified { path }`（`import A.B.*;`）、`Selective { path, names }`（`import A.B.{C, D};`）。どれも `location` と、先頭の `.`（`import .A.B.C;`）を表す `global_scope` を持つ | `crates/rumoca-ir-ast/src/nodes.rs`（`pub enum Import`） |
| 置き場 | `ClassDef` の `imports: Vec<Import>` | 同上（`pub struct ClassDef`） |
| GlobeMode の扱い | アウトラインの `ClassOutline.imports` に文字列で写す（`A.B.C`、`A.B.*`、`A.B.C` と `A.B.D`）。別名（`D =`）は落とす | `src/Rust/mod/globe-mode-frontend/src/rumoca.rs`（`import_paths`） |

### 変数の属性（`VariableView`）

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 取り出し | `start()` と `binding()` は `Option<ExprId>`、`fixed()` は `Option<&[bool]>` を返す | `crates/rumoca-ir-dae/src/model/view.rs`（`impl VariableView`） |
| `fixed_scalar(i)` | スカラー 1 つの `fixed`。値が 1 つなら全要素に行き渡らせ、配列なら要素の位置で引く（MLS §4.8、§4.8.6）。属性が無ければ None（GlobeMode は MLS の既定を使う: 状態・離散・代数は false、パラメータは true） | 同上 |
| 既定の start の補完 | DAE を組むとき、start の無い変数のうち、状態・代数・出力・離散 Real・離散値と、`fixed = false` のパラメータには、既定の start の式を補う（`needs_default_start`、`default_start_expression`） | `crates/rumoca-phase-dae/src/construction/variable_construction.rs` |
| 補った式の由来 | `DaeGeneration::DefaultStart`（説明は "language-defined default start"） | `crates/rumoca-ir-dae/src/provenance.rs` |
| `start()` はほぼ常に Some | 何も書かなくても Some になる。組み込みの型（`Real x;`）では、式の位置が宣言そのものと同じ。start の無い型（`type V = Real(unit = "V")`）では、由来が `DefaultStart`。GlobeMode は、このどちらでもない start を「明示された start」（`has_start`）とする | GlobeMode のテスト（`src/Rust/mod/globe-mode-frontend/src/init.rs` の単体テストと `explicit_start`、`tests/init.rs`） |
| 明示とみなすもの | 型から受け継いだ start（`type V = Real(start = 1)`）、式の start（`start = p`）、外からの修正（`Sub s(x(start = 2))`）。束縛のあるパラメータ（`parameter Real p = 2`）は、束縛が start にも写される（MLS §4.4.4） | 同上 |

### 初期化系

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 初期方程式 | `DaeView::initialization_owners()` が、初期方程式を Residual（1 行）か Structured（族）として列挙する | `crates/rumoca-ir-dae/src/model/view.rs` |
| `fixed = true` の start | ToDae（`compile_model_strict`）は、`fixed = true` の start を初期方程式に**変えない**（`initialization_owners()` に現れない）。S8 は `fixed = true` の状態を初期方程式とは別に数えればよく、2 重には数えない | GlobeMode のテスト（`src/Rust/mod/globe-mode-frontend/src/init.rs` の `fixed_start_is_not_turned_into_an_initial_equation`、`tests/init.rs` の `fixed_start_alone_adds_no_initial_equation`） |
| パラメータの初期値 | `DaeView::initial_parameter_values()`: 初期化で決めるパラメータの定義。Real でない `fixed = false` のパラメータを決める式は、`initialization_owners()` ではなくここに入る | `crates/rumoca-ir-dae/src/model/initial_parameters.rs`、GlobeMode のテスト |
| 離散変数の初期値 | `DaeView::initial_discrete_values()`: 離散変数の、初期化の時点の値（MLS §8.6）。定数で決まる離散値と、初期アルゴリズムの代入はここに入り、`initialization_owners()` には入らない（連続の変数を読む離散 Real の式は入る） | `crates/rumoca-ir-dae/src/model/view/arena_walks.rs`、GlobeMode のテスト |
| 初期アルゴリズムのパラメータ | 初期アルゴリズムで決める `fixed = false` のパラメータ（`p := 2`）は、束縛のあるパラメータになる | GlobeMode のテスト |

### 文書の組が変わったときの費用

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 無効化 | 文書の追加・差し替え・削除と、ソースの集合の変更で、`invalidate_resolved_state` が resolve 済みの状態（`query_state.resolved`）をすべて捨てる。S005 の表の「無効化」の行の関数は、定義がこのファイルにある | `crates/rumoca-compile/src/session/session_impl_queries.rs`（定義）、`session_impl.rs`・`session_impl_inputs.rs`（呼び出し） |
| 費用 | MSL を読んだ Session で、渡す文書の組が変わると、次の検査で MSL ごと解決し直す。約 1.5〜1.8 s（release。閉包の違うモデルを初めて検査したとき中央値 1700 ms、文書の組を変えずに同じモデルを検査したとき 67 ms） | GlobeMode の計測（`src/Rust/mod/globe-mode-db/tests/deps.rs` の `measure_checks_with_msl`。[ADR-0005](../adr/0005-lazy-query-db.md) の結果） |

## S009 で調べた API（v0.10.2）

初期方程式の参照（S8 のマッチングの入力、[ADR-0011](../adr/0011-init-system-matching.md)）に使ったもの。確認日 2026-10-19、rev は `Cargo.toml` のもの。

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 初期方程式の式 | `InitializationOwnerView::Residual { equation: ResidualEquationView, .. }` の `equation.residual()` と、`Structured { family: StructuredFamilyView, .. }` の `family.bodies()` は、連続の方程式（`ContinuousOwnerView`）と同じ型。`rumoca_eval_dae::for_each_scalar_coordinate_cached` でそのまま射影できる | `crates/rumoca-ir-dae/src/model/view.rs`（`InitializationOwnerView`）、GlobeMode の `globe-mode-frontend/src/init.rs` |
| 座標の種類 | `CoordinateView` は `Parameter` / `Input` / `State` / `Derivative` / `Algebraic` / `DiscreteReal` / `DiscreteValue` / `PreDiscreteReal` / `PreDiscreteValue` / `PreState` / `PreAlgebraic` / `Time` / `ClockInterval` / `Condition` / `Delay` / `Previous` / `Terminal` / `Binder` / `FunctionParameter`。S8 のマッチングでは `State` と `PreState` を状態の階数 0、`Derivative` を階数 1、`Algebraic` と `PreAlgebraic` を代数の階数 0、`Parameter` と離散（`pre` を含む）をそれぞれの列に写し、残りは既知として捨てる | `crates/rumoca-ir-dae/src/model/view.rs:1580` |
| 役割つきの ID | `ParameterId` / `StateId` / `AlgebraicId` / `DiscreteRealId` / `DiscreteValueId` の `index()` は `VariableId` の添字と同じ（`variable_identity_conversion!` で `VariableId::from_raw(value.index())`）。`view.variables()` の `(id, variable)` の `id.index()` と突き合わせられる | `crates/rumoca-ir-dae/src/ids.rs:168-186` |
| 取り分けた定義の式 | `InitialParameterValueView` と `InitialDiscreteValueView` は `target()`（`VariableId`）と `value()`（`ExprId`）と `provenance()` を持つ。`value()` も同じ射影で参照を取れる | `crates/rumoca-ir-dae/src/model/initial_parameters.rs`、`crates/rumoca-ir-dae/src/equations.rs:295-311` |
| パラメータの束縛 | パラメータの束縛（`VariableView::binding()` の `ExprId`）は `continuous_owners()` の方程式ではない。`Sub s(k = c)` の DAE の方程式は `s.k` を参照し、`c` は `s.k` の束縛にだけ現れる。束縛の式も同じ射影で参照を取れる | GlobeMode の `globe-mode-frontend/src/init.rs` のテスト `free_parameter_reached_through_a_binding_is_an_initialization_unknown` |
| 射影の失敗 | MSL の例題 566 本で、初期方程式の射影が失敗したものは無かった（S8 まで来た 369 本すべてでマッチングできた。[msl-funnel.md](../msl-funnel.md)） | GlobeMode の `cargo xtask msl-funnel` |
| **外からの修正が部品の中の同名の変数に漏れる** | `Part` が `Real theta1(fixed = true)` と `Joint joint(theta1(fixed = false))` を持ち `joint.theta1 = theta1` で結ぶとき、`Part part` をそのまま使えば `part.joint.theta1` は `fixed = false`（正しい）。例題の形で `Part part(theta1(fixed = true))` と修正すると、`part.joint.theta1` と `part.joint.der_theta1` も `fixed = true` になる（`VariableView::fixed_scalar`）。MSL の `Parts.RollingWheelSet` を `wheelSet(theta1(fixed = true), …)` と修正する `RollingWheelSetDriving` / `RollingWheelSetPulling` で、S8 のマッチングが「冗長 7」を出すのはこのため（omc は過剰と言わない）。**上流に報告する候補**。検証用のモデル `GlobeModeValidation.Init.ModifierScope` と db のテスト `modifier_leaks_into_subcomponents_in_the_compiler` に記録した | GlobeMode の計測（`globe check` で `Alias2.W1` / `W2`。2026-10-19） |
| **inner の修正が outer に伝わらない** | `inner System system(energyDynamics = FixedInitial)` を部品の `outer System system` から読む（`parameter Dynamics d = system.energyDynamics`、または条件に `system.energyDynamics` を直接書く）と、Rumoca は **型の既定**（`Free`）で評価する。`initial equation` の `if d == FixedInitial then p = ps_start; end if;` は消え（`initialization_owners()` に無い）、型の既定を `FixedInitial` にすれば現れる。outer を使わない連鎖（`parameter D b = a`、部品のパラメータの修正、親のパラメータでの修正）は正しい。MSL の Fluid の例題は `system(energyDynamics = FixedInitial)` の形なので、`PartialDistributedVolume` の初期方程式（`mediums.p = ps_start` など）が globe の DAE に無く、S8 が「不足」になる（`NonCircularPipes` で omc が冗長と言う 2 本はこれ）。**上流に報告する候補**。検証用のモデル `GlobeModeValidation.Init.OuterModification` と db のテスト `outer_modification_is_lost_by_the_compiler` に記録した | GlobeMode の計測（`globe check` で、`IfInit4` の 5 通りと MSL の `NonCircularPipes`。2026-10-19） |

## S010 で調べた API（v0.10.2）

S9（代数ループ）・S10（プリフライト）の入力と、閉包の切り替えの費用（[ADR-0012](../adr/0012-loops-and-preflight.md)、[ADR-0005](../adr/0005-lazy-query-db.md)）に使ったもの。確認日 2026-10-19、rev は `Cargo.toml` のもの。パスは `crates/` から。

| 項目 | 事実 | 出典 |
| --- | --- | --- |
| 代数ループの報告 | `phase_structural::analyze` は `StructuralDiagnostics::algebraic_loops` を一度も埋めない（宣言だけ）。ループは `collect_warnings` が ES002 の警告（message は "algebraic loop detected: {n} equations must be solved simultaneously"、notes に unknowns / equations）として `diagnostics` に積む。GlobeMode の照合（`Checker::reference_structure` の `loops`）は message から n を読む。ループの大きさは `phase_structural::sort` の `BltBlock::AlgebraicLoop { equations, .. }` からも取れる | `rumoca-phase-structural/src/diagnostics.rs:15`、`:25-70`、`lib.rs:168-217`、`types.rs:91-104` |
| 式の種類 | `ExpressionOperation`（Literal / Coordinate / Unary / Binary / 組み込みの呼び出し / 利用者の関数 / if 式 / 配列・レコード / 添字 / 内包 …）、組み込みは `PureBuiltin`、変化の度合いは `ExpressionView::variability()`。if 式の項は `[条件, 値, …, それ以外]` の順。`smooth` は 2 番目の引数が式 | `rumoca-ir-dae/src/model/view.rs:1666-1734`、`expression/nodes.rs:36-120`、`expression/value_types.rs:30` |
| 射影と if 式 | `for_each_scalar_coordinate` は if 式の条件の未知数も incidence に入れる。積では係数が 0 の座標を省く（`zero_coefficients`） | `rumoca-eval-dae/src/projection.rs:1163-1186`、`:1212-1238` |
| 数値の評価器 | `NumericEvaluator` は静的な値（パラメータ・定数の束縛、無ければ start。変数は start、無ければ束縛）だけを評価し、実行時の座標は拒む。値は式ごとに `require_finite` で確かめ、非有限は `InvalidValue`（"numeric evaluation produced a non-finite result"）。`div` / `mod` / `rem` の 0 除算も `InvalidValue`。`Overflow` は添字や要素数のあふれだけ。関数の中の `assert`（level = Error）は `AssertionFailed`（"function assertion failed"。文言は含まず、位置は `assert` の語）。level = Warning は無視する。外部関数は `ExternalFunction` | `rumoca-eval-dae/src/numeric.rs:546`（`initial_value`）、`:670`（`parameter_value`）、`:1770`（`require_finite`）、`:1494`、`:509`、`:478-483`、`:1529` |
| experiment annotation | `CompilationResult::experiment_*` は `sanitize_experiment_settings` の後の値で、非有限・StopTime < 0・Tolerance ≤ 0・Interval ≤ 0 を捨て、StopTime < StartTime なら StopTime を捨てる。値は数の字面だけを読み、名前は大文字と小文字を区別せず、根のクラスの `annotation` だけを見る。`CompilationResult` と flat のモデルには annotation が無いので、GlobeMode は `Dae::source_map()` の本文を Rumoca の parser で読み直して生の値を取る | `rumoca-compile/src/experiment.rs:300`、`:326`、`session.rs:1629` |
| 字面の 0 除算 | `parameter Real T = 1 / a`（a = 0）は、使われていなくても flat の定数評価が "division by zero" を出し、DAE への変換が `ED019`（"cannot be evaluated"）で止める。GlobeMode は ADR-0007 のコードの規則で `GM-U0001` に振り分ける（見直しの候補） | `rumoca-eval-flat/src/constant/errors.rs:69`、`rumoca-phase-dae/src/construction/analysis.rs:1439` |
| 文書の差し替えと解決 | `add_document` は毎回 `invalidate_resolved_state(DocumentMutation)` を呼び、`query_state.resolved.clear()` で解決済みの状態（`model_names`、`builds`、`dependency_fingerprints`）を捨てる。解決は Session の全文書（ライブラリを含む）を `merge_stored_definitions` で 1 つの木にまとめてやり直す。source root ごとに保存した解決を使い回す道は、source root の外から足した文書が 1 つでもあると使われない | `rumoca-compile/src/session/session_impl.rs:219`、`:312`（`remove_document`）、`:447-499`、`session/session_impl_queries.rs:79-83`、`:336-376`、`session.rs:1435` |
| モデルの名前の一覧 | `Session::model_names()` は解決を経るので、どれかの文書が解決できないと Err を返す。`rumoca_compile::parsing::collect_model_names` は構文木から名前を集める（公開） | `rumoca-compile/src/session/session_impl.rs:656`、`merge.rs:493` |

## S011 で調べた実行系（v0.10.2）

確認日 2026-10-19。rev `276e82bd` のソース（`crates/rumoca-sim`、`rumoca-solver`、`rumoca-solver-diffsol`、`rumoca-solver-rk45`、`rumoca-phase-solve`、`rumoca-test-msl`）を読んだ。行番号はこの rev のもの。

- **入口**（`rumoca_sim::`）:
  - `simulate_with_diagnostics(&Dae, &SimOptions) -> Result<SimResult, SimulationDiagnosticError>`（`lib.rs:128`）は `solver_mode`（Auto / Bdf / RkLike）で振り分けるが、失敗を `Solver(String)` に文字列化する。
  - `build_simulation` → `check_prepared_initialization` → `run_prepared_simulation`（`diffsol.rs:168,310,304`）は **`solver_mode` を見ず BDF だけ**。失敗は `SimError`（`stage()` で `SimFailureStage`: SolveLowering / StructuralAnalysis / BackendBuild / Initialization / EventIteration / ManifoldProjection / TargetIsolation / Integration）。`PreparedSimulation` は `Rc` と `RefCell` を持つので Send でない。
  - ソルバは diffsol 0.13 の BDF と、`rumoca-solver-rk45` の Dormand–Prince 5(4) だけ（`DiffsolMethod` は `Bdf` だけ。`rumoca-solver/src/solver.rs:59`）。
- **`SimOptions`**（`rumoca-solver/src/solver.rs:139-197`）: `t_start = 0`、`t_end = 1`、`rtol = atol = 1e-6`、`dt`（None なら区間の 1/500）、`max_wall_seconds = None`、`solver_mode = Auto`。最大の刻み数は無い。
- **`SimResult`**（`solver.rs:245-255`）: `times`、`names`、`data[変数][時刻]`、`n_states`、`variable_meta`（役割・単位・start など）、`termination`。行は初期化・出力の格子・イベントの左極限・イベント後の 4 種類で、同じ時刻が 2 行続くことがある。パラメータは含まない。Serialize は無い。
- **時間切れ**: 協調型。`TimeoutBudget::check()` は `advance_to` の繰り返しと根探索でだけ見る（`fmi_me/session.rs:486,1414`）。FMI の初期化は予算の外で、積分器の 1 歩の内側も見ない。Rumoca の MSL のハーネスは別プロセスのワーカーを監視から `process::exit` で止める（`rumoca-test-msl/src/bin/rumoca-sim-worker.rs:290-318`）。
- **大域の状態**: スレッドごとの計数（`step_counts`、`projection_fallbacks` など）と、プロセス全体の `nan_trace`（`simulate_with_diagnostics_auto_nan_trace` が切り替える。並列に回すと巻き込む）。
- **FMI-ME 風の境界**（`rumoca_solver::fmi_me`）: 公開されているのは `MeModelArtifact`・`MeSimulationSession` などと、数値積分のプラグインの trait `MeIntegratorBackend`（`initialize`・`advance`・`sample`・`truncate_reset`。`integrator.rs:874-923`）。核 `SolveMeKernel` の set_time・状態・導関数・イベント指標・イベント更新は `pub(crate)`（`kernel.rs:283-966`）。**外の driver から Rumoca のモデルを回す公開の API は無い。**
- **数値のプリフライトに使えるもの**: `eval_dae_at`（start 値で評価し、代数変数を Newton で更新したあとの値と `der(x)` を名前付きで返す。`nonfinite()` がある。初期化方程式の残差ではない）、`jacobian_for_dae`（状態についての ∂der(x)/∂x）、`diagnose_structural_singularity`（構造だけ）。**初期化系のヤコビ行列の階数を返す API は無い**（`runtime/projection/initial.rs:500-562` で SVD の数値の階数を tracing の debug に出すだけ）。
- **diffsol のパッチ**: ルートの `Cargo.toml:294-297` の `[patch.crates-io] diffsol = { path = "vendor/diffsol" }` で、crates.io の 0.13.2 に 4 件の修正（BDF の確定解、最小刻み、line search の Newton の収束の履歴、非線形の失敗の予算を 1 歩ごとにする）を当てている（`vendor/diffsol/RUMOCA_PATCH.md`）。`[patch]` はルートでしか効かないので、git 依存では修正前の版が入る。GlobeMode はルートに `diffsol = { git = "https://github.com/CogniPilot/rumoca", rev = "276e82bd…" }` の patch を置き、同じ版を使う（[ADR-0013](../adr/0013-exec-backend.md)）。
- **依存**: `default-features = false, features = ["solver-diffsol", "solver-rk45"]` で、GlobeMode の `Cargo.lock` に外部の package が 101 増えた（GlobeMode の crate 2 つを含めると 325 → 428。Windows の target の crate と Cranelift 0.125 が多い。wasm 以外では `rumoca-exec-cranelift` が無条件の依存）。ライセンスはすべて許可の区分（cargo deny で確かめた）。`instant` に保守終了の勧告 RUSTSEC-2024-0384 があり、wasm32 のときだけ使うので deny.toml の `ignore` に理由付きで足した。
- **wasm**: `build_simulation` の道と `SimulationSession` は `std::time::Instant::now()` を呼ぶ（`diffsol.rs:200,206`、`rk45.rs:107,112`）。wasm32-unknown-unknown では実行時に panic する見込み【未確認】。faer の `rand` 経由で getrandom 0.3 が入り、`--cfg getrandom_backend="wasm_js"` が要る。
- **MSL のハーネス**（`rumoca-test-msl/src/bin/rumoca-worker.rs`）: 区間は experiment（無ければ 0 → 1）、許容誤差は `Tolerance` を rtol と atol の両方に、出力は `Interval` か 500 区間、壁時計の予算は 12 秒。BDF の型付きの道で回し、エラー無く終わって NaN・inf が無ければ成功。
- **比較の指標**（`rumoca-sim/src/sim_trace_compare.rs`）: チャネルごとに時間平均の絶対誤差を参照の p95 − p05 で割って L とし、e = L/(1+L)。0.05 未満を high、0.20 未満を minor。管（ファネル）ではない。

## S012 で調べた実行系の公開の道（v0.10.2）

確認日 2026-10-19。rev `276e82bd` のソース（`crates/rumoca-sim`、`rumoca-solver`、`rumoca-phase-solve`、`rumoca-solver-diffsol`、`rumoca-solver-rk45`、`rumoca-phase-dae`）を読み、wasm の試作（wasm32-unknown-unknown + wasm-bindgen、Node v24.22.0）で確かめた。行番号はこの rev のもの。設計の判断は [ADR-0014](../adr/0014-session-route-wasm.md)。

- **数値積分のプラグイン**: `rumoca_solver::fmi_me` の trait `MeIntegratorBackend`（`initialize`・`advance`・`sample`・`truncate_reset`。`rumoca-solver/src/fmi_me/integrator.rs:874-923`）は公開。Rumoca の BDF と RK45 のプラグイン（`rumoca_solver_diffsol::model_exchange_integrator`、`rumoca_solver_rk45::model_exchange_integrator`）も公開。
- **FMI ME 風の host の生成と実行**（どれも公開。Rumoca に手を入れずに積分器を差せる）:

  | 順 | 関数 | 場所 |
  | ---: | --- | --- |
  | 1 | `lower_correlated_for_simulation_with_overrides`（lowering。パラメータの上書きを入れる） | `rumoca-sim/src/solve_lowering/overrides.rs:30` |
  | 2 | `rumoca_phase_solve::fmi::finish_fmi_component`（Solve IR を FMI の部品にする） | `rumoca-phase-solve` の `fmi.rs:125` |
  | 3 | `MeModelArtifact::new` | `rumoca_solver::fmi_me` |
  | 4 | `MeRetainedComponent::instantiate` | `rumoca-solver/src/fmi_me/session.rs:72` |
  | 5 | `into_lease`（初期化） | 同 `:116` |
  | 6 | `numerical_setup` | 同 `:238` |
  | 7 | `into_session(Some(Box<dyn MeIntegratorBackend>))` | 同 `:285` |
  | 8 | `run_to_stop` → `finish` | 同 `:502` |

  ハイブリッドの driver（イベント・根探索・出力の格子・時間切れ）は host が受け持つ。呼ぶ順は rumoca-sim の `me_backend::simulate_artifact` と同じ。
- **JIT は使えない**: Cranelift の JIT を使うかを決める `admitted_native_execution_backend` は `pub(crate)`（`rumoca-sim/src/native_execution.rs:311`）。上の道はネイティブでもインタプリタで回る。S012 の `Simulation.*` 12 本では、型付きの道と上の道（BDF のプラグイン）の結果がビット単位で同じだった。型付きの道で JIT が本当に使われているかは【未確認】。
- **`Instant::now` と wasm**: 型付きの道（`build_simulation`）と `rk45::simulate` は std の `Instant::now()` を呼ぶ（`rumoca-sim/src/diffsol.rs:200,206`、`rk45.rs:107,112`）。Node で実際に `RuntimeError: unreachable … <std::time::Instant>::now … build_simulation` になった（S011 の【未確認】を確かめた）。上の host の道の時間切れは `rumoca-solver/src/runtime/timeout.rs` で、wasm32 のときは `instant`（RUSTSEC-2024-0384、deny.toml で ignore 済み）を使うので、wasm でも効く。
- **Cranelift と wasm32**: `rumoca-exec-cranelift` は wasm32 では依存から自動で外れる（`rumoca-sim/Cargo.toml` の cfg、`rumoca-sim/src/lib.rs:50-70`）。wasm32 ではインタプリタで回る。
- **getrandom**: diffsol → faer → rand で getrandom 0.3 が入る。`.cargo/config.toml` の `[target.wasm32-unknown-unknown] rustflags = ['--cfg', 'getrandom_backend="wasm_js"']` だけでは「backend requires the `wasm_js` feature」で落ち、getrandom の feature `wasm_js` も要る。
- **フロントエンドと wasm**: フロントエンドは wasm32 で動く（rayon は呼んだスレッドで逐次に回る。`std::thread::scope` は Rumoca 側で `cfg(not(wasm32))`）。MSL 4.1.0 の `.mo` は 2556 本・15.3 MB で、in-memory の組（DurableExternal）として渡せる。wasm32-unknown-unknown は panic=abort なので `catch_unwind` は効かない。
- **パラメータの上書き**: `SimOptions::param_overrides: Vec<(String, f64)>`（`rumoca-solver/src/solver.rs:164`）。DAE は使い回せて再コンパイルは要らないが、lowering と組み立てはやり直す。`PreparedSimulation` は opts を固定で持つので使い回せない。実行の途中で変えられるのは入力だけ。
  - 変えられるのは role が Parameter で `is_tunable` が真のもの（`rumoca-phase-dae/src/construction/variable_construction.rs:332`）。一覧は `build_tunable_parameter_meta`（`rumoca-sim/src/lib.rs:545`）。
  - 失敗は `SimulationDiagnosticError::InvalidOverride`（`rumoca-sim/src/solve_lowering/overrides.rs:53-88`、`[EX003]`）。文言は 3 つ（structural or constant / not a tunable parameter / must be finite）。GlobeMode は `GM-E10006` にして文言を分ける。
  - Rumoca の `eval_dae_at` は上書きを見ない。プリフライトに上書きを反映するには、`lower_for_simulation_with_overrides` と `SolveRuntime::eval_at` を使う。
- **diffsol の陽的 RK**: diffsol の `ClosureNoJac` は非公開なので、陽的 RK（tsit45）もヤコビ行列つきの `rhs_implicit` で問題を作る。

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
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-ir-ast/src/nodes.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-ir-dae/src/model/view.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-ir-dae/src/provenance.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-phase-dae/src/construction/variable_construction.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-ir-dae/src/model/initial_parameters.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-ir-dae/src/model/view/arena_walks.rs>（S007）
- <https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-compile/src/session/session_impl_queries.rs>（S007）
- S012（確認日 2026-10-19）: rev `276e82bd` のソース <https://github.com/CogniPilot/rumoca/tree/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates>（`rumoca-sim`、`rumoca-solver`、`rumoca-phase-solve`、`rumoca-solver-diffsol`、`rumoca-solver-rk45`、`rumoca-phase-dae`）

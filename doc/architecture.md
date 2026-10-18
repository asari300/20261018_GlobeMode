# GlobeMode アーキテクチャ

- 版: v0（2026-10-18、S001）。
- 判断の理由は [adr/](adr/) に、機能の一覧は [features.md](features.md) に、順序は [roadmap.md](roadmap.md) にある。

## 1. 全体像

計算はすべて Rust のコアで行う。UI と各言語の API は、同じ facade（`globe-mode`）と同じ protocol 型（`globe-mode-protocol`）を通してコアを呼ぶ。

```mermaid
flowchart LR
  subgraph Front["フロントエンド（Phase A: Rumoca / Track B: native）"]
    P[parse] --> O[outline] --> CI[class_index / class_interface]
    CI --> F[flat] --> D[dae_structure]
  end
  CI --> G[connect_graph]
  D --> S[simulatability S0–S10]
  S --> X[executable]
  X --> SIM[sim driver + Integrator]
  SIM --> R[(ResultStore)]
  X --> FMU[FMU export 2.0 / 3.0]
  IMP[FMU import] --> SIM
  subgraph Clients
    UI[Svelte 5 UI<br/>Tauri / Browser Worker]
    LSP[LSP]
    CLI[globe CLI]
    API[Python · Julia · Zig · Elixir · Gleam]
  end
  G & S & R --> Clients
```

## 2. 配置

### 2.1 ディレクトリ

ユーザーが作った言語別の構成に合わせる。

```text
Cargo.toml                      virtual workspace（resolver 3, edition 2024）
mise.toml / mise.lock           ツールの版（rust / uv / zig / julia / erlang / elixir / gleam。P4 で node）。rust-toolchain.toml は置かない
pyproject.toml / uv.lock        Python の依存（uv）
src/Rust/mod/<crate>/           workspace の library crate（2.2 の表）
src/Rust/apps/desktop/          Tauri 2 のシェル（Linux, Windows, Android）
src/Rust/apps/experimental/     leptos/ と dioxus/（default-members から外す）
src/Rust/xtask/                 build、bump-rumoca、licenses、wasm --threads
src/Svelte/                     Svelte 5 + TypeScript の UI。Tauri とブラウザで共用する
src/Python/mod/                 Python パッケージ globe_mode（PyO3 と maturin）
src/Python/marimo/              marimo のノートブックと anywidget
src/Python/streamlit/           結果ダッシュボードとパラメータスイープ
src/Elixir/  src/Gleam/         Rustler の NIF を呼ぶ薄いラッパ
src/Julia/   src/Zig/           C ABI を呼ぶ薄いラッパ
src/Modelica/                   サンプルモデルと検証用ライブラリ
data/msl/                       MSL 4.1.0（取得スクリプトで置き、sha256 で確かめる。コミットしない）
doc/  log/                      文書と記録
```

### 2.2 Rust crate（`src/Rust/mod/`）

**（S002）** と書いたものは実装済み。ほかは予定。

| crate | 責務 | 主な依存 |
| --- | --- | --- |
| `globe-mode-core` **（S002）** | `TextRange`、`Diagnostic{code,severity,stage,message,file,range,notes}`、検査段階 `Stage`（S0〜S10）、診断コード。S005 で `normalize`（防火壁 0 の本文の正規化）、GM-U・GM-E6001・GM-I7001 のコード、`Diagnostic::is_unsupported`。今後: ID、単位、修正案（fix） | serde |
| `globe-mode-ir` **（S002）** | フロントエンドに依存しない中立 IR: `Outline`（クラス・コンポーネント・extends・connect、S004 で flow / stream と input / output）、`ClassIndex`（字句的な名前解決、extends を含むインターフェース、S004）、`BalanceCount`（スカラー）、`DaeCounts`（配列ごと、S003）、`DaeStructure`（未知数・方程式・incidence・`state_refs`、`without_ranges`、`merged_incidence`、S005）、`FlatSummary`（部品ごとの方程式数、S005）。今後: `ConnectSet`、変数の属性、初期方程式 | core |
| `globe-mode-frontend` **（S002）** | `trait Frontend{outline, check}`。feature `rumoca`（Phase A、`src/rumoca.rs`）と feature `native`（Track B）。S005 で `Checker`（ライブラリを一度だけ読んだ Session を使い回す）、`DaeStructure` の抽出（`structure.rs`）、GM-U の振り分け（`unsupported.rs`、[ADR-0007](adr/0007-unsupported-classification.md)） | rumoca-compile、rumoca-ir-dae・rumoca-eval-dae・rumoca-core（S005、同じ rev） |
| `globe-mode-db` **（S004）** | salsa のクエリ DB（§3.2）: `outline` → `outline_shape`（防火壁 1）→ `class_index` → `connect_graph`（S004）。`shape` → `source_shape`（防火壁 0）/ `shape_map` → `check` → `flat_summary` / `dae_shape`（防火壁 2）→ `structural`（S005）。`GlobeDb::with_libraries` と `check_model`（S0〜S6）。実行したクエリの記録（テストと計測） | salsa、frontend、graph、analysis |
| `globe-mode-analysis` **（S005: S6）** | Hopcroft–Karp、粗い Dulmage–Mendelsohn 分解、`StructuralReport`（S6 の GM-E6001 と、S7 の情報 GM-I7001）。今後: Tarjan BLT、Pantelides、`SimulatabilityReport` | core、ir、serde（Rumoca に依存しない。wasm32 でビルドできる） |
| `globe-mode-graph` **（S004: connect グラフ）** | connect グラフを flatten せずに作る。辺の種類（flow / stream / real / boolean …）、depth による部分回路の展開、graphology JSON。今後: ライブラリ / incidence のグラフ、差分、ForceAtlas2 | serde_json |
| `globe-mode-exec` | `HybridSystem` の実装: Rumoca の実行系（feature `rumoca-exec`）、FMU-ME アダプタ | rumoca-sim |
| `globe-mode-solver` | `trait Integrator`、ソルバ名の登録表、自前の固定刻みと陽的 RK | diffsol, sundials（feature） |
| `globe-mode-sim` | ハイブリッドループの driver、`ResultStore`（CSV / MAT v4 / Arrow IPC） | solver |
| `globe-mode-fmi` | FMU の Import、マスターアルゴリズム、パッケージング | fmi, fmi-schema |
| `globe-mode-fmu-runtime` | FMI 2 / 3 の C API を実装する cdylib（P5b） | — |
| `globe-mode-protocol` | serde 型と JSON Schema。TypeScript と Python の型を生成する | serde, specta |
| `globe-mode` | 公開する Rust API（facade）: `Workspace{open, set_text, outline, check, graph, simulate, export_fmu}` | 上記すべて |
| `globe-mode-lsp` | `lsp-types` と自前のディスパッチャ。Transport は stdio / Tauri のチャネル / Web Worker | lsp-types |
| `globe-mode-cli` **（S002）** | バイナリ `globe`（`outline` / `check` / `graph` / `flat`、`--json`、S003 で `--library`、S004 で `graph` とディレクトリの入力、S005 で `check` を db 経由にして S6・GM-U を表示し、`flat` を足した。終了コード 0 / 1 / 2） | clap |
| `globe-mode-capi` | `extern "C"`、不透明ハンドル、JSON とバイト列。cbindgen でヘッダを生成する | — |
| `globe-mode-py` | PyO3（abi3-py312） | pyo3 |
| `globe-mode-nif` | Rustler。Erlang モジュール名は `globe_mode_nif` | rustler |
| `globe-mode-wasm` | wasm-bindgen。ブラウザの Worker、Gleam の JS target、marimo の anywidget から使う | wasm-bindgen |
| `globe-mode-testkit` | MSL コーパス、ファネル比較、omc との照合、Reference-FMUs（S003 の突き合わせと S005 のファネルは、ひとまず xtask に置いた） | — |
| `xtask` **（S003）** | `cargo xtask fetch-msl` / `fetch-rumoca` / `msl-compare` / `msl-funnel`（S005）/ `bump-rumoca`。場所は `src/Rust/xtask` | sha2, zip, db, analysis |

### 2.3 ツールチェーン

[ADR-0003](adr/0003-toolchain.md) にまとめた。

- **言語の管理は mise・uv・cargo で分担する**。mise がツールの版（`mise.lock`）、uv が Python と依存（`uv.lock`）、cargo が Rust の依存（`Cargo.lock`）を受け持つ。
- Rust の版は `mise.toml` に一本化し、`rust-toolchain.toml` は置かない。mise が設定する `RUSTUP_TOOLCHAIN` が優先されるため。
- **既定は stable**。Rumoca の `rust-toolchain.toml`（nightly 固定）は Rumoca 自身のワークスペースでしか効かない。git 依存として取り込むと、GlobeMode 側のツールチェーンでビルドされる。
  - ただし、Rumoca のコードが nightly の機能を使っていれば stable では通らない。**P1 で確かめる**。
- スレッドを使う wasm だけは nightly でビルドする。`-Z build-std` と atomics が要るためで、コマンドは `xtask wasm --threads`。配信には COOP/COEP のヘッダか coi-serviceworker が要る。
- wasm と Android の target は mise の rust の `targets` で宣言する。Python 本体は uv が `.python-version` から取得する。手順は [mise-setup.html](mise-setup.html)。

## 3. 中核の設計

### 3.1 フロントエンドと Rumoca アダプタ

[ADR-0002](adr/0002-rumoca-adapter.md) にまとめた。

#### 依存のしかた

- `rumoca-*` はすべて同じ git の `rev` から取り、`default-features = false` にする。
- **umbrella crate `rumoca` は使わない**。zenoh・gamepad・UDP を引き込むため。

#### 閉じ込め方

- cargo-deny の `bans` に `wrappers = ["globe-mode-frontend", "globe-mode-exec"]` を設定し、ほかの crate から Rumoca を直接使わせない。
  - 対象は `rumoca-compile`。S005 で足した `rumoca-ir-dae`・`rumoca-eval-dae`・`rumoca-core` は Rumoca の内部の crate からも使われるので、`wrappers` に書けない。frontend だけが直接依存することは、Cargo.toml のレビューで守る。
- Rumoca の型は中立 IR（`globe-mode-ir`）に変換してから外に出す。Rumoca の API が変わっても直すのはアダプタだけで済み、Track B（自前コア）に替えても下流は変わらない。

#### 更新

- 月に1回 `xtask bump-rumoca` で `rev` を上げる。
- MSL コーパスのファネルの差分を作業記録に残す。

### 3.2 遅延評価とシミュレーション前のアサーション

[ADR-0005](adr/0005-lazy-query-db.md) にまとめた。

**salsa のクエリ**（`globe-mode-db`。S004・S005 で作ったものと、予定のもの）:

```text
SourceFile ─┬─▶ outline ─▶ outline_shape ─▶ class_index ─▶ connect_graph     IDE の機能（S004）
            │              防火壁 1
            └─▶ shape ─┬─▶ source_shape ─▶ check ─┬─▶ flat_summary          検査（S005）
                       │   防火壁 0        S0〜S5 └─▶ dae_shape ─▶ structural
                       └─▶ shape_map                  防火壁 2      S6
```

| クエリ | 入力 | 比較 | 役割 | 状態 |
| --- | --- | --- | --- | --- |
| `SourceFile` / `Workspace` | （入力） | — | ファイルのパスと本文 / 開いているファイルの集まり。MSL などのライブラリは salsa に入れず、`Checker` が一度だけ読む | S004 |
| `outline(file)` | SourceFile | Eq | アウトライン（位置あり）。IDE の機能（アウトライン、位置）に使う。構文の誤りは S0 | S004 |
| `outline_shape(file)` | outline | **Eq** | **防火壁 1**。位置を 0 にした写し。索引とグラフは、空白やコメントだけの編集では再計算しない | S004 |
| `class_index(workspace)` | outline_shape | Eq | 完全修飾名の索引（`within` を反映）。字句的な名前解決と、extends を含むインターフェース（`interface`） | S004 |
| `connect_graph(workspace, request)` | class_index | Eq | **flatten しなくても描ける**。`GraphRequest`（モデル、depth） | S004 |
| `shape(file)` | SourceFile | Eq | 正規化した本文と位置の対応表（`globe_mode_core::normalize`） | S005 |
| `source_shape(file)` | shape | **Eq** | **防火壁 0**。コメントを除き、空白を 1 個に畳んだ本文。空白やコメントだけの編集では、Rumoca のコンパイルが走らない | S005 |
| `shape_map(file)` | shape | Eq | 正規化後の位置 → 元の本文の位置。診断の位置を戻すのに使い、`check` の依存には入れない | S005 |
| `check(workspace, request)` | 全ファイルの `source_shape` と `Checker` | Eq | Rumoca の resolve・flatten・ToDae。`CheckReport`（S0〜S5 の診断、位置付きの `DaeStructure`、`FlatSummary`）。`CheckRequest` はモデル名 | S005 |
| `flat_summary(workspace, request)` | check | Eq | flat モデルの要約（`globe flat`）。S5 で数が合わないときの、部品ごとの方程式数 | S005 |
| `dae_shape(workspace, request)` | check | **Eq** | **防火壁 2**。位置を落とした `DaeStructure` | S005 |
| `structural(workspace, request)` | dae_shape | Eq | S6（`globe-mode-analysis` の `StructuralReport`） | S005 |
| `deps_closure(model)` | class_index | Eq | Rumoca に渡すファイル集合（広めに見積もる）。今はワークスペースの全ファイルを渡すので、どのファイルの中身が変わっても `check` が再実行される | 予定（S006 の候補） |
| `simulatability(model)` | structural など | Eq | 下表の S0〜S10 をまとめた `SimulatabilityReport` | 予定 |
| `executable(model, backend)` | dae_shape | LRU | 実行形式 | 予定（P3） |

- **`Checker`**（`globe-mode-frontend`）は salsa の外の値。MSL などのライブラリを読んだ Rumoca の `Session` を `Arc<Mutex<_>>` で持ち、`GlobeDb::with_libraries(paths)` で作る（[ADR-0005](adr/0005-lazy-query-db.md) の結果）。
  - 文書は、本文が変わったものだけ差し替え、無くなったものは取り除く。文書を変えると Rumoca は解決済みの状態を捨てるが、parse の結果とモデルごとのキャッシュは残る。
  - panic で Mutex が毒されたら、ライブラリを読んだ直後の写しに戻す。`reset()` でモデルごとのキャッシュを捨てられる。
- `GlobeDb::check_model(model)` は、`check`（S0〜S5）と `structural`（S6）をまとめ、診断の位置を `shape_map` で元の本文に戻す。S6 は S0〜S5 を通ったときだけ行う。
- Rumoca の AST は salsa に載せない。`outline` が、そのファイルを毎回 parse する。

シミュレーションの実行は salsa の外のジョブにする。結果は (executable, settings) のハッシュでキャッシュする。

**`assert_simulatable`** は simulate と export の前に必ず走る。

- 段階の順に検査し、Error が出た段階で止める。その段階の指摘は全件出す。
- `--force` を付けない限り、Error があれば実行を拒否する。

| 段階 | 検査 | 主なコード |
| --- | --- | --- |
| S0 | 構文 | GM-E0xx |
| S1 | 名前解決、`uses` の版の不一致 | GM-E1xx / W1xx |
| S2 | 型と次元。単位の不一致は警告 | GM-E2xx / W2xx |
| S3 | instantiate: partial なトップクラス、redeclare されていない replaceable | GM-E3xx |
| S4 | connector のバランス（MLS 9.3）、非互換な接続、未接続（警告）、トップレベルの input（情報。FMU の入力になる） | GM-E4xx / W4xx / I4xx |
| S5 | 方程式数と変数数。コンポーネントごとのローカルバランス（MLS 4.7）で、どのコンポーネントが原因かを示す | GM-E5xx |
| S6 | 構造特異: `x` と `der(x)` を 1 つの列にまとめた二部グラフ（方程式 × 未知数）に完全マッチングがあるか。Pantelides 法が失敗するのと同じ条件。無ければ粗い Dulmage–Mendelsohn 分解で、過剰な方程式と不足している未知数を span 付きで示す（S005） | GM-E6001 |
| S7 | インデックス: der の形（状態を既知とみなす）に完全マッチングが無ければ、微分インデックスが 1 より大きい見込みとして情報を出す（S005。モデルの誤りではない）。今後: Pantelides と状態選択 | GM-I7001 / E7xx / W7xx |
| S8 | 初期化: 初期化系のバランス、start 値の無い状態変数、fixed の過剰、値の無いパラメータ | GM-E8xx / W8xx |
| S9 | 代数ループ: 非線形の反復変数に start 値が無い | GM-W9xx |
| S10 | プリフライト: パラメータの評価、t0 で残差が有限か、ソルバの能力が足りるか、experiment annotation が妥当か | GM-E10xx |

- **「コンパイラが未対応」は GM-Uxxx として分ける**。対象は stream / expandable / overconstrained connector、state machine、operator record など。「モデルの誤り」と混同させないため。
  - 振り分けは [ADR-0007](adr/0007-unsupported-classification.md)。Rumoca のコードで決まるもの（`GM-U0001` 未対応、`GM-U0002` 内部エラー）と、既知の制約の一覧（`src/Modelica/msl-known-unsupported.txt`）に当たったもの（`GM-U0003`）。
  - severity は Error のまま（`--force` 無しでは実行しない）。CLI は `unsupported[GM-U…]` と表示する。
- 結果は `SimulatabilityReport{verdict, stages[], stats}` として返す。出力先は LSP の診断、CLI（ariadne 風）、protocol の JSON、グラフのハイライト。
- S5 の正しさは `omc checkModel()` の方程式数・変数数と照合して確かめる（S005 の時点では omc を入れていないので、まだ照合していない）。

### 3.3 ソルバ

```rust
trait HybridSystem {
    fn dims(&self) -> Dims;                                      // n_x, n_roots, 形式: Ode | MassMatrix | ImplicitDae
    fn rhs(&mut self, t: f64, x: &[f64], dx: &mut [f64]) -> Eval;
    fn residual(&mut self, t: f64, x: &[f64], dx: &[f64], r: &mut [f64]) -> Eval; // 任意
    fn jacobian(&mut self, t: f64, x: &[f64], j: &mut dyn JacSink) -> Eval;      // 任意。無ければ着色差分
    fn roots(&mut self, t: f64, x: &[f64], g: &mut [f64]) -> Eval;
    fn next_time_event(&self) -> Option<f64>;
    fn event_update(&mut self, t: f64, x: &mut [f64]) -> EventOutcome;
}

trait Integrator {
    fn caps(&self) -> Caps;  // implicit, adaptive, dae, native_roots, dense, wasm
    fn init(&mut self, s: &mut dyn HybridSystem, t0: f64, x0: &[f64], o: &SolverOptions) -> Result<()>;
    fn step(&mut self, s: &mut dyn HybridSystem, t_stop: f64) -> Result<Step>; // Advanced | Root | Reached
    fn dense(&self, t: f64, x: &mut [f64]) -> Result<()>;
    fn reset(&mut self, t: f64, x: &[f64]) -> Result<()>;
}
```

- ハイブリッドループは `globe-mode-sim` の driver が持つ。担当は時間イベント、根探索（ソルバ自身の機能か、密出力と Illinois 法）、イベント反復、リセット。
- **Modelica のモデルと FMI-ME の FMU は同じ driver を使う。**
- 代数ループは faer の LU を使った減衰 Newton 法で解く。feature `sundials` を有効にしたときは KINSOL を使う。

| GlobeMode ID | OpenModelica | Dymola | 実装 | ライセンス | wasm |
| --- | --- | --- | --- | --- | --- |
| `bdf`（既定） | dassl†、ida† | Dassl†、Lsodar† | diffsol BDF | MIT | ○ |
| `ida`、`cvode` | ida、cvode | Cvode | SUNDIALS（feature `sundials`） | BSD-3 | × |
| `esdirk34`、`trbdf2` | gbode（esdirk） | Esdirk34a | diffsol SDIRK | MIT | ○ |
| `esdirk23`、`esdirk45`、`sdirk34hw` | gbode | Esdirk23a/45a、Sdirk34hw | diffsol SDIRK と自前の係数表 | MIT | ○ |
| `tsit45` | — | — | diffsol | MIT | ○ |
| `dopri45`、`dopri853` | gbode（dopri45） | Dopri45、Dopri853 | 自前の陽的 RK（ode_solvers と照合する） | 自前 | ○ |
| `euler`、`heun`、`rk4`、`impeuler`、`trapezoid` | 同名、rungekutta | Euler、Rkfix2/3/4 | 自前の固定刻み | 自前 | ○ |
| `radau5`（P8） | gbode（radau） | Radau IIa | 自前。russell_ode は GPL の UMFPACK をリンクするので**使わない** | 自前 | ○ |
| `cerk23/34/45`、`qss` | qss | Cerk23/34/45 | 後回し | — | — |

† は別名。指定すると「dassl → bdf（同等の手法で、結果は一致しない）」と表示する。

KLU（LGPL）は同梱しない。

### 3.4 FMI 2.0 / 3.0

#### Import（`fmi` 0.8 と `fmi-schema` の上に作る）

| 種類 | 実装 |
| --- | --- |
| ME（2.0 / 3.0） | `FmuMeSystem` が `HybridSystem` を実装するので、全ソルバと driver をそのまま使える |
| CS（2.0 / 3.0） | `CoSimUnit{do_step, get/set, get/set_state, early_return}` |
| SE（3.0） | `ScheduledUnit{activate_model_partition}` と、優先度付きのクロックスケジューラ |

マスターアルゴリズムは次の順で作る。

1. 固定刻みの Jacobi / Gauss–Seidel から始める。実行順は `ModelStructure` の direct feedthrough から決め、代数ループは不動点反復で解く。
2. 次に可変刻みに進む。`canGetAndSetFMUState` があるときはロールバックする。FMI 3 のイベントモード、early return、SE にも対応する。
3. 系の構成は SSP か `.gmsys`（JSON）で与える。

ブラウザでの制約: ネイティブの FMU バイナリはデスクトップと Android でしか読めない。ブラウザでは `modelDescription.xml` の閲覧と、wasm バイナリ付きの FMU の実行に限る。

#### Export

| 段階 | 方法 | 対応 |
| --- | --- | --- |
| P5a（早い経路） | Rumoca の `fmi2` / `fmi3` テンプレートでソース FMU を作り、`cc` か `zig cc` で各 target 向けにビルドする | テンプレート生成物のライセンスはこの段階で確かめる |
| P5b（自前のランタイム） | `globe-mode-fmu-runtime`（BSD-2 の FMI ヘッダの上に作る）。IR を埋め込んで解釈実行するか、Rumoca の `rust-ode` テンプレートでコンパイルする | FMI 2 の ME / CS、FMI 3 の ME / CS / SE。解釈実行ならコンパイラが要らないので、ブラウザや Android からも出力できる |

- `ModelStructure` は BLT の incidence から作る。
- CS の FMU は自前のソルバで刻む。
- `fmi-export` は FMI 3 だけを Export できる代替手段として扱う。FMI 2 の Export は自前で作る。

#### FMI の検証

- Reference-FMUs（BSD-2）を FMI 2 と 3 の両方で動かし、参照 CSV と照合する。モデルは BouncingBall、Dahlquist、VanDerPol、Stair、Feedthrough、Resource、Clocks。
- Export した FMU は、次をすべて通す。
  - `fmpy.validate_fmu`
  - FMPy と fmusim でのシミュレーション
  - FMU Compliance Checker（FMI 2 のみ）
- 往復（Export → Import）の結果が、直接シミュレーションした結果のファネルに収まることを確かめる。
- OMSimulator は別プロセスでのみ使う。

### 3.5 connect のネットワークグラフ

S004 で最初の版を作った（`globe-mode-graph`、`globe graph`）。辺の種類は、コネクタのクラスから次のように決める。

| 種類 | 判定 | 例（`src/Modelica/GlobeModeValidation`） |
| --- | --- | --- |
| `flow` | コネクタに flow の変数がある（potential と flow） | 電気のピン（Real の v と flow Real の i） |
| `stream` | コネクタに stream の変数がある | Medium を持つ流体ポート |
| `real` / `integer` / `boolean` / `string` | コネクタが組み込みの型を継承している（`connector RealInput = input Real`） | Real・Boolean の信号 |
| `other` / `unresolved` | potential だけ / 索引に無い型 | — |

- **データ**: `connect_graph(path, depth)` が graphology 形式の JSON（nodes / edges / attributes）と、編集ごとの差分を返す。
  - ノードはコンポーネントとコネクタ。エッジは connect 文と、継承や包含の関係。
  - ノードの色は connector の種類（電気、並進、熱、信号など）で決め、大きさは次数で決める。
- **描画**: sigma.js（WebGL）と ForceAtlas2 で描く。ForceAtlas2 は Worker で回し、反復回数に上限を設ける。
  - 1万ノードを超えたら cosmos.gl に切り替える。
  - CLI では Rust の ForceAtlas2 で SVG を出力する。
- **操作**: 階層の展開と折り畳み（depth）、検索、近傍のハイライト。
  - 検査で見つかった問題（S4〜S6）をグラフ上に色で示す。

## 4. ターゲットごとの実行方法

| ターゲット | 計算 | UI | 配布物 |
| --- | --- | --- | --- |
| Desktop（Debian 系 / Windows 11） | Tauri のプロセス内でネイティブ実行 | Svelte 5（TauriBackend） | .deb、.AppImage、.msi |
| Android（7 以降。検証は API 34） | 同上。cranelift は無効 | 同上 | .apk |
| ブラウザ | **閲覧者の端末の Web Worker で wasm を実行**。サーバーでは計算しない。既定はシングルスレッドの stable ビルド | Svelte 5（WorkerBackend） | 静的サイト（CSP は `connect-src 'self'`） |
| CLI | ネイティブ | — | `globe` |
| marimo | anywidget で wasm を読む。代替は PyO3 の wheel | ノートブック | wheel と ESM |
| Streamlit | PyO3 の wheel | ダッシュボード | wheel |

Svelte の UI は `Backend` インターフェースだけに依存する。実装は TauriBackend と WorkerBackend の2つ。

## 5. 言語 API

| 言語 | 経路 | 置き場所 |
| --- | --- | --- |
| Rust | facade crate `globe-mode` | `src/Rust/mod/globe-mode` |
| Python | `globe-mode-py`（PyO3）→ パッケージ `globe_mode` | `src/Python/mod` |
| Julia | `globe-mode-capi` を ccall で呼ぶ。BinaryBuilder の JLL で配る | `src/Julia` |
| Zig | `globe-mode-capi` のヘッダを `@cImport` する | `src/Zig` |
| Elixir | `globe-mode-nif`（Rustler、rustler_precompiled） | `src/Elixir` |
| Gleam | Erlang target は `@external` で `globe_mode_nif` を呼び、JS target は `globe-mode-wasm` を呼ぶ | `src/Gleam` |

**共通の参照シナリオ**: 全言語で同じ手順を通す。open → check → graph → simulate（bdf）→ 結果を取得 → FMU を出力。P7 の完了条件はこれが6言語で通ること。

## 6. UI の比較実験（P7）

- 本線は Tauri 2 と Svelte 5。
- Leptos 版と Dioxus 版は `src/Rust/apps/experimental/` に、機能を絞って作る（エディタ、検査パネル、グラフ）。
- 同じ測定ハーネスで比べ、結果を `doc/bench/ui.md` に書く。
  - 測定項目: 起動時間、RSS / PSS、CPU、フレーム時間、バンドルサイズ
  - 環境: Linux、Windows、Android

## 7. 検証の基盤

| 対象 | 方法 |
| --- | --- |
| フロントエンド | MSL 4.1.0 の例題でファネルを数え、Rumoca の値と比べる（Track B では native と rumoca の差分）。例題 20 本で Rumoca の CLI と突き合わせる（`cargo xtask msl-compare`、S003） |
| MSL の全例題 | `cargo xtask msl-funnel`（`mise run msl:funnel`、S005）。Rumoca と同じ名前の規則で選んだ 566 本に S0〜S6 / OK / GM-U の判定を出し、Rumoca の基準値と並べる。検査器は 1 つ（MSL を一度だけ読む）で、モデルごとに `catch_unwind` で囲み panic を数える。時間がかかるので CI には入れない |
| 検査 | S5 を `omc checkModel()` と照合する（別プロセス） |
| シミュレーション | MSL の参照結果と、ファネル（許容帯）で比較する |
| FMI | Reference-FMUs、FMPy、fmusim、fmuCheck、往復 |
| ライセンス | cargo-deny、`pnpm licenses`、cargo-about、CycloneDX SBOM |
| CI | 差分だけを検査し、依存・ツール・CI の定義の変更や大きな変更のときだけ全体を検査する（[ADR-0006](adr/0006-incremental-ci.md)） |
| コードレビュー | Claude Code が主。副次的に、Windows 側の LM Studio のローカルモデルにも差分を読ませる（スキル `/local-review`、[ADR-0008](adr/0008-local-llm-reviewer.md)）。指摘は Claude が確かめて採否を決める |
| 性能 | P2: 空白だけの編集の再検査が 50 ms 未満（S005: S0〜S6 の再検査で中央値 0.01 ms）。P4: 5k ノードのグラフが 30 fps 以上 |

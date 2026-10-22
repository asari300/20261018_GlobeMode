# GlobeMode に要る機能

OpenModelica（OMEdit）と Rumoca を参考に、GlobeMode に要る機能を洗い出した。

- 調査の詳細は [research/openmodelica-dymola.md](research/openmodelica-dymola.md) と [research/rumoca.md](research/rumoca.md)。
- Phase は [roadmap.md](roadmap.md) の区分。

## 表の記号

- OMEdit・Rumoca 列: ○ ある / △ 部分的 / × 無い / — 対象外
- 優先度:
  - **Must**: 最初の公開に必須
  - **Should**: 公開後すぐに要る
  - **Could**: 余裕があれば

## 1. 編集と言語サービス

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| エラーに強いパーサ | ○ | ○ | Must | P1 | Phase A は Rumoca（parol）。Track B は rowan |
| 診断（構文・名前・型） | ○ | ○ | Must | P2 | salsa のクエリで差分だけ再計算する |
| 補完・定義ジャンプ・参照・リネーム・ホバー | △ | ○ | Must | P4 | `globe-mode-lsp`（Transport を抽象化） |
| アウトライン・inlay hint・call hierarchy | △ | ○ | Should | P4 | `outline` クエリの結果をそのまま使う |
| 整形・lint | × | ○ | Should | P4 | Rumoca の tool-fmt / tool-lint をアダプタ経由で使う |
| Text ビュー | ○ | ○（VS Code） | Must | P4 | CodeMirror 6 |
| Diagram / Icon ビュー（Placement / Line annotation） | ○ | × | Should | P8 | Svelte Flow。テキストと双方向に同期する |
| Documentation ビュー（HTML annotation） | ○ | × | Could | P8 | サニタイズした HTML を表示する |
| パラメータダイアログ（modifier、redeclare の選択） | ○ | × | Should | P8 | `class_interface` クエリから作る |

## 2. ライブラリと構造の可視化

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| MSL の読み込み（4.1.0） | ○ | ○ | Must | P1（S003 で済） | `cargo xtask fetch-msl` で `data/msl/` に取得し、`globe check --library` で読む。S005 から、読んだ Session を `Checker` で使い回す。必要になったファイルだけ parse するのは P2 |
| 検査に渡すファイルの絞り込み（依存の閉包） | — | × | Should | P2（S007） | `deps_closure` クエリ。ワークスペースのうちモデルが使うファイルだけを Rumoca に渡し、使わないファイルの編集では再検査しない。`globe deps <files> -m <model>` で閉包を表示する（例: `Connect.RealCircuit` は 5 / 23 ファイル） |
| ライブラリツリー | ○ | × | Must | P4 | `class_index`（`package.order`） |
| パッケージマネージャ（`uses` の版） | ○ | × | Could | P8 | 版の不一致は S1 の検査で警告する |
| **connect のネットワークグラフ（Obsidian 風）** | × | × | **Must** | P2（データは S004 で最初の版）/ P4（描画） | flatten 不要の `connect_graph(path, depth)`。sigma.js + ForceAtlas2。巨大なグラフには cosmos.gl |
| incidence グラフ・BLT の可視化 | △（変換デバッガ） | × | Should | P8 | `dae_structure` を同じグラフ基盤で描く |
| フラットモデルの表示（Instantiate） | ○ | ○ | Should | P2（S005 で要約） | `globe flat`（S005）: 変数・方程式の数、部品ごとの方程式数（`flat_summary` クエリ）。フラットモデルの本文の表示は後回し |

## 3. 検査とシミュレーション

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| Check Model（方程式数と変数数） | ○ | ○ | Must | P2（S005 でクエリ化） | S5。`check` クエリ（MSL は一度だけ読む）。MSL の全例題 566 本に判定を出した（S005）。S008 で `omc checkModel()` と照合し、判定のある 382 本すべてで一致（[omc-compare.md](omc-compare.md)） |
| **シミュレーション前のアサーション（S0〜S10）** | △ | △ | **Must** | P2（S005 で S0〜S6、S007 で S7・S8、S010 で S9・S10 の静的な部分） | `assert_simulatable`。Error があれば `--force` 無しでは実行しない。S005 で S6（構造特異、GM-E6001）と「コンパイラが未対応」の GM-U（[ADR-0007](adr/0007-unsupported-classification.md)）。S007 で S7・S8（[ADR-0009](adr/0009-structural-index-and-init.md)）。`globe check` に S7・S8 の行と `--json` の `s7` / `s8` / `init`。S010 で S9（代数ループ、GM-I9001 / GM-W9001）と S10（パラメータと start 値の評価、experiment annotation。GM-E10001〜GM-E10004 ほか）、`--json` の `s9` / `s10`（[ADR-0012](adr/0012-loops-and-preflight.md)）。S011 で S10 の数値の部分を実行の直前に足した（start 値で評価した t0 の値が有限か GM-W10002、ソルバの名前 GM-E10005 / GM-I10003。[ADR-0013](adr/0013-exec-backend.md)）。S018 で初期化の解き直し（初期化系を 2 回解いて動けば GM-W10004。[ADR-0020](adr/0020-init-resettle-check.md)）。初期化のヤコビ行列の階数は、Rumoca に公開の API が無いので後回し |
| S7: 構造インデックスと状態選択の候補 | ○（Pantelides とダミー微分） | ○（Pantelides） | Must | P2（S007） | Pantelides 法を `DaeStructure` の上で回す（Rumoca に依存しない）。構造インデックスの見込み、微分する方程式と回数、状態選択の候補、動的な状態の数を情報（GM-I7001 / GM-I7002）として出す。打ち切りは GM-E7001 |
| S8: 初期化系の検査 | ○（実行時の初期化。冗長な初期方程式は警告して外す） | △（初期化の計画） | Must | P2（S007、S009） | 初期条件の数と属性で判定し（S007）、S009 から初期化系（DAE とその微分 + 初期方程式 + `fixed = true`）をマッチングして冗長な初期条件と初期条件の無い未知数を名指しする（[ADR-0011](adr/0011-init-system-matching.md)）。過剰は GM-E8001（冗長な初期条件が拘束と重なるか離散変数を含むときは確定できないので GM-W8004）、不足は GM-W8001・GM-W8002（start の無い状態）、値の無いパラメータは GM-W8003。omc と「over specified」で照合する（S008・S009） |
| シミュレーションの実行（`globe simulate`） | ○ | ○ | **Must** | P3（S011 で着手） | S011: Phase A は Rumoca の実行系（`rumoca-sim`）を `globe-mode-exec` に閉じ込めて使い、結果は中立の `SimRun`、失敗は `GM-E11001`〜`GM-E11006`・`GM-U0004` / `GM-U0005` で返す（[ADR-0013](adr/0013-exec-backend.md)）。`globe simulate` は検査（S0〜S10）を通してから実行し、`--output` に CSV を書く。S012 で `--format mat`（MAT v4）、wasm32 でも同じ道で実行できるようにした（`globe-mode-wasm`、Node で確認。[ADR-0014](adr/0014-session-route-wasm.md)） |
| 複数ソルバ（OpenModelica / Dymola の名前を受け付ける） | ○ | △（bdf / rk） | Must | P3（S011・S012）/ P8 | 名前の登録表（`globe-mode-sim::solvers`、S011）。S011 で `bdf`・`dopri45`・`auto`。S012 で自前の積分器 6 つ（`esdirk34`・`trbdf2`・`tsit45`・`euler`・`heun`・`rk4`）を Rumoca の数値積分のプラグインとして差し、9 つになった（[ADR-0014](adr/0014-session-route-wasm.md)）。固定刻みの刻み幅は `--step`。`auto` は BDF が失敗したら dopri45 で回し直す（GM-I11002）。`dassl` などは別名（GM-I10003）、表にあって未実装のもの（`radau5`・`ida` など）は GM-E10005。`globe simulate --list-solvers` で一覧 |
| experiment annotation と実行設定 | ○ | ○ | Must | P3（S011） | 「指定 > experiment annotation > 既定」で決め、出どころを表示する（`globe-mode-sim::settings`、S011）。S012 で許容誤差の下限 1e-10（GM-W10003）と固定刻みの検査（GM-I10004・GM-E10004）。設定は protocol 型で持つ |
| パラメータを変えた再実行（再コンパイル無し） | ○ | △ | Should | P3（S012） | S012: `globe simulate --param NAME=VALUE`（Rumoca の `param_overrides`。DAE は使い回し、lowering と組み立てはやり直す）。変えられるのは tunable なパラメータだけ（`--list-parameters`、`globe_mode_exec::tunable_parameters`）。構造パラメータ・`final`・`Evaluate = true`・定数、無い名前、有限でない値は GM-E10006（[ADR-0014](adr/0014-session-route-wasm.md)）。今後: `executable` を LRU に保持する |
| パラメータスイープ | △ | × | Should | P6 | Streamlit と marimo から使う |
| 結果のプロット（時系列・パラメトリック・大容量） | ○ | △ | Must | P4 | uPlot 系。ResultStore は CSV / MAT v4 / Arrow（S011 で CSV。`globe-mode-sim::store`。S012 で MAT v4（`globe-mode-sim::mat`、OpenModelica・Dymola の dsres の形）。Arrow IPC は持ち越し） |
| 変換デバッガ（インデックス低減、tearing） | ○ | × | Could | P8 | BLT ビュー |
| 線形化（A/B/C/D） | ○ | ○ | Could | P8 | Rumoca の機能をアダプタ経由で使う |

## 4. FMI と SSP

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| FMI 2.0 Import（ME / CS） | ○ | × | **Must** | P5 | `fmi` crate 0.8 の上に作る |
| FMI 3.0 Import（ME / CS / SE） | △ | × | **Must** | P5 | SE はクロックスケジューラで動かす |
| FMI 2.0 Export（ME / CS） | ○ | ○（テンプレート） | **Must** | P5 | P5a は Rumoca のテンプレート、P5b は自前のランタイム |
| FMI 3.0 Export（ME / CS / SE） | △ | ○（テンプレート） | **Must** | P5 | 同上 |
| コシミュレーションのマスター | ○（OMSimulator） | × | **Must** | P5 | Jacobi / Gauss–Seidel から始め、可変刻みとロールバックに進む |
| SSP 2.0 | ○ | × | Should | P8 | 系の構成ファイル |
| FMI-LS-DAE | × | × | Could | — | 規格が alpha の段階なので注視するだけ |

## 5. 実行環境と言語 API

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| Desktop（Debian 系 Linux / Windows 11） | ○ | ×（VS Code） | Must | P4 | Tauri 2 + Svelte 5 |
| Android | × | × | Must | P6 | Tauri 2。計算はネイティブ |
| ブラウザ（閲覧者の端末で計算） | ×（OMWebbook はサーバー側） | ○（playground） | Must | P6（S012 で Node の wasm） | Web Worker で wasm を動かす。サーバーでは計算しない。S012 で `globe-mode-wasm`（wasm32-unknown-unknown + wasm-bindgen）の検査 → 実行を Node で確かめた（`mise run wasm:node`・`wasm:compare`、CI の `wasm-node`）。ブラウザ（`--target web`、Worker）は未試験 |
| CLI | ○（omc） | ○ | Must | P1 | `globe` |
| marimo | × | × | Must | P6 | anywidget に wasm を載せる。代替は PyO3 の wheel |
| Streamlit（結果ダッシュボード） | × | × | Should | P6 | PyO3 の wheel |
| Rust API | — | ○ | Must | P1 | facade crate `globe-mode` |
| Python API | ○（OMPython） | ○ | Must | P6 | PyO3（abi3-py312） |
| Julia API | ○（OMJulia） | × | Must | P7 | C ABI と ccall |
| Elixir / Gleam API | × | × | Must | P7 | Rustler の NIF。Gleam は `@external` |
| Zig API | × | × | Must | P7 | C ABI と cbindgen のヘッダ |
| Leptos / Dioxus 版 UI（負荷の比較用） | — | — | Could | P7 | 試験 feature。`doc/bench/ui.md` に結果を書く |

## 6. 品質

| 機能 | 優先度 | Phase | 方針 |
| --- | --- | --- | --- |
| MSL コーパスのファネル計測（parse → flatten → DAE → balanced → init → simulate） | Must | P1〜（S005 で全例題） | Rumoca と同じ段階で数える。`cargo xtask msl-funnel` が、Rumoca と同じ名前の規則で選んだ 566 本を数える（S005） |
| MSL の参照結果との照合（ファネル） | Must | P3（S011） | `cargo xtask msl-sim`（`mise run msl:sim`）が `msl:funnel` で OK の例題を別プロセスで実行し、MAP-LIB の参照結果（v4.1.0）と csv-compare の矩形の管（許容差 0.002）で比べる。報告は [msl-sim.md](msl-sim.md)（[ADR-0013](adr/0013-exec-backend.md)） |
| Rumoca との差分照合 | Must | P1〜 | Track B に移ってからも続ける |
| `omc` との照合（別プロセス） | Should | P2〜（S008 で実施） | 照合用に限る。リンクしない。`cargo xtask omc-compare`（`mise run msl:omc`、[ADR-0010](adr/0010-omc-comparison.md)）が `msl-funnel --json` の結果を omc の `checkModel` / `translateModel` と突き合わせ、報告を [omc-compare.md](omc-compare.md) に書く |
| Reference-FMUs、FMPy、fmusim、fmuCheck | Must | P5 | FMI の Import と Export の両方 |
| ライセンス監査（cargo-deny、npm、SBOM） | Must | P1〜 | [licenses.md](licenses.md) |

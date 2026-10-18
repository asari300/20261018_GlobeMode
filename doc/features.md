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
| 検査に渡すファイルの絞り込み（依存の閉包） | — | × | Should | P2（S006） | `deps_closure` クエリ。ワークスペースのうちモデルが使うファイルだけを Rumoca に渡し、使わないファイルの編集では再検査しない。`globe deps <files> -m <model>` で閉包を表示する（例: `Connect.RealCircuit` は 5 / 23 ファイル） |
| ライブラリツリー | ○ | × | Must | P4 | `class_index`（`package.order`） |
| パッケージマネージャ（`uses` の版） | ○ | × | Could | P8 | 版の不一致は S1 の検査で警告する |
| **connect のネットワークグラフ（Obsidian 風）** | × | × | **Must** | P2（データは S004 で最初の版）/ P4（描画） | flatten 不要の `connect_graph(path, depth)`。sigma.js + ForceAtlas2。巨大なグラフには cosmos.gl |
| incidence グラフ・BLT の可視化 | △（変換デバッガ） | × | Should | P8 | `dae_structure` を同じグラフ基盤で描く |
| フラットモデルの表示（Instantiate） | ○ | ○ | Should | P2（S005 で要約） | `globe flat`（S005）: 変数・方程式の数、部品ごとの方程式数（`flat_summary` クエリ）。フラットモデルの本文の表示は後回し |

## 3. 検査とシミュレーション

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| Check Model（方程式数と変数数） | ○ | ○ | Must | P2（S005 でクエリ化） | S5。`check` クエリ（MSL は一度だけ読む）。MSL の全例題 566 本に判定を出した（S005）。`omc checkModel()` との照合は omc を入れてから |
| **シミュレーション前のアサーション（S0〜S10）** | △ | △ | **Must** | P2（S005 で S0〜S6、S006 で S7・S8） | `assert_simulatable`。Error があれば `--force` 無しでは実行しない。S005 で S6（構造特異、GM-E6001）と「コンパイラが未対応」の GM-U（[ADR-0007](adr/0007-unsupported-classification.md)）。S006 で S7・S8（[ADR-0008](adr/0008-structural-index-and-init.md)）。`globe check` に S7・S8 の行と `--json` の `s7` / `s8` / `init` |
| S7: 構造インデックスと状態選択の候補 | ○（Pantelides とダミー微分） | ○（Pantelides） | Must | P2（S006） | Pantelides 法を `DaeStructure` の上で回す（Rumoca に依存しない）。構造インデックスの見込み、微分する方程式と回数、状態選択の候補、動的な状態の数を情報（GM-I7001 / GM-I7002）として出す。打ち切りは GM-E7001 |
| S8: 初期化系の検査 | ○（実行時の初期化） | △（初期化の計画） | Must | P2（S006） | 初期条件の数と属性で判定する。過剰は GM-E8001（離散変数を含むか、インデックスの低減で状態が減るときは確定できないので GM-W8004）、不足は GM-W8001・GM-W8002（start の無い状態）、値の無いパラメータは GM-W8003。初期方程式の incidence でのマッチングは S007 以降 |
| 複数ソルバ（OpenModelica / Dymola の名前を受け付ける） | ○ | △（bdf / rk） | Must | P3 / P8 | `trait Integrator` と名前の登録表 |
| experiment annotation と実行設定 | ○ | ○ | Must | P3 | 設定は protocol 型で持つ |
| パラメータを変えた再実行（再コンパイル無し） | ○ | △ | Should | P3 | `executable` を LRU に保持する |
| パラメータスイープ | △ | × | Should | P6 | Streamlit と marimo から使う |
| 結果のプロット（時系列・パラメトリック・大容量） | ○ | △ | Must | P4 | uPlot 系。ResultStore は CSV / MAT v4 / Arrow |
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
| ブラウザ（閲覧者の端末で計算） | ×（OMWebbook はサーバー側） | ○（playground） | Must | P6 | Web Worker で wasm を動かす。サーバーでは計算しない |
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
| Rumoca との差分照合 | Must | P1〜 | Track B に移ってからも続ける |
| `omc` との照合（別プロセス） | Should | P2〜 | 照合用に限る。リンクしない |
| Reference-FMUs、FMPy、fmusim、fmuCheck | Must | P5 | FMI の Import と Export の両方 |
| ライセンス監査（cargo-deny、npm、SBOM） | Must | P1〜 | [licenses.md](licenses.md) |

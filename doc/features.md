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
| 補完・定義ジャンプ・参照・リネーム・ホバー | △（S053: 補完は予約語と `Outline` の名前、`.` の後ろは部品の型の部品名。S132: 定義ジャンプは型・部品名・点つきの名前・extends・import を文書とライブラリの別ファイルへ、参照は同じクラスの中の字句の一致） | ○ | Must | P4 | `globe-mode-lsp`（Transport を抽象化）。S038 で stdio の `globe lsp`（診断・documentSymbol・hover）、S053 で補完、S059 で VS Code 拡張がつなぐ。S132 で LSP の定義ジャンプ・参照（VS Code から使える）、S138 でブラウザ UI・Desktop のエディタからの定義ジャンプ、S149 で同じ解決のホバー（名前の上に 300 ms で宣言の 1 行と説明文字列。`globe/hover`・wasm の `hover`。素のテキスト）。参照の UI は未、リネームは未着手 |
| アウトライン・inlay hint・call hierarchy | △ | ○ | Should | P4（S038 でアウトラインだけ） | `outline` クエリの結果をそのまま使う。S038 で LSP の documentSymbol（VS Code のアウトラインは S059 の拡張で出る）。inlay hint・call hierarchy は未 |
| 整形・lint | ○（S053。VS Code は S059 の拡張で標準の整形キー） | ○ | Should | P4 | 自前の `globe-mode-fmt`（依存 0。[format-style.md](format-style.md)、[ADR-0024](adr/0024-own-formatter-and-lint.md)）。CLI `globe fmt` / `globe lint`、LSP、wasm、ブラウザ UI、ウェブアプリ群（[web-apps.md](web-apps.md)）。S060 で CLI の走査を ruff に倣う（パスを省くとカレントディレクトリ、`.gitignore` を除く）、S061 で節の前後の空行と annotation の並べ替え（lint `GM-L0004`）、S062 で lint `connect-type-mismatch`（`GM-L0005`）と `Documentation` の HTML の整形。VS Code 拡張は 0.0.4。S130 で設定ファイル `globe-mode.toml`（`[fmt]`・`[lint] disable`。CLI と LSP が読む。[format-style.md](format-style.md) の 5 章）。S143 で Desktop も読む（sidecar の根は `--root`・`GLOBE_ROOT`・カレントディレクトリ。UI の整形・lint は設定に任せ、「設定を読み直す」で `workspace/didChangeConfiguration`。ブラウザ版は設定ファイルが無く固定値のまま） |
| Text ビュー | ○ | ○（VS Code） | Must | P4（S041） | CodeMirror 6。S041 でブラウザ UI のエディタと検査パネル（診断を押すとエディタの範囲へ）、S053 で補完と lint の波線。S138 で F12 と Ctrl/Cmd+クリックの定義ジャンプ（`Backend.definition`。ブラウザは wasm、Desktop は sidecar の `globe/definition`。S149 でホバー（`Backend.hover`、CodeMirror の `hoverTooltip`）。S151 でタブ（文書ごとの `EditorState`、閉じるときの確認、定義ジャンプはタブを開く）と最近使ったファイル（上限 10、自動保存はタブ全体）。S152 でダークモード（OS の `prefers-color-scheme` に追従し、手動で 明 / 暗 / OS に合わせる を選べる。配色は CSS の変数 `src/Svelte/src/theme.css`。CodeMirror・uPlot・sigma も追従。Leptos 版は S153 から最初から含む）。汎用の LSP クライアントは持たない。Desktop は sidecar の独自の要求、S049） |

| 整形・lint | ○（S053。VS Code は S059 の拡張で標準の整形キー） | ○ | Should | P4 | 自前の `globe-mode-fmt`（依存 0。[format-style.md](format-style.md)、[ADR-0024](adr/0024-own-formatter-and-lint.md)）。CLI `globe fmt` / `globe lint`、LSP、wasm、ブラウザ UI、ウェブアプリ群（[web-apps.md](web-apps.md)）。S060 で CLI の走査を ruff に倣う（パスを省くとカレントディレクトリ、`.gitignore` を除く）、S061 で節の前後の空行と annotation の並べ替え（lint `GM-L0004`）、S062 で lint `connect-type-mismatch`（`GM-L0005`）と `Documentation` の HTML の整形。VS Code 拡張は 0.0.4。S130 で設定ファイル `globe-mode.toml`（`[fmt]`・`[lint] disable`。CLI と LSP が読む。[format-style.md](format-style.md) の 5 章） |
| Text ビュー | ○ | ○（VS Code） | Must | P4（S041） | CodeMirror 6。S041 でブラウザ UI のエディタと検査パネル（診断を押すとエディタの範囲へ）、S053 で補完と lint の波線。S138 で F12 と Ctrl/Cmd+クリックの定義ジャンプ（`Backend.definition`。ブラウザは wasm、Desktop は sidecar の `globe/definition`。汎用の LSP クライアントは持たず、ホバーなどは未。Desktop は sidecar の独自の要求、S049）。**S141 で「開く」「保存」**（自分の `.mo` を持ち込み・持ち帰る。ブラウザ・Android は `<input type=file>` と Blob のダウンロード、Desktop は dialog / fs プラグインで「開く」「保存」「名前を付けて保存」。本文とファイル名は `localStorage` に自動保存して起動時に戻す（モデル名は本文から入れ直す））。複数の文書（タブ）・最近使ったファイルは未 |
| Diagram / Icon ビュー（Placement / Line annotation） | ○ | × | Should | P8 | Svelte Flow。テキストと双方向に同期する |
| Documentation ビュー（HTML annotation） | ○ | × | Could | P8 | サニタイズした HTML を表示する |
| パラメータダイアログ（modifier、redeclare の選択） | ○ | × | Should | P8 | `class_interface` クエリから作る |

## 2. ライブラリと構造の可視化

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| MSL の読み込み（4.1.0） | ○ | ○ | Must | P1（S003 で済） | `cargo xtask fetch-msl` で `data/msl/` に取得し、`globe check --library` で読む。S005 から、読んだ Session を `Checker` で使い回す。必要になったファイルだけ読む道は、wasm が S037（分割読み込み、全例題で全部読みと一致は S047）、ネイティブの CLI・`globe lsp`・Desktop が S054（閉包だけ。`ChuaCircuit` の最大 RSS 1,493 → 128 MiB、[ADR-0025](adr/0025-native-library-closure.md)） |
| 検査に渡すファイルの絞り込み（依存の閉包） | — | × | Should | P2（S007） | `deps_closure` クエリ。ワークスペースのうちモデルが使うファイルだけを Rumoca に渡し、使わないファイルの編集では再検査しない。`globe deps <files> -m <model>` で閉包を表示する（例: `Connect.RealCircuit` は 5 / 23 ファイル） |
| ライブラリツリー | ○ | × | Must | P4 | `class_index`（`package.order`）。**S127 で UI**: 「MSL を読む」の後に左の木（`LibraryTree.svelte`）。展開で直下の子だけを wasm の `library_tree`・LSP の `globe/libraryTree` から引き、モデルを押すとそのファイルをエディタに開いてモデル名を入れる（検査・実行・プロットへ続く）。並びは名前の順、単位はファイル。検索・アイコンは未 |
| パッケージマネージャ（`uses` の版） | ○ | × | Could | P8 | 版の不一致は S1 の検査で警告する |
| **connect のネットワークグラフ（Obsidian 風）** | × | × | **Must** | P2（データは S004 で最初の版）/ P4（描画は S046） | flatten 不要の `connect_graph(path, depth)`。sigma.js + ForceAtlas2。巨大なグラフには cosmos.gl。S046 で `Graph.svelte`（sigma 4 + graphology、深さ 1〜3、ノードのクリックでエディタへ）。合成 5k ノードはヘッドレス Chromium で約 10 fps（P4 の完了条件 30 fps は実 GPU で未測定）。S079 でライブラリのモデル（MSL の `ChuaCircuit` が 7 ノード）、S081 で文書のモデルが使うライブラリの部品、S089 で import の解決と索引の持ち越し。S101 で継承したクラスの入れ子のクラスも引く（MSL のモデル 2,002 本の未解決 321 → 320） |
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
| 複数ソルバ（OpenModelica / Dymola の名前を受け付ける） | ○ | △（bdf / rk） | Must | P3（S011・S012）/ P8 | 名前の登録表（`globe-mode-sim::solvers`、S011）。S011 で `bdf`・`dopri45`・`auto`。S012 で自前の積分器 6 つ（`esdirk34`・`trbdf2`・`tsit45`・`euler`・`heun`・`rk4`）を Rumoca の数値積分のプラグインとして差し、9 つになった。S026 で固定刻みの陰的な `impeuler`（1 次）・`trapezoid`（2 次）を足して 11 になった（1 歩ごとに Newton 法）（[ADR-0014](adr/0014-session-route-wasm.md)）。S027 で刻み幅の制御のある `esdirk23`（2(3) 次。TR-BDF2 と同じ方法で、刻みの端の値は `trbdf2` と一致する）・`esdirk45`（5(4) 次）・`sdirk34hw`（4(3) 次）を足して 14 になった（diffsol の SDIRK に自前の係数表。係数は次数の条件のテストで検査。出典は architecture.md §3.3）。S028 で陽的な `dopri853`（8 次、13 段 FSAL。diffsol の陽的 RK に自前の係数表。刻み幅の制御は 5 次の埋め込みだけで `dop853.f` と違う）を足して 15 になった。S029 で連続の陽的 RK `cerk23`・`cerk34`・`cerk45`（Owren–Zennaro の 3(2)・4(3)・5(4)。刻みの間の補間も主解と同じ次数）を足して 18 になった。S030 で自前の Radau IIA `radau5`（3 段 5 次、刻み幅は自動。3 次の埋め込みで誤差を見積もる）を足して 19 になった。S031 で自前の量子化状態法 `qss`（QSS2。時刻は量子の事象で決まる）を足して 20 になった。S032 で SUNDIALS の `cvode`（BDF、可変次数）を feature `sundials` を立てたビルドだけに足した（そのビルドでは 21）。S034 で同じ feature に SUNDIALS の `ida`（残差 y' − f を解く可変係数の BDF）を足した（そのビルドでは 22）。固定刻みの刻み幅は `--step`。`auto` は BDF が失敗したら dopri45 で回し直す（GM-I11002）。`dassl` などは別名（GM-I10003）、表にあって未実装のもの（feature を立てないビルドの `cvode`・`ida`）は GM-E10005。`globe simulate --list-solvers` で一覧。S126 で `msl-sim.yml` を 14 ソルバすべてで回し直し、[msl-solvers.md](msl-solvers.md) を S119 の後の版にした（`auto` は 312・管 303 / 311） |
| experiment annotation と実行設定 | ○ | ○ | Must | P3（S011） | 「指定 > experiment annotation > 既定」で決め、出どころを表示する（`globe-mode-sim::settings`、S011）。S012 で許容誤差の下限 1e-10（GM-W10003）と固定刻みの検査（GM-I10004・GM-E10004）。設定は protocol 型で持つ S142 で UI（`SimSettings.svelte`）: ソルバ（`Backend.solvers()`＝wasm の `solvers`・sidecar の `globe/solvers`）・終了時刻・許容誤差・出力の間隔・パラメータの上書き（`名前=値` の行）。experiment の値は検査（`CheckResponse.experiment`）で初期値と出どころ「experiment」に出て、触った欄だけが「指定」で `settings` に入る。直近の実行の値と出どころも表示。S150 でパラメータは一覧から選ぶ（`Backend.tunableParameters(model)`＝wasm の `tunable_parameters`・sidecar の `globe/tunableParameters`。検査のあとに取り、名前・単位・説明と値の欄、13 本以上は絞り込み。値を打った行だけ `parameters` に入る） |
| パラメータを変えた再実行 | ○ | △ | Should | P3（S012） | S012: `globe simulate --param NAME=VALUE`（Rumoca の `param_overrides`。DAE は使い回し、lowering と組み立てはやり直す）。変えられるのは tunable なパラメータだけ（`--list-parameters`、`globe_mode_exec::tunable_parameters`）。構造パラメータ・`final`・`Evaluate = true`・定数、無い名前、有限でない値は GM-E10006（[ADR-0014](adr/0014-session-route-wasm.md)）。S024 から、上書きがあればライブラリだけの session を複製して上書きを組み込んでからコンパイルし直す（同じ名前の修飾で写された値や時間イベントの時刻に、実行時の上書きが届かず黙って誤ったため。MSL の回路で約 2 s 増える。[ADR-0022](adr/0022-structural-parameter-overrides.md)）。変えられる名前の範囲は同じ。今後: `executable` を LRU に保持する S025: 配列の要素（`a[2]`）は実行時の上書きで届き、配列全体と、同じ名前の修飾で内側へ写された配列の要素は GM-E10006。 |
| パラメータスイープ | △ | × | Should | P6 | Streamlit と marimo から使う |
| 結果のプロット（時系列・パラメトリック・大容量） | ○ | △ | Must | P4（S042） | uPlot 系。S042 でブラウザ UI の `Plot.svelte`（uPlot、信号のチェックボックス）、S048 で MSL の `ChuaCircuit` を実行して描けた。結果は列ごとの道（wasm は S051、ブラウザ UI は S065、Desktop は S076 の `globe/simulateColumns`、Python は S085 の `simulate_columns`。marimo の `sim_app.py` は S078 の anywidget で描き、S085 から列の道）。S128 で系列の選択（選んだ系列だけを uPlot に渡す）・2 軸（系列ごとに右の軸）・パラメトリック（x に時刻以外の列。拡大は止める）。S146 で書き出し（CSV＝選んだ系列と x の列、PNG＝canvas を白地に。`Backend.saveBinary?`）。MAT v4 の読み込み・凡例つきの画像・SVG は未。ResultStore は CSV / MAT v4 / Arrow（S011 で CSV。`globe-mode-sim::store`。S012 で MAT v4（`globe-mode-sim::mat`、OpenModelica・Dymola の dsres の形）。Arrow IPC は持ち越し） |
| 変換デバッガ（インデックス低減、tearing） | ○ | × | Could | P8 | BLT ビュー |
| 線形化（A/B/C/D） | ○ | ○ | Could | P8 | Rumoca の機能をアダプタ経由で使う |

## 4. FMI と SSP

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| FMI 2.0 Import（ME / CS） | ○ | × | **Must** | P5 | `fmi` crate 0.8 の上に作る |
| FMI 3.0 Import（ME / CS / SE） | △ | × | **Must** | P5 | SE はクロックスケジューラで動かす |
| FMI 2.0 Export（ME / CS） | ○ | ○（テンプレート） | **Must** | P5 | P5a は Rumoca のテンプレート、P5b は自前のランタイム。**S094: P5a の FMI 2.0（ME・CS）を `globe fmu export` で出せる**（事象の無いモデル。[ADR-0031](adr/0031-fmi-export-p5a-first.md)）。S095: P5b の crate `globe-mode-fmu`（依存 0）が FMI 2.0 ME の 35 関数を出す（モデルは Decay を固定、任意のモデルの載せ替えは未。[ADR-0032](adr/0032-p5b-own-runtime.md)）。S109: 事象（状態事象・`reinit`・`sample`）のあるモデルも FMPy の ME で回る。S112: 代数・離散の変数（Real・Integer・Boolean）も FMU の外から読める。S114: Real の入力を `fmi2SetReal` で与えられる（FMPy の `input=`）。S105: `--build --target` で `zig cc` を呼び、1 つの FMU に Linux・Windows・macOS の置き場を同梱（P5a）。**S144: UI の「FMU を書き出す」**（ソースの FMU、FMI 2.0）。ブラウザ・Android は wasm が zip を `Uint8Array` で返してダウンロード、Desktop は LSP の `globe/exportFmu` が `outDir` に書く（`--build` は UI に無い） |
| FMI 3.0 Export（ME / CS / SE） | △ | ○（テンプレート） | **Must** | P5 | 同上。**S098: P5a の FMI 3.0（ME・CS）を `globe fmu export --fmi-version 3` で出せる**（SE は未）。S105 で 3.0 も複数の OS の置き場を同梱。**S137: P5b の `globe-mode-fmu` も同じ cdylib で FMI 3.0 ME を出す**（`fmi3*`、事象・代数・離散・入力のあるモデルも FMPy の validate と ME で 2.0 と同じ値。CS・SE は未） |
| コシミュレーションのマスター | ○（OMSimulator） | × | **Must** | P5 | Jacobi / Gauss–Seidel から始め、可変刻みとロールバックに進む |
| SSP 2.0 | ○ | × | Should | P8 | 系の構成ファイル |
| FMI-LS-DAE | × | × | Could | — | 規格が alpha の段階なので注視するだけ |

## 5. 実行環境と言語 API

| 機能 | OMEdit | Rumoca | 優先度 | Phase | GlobeMode での方針 |
| --- | --- | --- | --- | --- | --- |
| Desktop（Debian 系 Linux / Windows 11） | ○ | ×（VS Code） | Must | P4（S049・S056・S069・S084・S135・S136・S143・S147） | Tauri 2 + Svelte 5。S049 で `globe lsp` を sidecar にしたシェルと `desktop.yml` の .deb・.msi。S056 で MSL を読んで `ChuaCircuit` の検査・実行（RSS の基準値 sidecar 1.52 GiB・アプリ 136 MiB）。S069 で MSL を初回に取得（[ADR-0030](adr/0030-desktop-msl-placement.md)）、S084 で 4.1.0 / 4.0.0 の切り替えと取得の進捗・取り消し。S076 で結果を列で受ける。S136 で実行の進捗と「止める」（LSP の `$/progress`・`$/cancelRequest`。S135 で自動更新（`tauri-plugin-updater`。`release.yml` が .deb・.msi を Release に付け、secret `TAURI_SIGNING_PRIVATE_KEY` があるときだけ署名して `latest.json` を作る。UI に「更新を確認」）。S134 で Linux の locale が `C.UTF-8` でも白画面にならない（起動時に `en_US.UTF-8` に替える）。S143 で作業ディレクトリ（`--root`・`GLOBE_ROOT`）の `globe-mode.toml` を整形・lint に使い、UI の「設定を読み直す」で読み直す（`desktop.yml` の測定が `cfg=ok` を確かめる）。S147 で `desktop.yml` の UI の測定が UI の操作 7 項目（木・整形・グラフ・止める・実行の設定・プロットの選択・定義ジャンプ。Linux・Windows）も確かめる。鍵と secret の登録と、実機での更新の確認は未 |
| Android | × | × | Must | P6（S092・S100・S103・S147） | Tauri 2。sidecar を使えないので計算は `WorkerBackend`（wasm。[ADR-0023](adr/0023-desktop-and-web-process-layout.md)）。S092 で debug APK（`android.yml`）、S100 で API 34 のエミュレータで Decay の検査と実行、S103 で APK に MSL を同梱し ChuaCircuit の検査が通る。S106 で release の APK を ABI ごとに作る（aarch64 12.1 MiB。secret があるときだけ署名、smoke は使い捨ての鍵）。S147 で emulator の smoke が UI の操作 7 項目（木・整形・グラフ・止める・実行の設定・プロットの選択・定義ジャンプ）も確かめる（debug・release とも ok）。secret を登録した署名つき APK の実機確認は未 |
| ブラウザ（閲覧者の端末で計算） | ×（OMWebbook はサーバー側） | ○（playground） | Must | P6（S012 で Node の wasm） | Web Worker で wasm を動かす。サーバーでは計算しない。S012 で `globe-mode-wasm`（wasm32-unknown-unknown + wasm-bindgen）の検査 → 実行を Node で確かめた（`mise run wasm:node`・`wasm:compare`、CI の `wasm-node`）。S036 で `--target web` の糊と Web Worker（`src/Rust/mod/globe-mode-wasm/web/`）で回し、Node の `worker_threads` と Playwright の Chromium で Decay を確かめた（`mise run wasm:web`。panic の後は Worker を作り直して再生する）。S037 で MSL の分割読み込み（使うファイルだけを検査のときに読む）を既定にし、読み込み後の線形メモリを 262 → 62 MiB にした。S041 で Svelte の UI（`WorkerBackend`）、S048 で UI から MSL を読んで `ChuaCircuit` の検査・実行・プロット。S093 で CSP `connect-src 'self'` のまま実行でき、外への要求 0 件を確かめた。S133 で実行中の進捗（段と t / 終了時刻の割合）と「止める」（Worker を作り直す。[ADR-0033](adr/0033-progress-and-cancel.md)）。**S145 で配布物**: 手動の `web.yml` が wasm・UI・`MSL.json`・CSP の header の断片（`_headers`・`nginx.csp.conf`）を artifact `globe-web` に出す。`base: "./"` でサブパスに置いても動き（`test_ui.py` が `/app/` で確かめる）、幅 400 px で崩れない。COOP / COEP は不要。公開（Pages の有効化・URL）はユーザーの判断で未実施 |
| CLI | ○（omc） | ○ | Must | P1 | `globe` |
| marimo | × | × | Must | P6（S078・S102） | anywidget に wasm を載せる。代替は PyO3 の wheel。anywidget は extra `widget`（S097）。S102 で marimo の WASM（Pyodide）から `globe-mode-wasm` で Decay を回した（ブラウザの RSS 2 GB 未満）。WASM で MSL を渡す道は未 |
| Streamlit（結果ダッシュボード） | × | × | Should | P6 | PyO3 の wheel |
| Rust API | — | ○ | Must | P1 | facade crate `globe-mode` |
| Python API | ○（OMPython） | ○ | Must | P6 | PyO3（abi3-py312）。S129 で `simulate`・`simulate_columns` に `on_progress`（進捗の dict、`True` か Ctrl-C で中断。実行中は GIL を離す。[ADR-0033](adr/0033-progress-and-cancel.md)） |
| Julia API | ○（OMJulia） | △（S075: `ccall` だけの `src/Julia/globe_mode.jl`。パッケージ・JLL は未） | Must | P7 | C ABI（`globe-mode-c`）と ccall |
| Elixir / Gleam API | × | △（S082: Elixir の `src/Elixir/globe_mode`、S087: Gleam の `src/Gleam/globe_mode`（Erlang target）。Hex での配布・Gleam の JS target は未） | Must | P7 | Rustler の NIF（`globe-mode-nif`、Erlang モジュール `globe_mode_nif`）。Gleam は `@external` |
| Zig API | × | △（S075: `src/Zig` の例。手書きのヘッダを translate-c） | Must | P7 | C ABI（`globe-mode-c`）と手書きのヘッダ `globe_mode.h` |
| 言語 API の機能表（参照シナリオ） | — | ○ | Must | P7（S110・S111・S116・S118・S131・S133・S136） | [調査](research/lang-api-matrix-2026-11-03.md)。C・Zig・Julia・Elixir・Gleam・Python は open・check・simulate・graph・format・lint がある（format・lint と Julia の graph・`load_dir` は S111）。列の道（`simulate_columns`）は S116 で C・Zig・Julia・Elixir・Gleam にも入り、Python・wasm と同じ並び（facade の `Simulated::columns`）。FMU export は S118 で facade（feature `fmu`）の `export_fmu` に置き、C・Zig・Julia・Elixir・Gleam・Python から呼べる（wasm はファイルを書けないので `export_fmu` は対象外だが、S144 の `export_fmu_bytes`（zip のバイト列。build 無し）は wasm の `export_fmu` になった。CLI の `globe fmu export` も facade を呼ぶ）。進捗と中断は S121 の [ADR-0033](adr/0033-progress-and-cancel.md) で設計し、S123 で facade に、S129 で Python に、S131 で C・Zig・Julia（コールバック）と Elixir・Gleam（pid へのメッセージと `cancel(token)`）に入れ、S133 で wasm とブラウザ UI に、S136 で LSP と Desktop に入れた。参照シナリオ（Decay → check → simulate → x(1) → format）は S111 で 6 言語の試験に入り、6 言語・wasm・LSP・CLI で通る |
| Leptos / Dioxus 版 UI（負荷の比較用） | — | — | Could | P7 | 試験 feature。`doc/bench/ui.md` に結果を書く。**Leptos 版は S153 で作った**（textarea・検査・実行・MSL・ダークモード、UI と計算が 1 つの wasm。[ADR-0035](adr/0035-experimental-ui-leptos.md)）。**Dioxus 版は S155 で作った**（Leptos 版と同じ機能・同じ測り方。Dioxus 0.7.10、`dx` なし。[ADR-0036](adr/0036-experimental-ui-dioxus.md)）。Windows・macOS の列と CI は未 |

## 6. 品質

| 機能 | 優先度 | Phase | 方針 |
| --- | --- | --- | --- |
| MSL コーパスのファネル計測（parse → flatten → DAE → balanced → init → simulate） | Must | P1〜（S005 で全例題） | Rumoca と同じ段階で数える。`cargo xtask msl-funnel` が、Rumoca と同じ名前の規則で選んだ 566 本を数える（S005） |
| MSL の参照結果との照合（ファネル） | Must | P3（S011） | `cargo xtask msl-sim`（`mise run msl:sim`）が `msl:funnel` で OK の例題を別プロセスで実行し、MAP-LIB の参照結果（v4.1.0）と csv-compare の矩形の管（許容差 0.002）で比べる。報告は [msl-sim.md](msl-sim.md)（[ADR-0013](adr/0013-exec-backend.md)） |
| Rumoca との差分照合 | Must | P1〜 | Track B に移ってからも続ける。見つけた Rumoca の穴は [upstream.md](upstream.md) の 1 つの表にまとめる（S096）。S108 で下書き 31・32、S113・S115 で 17〜32 を Rumoca v0.10.4 で確かめ直した（直った件は提出の順から外す）。01〜16 の再確認は未 |
| `omc` との照合（別プロセス） | Should | P2〜（S008 で実施） | 照合用に限る。リンクしない。`cargo xtask omc-compare`（`mise run msl:omc`、[ADR-0010](adr/0010-omc-comparison.md)）が `msl-funnel --json` の結果を omc の `checkModel` / `translateModel` と突き合わせ、報告を [omc-compare.md](omc-compare.md) に書く |
| Reference-FMUs、FMPy、fmusim、fmuCheck | Must | P5 | FMI の Import と Export の両方。Export した Decay の FMU は FMPy（S094・S098）と fmusim（S098、`fmu-export.yml`）の validate と ME・CS の simulate を 2.0・3.0 とも通る。S105 で Windows・macOS（arm64）の FMPy も validate を通す（Windows・macOS の fmusim は未）。fmuCheck は未 |
| ライセンス監査（cargo-deny、npm、SBOM） | Must | P1〜 | [licenses.md](licenses.md) |

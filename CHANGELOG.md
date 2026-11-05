# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/); versions will follow [Semantic Versioning](https://semver.org/lang/ja/) from v0.2.0. 日本語で書く（英語の要約は [README.md](README.md)）。各行の S 番号は [doc/sessions/README.md](doc/sessions/README.md) の索引と `log/work/` の作業記録に対応する。

## [Unreleased]

**次の版は 0.2.0**（S172 で Cargo・Tauri・Python・npm・VS Code の拡張・Elixir・Gleam の版を 0.2.0 に揃えた。一覧は [architecture.md](doc/architecture.md) の「版を書いている場所」）。S001〜S179 の要点（S048 以降は S168 で、S166〜S179 は S179 で足した。内部の整理は束ねた）。日付と比較リンクは公開のときに入れる。GitHub の Release・タグ・PyPI・crates.io・VS Code の Marketplace のどれにも出していない（crates.io に出せないのは Rumoca の git 依存のため。[ADR-0002](doc/adr/0002-rumoca-adapter.md)）。

### 運用・計画（P0）

- S001: 調査と計画、運用基盤（worktree、mise・uv・cargo の分担、ライセンス規則、セッション記録）。
- S006: WSL2 から Windows の LM Studio（Bonsai-27B）を副レビュアーに呼ぶスキル `/local-review`。
- S033: リリースの見積もりと、エージェントに渡す並行作業の指示書 WP-0〜WP-J（`doc/plan/`）、Codex の監査のための `log/review/`。

### P1 骨格

- S002: workspace の骨格、Rumoca を `Frontend` のアダプタで包む、cargo-deny、Playwright。
- S003: MSL 4.1.0 の例題 20 本を Rumoca の CLI と突き合わせて 20 本一致（P1 完了）。

### P2 遅延クエリと検査

- S004: salsa の遅延クエリと `connect()` のグラフ、検証用パッケージ `GlobeModeValidation`。
- S005: `check` のクエリ化（`source_shape` の防火壁）、GM-U（コンパイラの限界）の振り分け（[ADR-0007](doc/adr/0007-unsupported-classification.md)）、S6（マッチングと Dulmage–Mendelsohn 分解）。MSL の全例題 566 本に判定。
- S007: `deps_closure`（検査に渡すファイルの絞り込み）、S7（Pantelides 法と状態選択の候補）、S8（初期化系の検査）。
- S008: omc 1.27.1 との照合。S5 は判定のある 382 本すべてで一致（P2 完了。[ADR-0010](doc/adr/0010-omc-comparison.md)）。
- S009: 初期化系のマッチングで冗長な初期条件と初期条件の無い未知数を名指し（[ADR-0011](doc/adr/0011-init-system-matching.md)）。
- S010: S9（BLT と tearing による代数ループ、非線形の反復変数の start 値）、S10（プリフライトの静的な部分）。
- S174: 診断が方程式を名指しする所（GM-I7001・GM-E6001 など）で、接続・流れの和・束縛から生まれた方程式の名前に変数の宣言が出ていたのを、残差の式の形（`a.p.v = b.n.v`）に直した。書かれた方程式と診断の範囲は変えていない。

### P3 シミュレーション

- S011: `globe simulate`、実行系を `globe-mode-exec` に閉じ込める（[ADR-0013](doc/adr/0013-exec-backend.md)）、結果の CSV と参照結果の管。MSL の例題 357 本のうち 303 本が終了時刻まで進み、292 / 302 本が管に収まる。
- S012: 検査 → コンパイル → 実行を WebAssembly で（`globe-mode-wasm`、Node 上の bdf、[ADR-0014](doc/adr/0014-session-route-wasm.md)）。`auto`（BDF が失敗したら dopri45）、`--param`、MAT v4。
- S013〜S016: MSL の突き合わせの許容誤差を参照結果と同じにし（[ADR-0015](doc/adr/0015-auto-and-reference-tolerance.md)）、実行の段をワーカーに知らせ（[ADR-0016](doc/adr/0016-preflight-on-build-lowering.md)）、lowering の上限と回し直しを入れた（[ADR-0017](doc/adr/0017-lowering-limit-and-hard-timeout-retry.md)）。S015 で 309 本が実行でき 301 / 308 本が管に収まる。
- S017: Raspberry Pi（aarch64）で回せるようにした（[ADR-0019](doc/adr/0019-rpi-aarch64-environment.md)）。
- S018〜S020: 初期化の解き直しの検査 GM-W10004（[ADR-0020](doc/adr/0020-init-resettle-check.md)・[ADR-0021](doc/adr/0021-resettle-only-when-relations-flip.md)）、重い検査（`msl:funnel`・`msl:sim`・`wasm:node`・`wasm:compare`）の再実行。
- S021〜S023: esdirk34 の詰め（Newton 法の失敗の数え方、ヤコビ行列を取る点、最初の刻みの下限）。全体 356 本で実行 286・管 277。
- S024・S025: `--param` が同じ名前の修飾で写された値と時間イベントに届かず黙って誤っていたのを直した（[ADR-0022](doc/adr/0022-structural-parameter-overrides.md)）。配列は要素だけ受け、配列全体は GM-E10006 で断る。
- S026〜S031: 自前のソルバを足した。固定刻みの陰的 `impeuler`・`trapezoid`、SDIRK の `esdirk23`・`esdirk45`・`sdirk34hw`、`dopri853`、連続の陽的 RK の `cerk23`・`cerk34`・`cerk45`、`radau5`、`qss`（QSS2）。
- S032・S034: feature `sundials` を立てたビルドだけに SUNDIALS 7.1.1 の `cvode` と `ida` を足した（既定・wasm・CI のビルドは変わらない）。
- S036: wasm を Web Worker で（ブラウザの module Worker と Node の `worker_threads` で共用）。panic の後は作り直して再生する。
- S037: wasm の MSL の分割読み込み。読み込み後の線形メモリ 262 → 62 MiB、`ChuaCircuit` の後 513 → 113 MiB。`msl:funnel`・`msl:sim` を GitHub Actions で手動で回すワークフロー。
- S043: 分割読み込みを MSL の全例題で全部読みと突き合わせる道（`tests/split.rs` の `GM_SPLIT_MODELS`・`GM_SPLIT_FRESH`、`msl-sim.yml` のジョブ `split-all`）。
- S047: GitHub Actions の `msl-sim.yml` の結果で `doc/msl-funnel.md`・`doc/msl-sim.md` を更新（OK 369、実行できた 309、管に収まった 301/308、前回との差 0）。分割読み込みは MSL の 566 本すべてで全部読みと一致。
- S050・S126: `msl-sim.yml` にソルバごとの matrix（ジョブ `solvers`）と、実行できた本数・管に収まった本数の表 `doc/msl-solvers.md`。S126 で 14 ソルバすべてを回し直した。`auto` は 312 本が実行でき、311 本のうち 303 本（97.4%）が管に収まる（S119 と同じ）。
- S051: wasm の `simulate_columns`（結果を列ごとの `Float64Array` で。10 万点 × 7 列で 548 → 159 ms）と、メモリを測る `resources.yml`。
- S054: ネイティブの CLI・`globe lsp`・Desktop も MSL は閉包のファイルだけを読む（`ChuaCircuit` の `globe check` の最大 RSS 1,493 → 128 MiB。[ADR-0025](doc/adr/0025-native-library-closure.md)）。
- S058・S063・S066・S067・S068・S071・S073: MSL 以外の公開ライブラリ 7 つ（Buildings・AixLib・IBPSA・ClaRa・ThermofluidStream・DeltaRobot・Chemical）を Actions で検査・実行する `libraries.yml` と表 `doc/libraries.md`。構文の誤りのファイルは読み飛ばして `GM-W1001` で知らせ、MSL 4.0.0 も選べ（`cargo xtask fetch-msl --version 4.0.0`）、`Modelica.Math` の `external "builtin"` も実行できる（[ADR-0029](doc/adr/0029-external-builtin-rewrite.md)）。
- S064: `auto` の回し直しを BDF → esdirk34 → dopri45 にした（[ADR-0028](doc/adr/0028-auto-esdirk34-fallback.md)）。
- S055・S088・S119・S139・S158: 実行できない例題の原因調べ（cerk23 の時間切れ、BDF の `GM-U0005`、サイリスタの事象の費用、SDIRK の退行の修正、sdirk34hw の `GM-E11003`）。
- S070・S072・S080・S083・S091・S096・S108・S113・S115: 公開ライブラリの失敗を最小の再現にし、上流（Rumoca・diffsol）への issue の下書きを `doc/research/upstream/` に、一覧を正典の [doc/upstream.md](doc/upstream.md) にまとめた。Rumoca v0.10.4 で確かめ直した件は提出の順から外した。提出はしていない。

### P4 UI・LSP

- S035: `globe-mode-protocol`（`--json` と wasm の JSON の serde 型）と facade `globe-mode`（`Workspace`）。CLI と wasm はその経由になり、JSON は snapshot 試験で 1 バイトも変えていない。
- S038: stdio の LSP サーバ `globe lsp`（診断・documentSymbol・hover）。
- S040: [ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md) Desktop と Web のプロセス構成（Desktop は `globe` を Tauri の sidecar に、Web は Web Worker のまま）。コードは変えていない。
- S041: `src/Svelte` にブラウザ UI の骨格（Svelte 5 + CodeMirror 6 + `WorkerBackend` + 診断パネル）。wasm に `check` を足した。
- S042: 実行とプロットの UI（「実行」ボタン、uPlot の `Plot.svelte`、CSV の列分け）。npm は uPlot（MIT）を足して 58 パッケージ、GPL・NC は 0。
- S046: connect グラフの表示（wasm の `graph`、sigma 4 + graphology + ForceAtlas2 の `Graph.svelte`、ノードのクリックで `onSelect`）。合成 5k ノード・約 1 万辺はヘッドレスで約 10 fps（参考値）。
- S048・S127: ブラウザ UI で MSL を読み、左のライブラリツリーからモデルを開いて検査・実行・プロットまで通る（ChuaCircuit の検査 error 0、44 信号）。
- S049・S056・S069・S084・S143: Desktop（Tauri 2）。`globe lsp` を sidecar にしたシェル、`desktop.yml` の .deb と .msi。MSL は初回に取得し（[ADR-0030](doc/adr/0030-desktop-msl-placement.md)）、4.1.0 / 4.0.0 を切り替えられ、作業ディレクトリの `globe-mode.toml` を読む。
- S053・S060・S061・S062: 自前の整形器と lint `globe-mode-fmt`（依存 0、インデント 2・80 桁、annotation の並べ替え、節の前後の空行、`Documentation` の HTML）、`globe fmt` / `globe lint`（ruff のようにパスを省くとカレントディレクトリ）、LSP の整形と補完、lint `GM-L0001`〜`GM-L0005`（connect の型の食い違い）。規則は [doc/format-style.md](doc/format-style.md)。
- S059・S163・S165: VS Code の拡張 `src/VSCode/globe-mode`（`globe lsp` につなぐ。整形・lint・検査・補完・構文の色付け。VSIX は `pack_vsix.py` で作る。Marketplace には出していない。S165 で本物の VS Code の中での試験 `vscode.yml`）。
- S065・S076: 実行結果を CSV でなく列ごとに受ける（ブラウザ・Desktop。10 万点で「実行」から戻るまで 810 → 約 220 ms）。
- S079・S081・S089・S101・S122: connect グラフがライブラリのモデル（MSL の部品）も引ける。
- S128・S146: プロットの系列の選択・右の軸・パラメトリック、CSV と PNG の書き出し。
- S130・S132・S138・S149・S161・S166: プロジェクトの設定 `globe-mode.toml`（整形・lint の規則）、LSP と UI のエディタの定義ジャンプ・ホバー・参照の一覧（Shift+F12）・リネーム（F2）。参照とリネームは同じクラスの中の部品と、S166 からクラス名（開いている文書の中。別ファイルのクラス・import の別名は断る）。
- S141・S142・S150・S151・S154: ファイルを開く・保存、実行の設定（ソルバ・終了時刻・許容誤差・間隔・パラメータの上書きと一覧）、タブと最近使ったファイル。
- S152: ダークモード（OS に追従、明 / 暗の手動の選択）。
- S157: 実行を止めたとき、そこまでの列をプロットに残す（[ADR-0033](doc/adr/0033-progress-and-cancel.md)）。
- S167: ライブラリの木にファイルの中の入れ子のクラスを出し、木の上に検索欄（部分一致、Enter と上下）。MSL 4.1.0 の 6,487 件がアウトラインと過不足なく一致、木に出ない例題は 123 本から 0 本。整形器の `extends` つきクラスの括弧の誤りも直した。
- S169: プロットの信号の一覧に絞り込み（部分一致・空白で AND・先頭 200 本）、`$` で始まる内部名を既定で隠し、CSV・PNG のファイル名を `モデル名-日付-時刻` に。
- S170: 進捗・失敗・結果が画面の中に出る（終わったとき 1 回だけスクロール）、2 モデルのファイルを select で選び直せる、モデル名がタブごとに残る、未保存の編集があるタブと同名のファイルを開き直しても編集は消えない。
- S171: エディタの補完を Rust 側につないだ（型の位置・`Modelica.Blocks.` の後ろ・`pin.` の後ろ。打ちかけでも出る。wasm で 1〜4 ms）。
- S173: コンパイラの限界（GM-U）で止まったとき「未対応（モデルの誤りではない）」の要約と色、強行（`GM-E8001` だけ効く）は効くときだけ「それでも実行する」、失敗の文面に最初の error を 1 行。
- S175: キーボードだけで開く・書く・検査・診断へ飛ぶ・実行・止める・保存まで通る（診断と参照の行の button、Ctrl/Cmd+Enter などのショートカット、プロットの `aria-label`、フォーカスの輪郭）。
- S179: 起動直後の案内（保存したタブが無い初回だけの 3 手順。閉じられる）、スキップリンク（エディタへ・結果へ）、タブの矢印キー・Home・End、未保存の印（●）と再読み込み前の確認。
- S180: ライブラリを使うときのモデル名の誤り（`-m PID_Controller` など）に、英語の原文でなく「モデルが見つからない」（`GM-E1002`）と候補を出す。`-L` も FILES も無いときの案内と、`-L` に MSL のサブフォルダを渡したときの「親のフォルダを渡す」案内。
- S182: CLI の使える流れ。`simulate` を `-o` なしで回すと「結果は書いていない」と 1 行（あれば「書いた: パス（n 行）」）。`-o`・`fmu export --out` の書けない出力先は計算の前に分かり、パスと理由を言う（`os error` を出さない）。**挙動が変わる点**: `globe … | head` は終了コード 0 で黙って終わる（今まで 2 や panic）、`check`・`simulate` の人間向けの出力は info の診断を件数だけにする（`-v` で全部。`--json` は変えない）。`--start-time -2` が通る。ヘルプに例（S0〜S10 に直し、内部のセッション番号を消した）。`--force` の診断の重複と、失敗の最後の行に最初の error。`graph` に `-L`。
- S183: Python に CLI と同じ読み取り（`Workspace.outline`・`deps`・`flat`）。C・Zig・Julia・Elixir・Gleam の README に、書いてあるとおりに動く最初の例（検査して実行して最後の値を読む）を置き、Julia の README を実物に合わせた。marimo の `sim_app.py` にライブラリとモデルの入力欄。
- S184: `--force` でコンパイルに失敗したとき同じ診断が 2 回返っていたのを facade の 1 か所で除いた（CLI・LSP・wasm・UI が同じ応答を見る。CLI だけで除いていたのをやめた）。応答に追加の `summary`（検査の最後の 1 行。`simulate` は実行しなかったときだけ）。文面は CLI のまま。
- S186: UI の細部（S185 の監査）。クラス名のリネーム（F2）がいまのモデル名のクラスなら欄も追い（Ctrl+Z で戻る）、ライブラリの木・検索から入れ子のクラスを開くと宣言の行へカーソルと画面が動く（`librarySource` に追加の `range`）。補完の一覧は Ctrl+Enter で閉じ、「MSL を読む」の後はフォーカスが検索欄へ、「結果へ」は結果を画面へ入れる。
- S188: **挙動が変わる**: ブラウザ・Desktop の UI のエディタで Tab は字下げ（空白 2 つ、選んだ行すべて）、Shift+Tab は戻す。フォーカスをエディタの外へ出すには Esc のあと Tab（Shift+Tab）か Ctrl+M（Mac は Shift+Alt+M）。説明はエディタの下とスクリーンリーダー向けの `aria-describedby`。
- S134・S135: Desktop が `C.UTF-8` の locale で白画面になる不具合の修正と、自動更新（`tauri-plugin-updater`。鍵を登録するまで「未設定」）。
- S140・S164: アプリの使える流れの点検（調査だけ。[2026-11-04](doc/research/app-flow-2026-11-04.md)、[2026-11-05](doc/research/app-flow-2026-11-05.md)）。

### P5 FMI

- S057: FMI の調査と `globe-mode-fmi`（`modelDescription.xml`）の準備。
- S094・S098・S105: `globe fmu export` が FMI 2.0・3.0 のソースの FMU（ME・CS）を出す（Rumoca の `fmi2` / `fmi3` のテンプレート。FMPy と fmusim で検証）。`--build --target <triple>` で Linux・Windows・macOS の共有ライブラリも同梱。
- S095・S099・S109・S112・S114・S137: 自前のランタイム `globe-mode-fmu`（依存 0）。FMI 2.0 ME の関数、モデルの compile、事象、代数・離散の変数、入力、FMI 3.0 ME。S117・S124 で `modelDescription.xml` の書き手を P5a と共有。
- S144・S154: UI の「FMU を書き出す」（ブラウザ・Desktop・Android。FMI 2.0 / 3.0 を選べる）。

### P6 Web・Android・ノートブック

- S078・S085・S097・S102・S104: marimo の anywidget の折れ線（extra `widget`）、Python の `simulate_columns`（列を polars で）、marimo の WASM（Pyodide）から `globe-mode-wasm` を回す（MSL も）。
- S092・S100・S103・S106・S147・S159: Android の APK（`android.yml`）。sidecar は使わず Worker 上の wasm で、MSL を同梱し、API 34 のエミュレータで検査・実行・UI の操作を確かめる。release の APK は ABI ごと。
- S093: ブラウザ版が CSP `connect-src 'self'` のまま実行でき、外への要求が 0 件であることを試験で確かめた。
- S145: ブラウザ版の配布物（`web.yml` が artifact `globe-web` に wasm・UI・`MSL.json`・CSP の header の断片を出す。サブパスで動く）。公開先はまだ無い。
- S160: Desktop の macOS（`desktop.yml` に `macos-latest`。署名・公証なしの .dmg）。

### P8 リリース・運用

- S044: `globe` を linux x86_64・linux aarch64・windows x86_64 で配るワークフロー `release.yml`（タグ `v*` で Release、手動は dry-run、`SHA256SUMS` つき）、この `CHANGELOG.md`、Cargo の metadata、README の Install。
- S045: 上流（Rumoca 11 件・diffsol 3 件）への issue の下書きを `doc/research/upstream/` に 14 件。ci.yml に毎週と手動の全体の検査のときだけ走る `rust-sundials`。
- S052: 配布物に cargo-about で作ったサードパーティのライセンス表 `THIRD-PARTY.md` を同梱（`about.toml`・`about.hbs`、`release.yml` のジョブ `third-party`）。
- S074: `release.yml` で Python の wheel を linux x86_64・aarch64・windows x86_64 の 3 つ作り、作った wheel を入れて pytest を回し、Release と `SHA256SUMS` に足す（wheel の中に `globe-mode-py` の `THIRD-PARTY.md`）。
- S077: ci.yml にジョブ `lang`（C ABI を Zig・Julia から呼ぶ例）。`release.yml` の wheel の版をタグから取り、Linux の wheel を `manylinux_2_39` にした（PyPI に出せる形）。
- S086: Linux の wheel を maturin の `--zig` で `manylinux_2_28` にした（glibc 2.28 以上。Debian 12・RHEL 9 にも pip で入る）。
- S090: Linux の `globe`（tarball）を `cargo-zigbuild` で glibc 2.28 に link した（前はランナーの 2.39。Debian 10・RHEL 8 以降でも動く）。
- S097: anywidget を extra `widget` に移した（`pip install "globe-mode[widget]"`。extra なしの wheel の依存が減る）。
- S135・S148: `release.yml` が Desktop の .deb・.msi も Release に付ける（鍵の secret があるときだけ署名して `latest.json`）。CI の zig の取得をミラー経由でなく ziglang.org から直接にした。
- S176: `release.yml` に macOS（arm64）を足した（`globe` の tar.gz、wheel、Desktop の署名なしの .dmg。鍵があれば `latest.json` に `darwin-aarch64`）。

### P7 言語 API

- S039: Python の wheel `globe_mode`（`globe-mode-py`、PyO3・abi3-py312・maturin）。load_dir → check → graph → simulate → polars の表の参照シナリオを `pytest` で通した。
- S075・S082・S087: C ABI の `globe-mode-c`（9 関数、手書きのヘッダ）と Zig・Julia、Elixir の NIF（`globe-mode-nif`）、Gleam の `@external`。参照シナリオ（Decay を開く → check → simulate → x(1) → format）は `mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test`。
- S110・S111・S116・S118: 公開 API の機能表を作り、6 言語に format / lint、列の道（`simulate_columns`）、FMU の書き出しを足した。
- S181: Python API を厳しくした。**未知の設定のキー（`stoptime` など）は黙って無視されず `TypeError` になる**（前は `ok: True` で既定の値のまま走った。使えるキーを添える）。設定の型の誤りはどの設定かを言う `ValueError`、`Workspace("data/msl")` は 1 つのライブラリ（前は 1 文字ずつ）、ライブラリが読めない `RuntimeError` の本文は `コード: 文`（元の診断は `.diagnostics`）。`export_fmu` の未知の options は `TypeError`。docstring・`py.typed`・`_native.pyi`、`on_progress` は既定のソルバでは積分の中に届かないと明記。
- S121・S123・S129・S131・S133・S136: 実行の進捗と中断をコア・Python・C と他の言語・wasm とブラウザの UI・`globe lsp` と Desktop に入れた（[ADR-0033](doc/adr/0033-progress-and-cancel.md)）。
- S125・S153・S155・S156・S160: UI の負荷の比較。測定ハーネス（[ADR-0034](doc/adr/0034-ui-bench-method.md)）と、Leptos 版・Dioxus 版の試験版（`src/Rust/apps/experimental/`、配布物には入らない）、Linux・Windows・macOS の結果 `doc/bench/ui.md`。

### 既知の限界

- Rumoca は git 依存の固定 rev（v0.10.2）で、crates.io には出せない。MSL 4.1.0 の例題 566 本のうち検査を通るのは 369 本、コンパイラの限界（`GM-U`）が 184 本（[doc/msl-funnel.md](doc/msl-funnel.md)）。
- 配布するバイナリに `sundials` のソルバ（`cvode`・`ida`）は入れない（cmake と C コンパイラが要る）。
- FMI の読み込み、SSP、Diagram / Icon / Documentation ビューは未実装。
- Desktop と Android は署名していない。Desktop の自動更新は鍵を登録するまで「未設定」。実機での長い操作は【未確認】。
- GitHub の Release・タグ・Pages・PyPI・Marketplace への公開はまだ無い（[doc/research/app-flow-2026-11-05.md](doc/research/app-flow-2026-11-05.md)）。

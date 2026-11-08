# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/); versions will follow [Semantic Versioning](https://semver.org/lang/ja/) from v0.2.0. 日本語で書く（英語の要約は [README.md](README.md)）。各行の S 番号は [doc/sessions/README.md](doc/sessions/README.md) の索引と `log/work/` の作業記録に対応する。

## [Unreleased]

**次の版は 0.2.0**（S172 で Cargo・Tauri・Python・npm・VS Code の拡張・Elixir・Gleam の版を 0.2.0 に揃えた。一覧は [architecture.md](doc/architecture.md) の「版を書いている場所」）。S001〜S179 の要点（S048 以降は S168 で、S166〜S179 は S179 で足した。内部の整理は束ねた）。日付と比較リンクは公開のときに入れる。GitHub の Release・タグ・PyPI・crates.io・VS Code の Marketplace のどれにも出していない（crates.io に出せないのは Rumoca の git 依存のため。[ADR-0002](doc/adr/0002-rumoca-adapter.md)）。

### 運用・計画（P0）

- S001: 調査と計画、運用基盤（worktree、mise・uv・cargo の分担、ライセンス規則、セッション記録）。
- S006: WSL2 から Windows の LM Studio（Bonsai-27B）を副レビュアーに呼ぶスキル `/local-review`。
- S033: リリースの見積もりと、エージェントに渡す並行作業の指示書 WP-0〜WP-J（`doc/plan/`）、Codex の監査のための `log/review/`。
- S194: 文書だけ。CLAUDE.md の 2 章をいまのツリーに合わせ、ADR-0032・0034・0035・0036 を Accepted にし、上流の再現用 `.mo` 34 本を `globe fmt` で整形した（診断は前後で同じ）。

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
- S201: **挙動が変わる点**（Worker とプロットのレビューの指摘）。(1) プロット: 同じモデルを再実行したとき、利用者が選び直した信号・右の軸・x 軸・絞り込みの文字・「内部の変数も表示」が残る（前は結果のたびに既定へ戻った）。新しい結果に無くなった信号は外れ、全部無くなれば既定。別のモデル・タブの切り替えは既定から。ボタン「既定に戻す」を足した。(2) wasm の「止める」: 2 回押しても無関係の Worker を殺さない、起動中や実行の順番待ちの間に押しても Worker が使えなくならない、`terminate()` は待つ呼び出しを reject する。
- S196: **挙動が変わる点**（UI のレビューの指摘）。(1) Desktop: 同じ名前の別のファイルを開くと別のタブ（`フォルダ名/名前`）になり、保存はそのタブが開いた・保存したパスに書く（前は最後に開いたパスに書き、別のファイルを上書きし得た）。(2) ライブラリのファイルのタブは読み取り専用（打つと案内が出る。整形・F2 も断る）。(3) 補完の一覧が開いているときの Tab は候補の確定（閉じていれば字下げ。S188 のまま）。(4) 自動保存（localStorage）が書けない・上限で古いタブを落としたときは 1 文出す。打鍵ごとの保存は 300 ms まとめ、閉じる前・隠れたとき・保存では必ずすぐ書く。別のウィンドウが保存を書いたら知らせる。タブの切り替えで参照の一覧・リネームの入力欄・最後の値が前のタブから残らない。実行中は F12・Alt+← に入らない。補完が上限の 200 件で切れていれば打ち足すと問い直す（`Modelica.Units.SI.Vol` で `Voltage`）。Windows でドライブの根に FMU を書いたときの `outDir` を `C:\` に。未保存のまま Desktop のウィンドウを閉じる確認は持ち越し。
- S190: **挙動が変わる点**。実行の後にプロットが既定で描く信号を、結果の列の先頭の 3 本から「状態（`der()` の付く変数）を先に、足りなければ出力、最大 3 本」に変えた（`DoublePendulum` は `revolute1.phi`・`revolute1.w` が出る）。選ぶ規則は Rust の 1 か所で、wasm・LSP の列の応答の `layout.defaults` と Python の `simulate_columns` の結果の `defaults` に載る。
- S170: 進捗・失敗・結果が画面の中に出る（終わったとき 1 回だけスクロール）、2 モデルのファイルを select で選び直せる、モデル名がタブごとに残る、未保存の編集があるタブと同名のファイルを開き直しても編集は消えない。
- S171: エディタの補完を Rust 側につないだ（型の位置・`Modelica.Blocks.` の後ろ・`pin.` の後ろ。打ちかけでも出る。wasm で 1〜4 ms）。
- S173: コンパイラの限界（GM-U）で止まったとき「未対応（モデルの誤りではない）」の要約と色、強行（`GM-E8001` だけ効く）は効くときだけ「それでも実行する」、失敗の文面に最初の error を 1 行。
- S175: キーボードだけで開く・書く・検査・診断へ飛ぶ・実行・止める・保存まで通る（診断と参照の行の button、Ctrl/Cmd+Enter などのショートカット、プロットの `aria-label`、フォーカスの輪郭）。
- S179: 起動直後の案内（保存したタブが無い初回だけの 3 手順。閉じられる）、スキップリンク（エディタへ・結果へ）、タブの矢印キー・Home・End、未保存の印（●）と再読み込み前の確認。
- S180: ライブラリを使うときのモデル名の誤り（`-m PID_Controller` など）に、英語の原文でなく「モデルが見つからない」（`GM-E1002`）と候補を出す。`-L` も FILES も無いときの案内と、`-L` に MSL のサブフォルダを渡したときの「親のフォルダを渡す」案内。
- S182: CLI の使える流れ。`simulate` を `-o` なしで回すと「結果は書いていない」と 1 行（あれば「書いた: パス（n 行）」）。`-o`・`fmu export --out` の書けない出力先は計算の前に分かり、パスと理由を言う（`os error` を出さない）。**挙動が変わる点**: `globe … | head` は終了コード 0 で黙って終わる（今まで 2 や panic）、`check`・`simulate` の人間向けの出力は info の診断を件数だけにする（`-v` で全部。`--json` は変えない）。`--start-time -2` が通る。ヘルプに例（S0〜S10 に直し、内部のセッション番号を消した）。`--force` の診断の重複と、失敗の最後の行に最初の error。`graph` に `-L`。
- S183: Python に CLI と同じ読み取り（`Workspace.outline`・`deps`・`flat`）。C・Zig・Julia・Elixir・Gleam の README に、書いてあるとおりに動く最初の例（検査して実行して最後の値を読む）を置き、Julia の README を実物に合わせた。marimo の `sim_app.py` にライブラリとモデルの入力欄。
- S184: `--force` でコンパイルに失敗したとき同じ診断が 2 回返っていたのを facade の 1 か所で除いた（CLI・LSP・wasm・UI が同じ応答を見る。CLI だけで除いていたのをやめた）。応答に追加の `summary`（検査の最後の 1 行。`simulate` は実行しなかったときだけ）。文面は CLI のまま。
- S187: CLI の案内の取りこぼし。`-L` に MSL のサブフォルダを渡したときの「親のフォルダを渡す」案内が実際の CLI では出ていなかったのを直した。`-L` を忘れて FILES だけ渡したとき、パッケージ名（`-m Modelica.Blocks`）は `GM-E1002`「パッケージで、モデルではない」と直下のモデル。**挙動が変わる点**: 短い名前（3 文字以下）や半分以上が違う綴り（`Nope`・`T`）に候補を出さなくなった（的外れな 5 件の代わりに完全修飾名の案内）、開いている文書のモデルが候補の先頭、パッケージ名の診断が Rumoca の原文（`GM-E1001`）から `GM-E1002` に、`globe deps` が `-L` を受けて（ライブラリのファイルも閉包に）入力が省ける。README の既知の限界と `fmt --check` の例を直した。

- S186: UI の細部（S185 の監査）。クラス名のリネーム（F2）がいまのモデル名のクラスなら欄も追い（Ctrl+Z で戻る）、ライブラリの木・検索から入れ子のクラスを開くと宣言の行へカーソルと画面が動く（`librarySource` に追加の `range`）。補完の一覧は Ctrl+Enter で閉じ、「MSL を読む」の後はフォーカスが検索欄へ、「結果へ」は結果を画面へ入れる。
- S189: **挙動が変わる**: FMU の既定のファイル名に FMI の版が付く（`Decay.fmu` から `Decay-fmi2.fmu` / `Decay-fmi3.fmu`。CLI `globe fmu export`・facade・Python `export_fmu`・LSP `globe/exportFmu`・C ABI・NIF・各言語・UI のダウンロード名。FMU の中身は不変）。書き出せないモデルの失敗は「このモデルは FMU に書き出せない（理由: …）」から始まる。
- S189: `src/Modelica` の 71 ファイルを `globe fmt` で整形した（意味は不変。`globe fmt --check src/Modelica` を CI と `mise run check` に追加）。
- S192: 整形器を直した。UTF-8 の文字の途中や省略した tuple の出力での panic（PR #17）、CRLF の複数行コメント・文字列で整形が拒否されること（issue #19）を直し、**`if` を `end for;` で閉じるような種類の違う `end` は `Err` になる**（issue #21。MSL 4.1.0 ほか 2,661 本の整形結果は前後で同じ）。issue #20 は S167 で修正済み。
- S226: **全体監査の「あれば良い」15 件**のうち 12 件を直した。**挙動が変わる**: lint の `class-name-pascal-case` は function を指摘しない（MSL の指摘 1,873 件のうち 1,749 件が function だった。#62）、FastAPI 版のウェブアプリの 503・500 の本文は固定の短い文になり、`GLOBE_BIN` のパスと `globe` の stderr はサーバのログへ（#82）。直した: 初期化の前に失敗した FMU のインスタンスは drop で `Terminate` を呼ばない（#53）、Julia の `ccall` が `Workspace` を GC から守る（#65）、Zig の例が `\` を含むパスを JSON にエスケープ（#66）、`cargo xtask fetch-msl`・`fetch-reference-fmus` が版を上げると前の展開を消す（#81）、イベントの 2 行の文書（#47）、`structure::split` の死んだ分岐（#61）、`test_ui.py` の `LONG`（#71）、`finish-session` の意味的な衝突の手順と `ci.yml` の手動実行の比較元（#80）、db のクエリの実行の記録は `GlobeDb::record_executed` で始めたときだけ積む（本番の LSP で増え続けない。#87）、facade の段階を求める式を `FullReport::failed_stage()` に（#88）。直さない: #45・#46（上流に口が要る）、#63（見た目だけ）。
- S223: **全体監査の「直した方がよい」18 件**を直した。**挙動が変わる**: `globe fmt -`（`--json` でない）の構文の誤りは終了コード 2（以前は 1。`--check` の「整形が要る」と取り違えない。#60）、`globe lint --disable` は規則のコード（`GM-L0003`）も受け、知らない規則は終了コード 2（#56）、出力点数が Rumoca の上限（`floor(区間/刻み)+1 >= 1e7`、刻みが時刻の精度より細かい）に当たる設定は実行前に `GM-E10004`（#44）、ライブラリの UTF-8 でないファイルは悪いバイトを U+FFFD にして読む（警告は出ない。#85）、`globe fmu run` は FMI 3.0 の配列変数を扱わず、既定の出力から外す（#50）、Desktop は外で書き換わったファイルへの保存を確かめる（#70）、wasm の Worker は線形メモリが 2 GiB を超えたら次の要求の前に作り直す（#69）。直した: ライブラリの BOM 付きファイル（#84）と木の BOM・panic（#59）、`fmu run` の CSV の見出し（#52）、P5b の Integer・Boolean の start（#51）、lint の `missing-description` の偽陽性（#57）と `line-too-long` の位置（#58）、connect グラフが自分を部品に持つモデルで膨らむ（#86）、Astro の `spawnSync`（#78）、使われていない依存 4 つ（#79）、release.yml・android.yml の第三者 action を SHA で固定（#76）、CLI を呼ぶウェブアプリ 5 種の試験を CI に（#77）。
- S218: **全体監査の「必ず直す」13 件**を直した。**挙動が変わる**: `CancelToken` を渡した `bdf`・`auto`・`dopri45`（Python・C・NIF・LSP のネイティブ）は、止めるとすぐ返る（`GM-I11004`。途中経過は返らず、裏の計算は終わるか予算まで続く。#43）、CLI を呼ぶウェブアプリ 6 種の lint から `connect-type-mismatch` が消える（#73）、`settings` の綴りの誤り（`stoptime` など）は黙って捨てずに断る（C・NIF・Julia・Zig・wasm。#83）、`globe fmu run --out x.mo` は .mo を上書きせず exit 2（#55）。直した: 書き出した FMU に `<DefaultExperiment>`（#48）、LSP が依存先の診断を出し直す（#64）、深い HTML の入れ子で落ちない（#54）、ワーカーが kill・クラッシュで終わっても FMU の展開先が /tmp に残らない（#49）、Desktop の最近使ったファイルを Ctrl+S で保存できる（#67）、Android のプロットの保存（#68）、セッション記録の伏せ字に `gho_` など（#75）、CI は push の run を止めない（#72）、Android の署名鍵は 1 step だけに見える（#74）。

- S219: **結果の読み込み**。CLI に `globe results <in.mat|in.csv> [-o out.csv|out.mat]`（形は拡張子で決める。`-o` 無しは CSV を標準出力へ。入力と同じ書き先は断る）、Python に `globe_mode.read_results(path)`（`simulate_columns` と同じ形の dict。`table` で表になる。MAT のパラメータは `parameters`）。omc の実物の MAT で試した。UI の「結果を開く」は別の仕事。
- S220: LSP に `textDocument/foldingRange`（クラス・節・複数行の annotation・連続コメント）、VS Code の拡張に `indentationRules`（`model`/`function`/`if … then`/`for … loop` などの次の行で字下げし、`end`・`else`・節の見出しで戻す）。UI（CodeMirror）の折りたたみは別 WP。
- S228: **UI の使い心地 4 点**。実行中に **Esc で止める**（Ctrl+. と同じ。入力欄・ダイアログの中では奪わず、補完の一覧が開いていれば閉じるだけ）。**タブの並べ替え**（ドラッグと Ctrl+Shift+PageUp / PageDown。順序は自動保存に乗る）。**実行の設定をモデル名ごとに保存**（localStorage `globe-mode.sim`。**挙動が変わる**: モデル名を変えるとそのモデルの設定に入れ替わり、保存が無ければ既定に戻る。「設定を既定に戻す」で消える）。実行の設定に**開始時刻・刻み幅（固定刻みのソルバのときだけ）・予算（秒）**の欄を足し、GM-E10004 を欄の下にも出す。
- S225: UI（CodeMirror）に折りたたみ（行番号の隣の印と Ctrl+Shift+[ / ]。LSP と同じ範囲）。範囲は Rust の同じ関数で、ブラウザ・Android は wasm に `folding_ranges(file, text)`、Desktop は sidecar の `textDocument/foldingRange`（UI が中継する method は 20 本）。入力が 300 ms 止まったら取り直す。
- S215: **挙動が変わる**: CLI を子プロセスで呼ぶウェブアプリ 6 種は、本文が 1 MiB を超えると 413（Streamlit・marimo は画面にエラー）、`globe` が 10 秒（`GLOBE_TIMEOUT_MS`）で終わらなければ kill して 5xx（FastAPI は 60 秒から 10 秒に）。Astro の `indent` は 0〜16、Phoenix・Gleam は `indent: 0` を受け、`disable` は規則名の形だけ。Rust のウェブアプリ 4 種は `Options` のフィールド不足（S061 から）でビルドできなかったのを直し、CI にジョブ `web` を足した（#22・#36）。

- S214: LSP と言語の束ねの issue を直した。**挙動が変わる**: (1) `globe/format`（VS Code の整形）の indent は、`globe-mode.toml` があればその値が勝つ（`textDocument/formatting` と同じ規則。無いときだけ params の indent）。(2) LSP は積まれた `didChange` をまとめ、次の変更が待っている古い版は検査しない（最後の診断は最新の版）。(3) `globe/simulate` に `max_wall_seconds` が無ければ 180 s の予算を置く。(4) Elixir の NIF は panic の後も同じ workspace を使え、Mutex を待つ NIF は DirtyIo で回る。(5) Julia の `simulate_observed` はコールバックの例外を止めてから投げ直す。
- S211: 整形器と `globe fmt` の issue 5 件を直した。**挙動が変わる**: ①`Rectangle`・`Line`・`Text` などの名前でも annotation の外（`extends Pkg.Line(R = 1)`、関数・record の呼び出し）は並べ替えず `origin` も足さない（#24。以前は利用者のモデルを壊した）、②`globe fmt --check -`（標準入力）は整形が要れば終了コード 1（以前は必ず 0。#26）、③`globe fmt` は同じフォルダの一時ファイルに書いて rename で差し替える（途中で落ちても元のファイルは切り詰められない。権限は保ち、symlink は指す先を書く。#27）、④括弧の入れ子が 256 段を越える入力は整形・lint が誤りを返す（以前は 100,000 段でスタックが溢れてプロセスごと落ちた。#28）、⑤UTF-8 の BOM 付きのファイルを検査・整形・lint でき、整形しても BOM は残る（以前は S0 の構文エラー。#32）。
- S209: **挙動が変わる**: `globe fmt --json` / `globe lint --json` もパスか `-` が要る。パスも `-` も無いと使い方を出して終了コード 2（S199 が例外にしていた `--json` のパスなしの標準入力を無くし、`-` の明示に揃えた）。標準入力を `--json` で呼ぶ所は `globe fmt --json -` / `globe lint --json -` にする。同梱のウェブアプリ（FastAPI・Streamlit・marimo・Phoenix・Wisp・Astro）は直した。Rust のウェブアプリ 4 種は crate を直接使うので影響なし。
- S222: **UI の「結果を開く」**。「開く」（Ctrl/Cmd+O）が `.mat`（MAT v4）と `.csv` も受け付け、「結果 <名前>」のタブに実行の結果と同じプロット（系列の選択・CSV/PNG の書き出し）で出す。ブラウザ・Android は wasm の `read_results`、Desktop は sidecar の `globe/readResults`。結果のタブはエディタを隠し、実行と保存は断る（再読み込みでは戻らない）。
- S199: CLI のレビュー指摘（`-o` が入力・ライブラリ・`.mo` を上書きしない、壊れた symlink を消さない、`-o -` の診断に位置と案内が付く、候補の整理）と終了コードの体系。**挙動が変わる**: (1) 未対応（GM-U）だけで止まった `check`・`simulate` は終了コード 3（モデルの誤りは 1 のまま。`--force` で実行できたときと、Error が混ざるときは 1）。(2) `| head` で読み手が先に閉じても、結果の終了コードを保つ（これまでは何でも 0。`pipefail` の CI で `globe fmt --check src | head` の失敗が見えた）。書き込みの失敗（ディスク満杯）は panic ではなく 1 行出して 2。(3) `fmt`・`lint` の標準入力は `-` を明示したときだけ（パスも `-` も無ければ使い方を出して終了コード 2。以前は入力がパイプなら標準入力を読み、`fmt --check` が何も検査せず 0 で終わった）。パスなしの `--json` は標準入力のまま（ウェブアプリの契約）。ターミナルでの `globe fmt`（パス省略でカレントディレクトリ）も `globe fmt .` と書く。(4) `-o` が入力のファイル・`-L` の配下・拡張子 `.mo` を指すと、計算の前に終了コード 2。`fmu export --out` は作り直す `<out>/<名前>/` に `.mo` か入力があると 2。(5) ワークスペースの候補（「候補: …」）は最大 5 件で、補足が既にあるパッケージ名の診断には付かない。ヘルプの末尾と README に終了コードの表、README に MSL の zip での入手の 3 行。
- S188: **挙動が変わる**: ブラウザ・Desktop の UI のエディタで Tab は字下げ（空白 2 つ、選んだ行すべて）、Shift+Tab は戻す。フォーカスをエディタの外へ出すには Esc のあと Tab（Shift+Tab）か Ctrl+M（Mac は Shift+Alt+M）。説明はエディタの下とスクリーンリーダー向けの `aria-describedby`。
- S198: LSP の参照・リネーム・補完のレビュー（9 件）の修正。**挙動が変わる点**: 部品のリネームが、ほかの部品の修飾子の名前・名前つきの引数の名前（`Sine s(amplitude = amplitude)` の左）と、同名の部品を宣言した入れ子のクラスの中を書き換えない（前は利用者の本文を壊した）。継承した部品と同じ名前への変更は断る。引用符つきの識別子（`'a b'`）は丸ごと一致するときだけ。LSP の `textDocument/completion`・`globe/completion` は配列でなく `CompletionList` `{isIncomplete, items}`（200 件に達したら `isIncomplete: true`。wasm の `completion` は配列のまま）。`textDocument/rename`・`prepareRename` は、ファイル名に対応する最上位のクラス（`M.mo` の `model M`、`package.mo`）と、根の下の開いていない `.mo` に同じ名前があるクラスのリネームを断る。開いていない文書への `globe/rename`・`textDocument/rename` は -32803、`globe/references` は空の配列。ライブラリのファイルがディスクで変わったら読み直し、ホバーが古い範囲で panic しない。LSP は 1 要求の panic で落ちずに -32603 を返す。
- S134・S135: Desktop が `C.UTF-8` の locale で白画面になる不具合の修正と、自動更新（`tauri-plugin-updater`。鍵を登録するまで「未設定」）。
- S140・S164: アプリの使える流れの点検（調査だけ。[2026-11-04](doc/research/app-flow-2026-11-04.md)、[2026-11-05](doc/research/app-flow-2026-11-05.md)）。
- S212: **ponytail-audit の issue 6 件**（#23・#29・#31・#34・#37・#38）を再現してから直した。FMU の Export は、`<out>/<名前>/` に `modelDescription.xml` が無ければ消さずに断る（Desktop・Python・C・NIF も。直す前は何でも `remove_dir_all` した。**空のフォルダも断る**）。出力の点数（区間 / Interval、固定刻みなら区間 / 刻み幅）が 1000 万を超える設定は `GM-E10004`（UI・Python・CLI・wasm とも）。Integer の上書きの小数部、Boolean の 0/1 以外は丸めずに `GM-E10006`。ライブラリのディレクトリの読み込み（LSP・CLI・Python・C の `Workspace::with_libraries`、`GlobeDb::load_dir`）が、ドットのフォルダを飛ばし、`package.mo` の無い `X/` の隣に `X.mo` があれば `X.mo` だけを読む（AixLib の `RadiatorType` の二重定義が消える）。FMI Import の RK4 は有限でない状態を `Error` にする（Dormand–Prince は S206 で直っていた）。#37（`Checker::reset`）は測ったが RSS が下がらないので足していない。
- S207: `globe-mode-fmt` の `lint` が本文の長さの 2 乗で遅くなっていたのを直した（クラス宣言・診断ごとに行・桁を先頭から数え直し、長い行ごとに全行を探していた）。4.4 MB で 88.7 s が 0.57 s、MSL の `Fluid/Dissipation.mo`（630 KB）は wasm で 508 ms が 43 ms。診断の内容・順序・位置と `format` の出力は MSL ほか 2,661 本で変わらない（挙動の変化なし）。

### P5 FMI

- S057: FMI の調査と `globe-mode-fmi`（`modelDescription.xml`）の準備。
- S094・S098・S105: `globe fmu export` が FMI 2.0・3.0 のソースの FMU（ME・CS）を出す（Rumoca の `fmi2` / `fmi3` のテンプレート。FMPy と fmusim で検証）。`--build --target <triple>` で Linux・Windows・macOS の共有ライブラリも同梱。
- S210: **FMI Import のワーカー**。`globe fmu run` は FMU の読み込みと実行を**別プロセス**（自分自身を隠しの副命令 `fmu __worker` で起動）に隔離する。FMU が `abort`・segfault・無限ループ・メモリの食いつぶしで落ちても `globe` は落ちず、終了の状態（signal など）と標準エラーの末尾を添えた誤り（終了コード 1）になる。`--timeout 秒`（超えたら kill）、`--memory-limit MiB`（Linux だけ）、`--progress`（時刻を標準エラーへ）を足した。**挙動が変わる点**: 実行前の警告の文面、FMU の失敗の前置きは変わらないが、実行は子プロセスで行う（起動に数十 ms）。ディスクとネットワークは隔離しない（sandbox は持ち越し）。API は `globe-mode-fmi-import` の `run_in_worker` / `worker_main`（将来の facade・UI・Python が同じ形を使える）。依存は増えない（[ADR-0026](doc/adr/0026-fmi-import-placement.md) の (n)）。
- S229: **FMI Import を facade・Python・Desktop の UI から使えるように**。facade `globe-mode`（feature `fmu-import`）に `import_fmu_info`（`globe fmu info` と同じ情報）と `run_fmu`（ワーカーの実行ファイルは呼び手が渡す。結果の列は `Simulated::columns` と同じ形、進捗、中断で 50 ms 以内にワーカーを kill）。CLI の `fmu info` / `run` も facade を呼ぶ（出力は同じ）。Python: `fmu_info(path)`・`run_fmu(path, kind=, stop_time=, outputs=, parameters=, timeout=, on_progress=)` と例外 `FmuError`（ワーカーは `GLOBE_BIN` か PATH の `globe`。無ければ `FmuError`）。LSP: `globe/fmuInfo`・`globe/fmuRun`（`$/progress`・`$/cancelRequest`。Desktop の中継は 20 から 22 method）。Desktop の UI: 「開く」で `.fmu` を選ぶと警告つきの確認のあと結果のタブにプロットを出す（「止める」で kill）。ブラウザ・Android は「Desktop だけ」と理由を出す。
- S206: **FMI Import のレビューの指摘 4 件**と案 B の決定。ME の積分で、有限でない誤差と FMU の `Discard` を刻みを縮めて再試行する（直す前は、試しの点が定義域の外に出ると NaN を `Ok` で返した）。出力の格子点が 1000 万を超える `--step`（既定の `stepSize` を含む）は、実行の前に「`--step` を大きくする」と断る（ME・CS。**挙動が変わる**）。`DefaultExperiment` の `stepSize` を XML として読む。3.0 の真偽の出力引数を `u8` で受ける。ユーザーの決定で、案 B（`fmi` crate）は入れず自前で作る。
- S205: **FMI Import の Model Exchange**（FMI 2.0・3.0）。`globe-mode-fmi-import` に `FmuMe`・`run_me`（FMU の微分を crate の中の RK4 / Dormand–Prince 5(4) で積分し、state・time・step event を扱う。依存は増えていない）。CLI は `globe fmu run --kind me|cs`（**省略時は Co-Simulation を持てば CS で、S204 の挙動は変わらない**。CS を持たない FMU は ME で回す）、`globe fmu info` に連続状態と event indicator の数を追加。BouncingBall の衝突 11 回の時刻が解析解と 3.2e-11 で一致。硬い系（陰的な積分）は未。
- S204: **FMI Import の最初の一切れ**。新 crate `globe-mode-fmi-import` が FMU（zip）を安全に展開し（zip slip・シンボリックリンク・2 GiB 超を拒む）、共有ライブラリを読み込んで FMI 2.0・3.0 の Co-Simulation を回す（自前の C ABI。Reference-FMUs の BouncingBall・Dahlquist・VanDerPol の参照の CSV と全桁で一致、Export した Decay の往復も通る）。CLI に `globe fmu info <file.fmu>`（`modelDescription.xml` の要約。ネイティブコードは実行しない）と `globe fmu run <file.fmu> [--stop-time …] [--step …] [--set 名前=値]`（CSV を出す。**実行の前に「FMU の中のネイティブコードを実行する」と標準エラーに 1 行出す**）。ADR-0026 を Accepted に。`fmi` crate 0.8 は zip 8.6.0 の脆弱性（4 件）などで入れていない。ME・マスター・SE は未。
- S095・S099・S109・S112・S114・S137: 自前のランタイム `globe-mode-fmu`（依存 0）。FMI 2.0 ME の関数、モデルの compile、事象、代数・離散の変数、入力、FMI 3.0 ME。S117・S124 で `modelDescription.xml` の書き手を P5a と共有。
- S144・S154: UI の「FMU を書き出す」（ブラウザ・Desktop・Android。FMI 2.0 / 3.0 を選べる）。
- S224: Export した FMU の型。**FMI 2.0 の P5a は Enumeration の変数を `<Enumeration declaredType>` と `<TypeDefinitions>` で書く**（これまでは `<Integer>` で、型の名前とリテラルが落ちていた。Enumeration を持たないモデルの XML は変わらない）。P5b（`globe-mode-fmu`）は 2.0・3.0 とも Enumeration のパラメータ・入力・出力を型の定義つきで書き、Integer（3.0 は Int64）で読み書きし、String のパラメータを `fmi2GetString`/`fmi3GetString` で読む（`fixed`。書くと `fmi2Error`）。**P5a の 3.0 の Enumeration は Int32 のまま**（Rumoca の C が `fmi3GetInt64` 未対応）。`dependencies` は出力ごとの依存の情報が無く持ち越し。
- S231: **P5b の Co-Simulation**。自前のランタイム `globe-mode-fmu` が `fmi2DoStep`・`fmi3DoStep` を出す（固定刻みの RK4 と ME と同じ事象処理。FMPy の CS で Decay の x(1) が e⁻¹ と 4e-15、BouncingBall・InputDecay も通る）。`model_description_kind` で `<CoSimulation canHandleVariableCommunicationStepSize>` を書く（`model_description`・`model_description_3` は ME だけのまま）。入力・出力の微分、`fmi2CancelStep`、`fmi2Get*Status` は `Error`。CLI の `globe fmu export` は P5a だけなので変えていない。facade `globe-mode` の `serde` を optional でなくし、`cargo test -p globe-mode` が単体で通る（#89）。

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
- S195: `release.yml` のタグの経路を、タグを打たずに確かめられるようにした（手動実行の入力 `tag`。手動は Release を作らない）。版はファイルが決め、タグの版と違えば最初のジョブで止める（書き換えをやめた。pre-release のタグ `v0.2.0-rc1` の接尾辞は版に入らない）。鍵は bundle の step だけに渡し、softprops/action-gh-release をコミットで固定。手順は `doc/release.md`。

### P7 言語 API

- S039: Python の wheel `globe_mode`（`globe-mode-py`、PyO3・abi3-py312・maturin）。load_dir → check → graph → simulate → polars の表の参照シナリオを `pytest` で通した。
- S075・S082・S087: C ABI の `globe-mode-c`（9 関数、手書きのヘッダ）と Zig・Julia、Elixir の NIF（`globe-mode-nif`）、Gleam の `@external`。参照シナリオ（Decay を開く → check → simulate → x(1) → format）は `mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test`。
- S110・S111・S116・S118: 公開 API の機能表を作り、6 言語に format / lint、列の道（`simulate_columns`）、FMU の書き出しを足した。
- S181: Python API を厳しくした。**未知の設定のキー（`stoptime` など）は黙って無視されず `TypeError` になる**（前は `ok: True` で既定の値のまま走った。使えるキーを添える）。設定の型の誤りはどの設定かを言う `ValueError`、`Workspace("data/msl")` は 1 つのライブラリ（前は 1 文字ずつ）、ライブラリが読めない `RuntimeError` の本文は `コード: 文`（元の診断は `.diagnostics`）。`export_fmu` の未知の options は `TypeError`。docstring・`py.typed`・`_native.pyi`、`on_progress` は既定のソルバでは積分の中に届かないと明記。
- S208: 言語の面（Python・wasm・LSP・C・NIF）の同じ処理を facade の関数に寄せた（`simulate_request`・`ExecStage::as_str`・`Workspace::outline_json`・`Simulated::columns`・`FmuOptions::from_json`・`Workspace::open`・`to_json`）。応答の JSON・例外・進捗の形は変えていない（挙動の変更なし）。issue #16 の残り（`memory.mjs` の写し、xtask の `msl-sim` の `solver`・`known_deviations` の `None` の分岐）も片付けた。
- S203: 残りの open の issue 8 件を点検した。**挙動が変わる**: 強行（`force`）で走っても検査の Error が残る結果は、応答の `ok` が `false`（以前は `true`。`globe simulate --force` の終了コード 1 と同じ向き）。wasm の `last_panic()` は次の `Workspace` の作成・実行で消える。結果を CSV にできないときは `ok: false` と `refused` を返す。`.cargo/config.toml`・`.gitattributes` の変更で CI が全体を検査する。`wasm:compare` の既知の差に上限を持たせ、有限でない値（Inf・NaN）を区別した。
- S193: 実行系の open の issue 8 件を点検した。**知らないソルバ名（`--solver rk5`）で、置き換えた bdf についての刻み・許容誤差の警告が出ていたのを直し**（GM-E10005 だけ）、**壊れた MAT のヘッダで debug ビルドが panic するのを `InvalidData` に直した**。#1・#3・#5 は既に直っていた（S024・S021・S013）。#4・#6 は直さず残した（解説書と roadmap の表）。
- S213: Export した FMU のランタイム（issue #40）。**`fmi2SetReal`/`fmi3SetFloat64` で与えた状態の値（FMPy の `start_values={'x': 5, 'k': 2}`）が、パラメータや開始時刻の変更による作り直しで既定に戻っていたのを直した**。`globe fmu export --build` は `CC="ccache cc"` のように空白で区切った引数つきの `CC` を受ける。Windows の `file:///C:/…` の resources を読める。入口の panic は、落ちる入力が見つからず直していない。
- S200: Python の例外の階層とレビューの指摘（Opus、2026-11-06）の修正。**`GlobeModeError`（基底）を足し、`LibraryError`（`RuntimeError`）・`SettingsError`・`ModelNotFoundError`・`SourceError`・`ExportError`・`ResultError`（いずれも `ValueError`）を子にした**（`.diagnostics`・`.code` つき。既存の `except ValueError` / `RuntimeError` は受けられる。呼び方の誤りは素の `TypeError`、モデルの誤りは今までどおり結果の dict）。**挙動が変わる: `simulate` / `simulate_columns` の `on_progress` はキーワード専用**（前は位置引数の 2 番目で、設定の dict を渡すと検査の後に `'dict' object is not callable`。呼べないものは入口で `TypeError`）。`on_progress` が無くても実行の間は GIL を離す（ほかの Python のスレッドが動き、Ctrl-C が効く）。`check`・`graph`・`deps`・`flat`・`export_fmu` も GIL を離す。numpy の数・`Fraction` を設定に使える、NaN・inf・bytes のライブラリ・`export_fmu(targets="…")` の文字列を正しく扱う、`parameters=None` は省略、`table(ws.check(m))` は `KeyError` でなく `ResultError`。列の道の写しを 1 回減らし、ピークのメモリを下げた。
- S191: Python だけにあった読み取り（`outline`・`deps`・`flat`）を C ABI（`gm_outline`・`gm_deps`・`gm_flat`）・Zig・Julia・Elixir・Gleam・wasm（`Workspace`・`GlobeWorker`）にも揃えた。列の応答に `defaults`（既定で描く信号。S190）を C ABI・NIF にも足し、`summary`（S184）が各言語で出ることを試験で確かめた。
- S202: C ABI と NIF のレビューの指摘を直した。**挙動が変わる**: C ABI は進捗のコールバックの中から同じ `ws` を使う `gm_*` を呼ぶと `{"error": "ワークスペースは使用中…"}` を返し（`gm_set_file`・`gm_load_dir` は -1）、`gm_workspace_free` は何もしない（前は未定義動作・use-after-free）。NIF の `set_file`・`format`・`lint` は dirty スケジューラで回り、別のプロセスの長い実行の間も BEAM の通常のスケジューラを止めない（`set_file` は DirtyIo）。`gm_workspace_free` の Drop の panic を C へ出さない。応答の JSON は変わらない。
- S197: Desktop のシェルのレビューの指摘を直した。**挙動が変わる**: 応えない検査（`globe/check` など）でも「止める」が効き、2 秒で sidecar を作り直す。UI から sidecar へ中継する method は UI が使うものだけ（それ以外は誤り）。Tauri の CSP を入れた（ブラウザ版と同じ＋IPC）。MSL の切り替えの失敗でライブラリを元に戻し、`fetch_msl` は同時に 1 つだけ（HEAD は 10 秒、展開中も取り消せる）。
- S121・S123・S129・S131・S133・S136: 実行の進捗と中断をコア・Python・C と他の言語・wasm とブラウザの UI・`globe lsp` と Desktop に入れた（[ADR-0033](doc/adr/0033-progress-and-cancel.md)）。
- S125・S153・S155・S156・S160: UI の負荷の比較。測定ハーネス（[ADR-0034](doc/adr/0034-ui-bench-method.md)）と、Leptos 版・Dioxus 版の試験版（`src/Rust/apps/experimental/`、配布物には入らない）、Linux・Windows・macOS の結果 `doc/bench/ui.md`。

### 既知の限界

- Rumoca は git 依存の固定 rev（v0.10.2）で、crates.io には出せない。MSL 4.1.0 の例題 566 本のうち検査を通るのは 369 本、コンパイラの限界（`GM-U`）が 184 本（[doc/msl-funnel.md](doc/msl-funnel.md)）。
- 配布するバイナリに `sundials` のソルバ（`cvode`・`ida`）は入れない（cmake と C コンパイラが要る）。
- FMI の読み込み、SSP、Diagram / Icon / Documentation ビューは未実装。
- Desktop と Android は署名していない。Desktop の自動更新は鍵を登録するまで「未設定」。実機での長い操作は【未確認】。
- GitHub の Release・タグ・Pages・PyPI・Marketplace への公開はまだ無い（[doc/research/app-flow-2026-11-05.md](doc/research/app-flow-2026-11-05.md)）。

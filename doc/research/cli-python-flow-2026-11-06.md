# CLI・Python・他言語の使える流れの点検（S178）

- 確認日: 2026-11-06。main の `b14345f`（S177 のマージ。版 0.2.0）。コードは変えていない。
- 目的: ブラウザ版は [S140](app-flow-2026-11-04.md)・[S164](app-flow-2026-11-05.md) の 2 回の点検で穴を埋めた。**同じ方法で UI 以外の面を利用者の目で通しで使い**、穴を順位付きで出す。0.2.0 として配る前の点検（公開はしていない。ユーザーの判断）。
- 対象: (1) CLI `globe`、(2) Python `globe_mode`（と marimo のノートブック）、(3) C・Zig・Julia・Elixir・Gleam（README の例を 1 回ずつ回しただけ）。
- 確認の記号は S164 と同じ。【操作】実際に打った・回した。【読】コード・文書を読んだ。【未確認】どちらもできていない。推測は「推測」と書く。

## 1. 方法と条件

| 項目 | 内容 |
| --- | --- |
| CLI のビルド | main の `b14345f`（worktree `s178-cli-python-flow-audit`、main と同じ内容）で `cargo build --release -p globe-mode-cli`（共有 target、`CARGO_BUILD_JOBS=2`、58 s）。共有 target にあった `globe` は `0.0.1`（2026-11-05 19:01）で古かったので作り直し、`globe --version` が `globe 0.2.0`、日時 2026-11-05 23:49 を確かめた |
| Python | scratch に `uv venv`、チェックアウトから `uv pip install ".[widget]"`（maturin。共有 target。worktree に `target/` は出なかった）。`globe_mode 0.2.0`、`_native.abi3.so` は 64 MB。main の `.venv` は触っていない |
| 入力 | 自作の `RC.mo`（1 状態）・`日本語 dir with space.mo`（日本語・空白のパス）・わざと壊した 3 本（構文の誤り、方程式が足りない、バイナリ）、`GlobeModeValidation.Simulation.Decay`、MSL 4.1.0（`-L data/msl` に丸ごと）の例題 5 本（ChuaCircuit・PID_Controller・Rotational.First・TwoMasses・CauerLowPassAnalog）と未対応の 1 本（Rotational.Examples.Friction） |
| 負荷 | WSL2、RAM 3 GB。測った間の load average は 1〜2.5（最初の MSL の 1 回だけ 6.6）。待ち時間は 1 回の測定で、静かなときの値 |
| 他言語 | `mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test` を 1 本ずつ（共有 target に release の `libglobe_mode_c.so`・`libglobe_mode_nif` があったので、**冷えた状態のビルド時間ではない**） |
| 道具 | 一時スクリプトは scratchpad に置き、リポジトリには入れていない。`marimo export html` が worktree に `src/Python/marimo/__marimo__/` を作ったので消した（5 章 P11） |

## 2. 通った流れ

| 面 | 流れ | 結果 | 確認 |
| --- | --- | --- | --- |
| CLI | 自作 RC を `check` → `simulate -o rc.csv`（501 行）→ `--param R=500` → `-o rc.mat`（MAT v4）→ `--json` → `fmu export`（2.0・3.0）→ `fmt` → `lint` → `outline` → `flat` → `deps` → `lsp` の initialize から exit | すべて動いた。RC は検査・実行とも 0.02 s、FMU 0.08〜0.12 s。`--format mat -o x.csv` は MAT v4 を書く（ヘルプどおり、形式が拡張子に優先） | 【操作】 |
| CLI | MSL 5 本の `check` と `simulate -o /dev/null` | 全部 exit 0。ChuaCircuit 検査 1.19 s・実行 1.61 s、PID_Controller 1.43 s・1.87 s、Rotational.First 1.35 s・1.57 s、TwoMasses 1.58 s・1.42 s、CauerLowPassAnalog 1.35 s・1.29 s。ブラウザ版（S164）と同じ桁 | 【操作】 |
| CLI | `fmu export --build --fmi-headers <ヘッダのあるディレクトリ>`、FMI 3.0、MSL の ChuaCircuit | `binaries/linux64/*.so` が入った zip を作れた（1.09 s）。ただしヘッダは `/usr/include/omc/cpp/FMU2`（OpenModelica のパッケージ）を借りた。FMI 3.0・MSL も 0.1〜1.2 s | 【操作】 |
| CLI | `lsp` | initialize 6 ms、機能は textDocumentSync・documentSymbol・hover・formatting・completion・definition・references・rename。構文の誤りの診断が来て、shutdown・exit が終了コード 0 | 【操作】 |
| Python | README の 5 行（`Workspace(libraries=[msl])` → `check` → `table(simulate_columns)`） | **書いてあるとおりに動く**。`True`、`(50219, 45) ['time', 'L.i', 'L.v']`。Workspace 0.68 s、check 1.06 s、simulate_columns 0.22 s | 【操作】 |
| Python | `simulate`（CSV）・`table`・`format`・`lint`・`graph`・`export_fmu`（2.0・3.0）・`solvers`・`on_progress`・`KeyboardInterrupt`・コールバックの例外 | 動く。コールバックの例外は `RuntimeError` で伝わり、`KeyboardInterrupt` も出る | 【操作】 |
| marimo | `sim_app.py`（`python` で全セル）、`marimo export html` を `sim_app.py`・`fmt_app.py`、`marimo check` | エラー無し。HTML 56 KB・51 KB。anywidget の canvas は書き出せるが、**ブラウザで開いての見え方は見ていない**【未確認】 | 【操作】 |
| C・Zig | `mise run c:zig` | exit 0、12 s。`check` → `simulate`（json=14521 bytes）→ 列 → 中断（`termination=cancelled`）→ 整形 → FMU | 【操作】 |
| Julia | `mise run c:julia` | exit 0、4 s。同じ参照シナリオ。列 json=29.84 ms・columns=2.33 ms | 【操作】 |
| Elixir | `mise run elixir:test` | 2 tests, 0 failures、8 s | 【操作】 |
| Gleam | `mise run gleam:test` | 2 passed、3 s | 【操作】 |

S110 の対応表（[lang-api-matrix](lang-api-matrix-2026-11-03.md)）と食い違うのは、Julia の README だけ（N16）。

## 3. 誤った使い方（CLI）

stdout / stderr の別と終了コードは実測。文面は要点だけ（原文の長い行は切った）。

| # | 打ったもの | 終了コード | 出た所 | 文面 | 直し方が分かるか |
| --- | --- | --- | --- | --- | --- |
| E1 | 無いファイル `check nofile.mo -m RC` | 2 | stderr | `error: nofile.mo を読めない: No such file or directory (os error 2)` | 分かる。`os error 2` は余計 |
| E2 | 無いモデル名（ファイル入力）`check rc.mo -m Nope` | 1 | stdout | `error[GM-E1002] S1 名前解決: モデル \`Nope\` が見つからない` と `note: 候補: RC` | 分かる（候補つき） |
| E3 | `-m` を忘れる | 2 | stderr | clap の英語。`the following required arguments were not provided: --model <MODEL>` と `Usage:` | 英語だが分かる |
| E4 | **`-L` を忘れる**（MSL のモデル名だけ）`check -m Modelica.Blocks.Examples.PID_Controller` | 2 | stderr | `the following required arguments were not provided: <FILES>...` | **分からない**。FILES を求められ、`-L` のことは出ない |
| E5 | **MSL のサブフォルダだけを `-L` に**（`-L "data/msl/Modelica 4.1.0"`）| 1 | stdout | `…/Constants.mo: error[GM-E1001] S1 名前解決: unresolved component reference: 'ModelicaServices.Machine.eps'`、`note: rumoca: ER002`、`ほかに 3 件の失敗がある`（1.16 s かけて） | **分からない**。README にだけ「サブフォルダではなく `data/msl` ごと」とある |
| E6 | 存在しない `-L /nonexistent/msl` | 2 | stderr | `error: ライブラリ /nonexistent/msl がディレクトリではない` | 分かる（無い場合も「ディレクトリではない」と言う） |
| E7 | 構文の誤り（`Real x` の `;` 忘れ）`check bad.mo -m Bad` | 1 | stdout | `bad.mo:2:8: error[GM-E0001] S0 構文: unexpected \`x\``、`Bad: NG（S0 構文 で停止）` | 位置は出るが、**直し方は分からない**。同じファイルを `fmt` に渡すと `文の途中に equation（; が抜けている）` と出て、こちらの方が良い |
| E8 | 方程式が足りない `check unbal.mo -m Unbal` | 1 | stdout | `error[GM-E5002] S5 …: unbalanced model: 1 equations, 2 unknowns (balance = -1); unknowns[state=1 algebraic=1 output=0 discrete_real=0 …]`、`note: rumoca: ED001` | 数は分かる。**どの変数が決まらないかは出ない**。Rumoca の英語の原文 |
| E9 | 未対応の例題 `check -L data/msl -m …Rotational.Examples.Friction` | 1 | stdout | `unsupported[GM-U0003] S5 …`、`note: 既知の制約（ED001）… モデルの誤りではなく Rumoca の制約とみなす`、最後の行は `NG（S5 方程式数と変数数 で停止）` | 注に書いてあるが、**終了コードも最後の行もモデルの誤りと同じ**（UI は S173 で「未対応」と分けた） |
| E10 | **モデル名が短い・パッケージ名・`.mo` つき**（ライブラリ使用）`-m PID_Controller`、`-m Modelica.Blocks`、`-m …PID_Controller.mo` | 1 | stdout | `error[GM-E1001] S1 名前解決: PID_Controller could not be compiled because resolve/parse failed` | **分からない**。英語の原文、候補なし（ファイル入力の E2 は候補を出す）。1.3 s かかる |
| E11 | 数でない引数 `--stop-time abc` | 2 | stderr | clap の英語。`invalid value 'abc' for '--stop-time <STOP_TIME>': invalid float literal` | 分かる（英語） |
| E12 | **負の数 `--stop-time -1`、`--start-time -2`** | 2 | stderr | `error: unexpected argument '-1' found`、`tip: to pass '-1' as a value, use '-- -1'` | **誤解させる**。`--` を足しても通らず、`--stop-time=-1` なら通る。開始時刻が負は実用の値 |
| E13 | `--stop-time=-1`、`--tolerance 0`、`--solver foo`、`--solver cvode` | 1 | stdout | `error[GM-E10004] … 区間が空`、`GM-E10004 … Tolerance（0）が正でない`、`GM-E10005 ソルバ「foo」は無い` と使えるソルバの一覧、cvode は「このビルドでは未実装。近い手法: bdf」 | 分かる。最後の行は `RC: 実行に失敗した` だけで、理由は上の行 |
| E14 | `--param R`・`--param R=abc`・`--param zz=1` | 2・2・1 | stderr・stderr・stdout | `--param R: 「名前=値」の形でない`、`値「abc」が数でない`、`GM-E10006 … zz という変えられる（tunable な）パラメータは無い` + `rumoca: [EX003]` | 分かる。`--list-parameters` を案内しない |
| E15 | **書けない出力先** `-o /nonexistent/dir/x.csv` | 2 | stderr | `error: 結果を /nonexistent/dir/x.csv に書けない: No such file or directory (os error 2)` | 分かる。ただし**計算の後**に分かる（ChuaCircuit で 1.64 s、長い計算なら全部無駄） |
| E16 | `fmu export --out /proc/nope/x`、`--out rc.mo`（ファイル） | 1・1 | stderr | `error: No such file or directory (os error 2)`、`error: Not a directory (os error 20)` | **どのパスか言わない** |
| E17 | `fmu export --build`（`--fmi-headers` なし） | 2 | stderr | clap の英語。`required arguments were not provided: --fmi-headers <FMI_HEADERS>` | 何を渡すか分からない。ヘッダの入手先はヘルプに「例: FMPy の `c-code`」とだけ |
| E18 | 強行 `simulate unbal.mo --force`、未対応に `--force` | 1 | stdout | 同じ診断が **2 回**出て、最後は `DAE まで変換できないので実行できない`。通常は `実行しない（… 強行しても実行できない）` | 分かる。重複は見苦しい |
| E19 | 壊れた設定 `globe-mode.toml`（`[fmt]` の `indnet = 4`、`[fmt` の閉じ忘れ） | 2 | stderr | `error: <絶対パス>/globe-mode.toml: 知らないキー fmt.indnet`、TOML の位置つきの誤り | 分かる（良い例） |
| E20 | バイナリ・空のファイル | 2・1 | stderr・stdout | `bin.mo を読めない: stream did not contain valid UTF-8`、空は `モデル \`X\` が見つからない` | 分かる |
| E21 | 打ち間違いの副命令 `globe chek`、引数なし | 2 | stderr | `unrecognized subcommand 'chek'`、`tip: a similar subcommand exists: 'check'`。引数なしはヘルプ | 分かる（clap の英語） |

panic・Rust の backtrace・`RUST_BACKTRACE` の案内は、どの誤った使い方でも出なかった（0 件）。

## 4. 環境と出力の見え方

| 観点 | 結果 | 確認 |
| --- | --- | --- |
| `\| head`（パイプ） | 小さい出力は何も起きない。大きい CSV を `simulate -o - \| head -1` にすると、`error: 結果を - に書けない: Broken pipe (os error 32)` が stderr に出て**終了コード 2**。ほかの道具は黙って終わる | 【操作】 |
| `NO_COLOR` | 色は元から使っていない（ESC は 0 バイト）。効く・効かないの問題が無い | 【操作】 |
| `LC_ALL=C` | 文面は UTF-8 のまま日本語で出る（`LC_ALL=C` でも崩れなかった。端末側が UTF-8 のとき）。非 UTF-8 の端末では読めない【未確認】 | 【操作】 |
| 空白・日本語のパス | `check "日本語 dir with space.mo" -m Jp`、`-o "出力 結果.csv"` が通った。MSL の絶対パスの `Modelica 4.1.0`（空白入り）も診断の先頭にそのまま出る | 【操作】 |
| stdout と stderr の使い分け | 診断・要約は `check`・`simulate` とも **stdout**。`simulate -o -`（CSV を標準出力へ）のときだけ診断・要約が stderr に移る（CSV を壊さないため）。`--json` の誤り（読めない入力）は stdout が空で stderr に平文。`fmt` の整形済みの件数は stdout、個々の失敗は stderr | 【操作】 |
| 成功した MSL の例題の出力 | PID_Controller は `check` が stdout に 11 行・約 3 KB。先頭 4 行は `info[GM-I7001]`・`GM-I7002` と `note:` で、**絶対パス（空白入り）**から始まり、微分した方程式を 1 行に並べる。`simulate` も同じ（約 2.2 KB）。要約は最後の数行 | 【操作】 |
| 終了コードの規約 | 0 成功 / 1 Error の診断・`--check` で要整形 / 2 入力の読み込み失敗・clap の誤り・書き出しの失敗。ソース先頭の注と実測が合う。`fmt` の標準入力の構文誤りは 1、ファイルの構文誤りは 2 | 【読】`main.rs`・【操作】 |
| 出力の保存の案内 | `simulate` を `-o` なしで回すと**結果は捨てられる**（要約の最後の値だけ）。「`-o` で保存」の案内も、書いた後の「書いた」の行も無い | 【操作】 |
| `fmt` の既定の挙動 | `fmt`・`fmt --check` を**パス無し**で、入力が TTY でないとき（CI・スクリプト・`< /dev/null`）は標準入力を読む。整形が要るファイルがカレントにあっても、**何も出さず終了コード 0** | 【操作】`fmt --check < /dev/null`（`q.mo` が要整形）が exit 0・出力 0 バイト |

## 5. つまずき（事実）

番号は 6 章の順位とは別。種類は a（最初の 10 分）・b（機能の穴）・c（ユーザーの判断）。

| 番号 | 種類 | 事実 | 確認の方法 |
| --- | --- | --- | --- |
| P1 | a | **ライブラリを使うときの「モデルが無い」が英語の原文で候補が無い**。CLI の E10 と同じ。Python も `ws.check("PID_Controller")` は例外にならず `{"passed": False}`、診断は `PID_Controller could not be compiled because resolve/parse failed`。ファイル入力の「モデル `Nope` が見つからない。候補: RC」（GM-E1002）はライブラリのときは出ない | 【操作】E10、`py2.py` |
| P2 | a | **MSL の置き場の誤りの手がかりが無い**。`-L` を忘れた（E4）と、サブフォルダを渡した（E5）は、どちらも直し方に届かない。Python の `Workspace("data/msl")`（リストでなく文字列）は 1 文字ずつ別のライブラリとして読み、`ライブラリ d を読み込めない` | 【操作】E4・E5、`py2.py` の `ws-str-libs` |
| P3 | a | **`simulate` は `-o` を付けないと結果を捨てる**。要約だけで、保存の案内も、保存後の「書いた」の行も無い。書けない出力先は計算の後で分かる（E15） | 【操作】`sim-rc`、T10 |
| P4 | a | **`fmt` のパス省略と非 TTY**（4 章）。設計の意図（パイプなら標準入力、S060）どおりだが、CI で `globe fmt --check` と書くと**何も検査せずに通る**。ruff は標準入力を `-` のときだけ読む | 【操作】、ヘルプの「`-`、または `--json` か入力がパイプのときは標準入力」 |
| P5 | a | **負の数の引数が通らない**（E12）。clap の `allow_negative_numbers` が無く、`--start-time -2` が使えない。出る tip（`-- -2`）は誤り | 【操作】S4・T11 |
| P6 | a | **ヘルプが初見に厳しい**。(1) 例が 1 つも無い。(2) `-m` が必須で位置引数でないこと、`-L` が要ることはヘルプの 1 行目では分からない。(3) `check` の説明は「（S0〜S8）」と古い（実際は S10 まで通す。`simulate` は「S0〜S10」）。(4) 説明に内部の通し番号（`S053`・`S011`・`S094`・`S130` など）が出る。(5) `outline <FILE>` は引数の説明が空。(6) clap の定型部分（`Usage`・`Print help`・`error:`・`tip:`）は英語で、説明文は日本語 | 【操作】`globe --help`・各 `--help` |
| P7 | a | **内部の表示が漏れる**。`No such file or directory (os error 2)`（E1・E15・E16）、`Broken pipe (os error 32)`、`stream did not contain valid UTF-8`。Python は `ValueError: settings が読めない: invalid type: string "abc", expected f64`（**どの設定か言わない**）。`RuntimeError` の本文が JSON の文字列（`[{"code":"GM-E1003",…}]`） | 【操作】`py2.py` |
| P8 | a | **成功した検査の診断が長い**（4 章）。info の長い 1 行、絶対パス。要約が最後の数行に埋まる。静かにする・詳しくするの切り替え（`-q`・`-v`）が無い。S164 の N7（UI の診断の「情報」が長い）の CLI 版 | 【操作】PID_Controller |
| P9 | b | **未対応（GM-U）が CLI で誤りと同じに見える**（E9）。終了コード 1・最後の行 `NG`・赤い表示は無し。MSL の例題を CLI で回す CI で、モデルの誤りと Rumoca の限界を終了コードで分けられない。UI は S173 で分けた | 【操作】E9・S13 |
| P10 | b | **Python の進捗と中断は、既定のソルバでは積分の中に届かない**。ChuaCircuit を `stop_time=2e6`（8.9 s）で `simulate_columns(on_progress=…)` に渡すと、イベントは 5 個（lowering・preflight・build・initialization・integration）だけで `time` は全部 `None`、コールバックが `True` を返しても 8.67 s で完走（`termination` は `stop_time`）。`dopri45`・`auto` も同じ。`esdirk34` なら 22 個中 17 個に時刻が付く。**ADR-0033 に既知の制約として書いてある**（ネイティブの型付きの道は段だけ）が、`simulate` の docstring は「積分の t は 100 ms に 1 回まで」とだけ書く。Ctrl-C も同じく次の段まで届かない | 【操作】`py3.py`・【読】ADR-0033 の 41・134 行 |
| P11 | a | **Python の型・docstring・例外**。(1) `py.typed`・`.pyi` が無く、戻り値は `dict`（`check` は 13 キーの dict）。(2) `help()` で空なのは `Workspace`・`check`・`graph`・`load_dir`・`set_file`・`solvers`（docstring があるのは `simulate`・`simulate_columns`・`export_fmu`・`table`・`format`・`lint`）。(3) 例外クラスは組み込みの `ValueError`・`RuntimeError`・`TypeError` だけで、独自の基底が無い。同じ「悪い入力」が、未知のモデルは例外なしの `passed: False`、設定の型の誤りは `ValueError`、ライブラリが読めないのは `RuntimeError` と割れる。(4) **未知の設定キーは黙って無視**（`simulate("RC", stoptime=3)` は `stop_time` が 5 のまま `ok: True`）。FMU の `options` は未知のキーを弾く。(5) 結果の成否のキーは `ok`（Python・facade）と `succeeded`（CLI の `--json`）で違う。(6) `globe_mode.main()`（`globe-mode` の入口）の文面が `cargo run -p globe-mode-cli -- --help` を案内し、`src/Python/README.md` は「今は uv_build でビルドし、P6 で PyO3 と maturin に置き換える」のまま | 【操作】`py2.py`・`py3.py`、【読】`__init__.py`・`src/Python/README.md` |
| P12 | b | **Python に無いもの**。`outline`・`flat`・`deps`・LSP・ファイル / ディレクトリの整形（`globe fmt` 相当。`format(text)` と `lint(text)` は文字列だけ）、`check` の人が読む要約（CLI の「方程式 44 / 変数 44 …」にあたる文字列。dict を自分で読む）、Jupyter での表示（`_repr_html_` なし。`check` は dict の長い表示）。`graph` は CLI より多く返す | 【操作】`dir(globe_mode)`、【読】`globe-mode-py/src/lib.rs` |
| P13 | b | **CLI の面が揃っていない**。`outline`・`graph`・`deps` に `-L` が無い。MSL の ChuaCircuit の `graph` は、MSL を位置引数に渡して 6.98 s（`check -L` は 1.2 s）。`graph -L …` は clap の `to pass '-L' as a value, use '-- -L'` というおかしな tip。`deps` は無いモデルでも終了コード 0 | 【操作】T15・`graph-L`・T17 |
| P14 | b | **MSL を取る道が CLI に無い**。`cargo xtask fetch-msl` はチェックアウトと Rust が要る。Release の `globe`（`release.yml` は 3 ターゲットを作る。公開 0 件）だけを入れた人は MSL を取れない。Desktop には `fetch_msl` がある（ADR-0030） | 【読】README・`release.yml`・`mise.toml` |
| P15 | b | **FMU の `--build` を最初に通せない**。`fmi2Functions.h`（3.0 は `fmi3Functions.h`）のあるディレクトリが要り、ヘルプは「例: FMPy の `c-code`」とだけ。`cargo xtask fetch-reference-fmus` の取得物（`data/reference-fmus`）にヘッダは**無い**（`find` で 0 件）。通ったのは OpenModelica のパッケージのヘッダを借りたとき。書き出した `.so` の名前は `Rumoca_<uuid>.so`（モデルの名前でない）。FMI 2.0 と 3.0 は同じ `<名前>.fmu` で上書きされる | 【操作】`fmu-build`・`unzip -l` |
| P16 | a | **他言語の README は「最初の例」が `mise run` だけ**。Zig・Julia・Elixir・Gleam とも、自分のコードから呼ぶ数行が README に無い（API の一覧は Elixir・Gleam の README にある）。Julia の README は「`Workspace`・`set_file`・`check`・`simulate`」の 4 つだけを挙げ、実物（`globe_mode.jl`）には `load_dir`・`simulate_columns`・`simulate_observed`・`export_fmu`・`graph`・`format`・`lint` もある。配布物は無い（Hex・JLL は未。既知）。4 本とも参照シナリオは通った | 【操作】【読】各 README・`globe_mode.jl` |
| P17 | a | **marimo のノートブック**。`sim_app.py` は `Decay` 固定で、`Path(__file__).parents[2] / "Modelica"` を読む（チェックアウトの外では動かない）。自分のモデルを開く欄が無い。`marimo export html` は `src/Python/marimo/__marimo__/session/*.json` を作り、`.gitignore` に無いので `git status` に出る | 【読】`sim_app.py`・【操作】 |
| P18 | c | **0.2.0 での CLI と Python の配布物**。Release・PyPI は 0 件（ユーザーの判断。roadmap の「いま残っているもの」）。CLI は `release.yml` が 3 ターゲット（linux x86_64・aarch64、windows x86_64）、Python は wheel のみ。macOS 向けの `globe` は `release.yml` に無い | 【読】`release.yml`・README |

## 6. 穴の順位（上位 13）

### 6.1 順位

| 順 | 種類 | 穴 | 利用者への影響 | 見込み |
| --- | --- | --- | --- | --- |
| 1 | a | ライブラリのモデル名の誤りに、候補つきの日本語の誤りを出す（P1）。CLI・Python・LSP で同じ | 最初に打つ `-m PID_Controller` の間違いで、英語の原文だけが返る | S |
| 2 | a | `-L` の手がかり（P2）: `-L` を忘れたときの案内、`-L` がパッケージ本体のとき（サブフォルダ）の案内、Python の `libraries` に文字列を渡したときの誤り | MSL を入れた初日に、直し方の分からない誤りが 2 通りある | S |
| 3 | a | `fmt --check` が CI で黙って通る（P4）。パス省略 + 非 TTY の扱い | 整形の検査が効かない。気づかない | S（決めてから） |
| 4 | a | `simulate` の `-o` なしと書き出し先の案内、書けない出力先を計算の前に検査（P3） | 結果を捨てる。長い計算の後で失敗 | S |
| 5 | a | 負の数の引数（P5） | `--start-time -2` が使えない。tip が誤り | S（1 行） |
| 6 | a | ヘルプの整備（P6）: 例、内部の通し番号を消す、`check` の S8 を直す、`-L` と `-m` の説明 | 最初の 10 分の入口 | S |
| 7 | a | 成功した検査の診断を畳む（`-q`・`-v`）、パスを短く（P8） | 要約が埋もれる | S〜M |
| 8 | a | 内部の表示の漏れ（P7）: `os error`、`Broken pipe`、serde の型の誤りに設定名を付ける、`--out` のパスを名指す | 何が悪いか分からない行がある | S |
| 9 | b | 未対応（GM-U）を CLI で分ける（P9）。終了コードと最後の行 | MSL を CLI で回す CI で誤りと限界を分けられない | S（決めてから） |
| 10 | b | Python の型・docstring・例外・未知の設定キー（P11） | 打ち間違いが黙って通る。IDE の補完が効かない。例外が割れる | M |
| 11 | b | Python の進捗・中断の説明と、既定のソルバでの届き方（P10） | 「止められる」と読めるのに既定では止まらない | S（説明）、M 以上（実装。Rumoca に口が無い） |
| 12 | b | CLI の面を揃える（P13）と MSL を取る道（P14）、Python に `flat`・`outline`・`deps`・要約（P12） | CLI だけで完結しない | M |
| 13 | b | FMU `--build` の入口（P15）: ヘッダの入手、`.so` の名前、版つきのファイル名 | 書き出した FMU が他のツールで読めるまでが長い | M（同梱するならユーザーの判断） |

次点: 他言語の README に「自分のコードから呼ぶ数行」（P16）と Julia の README の更新（S）、marimo のノートブックを自分のモデルで（P17。S〜M）、`src/Python/README.md`・`globe_mode.main()` の古い文面（S）。順位の付け方は S164 と同じ。最初の 10 分で必ず当たるものを先に、機能の穴を次に。

### 6.2 WP の案（1 WP = 半日以内。互いにぶつからない組）

組が違えば触るファイルが重ならない。同じ組の中は順に回す。

| 組 | WP | 内容 | 触るファイルの見込み | 見込み |
| --- | --- | --- | --- | --- |
| A（モデル名・`-L`） | A1 | ライブラリ使用時の「モデルが無い」を GM-E1002 + 候補（末尾の一致・前方一致・近い名前）にする。短い名前・`.mo` つき・パッケージ名のときの案内。`-L` が `package.mo` を直下に持つとき（サブフォルダ）の案内、`-L` を忘れたときの案内（CLI の `required_unless_present` の誤りを `-L` つきの文に）。Python の `libraries` が `str` のとき `TypeError` | `globe-mode-frontend`（名前解決の診断）か `globe-mode` の facade、`globe-mode-cli/src/main.rs`（`-L` の検査）、`globe-mode-py/src/lib.rs`、snapshot | S〜M |
| B（CLI の入出力） | B1 | `simulate` の `-o` なしの案内、書き出し先の事前の検査（計算の前に作れるか）、書いた後の「書いた: <パス> （n 行）」、`Broken pipe` を終了コード 0 で静かに、`os error` を落として対象のパスを名指す（E1・E15・E16）、`fmu export --out` のパス | `globe-mode-cli/src/simulate.rs`・`main.rs`、`tests/*.rs` | S |
| B | B2 | 負の数の引数（`allow_negative_numbers`）、ヘルプの整備（例、内部の通し番号を落とす、`check` の S8、`-m`・`-L` の説明、`outline` の引数）、`graph`・`deps`・`outline` に `-L` | `globe-mode-cli/src/main.rs` の clap の定義、`tests/snapshots` | S |
| B | B3 | `-q`・`-v`（成功した検査の info を畳む・詳しく）、診断のパスを相対にする。**`fmt` のパス省略と非 TTY の挙動、未対応の終了コードは、ユーザーの判断の後**（7 章） | `globe-mode-cli/src/main.rs` の診断の印字、`tests/json_snapshots.rs` | S〜M |
| C（Python） | C1 | `libraries` の型の検査、未知の設定キーを `ValueError`（`stoptime` を弾く。CLI の `SimSettings` は `deny_unknown_fields` を使えるか確かめる）、`GlobeModeError`（`ValueError` を継ぐ）、`RuntimeError` の JSON の文字列を読める文にする、docstring（`Workspace`・`check`・`graph`・`load_dir`・`set_file`・`solvers`）、`py.typed` と `TypedDict` の戻り型 | `src/Python/mod/globe_mode/__init__.py`、`globe-mode-py/src/lib.rs`、`src/Python/tests/test_api.py` | M（半日を超えうる。`TypedDict` は後に回してよい） |
| C | C2 | `outline`・`flat`・`deps` と `check` の要約文字列（`Workspace.summary` など）を Python に、`src/Python/README.md`・`globe_mode.main()` の古い文面、`on_progress` の docstring に「既定のソルバでは段だけ」と書く | `globe-mode-py/src/lib.rs`、`__init__.py`、`src/Python/README.md` | S〜M |
| D（文書・他言語） | D1 | 他言語の README に自分のコードから呼ぶ数行（Zig・Julia・Elixir・Gleam）、Julia の README を実物に合わせる、marimo の `sim_app.py` にモデルと `-L` の入力欄、`__marimo__/` を `.gitignore` | `src/{Zig,Julia,Elixir,Gleam}/README.md`、`src/Python/marimo/sim_app.py`・`README.md`、`.gitignore` | S |
| E（FMU） | E1 | `--build` の入口: ヘッダの入手（取得の `xtask` か同梱。**同梱はユーザーの判断**）、`.so` の名前、FMI の版つきのファイル名 | `globe-mode-fmi/src/export.rs`、`globe-mode-cli`、`src/Rust/xtask` | M |

並行の組み方: A・B・C・D は触るファイルが違う（A は frontend と `-L` の検査で `main.rs` の 1 か所を B と共有する。**A1 の `main.rs` の差分は小さいので、B より先に済ませる**か、B2 の後に回す）。B 列の 3 つは `main.rs` を共有するので順に。C は Python だけで、A1 の診断の文面が先に変わると `test_api.py` の期待値を直す。**A1 → B1 → B2 → B3** の順で 1 人、C と D は別に。

## 7. ユーザーの判断が要るもの

WP にしない。判断の材料である。

| 項目 | 判断の中身 | 材料 |
| --- | --- | --- |
| `fmt` のパス省略 + 非 TTY | いまの「パイプなら標準入力」（ruff 風の S060）を、`-` のときだけ標準入力に変えるか。変えると、エディタが `globe fmt` をパス無しで標準入力に流す使い方（`--json` でない）を壊す | P4。`globe fmt` のヘルプ。VS Code の拡張は LSP 経由か、Phoenix・Wisp・marimo のアプリは CLI を標準入力で呼ぶ（使われ方は【未確認】） |
| 未対応の終了コード | `GM-U` だけのときに終了コード 3 にするか（`--json` の形は変えない）。既存の使い方（MSL の funnel などは JSON を読む）を変えない案が安全 | P9。UI の S173 |
| FMI のヘッダの同梱 | `fmi2Functions.h` などを GlobeMode に入れるか、取得にするか。BSD-2 の再配布の扱い。入れないなら `--build` の案内だけ | P15。FMI の標準のヘッダのライセンスは【未確認】 |
| Python の例外の階層 | `GlobeModeError` を足し、既存の `ValueError`・`RuntimeError` を継ぐ（互換）か。「未知のモデル」を例外にするか（いまは `passed: False`） | P11。0.2.0 の公開前なら互換の制約は軽い |
| CLI の `msl fetch` | `globe` に MSL を取る副命令を足すか（Release のバイナリだけで使えるように） | P14。ADR-0030（Desktop の置き方）。公開の判断と連動 |
| 公開（CLI・Python） | CLI の Release、macOS 向け `globe`、PyPI。いま 0 件 | P18。roadmap の「いま残っているもの」の最初の 2 行 |
| ヘルプ・誤りの言語 | 説明は日本語、clap の定型は英語のままか、clap を日本語に直すか（`error:`・`tip:`・`Usage`）。README は日本語と英語の要約がある | P6。CLAUDE.md の「文書は日本語」 |

## 8. 見つけた不具合（コードは直していない）

| # | 事実 | 場所 | 見込み |
| --- | --- | --- | --- |
| B1 | `--stop-time -1` の clap の tip（`-- -1`）が誤り（`=` で渡せば通る）。`--start-time` が負の値を受けられない | `main.rs` の `#[arg]`（`allow_negative_numbers` なし） | 1 行 |
| B2 | `globe check` の説明が「（S0〜S8）」のまま（実際は S10 まで） | `main.rs` の `Check` の doc コメント | 1 行 |
| B3 | `fmt --check`（パス無し・非 TTY）が黙って 0 を返す | `main.rs` の `fmt` の入力の判定 | 判断が先（7 章） |
| B4 | Python の未知の設定キー（`stoptime`）が黙って無視される | `__init__.py` の `simulate`（`json.dumps(settings)`）と `SimSettings` の読み | 小 |
| B5 | `src/Python/README.md`・`globe_mode.main()` の文面が古い | 上のファイル | 1 行ずつ |
| B6 | 強行（`--force`）で診断が 2 回出る | `simulate.rs` | 小 |
| B7 | `simulate` の最後の行 `実行に失敗した` が理由を言わない（理由は上の行）。`--json` の `refused` も同じ | `simulate.rs` | 小 |
| B8 | `marimo export html` が作る `src/Python/marimo/__marimo__/` が未追跡のまま `git status` に出る | `.gitignore` | 1 行 |
| B9 | Julia の README が実物より少ない | `src/Julia/README.md` | 文書 |

## 9. 再現の手順

```bash
# 確認日 2026-11-06、main b14345f
export CARGO_TARGET_DIR=<main>/.git-worktree/target-shared CARGO_BUILD_JOBS=2
cargo build --release -p globe-mode-cli && "$CARGO_TARGET_DIR/release/globe" --version   # globe 0.2.0
# CLI（-L は data/msl ごと）
globe check -L data/msl -m Modelica.Blocks.Examples.PID_Controller
globe check -L data/msl -m PID_Controller                       # P1（英語の原文、候補なし）
globe check -L "data/msl/Modelica 4.1.0" -m Modelica.Blocks.Examples.PID_Controller   # P2
globe simulate rc.mo -m RC --stop-time -1                       # P5（clap の誤った tip）
globe fmt --check < /dev/null                                   # P4（要整形のファイルがあっても exit 0）
# Python（scratch の venv）
uv venv "$SCRATCH/pyvenv" && uv pip install --python "$SCRATCH/pyvenv/bin/python" ".[widget]"
# on_progress が積分の中に届かない（P10）: ChuaCircuit を stop_time=2e6 で回し、solver を bdf と esdirk34 で比べる
# 他言語
mise run c:zig; mise run c:julia; mise run elixir:test; mise run gleam:test
```

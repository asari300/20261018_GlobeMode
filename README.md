# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode checks, simulates and edits [Modelica](https://modelica.org/) models. The same Rust core runs as a CLI (`globe`), a language server, a Tauri desktop app (Linux, Windows, macOS), an Android app, in the browser as WebAssembly (the computation stays on the visitor's machine), in Python, and from C, Zig, Julia, Elixir and Gleam. The compiler frontend is [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0, pinned git dependency) behind an adapter.

> **Status: pre-release.** Nothing is published yet: no GitHub Release, tag, GitHub Pages site, PyPI package, crates.io crate or VS Code Marketplace entry. Build from a checkout, or take the build artifacts of the GitHub Actions workflows. The desktop and Android builds are unsigned. Details are in Japanese below ("使い始め方"); changes are in [CHANGELOG.md](CHANGELOG.md).

## What works

- **Checks before simulation** (S0 to S10): syntax, names, types, connections, equation/variable balance, structural singularity, index, initialization, algebraic loops, parameter and start-value evaluation. Errors name the reason a model cannot be simulated. The balance check agrees with OpenModelica's `omc checkModel()` on all 382 MSL examples that have a verdict ([doc/omc-compare.md](doc/omc-compare.md)).
- **Simulation** with 20+ solvers (`bdf`, `dopri45`, `auto`, `esdirk34`, `radau5`, `rk4`, ...; `cvode` and `ida` only in a build with the `sundials` feature; list in [doc/msl-solvers.md](doc/msl-solvers.md)), parameter overrides, CSV and MAT v4 output (and `globe results` / Python `read_results` to read them back), progress and cancel. **Several solvers at once**: `globe simulate --solvers bdf,esdirk34,radau5 [--parallel] [--compare 6] [--max-order 5]` runs one model with up to 16 solvers (checked and compiled once; `-o DIR` writes `DIR/<solver>.csv`), and `--compare N` judges every run against a reference with the rectangular tube that the MSL reference comparison uses (width 10^-N), printing a table. The run settings of the UI have the same: a "several" solver choice (check boxes), one-by-one or parallel, "compare the results when all are done" (off by default), and the maximum integration order (`cvode` and `ida` only; other solvers ignore it with a note). The runs are overlaid in the plot as `x [bdf]` and listed in a run table and a comparison table ([ADR-0040](doc/adr/0040-multi-solver-compare.md)). Limits: parallel runs are threads of one process (up to 4, sharing the compiled model; the browser runs them one by one), so a hard run cannot be killed from outside. The pass/fail verdict counts points outside the tube; the largest relative and absolute differences (per signal, per pair of solvers and overall; S328) are printed next to it as numbers, and near a point where the reference crosses 0 a tiny absolute difference gives a large relative one, so the verdict is the tube's. `-o DIR` is checked before the run, solver names are compared without case and surrounding spaces (`bdf` and `BDF` are duplicates), and `--compare` with a single solver is refused (S327). The browser view of a several-solver run is covered by a test in a real Chromium (S348), which also fixed settings labels that broke between the name and the input. `auto` (BDF, then esdirk34, then dopri45 when one fails) hands each retry only what is left of the time budget, so `--timeout` is a real limit; when nothing is left the run ends as timed out (`GM-E11005`; S344, [ADR-0041](doc/adr/0041-auto-remaining-budget.md)).
- **Editing**: formatter, lint, completion, hover, go-to-definition, references and rename (components, class names and `import` aliases) in the language server and the VS Code extension, and in the browser/desktop/Android UI. The language server reaches the `.mo` files under the project root that are not open, the libraries (`globe lsp --library`) and inherited components (S230, S235; library files are never rewritten; the target of a `redeclare` is not reached). The UI editor stays within the open document (S161, S166, S235); library tree, plot (series selection, right axis, CSV/PNG export), connect graph, tabs, open/save, run settings, FMU export, dark mode.
- **Parameter dialog**: double-click a component in the diagram (or press "Parameters..." in the run settings) to list its parameters (name, type, default, unit, description). A value goes into the run settings as a run-time override (the source is not touched), and "Write to source" writes it as the declaration's modifier `(k = 2)` in the editor (one undo step, nothing is saved); "Remove modifier" takes it off again, and an enumeration is written as `Type.literal`. Nested names, `each`, `final` and broken expressions are refused with a reason.
- **FMU export** (FMI 2.0 and 3.0, Model Exchange and Co-Simulation) from the CLI, the UI and every language API. FMI import (Co-Simulation and Model Exchange) runs from the CLI (`globe fmu run`), the Rust facade, Python (`run_fmu`, and `cosim` for several FMUs wired together from a dict, `.json` or `.ssp`) and the Desktop UI (Open a `.fmu`, with a confirmation; not the browser or Android): the FMU's native code runs in a separate worker process, so a crashing FMU does not take the CLI down (`--timeout`, `--memory-limit` on Linux; the disk and network are not isolated).
- **Co-simulation and SSP (minimum version)**: `globe fmu cosim` couples several Co-Simulation FMUs (FMI 2.0 and 3.0 mixed) with a fixed communication step, Jacobi or Gauss-Seidel, `start` values and Real/Int32/Boolean connections, from a JSON spec or from an SSP 2.0.1 `.ssp` (components, connections, `ParameterBindings` from `.ssv` or inline). `globe fmu ssp-export spec.json -o system.ssp` writes the same spec as an SSP with one System (Real, Int32 and Boolean connections; reading it back with `globe fmu cosim` gives the same CSV); `--ssv` writes `start` values to a per-component `.ssv` instead of inline. `globe fmu ssp-import system.ssp -o work/spec.json` goes the other way (the FMUs are extracted next to the JSON; it never overwrites an existing file). In the Desktop UI, Open also takes a `.ssp` or a spec `.json` and plots the co-simulation result (not the browser or Android). No variable step, no algebraic loops ([ADR-0026](doc/adr/0026-fmi-import-placement.md)). Reading a `.ssp` with nested Systems flattens them to `outer.inner` component names: a connection through a System's own connector becomes one wire, and a System's `ParameterBindings` follow the SSP rule (the upper level wins); writing nested Systems is not done. FMI 3.0 Scheduled Execution export covers continuous models only (P5b, `--kind se`). Exported FMUs carry `dependencies` for outputs and derivatives.
- **Analysis views**: `globe linearize` prints the state-space matrices A, B, C, D by numerical central differences (CLI, Python `Workspace.linearize`, and the toolbar "Linearize" button, which shows the A, B, C, D tables and exports JSON; the result also carries the eigenvalues of A and a stability label, stable / unstable / marginal, shown in the UI as an eigenvalue table and a one-line label; the C ABI, Zig, Julia, Elixir and Gleam can call `linearize` and `blt` too; [ADR-0038](doc/adr/0038-linearize-numeric.md)). `globe blt` and the toolbar "BLT" button show the BLT blocks in solving order with algebraic loops highlighted (CLI, UI, LSP and Python `Workspace.blt`; no jump to the source yet). `globe stages` summarizes how a model changes at each transform stage (flat, DAE, index reduction, BLT, tearing; also from the LSP, wasm and Python `Workspace.stages`; the toolbar "Stages" button opens a panel in the UI, and the flat to DAE step is counts only). A marimo notebook that tries these in one place is `src/Python/marimo/analysis_app.py`.
- **Icon and Diagram views**: the library tree draws each class's icon, and the right panel shows the Icon and a Diagram of the class under the cursor (also in the VS Code side panel). In the VS Code panel a selected component can be moved with the arrow keys (2 units, Shift for 10) and Enter opens its parameters in a QuickPick/InputBox that writes the declaration's modifier (double-click works too; the command "GlobeMode: Edit part parameters" starts from the cursor); a connection is added from the keyboard in the UI and in the panel (select a component, press `c`, pick its connector with Tab or the arrow keys and press Enter for the start, then a connector of another component and Enter for the end; Esc cancels; `+` on a selected connection adds a middle point), while moving and deleting points stay mouse-only. In the UI and in the VS Code side panel you can drag a component to move it and drag from one connector to another to add a `connect(...)`; in the UI you can also drop a library-tree row on the diagram (or press its "to diagram" button) to add a component. Selecting a component or a connection in the diagram (UI and VS Code panel) and pressing Delete removes it (a component also removes its `connect`s; an indexed end such as `b[2].p` is told apart from `b[1].p`; not for a declaration with several names). All are edits of the source text in the editor (undo works, nothing is saved). Selecting a connection shows a handle on each middle point: drag it to move the point, double-click the line to add one (the two end points stay; a connection without `points` gets a three-point `Line`), in the UI and in the VS Code panel. Deleting a connection point, moving one by keys, and adding a component from the VS Code panel, are not done ([ADR-0037](doc/adr/0037-icon-diagram-as-shape-data.md)).
- **Language server on files with syntax errors** (first use of the own Track B parser): the outline and folding ranges still work when Rumoca cannot parse a file, and `a.` completion works in a half-written document (only when the existing path returns nothing; the existing output is unchanged). Go-to-definition and hover also answer in a document Rumoca cannot read, from the own syntax and name resolution (same rule: only when the existing path has no answer).
- **Inlay hints, call hierarchy, completion after `redeclare`** (S345, S352, S354): inlay hints put the parameter name before each positional argument of a function call (`f(x, y)` shows `f(a: x, b: y)`; the names are the callee's `input` list, found by the own name resolution; nothing is shown for built-ins, components, names it cannot resolve, or named arguments; they also appear in a half-written document). Call hierarchy (`prepareCallHierarchy`, incoming and outgoing calls) is for Modelica `function`s only: callers are searched in the open documents, in the `.mo` files under the project root and in the libraries, and callees are the functions the body calls (unresolvable names, built-ins and record constructors are left out). The VS Code extension uses both automatically, and the workflow `vscode.yml` checks them in a real VS Code on Linux, Windows and macOS (what the editor draws on screen is not checked). Completion after `a.`, go-to-definition and hover also see a type replaced by `redeclare` or `class extends` (S345; S351 keeps it from listing the fields of a same-named type that was not replaced).
- **Formatter and line comments** (S343, S346): when the formatter sorts the arguments of `annotation(...)` (and other parenthesised arguments), a line comment moves together with its argument, both a trailing `// ...` on the same line and a comment line above the next argument; the `annotation-order` lint agrees.
- **Native check (trial, not the default)**: `globe check --frontend native` checks with the own Track B stages (parse, name resolution, flatten, DAE) instead of Rumoca, up to S6 (syntax, references, equation/variable balance, structural singularity); S7 and later are not checked. It reads only the closure of library files the model uses, and the balance and singularity diagnostics (`GM-E5002`, `GM-E6001`) carry a line and column. What the own stages cannot handle yet stops with `GM-U0006` (exit code 3); it does not fall back to Rumoca. The default stays Rumoca. The same choice exists in the language server (`[check] frontend = "native"` in `globe-mode.toml`, for the published diagnostics) and in Python (`Workspace.check(model, frontend="native")`), and in the browser UI (a "Rumoca / own (trial)" select next to the Check button; the wasm build includes the own stages, +0.35 MB or 1.8%). The Desktop UI (sidecar) has the same select (S349: the LSP `globe/check` takes `params.frontend`); its default "follow settings" (S350) sends no `frontend`, so `[check] frontend` in `globe-mode.toml` decides, as for the editor diagnostics (the browser has no settings file and treats it as Rumoca). The own flattener handles `redeclare`, `replaceable`, `class extends`, `Complex`, arrays of records and arrays whose size `[:]` comes from a binding. Of the 566 MSL examples, name resolution leaves no unresolved reference in all 566 (S341), 548 flatten (S355 adds `stream`: `inStream`, `actualStream` and the outside-connector equations; S361 adds `connect`s to an outer-scope `inner`, `inner outer`, `if` on `cardinality`) and all 548 become a DAE; `mise run msl:diff-frontend` compares it with Rumoca (0 regressions). Where Rumoca also gets through (521 examples; 377 of them reach its balance check), the variable names agree on 518 (in the other 3, Rumoca ignores a conditional declaration whose condition a `redeclare model` modification sets) and the equation count on 508 (the other 13 look like Rumoca's counting errors), and the balance verdict, the equation and unknown counts, the structural regularity and the state count agree on 377, 376, 377 and 376 of the 377 (S335 to S337, S355, S361, S362, S364). The 18 it still refuses: `composite-binding` 6, `expandable` 5, `non-constant-dim` 4 and 3 others ([doc/frontend-diff.md](doc/frontend-diff.md)).
- **Browser and Android load MSL in pieces**: the UI fetches only the chunks (`msl/c/*.json`) that the open model needs, found through `msl/index.json`, and prefetches two levels of dependencies in one round. For ChuaCircuit the first fetch is 4.6 MiB instead of 15.2 MiB and the wasm memory after loading is 14 MiB instead of 62 MiB. The artifact `globe-web` and the APK no longer carry `MSL.json` (the APK keeps the chunks in one file, `msl/pack.bin`, so it stays within 2% of its old size).
- **APIs**: Rust, Python (PyO3), C ABI, Zig, Julia, Elixir, Gleam, WebAssembly.

Not done: FMI import from the browser/Android UI and the C/Elixir/Gleam APIs (CLI, Rust, Python and the Desktop UI work), the rest of SSP (writing nested systems, units), an analytic Jacobian (the matrices are numerical), an own compiler frontend as the default (Track B: the parser and name resolution feed the outline, folding, completion, definition, hover, inlay hints and call hierarchy; the flattener and the DAE builder `globe-mode-dae` feed only the trial `--frontend native` check up to S6, not simulation; 548 of the 566 MSL examples flatten, and on the 377 that Rumoca also balances the balance verdict and the structural regularity agree on all 377; `mise run msl:diff-frontend` compares them with Rumoca), signed installers and automatic updates (the updater is off until a key is registered), a public web deployment. The full list with priorities is [doc/features.md](doc/features.md); the plan is [doc/roadmap.md](doc/roadmap.md).

UI keyboard shortcuts (browser, Desktop, Android with a keyboard; Cmd instead of Ctrl on macOS; they work inside the editor and in the input fields). Diagnostic and reference rows are real buttons: Tab to one and press Enter or Space to jump to the editor.

| Keys | Action |
| --- | --- |
| Ctrl+Enter | Run (this replaces CodeMirror's "insert blank line") |
| Ctrl+Shift+Enter | Check |
| Ctrl+. or Esc | Stop a running simulation (Esc is ignored in input fields and dialogs; with a completion list open it only closes the list) |
| Ctrl+Shift+PageUp / PageDown | Move the current tab left / right (tabs can also be dragged; the order is autosaved) |
| Ctrl+S / Ctrl+Shift+S | Save / Save as (Desktop) |
| Ctrl+O | Open |
| F12, Shift+F12, F2, Alt+Left | Go to definition, references, rename, back (editor) |
| Tab / Shift+Tab | In the editor: indent / outdent by two spaces (selected lines too); while the completion list is open, Tab accepts the selected candidate (Enter does too) |
| Esc then Tab (Shift+Tab), or Ctrl+M (Shift+Alt+M on macOS) | Move focus out of the editor to the next (previous) element; Ctrl+M toggles "Tab moves focus" |

## Getting started

All of these start from a checkout (`git clone https://github.com/asari300/20261018_GlobeMode`) and the tools pinned by [mise](https://mise.jdx.dev/) (`mise trust && mise install --locked`).

| Surface | How |
| --- | --- |
| CLI | `cargo build --release -p globe-mode-cli`, then `target/release/globe check -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit` (MSL: `cargo xtask fetch-msl`) |
| Browser (quickest way to see the GUI) | `mise run ui:preview` builds the wasm and the UI and serves it at <http://localhost:4173/> (first run takes a few minutes; `cargo xtask fetch-msl` first if you want MSL in the library tree). Or run the workflow `web.yml` and download the artifact `globe-web`. |
| Desktop | Run the workflow `desktop.yml` and download the artifact `desktop-linux` (.deb), `desktop-windows` (.msi) or `desktop-macos` (.dmg). Unsigned, so the OS warns. |
| Android | Run the workflow `android.yml` and download the artifact `globe-android` (debug APK). |
| VS Code | Build `globe`, then `cd src/VSCode/globe-mode && pnpm install && pnpm package` and install the `.vsix`. |
| Python | `uv sync` in the checkout, then `from globe_mode import Workspace, table`. |
| C, Zig, Julia, Elixir, Gleam | `mise run c:zig`, `c:julia`, `elixir:test`, `gleam:test` run the reference scenario. |

## Known limits

Of the 566 MSL 4.1.0 examples, 369 pass all checks, 13 fail them (model errors or structural singularity) and 184 hit a compiler limitation (`GM-U`, mostly in Rumoca v0.10.2). Of the 356 runnable examples that pass, 312 run to their stop time, and 303 of the 311 that have a reference result stay inside the reference tube (relative 0.2 %) of the MSL 4.1.0 reference results ([doc/msl-funnel.md](doc/msl-funnel.md), [doc/msl-sim.md](doc/msl-sim.md), run of 2026-11-03). Results for seven public libraries other than MSL are in [doc/libraries.md](doc/libraries.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions. How dependencies, Rumoca and the Modelica Standard Library are treated: [doc/licenses.md](doc/licenses.md).

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。同じ Rust のコアを、CLI `globe`、言語サーバ、Tauri の Desktop（Linux・Windows・macOS）、Android、ブラウザ（WebAssembly。計算は閲覧者の端末）、Python、C・Zig・Julia・Elixir・Gleam から使う。コンパイラのフロントエンドは [Rumoca](https://github.com/CogniPilot/rumoca)（Apache-2.0。git の rev で固定）をアダプタ越しに使う。

> **状態: 公開前。** GitHub の Release・タグ・Pages・PyPI・crates.io・VS Code の Marketplace には、まだ何も出していない。入れるには、チェックアウトからビルドするか、GitHub Actions のワークフローの artifact を使う。Desktop と Android のビルドは署名していない。変更点は [CHANGELOG.md](CHANGELOG.md)。

### できること

[doc/features.md](doc/features.md) の ○ のうち、いま main で動くもの。

- 実行前の検査（S0〜S10）: 構文、名前、型、接続、方程式数と変数数、構造特異、インデックスと状態選択、初期化、代数ループ、パラメータと start 値の評価。実行できない理由を名指しする。方程式数と変数数（S5）は OpenModelica の `omc checkModel()` と、判定のある MSL の例題 382 本すべてで一致した（omc は照合のために別プロセスで呼ぶだけ。[doc/omc-compare.md](doc/omc-compare.md)）。
- シミュレーション: 20 以上のソルバ（`bdf`・`dopri45`・`auto`・`esdirk34`・`radau5`・`rk4` など。`cvode`・`ida` は feature `sundials` のビルドだけ。一覧は [doc/msl-solvers.md](doc/msl-solvers.md)）、パラメータの上書き、CSV と MAT v4 の出力、進捗と中断。**複数のソルバで回して比べる**: `globe simulate --solvers bdf,esdirk34,radau5 [--parallel] [--compare 6] [--max-order 5]` が 1 つのモデルを最大 16 個のソルバで回し（検査とコンパイルは 1 回。`-o DIR` は `DIR/<ソルバ>.csv`）、`--compare N` で全部の実行を基準と、MSL の参照結果の比較と同じ矩形の管（幅 10^-N）で判定して表にする。UI の実行の設定にも同じ欄がある（ソルバの「複数」〔チェックボックス〕、1 つずつか並列か、「全部終わったら結果を比べる」〔既定 off〕、最大積分次数〔`cvode`・`ida` だけ。ほかは注記つきで無視〕）。結果は `x [bdf]` の名前でプロットに重ね、実行の表と比較の表も出す（[ADR-0040](doc/adr/0040-multi-solver-compare.md)）。比較の結果には、管の判定に加えて、変数ごと・ソルバごと・全体の最大の相対差と絶対差（基準の出力点に線形補間。S328）も出る。基準が 0 を通る点の近くでは相対差が大きく出るので、合否は管で決める。`-o DIR` は計算の前に検査し、ソルバ名は大文字小文字と前後の空白を無視して比べ（`bdf` と `BDF` は重複）、1 つだけの `--compare` は断る（S327）。ブラウザ版の複数ソルバの画面は本物の Chromium で通す試験があり（S348）、実行の設定の欄が名前と入力欄の間で折り返される崩れも直した。`auto`（BDF が失敗したら esdirk34、さらに失敗したら dopri45）の回し直しには、時間の予算の残りだけを渡す（`--timeout` が本当の上限になる。残りが無ければ時間切れ `GM-E11005` で終える。S344、[ADR-0041](doc/adr/0041-auto-remaining-budget.md)）。限界: 並列は同じプロセスのスレッド（最大 4 本、コンパイル結果を共有。ブラウザは 1 つずつ）なので、硬い実行を外から止められない。
- 編集: 整形・lint・補完・ホバー・定義ジャンプ・参照・リネーム（部品・クラス名・`import` の別名）を、言語サーバ、VS Code の拡張、ブラウザ・Desktop・Android の UI から。言語サーバは、開いていない根の下の `.mo`・ライブラリ（`globe lsp --library`）・継承した部品にも届く（S230・S235。ライブラリのファイルは書き換えない。`redeclare` の先は未）。UI のエディタは開いているタブの全部まで（S161・S166・S235・S368。開いていないファイルは未）。UI にはライブラリツリー、プロット（系列の選択・右の軸・CSV / PNG の書き出し）、connect グラフ、タブ、ファイルを開く・保存、実行の設定、FMU の書き出し（Desktop は P5a / P5b の選択つき）、ダークモードがある。
- パラメータのダイアログ: 図の部品のダブルクリック（か実行の設定の「パラメータ…」）で、部品のパラメータ（名前・型・既定値・単位・説明）の表が開く。値は実行の設定の上書きに乗る（ソースは変えない）。「ソースに書く」を押すと、部品の宣言の修飾 `(k = 2)` としてエディタに書く（元に戻せる。保存はしない）。「修飾を外す」で戻せ、列挙は `型.リテラル` で書く。入れ子の名前・`each`・`final`・壊れた式は理由つきで断る。
- FMU の書き出し: FMI 2.0・3.0 の Model Exchange・Co-Simulation を、CLI・UI・各言語の API から。FMI の読み込み（Co-Simulation・Model Exchange）は CLI の `globe fmu run`・Python の `run_fmu`（複数の FMU をつなぐ `cosim` は dict・`.json`・`.ssp`）・Desktop の UI の「開く」（確認つき。ブラウザ・Android は不可）。FMU の中のネイティブコードは別プロセス（ワーカー）で実行するので、FMU が落ちても呼び出し側は落ちない（`--timeout`、Linux は `--memory-limit`。ディスクとネットワークは隔離しない）。
- コシミュレーションと SSP（最小版）: `globe fmu cosim` が、複数の Co-Simulation の FMU（FMI 2.0 と 3.0 の混在可）を固定の通信刻みでつなぐ。Jacobi と Gauss–Seidel、`start`、Real・Int32・Boolean の接続に対応し、結線は JSON か SSP 2.0.1 の `.ssp`（`Component`・`Connection`・`DefaultExperiment`、`ParameterBindings` は `.ssv` かインライン）で渡す。`globe fmu ssp-export spec.json -o system.ssp` は同じ JSON を System 1 つの `.ssp` に書き出す（結線は Real・Int32・Boolean。`globe fmu cosim` で読み戻すと同じ CSV になる。`--ssv` を付けると `start` を Component ごとの `.ssv` に書く）。逆の `globe fmu ssp-import system.ssp -o work/spec.json` は、`.ssp` を spec.json にし、FMU を JSON の隣へ取り出す（同名のファイルは上書きしない）。Desktop の UI は「開く」が `.ssp` と spec の `.json` も受け、コシミュレーションの結果をプロットに出す（ブラウザ・Android は不可）。可変刻み・代数ループは未。`.ssp` の入れ子の System は読むとき `外側.内側` の名前の Component に平らにし（System のコネクタを通る接続は 1 本の結線になり、System の `ParameterBindings` は上位が勝つ）、入れ子の System の書き出しは未（[ADR-0026](doc/adr/0026-fmi-import-placement.md)）。FMI 3.0 の Scheduled Execution の書き出しは、クロックの無い連続モデルだけ（P5b、`--kind se`）。書き出した FMU には出力と導関数の `dependencies` も入る。
- 解析のビュー: `globe linearize` が状態空間の行列 A・B・C・D を数値の中心差分で出す（CLI、Python の `Workspace.linearize`、ツールバーの「線形化」ボタン〔A・B・C・D の表と JSON の書き出し〕。結果には A の固有値と安定性〔stable・unstable・marginal〕も入り、UI は固有値の表と安定性の 1 行を出す。C ABI・Zig・Julia・Elixir・Gleam からも `linearize`・`blt` を呼べる。[ADR-0038](doc/adr/0038-linearize-numeric.md)）。`globe blt` とツールバーの「BLT」ボタンが、方程式の BLT のブロックを解く順に出し、代数ループを目立たせる（CLI・UI・LSP・Python の `Workspace.blt`。式からソースへ飛ぶのは未）。`globe stages` が、モデルが変換の段（flat・DAE・インデックスの低減・BLT・tearing）ごとにどう変わるかを要約する（LSP・wasm・Python の `Workspace.stages` からも。ツールバーの「段」が UI のパネルを開く。flat から DAE は数だけ）。
- Icon と Diagram のビュー: ライブラリの木の各行にそのクラスのアイコン、右のパネルにカーソルのクラスの Icon と図（VS Code の側のパネルにも）が出る。VS Code の側のパネルでは、選んだ部品を矢印キー（2 単位、Shift で 10 単位）で動かし、Enter で部品のパラメータを QuickPick・InputBox で入力して宣言の修飾に書ける（ダブルクリックでも同じ。コマンド「GlobeMode: 部品のパラメータを編集」はカーソルの部品から始める）。結線の追加は UI と VS Code のパネルでキーボードからもできる（部品を選んで `c`、そのコネクタを Tab・矢印で選んで Enter で始点、別の部品のコネクタを選んで Enter で終点。Esc でやめる。選んだ結線の `+` で途中の点が 1 つ入る）が、途中の点の移動・削除はマウスだけ。UI と VS Code の側のパネルでは部品のドラッグで移動、コネクタから別のコネクタへのドラッグで `connect(...)` の追加ができ、UI ではライブラリの木の行を図へドロップする（か「図へ」ボタン）と部品を足せる。図で部品か結線を選んで Delete を押すと消せる（UI でも VS Code のパネルでも。部品は結線も一緒に消える。`b[2].p` のような添字つきの端は `b[1].p` と区別する。複数の名前の宣言は断る）。どれもエディタのソースへの編集で、元に戻せ、保存はしない。結線を選ぶと途中の点につまみが出て、ドラッグで動かし、線のダブルクリックで点を足せる（UI でも VS Code のパネルでも。両端の点は動かさず、`points` の無い結線には 3 点の `Line` を作る）。結線の点の削除・キーでの移動と、VS Code のパネルからの部品の追加は未（[ADR-0037](doc/adr/0037-icon-diagram-as-shape-data.md)）。
- 構文の誤りのあるファイルでの言語サーバ（自前の Track B のパーサを初めて製品につないだ）: Rumoca が読めないファイルでもアウトラインと折りたたみが出て、書きかけの文書でも `a.` の補完が出る（今の道で候補が出るときは変えない）。定義ジャンプとホバーも、Rumoca が読めない文書では自前の構文と名前解決で答える（同じ規則で、今の道に答えが無いときだけ）。
- inlay hint・call hierarchy・`redeclare` の後の補完（S345・S352・S354）: inlay hint は関数の呼び出しの位置引数の前に引数の名前を出す（`f(x, y)` は `f(a: x, b: y)`。名前は呼び先の `input` の並びを自前の名前解決で引いたもの。組み込み・部品・引けない名前・名前つき引数には出さない。書きかけの文書でも出る）。call hierarchy（`prepareCallHierarchy`・呼び元・呼び先）は Modelica の `function` だけで、呼び元は開いた文書・根の下の `.mo`・ライブラリから探し、呼び先は本体が呼ぶ関数（引けない名前・組み込み・レコードの構築子は出さない）。VS Code の拡張はどちらも自動で使い、`vscode.yml` が Linux・Windows・macOS の本物の VS Code で確かめる（画面の描画は確かめていない）。`a.` の補完・定義ジャンプ・ホバーは `redeclare` や `class extends` で差し替えた後の型も引く（S345。S351 で、差し替えていない同名の別の型の欄は出さない）。
- 整形の行コメント（S343・S346）: `annotation(…)`（ほかの括弧の引数も）の引数を並べ替えるとき、行コメントは引数と一緒に動く。引数と同じ行の末尾の `// …` も、次の引数の前の行にあるコメントも。lint の `annotation-order` も同じ。
- 自前の段での検査（試し。既定は Rumoca）: `globe check --frontend native` が、Rumoca の代わりに自前の Track B の段（parse → resolve → flat → dae）で検査する。S6 まで（構文・参照・方程式数と変数数・構造特異）で、S7 以降は見ない。モデルが使うライブラリのファイルの閉包だけを読み、釣り合い・構造特異の診断（`GM-E5002`・`GM-E6001`）には行・列が付く。自前の段が扱えないものは `GM-U0006`（終了コード 3）で止め、Rumoca には切り替えない。同じ選択を言語サーバ（`globe-mode.toml` の `[check] frontend = "native"`。出す診断が自前の段になる）と Python（`Workspace.check(model, frontend="native")`）でもできる。ブラウザ（wasm）の UI も、検査ボタンの横の選択（Rumoca / 自前（試験））で選べる（wasm は自前の段を入れて +0.35 MB、+1.8%）。Desktop（sidecar）の UI にも同じ選択がある（S349。LSP の `globe/check` が `params.frontend` を取る）。選択の既定は「設定に従う」（S350）で、`frontend` を渡さないので `globe-mode.toml` の `[check] frontend` が決める（エディタの波線と同じ。設定ファイルの無いブラウザでは Rumoca と同じ）。自前の平坦化は `redeclare`・`replaceable`・`class extends`・`Complex`・レコードの配列・束縛から大きさが決まる `[:]` の配列も扱う。MSL の例題 566 本のうち、名前解決は 566 本とも未解決の参照が 0（S341）、平らにできるのは 548 本（S355 で `stream`〔`inStream`・`actualStream`・外側のコネクタの端の方程式〕が、S361 で外側の階層の `inner` につなぐ `connect`・`inner outer`・`cardinality` の `if` が入った）で、548 本とも DAE になる（`mise run msl:diff-frontend` で Rumoca と突き合わせる。回帰 0）。Rumoca も通る例題（521 本。釣り合いまで通るのは 377 本）では、変数の名前が 518/521（残り 3 本は、`redeclare model` の修飾で条件を真にした条件つき宣言を Rumoca が評価しないため）、方程式の数が 508/521（残り 13 本は Rumoca の数え方の誤りとみている）、釣り合いの判定・方程式と未知数の数・構造の正則・状態の数が 377・376・377・376 / 377 で一致する（S335〜S337・S355・S361・S362・S364）。まだ断る 18 本の理由は `composite-binding` 6・`expandable` 5・`non-constant-dim` 4 ほか 3 本（[doc/frontend-diff.md](doc/frontend-diff.md)）。
- ブラウザと Android の MSL は塊ごとに読む: UI は `msl/index.json` を頼りに、開いたモデルに要る塊（`msl/c/*.json`）だけを取り、依存を 2 段先までまとめて先読みする。ChuaCircuit は初回の取得が 15.2 → 4.6 MiB、読み込み後の wasm のメモリが 62 → 14 MiB。配布物 `globe-web` と APK に `MSL.json` は入らない（APK は塊を 1 本のパック `msl/pack.bin` にまとめるので、大きさは `MSL.json` の時代の +1.7% に収まる。S340）。
- API: Rust（facade `globe-mode`）、Python（PyO3）、C ABI、Zig、Julia、Elixir、Gleam、WebAssembly。

できていないもの: FMI の読み込みのブラウザ・Android の UI と C・Elixir・Gleam の API（CLI・Rust・Python・Desktop は済）、SSP の残り（入れ子の System の書き出し・単位）、解析ヤコビ行列（行列は数値微分）、既定になる自前のコンパイラ（Track B。パーサと名前解決はアウトライン・折りたたみ・補完・定義ジャンプ・ホバー・inlay hint・call hierarchy にもつながり、平坦化〔flatten〕と DAE の構築 `globe-mode-dae` は試しの `--frontend native` の検査（S6 まで）だけにつながって、実行にはつながっていない。MSL の例題 566 本のうち 548 本を平らにでき、Rumoca も釣り合いまで通った 377 本では釣り合いの判定と構造の正則が 377 本すべて一致する。Rumoca と突き合わせる `mise run msl:diff-frontend` はある）、署名したインストーラと自動更新（鍵を登録するまで更新は未設定）、ブラウザ版の公開。順位つきの一覧は [doc/features.md](doc/features.md)、順序は [doc/roadmap.md](doc/roadmap.md)。

UI のキーボード（ブラウザ・Desktop・Android。macOS は Ctrl の代わりに Cmd。エディタの中でも入力欄でも効く）: Ctrl+Enter 実行（CodeMirror の「空行を挿入」は覆う）、Ctrl+Shift+Enter 検査、Ctrl+. 止める、Ctrl+S 保存（Desktop は Ctrl+Shift+S で名前を付けて保存）、Ctrl+O 開く。診断と参照の行は本物の button で、Tab で選んで Enter か Space を押すとエディタの該当位置へ移る。プロットの canvas には信号名と時間の範囲の代替テキストが付く。ページの先頭にスキップリンク（エディタへ・結果へ）があり、タブは左右の矢印・Home・End で移れ、未保存のタブには ● が付く。初めて開いたとき（保存したタブが無いとき）だけ、はじめの 3 手順の案内が出る（閉じられる）。

### 使い始め方

どれも、チェックアウト（`git clone https://github.com/asari300/20261018_GlobeMode`）と、[mise](https://mise.jdx.dev/) が固定した道具（`mise trust && mise install --locked`）から始める。道具の分担は [ADR-0003](doc/adr/0003-toolchain.md)、手順は [doc/mise-setup.html](doc/mise-setup.html)。

#### まず GUI を見る（クローン直後の最短手順）

```bash
git clone https://github.com/asari300/20261018_GlobeMode && cd 20261018_GlobeMode
mise trust && mise install --locked   # 道具（rust / node / pnpm / wasm-bindgen …）
cargo xtask fetch-msl                 # 任意。ライブラリツリーで MSL を読むなら先に（約 100 MB）
mise run ui:preview                   # wasm と UI をビルドして http://localhost:4173/ に配る（初回は数分。Ctrl+C で止める）
```

ブラウザで <http://localhost:4173/> を開く。初回の案内に沿って、左のタブの `Decay` を Ctrl+Enter で実行するとプロットが出る。Ctrl+O で自分の `.mo`（結果の `.mat`/`.csv` も）を開ける。計算はブラウザの Web Worker の wasm で行い、サーバは静的ファイルを配るだけ。WSL2 なら Windows のブラウザから同じ URL で開ける。

Desktop（Tauri）版を手元で動かすなら `src/Rust/apps/desktop` で `cargo tauri dev`（GTK などの依存と構成は [ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md)。CI の手順は `.github/workflows/desktop.yml`）。

#### MSL を取る

多くの例題は Modelica Standard Library（MSL）4.1.0 を読む。

```bash
cargo xtask fetch-msl            # data/msl/ に取得（sha256 を確かめて展開。コミットしない）
```

チェックアウトが無いときは、MSL の公式の release の zip を展開して、その場所を `-L` に渡せばよい（版 4.1.0。URL と sha256 は `cargo xtask fetch-msl` が持つ値と同じ。出典は [MSL の release](https://github.com/modelica/ModelicaStandardLibrary/releases/tag/v4.1.0)、値は `src/Rust/xtask/src/main.rs` の `MSL_URL`・`MSL_SHA256` から写した。2026-11-06）。

```bash
curl -fLO https://github.com/modelica/ModelicaStandardLibrary/releases/download/v4.1.0/ModelicaStandardLibrary_v4.1.0.zip
echo "5409cfa17797c52d866dc9d2675badc1378b4bbf88066e60899a489cb1cb8a2d  ModelicaStandardLibrary_v4.1.0.zip" | sha256sum -c -
unzip -q ModelicaStandardLibrary_v4.1.0.zip -d msl   # globe check -L msl -m Modelica.Blocks.Examples.PID_Controller
```

CLI・言語サーバには `-L data/msl` で渡す（サブフォルダではなく `data/msl` ごと）。ブラウザ・Desktop・Android の UI は「MSL を読む」ボタンから読む（ブラウザと Android は塊ごとに取る。S301・S308。Desktop は初回に取得する。[ADR-0030](doc/adr/0030-desktop-msl-placement.md)）。

#### CLI `globe`

```bash
cargo build --release -p globe-mode-cli    # target/release/globe（PATH に置く）

# 検査（S0〜S10）と実行。MSL の ChuaCircuit で検査に約 2 秒、実行に約 2 秒（静かな WSL2 での実測 1.6 s・2.1 s）
globe check    -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit
globe simulate -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit -o chua.csv   # .mat なら MAT v4
globe simulate --list-solvers
# 複数のソルバで回して比べる（最大 16 個。-o は DIR で DIR/<ソルバ>.csv。--compare 6 は矩形の管 10^-6 で判定。--max-order は cvode・ida だけ）
globe simulate -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit --solvers bdf,esdirk34,radau5 --parallel --compare 6 -o runs/
globe results omc_res.mat -o run.csv   # 結果の MAT v4 / CSV を読んで CSV か MAT v4（.mat）に直す。-o 無しは CSV を標準出力へ
# -o を省くと結果は書かず、要約だけ（stderr に「結果は書いていない」）。-o - は標準出力。info の診断は件数だけ（-v で全部）

# 自分のファイルやフォルダ（モデルの完全修飾名を -m に）
globe check src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.Decay
# 自前の段（parse → resolve → flat → dae）での検査。試し（S6 まで。既定は Rumoca）。扱えないものは GM-U0006（終了コード 3）
globe check src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.Decay --frontend native

globe fmt --check src/Modelica   # ファイルかフォルダを指す。整形が要るファイルを列挙して終了コード 1。--check を外すと書き換える（リポジトリの根の `.` は検証用の入力も含むので通らない）
globe fmt -               # `-` は標準入力 → 標準出力。パスも `-` も無ければ使い方を出して終了コード 2。ウェブアプリは `globe fmt --json -`
globe lint .              # line-too-long など。--disable <規則> で止める。`-` は標準入力
globe fmu export src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.Decay --out out/ [--fmi-version 3] [--runtime p5b]   # out/ に展開したフォルダと GlobeModeValidation_Simulation_Decay-fmi2.fmu（3.0 は -fmi3）。版つきなので 2.0 と 3.0 を同じ out/ に書ける
globe fmu export -L data/msl -m Modelica.Mechanics.Translational.Examples.Accelerate --out out/ --runtime p5b   # P5b（GlobeMode の実行時の共有ライブラリを入れた FMU。イベントのあるモデルも載る）out/…-p5b-fmi2.fmu。使う MSL のファイルだけを同梱。共有ライブラリは cargo build --release -p globe-mode-fmu か --runtime-lib <path>。host の OS 用
globe fmu run model.fmu --stop-time 1 --out out.csv   # FMU を別プロセスで 1 本実行（CS・ME）
globe fmu cosim spec.json -o out.csv   # 複数の CS の FMU をつなぐ。spec.json は {"fmus": {"a": "a.fmu", "b": "b.fmu"}, "connections": [["a.y", "b.u"]], "step": 0.1, "stop_time": 1, "algorithm": "gauss-seidel", "start": {"a.k": 2.0}}（algorithm・start は省略可）
globe fmu cosim system.ssp --step 0.1 [--stop-time 1] -o out.csv   # SSP 2.0.1 の最小版（.ssv の ParameterBindings も読む）
globe fmu ssp-export spec.json -o system.ssp   # 上の spec.json を System 1 つの .ssp に書く（結線は Real・Int32・Boolean。通信刻みと algorithm は独自の Annotation。読み戻すと同じ結果）
globe fmu ssp-export spec.json -o system.ssp --ssv   # start を Component ごとの .ssv に書く（既定はインライン）
globe fmu ssp-import system.ssp -o work/spec.json   # .ssp を spec.json に戻す（FMU は work/ に取り出す。同名のファイルは上書きしない）
globe linearize src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.LinearSystem [--time T] [--param NAME=VALUE] [--json]   # A・B・C・D（数値の中心差分）と、A の固有値・安定性
globe blt -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit [--json]   # BLT を解く順のブロックで。代数ループを見出しで示す
globe stages src/Modelica/GlobeModeValidation -m GlobeModeValidation.Index.Pendulum [--json]   # flat → DAE → インデックスの低減 → BLT → tearing の段ごとの要約
globe lsp -L data/msl     # 標準入出力の LSP サーバ
```

終了コードは次のとおり（`globe --help` の末尾にも出る。`| head` で読み手が先に閉じても変わらない）。

| コード | 意味 |
| --- | --- |
| 0 | 成功 |
| 1 | モデルの誤り（Error の診断）・整形が要る（`fmt --check`）・lint の指摘がある |
| 2 | 使い方の誤り・入力を読めない・結果を書けない（ディスク満杯を含む） |
| 3 | 未対応（GM-U）だけで止まった（`check`・`simulate`。モデルの誤りではなくコンパイラが未対応。Error と混ざっていれば 1） |

ほかに `globe outline`・`graph`・`flat`・`deps`。`globe <コマンド> --help` で全部の引数が出る。設定はカレントディレクトリから上へたどって最初の `globe-mode.toml`（[doc/format-style.md](doc/format-style.md)）。`cvode`・`ida` が要るときは cmake と C コンパイラを入れて `--features sundials` を付ける。

#### ブラウザ

wasm と UI をビルドして、静的ファイルとして配る。計算はブラウザの Web Worker で行い、サーバでは計算しない。

```bash
cargo xtask fetch-msl                      # 先に。「MSL を読む」用の msl/index.json と塊（分割配信。S301）と MSL.json が UI に入る（無くても動く）
mise run wasm:build                        # release の wasm（globe-mode-wasm）
cd src/Svelte
pnpm install --frozen-lockfile
pnpm build                                 # dist/
python3 -m http.server 8000 -d dist        # http://localhost:8000/ を開く（上の 3 段をまとめたのが mise run ui:preview）
```

開発用の `pnpm dev` と試験は [src/Svelte/README.md](src/Svelte/README.md)。ビルド済みの束は、手動のワークフロー `web.yml`（`gh workflow run web.yml`）が artifact `globe-web` に出す（`gh run download <run-id> -n globe-web -D globe-web`、中身は `dist/` と CSP の header の断片）。公開先（Pages など）はまだ無い。

#### Desktop（Tauri）

手動のワークフロー `desktop.yml`（`gh workflow run desktop.yml`。Linux・Windows・macOS で約 30 分）が、インストーラを artifact に出す。

| OS | artifact | 入れ方 |
| --- | --- | --- |
| Linux（Debian 系） | `desktop-linux`（.deb） | `sudo apt install ./<ファイル>.deb` |
| Windows 11 | `desktop-windows`（.msi） | ダブルクリック |
| macOS（Apple silicon） | `desktop-macos`（.dmg）、`desktop-macos-app`（.app の tar） | .dmg を開いて `/Applications` へ |

タグ `v*` を打つと `release.yml` が macOS（Apple silicon）の .dmg・`globe` の tar.gz・Python の wheel も作って Release に付ける（まだタグは打っていない。手動の実行は dry run で artifact に残すだけ。入力 `tag` を付けるとタグの経路も確かめられる。手順は [doc/release.md](doc/release.md)）。取得は `gh run download <run-id> -n <artifact>`。**署名も公証もしていない**ので、OS が警告を出す（Windows の SmartScreen、macOS の Gatekeeper。macOS は「システム設定」の「プライバシーとセキュリティ」で「このまま開く」を押す。Apple の案内 <https://support.apple.com/en-us/102445>、確認日 2026-11-05）。起動して「MSL を読む」を押すと MSL 4.1.0 を取得する。Desktop は検査と実行を sidecar の `globe lsp` に任せる（[ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md)）。作業ディレクトリの `globe-mode.toml` を整形と lint に使う。

#### Android

手動のワークフロー `android.yml`（`gh workflow run android.yml`。main への push でも、Desktop と UI の変更のときに動く）が、署名なしの debug APK（artifact `globe-android`、aarch64・x86_64 入りの universal）を出す。

```bash
gh run download <run-id> -n globe-android
adb install app-universal-debug.apk
```

Android は sidecar を使えず、計算は Web Worker 上の wasm（ブラウザと同じ）。MSL は APK に分割配信の形（`msl/index.json` と、塊を 1 本にした `msl/pack.bin`。`MSL.json` は入れない。S340）で同梱する。ABI ごとの release APK は artifact `globe-android-release`（署名は鍵の secret があるときだけ。無ければ署名なしで、そのままは入れられない）。実機での確認は、まだエミュレータ（API 34）の自動の操作だけ。

#### VS Code の拡張

`.mo` を開くと `globe lsp` につなぐ小さな拡張（整形・lint・補完・検査・定義ジャンプ・参照・名前の変更・inlay hint・call hierarchy・構文の色付け）。Marketplace には出していないので、`.vsix` を作って入れる。

```bash
cargo build --release -p globe-mode-cli       # globe（PATH に置くか、設定 globeMode.path に絶対パス）
cd src/VSCode/globe-mode
mise exec -- pnpm install --frozen-lockfile
mise exec -- pnpm package                      # globe-mode-<版>.vsix
```

VS Code の拡張機能ビュー（Ctrl+Shift+X）の「…」→「VSIX からのインストール...」で選ぶ。MSL は設定 `globeMode.libraries` に並べる。整形は Shift+Alt+F。詳細は [src/VSCode/globe-mode/README.md](src/VSCode/globe-mode/README.md)。

#### Python

```bash
uv sync          # チェックアウトの中。maturin が globe-mode-py をビルドして editable に入れる（Rust が要る）
```

```python
from globe_mode import Workspace, table

ws = Workspace(libraries=["data/msl"])  # cargo xtask fetch-msl で取得した MSL
print(ws.check("Modelica.Electrical.Analog.Examples.ChuaCircuit")["passed"])      # True
df = table(ws.simulate_columns("Modelica.Electrical.Analog.Examples.ChuaCircuit"))  # polars.DataFrame
print(df.shape, df.columns[:3])                                                    # (50219, 45) ['time', 'L.i', 'L.v']
```

`ws.linearize(model)`（状態空間の A・B・C・D。numpy があれば ndarray）、`cosim(spec)`（複数の FMU をつなぐ。dict・`.json`・`.ssp`。S258）、`run_fmu`・`fmu_info` も同じ流儀で使える（ワーカーの `globe` を PATH か `GLOBE_BIN` に）。
`ws.blt(model)` は BLT のブロックを解く順の dict のリストで返す（S262）。

PyPI には出していない。marimo・Jupyter の折れ線 widget は extra `widget`（`uv sync --extra widget`）。解析の機能（`linearize`・`blt`・`stages`・`cosim`・`check(frontend="native")`）を 1 本で試せる marimo の見本は `src/Python/marimo/analysis_app.py`。ノートブックは [src/Python/marimo/README.md](src/Python/marimo/README.md)、ダッシュボードは [src/Python/streamlit/README.md](src/Python/streamlit/README.md)。

#### C・Zig・Julia・Elixir・Gleam

どれも C ABI（`globe-mode-c`）か NIF（`globe-mode-nif`）の薄い束ねで、`GlobeModeValidation.Simulation.Decay` を開いて check → simulate → 列 → 整形 → FMU の書き出し（Elixir・Gleam は check → simulate まで）を通す参照シナリオがある。

| 言語 | 実行 | 説明 |
| --- | --- | --- |
| C / Zig | `mise run c:zig` | [src/Zig/README.md](src/Zig/README.md) |
| Julia | `mise run c:julia` | [src/Julia/README.md](src/Julia/README.md) |
| Elixir | `mise run elixir:test` | [src/Elixir/README.md](src/Elixir/README.md) |
| Gleam | `mise run gleam:test` | [src/Gleam/README.md](src/Gleam/README.md) |

Hex・Julia のパッケージ登録はしていない。

### 既知の限界

MSL 4.1.0 の例題 566 本のうち、S0〜S10 の検査を通るのは 369 本。通らないのは 13 本（モデルの誤りか構造特異）、コンパイラ側の限界で判定できないのは 184 本（`GM-U`。多くは Rumoca v0.10.2 の穴で、[doc/upstream.md](doc/upstream.md) に集めた）。検査を通った例題のうち実行の対象にした 356 本（partial などを除く）では、312 本が終了時刻まで実行でき、MAP-LIB の参照結果（MSL 4.1.0）と比べられた 311 本のうち 303 本（97.4%）が、参照結果の管（相対の幅 0.2%）に収まる。出典は [doc/msl-funnel.md](doc/msl-funnel.md) と [doc/msl-sim.md](doc/msl-sim.md)（2026-11-03 の実行）。MSL 以外の公開ライブラリ 7 つの結果は [doc/libraries.md](doc/libraries.md)。

そのほかの限界は次のとおり。

- `globe` を crates.io には出せない。Rumoca を git 依存で固定しているため（[ADR-0002](doc/adr/0002-rumoca-adapter.md)）。
- 参照・リネームの範囲は、UI（ブラウザ・Desktop・Android）では開いているタブの全部の中の部品（継承した先・`b.x` も）・クラス名・`import` の別名（S161・S166・S235・S368。ライブラリで同じクラスに行き着く参照は一覧に出すだけで変えない）。開いていないファイルと、ライブラリのファイルの書き換えは、UI からは未。言語サーバ（VS Code など）は S230・S235 で、開いていない根の下の `.mo`・ライブラリ・継承した部品・`b.x` にも届く（リネームでライブラリのファイルは変えない。`redeclare`・`replaceable` の先は未）。
- 補完は予約語と本文の名前に加え、型の位置・点つきの名前でライブラリのクラス、`.` の後ろで部品の名前（S171。エディタ（ブラウザ・Desktop）と言語サーバ）。
- ライブラリツリーの検索は名前の部分一致（`.` を含めると完全修飾名。S167）。
- Desktop の自動更新は、鍵を登録するまで「未設定」。
- Desktop（Windows・macOS・Linux）と Android は、署名と、実機での長い操作の確認が未（CI の自動の操作は成功している）。

### 開発に加わる

作業は git worktree の feature ブランチで行い、`main` を直接編集しない。規則と締めの手順は [CLAUDE.md](CLAUDE.md)（worktree、ライセンス、文書、ログ、検査）。

```bash
mise run check        # Rust の fmt / clippy / test / deny と uv sync --locked
mise run docs:check   # スライドとリンクの検査（Playwright）
mise run msl:diff-frontend   # Track B の自前の段（parse → resolve → flat → dae）を Rumoca と MSL の例題・GlobeModeValidation で突き合わせる（release で約 1.5 分、最大 RSS 1.1 GiB。報告は doc/frontend-diff.md。回帰があれば終了コード 1。CI には入れていない）
```

Track B の自前の段（`globe-mode-syntax`・`-resolve`・`-flat`・`-dae`）か Rumoca の rev を変えたら、`msl:diff-frontend` も通す。

全体像は [doc/architecture.md](doc/architecture.md)、設計判断は [doc/adr/](doc/adr/)、セッションごとの記録は [doc/sessions/](doc/sessions/README.md)。

### エディタ機能（整形・lint・LSP）

`.mo` は CLI（`globe fmt` / `globe lint` / `globe lsp`）、VS Code 拡張（[src/VSCode/globe-mode](src/VSCode/globe-mode)、Shift+Alt+F で整形）、ブラウザの UI と Web アプリ（[doc/web-apps.md](doc/web-apps.md)）から同じ規則で整形・検査できる。規則の詳細は [doc/format-style.md](doc/format-style.md)。

#### 処理の流れ

```mermaid
flowchart LR
  src[".mo の本文"] --> entry{"入口<br/>CLI / VS Code / ブラウザ / Web アプリ"}
  entry -->|整形| fmt["globe-mode-fmt<br/>字句 → 構造 → 80 桁の配置<br/>annotation の並べ替え・HTML の段付け"]
  fmt --> guard{"空白以外の<br/>トークンが不変？<br/>（HTML の文字列は空白以外の文字）"}
  guard -->|はい| out["整形した本文"]
  guard -->|いいえ| keep["元のまま + エラー"]
  entry -->|lint| lint["globe-mode-fmt の lint<br/>GM-L0001〜0004"]
  entry -->|lint / LSP| outline["Rumoca のアウトライン<br/>→ クラスの索引"]
  outline --> types["connect の型の突き合わせ<br/>GM-L0005"]
  outline --> graph["connect グラフ"]
  entry -->|検査| check["S0〜S10 の検査<br/>（globe check・LSP の診断）"]
  check --> sim["コンパイル → 実行"]
```

#### 今の規則

| 種類 | 規則 | 既定 | 切り方 |
| --- | --- | --- | --- |
| 整形 | インデント 2・行幅 80 桁。収まらない呼び出しと配列は開き括弧で改行 | オン | `--indent 4` |
| 整形 | 図形プリミティブの引数を並べ替える（位置・形状 → points → fill → line） | オン | — |
| 整形 | `annotation` を experiment → Diagram → Icon → Documentation、`Documentation` を header → info → revisions に並べ替える | オン | — |
| 整形 | 図形プリミティブに `origin = {0, 0}` を足す | オン | `--no-insert-origin` |
| 整形 | `within` の後・クラス頭の後・`end` の前に空行 | オン | — |
| 整形 | `equation` / `algorithm` の前後に空行 | オン | `--no-blank-around-sections`、VS Code の `globeMode.blankAroundSections` |
| 整形 | `Documentation` の HTML を改行とインデントで整える（空白だけを変える。lint はしない。S062） | オン | `--no-format-html`、VS Code の `globeMode.formatHtml` |
| 整形 | 演算子の両側に空白、カンマは後ろだけ、行末の空白を消す | オン | — |
| lint | `line-too-long`（GM-L0001）: 整形しても 80 桁を超える行 | warning | `globe lint --disable <規則>` |
| lint | `class-name-pascal-case`（GM-L0002）: クラス名が大文字で始まらない（function は除く） | warning | 同上 |
| lint | `missing-description`（GM-L0003）: クラスに説明の文字列が無い | warning | 同上 |
| lint | `annotation-order`（GM-L0004）: annotation の引数が上の順でない。整形で直る | 通知 | 同上 |
| lint | `connect-type-mismatch`（GM-L0005）: connect の両端のコネクタの型が合わない（S062） | warning | 同上 |
| 検査 | S0〜S10（構文、方程式数と変数数、構造特異、初期化、代数ループ、プリフライト） | error / warning | — |

#### 今後必要になりそうな機能

| 機能 | 今 | 理由 |
| --- | --- | --- |
| LSP がワークスペースのフォルダ全体を読む | 診断と connect の型の突き合わせは開いたファイルだけ（参照・リネーム・call hierarchy の呼び元は、S230・S354 から根の下の開いていない `.mo` も読む） | 開いていないファイルの型を connect の型の突き合わせで解決するため |
| MSL などライブラリの型での connect の型の突き合わせ | ワークスペースの型だけ | `--library` で読んだ型は索引に無く、突き合わせを省いている |
| connect の向きの検査（input 同士、output 同士） | 無い | 型が合っていてもつなげない組み合わせがある |
| 配列の大きさ・単位の食い違いの検査 | 無い | 型の検査の次の段 |
| 参照・リネームの範囲の拡張 | 定義ジャンプ・参照・リネーム・ホバーは LSP と UI（ブラウザ・Desktop・Android の F12・Shift+F12・F2）にある（S132・S138・S149・S161）。UI の範囲は開いているタブの全部（S368。クラスのリネームはそのクラスを定義したタブの中から、部品は継承した先と `b.x` まで、ライブラリは一覧に出すだけ）。LSP は S230・S235 で、開いていない根の下の `.mo`・ライブラリ・継承した部品・`b.x` まで | UI は開いているタブの本文が正で、ディスクの版と食い違うので、開いていないファイルは見ない。`redeclare`・`replaceable` の先は LSP も未 |
| `globe fmt --diff` と `--exclude` | 無い | ruff と同じ操作性にするため |
| 整形の設定ファイル | `globe-mode.toml`（S130）。CLI・LSP・VS Code・Desktop（S143）が読む。ブラウザの UI は読まず、既定（インデント 2 ほか）を使う | プロジェクトごとに規則を揃えるため |
| ブラウザの UI と Web アプリで空行・HTML の規則を切る口 | 既定のオンだけ | VS Code・CLI と揃えるため |
| `src/Modelica` の既存ファイルを今の規則で整形 | 済み（S189）。`globe fmt --check src/Modelica` を CI と `mise run check` で回す | 位置に依存する試験と snapshot を保つため |
| FMI の読み込み・書き出し | 書き出しは FMI 2.0・3.0 の ME・CS（S094・S098、事象の無いモデル）。読み込みは CLI の `globe fmu run`・Python・Desktop の UI（S204・S205・S210・S229）。コシミュレーションは `globe fmu cosim`（S249・S259）、SSP 2.0.1 は最小版の読み取り（S253・S263）と書き出し（S274）。3.0 の SE の書き出しは連続モデルだけ（S251） | [doc/roadmap.md](doc/roadmap.md) |

| 文書 | 内容 |
| --- | --- |
| [doc/features.md](doc/features.md) | OpenModelica と Rumoca を参考に整理した、必要な機能の一覧 |
| [doc/architecture.md](doc/architecture.md) | crate の構成、遅延評価、実行前のアサーション、ソルバ、FMI、connect グラフ |
| [doc/roadmap.md](doc/roadmap.md) | P0〜P8 と Track B（自前のコア）、各段階の完了条件 |
| [doc/licenses.md](doc/licenses.md) | ライセンス方針。依存の許可・条件付き・禁止 |
| [doc/upstream.md](doc/upstream.md) | 上流（Rumoca・diffsol）への issue の下書きの一覧、提出の順と手順 |
| [doc/adr/](doc/adr/) | 設計判断の記録 |
| [doc/research/](doc/research/) | 調査の記録（出典 URL と確認日つき） |
| [doc/sessions/](doc/sessions/) | セッションごとの HTML スライドと解説書 |
| [CLAUDE.md](CLAUDE.md) | このリポジトリでの作業ルール |

ライセンスは MIT OR Apache-2.0。依存を足すときは [doc/licenses.md](doc/licenses.md) の区分に従う。Rumoca（Apache-2.0）は `globe-mode-frontend` と `globe-mode-exec` からだけ使い、OpenModelica は読むだけ（`omc` を別プロセスで呼ぶ照合を除く）。MSL 4.1.0 は取得するだけでコミットしない。

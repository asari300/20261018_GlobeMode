# src/Svelte

Svelte 5 と TypeScript の UI（P4）。Tauri（デスクトップ、Android）とブラウザで共用する。

- 計算は `Backend` インターフェースを通して呼ぶ。実装は `TauriBackend`（ネイティブ）と `WorkerBackend`（Web Worker 上の wasm）
- 業務ロジックを置かない
- 使っている部品は次のとおり（`package.json`）。
  - エディタ: CodeMirror 6（`@codemirror/autocomplete`・`commands`・`language`・`lint`・`state`・`view`）
  - connect グラフ: sigma.js、graphology、ForceAtlas2
  - プロット: uPlot
- 計画で挙げた `@codemirror/lsp-client`、cosmos.gl、Svelte Flow は `package.json` に入っていない。
- 下の「使い方（S041）」と「実行とプロット（S042）」は、その時点の記述。いまの UI の機能は [README.md](../../README.md) と [doc/features.md](../../doc/features.md)。

## 使い方（S041）

ブラウザ UI の骨格。`Backend`（`src/lib/backend.ts`）、`WorkerBackend`（`src/lib/worker-backend.ts`。`globe-mode-wasm/web/client.js` の `GlobeWorker` を包む）、CodeMirror 6 のエディタ、診断パネル（段階 S0〜S10、重さの順。行を押すとエディタの選択が動く）だけ。Tauri、グラフ、プロットは未実装。

```bash
mise run wasm:build          # wasm と糊を target/wasm-pkg/web に作る
cd src/Svelte
pnpm install
pnpm dev                     # 糊を public/wasm/ にコピーして vite（http://localhost:5173）
mise run ui:check            # svelte-check と、Playwright の Chromium での試験（src/Svelte/tests/test_ui.py）
mise run ui:build            # dist/ に静的サイトを作る
```

## 使い心地（S228）

- **Esc で止める**: 実行中の Esc は Ctrl+. と同じ（入力欄・select・ダイアログの中では奪わない。補完の一覧が開いていれば CodeMirror が閉じるだけ）。
- **タブの並べ替え**: タブのドラッグ&ドロップ（HTML5 の `draggable`）と Ctrl+Shift+PageUp / PageDown（Cmd も）。順序は `globe-mode.tabs` に 300 ms のまとめ書きで残り、動かしたあとも開いているタブと `aria-selected` は同じ。ブラウザによっては Ctrl+Shift+PageUp / PageDown をブラウザのタブの移動に使うが、ページが先に受けて止める。
- **設定のモデルごとの保存**: `SimSettings.svelte` の欄（ソルバ・時刻・許容誤差・間隔・刻み幅・予算・パラメータ）は、モデル名ごとに localStorage の `globe-mode.sim`（モデル名 → 触った欄だけ。上限 50 モデル）へ残る。モデル名を変える・タブを切り替えると、そのモデルの保存に入れ替わる（保存が無ければ既定）。「設定を既定に戻す」でそのモデルの保存を消す。クラス名のリネーム（F2）でモデル名が変わると、新しい名前の保存（無ければ既定）に入れ替わる。
- **開始時刻・刻み幅・予算**: protocol の `settings.start_time` / `step` / `max_wall_seconds`。刻み幅は固定刻みのソルバ（`euler`・`heun`・`rk4`・`impeuler`・`trapezoid`）を選んだときだけ有効で、他のソルバでは送らない。GM-E10004（出力が 1000 万点を超えるなど）は欄の下の赤字（`data-refusal`）にも出る。

## 実行とプロット（S042）

「実行」ボタンが `Backend.simulate({ model })` を呼ぶ（ソルバは既定のまま）。`refused` と診断は診断パネルとその上の赤字に出る。結果の CSV（`SimulateResponse.csv`）は `src/lib/plot/csv.ts` が列に分け、`src/lib/plot/Plot.svelte` が uPlot（MIT）で時系列に描く。信号はチェックボックスで選ぶ（既定は最初の 3 本）。ズームは uPlot の既定（ドラッグで拡大、ダブルクリックで戻す）。2 軸（S128）は実装済み。凡例の凝った UI は未実装。

「開く」（Ctrl/Cmd+O）は `.mo` のほかに結果のファイル `.mat`（MAT v4）・`.csv` も受け付ける（S222）。`Backend.openFile` が `columns` を返し（ブラウザ・Android は wasm の `read_results`、Desktop は sidecar の `globe/readResults`）、App が「結果 <ファイル名>」のタブ（`Tab.results`）に開く。プロットとその CSV・PNG の書き出しは実行の結果と同じ部品で、エディタは隠れ、実行と保存はできない。結果のタブは、Desktop ではパス（`Tab.path`）を自動保存に乗せて再読み込みで `Backend.readResultsAt`（sidecar の `globe/readResults`）から読み直し、読めなければ閉じて理由を出す。ブラウザ・Android はパスが無いので再読み込みで閉じる（注に書く。S236）。MAT のパラメータは `ResultColumns.parameters`（`globe/readResults`・wasm の `read_results` の `parameters`）で、結果のタブに折りたたみの表を出す。Desktop の「最近使ったファイル」には `.mat`・`.csv`・`.fmu` も入り、選ぶと同じ道で開く（`read_recent`・`write_recent` は `.mo` だけのまま）。

`.fmu`（S229）は `lib/FmuForm.svelte` が警告と実行の設定（終了時刻・刻み・予算〔秒〕・種類 CS / ME。空の欄は FMU の既定）を出し、設定は FMU のパスごとに localStorage `globe-mode.fmu` に残す（S236。S229 の確認ダイアログの代わり。ME の積分器の選択は出さない）。`Backend.runFmu(path, request, onProgress)` の request が `globe/fmuRun` に乗る。

アイコン（S244）は同じ `modelicaDocs` が説明と一緒に `Backend.icon("", { file, line, character })` で引き（ブラウザ・Android は wasm の `icon`、Desktop は sidecar の `globe/icon`。Rust は `globe-mode-fmt` の `icon`）、`lib/IconView.svelte` が説明パネルの上の 120px の枠に SVG の要素として描く。Rust が返すのは図形の形のデータ（rect・ellipse・line・polygon・text・bitmap。座標は Modelica のまま、y は上向き）で、UI は y を反転して `viewBox` を extent に合わせ、線の太さは画面の px（`vector-effect`）、地は常に明るい（暗でも読める）。SVG の文字列は作らず `{@html}` も使わない（[ADR-0037](../../doc/adr/0037-icon-diagram-as-shape-data.md)）。試験は `test_ui.py` の `icon_view`。

図（S245）は同じ `modelicaDocs` が `Backend.diagram("", { file, line, character })` で引き（wasm の `diagram`・sidecar の `globe/diagram`。Rust は `globe-mode-fmt` の `diagram`）、`lib/DocsPanel.svelte` の「図」のタブの `lib/DiagramView.svelte` が読み取り専用の SVG に描く。部品は自分のクラスの Icon の図形を `<g transform="matrix(…)">`（Rust が返す行列を y 反転した座標に直しただけ）に置き、`connect` は `Line` の折れ線か部品の中心どうしの直線、部品名を下に小さく、クリックかEnterで宣言の行にカーソルを移す（`App.svelte` の `onJump`）。図形の描画は `lib/ShapeView.svelte`（Icon と共用）。試験は `test_ui.py` の `diagram_view`（自分のモデルと、MSL の ChuaCircuit の部品 7・connect 9）。

線形化（S260）はツールバーの「線形化」（`App.svelte` の `linearize` → `Backend.linearize` → LSP `globe/linearize`・wasm `linearize`）。結果は `lib/LinearizeView.svelte`（A・B・C・D の表と JSON の書き出し）。

パラメータのダイアログ（S248）は `lib/ParameterDialog.svelte`（`<dialog>`）。図の部品のダブルクリック・右クリック（`DiagramView` の `onParams` → `App.svelte` の `openParams` → `SimSettings`）か、実行の設定の「パラメータ…」ボタン（モデル全体）で開く。`Backend.parameters(model)`（wasm の `parameters`、sidecar の `globe/parameters`。Rust は flat モデルのパラメータ宣言）の表で、値を入れると `SimSettings` の `picked`（実行の設定の `parameters`、モデルごとの保存〔S228〕に乗る）に入る。Boolean はチェック（0/1）、Enumeration は選択（1 始まりの ordinal）、String と配列は変えられない。変えた値だけが入り、既定と同じ値に戻すか「既定に戻す」で消える。GM-E10006 の診断は名前で欄の近くに出す。試験は `test_ui.py` の `parameter_dialog`。

説明パネル（S232）は `lib/docs.ts` の `modelicaDocs`: 入力かカーソルの移動が 300 ms 止まったら、カーソルのあるいちばん内側のクラスの Documentation（`annotation(Documentation(info, revisions))` の生の HTML）を `Backend.documentation("", { file, line, character })` で引く（ブラウザ・Android は wasm の `documentation`、Desktop は sidecar の `globe/documentation`。Rust は `globe-mode-fmt` の `documentation`・`class_at`）。クラス名で引くこともでき（`documentation("Modelica.Blocks.Continuous.PID")`、ライブラリのクラス）、UI はライブラリの木・検索で開くと宣言の行にカーソルが置かれる道で引く。HTML は第三者のものなので信用せず、`lib/html-sanitize.ts`（DOMParser で読み、白リストのタグだけを素のデータの木に組み直す）を通して `lib/DocsPanel.svelte` が要素と文字列として描く（**`{@html}` に生の文字列を渡さない**）。白リストは p・br・b・i・em・strong・sub・sup・ul・ol・li・h1〜h6・table・thead・tbody・tr・td・th・pre・code・hr・blockquote・dl・dt・dd と、href が http・https・modelica の a（`target="_blank" rel="noopener noreferrer"`）。img は alt の文字に置き換え（`modelica://…/Resources/…` の画像は表示しない）、`script`・`style`・`iframe` などは中身ごと、知らないタグは外して中身だけ、属性は a の href 以外すべて捨てる。試験は `node --test tests/test_html_sanitize.mjs`（`test_ui.py` が回す）と `test_ui.py` の `docs_panel`。

折りたたみ（S225）は `lib/editor.ts` の `modelicaFolding`: 範囲は `Backend.foldingRanges(file)`（LSP の `foldingRange` と同じ Rust の関数。ブラウザ・Android は wasm の `folding_ranges`、Desktop は sidecar の `textDocument/foldingRange`）で、入力が 300 ms 止まったら本文を送って取り直し、`foldService` と `foldGutter` に渡すだけ。`end M;` などの行は畳んでも残る。

- CSP は `index.html` の meta で `connect-src 'self'` のまま。`'wasm-unsafe-eval'` は wasm の初期化に、`style-src 'unsafe-inline'` は Svelte と CodeMirror が注入する style に要る
- TypeScript は 6 系（svelte-check 4.7 が 7 を単独では受けない）
- 診断の `range` は UTF-8 のバイト位置。UI で UTF-16 の位置に直して選択する

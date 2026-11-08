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

「開く」（Ctrl/Cmd+O）は `.mo` のほかに結果のファイル `.mat`（MAT v4）・`.csv` も受け付ける（S222）。`Backend.openFile` が `columns` を返し（ブラウザ・Android は wasm の `read_results`、Desktop は sidecar の `globe/readResults`）、App が「結果 <ファイル名>」のタブ（`Tab.results`）に開く。プロットとその CSV・PNG の書き出しは実行の結果と同じ部品で、エディタは隠れ、実行と保存はできない。結果のタブは自動保存しない。

説明パネル（S232）は `lib/docs.ts` の `modelicaDocs`: 入力かカーソルの移動が 300 ms 止まったら、カーソルのあるいちばん内側のクラスの Documentation（`annotation(Documentation(info, revisions))` の生の HTML）を `Backend.documentation("", { file, line, character })` で引く（ブラウザ・Android は wasm の `documentation`、Desktop は sidecar の `globe/documentation`。Rust は `globe-mode-fmt` の `documentation`・`class_at`）。クラス名で引くこともでき（`documentation("Modelica.Blocks.Continuous.PID")`、ライブラリのクラス）、UI はライブラリの木・検索で開くと宣言の行にカーソルが置かれる道で引く。HTML は第三者のものなので信用せず、`lib/html-sanitize.ts`（DOMParser で読み、白リストのタグだけを素のデータの木に組み直す）を通して `lib/DocsPanel.svelte` が要素と文字列として描く（**`{@html}` に生の文字列を渡さない**）。白リストは p・br・b・i・em・strong・sub・sup・ul・ol・li・h1〜h6・table・thead・tbody・tr・td・th・pre・code・hr・blockquote・dl・dt・dd と、href が http・https・modelica の a（`target="_blank" rel="noopener noreferrer"`）。img は alt の文字に置き換え（`modelica://…/Resources/…` の画像は表示しない）、`script`・`style`・`iframe` などは中身ごと、知らないタグは外して中身だけ、属性は a の href 以外すべて捨てる。試験は `node --test tests/test_html_sanitize.mjs`（`test_ui.py` が回す）と `test_ui.py` の `docs_panel`。

折りたたみ（S225）は `lib/editor.ts` の `modelicaFolding`: 範囲は `Backend.foldingRanges(file)`（LSP の `foldingRange` と同じ Rust の関数。ブラウザ・Android は wasm の `folding_ranges`、Desktop は sidecar の `textDocument/foldingRange`）で、入力が 300 ms 止まったら本文を送って取り直し、`foldService` と `foldGutter` に渡すだけ。`end M;` などの行は畳んでも残る。

- CSP は `index.html` の meta で `connect-src 'self'` のまま。`'wasm-unsafe-eval'` は wasm の初期化に、`style-src 'unsafe-inline'` は Svelte と CodeMirror が注入する style に要る
- TypeScript は 6 系（svelte-check 4.7 が 7 を単独では受けない）
- 診断の `range` は UTF-8 のバイト位置。UI で UTF-16 の位置に直して選択する

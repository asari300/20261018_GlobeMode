# GlobeMode Modelica（VS Code 拡張）

S059 で作った。`.mo` を開くと `globe lsp` を起動してつなぐだけの小さな拡張で、整形・lint・補完・検査は Rust 側（`globe-mode-lsp`・`globe-mode-fmt`）が受け持つ。整形の規則は [doc/format-style.md](../../../doc/format-style.md)。

## できること

| 機能 | 操作 |
| --- | --- |
| 整形 | Shift+Alt+F（拡張が `.mo` に割り当てる。Windows の VS Code の標準と同じキーで、Linux でも同じ）。コマンドパレットの「ドキュメントのフォーマット」でもよい |
| 保存時の整形 | 設定 `globeMode.formatOnSave`（既定はオフ）。設定画面で「globeMode」と検索してチェックを入れる。再読み込みは要らない |
| 節の前後の空行 | 設定 `globeMode.blankAroundSections`（既定はオン）。`equation` / `algorithm` の前後に空行を置く。切ると書いた空行だけが残る。再読み込みは要らない（S061） |
| HTML の整形 | 設定 `globeMode.formatHtml`（既定はオン）。`Documentation` の `info` / `revisions` などの HTML を改行とインデントで整える。変えるのは空白だけで、lint はしない（S062） |
| annotation の並び | `experiment` → `Diagram` → `Icon` → `Documentation`（中は header → info → revisions）でなければ、通知（`GM-L0004`）が出る。整形で直る |
| lint と検査の診断 | 開いて編集すると波線が出る（`GM-L0001`〜`0003` と検査の `GM-E…`）。一覧は Ctrl+Shift+M |
| 構文の色付け | 予約語・組み込みの型・文字列・数値・コメントを TextMate 文法で色付ける（S163。意味に基づく色付けは未）。`/*` を打つと `*/` を補う |
| 補完 | Ctrl+Space、または `.` のあと |
| 参照・名前の変更 | クラス名の上で Shift+F12（参照）・F2（名前の変更）。開いている文書、ワークスペースの根の下の開いていない `.mo`（隠しフォルダは除く）、ライブラリ（`globe lsp -L`）に届く（S230）。名前の変更は開いていない `.mo` も 1 回の編集にまとめ（VS Code が開いて適用する）、**ライブラリのファイルは変えない**（参照には出し、「ライブラリの N 箇所は変えない」と知らせる）。ファイル名に対応する最上位のクラスと、識別子はあるのに同じクラスと解決できなかった開いていない `.mo`（`import` を通る参照など）は、そのファイルを挙げて断る。**S235**: 部品（宣言・使い方・継承した部品・`b.x` の `x`）の参照・名前の変更は、定義元に寄せて継承した先と `b.x` に広がる（定義元がライブラリなら参照だけ。`redeclare` の先は対象外）。`import D = A.B;` の別名 `D` の名前の変更はその文書の中だけ。`D.X` やワイルドカード（`import A.B.*;`）を通るクラスの参照も数え、`X` の名前の変更に入る |
| アウトライン・ホバー | エクスプローラーのアウトライン、クラス名にマウスを載せる |
| 折りたたみ | 行番号の脇の ▾。クラス、`equation` / `algorithm` / `public` / `protected` の節、複数行の `annotation(…)`、連続するコメントを畳める（S220。`globe lsp` の `foldingRange`。構文が壊れている間はクラスと節は出ない） |
| 字下げの規則 | `model` / `function` などと `equation`・`if … then`・`for … loop` の次の行で字下げし、`end`・`else`・`elseif`・節の見出しで戻す（S220。`language-configuration.json`。`//` の行で Enter しても字下げしない） |
| 側のパネル | エクスプローラーのビュー「GlobeMode」（S247）。Svelte の UI の説明パネルと同じ、上からアイコン・図・説明。カーソルのあるクラスについて、カーソルが 300 ms 止まったら `globe/icon`・`globe/diagram`・`globe/documentation` を引く。図の部品をクリック（Enter / Space）すると宣言の行へ飛ぶ。部品をドラッグすると `Placement` の `origin` を書き換え、コネクタから別のコネクタへドラッグすると `connect` を足す（S264。編集は文書に入るだけで保存しない。Ctrl+Z で戻る）。部品のダブルクリックでそのパラメータを QuickPick・InputBox で入力し、宣言の修飾 `(名前 = 値)` として書く（S306。空の入力で外す）。**キーボードだけでも操作できる**（S309。下の表）。`.mo` 以外のエディタでは更新しない |

### 側のパネルのキー操作（S309・S316）

図の部品・結線は Tab でフォーカスを移すと選ばれる（青い破線の枠か太線）。

| キー | 動き |
| --- | --- |
| 矢印キー | 選んだ部品を 2 単位動かす（Shift で 10 単位。1 回ごとに `Placement` を書き換える。Ctrl+Z で戻る） |
| Enter | 選んだ部品のパラメータを QuickPick・InputBox で入力する（部品以外は宣言の行へ飛ぶ） |
| Space | 宣言の行へ飛ぶ |
| Delete / Backspace | 選んだ部品・結線を消す |
| `c`（部品を選んで） | 結線を足す（S316）。その部品のコネクタを Tab・矢印（Shift+Tab で戻る）で選んで Enter で始点、続けて別の部品のコネクタを選んで Enter で終点（`connect` が入る）。Esc でやめる。選び中のコネクタは枠と図の下の文（`role="status"`）で分かる |
| `+`（結線を選んで） | いちばん長い線分の中点に途中の点を足す（点を動かすのはまだマウス） |

コマンドパレットの「GlobeMode: 部品のパラメータを編集」（`globeMode.editParams`）は、カーソルが部品の宣言の行にあればその部品、なければ部品を選ぶ QuickPick から始める。途中の点を動かす・消す操作は、まだマウスだけ。

拡張は `.mo` の既定を `editor.tabSize: 2`・80 桁のルーラーにする。インデント 4 にしたいときだけ、設定で `[modelica]` の `editor.tabSize` を 4 にする。

## 入れ方

```bash
cargo build --release -p globe-mode-cli          # target/release/globe
cd src/VSCode/globe-mode
mise exec -- pnpm install
mise exec -- pnpm package                        # globe-mode-<版>.vsix
```

できた VSIX を VS Code に入れる。**プロファイルを使っているときは、画面から入れる**（CLI の `code --install-extension` は既定のプロファイルに入れるので、別のプロファイルの窓には出てこない）。

1. 拡張機能ビュー（Ctrl+Shift+X）の右上の「…」→「VSIX からのインストール...」で `globe-mode-<版>.vsix` を選ぶ。
2. 「ウィンドウの再読み込み」。
3. `.mo` を開き、右下の言語が「Modelica」になっていることを確かめる。ほかの言語（Lua など）のときは、そこを押して Modelica を選ぶ。

入ったかどうかは、拡張機能ビューの検索欄に `@installed GlobeMode` と打つと見える。Remote - WSL の窓では「WSL: …」の欄に出る。

整形は独自の要求 `globe/format` で頼む（S061）。vscode-languageclient が `textDocument/formatting` の追加の設定を捨てるため。indent はプロジェクトの `globe-mode.toml` の `[fmt] indent` が勝つ（S214。無ければエディタのタブ幅）。

`globe` が PATH に無いときは、設定 `globeMode.path` に絶対パスを書く。MSL などのライブラリは `globeMode.libraries` に並べる（`globe lsp --library` に渡る）。どちらも変えたら「ウィンドウの再読み込み」。

WSL の中のファイルを Windows の VS Code で開くときは、Remote - WSL の窓で入れる。拡張は `extensionKind: workspace` なので、`globe` と同じ WSL の側で動く。

外すときは拡張機能ビューで「アンインストール」。

## 検査

```bash
GLOBE_BIN=$PWD/../../../target/debug/globe mise exec -- pnpm test
```

`globe lsp` を VS Code と同じ形（`file://` の URI、拡張の既定の tabSize）で起こし、整形・lint の診断・補完を 1 往復ずつ確かめる。VS Code の本体は起動しない。`test/panel.test.mjs`（S247）は側のパネルの無害化と SVG の組み立て、`test/diagram-edit.test.mjs`（S264）は図のドラッグとメッセージの検査を試す（`globe` は要らない）。

本物の VS Code の中での確かめ（S165）は Actions の `vscode.yml`（手動と、main への `src/VSCode/**` の push）で回す: `gh workflow run vscode.yml`。手元では `pnpm build && PATH=$PWD/../../../target/release:$PATH xvfb-run -a pnpm test:vscode`（VS Code を約 150 MB 取得して起動する）。

## 側のパネルの作り（S247）

引くのは拡張ホストの `panel.js`、描くのは webview の中の `webview.js`（`pnpm build` が `dist/webview.js` に束ねる）。

- **無害化と SVG は Svelte の UI と同じファイルを読む**（写しは作らない）。`webview.js` が `../../Svelte/src/lib/html-sanitize.ts`（説明の HTML を白リストの木にする）と `shape-svg.ts`（アイコンと図を SVG の要素の木にし、`toDom` で DOM にする）を import し、esbuild が束ねる。Svelte の側は `IconView.svelte`・`DiagramView.svelte` が `shape-svg.ts` を、`DocsPanel.svelte` が `html-sanitize.ts` を読む。`pnpm test` の `test/panel.test.mjs` も同じファイルを（node 24 が `.ts` をそのまま読む）直接 import して試す。
- **安全**: HTML の文字列を作らず、`innerHTML` を使わない（要素は `createElement` / `createElementNS`、文字は `textContent`・テキストノード）。CSP は `default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-…'`（外への通信なし、nonce の無いスクリプトなし）。リンクは `http`・`https`・`modelica` だけ。
- **図の編集（S264）**: ドラッグの処理は UI と同じ `diagram-drag.ts`（`DiagramView.svelte` から出した）を束ねる。webview は離したら `move` / `connect` を `postMessage` し、拡張（`panel.js`）が `edit.js` で型と値を検査して `globe/movePart`・`globe/addConnect` を呼び、返った TextEdit を `WorkspaceEdit` で文書に入れて `edited` を返す（失敗なら webview が見た目を戻す）。`file`・`class` は webview から受け取らず、最後に描かせた図のものを使う。入れたあとの図は文書変更の監視（300 ms）が引き直す。詳しくは [ADR-0037](../../../doc/adr/0037-icon-diagram-as-shape-data.md) の付記。
- **パラメータの入力（S306）**: 部品のダブルクリックで webview が `{type: "params", part}` だけを送る。拡張は `globe/parameters` から `部品.名前`（直下の 1 段、配列・String は除く）を QuickPick に出し、Real・Integer は InputBox（空で修飾を外す）、Boolean・列挙は QuickPick で値を受け、`globe/setModifier` が返した TextEdit を `WorkspaceEdit` で入れる（undo 1 回・保存しない）。値の字句の検査は Rust 側で、断られた理由は通知に出る。webview 内の表より小さい方として QuickPick・InputBox にした（[ADR-0037](../../../doc/adr/0037-icon-diagram-as-shape-data.md) の付記）。
- **図の削除（S293）**: 部品か結線（透明の太い線 `.hit`）をクリック・Tab で選び（青い枠・太線）、Delete / Backspace で `deletePart` / `deleteConnect` を `postMessage`。選びの処理は UI と同じ `diagram-select.ts`。拡張は同じく検査して `globe/deletePart`・`globe/deleteConnect` を呼び、返った複数の TextEdit を 1 つの `WorkspaceEdit` で入れる（undo 1 回）。部品のクリックで宣言へ飛ぶとき、フォーカスは webview に残す（エディタへ移ると Delete が文字を消す）。
- **結線の途中の点（S302）**: 結線を選ぶと途中の点に四角のつまみ（`rect.wp`）が出て、ドラッグで動かす（2 単位に吸着）。結線の線のダブルクリックでいちばん近い線分に点を足す。つまみと点の決め方は UI と同じ `diagram-waypoint.ts`、ドラッグは `diagram-drag.ts`。webview は `moveWaypoint` / `addWaypoint` を `postMessage` し、拡張は端・index（整数）・座標（有限で上限つき）・ends（2 点）を検査して `globe/moveWaypoint`・`globe/addWaypoint` を呼び、`WorkspaceEdit` で入れる（undo 可、保存しない。`file`・`class` は受け取らない）。端の点は動かせない。
- `activate` は `{ panel }` を返す。e2e が webview の描き終えた数（`panel.state`）と `panel.reveal(行)` を見るため（webview の DOM は拡張ホストから見えない）。

## 依存

| 名前 | 版 | ライセンス | 用途 | 確認日 |
| --- | --- | --- | --- | --- |
| vscode-languageclient（と vscode-jsonrpc ほか推移的に 5 つ） | 9.0.1 | MIT（minimatch・semver は ISC） | LSP のクライアント。`dist/extension.js` に束ねて同梱 | 2026-11-01 |
| esbuild | 0.25.12 | MIT | 束ねるだけ（開発依存） | 2026-11-01 |
| @vscode/test-electron、vscode-textmate、vscode-oniguruma | 3.1.0、9.3.2、2.0.1 | MIT | VS Code の起動試験と Oniguruma での文法の試験（開発依存。VSIX には入らない） | 2026-11-05 |

`@vscode/vsce` は使わない。ライセンスを確かめられない署名用のバイナリ `@vscode/vsce-sign` を引き込むため。VSIX は `pack_vsix.py`（Python の標準ライブラリだけ）で作る。

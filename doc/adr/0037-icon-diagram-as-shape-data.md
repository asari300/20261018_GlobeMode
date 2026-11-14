# ADR-0037 Icon / Diagram は形のデータ（JSON）で UI に渡し、UI が SVG の要素を組み立てる

- 状態: Accepted（2026-11-09、S244 の Icon、S245 の Diagram〔読み取り専用〕。編集は次）
- 関係: [ADR-0024](0024-own-formatter-and-lint.md)（字句解析だけで読む `globe-mode-fmt`）、S232 の Documentation ビュー（[解説書](../sessions/S232-docs-view/解説書.md)。同じ探し方と同じ口）、[features.md](../features.md) の P8

## 背景

P8 の Diagram / Icon ビューの最初の一切れとして、カーソルのクラス（ライブラリの木で選んだクラスも）の `annotation(Icon(coordinateSystem(extent), graphics))` を「説明」パネルの上に描く。Icon の中身（Rectangle・Ellipse・Line・Polygon・Text・Bitmap）は第三者のライブラリのファイルから来る。S232 の Documentation と同じく、**信用しない入力を文字列のまま DOM へ渡さない**。

## 決定

1. **Rust は形のデータを返す。SVG の文字列は作らない。** `globe-mode-fmt` の `icon(src, 名前, library)` が `Icon { class, extent, shapes }` を返し、LSP の `globe/icon`・wasm の `icon` が JSON にする。図形は `kind`（rect・ellipse・line・polygon・text・bitmap）と、評価済みの属性（`origin`・`rotation`・`extent`・`points`・`line`・`fill`〔`[r,g,b]` か null〕・`thickness`・`dash`・`radius`・`text`・`fontSize`・`align`・`arrows`）。座標は Modelica のまま（y は上向き）。UI（`lib/IconView.svelte`）が y を反転し、SVG の要素として描く（`{@html}` に文字列を渡さない）。Diagram の配置と connect の線も同じ形（図形のリスト）に足す。
2. **字句解析だけで読む。** Documentation と同じく Rumoca の構文解析を使わない。annotation の引数を小さな再帰下降（数・文字列・`{}`・名前・呼び出し）で読み、読めない値・要素・属性は飛ばして続ける（壊れた値でも空のアイコン。括弧が閉じない文書は文の切り出しで落ち、null）。式（`2 * 3`）は読めない値、`DynamicSelect(a, b)` は最初の引数（評価しない）。入れ子は 64 段で打ち切る。
3. **`extends` の基底クラスのアイコンを 1 段だけ重ねる**（基底→自分の順に描く。`coordinateSystem` は自分、無ければ基底）。名前は外側のクラスから順に試す相対名の解決（同じファイル→ライブラリ）。深さは `EXTENDS_DEPTH`（1）。Modelica.Blocks の部品は `SISO → BlockIcon` と 2 段のものが多く、枠の Rectangle が出ないものがある。上げるのは定数 1 つ。
4. **属性の扱い**: 塗りは `fillPattern` が None 以外なら Solid（グラデーションの塗りは無し）、`smooth = Bezier` は折れ線、`arrow` は三角か V 字、`visible = false` は飛ばす、Bitmap は枠とファイル名だけ、Text の `%name` は最後の名前・`%class` は完全修飾名、それ以外の `%k` を含む文字列は（評価できないので）飛ばす。
5. **口**: wasm `Workspace.icon(class, text, line, character)`、LSP `globe/icon`（`RPC_METHODS` 23 から 24）、`Backend.icon(cls, at?)`。引き方は `documentation` と同じ（クラス名か、行・列のいちばん内側のクラス。文書を先に、無ければライブラリ）。
6. **見え方**: 枠は高さ 120px で、地は常に明るい（Modelica の色は白地が前提。暗でも読める）。線の太さは画面の px（`vector-effect: non-scaling-stroke`）。

## 結果

- 良い: HTML の文字列を作らないので、第三者の入力で DOM が壊れない。UI に業務ロジックは無く（色と座標の反転だけ）、Leptos・Dioxus・VS Code も同じ JSON を描ける。
- 悪い: 1 段の `extends`、グラデーション、楕円の弧（startAngle）、`DynamicSelect` の評価、Bitmap の画像、`%k` の置き換えは出ない。Diagram（部品の配置と connect の線、編集）は次の WP。

## 付記（S245: Diagram ビュー、読み取り専用）

- **同じ形のデータで渡す。** `globe-mode-fmt` の `diagram(src, 名前, library)`（`icon.rs` の隣の `diagram.rs`。字句解析・`split`・`class_spans` だけ）が `Diagram { class, extent, parts, connections, shapes }` を返し、LSP `globe/diagram`（`RPC_METHODS` 24 から 25）・wasm `diagram`・`Backend.diagram(cls, at?)` が JSON にする。図形は Icon と同じ `IconShape`。UI は `lib/ShapeView.svelte`（S244 の描画を切り出した）を Icon と Diagram で共用する（S247 で下の付記のとおり `shape-svg.ts` + `SvgTree.svelte` に替えた）。
- **部品**は `annotation(Placement(transformation(extent, origin, rotation)))` を持つ宣言だけ（`visible = false`・Placement が無い・`extent` が読めないものは飛ばす。`extent` が無ければアイコンの枠のまま）。型は `import` の別名・`.*`・内側から外側へのスコープの順で引き、そのクラスの Icon を `icon_named`（`%name` を部品の名前にする）で重ねる（`extends` は Icon と同じ 1 段）。引けない型は空のアイコン（UI は破線の枠）。配列の部品（`R[3]`）は 1 つ。
- **変換**は Rust が行列 `[a, b, c, d, e, f]`（Modelica の座標、y 上向き）にして返す。`extent` の向きが逆なら反転（Modelica の `flipHorizontal` 相当）、`origin` を中心に `rotation`（度、反時計回り）。UI は y を反転した座標に直す（`matrix(a -b -c d e -f)`）だけで、`<g transform>` に置く。`bbox` は変換後の枠（部品名の位置と、`coordinateSystem` が無いときの `extent`）。
- **connect** は `Line(points)` があればその折れ線（色・太さ・パターンも）、無ければ 2 つの部品の中心を結ぶ直線。端が部品でない（モデル自身の connector）connect で `Line` も無ければ飛ばす。`for` の中の connect も 1 本として数える。
- **持ち越し**: `extends` で継いだ部品、`import A.{B, C}`、`DynamicSelect`、`iconTransformation`、反転した部品の文字（鏡像のまま）、編集（ドラッグ・接続）。

## 付記（S247: VS Code の側のパネルと、共有の置き場）

- **VS Code 拡張にも「アイコン / 図 / 説明」の側のパネル**（エクスプローラーのビュー `globeMode.panel`）を足した。引くのは `globe lsp` の `globe/icon`・`globe/diagram`・`globe/documentation`（LSP は変えない）で、拡張に業務ロジックは無い。描くのは webview の中。
- **共有の置き場は `src/Svelte/src/lib/` のまま、移さない。** 拡張の `webview.js` が `../../Svelte/src/lib/html-sanitize.ts` と `shape-svg.ts` を相対パスで import し、esbuild（`pnpm build`）が `dist/webview.js` に束ねる。VSIX に入るのは束ねた結果だけなので、配布物は `src/Svelte` に依存しない。どちらの側にも写しは無い。共通の置き場を新設して両側から import する案は、ファイルを動かすだけで得るものが無いので採らなかった。`vscode.yml` の paths に 2 つのファイルを足し、Svelte の側の変更でも拡張の試験が回る。
- **`shape-svg.ts`（新）**: S244・S245 の描画（旧 `ShapeView.svelte` と `DiagramView.svelte` の部品の組み立て）を、純粋な関数 `shapeNode` / `iconNodes` / `diagramNodes`（`SvgNode = {tag, attrs, text?, children?}` の木を返す）に切り出した。Svelte の側は `SvgTree.svelte`（`<svelte:element>` で木を要素として描く）、webview は `toDom`（`createElementNS` と `setAttribute`、文字は `textContent`）で DOM にする。見た目と DOM の形（`g[data-part]`・`[data-conn] polyline`・`g[data-part] > g` の `transform`）は変えない。部品のクリックは svg で一括して受け（`data-line` に宣言の行）、Svelte でも webview でも同じ。
- **無害化**: 説明の HTML は webview の中で `html-sanitize.ts` の `sanitize`（`DOMParser` で読む白リスト）を通し、返った木を `createElement` と文字として描く。拡張ホスト（node）には DOM が無いので、洗うのは webview の側。
- **CSP**: `default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-…'`。`innerHTML` は使わない（`test/panel.test.mjs` が両ファイルを grep する）。
- **確かめ**: `pnpm test` に無害化と SVG の組み立て（`panel.test.mjs`。node 24 が `.ts` を直接読む）、`e2e/suite.cjs` に「ビューが開き、ChuaCircuit の図に部品が 7 個」。webview の DOM は拡張ホストから見えないので、webview が描き終えた数を `postMessage` で返し、`activate` が返す `panel.state` を e2e が読む。
- **持ち越し**: 図の編集、画像（`modelica://…/Resources/…`）、テーマに合わせた地の色（地は Svelte の UI と同じく常に白）。

## 付記（S250: 部品の移動）

- **編集の最初の一切れとして、部品のドラッグ移動を入れた。** 書き換えは Rust（`globe-mode-fmt` の `move_part`）が決める。字句解析で宣言を見つけ、`Placement(transformation(origin = {x, y}))` の `{x, y}` を新しい値の `{x, y}` にする編集（バイト範囲と文字列）を返す。`origin` が無ければ `transformation(` の直後に `origin = {dx, dy}, ` を足す（`extent` は `origin` からの相対なので触らない。`rotation` があっても `origin` は図の座標の平行移動なので、そのまま足せる）。Placement の無い宣言・`origin` が数の並びでない宣言（式）は動かさない（null）。値は小数 3 桁に丸める。
- **LSP `globe/movePart`**（`{file, class, part, dx, dy}`、TextEdit の配列か null。`RPC_METHODS` は 27）と wasm の `move_part`（Worker の `move_part`、`WorkerBackend.movePart`、`TauriBackend.movePart`）で返す。ファイルは書かない。UI は返った編集をエディタに 1 回の `dispatch` で入れる（Ctrl+Z で戻せる。保存はユーザー）。
- **UI**: `DiagramView.svelte` の svg でポインタを受け、ドラッグ中は部品の `<g>` に `transform` だけを付けて動かし、離したら 2 単位の格子に吸着した (dx, dy)（Diagram の座標、y 上向き）を `onMove` に渡す。4 px 未満は移動と見ず従来のクリック（宣言へ飛ぶ）、移動の直後のクリックは飛ばさない。ダブルクリック・右クリック（S248 のパラメータ）とは衝突しない。編集が入って図が読み直されたら見た目の `transform` を捨てる。結線は `Line(points)` を持たなければ中心どうしの直線なので、読み直しで部品に追従する。`Line(points)` の点は動かさない。
- **持ち越し**: 結線の点の編集、部品・結線の追加、部品の回転・拡大縮小、VS Code の側のパネルでの移動（今回は触っていない）、`extends` で継いだ部品。

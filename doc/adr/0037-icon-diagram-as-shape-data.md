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

## 付記（S261: 結線の追加）

- **Diagram の編集の 2 つ目として、結線（`connect`）の追加を入れた。** コネクタから別のコネクタへドラッグして離すと、クラスに `connect(a.p, b.n) annotation(Line(points = {{x1, y1}, {x2, y2}}, color = {0, 0, 255}));` を足す編集を Rust（`globe-mode-fmt` の `add_connect`）が返す。最後の `equation` 節の末尾（クラスの `annotation` の前）にその節の字下げで足し、`equation` が無ければクラスの `annotation`（無ければ `end`）の行の前に `equation` 節ごと足す（`equation` は `end` の字下げ、文は最初の宣言の字下げ）。`initial equation`・`algorithm` の節は数えない。端は英数字・`_`・`.`・`[]` だけを受ける（ソースに入れる文字列なので）。挿入位置の行に他の文が詰めてあれば断る（null）。
- **コネクタの位置**: S245 の図は部品の中のコネクタを出していなかったので、`Part.connectors`（名前と図の座標）を足した。部品のクラスと `extends` の 2 段までで、`Placement` を持つ `parameter`・`constant` でない宣言の `iconTransformation`（無ければ `transformation`）の `extent` の中心を `origin` のまわりに `rotation` 回し、部品の変換で図の座標にする。基底クラスの探し方は Icon と共通（`icon.rs` の `find_base`）。部品のクラスの中の部品（`LimPID` の `Gain` など、Placement を持つ非コネクタ）を除くため、`iconTransformation` が無い宣言は型を引いて、`connector` 以外と分かれば除く（`is_connector`。字句解析せず `<種類> <名前>` の字面を探すだけ）。**型が引けなければ残す**ので、非コネクタが混ざることはある（結線できるかは検査が後で GM の誤りにする）。MSL で `CauerLowPassAnalog` の `Capacitor`（`p`・`n`）、`PID_Controller` の `LimPID`（`u_s`・`u_m`・`y`・`u_ff`）、`Inertia`（`flange_a`・`flange_b`）を確かめた。
- **口**: LSP `globe/addConnect`（`{file, class, from, to, points}`、TextEdit の配列か null。`RPC_METHODS` は 28）、wasm `add_connect`、`Backend.addConnect`。`globe/diagram` の部品に `connectors: [{name, pos}]` が増えた。
- **UI**: 各コネクタに透明な円（`.conn`、半径は図の大きさの 1/40 の 0.7 倍。ホバーで色が付く）をヒット領域として部品の `<g>` の中に置く。コネクタから始めたドラッグは部品の移動でなく結線で、仮の線を描き、別のコネクタの上で離したら確定する。同じコネクタ・コネクタ以外で離したら何もしない。点は両端のコネクタの位置の 2 点だけ。
- **型の互換は見ない。** 今回は端のコネクタの型（`Pin` と `RealInput` など）を確かめず、足したあとの検査（`globe check`・lint `connect-type-mismatch`）が誤りを出す。エディタで Ctrl+Z すれば戻る。
- **持ち越し**: 結線の途中の点（折れ線）、既存の結線との重複の検査、部品の追加、VS Code の側のパネルでの結線、配列のコネクタ（`p[1]`）の選択、型の互換の事前の検査。

## 付記（S265: 部品の追加）

- **Diagram の編集の 3 つ目として、部品の追加を入れた。** ライブラリの木（`LibraryTree.svelte`）の行を図へドラッグ＆ドロップすると、カーソルのクラスに `<型の完全名> <名前> annotation(Placement(transformation(origin = {x, y}, extent = {{-10, -10}, {10, 10}})));` の宣言を足す編集を Rust（`globe-mode-fmt` の `add_part`）が返し、UI がエディタに 1 回の `dispatch` で入れる（Ctrl+Z で戻せる。保存しない）。`origin` はドロップした図の座標を 2 単位に吸着（`move_part` と同じ格子）。
- **足す場所**: クラスの最後の public の宣言（`protected`・`equation`・`algorithm`・`initial`・クラスの `annotation`・`end` の手前まで）の次の行に、その宣言の字下げで足す。宣言が無ければ、最初の節かクラスの `annotation`（無ければ `end`）の行の前に、`end` の字下げ + 2 桁で足す。入れ子のクラスは自分の `end`。挿入位置の行に他の文が詰めてあれば断る（null）。結線の追加（S261）と同じ要素の切り出し（`own_elements`）を共有した。
- **名前**: 型の最後の部分の頭を小文字にしたもの（`Resistor` → `resistor`）。クラスの中の識別子の字句（宣言・`connect` の端・型名など）・キーワード・型の最後の部分そのものと重なれば `1`、`2`… を付ける（`resistor1`）。字句の一致を見るだけで、継いだクラスの名前は見ない（継いだ名前との重複は検査が後で誤りにする）。
- **型の種類**: `package`・`function`・`record`・`type` と分かれば断る（null）。`model`・`block`・`connector`・`class` は受ける。種類は `is_connector` と同じ字面の探し方（`kind_of`。ライブラリのファイルから `<種類> <名前>` を探す）で読み、**ライブラリで引けない・読めなければ受ける**。型が名前の並び（英数字・`_`・`.`）でなければ断る（ソースに入る文字列なので）。UI も木の行の種類が `package`・`function`・`record`・`type` で終わるものは `draggable` にしない。
- **口**: LSP `globe/addPart`（`{file, class, type, x, y}`、TextEdit の配列か null。`RPC_METHODS` は 29）、wasm `add_part`（Worker の `add_part`、`WorkerBackend.addPart`、`TauriBackend.addPart`）。
- **UI**: 木の行は `draggable` で `application/x-globe-class` に完全名を載せる。`DiagramView.svelte` の svg が `dragover`・`drop` を受け、ドロップ点を Diagram の座標（y 上向き）にして `onAdd` に渡す。キーボードの人のために、図が開いているとき木の各行に「図へ」ボタンを出し、図の `extent` の中心に足す。App の図の編集（移動・結線・追加）は `editDiagram` に共通化した。
- **持ち越し**: 名前を付ける入力（足したあとは検査後に F2 のリネームで変える）、足した部品の自動選択、パラメータの初期値の入力、VS Code の側のパネルでの追加（S264 の範囲）、型の互換・継いだクラスとの名前の重複の事前の検査、ブラウザでの操作の試験（`test_ui.py`）。

## 付記（S264: VS Code の側のパネルでの移動と結線）

- **S250 の部品の移動と S261 の結線を、VS Code の側のパネルでもできるようにした。** Rust・LSP は変えない（`globe/movePart`・`globe/addConnect` をそのまま呼ぶ）。
- **ドラッグの処理は共有する。** `DiagramView.svelte` に埋まっていた処理（2 単位の吸着、4 px 未満はクリック、ポインタの捕まえ先は部品の `<g>`、`.conn` から始めたら結線、仮の線、離したあとのクリックを飛ばさない）を `src/Svelte/src/lib/diagram-drag.ts` の `diagramDrag(svg, {onMove, onConnect})`（イベントの処理 `down` / `move` / `up` と `swallowed()` を返す）に出し、Svelte と webview の両方が読む（`shape-svg.ts` と同じ置き方。写しは無い）。値は渡された svg の API だけを呼び（`DOMPoint`・グローバルの `document` を使わない）、node の試験に偽の svg を渡せる。`DiagramView` の見た目と挙動は変えない（`svelte-check` 0 件）。
- **編集の流れ**: webview はドラッグ中は `transform` だけ動かし、離したら `{type: "move", id, part, dx, dy}`・`{type: "connect", id, from, to, points}` を拡張へ `postMessage` し、`{type: "edited", id, ok}` を待つ（10 秒で失敗扱い。失敗なら見た目を戻す）。拡張（`panel.js`）は `edit.js` の `parseEdit` で検査し（id は整数、名前は `\w+`、`from`・`to` は `名前.名前`、数は有限で絶対値 1e6 以下、点は 64 個まで。通らなければ捨てる）、LSP を呼び、返った TextEdit を `client.protocol2CodeConverter.asTextEdits` で変えて `WorkspaceEdit` で入れる（undo 1 回。保存しない）。**`file` と `class` は webview の言い分を使わず**、拡張が最後に webview へ描かせた図（`panel.target`）のものを使う。LSP の応答を待つあいだに文書の版が変わったら入れない。
- **適用後の図の更新は、既存の文書変更の監視（`onDidChangeTextDocument` → 300 ms 後の `refresh`）で足りる。** 引き直した図が `show` で DOM ごと作り直されるので、見た目の `transform` も消える。
- **CSP・DOM**: 変えていない（`default-src 'none'`、script は nonce、`innerHTML` 不使用）。
- **確かめ**: `pnpm test` に `diagram-edit.test.mjs`（偽の svg での吸着・クリック・結線・戻し、メッセージの検査）、`e2e/suite.cjs` に「`panel.handle` へ move・connect を渡すと文書が書き換わり、undo で戻り、図が引き直される」。webview の DOM は拡張ホストから操作できないので、e2e はドラッグそのものでなく webview が送るメッセージから先を回す。
- **持ち越し**: 実機の VS Code でのポインタ操作の目視、結線の途中の点、部品の追加、型の互換の事前の検査。

## 付記（S281: 部品と結線の削除）

- **Diagram の編集の 4 つ目として、部品と結線の削除を入れた。** Rust（`globe-mode-fmt` の `delete_part`・`delete_connect`）が消す範囲の編集（バイト範囲と空の文字列。重ならず先頭から並ぶ複数）を返し、UI がエディタに 1 回の `dispatch` で入れる（Ctrl+Z で戻せる。保存しない）。
- **`delete_part(src, class, part)`**: `part` の宣言（S250 と同じく Placement を持つ宣言）と、クラス直下の `connect(...)` のうち端の最初の名前（`b[1].n` の `b`）が `part` のものを、まとめて消す。`if` の中の `connect` は 1 つの文として扱われるので数えない。宣言が `A a, b;` のように深さ 0 の `,` を含む（複数の名前）ときは**断る**（null）。名前だけ外す編集は、宣言ごとの `annotation` の帰属が曖昧なので入れない。
- **`delete_connect(src, class, a, b)`**: `connect(a, b)` を消す。端は `globe/diagram` の `connections` の `from`・`to` と同じ字句（添字を含む。S283 で直した。`b[2].p` と `b[ 2 ].p` は同じ、`b[1].p` は別）、逆の並びも同じ結線とみなす。同じ端の `connect` が複数あれば全部消す。
- **範囲**: 文（同じ行の後ろのコメントを含む要素）が行に 1 つだけなら、字下げと行末の改行ごと消す。同じ行に他の文があれば、その要素と後ろの空白だけ。`equation` 節が空になっても節の見出しは残す（空の節は有効）。BOM は S250 と同じく数えて足す。
- **口**: LSP `globe/deletePart`（`{file, class, part}`）・`globe/deleteConnect`（`{file, class, from, to}`）、TextEdit の配列か null（`RPC_METHODS` は 33）。wasm `delete_part`・`delete_connect`、`Backend.deletePart`・`deleteConnect`（`WorkerBackend`・`TauriBackend`）。
- **UI**: `DiagramView.svelte` で部品（`[data-part]`）か結線を選ぶ。クリックと Tab（フォーカス）で選び、何も無い所のクリックで外す。選んだ部品は青い破線の枠と薄い地、結線は青い太線（`.sel`）。結線の当たり判定は、線と同じ点の透明な太い線（`stroke-width` 10 の非拡大の `polyline.hit`、`data-hit`、`tabindex 0`、`role=button`）を `[data-conn]` の外に置いた（図形の数を変えない）。Delete / Backspace は svg の中でだけ受け（`onkeydown`）、`onDelete` を呼ぶ。部品のクリックは従来どおり宣言の行へ飛ぶが、エディタへ移ったフォーカスを選んだ部品か結線へ戻す（さもないと Delete がエディタの文字を消す）。編集で消えた対象の選びは図の読み直しで外れる。
- **確かめ**: Rust は `delete_part_removes_the_declaration_and_its_connects`（部品とつながる結線をまとめて・配列・結線だけ〔逆の並び〕・字下げ・同じ行に他の文・複数の名前・BOM）、LSP 1 本、wasm 1 件、`pnpm check` 0 件、`test_tauri.mjs` の rpc 一覧。**ブラウザでの操作（`test_ui.py`）は書いていない**。
- **S283 で直した（配列の添字）**: 当初は `connect_ends` が添字を落とし、`b[2].p` を消すと `b[1].p` の結線も消えた。`connect_ends` が `[…]` の字句を端に含める（空白を除き、`b[i + 1].p` は `b[i+1].p`）ようにして、`delete_connect`・`add_connect` の突き合わせと `globe/diagram` の `connections` の `from`・`to`（UI の `data-conn`・`deleteConnect` へ渡す端）が添字つきになった。`delete_part` は端の最初の名前（`first`）で数えるので、添字があっても部品 `b` の結線は全部消える（変わらない）。試験は `delete_connect_tells_array_indices_apart`（Rust）と LSP `delete_connect_removes_only_the_matching_array_index`。
- **持ち越し**: ~~VS Code の側のパネルでの削除~~（S293 で済）、複数選び、宣言の複数の名前から 1 つだけ外す、消した部品を参照する他の文（方程式の `a.x` など）の整理、ブラウザでの操作の試験、削除の確認の問い。
- **S294: Diagram の編集の 5 つ目として、部品のパラメータの修飾を書く。** パラメータダイアログ（S248）の値は実行時の上書きだけだったので、同じ値を部品の宣言の修飾 `(k = 2)` としてソースへ書けるようにした。実行時の上書き（実行の設定の `parameters`。ソースを変えず、モデルごとに localStorage へ残る）はそのまま残し、ダイアログの見出しに 2 つの違いを一言書く。
- **`set_modifier(src, class, part, name, value)`**（`globe-mode-fmt`）: `part` の宣言（Placement が無くてもよい。`A a, b;` は名前ごとの区切り）の名前の直後（配列の添字の後）に `(name = value)` を足す。`(…)` があれば末尾に `, name = value` を足し、`name` の引数があれば `=` の右の式だけ置き換える（`R(min = 0) = 1` の `(min = 0)`、式の後ろの説明の文字列は残す）。`value` が空なら外す（唯一の引数なら括弧ごと、先頭・末尾なら区切りごと、自分の修飾を持つ引数は ` = 式` だけ）。何も変わらなければ空の範囲の編集。返すのは `Result<(バイト範囲, 文字列), 理由>`。
- **断る（理由つき）**: 入れ子の名前（`a.b`。部品の直下の 1 段だけ）、`each`・`final` つきの引数、`name.x = …` の入れ子の引数が既にある、式が壊れている、クラス・部品が無い。式の検査は字句と括弧の対応（コメント・`;`・`=`・深さ 0 の `,` が無い）だけで、型や名前は見ない（`globe-mode-fmt` は依存 0 で、構文の木〔`globe-mode-syntax`〕を持たない。式の中身は実行・検査が診断する）。`redeclare`・配列の部品の要素ごとの修飾・モデル自身のパラメータの束縛の書き換えは今回は書かない。
- **口**: LSP `globe/setModifier`（`{file, class, modifiers: [{part, name, value}]}`、`RPC_METHODS` は 36）。複数を順に本文へ当て、最初と最後の本文の差を 1 つの TextEdit にする（同じ部品に複数を足しても `(a = 1)(b = 2)` のように壊れない）。断る理由は -32803 の `message`（`globe/rename` と同じ）。wasm `set_modifier`（断れば例外）、`Backend.setModifier`。
- **UI**: `ParameterDialog.svelte` に「ソースに書く（N 本）」。実行時の上書きに入れた値のうち `部品.名前`（`c.k`）の形で数に読めるもの（Boolean は `true`・`false`）を `App.svelte` の `writeModifiers` に渡し、エディタへ 1 回の `dispatch`（Ctrl+Z で戻せる。保存しない）。書いた値は実行時の上書きから外す。列挙・モデル自身・入れ子・配列の行は対象外。断られたらダイアログに理由を出す。
- **確かめ**: Rust は `set_modifier_adds_replaces_and_removes`（足す・添字・複数の名前・書き換え・説明の文字列・外す〔唯一・先頭・末尾・自分の修飾〕・断る 9 通りと BOM）、LSP 1 本（複数を 1 つの編集に・変わらなければ空・-32803）、wasm 1 件と `smoke.mjs`、`pnpm check` 0 件、`test_tauri.mjs` の rpc 一覧。**ブラウザでの操作の試験（`test_ui.py`）は足していない**（wasm のビルドを回していない）。
- **持ち越し**: 修飾を外す UI（Rust の `value` 空は使えるが、ダイアログに口が無い）、入れ子の名前・`each`・`final`・`redeclare`、配列の部品、列挙（型の名前をダイアログが持たない）、モデル自身のパラメータの既定値の書き換え、書いたあとの既定値の列の更新。（S298 で、修飾を外す UI・列挙・既定値の列の更新は済）
- **S298: S294 の持ち越しのうち 3 つを済ませた。** (1) 行の「修飾を外す」（`PART_NAME` の行）が空の `value` で `set_modifier` を呼ぶ。外した行は実行時の上書きからも外す。修飾が無い行では「ソースは変わらなかった」と出る。(2) 列挙: `ModelParameter` に `enum_type`（型の完全修飾名）を足し（LSP・wasm の JSON にも出る）、ダイアログは選んだ ordinal をリテラル名にして `型.リテラル`（識別子でなければ `'x y'`）として書く。Rumoca の flat の `variable_type_names` は書いた通りの名前（`Speed`）で修飾が無いので、`enum_literal_ordinals` の別名のうち ordinal 1 のリテラルで `…<型>.<リテラル>` の形をしたいちばん長いものから修飾名を取る（`EnumType.qualified`。FMI の `declaredType` が使う `name` は変えない）。同じ末尾の名前の型が複数あれば先に見つかったものを使う（取り違えれば書いたあとの検査が診断する）。(3) 書いたあと `App.writeModifiers` が書いた本文を `setFile` で Workspace に入れ直し、ダイアログが `parameters` を引き直して既定値の列を再検査を待たずに更新する。確かめは `parameters` の試験に `enum_type`、`test_ui.py` に Real と列挙の書く・外すの往復（既定値の列が 1 → 3 → 1）。

## 付記（S293: VS Code の側のパネルでの削除）

- **S281 の部品・結線の削除を、VS Code の側のパネルでもできるようにした。** Rust・LSP は変えない（`globe/deletePart`・`globe/deleteConnect` をそのまま呼ぶ）。
- **選びの処理は共有する。** `DiagramView.svelte` に埋まっていた選び（クリック・フォーカスで部品か `.hit` を選び、何も無い所のクリックで外す、`.sel` を付け直し、図が変わって消えた対象を外す、Delete / Backspace の対象）を `src/Svelte/src/lib/diagram-select.ts` の `pickSel`・`markSel`・`deleteOf` に出し、Svelte と webview の両方が読む（`diagram-drag.ts` と同じ置き方）。Svelte の見た目と挙動は変えない。
- **webview**: 図（`#diagram`）の `click`・`focusin` で選び、`keydown` の Delete / Backspace で `{type: "deletePart", id, part}`・`{type: "deleteConnect", id, from, to}` を `postMessage`（S264 の `ask` と同じ。`edited` を待つ）。図は `show` のたびに作り直すので、`show` の最後に選びの印を付け直す。選んだ部品は青い破線の枠、結線は青い太線（CSS は Svelte と同じ）。
- **拡張**: `edit.js` の `parseEdit` が 2 種を足した（id は整数、`part` は `\w+`、結線の端は `名前` か `名前[添字]` を `.` でつないだ 2 つ。添字の字は `\w , + * -` だけ。`deleteConnect` は文字列を比べるだけでソースに書かないので `b[i+1].p` も通し、`file`・`class` は受け取らない）。`panel.js` は S264 と同じ `apply`（LSP の応答を待つあいだに文書の版が変わったら入れない。複数の TextEdit を 1 つの `WorkspaceEdit` で入れて undo 1 回、保存しない）に流す。LSP が null（消せない。複数の名前の宣言など）なら何もせず、webview へ `ok: false`。
- **フォーカス**: 部品のクリックで宣言の行へ飛ぶ `reveal` を `preserveFocus: true` にした。エディタへフォーカスが移ると、続く Delete がエディタの文字を消すため（Svelte の `DiagramView` が飛んだあとに選びへフォーカスを戻すのと同じ理由）。
- **CSP・DOM**: 変えていない（`default-src 'none'`、script は nonce、`innerHTML` 不使用）。
- **確かめ**: `pnpm test` に、メッセージの検査（添字つきの端・余計な字・`file`/`class`）、偽の要素での選び・印・削除の対象。`e2e/suite.cjs` に「`panel.handle` へ connect → deleteConnect（逆順の端）→ undo → deletePart を渡すと、宣言と `connect(` がまとめて消え、undo で戻る」。webview の DOM は拡張ホストから操作できないので、e2e は webview が送るメッセージから先を回す（S264 と同じ）。`pnpm check`（svelte-check）0 件。
- **持ち越し**: 実機の VS Code でのクリック・Tab・Delete の目視、複数選び、削除の確認の問い。

## 付記（S299: 結線の途中の点）

- **Diagram の編集の 5 つ目として、結線の途中の点の移動と追加を入れた。** Rust（`globe-mode-fmt` の `move_waypoint`・`add_waypoint`）が `connect(a, b) annotation(Line(points = {…}))` の `points` の中の 1 点の範囲だけを書き換える編集を返し、UI がエディタに 1 回の `dispatch` で入れる（undo 可、保存しない）。他の点・色・空白は触らない。結線は S281 と同じく端を添字を含めて比べ、逆の並びも同じ。同じ端の `connect` が複数あれば最初の 1 本だけ。座標は S288 の共有の入口 `coord`（有限で絶対値 1e6 以下）を通る。
- **`move_waypoint(src, class, a, b, index, dx, dy)`**: `Line(points)` の `index` 番目（0 始まり）の `{x, y}` を (dx, dy) だけ動かす。**最初と最後の点は部品のコネクタに付くので動かさず、断る**（None）。`points` が無い・点の数が 3 未満・範囲外・結線が無いときも None。
- **`add_waypoint(src, class, a, b, index, at, ends)`**: `points` が**あれば** `index`（1 以上、点の数未満＝最後の点の手前まで）の点の前に `{x, y}, ` を挿す。**無ければ**（中心どうしの直線として描かれている結線）、`ends`（始めと終わり。UI は両端のコネクタの位置を渡す）と `at` の 3 点で `Line(points = …)` を作る。置き場所は `Line(` の直後、無ければ `annotation(` の直後に `Line(points = …)`、`annotation` が無ければ文の末尾（`;` の前）に ` annotation(Line(points = …, color = {0, 0, 255}))`（`add_connect` と同じ色）。`points` が無いのに `ends` が無ければ断る。`points` を持つ `Line` の `points` が読めない（点でない要素）ときも断る。
- **なぜ `ends` を呼び出し側から受けるか**: 中心どうしの直線は、図に描くための仮の線で、Modelica の `Line(points)` の端はコネクタの位置と決まっている（OMEdit などが同じ前提で読む）。コネクタの位置は `globe/diagram` の部品の `connectors` に出ているので、UI がそれを渡す。Rust 側でライブラリを引き直して計算すると、wasm の `add_waypoint` にもライブラリの引数が要る。UI の値は座標の検査（`coord`）を通るだけで、他は信用しない。
- **口**: LSP `globe/moveWaypoint`（`{file, class, from, to, index, dx, dy}`）・`globe/addWaypoint`（`{file, class, from, to, index, x, y, ends}`）、TextEdit の配列か null。`RPC_METHODS` は +2。wasm `move_waypoint`・`add_waypoint`（`ends` は JSON の文字列）、`Backend.moveWaypoint`・`addWaypoint`（`WorkerBackend`・`TauriBackend`）。
- **UI**: `DiagramView.svelte` で結線を選ぶ（S281）と、`Line(points)` の途中の点（最初と最後を除く）に小さな四角のつまみ（`rect.wp`、`data-wp` に結線、`data-index` に番号）が出る。ドラッグは `diagram-drag.ts` の `onWaypoint`（部品と同じく 4 px 未満はクリック、2 単位に吸着、離したら呼ぶ。失敗なら見た目を戻す）。結線の線の上（`polyline.hit`）のダブルクリックは、クリックの点にいちばん近い線分を探し、その次の番号で `onAddWaypoint` を呼ぶ（座標は 2 単位に吸着）。右クリックでは足さない。つまみは Svelte の側だけが描き、`shape-svg.ts` の `diagramNodes`（VS Code と共有）は変えない。
- **確かめ**: Rust は `waypoints_move_add_and_refuse_the_ends`（動かす・逆の並び・端と範囲外と `points` の無い結線と不正な座標を断る・`points` への挿入・`points` の無い結線の 3 パターン〔`annotation` なし・`Line` に `points` なし・`Line` なし〕・足した結果を `diagram` が読み直す）、LSP 1 本、wasm 1 件（`decay.rs` と `smoke.mjs`）、`diagram-edit.test.mjs` に `.wp` のドラッグ、`pnpm check` 0 件、`test_tauri.mjs` の rpc 呼び出し。**ブラウザでの操作（`test_ui.py`）と、VS Code のパネルは入れていない**。
- **持ち越し**: VS Code の側のパネルでの途中の点、点の削除、ドラッグ中の線の追従（今は離したときに線が動く）、ブラウザでの操作の試験、折れ線でない `Line`（`smooth`）の扱い。

## 付記（S302: VS Code の側のパネルでの結線の途中の点）

- **S299 の途中の点の移動と追加を、VS Code の側のパネルでもできるようにした。** Rust・LSP は変えない（`globe/moveWaypoint`・`globe/addWaypoint` をそのまま呼ぶ）。操作は Svelte 版と同じ（結線を選ぶと途中の点に四角のつまみ、ドラッグで動かす。線のダブルクリックで点を足す）。
- **共有**: `DiagramView.svelte` に埋まっていたつまみの決め方（`handlesOf`・`halfOf`）、ダブルクリックで足す点の決め方（いちばん近い線分の次の番号・2 単位に吸着・両端のコネクタの位置 `ends`。`addWaypointOf`）、つまみの要素の木（`handleNode`。webview が `toDom` で DOM にする）を `src/Svelte/src/lib/diagram-waypoint.ts` に出した。ドラッグは S299 の `diagram-drag.ts` の `onWaypoint` をそのまま使う。Svelte の見た目と挙動は変えない（`.wp` の rect は Svelte が自分のテンプレートで描く）。
- **webview**: 図の `show` のあと、選びが結線ならつまみを `svg` の最後に足す（選びが変わったときだけ作り直す。つまみのクリックでは作り直さない）。`onWaypoint` は `{type: "moveWaypoint", id, from, to, index, dx, dy}`、ダブルクリックは `{type: "addWaypoint", id, from, to, index, x, y, ends}` を `postMessage`（S264 の `ask` と同じ。`edited` を待ち、失敗なら見た目を戻す）。CSP（`default-src 'none'`、nonce のスクリプト）・`innerHTML` 不使用は変えない。
- **拡張**: `edit.js` の `parseEdit` が 2 種を足した。端は `名前[添字].名前`（`deleteConnect` と同じ）、`index` は 0 以上 1 万以下の整数、座標は有限で絶対値 1e6 以下、`ends` は 2 点。`file`・`class` は受け取らない。`panel.js` は S264 と同じ `apply`（応答を待つあいだに文書の版が変われば入れない。undo 可、保存しない）に流す。端の点・`points` の無い結線の移動は LSP が null を返し、webview へ `ok: false`。
- **確かめ**: `pnpm test` にメッセージの検査（端・index・座標・ends・`file`/`class`）、共有の関数（つまみの対象、点の決め方、要素の属性）、webview・panel のメッセージ名。`e2e/suite.cjs` に「結線を足して `addWaypoint` → `moveWaypoint`（端の逆順）で `Line(points)` が書き換わり、端の点は動かせず、undo 3 回で戻る」。webview の DOM は拡張ホストから操作できないので、e2e は webview が送るメッセージから先を回す（S264・S293 と同じ）。
- **持ち越し**: 実機の VS Code でのつまみのドラッグ・ダブルクリックの目視、点の削除、ドラッグ中の線の追従。

## 付記（S306: VS Code の側のパネルでのパラメータの入力）

- **S294・S298 の「ソースへ修飾を書く」を、VS Code の側のパネルでもできるようにした。** Rust・LSP は変えない（`globe/parameters` と `globe/setModifier` をそのまま呼ぶ）。
- **操作**: 図の部品のダブルクリック（UI のパラメータダイアログと同じ操作）。webview は `{type: "params", part}` を `postMessage` するだけで、拡張（`panel.js` の `editParams`）が受ける。QuickPick で部品のパラメータ（`globe/parameters` の `部品.名前` のうち直下の 1 段。配列・String は UI と同じく除く）を選び、Real・Integer は InputBox（既定値を初期値に。空で修飾を外す）、Boolean は QuickPick の `true` / `false`、列挙は QuickPick のリテラル（書くのは `型.リテラル`。`enum_type` が無ければ断る）、どちらも末尾の「修飾を外す」で外す。`globe/setModifier` が返した TextEdit を、S264・S293 と同じ `apply`（`WorkspaceEdit` で undo 1 回・保存しない・待つあいだに文書の版が変われば入れない）で入れる。
- **QuickPick と InputBox にした理由**: webview の中に小さな表を作る案より小さい。webview は CSP が `script-src 'nonce'` だけで、入力部品・検証・列挙の選びを足すと `webview.js` が増える。QuickPick・InputBox は VS Code が出し、キーボードで使え、e2e からは `panel.ui` を差し替えて値を渡せる。代わりに、多くの値を一度に見て直すことはできない（その要望が出たら表へ）。
- **検査**: webview から受け取るのは部品の名前（識別子）だけ（`edit.js` の `parseParams`）。`file`・`class` は受け取らず、最後に描かせた図のものを使う。値の字句の検査は Rust の `set_modifier` に任せ、断られたら -32803 の理由を `showErrorMessage` に出す（InputBox で数に絞らない。`2*R0` のような式も書ける）。変わらなかった（外す修飾が無い）ときも「ソースは変わらなかった」と出す。
- **確かめ**: `pnpm test` に `parseParams`（`file`・`class` を捨てる、入れ子・空白つきの名前を断る）・`partParams`（部品直下の 1 段だけ・配列・String・別の部品・モデル自身を除く）・`enumValue` の試験。e2e（`e2e/suite.cjs`、`e2e/fixtures/Params.mo`）に、部品 `r1` の `R` を 100 にすると宣言が `Res r1(R = 100)` になり undo で戻る、空の入力で外れる、列挙が `(mode = Params.Mode.high)` で入る、`1;2` は理由つきで断られて文書が変わらない、の 1 本（QuickPick・InputBox は `panel.ui` の偽物。ダブルクリックは拡張ホストから起こせないので、webview のメッセージをそのまま渡す）。
- **持ち越し**: キーボードだけで部品のパラメータを開く口（今はダブルクリックのみ。コマンドパレットのコマンドにするのは小さい）、複数の値を一度に直す表、入れ子・`each`・`final`・`redeclare`、配列の部品。

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

- **同じ形のデータで渡す。** `globe-mode-fmt` の `diagram(src, 名前, library)`（`icon.rs` の隣の `diagram.rs`。字句解析・`split`・`class_spans` だけ）が `Diagram { class, extent, parts, connections, shapes }` を返し、LSP `globe/diagram`（`RPC_METHODS` 24 から 25）・wasm `diagram`・`Backend.diagram(cls, at?)` が JSON にする。図形は Icon と同じ `IconShape`。UI は `lib/ShapeView.svelte`（S244 の描画を切り出した）を Icon と Diagram で共用する。
- **部品**は `annotation(Placement(transformation(extent, origin, rotation)))` を持つ宣言だけ（`visible = false`・Placement が無い・`extent` が読めないものは飛ばす。`extent` が無ければアイコンの枠のまま）。型は `import` の別名・`.*`・内側から外側へのスコープの順で引き、そのクラスの Icon を `icon_named`（`%name` を部品の名前にする）で重ねる（`extends` は Icon と同じ 1 段）。引けない型は空のアイコン（UI は破線の枠）。配列の部品（`R[3]`）は 1 つ。
- **変換**は Rust が行列 `[a, b, c, d, e, f]`（Modelica の座標、y 上向き）にして返す。`extent` の向きが逆なら反転（Modelica の `flipHorizontal` 相当）、`origin` を中心に `rotation`（度、反時計回り）。UI は y を反転した座標に直す（`matrix(a -b -c d e -f)`）だけで、`<g transform>` に置く。`bbox` は変換後の枠（部品名の位置と、`coordinateSystem` が無いときの `extent`）。
- **connect** は `Line(points)` があればその折れ線（色・太さ・パターンも）、無ければ 2 つの部品の中心を結ぶ直線。端が部品でない（モデル自身の connector）connect で `Line` も無ければ飛ばす。`for` の中の connect も 1 本として数える。
- **持ち越し**: `extends` で継いだ部品、`import A.{B, C}`、`DynamicSelect`、`iconTransformation`、反転した部品の文字（鏡像のまま）、編集（ドラッグ・接続）。

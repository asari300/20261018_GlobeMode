# S245 `diagram-view`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s245-diagram-view` |
| worktree | `.git-worktree/s245-diagram-view` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S245-diagram-view/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0037 の次。Diagram ビュー（読み取り専用）。部品を `Placement` どおりに部品のクラスの Icon で置き、`connect` を `Line` の折れ線か中心どうしの直線で、`Diagram(graphics)` を重ねて描く。編集はしない。

## 決めたこと

- S244 と同じ形のデータで渡す。部品は変換の行列（Modelica の座標、y 上向き）と Icon の図形、UI は `matrix(a -b -c d e -f)` に直すだけ。
- 依頼の `flipHorizontal` / `flipVertical` は Modelica の仕様の属性ではないので、`extent` の点の順（x1 > x2 なら水平反転）で反転を決める。
- `Placement` の `transformation` があって `extent` が読めないものは壊れた Placement として飛ばす。`extent` が無ければアイコンの枠のまま。
- 型が引けない部品は空のアイコンで残し、UI は破線の枠にする（数は保つ）。
- `coordinateSystem` が無いときは仕様の既定（-100〜100）でなく、部品・線が収まる枠にする（枠の外の部品が切れるため）。
- `%name` は部品の名前にするため、`icon.rs` に `icon_named` を足した（`icon` は従来どおり）。
- `ShapeView.svelte` に図形の描画を切り出し、Icon と Diagram で共用。タブの `role="tab"` は既存のファイルのタブの試験（`get_by_role("tab")`）とぶつかるので `aria-pressed` のボタンにした。
- 図は説明と一緒に毎回引く（タブが閉じていても）。重ければタブが開いているときだけにする。

## やったこと

`globe-mode-fmt/src/{diagram.rs,icon.rs,structure.rs,lib.rs}`、`globe-mode-lsp/src/{diagram.rs,icon.rs,lib.rs}`、`globe-mode-wasm/{src/lib.rs,web/worker.js,web/client.js,node/smoke.mjs,tests/decay.rs}`、`apps/desktop/src/shell.rs`（`RPC_METHODS` 25）、`src/Svelte/src/lib/{backend,worker-backend,tauri-backend,docs}.ts`、`ShapeView.svelte`・`DiagramView.svelte`・`IconView.svelte`・`DocsPanel.svelte`、`App.svelte`（状態 1 つと `onJump`）、`tests/test_ui.py`、文書（ADR-0037 の付記、features、roadmap、CHANGELOG、`src/Svelte/README.md`）。

## 検証

- `cargo test -p globe-mode-fmt --lib diagram` 2 本（部品 2 つ・connect・Line・rotation・反転・配列・型が引けない・hidden・壊れた Placement・Placement 無し・`coordinateSystem` 無し・壊れた文書）、lsp の `diagram_is_served_with_parts_and_connections`、wasm の `diagram_is_read_as_shape_data_from_the_document`: 通った。
- `mise run wasm:node`（MSL あり）: 文書の図と、ChuaCircuit の部品 7・connect 9（全部品でアイコンが引けた）が ok。
- `mise run ui:check`: `pnpm check` 0 エラー、`test_ui.py` 全部 ok（新しい `diagram_view`: 自分のモデルで部品 2・connect 2〔Line あり・なし〕・Diagram の文字・クリックで宣言の行、MSL の ChuaCircuit で部品 7・connect 9）。初回は `role="tab"` が既存のファイルのタブの試験とぶつかり、`aria-pressed` のボタンに直した。`cos 90°` の丸め（6e-17）で points の文字列が崩れたので、行列を 1e-9 に丸めた。
- MSL の例題 20 本（`src/Modelica/msl-compare.txt`）: 部品 174、connect 178、Diagram の図形 21。アイコンが空の部品 0。ソースの正規表現での数（connect と Placement）と、独立ファイルの 13 本で全部一致。1 本あたり 2〜26 ms（wasm、Node）。読めなかったものは 0（飛ばしたものの分類: この 20 本では無し。自分のモデルでは hidden・壊れた Placement・Placement 無し・端が部品でない connect）。
- `mise run check` 成功（fmt・clippy・test・deny・uv sync）。
- `check_docs.py` 切れ 0、`render_slides.py`（S245 のデッキ）問題 0。
- 目では見ていない（Playwright の DOM の確認だけ）。

## 残したこと

編集（ドラッグ・接続）、`extends` で継いだ部品、`import A.{B, C}`、`iconTransformation`、`DynamicSelect`、反転した部品の文字、VS Code・Leptos・Dioxus。

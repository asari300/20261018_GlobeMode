# S079 `graph-library-models`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s079-graph-library-models` |
| worktree | `.git-worktree/s079-graph-library-models` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S079-graph-library-models/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

親のエージェントから。facade の `Workspace::graph(model, depth)` が、ライブラリ（`with_libraries` / `with_library_sources` / Python の `libraries=`）として読んだモデルを「索引に無い」にする（S039・S046・S048 の持ち越し）。`check`・`simulate` と同じ道（分割読み込みでも）で引けるようにする。facade のテストを `with_libraries` と `with_library_sources_split` の両方で 1 本、Python のテストの `load_dir` の迂回を外す。S076 が触る `globe-mode-lsp`・`globe-mode-protocol`・`src/Svelte/src/lib/tauri-backend.ts` は触らない。main へのマージ・push は親。

## 決めたこと

- ライブラリのクラスは検査器（Rumoca の Session）の中にしか無く、facade・db からそれを読む既存の API が無かった。依頼の「graph と facade の範囲」を少し外れ、`globe-mode-frontend` に `Checker::library_outlines(model)` を 1 つ足した（Session のライブラリの文書の構文木 → `class_outline`。作業中の文書は除く）。本文を facade で持ち直す案はメモリが倍、全部を解析し直す案は遅いので採らなかった。
- 分割読み込みでは、先に `ensure_library(model)`（検査と同じ閉包）を呼ぶ。部品の型は flatten に要るので閉包に入る。
- `GlobeDb::connect_graph` は、salsa の `connect_graph` が Err で、ライブラリがあるときだけライブラリの索引で作り直す。ワークスペースのモデルの結果は変えない。索引はライブラリ → 文書の順（後勝ちで文書を優先）。キャッシュはしない（`shortcut:` を書いた）。
- Python の `test_reference_scenario` は `Workspace(libraries=[MODELICA])` にした（`load_dir` の迂回を外した）。`test_widget.py` と `sim_app.py` の `load_dir` は `simulate` だけなのでそのまま。
- frontend は読むだけのメソッドを足しただけで検査の道を変えていないので、`msl:compare` は回していない。

## やったこと

- `src/Rust/mod/globe-mode-frontend/src/rumoca.rs`: `Checker::library_outlines`
- `src/Rust/mod/globe-mode-db/src/lib.rs`: `GlobeDb::connect_graph` のライブラリの道
- `src/Rust/mod/globe-mode/tests/graph_library.rs`: `RealCircuit` を 4 つの読み込みの道（`with_libraries`・`with_libraries_full`・`with_library_sources_split`・`with_library_sources`）で 4 ノード、無いモデルは Err、`data/msl` があれば `ChuaCircuit`
- `src/Rust/mod/globe-mode-wasm/src/lib.rs`・`tests/decay.rs`: 古くなったコメント
- `src/Python/tests/test_api.py`: `libraries=` で graph
- `doc/roadmap.md`: 持ち越し 3 か所に取り消し線、S079 の節

## 検証

- `cargo test -p globe-mode --test graph_library`: 2 件 OK（`RealCircuit` 4 ノード × 4 つの道、MSL の `ChuaCircuit` 深さ 1 で 7 ノード。15.7 s）
- コードの検査（`mise run check`）: 通った（ci_plan 23 件、fmt・clippy -D warnings、test 602 passed / 0 failed / 11 ignored、deny の advisories・bans・licenses・sources ok、`uv sync --locked`）。1 回目は fmt、2 回目は clippy の `cloned_ref_to_slice_refs` で落ち、直した
- `mise run wasm:node`: 通った（Decay 3 ソルバ、MSL 2556 ファイルで ChuaCircuit bdf C1.v(T) = -0.99289、283.8 s）
- `mise run wasm:compare`: 通った（367 / 380 件が一致、既知の差 13 件、一致しない 0 件。1 回目は共有 target の古いビルドで E0599、touch して回し直した）
- `uv run pytest -q src/Python/tests`: 4 passed（`libraries=` の graph を含む）
- `check_docs.py`: スライド OK、文書 340 件 / 相対リンク 1644 件 / 切れ 0 件
- `render_slides.py`: 1 回試してスクリーンショットで時間切れ（手元の負荷）。CI に任せる
- セッション記録の `--check`: 親が写すので未実施
- ステージした差分の漏洩検査: 0 件

## 残したこと

- ワークスペースのモデルがライブラリの部品を使うとき、その型はライブラリを見に行かない（今までどおり `unresolved`）。
- ライブラリの索引のキャッシュ（全部読む道では呼ぶたびに MSL の 2556 文書から作る）。
- `msl:compare` は回していない（検査の道は不変）。

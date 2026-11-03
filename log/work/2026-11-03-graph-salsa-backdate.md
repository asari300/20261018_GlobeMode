# S122 `graph-salsa-backdate`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s122-graph-salsa-backdate` |
| worktree | `.git-worktree/s122-graph-salsa-backdate` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S122-graph-salsa-backdate/` |
| セッション記録 | （親が写す） |

## 依頼

親のエージェントから。S089 の持ち越しの 2 つ目「ライブラリがあるとき、文書のモデルの graph は salsa の外で作るので、salsa の再計算の省き（backdate）が効かない（S081 と同じ）」を直す。確かめ方は (1) 同じ文書の 2 回目、(2) コメントだけの編集で再計算されない、(3) 部品を足すと再計算される の 3 本と、ChuaCircuit の 7 ノード・9 辺が変わらないこと。main へのマージ・push・worktree の削除は親がやる。

## 決めたこと

- ライブラリの索引は salsa の入力 `Library`（欄 `Arc<ClassIndex>`）にした。tracked の返り値で `Eq` にする道もあったが、毎回 MSL の索引を比べることになる。入力は比べないので、`Checker::library_outlines` が新しい世代を返したときだけ差し替える（世代を入力にする案と同じ意味で、索引も一緒に持てる）。
- graph は tracked の `library_graph(workspace, library, request)`。中身は S089 の `connect_graph` の後半と同じ（`class_index` を `ClassIndex::over` で重ねて `build`）。`class_index` は S089 で毎回 `from_outlines` していたワークスペースの索引と同じもの。
- `library_outlines` は毎回呼ぶまま（分割読み込みで閉包が増えたかは Session でしかわからない）。
- 同じワークスペースの別のモデルの編集は `class_index` が変わるので graph を作り直す。ライブラリが無いときと同じなので、モデルごとの依存に分けるのはやめた（依頼の「別のモデル」はここだけ満たしていない）。

## やったこと

- `src/Rust/mod/globe-mode-db/src/lib.rs`: 入力 `Library`、クエリ `library_graph`、`GlobeDb` の欄 `library_index` を `library: Option<(usize, Library)>` に、`connect_graph` の後半、モジュールの説明。
- `src/Rust/mod/globe-mode-db/tests/validation.rs`: `library_graph_is_backdated`（3 本）。
- `src/Rust/mod/globe-mode/tests/graph_library.rs`: ChuaCircuit の 2 回の graph でノードと辺の数を出して 7・9 を確かめる。
- 文書: ADR-0005 の結果に S122、architecture.md のクエリの表に `library_graph`、roadmap、索引、スライド、解説書。

## 検証

- `library_graph_is_backdated`: `library_graph` の実行 (1) 0 回、(2) 0 回（`outline` 1 回）、(3) 1 回でノード +1。
- MSL の `ChuaCircuit`（`with_libraries_full`、debug）: 1 回目 181.1 ms、2 回目 0.069 ms（S101 は 4.6 ms）、7 ノード・9 辺。
- `cargo test -p globe-mode-db -p globe-mode-graph`: すべて通過（`validation` 15 通過・1 ignored ほか、失敗 0）。`cargo test -p globe-mode --test graph_library`: 3 通過
- コードの検査（`mise run check`）: `test_ci_plan.py` 25 OK、fmt・clippy 通過、`cargo test --workspace` は 623 通過・失敗 0 のあと、`globe-mode` の doctest が共有 target の古い `globe_mode_fmt`（E0460）で落ちた。`globe-mode-fmt` と `globe-mode` を touch して `cargo test --workspace --doc`（1 通過・失敗 0）、`cargo deny check`（advisories・bans・licenses・sources ok）、`uv sync --locked` を回し直して通った
- `check_docs.py`: デッキすべて OK / 文書 516 件・相対リンク 2583 件中の切れ 0 件。`render_slides.py`: デッキ 1 件 / 問題 0 件（1 回目は 30 s の時間切れ。Chromium を片付けて回し直した）
- セッション記録の `--check`: 親が写すので、このセッションでは回していない
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 同じワークスペースの別のモデルの編集でも graph を作り直す（`class_index` が変わるため）。
- `Medium.AbsolutePressure` のような短いクラスの別名の中（S101 の持ち越し）は触っていない。

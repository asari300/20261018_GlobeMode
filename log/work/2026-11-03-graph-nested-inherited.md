# S101 `graph-nested-inherited`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s101-graph-nested-inherited` |
| worktree | `.git-worktree/s101-graph-nested-inherited` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S101-graph-nested-inherited/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S089 の持ち越しの 1 つ目「継承したクラスの中の入れ子のクラスは、索引（`ClassIndex`）がまだ引かない」を直す。MSL で実例を 1 つ見つけ、前後を数字で示す。ついでに S094 の作業記録の ME の差 `2.6e-9` を `2.6e-8` に直す。salsa の backdate、exec・fmu・fmi・CLI の `fmu`・workflows・Svelte・Python・desktop は触らない。新しい依存は足さない。main へのマージ・push・worktree の削除は親がやる。

## 決めたこと

- 直すのは `globe-mode-ir` の `ClassIndex::resolve` だけ（graph・db・frontend は `resolve` を通るので変えない）。既定のまま進めた。
- extends 先の名前は、受け継いだものを見ずに引く（MLS §5.6.1）。見ると自分自身の extends へ戻って止まらない。深さ 32 で打ち切る（`collect_interface` と同じ）。
- 順: 最初は MLS どおり各スコープで「クラス → 受け継いだ入れ子 → import」にしたが、MSL の例題 479 本の graph（深さ 4、debug）が 40.2 → 174.3 s になった。字句と import で引けないときだけ受け継いだ入れ子を見る版（`shortcut:` のコメント付き）にした。MSL のモデル 2,002 本（深さ 1）は 23.2 → 24.2 s。
- 依頼の `Modelica.Electrical.Analog.Examples.*`・`Modelica.Mechanics.Rotational.Examples.*` には該当が無かった（例題 479 本の `unresolved` は前から 0）。MSL の非 partial の model・block 2,002 本を走査して、実例 `Modelica.Fluid.Pipes.StaticPipe` を見つけた。

## やったこと

- `src/Rust/mod/globe-mode-ir/src/lib.rs`: `resolve` を `resolve_in(scope, name, inherited)` の 2 回に分け、`inherited` を足した。単体テスト `resolve_finds_nested_classes_of_extended_classes`（2 段の extends と循環）。
- `src/Rust/mod/globe-mode-db/tests/validation.rs`: `nested_class_of_extended_class_is_resolved`（salsa の `connect_graph` を通る最小の例）。
- `src/Rust/mod/globe-mode/tests/graph_library.rs`: MSL があれば `StaticPipe` の `flowModel` がノードになることを確かめる。
- `log/work/2026-11-02-s094-fmi-export-rumoca.md`: ME（`--relative-tolerance 1e-8`）の差 `2.6e-9` → `2.6e-8`（0.3678794412 − 0.3678794151 = 2.61e-8）。
- 文書: 解説書・スライド・索引・roadmap の「S101 で済んだこと」。

## 検証

- 走査（一時のテスト。コミットしていない。`with_libraries_full`、MSL 4.1.0）:
  - MSL のモデル 2,002 本（深さ 1）: ノード 10,338 → 10,339、辺 11,187 → 11,187、型の付いた辺 10,931 → 10,931、`unresolved` 321 → 320。変わったのは `Modelica.Fluid.Pipes.StaticPipe` だけ（ノード 1 → 2、`unresolved` 3 → 2、残りは `Medium.AbsolutePressure`・`Medium.MassFlowRate`）。索引に無いと出た名前 34 本（走査の名前の切り出しの誤り）は前後で同じ。
  - MSL の例題 479 本（深さ 1 と 4）: 前後で差 0。ノード 5,204・辺 5,997（深さ 1）。
- 最小の例: 前は `unresolved` が `["Part"]`、後は 0、ノード 2、辺 1 本は `flow`。
- `ChuaCircuit`: ノード 7・辺 9（S079・S089 と同じ）、2 回目の graph 4.6 ms（debug）。
- `cargo test -p globe-mode-ir -p globe-mode-graph -p globe-mode-db`: 全部通過。`cargo test -p globe-mode --test graph_library`: 3 通過。
- コードの検査（`mise run check`）: 1 回目は `cargo fmt --check` で落ちた（`cargo fmt --all` で直した）。2 回目は fmt・clippy・test（通過 613、失敗 0、ignored 11）・deny（advisories / bans / licenses / sources ok）・`uv sync --locked` が通った
- `check_docs.py`: デッキすべて OK / 文書 424 件・相対リンク 2,073 件中の切れ 0 件
- `render_slides.py`（S101 のデッキだけ）: デッキ 1 件 / 問題 0 件（6 枚 × desktop-light・desktop-dark・mobile）。他のエージェントの zig のビルドで load 30 のとき 3 回とも Chromium の screenshot が 30 s で時間切れ、load 7 で通った
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 短いクラスの別名の中（`Medium.AbsolutePressure` など）は引かない。2,002 本の `unresolved` 320 のうち 269 は `Medium.` で始まる。
- 受け継いだ入れ子と外側のスコープの名前がぶつかるときの順（`shortcut:`）。
- salsa の backdate（S089 の持ち越しの 2 つ目）。

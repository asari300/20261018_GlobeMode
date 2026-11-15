# S361 `flat-outer-connect`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s361-flat-outer-connect`（S355 の `feature/s355-flat-stream` の 4d916cb0 から切り、main の 93d9670c〔S358 まで〕を取り込んだ） |
| worktree | `.git-worktree/s361-flat-outer-connect` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S361-flat-outer-connect/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S355 のあと MSL の例題で flat が断るのは 28 本。うち `connect` が 10 本で、9 本は `StateGraph` の `outer` のコネクタ（`stateGraphRoot.subgraphStatePort`）への `connect`（`inner` が後ろで宣言されていて葉がまだ無い。`BatchPlant_StandardWater`・`ControlledTankSystem.ControlledTanks` を含む）、残る 1 本は `LogicalSample`（大きさの違う配列の `connect`）。後ろで宣言された `inner` に `outer` のコネクタを `connect` できるようにする（S284・S329・S355 の `pending`・後ろの `inner` の扱いを再利用）。余力で `LogicalSample`。Rumoca の flat と名前・方程式の数を合わせ、回帰 0。最後に全体の検査を 1 回回して数字を作業記録と報告に書き、ADR-0039 の付記・roadmap（要約の形）・architecture・README・CHANGELOG の Track B の数字を直す。

## 決めたこと

- **base は S355 のブランチにした**（S355 が main に未着。回帰の基準 `target/frontend-diff.json` は S355 の全体の検査のもの）。作ったあと main（S358 まで）を取り込んだ。衝突は `globe-mode-flat/tests/flat.rs` の末尾の 1 か所（S358 の `read_dirs_skips_hidden_dirs` と S355 の `stream_sets_and_in_stream` が同じ位置に足されていた）で、両方残した。
- **9 本の原因は「葉がまだ無い」だけではなかった**。`step` の階層の `gen_connects` が `connect(outerStatePort.subgraphStatePort, stateGraphRoot.subgraphStatePort)` を単独の集合にするので、`inner` が前で宣言されていても部品の数だけ flow の和ができる（Rumoca は全部の `connect` を集めた 1 つの集合）。「後ろで宣言されたものだけ」ではなく、**外側の階層で宣言した `inner` の中を指す端は、全部その `inner` を宣言する階層へ持ち越す**ことにした（`late_conns`）。外側のコネクタの端の符号は持ち越す前の階層で決まるので一緒に持ち越す。MSL の例題では `inner` の宣言の位置によらず数が合う。
- **`inner outer` は実体を作る**。`StateGraph.PartialCompositeStep` の 1 か所だけ。これまで外側の `inner` の別名にして部品を作らなかったが、Rumoca の flat は `compositeStep.stateGraphRoot.*` を作る（名前が 6 本足りなかった）。同じ階層の `connect` の端だけ外側の `inner` へ付け替える（Rumoca の `redirect_connection_for_inner_outer` を読んで合わせた）。
- **`cardinality` の `if`**: 数える・選ぶのは全部の `connect` がそろったあと。枝ごとに平らにした方程式を目印の 1 行の位置に置き、`apply_connection_graph` のあと（`expand_streams` の前）に置き換える。位置（`at`）ではなく目印の文字列で探すのは、`apply_connection_graph` が方程式の列を積み直して位置が動くため。Rumoca の `compute_cardinality_counts`（`connect` の端の完全な名前の出現の数）と同じ数え方。方程式が部品あたり 2 本多かった（`Step` の `if cardinality(inPort[i]) == 0 …`）のが、10 本のうち 9 本で名前は合うのに数が合わなかった原因。
- **`constrainedby` の修飾は差し替えた部品にだけ掛けた**。差し替えていない既定の型には掛けていない（`LogicalSample` は差し替えた側で足りる。既定の型に掛けると 498/511 だった一致が動くか確かめていない。Rumoca の挙動も未確認）。
- **内包の中の `inStream`**（`BatchPlant_StandardWater` が `connect` を通ると `inStream(multiPort.ports_b[j].h_outflow): not a variable` で止まった）は、S355 が shortcut として断っていた形。`pr`（印字）で、本体に `inStream`・`actualStream` を含む内包を添字ごとに展開した。添字が複数の内包は断る（MSL に無い）。
- **DAE は触らない**（S362 が `globe-mode-dae` を触る）。新しく通った 10 本で DAE が食い違う 4 本は、持ち越しとして ADR に書いた。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `late_conns`・`late_owner`（持ち越し。`gen_connects` に `pfx` を渡す）、`bridge`・`bridge_end`（`inner outer`）、`card_ifs`・`card`・`defer_card_if`・`resolve_card_ifs`・`eval` の `cardinality`、`inst_decl` の `constrainedby` の修飾、`pr` の内包の展開。
- `globe-mode-flat/tests/flat.rs`: `outer_connect_to_later_inner`・`inner_outer_is_instantiated_and_bridged`・`cardinality_selects_if_branch`・`redeclare_keeps_constraining_modification`・`in_stream_in_comprehension`。
- 文書: ADR-0039 付記（S361）と推移の表・残りの理由、roadmap・architecture・features・README・CHANGELOG、`doc/frontend-diff.md`（生成）、セッション文書、索引。

## 検証

- **全体の `diff-frontend`**（共有 target の `release/xtask` が入れ替わるので、作った直後に `bin-copies/xtask-s361c` へ写し、`flock` の下で回した）: MSL の例題 566 本で、平らにできた 538 → **548**、Rumoca も通った 511 → 521 本で名前 508/511 → **518/521**・方程式の数 498/511 → **508/521**、DAE 538/538 → **548/548**、Rumoca の釣り合いまで通った 367 → 377 本で判定 **374/377**・方程式と未知数の数 **373/377**・状態の数 **376/377**・構造の正則 **369/377**、`connect` 10 → **0**。`GlobeModeValidation` 75 本は変わらず（Rumoca も通った 74 本で名前・方程式の数・判定・数・状態 74/74、正則 73/73）。**回帰 0 本、改善 10 本**。1009 s、最大 RSS 5.8 GiB。
- 新しく通った 10 本: 名前・方程式の数 10/10。DAE は 6 本が 4 項目とも一致。食い違う 4 本は、構造の正則 3 本（`ExecutionPaths`・`ShowCompositeStep`・`ShowExceptions`。`{a.join[1].occupied, a.join[2].occupied} = fill(…)` の行の incidence が自前の DAE で空。理由は dae の側で未確認）と `BatchPlant_StandardWater`（方程式の数 2365 対 2389・正則。レコードの欄ごとの行【推測】）。
- `cargo test -p globe-mode-flat`（53 本）・`-p globe-mode-dae`・`-p globe-mode-frontend` 通過、`cargo clippy -p globe-mode-flat -p globe-mode-dae --all-targets -- -D warnings`・`cargo fmt --all --check` 通過。
- `cargo test -p globe-mode-dae -p globe-mode-frontend`（lib.rs を touch してから）失敗 0。`git fetch && git merge origin/main`（S359・S360 を取り込んだ）は `doc/sessions/README.md` の 1 か所だけ衝突し、S361・S360・S359 を残して解いた。取り込んだ変更は LSP（call hierarchy）・UI の試験・上流の下書き・文書で、flat・dae に及ぶ変更は無い（全体の `diff-frontend` は回し直していない）。取り込み後の `check_docs.py`: 相対リンク 5455 件中の切れ 0 件。`render_slides.py`（S361 のデッキだけ）: 問題 0 件。markdownlint 0 件。
- 共有 target の罠: 最初の `cargo test` が別の worktree の古い `globe-mode-resolve` を使い、`mo_files` が見つからない E0432 になった（resolve・flat の lib.rs を touch して直った）。あとで `cargo test -p globe-mode-flat` が 53 本でなく 48 本しか走らず（共有 target の `flat-…` の試験のバイナリが別の worktree のものに入れ替わっていた）、tests/flat.rs と lib.rs を touch して 53 本通った。
- 最初の 10 本の突き合わせは S362 の全体の検査の `flock` を待った（約 20 分）。

## 残したこと

- DAE の 4 本（`{…} = fill(…)` の incidence、レコードの欄ごとの行）。`globe-mode-dae` の側。
- 断る 18 本: `composite-binding` 6・`expandable` 5・`non-constant-dim` 4・`connect-array` 1・`non-constant-for` 1・`unsized-array` 1。
- 差し替えていない既定の型への `constrainedby` の修飾、`cardinality` の入れ子（断る）、`smooth(0, m_flow * actualStream(x))`。

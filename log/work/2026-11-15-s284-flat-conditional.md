# S284 `flat-conditional`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s284-flat-conditional` |
| worktree | `.git-worktree/s284-flat-conditional` |
| Phase | Track B（B3 の 2 つ目） |
| セッション文書 | `doc/sessions/S284-flat-conditional/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

S278 の `globe-mode-flat` が断る理由の上位を減らす。(1) 条件つきの宣言（定数の条件。偽なら宣言とつながる `connect` を落とす）、(2) `inner`/`outer`（外側の最初の同じ名前の `inner` に付け替え。無ければ断る）、(3) レコードの束縛・定数の大きさ・`[:]`。`globe-mode-flat` の中だけを変え、resolve・syntax は変えない。

## 決めたこと

- 条件が評価できない宣言は今までどおり断る（近似しない）。
- 定数の条件の `if` の方程式も選んだ枝だけを平らにする。条件つきの宣言の部品が `if not useHeatPort then … end if` の方程式を持つので、これを入れないと Rumoca の flat と方程式の数が合わなかった（`FluidHeatFlow` の 10 本で 15〜40 本ずれた）。ついでに `connect-in-if-when` の 13 本も通った。
- コネクタの中の parameter・constant は `connect` の等式にしない（Rumoca の flat と同じ）。
- レコードの束縛は欄ごとの修飾に落とす（明示した修飾 > 束縛 > 型の修飾）。関数の戻り値のレコード（Spice3 の `…RenameParameters`）・外部オブジェクト（`ExternalCombiTimeTable`）は評価が要るので断る。
- 大きさは式の形（`dims_of`）で決める。ユーザー定義の関数の戻り値は断る（`shortcut:` コメントを残した）。

## やったこと

- `src/Rust/mod/globe-mode-flat/src/flatten.rs`: 条件つきの宣言（`removed` と `is_removed`）、定数の条件の `if`（`const_branch`）、`inner`/`outer`（`inners`・`alias`・`find_inner`。`cref` と `eval` が別名を引く）、レコードの束縛（`record_value`・`record_fields`・`Bind::suffix`）、大きさ（`decl_dims`・`dims_of`・`pkg_const`・`var_dims`・`pending`、`Val::E`）、コネクタの parameter を等式にしない。
- `src/Rust/mod/globe-mode-flat/tests/flat.rs`: 単体 3 本を足し（条件つきの宣言と connect、`inner`/`outer`、レコードの束縛と大きさ）、`declined_forms` と `connect_in_for_and_if_kept_as_text` を新しい動きに合わせた。`tests/compare.rs`: `GLOBE_FLAT_SHOW=<理由,…>` で断った本数を全部出す。
- doc: ADR-0039 の付記（S284）、roadmap（Track B の 2 か所）、architecture の crate 表、features、セッションの索引、セッション文書。

## 検証

- `cargo test -p globe-mode-flat`: 10 pass・4 ignored（S278 は 7 pass）。`cargo clippy -p globe-mode-flat --all-targets -- -D warnings` 警告 0、`cargo fmt` 済み。依存の追加なし（`cargo deny` は対象外）。
- `GlobeModeValidation` の 75 本: 自前で平らにできた 71 → **72**（断った 3 は `redeclare`）。両方で通った 71 本で名前 71/71・方程式の数 71/71（S278 は 70/70）。
- MSL 4.1.0 の例題 566 本: 自前で平らにできた 67 → **192**。Rumoca の flat まで通った 189 本で名前 **189/189**・方程式の数 **186/189**（S278 は 66/66・64/66）。合わない 3 本は `UniformNoise`（10 対 12）・`UniformNoiseXorshift64star`（10 対 11）・`GenerationOfFMUs`（210 対 214）で、原因は未確認。
- 断った理由の内訳（最初の理由だけ。S278 → S284）: `redeclare` 205 → 263、条件つきの宣言 131 → 6、`inner`/`outer` 43 → 0、大きさが定数でない 35 → 2、レコードの束縛 33 → 41、`[:]` 25 → 13、`class extends` 11、`replaceable` 5 → 9、配列の型 5 → 13、配列のコネクタの一括接続 4 → 7、`connect` の集合の不一致 0 → 7、`expandable` 2。先へ進んだ分が別の理由に当たるので、増えた行もある。
- 注意: 共有 target に別の worktree の syntax・resolve のビルドが混ざり、同じコードでテストが落ちたり本数が 67 に戻ったりした。何度も出た。`touch src/Rust/mod/globe-mode-{syntax,resolve,flat}/src/lib.rs` のあとに回した数字だけを載せた。
- `check_docs.py`: 全デッキ OK・相対リンク 4787 件中の切れ 0 件。`render_slides.py`（S284 のデッキ）: 問題 0 件。
- `mise run check`（workspace 全体）は回していない（共有 target に別の worktree のビルドが混ざり、`cargo test -p globe-mode-flat` ですら何度か偽の失敗が出たため）。`globe-mode-flat` だけ fmt・clippy・test を通した。依存の追加は無いので `cargo deny` は対象外。
- origin/main（21e8cb38）を取り込んだ。main の `globe-mode-resolve`（S275）は成分参照を最後の名前まで解くので、統合ブランチ（`feature/integrate-1116`）の df9ca1cd と同じ 1 行（`lib.resolve(r)` → `lib.resolve_head(r)`）を足した。取り込み後に数字を取り直し、同じ（MSL 192・189/189・186/189、Validation 72・71/71・71/71）。
- 漏洩の検査: 差分にメールアドレス・ホームのパス・トークン 0 件。

## 残したこと

- `redeclare` / `replaceable`（263 本。resolve の `Redeclare` と合わせる）。
- 関数の戻り値のレコード（Spice3）・外部オブジェクト・配列の修飾つきの条件（`useHeatPort = fill(…)`）・配列のコネクタの一括接続・`connect` の集合の不一致 7 本（`StateGraph` が多い）の原因。
- 方程式の数が合わない 3 本の原因。
- `xtask diff-frontend` への組み込み、B4（DAE）。

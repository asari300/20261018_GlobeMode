# S324 `flat-composite`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s324-flat-composite` |
| worktree | `.git-worktree/s324-flat-composite` |
| Phase | Track B（B3 の 4 つ目） |
| セッション文書 | `doc/sessions/S324-flat-composite/` |
| セッション記録 | 省く（`log/local/` にこの WP 専用の記録が無い） |

## 依頼

平坦化で断る理由の上位 2 つ、`composite-binding`（155 本）と `connect-array`（72 本）を減らす。`globe-mode-flat` の中だけを変える。

## 決めたこと

- 宣言が後ろの兄弟のレコードは、巻き戻しをせず、束縛される側の型の欄に `other.f` を当てる（`WHOLE`）。相手の欄を調べないので、順序に依らない。相手の欄がもう変数にあるときは今までの経路のまま（回帰を避ける）。
- 関数の戻り値のレコードは同じ `WHOLE` で `f().T`。外部オブジェクトは不透明な 1 つの変数。
- 配列のレコードは 1 次元だけ（リテラルは要素ごと、別の配列は `x[k]`）。
- `connect` の端は `gen_connects` の先頭で展開する（`lv.conns` を作り直す）。部品の配列を通した端と配列のコネクタ、配列の変数 1 つと要素ごとの端の 2 通り。
- `a.pin.v` の方程式の数と印字を直した（配列のコネクタを通すと方程式の数が合わなかった）。
- `globe-mode-dae` は触らない。`f(…).T` を読めない問題は S325 へ申し送る。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `WHOLE`・`is_ext_obj`・`record_value` の関数の呼び出しと欄の接尾辞・配列のレコード・`expand_end`・`split_array_var`・`array_elems`（`shape` と `pr`）。
- `globe-mode-flat/tests/flat.rs`: `record_bound_to_later_sibling_and_function_value`・`connect_whole_arrays`。
- `xtask/src/diff_frontend.rs`: `--dump` で断った理由と差を出す、Rumoca の部品ごとの方程式数を出す、10 秒以上かかったモデルを出す。
- 文書: ADR-0039 の付記（S324）、roadmap・features、スライド・解説書・索引。

## 検証

- `cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend`: 全部 pass（flat 26 本）。clippy（`-D warnings`）0、fmt 差分 0。
- `mise run msl:diff-frontend`（release、共有 target）: 回帰 0、改善 79。flat 279 → 358、composite-binding 155 → 31、connect-array 72 → 1。Rumoca も通った 350 本で名前 350/350・方程式の数 340/350、DAE 253 本で判定 253/253・方程式と未知数の数 253/253・状態の数 253/253・正則 232/253。`GlobeModeValidation` は変わらず。
- release の全体で約 355 s、最大 RSS 6.1 GiB（`Spice3BenchmarkFourBitBinaryAdder` が Rumoca で約 144 s）。

## 残したこと

- 方程式の数が合わない 10 本（Rotational）の原因は未確認（Rumoca が `tau0 = mu0*cgeo*fn` を 2 本と数える）。
- `f(…).T` を DAE が読めない（`MixtureGases` 1 本。S325）。
- 次の上位: 条件つきの宣言 58、配列の修飾 38、`Complex` の式 31、`stream` 20、`for` の範囲 16。

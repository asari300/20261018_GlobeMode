# S329 `flat-arrays-complex`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s329-flat-arrays-complex` |
| worktree | `.git-worktree/s329-flat-arrays-complex` |
| Phase | Track B（B3 の 5 つ目） |
| セッション文書 | `doc/sessions/S329-flat-arrays-complex/` |
| セッション記録 | 省く（`log/local/` にこの WP 専用の記録が無い） |

## 依頼

S324 の後に上位になった断る理由（array-modification 38、conditional-component 58、composite-binding 31〔`Complex`〕、non-constant-for 16）を `globe-mode-flat` の中だけで減らす。

## 決めたこと

- 部品の配列への修飾は要素ごとに分ける（1 次元の `{…}` は項、内包・多次元は `({…})[k]`、`fill` は `x`、`each` は分けない）。
- `Complex` の演算子は演算子の関数を読まず、MSL の定義を決め打つ（小さい方）。型の分からない参照は Complex と見なす。
- 条件の評価に、文字列の比較（hash）、`outer` の欄の付け替えと宣言が後ろの inner の欄、定数の `if` のレコード、`numberOfSymmetricBaseSystems`（決め打ち）を足した。
- `globe-mode-dae` は触らない。

## 検証

- `cargo test -p globe-mode-flat`: 29 本 pass（試験 3 本を追加）。clippy（`-D warnings`）0、fmt 差分 0。
- `mise run msl:diff-frontend` 相当（release、共有 target。2 回回した）: flat 358 → 471、回帰 0、改善 113。array-modification 38 → 0、conditional-component 58 → 0、composite-binding 31 → 25、non-constant-for 16 → 1。名前 350/350 → 425/463、方程式の数 340/350 → 435/463、DAE は釣り合い 295/297・状態 293/297・正則 289/297（母数 297）。`GlobeModeValidation` は変わらず。646 s、最大 RSS 4.8 GiB。
- 名前の食い違い 51 本のうち 37 本は Rumoca が `cylinders[n]`・`Complex aw[2]` を展開しない。

## 残したこと

- Rumoca が配列を展開しない条件【未確認】。
- DAE が読めない 41 本（S330）。
- `ActuatorWithNoise`・`EngineV6`・`EngineV6_analytic` の方程式の数の原因は未確認。
- unsized-array 28、stream 22、composite-binding 25。

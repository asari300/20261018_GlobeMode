# S312 `dae-mismatches`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s312-dae-mismatches` |
| worktree | `.git-worktree/s312-dae-mismatches` |
| Phase | Track B（B3・B4 の続き） |
| セッション文書 | `doc/sessions/S312-dae-mismatches/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S297（`feature/s297-flat-redeclare`）で平坦化が MSL 279 本に広がった時点の `mise run msl:diff-frontend` の食い違い（両方で通った 273 本で方程式の数 254/273、DAE の釣り合いの判定 191/201）の原因を確かめ、直せるものは `globe-mode-flat`（数え方）か `globe-mode-dae` で直す。直せないものは既知の差として ADR に書く。`globe-mode-flat` の `Var`・`Equation` の形は変えない（S311 が由来を足している）。

## 決めたこと

- 調べ方は xtask に足す（`diff-frontend --class <名前,…> [--dump]`、`FlatSummary::equation_origins`〔JSON には出さない〕）。モデル全体では部品ごとの数が自前側に無い（`Equation` に由来が無い）ので、部品を単独で検査して Rumoca と比べ、小さいモデルへ絞る。
- 接続集合は葉の変数の単位にする（Rumoca の flat と同じ）。`protected` のコネクタの flow = 0 は、丸ごと `connect` の端か、どこにもつながっていない葉だけ。
- 過剰決定コネクタは平らにし終えたあとの後処理（`flatten/graph.rs`）。全域森の選び方（辺の順）は Rumoca と同じにしない。接続集合ごとの等式の数の合計は選び方によらないので、数は合う。切れた辺の `equalityConstraint` は戻り値の大きさ 0 と見て等式を作らない（shortcut。MultiBody の `Orientation` は未対応）。
- S297 の見立て（Complex のレコード）は外れ。`Connections.isRoot` と `protected` は当たり。
- Clocked の正則（16 本）と `WaterIF97` の正則（1 本）は、クロックの分割を持たないので直さず、既知の差にする。原因は未確認。

## やったこと

- `globe-mode-flat`: 行列 `[…; …]` の大きさ（`dims_of`）、タプルの左辺の大きさ（`shape`）、接続集合を葉の変数の単位に（`gen_connects`）、`protected` のコネクタの flow、`flatten/graph.rs`（過剰決定コネクタ）、定数の `if` 式を畳む（`const_if_expr`）、`pkg_binding` の輪（別のパッケージの同名の定数に差し替え後の `extends` の修飾を当てていた）。試験 6 本（`tests/flat.rs`）。
- `globe-mode-dae`: 束縛の `der`、レコードの方程式を欄ごとに数える（行は欄の変数すべてに触れる）、大きさ 0 の配列は未知数 0。試験 2 本。
- `globe-mode-ir`・`globe-mode-frontend`: `FlatSummary::equation_origins`（調査用）。
- `xtask`: `diff-frontend --class --dump`。
- 文書: ADR-0039 付記（S312）、roadmap・architecture・features の数字、`doc/frontend-diff.md`（生成物）、セッションの索引・スライド・解説書。

## 検証

- 検査の数字（MSL の例題 566 本、S297 → S312）: 方程式の数の一致 254/273 → 273/273、釣り合いの判定 191/201 → 201/201、方程式と未知数の数 191/201 → 201/201、状態の数 185/201 → 201/201、構造の正則 179/201 → 184/201。`GlobeModeValidation` 75 本は変わらず。回帰 0 本（前回の JSON は S297 の `doc/frontend-diff.md` を出した実行）。
- `mise run msl:diff-frontend` の代わりに、worktree 専用の target で `diff-frontend` を release で回した（約 165 s）。
- 共有の `target-shared` のままだと、別の worktree が同じ名前の crate を先にビルドした成果物が混ざり、直した結果が出ないことがあった（flat の試験が落ちる、xtask の結果が古いなど）。worktree 専用の `target-s312` を作って回した。

## 残したこと

- 構造の正則の差 17 本（Clocked 16 本・`Modelica.Media.Examples.WaterIF97`）。
- 平らにできない 287 本。

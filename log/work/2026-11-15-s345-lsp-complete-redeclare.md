# S345 `lsp-complete-redeclare`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s345-lsp-complete-redeclare`（`feature/s341-resolve-redeclare` から切った。S341 が main に入っていなかったため） |
| worktree | `.git-worktree/s345-lsp-complete-redeclare` |
| Phase | Track B（B2 の続き。製品へのつなぎ） |
| セッション文書 | `doc/sessions/S345-lsp-complete-redeclare/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S341 で `globe-mode-resolve` の `step` が差し替えた後の型を引けるようになった。LSP の補完 `members_of`（S279）は `state.` の候補を既定の型の要素のままにしている。定義ジャンプ・ホバー（S296）も同じ道を使うか確かめる。LSP の補完・定義ジャンプ・ホバーが S341 と同じ規則で差し替えた後の型へ出す・飛ぶようにする。resolve の規則を LSP に複製せず、resolve の関数を使う。`resolve_compare` の ignored 試験も回して null の変化を報告する。`msl:diff-frontend` は不要。

## 決めたこと

- **規則は resolve に 1 つ**: `step` の中の「`ctx` が `extends B(redeclare T x)` で差し替えた部品の型 → `ctx` から見直した型の名前」を `redeclared_types(ctx, c, k)` に切り出した。`step` は同じ順で使う（既定の型で引けず `Redeclare` のときだけ。resolve の数え方は変わらない）。`members_of` は最後が部品のとき、`redeclared_types` の先頭、無ければ書いてある型で要素を集める。補完は「引けるか」でなく「型を 1 つ選ぶ」ので、差し替えた型を先にした（差し替えた型は既定の型を受け継ぐか下位の型なので、既定の要素を含む）。
- **定義ジャンプ・ホバーの確かめ**: 書きかけの文書は S296 から `Library::resolve` で引くので、S341 の `step` のまま差し替えた型に飛ぶ（直す所は無かった。試験で確かめた）。**読める文書**は今の道（アウトライン）が既定の型で引いて `null` になるので、`definition` が、今の道で `null` かつ名前に `.` があるときだけ `tolerant_definition` で引き直す。`.` の無い名前（`Real`・`time` の builtin）で木（S279 の計測で debug 8.8 秒・約 75 MB。#107 のコメントは 0.6 s）を作らせないため。`textDocument/hover` の読める文書は、名前ではなくクラスの見出しを返す（S053 から）ので触らない。名前のホバーは `globe/hover`（UI 向け）が `definition` を通る。
- **補完の今の道との順序は変えない**: 今の道で候補が 1 件でも出る間は自前の道に回さない（毎回木を作る費用を払わないため）。既定の型が空（Media の `ThermodynamicState`）なら、カーソルの行だけ書きかけの文書でも自前の道に回る（試験で確かめた）。既定の型に要素があり差し替えた型にだけある要素は、今の道が読めないときだけ出る（残したこと）。

## やったこと

- `globe-mode-resolve/src/lookup.rs`: `redeclared_types` を足し、`step` の差し替えの所を置き換え、`members_of` で使う。
- `globe-mode-lsp/src/definition.rs`: `definition` を、読めない文書（`tolerant_definition`）・今の道（`outline_definition`）・今の道が `null` のときの引き直し、に分けた。
- 試験: `globe-mode-resolve/tests/resolve.rs` に `members_of_follows_redeclared_types`（S341 の試験の例を定数 `REDECLARED` にして共有）、`globe-mode-lsp/src/lib.rs` に `completion_and_definition_follow_redeclared_types`。
- 文書: ADR-0039 付記（S345）・roadmap・features・architecture・CHANGELOG・セッション文書・索引。

## 検証

@@VERIFY@@

## 残したこと

- 補完は、今の道で候補が 1 件でも出る間は既定の型のまま。差し替えた型にだけある要素を、今の道が読めるときにも出すには自前の道の結果を足し合わせる必要があり、木を毎回（最初の 1 回）作る費用との引き換えになる。
- 持ち主を受け継がないクラスから見る `a.state.`、部品の修飾の中の `A a(redeclare T x)` は S341 と同じく見ない。
- `.` のある名前で今の道が `null` のとき（`x.start` などの属性・誤字）も、引き直しで木を 1 回作る（最初の 1 回だけ）。
- VS Code の拡張は触っていない。

# S336 `flat-composite-binding`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s336-flat-composite-binding`（main の 0854d1e4 から切った） |
| worktree | `.git-worktree/s336-flat-composite-binding` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S336-flat-composite-binding/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

自前の flat が MSL の例題 25 本を理由 `composite-binding` で断っている。レコード・複素数・配列の部品に式で束縛するものを欄ごとに分けて平らにし、Rumoca の flat と変数の名前・方程式の数を合わせる。原因ごとに分けて多いものから直す。全部でなくてよいが回帰 0。同時に S335 が `unsized-array` を直している。

## 決めたこと

- **25 本を `diff-frontend --class … --dump` の理由で分けた**。内包 8（`uInternal`）、配列の式 15（`vs`・`v`）、レコード配列の内包の修飾 1（`G_m`）、2 次元と関数 1（`sTM`）。直すたびに次の理由が現れた（`fill(Complex(1, 0), nin)`、そのあとに S335 の `unsized-array`）。
- **束縛に添字を付けて `record_value` に渡す形に一般化した**。1 次元の `{…}` の項を選ぶ特別扱い（`inst_decl`）を `pick_elem` に移し、内包・N 次元・関数の戻り値を同じ道にした。`Complex` の配列の式は `complex_parts` の葉に `[k]` を当てる（`index_leaf`）。
- **Rumoca の数え方に合わせた**: `GlobeModeValidation` の小さなモデル（`$SCRATCH/val`。コミットしない）で確かめた。`Complex c[3]` の配列の方程式 `c = a - b` は 3 本（6 本ではない）、`c = {a[k]*b[k] for k in 1:3}` も 3 本。自前は内包を 1 本と数えていたので `comp_len` で範囲の長さにした。
- **S335 とは触る場所が重なるので、S335 の WIP を仮にマージして確かめた**（コミットしていない）。19 本は `indexNonPos[:] = f(m)` の `unsized-array` で止まる（S335 の領分）。合わせると 25 本すべてが平らになり、Rumoca の flat まで通った 7 本で名前の集合 7/7。ただし S335 の `var_dims` の新しい分岐が、部品の配列を通した欄 `plug_p.pin.i` に `Some([])` を返し、`plug_p.pin.i + plug_n.pin.i = fill(Complex(0), m)` を 3 本から 1 本にする（7 本の方程式の数が合わない原因の 1 つ）。S335 側か最後のマージで直す。
- 共有 target の罠: cargo の識別子は workspace からの相対で決まるので、worktree 間で `debug/xtask`・`release/xtask` と test のバイナリが上書きされる（`DBG` の出力が消えた・試験が古いままだった）。touch してから作り、作ったバイナリをすぐ scratchpad に写して使った。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `inst_decl` の `bound`（N 次元・内包）、`pick_elem`・`index_leaf`・`complex_value`・`comp_len`、`complex_parts` の `fill` と葉の添字、`record_value` の部品の配列を通した欄（`lit`）、`shape` の内包。
- `xtask/src/diff_frontend.rs`: `--dump` は自前が断ったときも Rumoca の flat を印字する。
- 試験 2 本（`complex_arrays_expand_elementwise`・`comprehension_equation_counts_its_range`）。文書: ADR-0039 付記（S336）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

@@VERIFY@@

## 残したこと

@@LEFT@@

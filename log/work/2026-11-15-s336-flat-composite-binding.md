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
- **Rumoca の数え方に合わせた**: `GlobeModeValidation` の小さなモデル（scratchpad に置いた。コミットしない）で確かめた。`Complex c[3]` の配列の方程式 `c = a - b` は 3 本（6 本ではない）、`c = {a[k]*b[k] for k in 1:3}` も 3 本。自前は内包を 1 本と数えていたので `comp_len` で範囲の長さにした。
- **S335 とは触る場所が重なるので、S335 の WIP を仮にマージして確かめた**（コミットしていない）。19 本は `indexNonPos[:] = f(m)` の `unsized-array` で止まる（S335 の領分）。合わせると 25 本すべてが平らになり、Rumoca の flat まで通った 7 本で名前の集合 7/7。ただし S335 の `var_dims` の新しい分岐が、部品の配列を通した欄 `plug_p.pin.i` に `Some([])` を返し、`plug_p.pin.i + plug_n.pin.i = fill(Complex(0), m)` を 3 本から 1 本にする（7 本の方程式の数が合わない原因の 1 つ）。S335 側か最後のマージで直す。
- 共有 target の罠: cargo の識別子は workspace からの相対で決まるので、worktree 間で `debug/xtask`・`release/xtask` と test のバイナリが上書きされる（`DBG` の出力が消えた・試験が古いままだった）。touch してから作り、作ったバイナリをすぐ scratchpad に写して使った。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `inst_decl` の `bound`（N 次元・内包）、`pick_elem`・`index_leaf`・`complex_value`・`comp_len`、`complex_parts` の `fill` と葉の添字、`record_value` の部品の配列を通した欄（`lit`）、`shape` の内包。
- `xtask/src/diff_frontend.rs`: `--dump` は自前が断ったときも Rumoca の flat を印字する。
- 試験 2 本（`complex_arrays_expand_elementwise`・`comprehension_equation_counts_its_range`）。文書: ADR-0039 付記（S336）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

- `mise run msl:diff-frontend` と同じ `diff-frontend`（`flock` つき、作ったバイナリを写して回した。共有 target の `release/xtask` は他の worktree に上書きされるため）: MSL の例題 566 本（S334 → S336）で、平らにできた 471 → **477**、Rumoca も通った 463 → 468 本で名前 463/463 → **468/468**・方程式の数 451/463 → **457/468**、DAE 471 → **477/477**、`composite-binding` 25 → **0**、`unsized-array` 28 → 47（19 本が移った）。**回帰 0 本、改善 7 本**。`GlobeModeValidation` 75 本は変わらない（73/73・73/73・73/73・正則 72/72）。1348 s、最大 RSS 5.4 GiB。
- **S337・S341 などを取り込んだ後（`git merge origin/main`、flatten.rs・flat.rs の試験・文書で衝突。両方残して解いた）に回し直した**: 同じ `diff-frontend`（`flock` つき）で 794 s、最大 RSS 5.6 GiB。平らにできた 477/566、Rumoca も通った 468 本で名前 468/468・方程式の数 457/468、DAE 477/477、状態 334/334、resolve 566/566（S341）。取り込む前の実行との差は **回帰 0 本**、改善 100 本（S341 の resolve 99 本と `GlobeModeValidation` の 1 本）。`ShowTransferFunction` は S337 の `record_array_len` でも一致するので、`comp_len` の試験は左辺が式の `a + b = {… for k in 1:n}` に替えた。
- **続けて S339 なども取り込んだ**（2 回目の `git merge origin/main`。`flatten.rs` の衝突は、S339 の `is_msl_complex` を自分の `complex_value` でも使う形で、roadmap・architecture は main の側に自分の分を足し直して解いた）。同じ `diff-frontend`（`flock` つき）の最後の実行は、数字が同じ（477/566・468/468・457/468・DAE 477/477・状態 334/334・resolve 566/566）で回帰 0・改善 0、671 s、最大 RSS 5.5 GiB。取り込み後の `cargo fmt --check`・`cargo clippy -p globe-mode-flat -p xtask --all-targets -- -D warnings`・`cargo test -p globe-mode-flat -p globe-mode-dae`（flat 45・dae 14）通過。
- 1 回目の `msl:diff-frontend` は 2800 s かけたが、`cargo xtask` が他の worktree のバイナリを使っていて（cargo の fingerprint が worktree 間で共有され、ソースの mtime がその build より古いと「最新」と見なされる）自前の変更が入っていなかった（`composite-binding` 25 のまま）。touch して作り直し、バイナリを写して回し直した。
- 前回の JSON（S334 の 641 本）は S335 の worktree の `target/frontend-diff.json` から写した（S334 の worktree は片付いていた）。
- 取り込み後の `cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-resolve -p globe-mode-frontend -p globe-mode-lsp` 通過（flat 38・dae 14・lsp 73）、`cargo clippy -p globe-mode-flat -p xtask --all-targets -- -D warnings` 0 件、fmt 済み。
- `check_docs.py`: 全デッキ OK、相対リンク 5199 件中の切れ 0 件。`render_slides.py`（S336 のデッキだけ）: 問題 0 件。ステージした差分にメールアドレス・ホームのパス・トークンは 0 件。セッション記録はコピーしていない（専用の記録が `log/local/` に無い）。

## 残したこと

- `unsized-array` 47 本（S335）。25 本のうち 19 本はここで止まる。S335 と結合したあとで、方程式の数 7 本の比較をやり直す。
- 内包の添字が 2 つ以上、`Complex` の関数に配列を渡す式。
- /tmp（tmpfs 3.9 GB）が他の担当の xtask のコピー（`xtask-s337` は 783 MB）などで一時的に満杯になった。自分の分は消した。

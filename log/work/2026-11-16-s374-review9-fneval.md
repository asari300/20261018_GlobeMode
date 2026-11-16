# S374 `review9-fneval`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s374-review9-fneval`（main の `b4019e6e`〔S370 のマージ〕から切った） |
| worktree | `.git-worktree/s374-review9-fneval` |
| Phase | Track B（B3 flat） |
| セッション文書 | `doc/sessions/S374-review9-fneval/` |
| セッション記録 | 専用の記録が `log/local/` に無いので省いた（並行の担当のうち、この WP の記録は取っていない） |

## 依頼

9 回目のレビュー（S369〜S372 の変更）の issue のうち S370 の関数の評価器（`globe-mode-flat/src/flatten/fneval.rs`）への #148〜#152 を、自分で再現してから直す。誤りなら直さず根拠をコメントする。コミットには `Fixes #n` を 1 件ずつ。各修正に回帰試験を 1 本（直す前は落ちる）。最後に全体の検査（写したバイナリ・flock、基準 `bin-copies/frontend-diff-s370c.json`）で回帰 0（FFT の 4 本が平らなまま）と `mise run wasm:node`（`fneval.mjs` を含む）。並行で S373 が `globe-mode-flat`（主に `flatten.rs`、公開ライブラリの食い違い）を触っている。

## 決めたこと

- **5 件とも再現して直した（見送り 0）**。再現は `target-shared/review9/repro/`（`rec2_*.mo`・`rec3_15.mo`・`mem1.mo`・`arr1_big.mo`・`arr4.mo`・`memo1.mo`・`fr1.mo`〜`fr3.mo`・`mod1.mo`・`ev.sh` の表）。直す前の動きは、試験を先に書いて落ちることで確かめた（#148 の試験は終わらず 40 秒で切った。#149・#152 は assert、#151 は panic）。
- **仕事量は 1 つの予算にした**（#148・#149・#150 の 3 つ目の症状が同じ根）。`fn_steps` を呼び出しごとに戻さず flat で 1 つ、上限は既存の `MAX_ITERS`（release 100 万・試験 1 万）。新しい定数は呼び出し 1 回の重さ `CALL_COST` = 64 だけ（issue の案の「例: 64」）。**予算の大きさは、FFT の 4 本が `realFFTsamplePoints` で使った tick（86・86・668・1244。一時的に `eprintln` で測った）の 0.1% に収まる `MAX_ITERS` のままにした**。最初は `4 * MAX_ITERS`（呼び出し 6 万回・debug で最悪 20 秒）にしていたが、大きすぎるので戻した。debug で 1 tick が約 5 µs（`for` 90 万回で 9.2 秒）で、予算いっぱいで 5 秒前後。release は測っていない。
- **#149 は issue の案から形を変えた**。生きている配列の総数を `MAX_SCALARS` で抑える案をいったん実装し（`fn_live`。`run_fn` で戻す）、配列の確保・複製を要素の数だけ予算に数える #148 の仕組みが累計で同じ上限になると気づいて外した（累計は生きている数以上）。試験は予算の数え方で書き直した（30 万要素 × 3 段は通り、4 段は断る）。トレードオフ: 同じ大きさの配列を何度も確保する関数は累計 100 万要素までしか呼べない。
- **配列は `Rc<Vec<Val>>`**（issue の案の「`Rc<[Val]>`」でなく `Vec`。`Rc::make_mut` で添字の代入をする）。`max`・`min` だけ借りる形にする案もあったが、引数 `g(a, a, …, a)` で呼び出しの前に全部複製される経路が残るので、全部の読みを `Rc` にした。
- **memo は成功したスカラーだけ、鍵は（関数、`heir`、引数の `Debug`）、`cenv` が空のときだけ**。関数の中で引くパッケージの定数は `ctx.cenv`（`redeclare` の差し替え）と `heir` で変わる。`cenv` は `Rc<RefCell<…>>` でインスタンスごとなので、ポインタで鍵にすると解放後に別のものが同じアドレスを取る危険があり、空のときだけにした。深さの上限・予算で断られた `None` は文脈で変わるので覚えない。引数に配列があるとき（鍵が大きくなる）も覚えない。
- **#152 の文字列は「分からない」**。`Val::S` は hash で、`Val` は `Copy`。文字列を持たせる波及が大きいので `<`・`<=`・`>`・`>=` を `None` にした（issue の案の 2 つ目）。NaN を含む比較も `None`（以前は偽・真）。`integer` は収まらない値と NaN で `None`、`floor`・`ceil` は収まらなければ Real のまま。`rem` は整数どうしなら整数を返す（MLS の「引数が Real なら Real、そうでなければ Integer」）。
- **`for` の範囲は配列を作らない**（`Box<dyn Iterator>`）。issue には無いが、反復数の溢れ（#151）の直しの場所で、`MAX_ITERS` 個の `Vec<i64>`（8 MB）を入れ子の `for`・再帰の段数だけ同時に持つ形が #149 と同じ種類のメモリの穴だったので一緒に直した。
- **issue の再現との違い**: #148 の `rec2_N.mo`（同じ引数の 4 本の枝）は memo のおかげで 0.04 秒で正しく通る（大きさは 1）ので、issue の「断ること」でなく、試験は引数が毎回違う形（`g(n - 1, 4*k + j)`）で書いた。同じ引数の形は「16 回で済んで通る」試験にした。
- **コミットは 1 つ**にした（`Fixes #148`〜`#152` を 1 行ずつ）。#148・#149・#150 は同じ予算（`tick_n`・`run_fn`）、#150 と #152 は `extremum` の形、#150 と #151 は `for_vals` が同じ関数で、hunk が絡んで 1 件ずつに切ると途中のコミットがビルドできない。S372 は issue ごとに切ったが、今回は切れなかった。

## やったこと

- `globe-mode-flat/src/flatten/fneval.rs`: `tick_n`（重み付き）・`CALL_COST`・`run_fn`（memo・呼び出しの重さ・`fn_blown` を廃止）・`V::A` を `Rc` に・`MemoKey`・`for_vals`（lazy・`checked_add`）・`max`/`min`（走査を数える）・配列の宣言と代入の確保を数える・`builtin`（整数のまま・`int_or_real`）・`extremum`（`&[Val]`・`ord`）。
- `globe-mode-flat/src/flatten.rs`: `Val` に `Debug`、`F` の `fn_blown` を `fn_memo` に、`ord`（整数どうしは整数、NaN・文字列は `None`）、`binop` の比較、`extremum(&xs, …)`。試験 `function_work_is_budgeted_across_calls`・`function_for_range_count_does_not_overflow`。
- `globe-mode-flat/tests/flat.rs`: 試験 `function_local_arrays_have_a_total_cap`・`integer_constants_do_not_pass_through_f64`。
- 文書: ADR-0039（付記 S370 の上限と shortcut に S374 を足し、付記 S374）・roadmap・architecture・CHANGELOG・セッション文書・索引・`log/review/2026-11-16-s369-s372-review9.md`。

## 検証

- `cargo fmt --check`・`cargo clippy -p globe-mode-flat --all-targets -- -D warnings`・`cargo test -p globe-mode-flat`（単体 10 と `tests/flat.rs` 59 ほか）が通る。依存の変更は無いので `cargo deny` は回していない（`mise run check` の全体は、他の担当との RAM・負荷を避けて回していない）。
- 試験が直す前に落ちること: 4 本とも、変更前のコードに試験だけ当てて確かめた。`function_for_range_count_does_not_overflow` は `fneval.rs:524` の `attempt to add with overflow`、`function_local_arrays_have_a_total_cap` は 4 段が `Ok(4)`、`integer_constants_do_not_pass_through_f64` は `mod(9007199254740993, 3) + 1` が 4（MLS は 1）、`function_work_is_budgeted_across_calls` は終わらず 40 秒で切った。memo の部分（同じ引数の 4 本の枝・`n = w(2)` を 20 か所）は、予算だけで memo を外すと落ちる（呼び出しごとに戻す前の道では通るので、この部分だけは「直す前に落ちる」でなく「memo を外すと落ちる」）。
- 再現の数字（debug、`globe check --frontend native`。直す前は issue の値）: `rec2_10.mo` 84.7 秒 → 0.04 秒、`rec3_15.mo` 55.7 秒 → 0.04 秒（どちらも同じ引数の枝で、memo により大きさ 1 で通る）、`mem1.mo`（RSS 709 MB）→ `non-constant-dim` で 0.14 秒、`arr1_big.mo`（`max(a)` が 8.2 秒）→ 断る 0.08 秒、`arr4.mo`（`a[1]`）通る 0.07 秒、`memo1.mo`（8 か所の使用）7.3 秒 → 1.0 秒（1 か所は 0.9 秒）、`fr1.mo`〜`fr3.mo`（panic）→ `non-constant-dim` で断る、`mod1.mo`（panic）→ 通る。
- FFT の 4 本の仕事量（`GM_FNSTEPS` で一時的に測った。コードには残していない）: `RealFFT1` 86・`RealFFT2` 86・`Rectifier6pulseFFT` 668・`Rectifier12pulseFFT` 1244（予算は 100 万）。
- 全体の検査（`msl:diff-frontend` 相当。写したバイナリ `xtask-s374`・`flock`・基準 `frontend-diff-s370c.json`）: 872 s（`flock` の待ちを除く。待ちを入れた wall clock は 25 分）、最大 RSS 6.1 GiB（`/usr/bin/time` で 6.36 GB）。**回帰 0 本・改善 0 本**。MSL 566 本: parse 566/566・resolve 566/566・flat 565/566・名前 529/532・方程式の数 517/532・DAE 565/565・判定 382/382・数 381/382・状態 381/382・構造の正則 382/382（S370 と同じ。FFT の 4 本は平らなまま）。`GlobeModeValidation` 75 本: parse・resolve・flat・DAE 75/75、名前・方程式 74/74、判定 74/74・数 74/74・状態 74/74、構造の正則 73/73。報告は `doc/frontend-diff.md`（基準の JSON の絶対パスは相対に直した）。最初に作った写し（呼び出しの重さを `run_fn` で数える版）は、`external` などで断る関数でも 64 を数えるのを直してから作り直した写しに入れ替えて回した（回す前に入れ替えた）。
- `wasm:node`: `mise run wasm:node` の中身を、自分の wasm（`target-shared/s374/wasm-pkg`。共有の `wasm-pkg` は他の担当のビルドで入れ替わるので使わない。`CARGO_TARGET_DIR=…/s374` で `node` の各スクリプトを回した）で回した: `smoke.mjs --msl data/msl` 通る（panic の文言も）、`broken-chunks.mjs` 通る、`fneval.mjs --msl data/msl` 通る（Node の既定のスタックと `--stack-size=100` の両方。FFT の MSL の本文が native の検査で S0〜S6 を通り、再帰 16 段 × 括弧 16 段 × `if` 8 段の終わらない再帰も溢れず誤りで返る）、`split.mjs --limit 5` は 5 本とも全部読みと一致。`wasm:compare` は回していない（実行系・`globe-mode-sim` は変えていない）
- `check_docs.py`: デッキ（S374 を含む全部）OK / リンク 文書 1355 件・相対リンク 5573 件の切れ 0 件。`render_slides.py`（自分のデッキだけ。desktop のライト・ダーク・mobile の 3 つ、8 枚ずつ。スクリーンショットで S02・S05 を目で確かめた。最初の 1 回は他の担当の負荷で `Page.screenshot` が 30 秒で時間切れになり、撮り直した）: デッキ 1 件 / 問題 0 件。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークンの grep）: 0 件。`save_session_log.py --check`（`log/work/`・`log/review/` の 2 枚）: 取りこぼし 0 件。

## 残したこと

- 文字列の `<` の辞書順（`Val` に文字列を持たせる）。
- memo は `redeclare` の文脈（`cenv` が空でない）と、配列を返す関数・配列を引数に取る呼び出しでは働かない。
- release で予算いっぱいまで回した時間は測っていない。実在の大きなライブラリ（Buildings・AixLib）で、1 回の flat の予算（100 万 tick。呼び出しなら約 1.5 万回）が足りなくなる例題が出るかは、`libraries.yml` を回して確かめる（持ち越し。MSL 566 本では回帰 0）。
- `rec2_10.mo` のような同じ引数の再帰は memo で通る。memo が働かない形（`redeclare` の文脈）で同じ再帰を書くと、予算（約 1.5 万回の呼び出し）で断る。

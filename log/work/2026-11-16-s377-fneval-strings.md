# S377 `fneval-strings`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s377-fneval-strings`（main の `3806d109`〔S376 のマージ〕から切った） |
| worktree | `.git-worktree/s377-fneval-strings` |
| Phase | Track B（B3 flat） |
| セッション文書 | `doc/sessions/S377-fneval-strings/` |
| セッション記録 | 専用の記録が `log/local/` に無いので省いた（親から渡された WP。この WP の記録は取っていない） |

## 依頼

自前の flat の定数の評価・関数の評価器（`flatten.rs` の `eval`、`flatten/fneval.rs`。S370・S374・S376。ADR-0039 の付記）の残りを直す。(1) 文字列の `<`・`<=`・`>`・`>=` が「分からない」（S374。`Val::S` は hash で `Val` が `Copy`）を MLS の辞書順にする最小の形（`==`・`<>`・`+`・`String(…)` も同じ表で扱えるか確かめる）。(2) S376 の残り: 入れ子の package の中の関数（`P5.Inner.f`）の定数、関数の中から呼ぶ兄弟の関数の `redeclare function`、`(…)` の修飾つきの package の差し替えを `dyn_const` が見ない。値が黙って違うものは直し、直せないものは「分からない」にする。各修正に回帰試験 1 本（直す前は落ちる。omc があれば `checkModel` の変数の数と比べる）。最後に全体の検査で回帰 0、`mise run wasm:node`（load 10 未満）。ADR-0039 の付記・roadmap・CHANGELOG を直す。今 `globe-mode-flat` を触る WP は他に無い。

## 決めたこと

- **文字列は表の番号にした**（`Val::S(u32)`。issue の「文字列の表を F に持ち、`Val::S` はその索引にする」）。`F::strs`（`Strs`: `RefCell` の `Vec<Rc<str>>` と `HashMap<Rc<str>, u32>`）。同じ中身は同じ番号なので `==`・`<>` は番号の比べのまま（hash の衝突も無くなった）。`binop`・`ord`・`extremum` に表を渡す。別案（`Val::S(&'static str)` を `Box::leak` で作る）は、長く動く LSP で文字列が溜まり続けるので採らない。
- **辞書順は UTF-8 のバイト順**（Rust の `str::cmp`）。omc の `checkModel` の向きと一致した（`"Z" < "a"`・`"ab" < "abc"`・`String(-3) < "-2"` など 15 本）。非 ASCII は omc が字句解析で受けなかったので、Rust の符号位置順を信じただけ（【未確認】）。MLS の条文は読んでいない（【未確認】）。
- **エスケープを解いてから表に入れる**（`unquote`）。字面のまま入れると `"\t" < "Z"` の向きが逆になる（字面の `\t` は 0x5C で `Z` の 0x5A より大きいが、解いた 0x09 は小さい。omc は後者）。
- **`+` は連結、ほかの算術と文字列と数の混ぜは「分からない」**。以前は `(K::Plus, S, S)` が `R(hash の和)` になる誤りが隠れていた（`-"a"`・`abs("a")` も hash を数として計算した）。連結の長さは 64 KiB まで（`c_i = c_{i-1} + c_{i-1}` の連鎖が 2^34 バイトを作らない。試験が 34 段で断ることを確かめる）。
- **`String(…)` は整数・論理・列挙の要素だけ**（要素の名前。omc で `"green"` と一致）。実数の `String(1.5)` は書式（`significantDigits`）が要るので「分からない」。名前つきの引数（`minimumLength`）がある呼び出しも今までどおり「分からない」。関数の本体の式（`fx`）にも同じ分岐を置いた。
- **`P5.Inner.f` は `via_class`（名前の前半を丸ごと 1 つ解決）をやめ、名前の先頭から 1 つずつ解決した列（`prefix_class`）を、関数の外側のクラスの列と末尾から合わせる**（`reaching`）。`P5.Inner` は `Inner` 自身に解決されて受け継ぎが無く、`PP` を受け継ぐのは `P5` の位置だから。外側ごとに別の package が受け継いでいて 1 つに決まらなければ「分からない」（黙って違う値を出さない）。`redeclare package P = P5` の `P.Inner.f` は差し替え先から引いた列で同じ。`import` の別名を通した名前は外側を名前から引けない（shortcut）。
- **兄弟の関数の `redeclare` は `Ctx` に `inst`（呼び出したインスタンスの系統）を足して引く**。別案（`FN` の文脈でも `cenv` に名前があれば引く）は、別のインスタンスから静的に呼ぶ `Sub.f` に、そのインスタンスの `g` の差し替えが当たってしまうので採らない（試験の後半で確かめる）。`Ctx` の作る場所が 13 あるので `inst: None`／`ctx.inst.clone()` を足した。`fn_memo` の鍵には、差し替えがあるときだけ `inst` の hash を足す。
- **`dyn_const` は戻り値を二重の `Option` にした**。内側の `None`（差し替えられているが決まらない）を外側にすると静的な参照に落ちて、宣言した側の値を返してしまう（試験の `P.m` が 2 のまま）。`reaches_mod` は束縛をたどって、修飾された名前・関数の呼び出し（組み込みを除く）を含むか調べる保守的な判定。MSL に `redeclare package … = X(…)` の形は 0 件（grep）。
- **コミット**: 4 件とも `eval`・`call_target`・`Ctx` に絡むので、1 コミットにまとめた（途中の WIP コミットは 1 本）。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `Val::S(u32)`・`StrTab`・`Strs`（`intern`・`get`・`cmp`・`cat`）・`unquote`（新）、`F::strs`、`ord`・`binop` が表を取る、`string_of`（新）、`eval` の `String(…)`・文字列リテラル、`Ctx::inst`（新。13 か所）、`prefix_class`（新）・`via_class`（`prefix_class` を使う）、`dyn_const`（二重の `Option`、`extra`）・`reaches_mod`（新）、`size` の分岐。試験 4 本。
- `globe-mode-flat/src/flatten/fneval.rs`: `call_target`（名前が通った package の列、`Ctx::inst()`）・`reaching`・`inst_hash`（新）、`run_fn` の鍵、`run_body` の `inst`、`builtin`・`unop` が文字列を断る、`extremum` が表を取る、`fx` の `String(…)`。
- `globe-mode-flat/tests/flat.rs`: S374 の `integer_constants_do_not_pass_through_f64` が「`"a" < "b"` は分からない」と期待していたので、辞書順の期待に直した（`"a" < "b"` は 3、`"b" < "a"` は 4、`"a" < 1` は分からない）。
- 文書: ADR-0039（付記 S377）・roadmap・CHANGELOG・architecture（S377 を足した）・セッション文書・索引・`doc/frontend-diff.md`。

## 検証

- 回帰試験（直す前に落ちることを確かめた。4 本とも 1 回の実行で全部落ちた）: `globe-mode-flat` の単体に 4 本（`string_comparison_concat_and_string_function_are_decided`・`nested_package_function_reads_constants_of_the_reaching_package`・`function_calls_the_redeclared_sibling_function`・`parenthesized_package_redeclare_changes_the_constants_it_reaches`）。直す前の値: `"a" < "b"` は `non-constant-dim`、`P5.Inner.f(1)` は 2（6 のはず）、`a.x` は 3（7 のはず）、`P.n` は 1（5 のはず）。
- omc の `checkModel`（1.27.2）との突き合わせ（変数の数）: 文字列 15 本（辞書順・連結・`String`・エスケープ。非 ASCII の 1 本と、引用符を誤った 1 本は除いた）、`P5.Inner.f`（`A1` 6・`A2` 2・`A3` 12・`A4` 24 = 6 + 12 + 2 + 4）、兄弟の `redeclare`（`B1` 10 = 7 + 3、`B2` 3）、`P = PP(n = 5)`（`C1` の `P.n` は 5、宣言なしの `C2` は 1）が、自前の試験の値と全部一致した。`P.m`（`m = n + 1`）は omc が 6 で、自前は「分からない」（決めない）。
- コードの検査: `mise run check` 通った（`python3 .claude/hooks/test_save_session_log.py`・`test_ci_plan.py`・`cargo fmt --all --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`cargo test --workspace`〔105 組の結果、1118 本が通り 0 本が失敗。S376 の 1114 本に新しい試験 4 本〕・`cargo deny check`〔advisories・bans・licenses・sources ok〕・`globe fmt --check src/Modelica`・`uv sync --locked`、exit 0。load は 2〜10）。`globe-mode-flat` の単体は 20 本・`tests/flat.rs` は 59 本が通る（S374 の `integer_constants_do_not_pass_through_f64` の期待を辞書順に直した）。
- 全体の検査（`msl:diff-frontend` 相当。写したバイナリ `xtask-s377`・`flock`・基準 `frontend-diff-s376.json`）: 887 s（wall clock 14:50）、最大 RSS 5.9 GiB（VmHWM。`/usr/bin/time` で 6.1 GB）。**回帰 0 本・改善 0 本**。641 本の結果が時間と最大の常駐メモリを除いて基準と全部同じ（JSON を突き合わせた）。MSL 566 本: parse 566/566・resolve 566/566・flat 565/566・名前 529/532・方程式の数 517/532・DAE 565/565・判定 382/382・数 381/382・状態 381/382・構造の正則 382/382（FFT の 4 本は平坦化できたまま）。`GlobeModeValidation` 75 本: parse・resolve・flat・DAE 75/75、名前・方程式 74/74、判定・数・状態 74/74、構造の正則 73/73。結果が動かなかったのは予想どおり（文字列の大小・`(…)` つきの package の差し替えを評価に使う例題は MSL に無い）。`doc/frontend-diff.md` は時間と RSS だけが変わった（報告の「前回の結果」の欄に出る基準のパスは、写した置き場の絶対パスになるので、相対の `target/frontend-diff.json` に戻した）。
- `wasm:node`（`mise run wasm:node` の中身を、共有の target の warm な wasm32 で release ビルドし〔fat LTO・codegen-units 1・opt-level s・strip の環境変数は mise と同じ〕、wasm-bindgen の出力は自分の置き場 `target-shared/s377/root/wasm-pkg` に出して回した。共有の `wasm-pkg` は他の担当のビルドで入れ替わるので使わない。load は 1〜2）: `smoke.mjs --msl data/msl` 通る（Decay の bdf・dopri45・auto、`check_with(native)`、panic の文言も）、`broken-chunks.mjs` 通る、`fneval.mjs --msl data/msl` 通る（Node の既定のスタックと `--stack-size=100`。FFT の MSL の本文が native の検査で S0〜S6 を通り、深い再帰も誤りで返る）、`split.mjs --msl data/msl --limit 5` 通る（5 本とも全部読みと同じ）。wasm-bindgen のあとの node 用の `.wasm` は 19,812,320 バイト。
- `check_docs.py`・`render_slides.py`: `check_docs.py`: デッキ（S377 を含む全部）OK / リンク 文書 1377 件・相対リンク 5658 件の切れ 0 件。`render_slides.py`（自分のデッキだけ。desktop のライト・ダーク・mobile の 3 つ、7 枚ずつ。S04 の図と S05 の表のスクリーンショットで余白と図の線のつながりを目で確かめ、S04 の図は右の 3 つの箱へ線を分けて直した）: デッキ 1 件 / 問題 0 件（最初の 1 回は `Page.screenshot` が 30 秒で時間切れになり、撮り直した）。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークンの grep）と `save_session_log.py --check`: メールアドレス・ホームのパス・トークンの grep は 0 件。`save_session_log.py --check`（`log/work/` と解説書の 2 枚）: 取りこぼし 0 件。

## 残したこと

- 文字列の変数を持つ関数（`Slot` の型に文字列）、実数の `String(x)` の書式。
- `(…)` の修飾を当てた束縛の再評価（`m` を 6 に決める）。
- `import` の別名を通した入れ子の package の関数（外側の package を名前から引けない）。
- 非 ASCII の大小は omc と突き合わせていない。`String(列挙の要素)` の MLS の条文は【未確認】。

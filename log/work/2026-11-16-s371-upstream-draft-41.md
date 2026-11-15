# S371 `upstream-draft-41`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s371-upstream-draft-41`（base は main の `44b880dd`） |
| worktree | `.git-worktree/s371-upstream-draft-41` |
| Phase | 文書（上流への報告の下書き） |
| セッション文書 | `doc/sessions/S371-upstream-draft-41/` |
| セッション記録 | なし（親のセッションから呼ばれたサブエージェント。`log/local/` に専用の記録が無いので省いた） |

## 依頼

S359 の「39 の備考」の別の限界（部品の中でレコードの配列の欄を切り出すと Rumoca が ED019 `record-array member slice` で止まる。例 `sum(bw.re)`、`bw` は `Complex bw[n]`）を、下書き 41 にする。ユーザーが 2026-11-16 に「ED019 は下書き 41 にして」と指示した。既存の書式（`doc/research/upstream/` の 37〜40、`doc/upstream.md`）で 4 点セット（英語 `.md`・再現の `.mo`・日本語 `_jp.md`・スライド 1 枚の `_jp.html`）を作り、v0.10.2（`data/rumoca`）と v0.10.4（`data/rumoca-latest`）で再現を確かめ、omc で通ることも書く。上流には出さない。Rust のコードは変えない。

## 決めたこと

- **題を「大きさを `size(...)` で決めるとき」にした。** S359 の備考の「部品の中でスライドすると出る・最上位なら通る」は、二分法で成り立たなかった。レコードの配列の大きさがリテラル・`Integer` の parameter（`n = 3`）・`constant`・`2 + 1`・`max`・`min`・`abs`・`div`・`mod` なら通り、`size(...)`・`ndims(...)`・`integer(...)` を含むと、最上位のクラスでも落ちる。参照の形もスライド `aw.re` に限らず、`aw[1].re`・方程式の左辺の `aw[i].re` で出る（最初の参照を指す）。成分を 1 つも参照しない形は 39 の `ED001`。39 の備考（英語・日本語）に「最初の版は誤っていた」と書いて直し、41 へリンクした。
- 判定は**バグ**。MLS 3.7 の 10.6.9（Slice Operation）の本文をページで読んだ（`a.m` は `{a[1].m, ...}`。レコードの配列ではメンバーを各レコードから選ぶ）。omc 1.27.2 は全部通す。
- **再現の `.mo` は `Rep41` の `A`〜`H`。** 失敗は `A`（スライド）・`B`（1 要素の成分）・`C`（部品）・`D`（左辺）・`E`（`Integer` の parameter が `size(a, 1)`）、対照は `F`（リテラル）・`G`（`n = 3`）・`H`（`Real` の配列）。MSL の `Complex` の形は本文に `ComplexSlice.mo`（`Q`・`K`）として貼った（39 と同じ流儀）。`globe fmt` で整形し、整形の前後で変わらないこと（`--check` が通ること）を確かめた。
- 原因は追っていない。flat は通り DAE を作る段で出ること（`--emit flat-json` は通る。`instance_relations` の `Aggregate` が 1 つ対要素ごと）と、main のソースに同じ文言があること（`record_array_fields.rs` の `component_ancestry`、`b55eafce`）だけを書いた。main の回帰試験は `Pin pin[3]`（リテラル）で、`size(...)` の大きさは無い（ファイルを読んでの推測、回していない）。
- MSL の例題は、2026-10-31 の funnel の結果（v0.10.2）で 2 本。`ShowTransferFunction`（`size(b,1)`、v0.10.4 でも落ちる）と `ShowImpedance`（`parameter RCData rcData[nRC]`、v0.10.2 は落ち v0.10.4 は通る）。後者は別の形で、理由を追っていない。ほかのライブラリは見ていない【未確認】と書いた。
- 提出の順は「—」の行（出すかはユーザーが決める）。出すなら `A`・`B`・`F`・`G` の 4 本。

## やったこと

- `doc/research/upstream/41-ed019-record-array-member-slice-dimension-from-size-call.{md,mo}`・`_jp.md`・`_jp.html`（4 点セット）。
- `doc/upstream.md`: 英語の要約（45 → 46 件、41 の説明、37〜41 の日本語版）、冒頭に S371、一覧に 41 の行、提出の順の「—」の行、39 の行の文言（ED019 を下書き 41 に）、回避の段落に 41。
- `doc/research/upstream/README.md`: S371 の追記、日本語版の範囲、一覧に 41 の行、39 の途中の限界の文言、「レコードの配列の成分の ED019（41、S371）」の節。
- `doc/research/upstream/39-…md`・`_jp.md`: 備考を 41 へのリンクと訂正に書き換えた。
- `doc/sessions/S371-upstream-draft-41/`（スライドと解説書）、`doc/sessions/README.md`、この記録。

## 検証

- Rumoca の CLI（`rumoca 0.10.2`・`rumoca 0.10.4`）で `Rep41` の 8 モデル × 2 版、`ComplexSlice.mo` の `Q`・`K` × 2 版、MSL の `ShowTransferFunction`・`ShowImpedance` × 2 版。`A`〜`E`・`Q`・`ShowTransferFunction` が両版で ED019、対照の `F`・`G`・`H`・`K` は両版で通る。`ShowImpedance` は v0.10.2 が ED019、v0.10.4 が通る（`--emit dae-json` が終了コード 0）。
- 大きさの変種（`A` の本体で大きさだけを変えたもの）を 2 版で回した。リテラル・`Integer`・`constant`・`2 + 1`・`max`・`min`・`abs`・`div`・`mod` は通り、`size(...)`（`a` が parameter・constant・固定の大きさ）・`ndims`・`integer` は落ちる。使った変種は本文に書いた（`.mo` に入れたのは `A`・`E`・`F`・`G`）。
- omc 1.27.2 の `checkModel`: `A`〜`H` が 7 と 7（`D` は 6 と 6）、`Q`・`K` が 7 と 7、`ShowTransferFunction` が 29 と 29、`ShowImpedance` が 4 と 4。
- 最新のリリースが v0.10.4（2026-11-02）、main が `b55eafce`（2026-11-13）のままであることを `gh api` で確かめた（2026-11-16）。ソースは GitHub の contents API で読んだ。ビルドしていない。
- `check_docs.py`: 全デッキ OK（S371 を含む）、文書 1,341 件・相対リンク 5,540 件・切れ 0 件。`render_slides.py`（S371 のデッキと 41 の `_jp.html`）: デッキ 2 件・問題 0 件（`_jp.html` は題を 1 行に縮め、表の下に 200 px 以上の余白を残した。最初の撮影は Chromium の最初のスクリーンショットが約 90 s かかって 30 s で時間切れになった（40 の `_jp.html` も同じ。フォントの初回の読み込み）。2 回目からは 0.1 s で、回し直して通った）。
- 回していないもの: `mise run check`（Rust・依存は変えていない）、markdownlint（入っていない。見出し・リスト・表・コードブロックの前後は空行にした）、セッション記録の `--check`（記録を作っていない）、`msl:*`（コードを変えていない）。

## 残したこと

- 提出するかはユーザーが決める。出すなら Rumoca の main で再現を確かめてから（main はビルドしていない）。
- なぜ大きさの書き方で結果が変わるかは追っていない（`size`・`ndims`・`integer` で落ち、`max`・`div` などで通る違い）。
- ほかのライブラリ（Buildings・AixLib など）で同じメッセージが出る例題の数は見ていない【未確認】。
- GlobeMode の Rumoca の rev を v0.10.4 に上げる版の最初に、`ShowTransferFunction`・`ShowImpedance` が `msl:funnel` でどう変わるかを見る（`ShowImpedance` は通る見込み）。

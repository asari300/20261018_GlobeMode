# S359 `upstream-drafts`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s359-upstream-drafts`（base は main の `ce38ea4f`） |
| worktree | `.git-worktree/s359-upstream-drafts` |
| Phase | 文書（上流への報告の下書き） |
| セッション文書 | `doc/sessions/S359-upstream-drafts/` |
| セッション記録 | なし（親のセッションから呼ばれたサブエージェント。`log/local/` に専用の記録が無いので省いた） |

## 依頼

今日のセッションで出た「上流（Rumoca）への依頼の候補」3 件を、`doc/upstream.md` の流儀の下書き（4 点セット: 英語 `.md`・再現の `.mo`・日本語 `_jp.md`・スライド 1 枚の `_jp.html`）にする。(1) `rk45::simulate_artifact` が `pub(crate)`（GlobeMode の #46、S344・ADR-0041）、(2) 初期化済みの状態を受け取る口が無く初期化を 3 回解く（#45、S226・S344）、(3) `Complex aw[3]` への `aw = {…}`・`aw = bw` を Rumoca が釣り合いまで通せない（S351）。v0.10.2 と v0.10.4 の両方で確かめ、`doc/upstream.md` の一覧に足す。出さない（issue も PR も作らない）。Rust のコードは変えない。

## 決めたこと

- 番号は 37・38・39（36 の次）。37・38 は API の要望（バグでなく、結果は誤っていない）、39 は不具合。本文の先頭の Verdict にそう書いた。
- **37・38 は実行・コンパイル・計測をしていない。** Rumoca のソース（git の checkout `276e82b`・`9c8c56f`、main は `b55eafce` を GitHub の contents API で）を読み、行番号を付けた。CLI で動くことの確認（`rumoca sim` が両版で 501 点）はしたが、重複した lowering・初期化は CLI では見えない。費用の数字は ADR-0017（S015、v0.10.2）の lowering の秒だけを引用し、2 回目の費用は測っていないと書いた。38 は費用が無いので、提出の順で 15 位の「残り」に置いた。
- **39 は S351 の言い方を直した。** S351 は「`Complex aw[3]` への `aw = {…}` は Rumoca が釣り合いまで通らない」としたが、二分法（レコードの種類 × 大きさの書き方 × 右辺の書き方の 8 形 × 2 版）で、次が分かった。(a) 普通のレコードは両版で失敗（v0.10.2 は大きさ 3 で ED008、`size(a, 1)` で ED001、v0.10.4 はどちらも ED001 の 3 本対 6 個）。(b) MSL の `Complex.mo`（operator record）は、v0.10.4 で大きさ 3 の形が通る（v0.10.2 は ED008）。大きさが `size(a, 1)` の形は両版で ED001。v0.10.4 の flatten に `operator_records/arrays.rs`（v0.10.2 に無い）があるためと見られる【推測。追っていない】。(c) for 文の形と、もう一方を for 文で決める `aw = bw` は両版で通る。omc 1.27.2 の `checkModel` は全部 6 と 6（`C` は 12 と 12）。
- **MSL の `ShowTransferFunction` は、方程式の数より前に別の限界で止まる**（S351 の「0/1」はそれが先に当たったと見られる）。`Modelica.ComplexBlocks.ComplexMath.TransferFunction` の `sum(bw.re)` で、両版とも ED019 `record-array member slice`。部品の中でレコードの配列の成分を切り出すと出る（最上位のクラスなら通る。omc は通す）。最小の再現を作ったが、依頼が 3 件なので別の下書き（40）にはせず、39 の備考と解説書に書いた。作るかは親・ユーザーが決める。
- 再現用の `.mo` は `globe fmt` で整形した（`annotation` がモデルの終わりに寄る）。整形後も両版で回ることを確かめた。

## やったこと

- `doc/research/upstream/37-…`・`38-…`・`39-…` の 4 点セット（`.md`・`.mo`・`_jp.md`・`_jp.html`）、計 12 ファイル。
- `doc/upstream.md`: 英語の要約（41 → 44 件）、一覧に 37〜39 の行、提出の順（15 位に 37・38、39 は「—」の行）、回避を外す条件の下の注意書き、冒頭に S359 の確かめ方。
- `doc/research/upstream/README.md`: 件数・日本語版の範囲、一覧に 37〜39 の行、「実行系の API の要望 2 件とレコードの配列式（37〜39、S359）」の節。
- `doc/sessions/S359-upstream-drafts/`（スライドと解説書）、`doc/sessions/README.md`、この記録。

## 検証

- 39: 6 モデル（`A`〜`F`）× 2 版、MSL の `Complex` の 2 形（`L`・`P`）× 2 版、omc 1.27.2 の `checkModel`（上の結果）。
- 37・38: 両版の `rumoca sim`（`rk-like`・`bdf`）が 501 点で終わる。ソースは `rk45.rs`・`diffsol.rs` の行番号で突き合わせた（v0.10.2 `276e82bd`・v0.10.4 `9c8c56fc`・main `b55eafce`）。最新のリリースが v0.10.4 のままであることを `git ls-remote` と releases の API で確かめた（2026-11-16）。
- MLS 3.7 の 10.6.1・4.8（Definition 4.4）は WebFetch の要約を通して読んだ（節の番号と規則。全文は読んでいない）と本文に書いた。
- `check_docs.py`: 全デッキ OK（S359 を含む）、文書 1,288 件・相対リンク 5,386 件・切れ 0 件。`render_slides.py`（S359 のデッキと 37〜39 の `_jp.html` の 4 件。load 18〜26 の下で回した）: 問題 0 件。
- 回していないもの: `mise run check`（Rust・依存は変えていない）、markdownlint（入っていない。見出し・リスト・表・コードブロックの前後は空行にした）、セッション記録の `--check`（記録を作っていない）。ステージした差分のメールアドレス・ホームのパス・トークンの検索: 0 件。

## 残したこと

- 37・38 のコンパイル（`E0603`）と、重複した lowering・初期化の費用の測定はしていない。出す前に、Rumoca の main で確かめ、費用を測るかを決める。
- 39 の 3 という数の出どころ（flat の `scalar_count` 3 の残差 1 本）は追っていない。`ED019 record-array member slice` は別の下書きにしていない。
- Rumoca main はビルドしていない（37・38 はソースだけ、39 は未確認）。
- `doc/msl-*.md`・`frontend-diff.md` は触っていない（報告）。GlobeMode の Rumoca の rev を v0.10.4 に上げるときに、39 の `Complex` の形が `msl:diff-frontend` でどう変わるかを見る。

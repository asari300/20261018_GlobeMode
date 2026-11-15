# S335 `flat-unsized-array`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s335-flat-unsized-array`（`main`〔S334 のマージ後〕から切った） |
| worktree | `.git-worktree/s335-flat-unsized-array` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S335-flat-unsized-array/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

自前の flat が MSL の例題 28 本を `unsized-array` で断っている。大きさ `[:]` の配列の大きさを、修飾・束縛式・`size()`・接続の相手から決めて平らにできるようにする（MLS 10.1）。Rumoca の flat と変数の名前・方程式の数を突き合わせて一致させる。回帰 0 が条件。S336（`composite-binding`）が同時に `globe-mode-flat` を直している。

## 決めたこと

- 28 本を `--dump` の最初の理由で原因ごとに分けた: `BooleanTable` の `table=`（12）、`vector([zeros(..); b])`（6）、レコードの束縛の欄 `OCV_SOC`（7）、関数の戻り値（2）、ファイルを読む `readRealMatrix`（1）。
- 直しは `dims_of`（式の形）と、束縛の欄の引き方（`bind_dims`・`field_bind`）の 2 か所。関数の戻り値は、整数の入力を引数の値に束ねた文脈（`Ctx::pfx = FN`）で出力の添字を評価する。
- 宣言が後ろの部品の欄は型の宣言から決めるが、**宣言の場所で修飾された欄を直接使うときは決めない**。最初は既定値で決めて試験が黙って誤った大きさを返したので、`mentions` で止めた。
- `ReadRealMatrixFromFile` は決めない（ファイルを読む外部関数）。
- `rcData[] = cellData*`（5 本）は `eval` が suffix の欄を評価するようになった副産物で `non-constant-dim` から `composite-binding` に進んだ。S336 は `Complex` の配列を要素ごとにしたが、この形（配列のレコードを別のレコードの欄の配列で束縛）は残った。
- 最初の全体の検査は回帰 52 本（Machines・FundamentalWave の方程式の数が少ない）を出した。`SoftStarter` の 8 本の差も同じ原因で、`--class` で `SoftStarter` だけ確かめて「Rumoca の数え方の誤りかもしれない」と読みかけたのは誤りだった（単体の部品が一致していても、組み合わせの中の差は自前の誤りのことがある）。原因は `var_dims` に足した「宣言が後ろの部品の欄」の補助が、平らにしたあとも残る `pending` の部品の配列（`plug_p.pin`）を未処理と見て、`plug_p.pin.i` を要素の数をかけずに 1 本と数えたこと。部品が配列でなく、`tgt.` の変数がまだ無いときだけ使う（`not_instantiated`）。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `dims_of`（添字つき参照・内包・`vector`・要素ごとの関数・`if` の枝の決め方・定数でない `parameter`）、`fn_frame`・`class_out_dims`（整数の入力）、`bind_dims`・`field_bind`、`pending_bind`・`pending_type`・`pkg_field_dims`（宣言が後ろの部品の欄）、`eval` の suffix の欄。
- `globe-mode-flat/tests/flat.rs`: 試験 1 本。
- 文書: ADR-0039 付記（S335）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

- コードの検査（変えた crate だけ。S336 を取り込んだ後にも通し直した）: `cargo fmt --check` OK、`cargo clippy -p globe-mode-flat -p globe-mode-dae -p globe-mode-resolve -p globe-mode-syntax --all-targets -- -D warnings` 0 件、`cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend -p globe-mode-lsp` すべて通過（flat の試験は S337 の分と合わせて 37 本）。依存は変えていないので `cargo deny` は回していない。`mise run check`（workspace 全体）は同時の担当が多く、回していない。
- `msl:diff-frontend`（`xtask` をビルド直後に `bin-copies/` へコピーし、`flock` の下で同じ引数）。
  - **S336 を取り込む前**（S337・S341 を取り込んだ後。前回の JSON は S334 のもの）: 609 s、5.5 GiB、回帰 0、改善 120 本。平らにできた 471 → 493、名前 484/484、方程式の数 473/484、DAE 493/493、Rumoca の釣り合いまで通った 351 本で判定・数・状態・正則 351/351。
  - **S336（と S339・S351）を取り込んだ後**（前回の JSON は S336 のもの）: **883 s、最大 RSS 5.8 GiB、回帰 0 本、改善 41 本**。自前で平らにできた 477 → **518**/566、Rumoca も通った 491 本で名前 **491/491**・方程式の数 **480/491**（合わない 11 本は Rumoca の数え方の誤り）、DAE **518/518**、Rumoca の釣り合いまで通った 351 本で判定・数・状態・構造の正則が **351/351**、resolve 566/566。`GlobeModeValidation` 75 本は変わらない。断る理由の残りは `stream` 21・`connect` 9・`composite-binding` 6・`expandable` 5・`non-constant-dim` 4・`connect-array` 1・`non-constant-for` 1・`unsized-array` 1（計 48 本）。
  - S336 の担当が心配していた `plug_p.pin.i + plug_n.pin.i = fill(Complex(0), m)` の数が 3 → 1 に減る問題は、合わせても起きていない（Machines の 50 本の方程式の数が一致）。
- その前の 2 回の全体の検査: 1 回目は回帰 52 本（上の `not_instantiated` で直した）、2 回目は S341 の resolve が共有 target で混ざった（flat・DAE の数字は同じ）。
- `unsized-array` の 28 本（S334 の時点）: 20 本が平らになり、Rumoca と一致（19 本 + `SoftStarter`）。残りは `composite-binding` 6（`rcData[] = cellData*` 5 本と `CCCV_Stack`）・`expandable` 1・`unsized-array` 1。
- `check_docs.py`: デッキ OK / リンク 1243 件中の切れ 0 件。`render_slides.py`（S335 のデッキ）: 問題 0 件。
- セッション記録の `--check`: この WP 専用の記録が無いのでコピーなし（0 件）。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件。

## 残したこと

- `rcData[] = cellData*`（5 本）と `CCCV_Stack`（`composite-binding`）、`CCCV_Cell` の `expandable`、`ReadRealMatrixFromFile`。
- 状態の数 3 本（`IMC_*`、S337）。
- 共有 target の `release/xtask` を別の worktree が上書きして、古い結果（直したはずの例題が全部断られる）を 2 回見た。変えた crate の `lib.rs` を touch して作り直し、`release/xtask` をコピーして回すと安定した。

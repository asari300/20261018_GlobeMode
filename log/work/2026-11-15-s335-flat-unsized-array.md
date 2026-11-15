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
- `rcData[] = cellData`（5 本）は `eval` が suffix の欄を評価するようになった副産物で `non-constant-dim` から `composite-binding` に進んだ。S336 の担当なので触らない。
- `SoftStarter` の方程式の数が Rumoca と 8 本違う（自前が少ない）。単体で平らにした部品は Rumoca と一致するので、組み合わせた中の差と見ているが原因は未確認。Rumoca の数え方の誤り（S330 の表の数え）と同じ型かは確かめていない。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `dims_of`（添字つき参照・内包・`vector`・要素ごとの関数・`if` の枝の決め方・定数でない `parameter`）、`fn_frame`・`class_out_dims`（整数の入力）、`bind_dims`・`field_bind`、`pending_bind`・`pending_type`・`pkg_field_dims`（宣言が後ろの部品の欄）、`eval` の suffix の欄。
- `globe-mode-flat/tests/flat.rs`: 試験 1 本。
- 文書: ADR-0039 付記（S335）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

@@VERIFY@@

## 残したこと

- `rcData[] = cellData`（5 本、S336）、`CCCV_Cell` の `expandable`、`ReadRealMatrixFromFile`。
- `SoftStarter` の方程式の数の差 8 本。
- 共有 target の `release/xtask` を別の worktree が上書きして、古い結果（直したはずの例題が全部断られる）を 2 回見た。変えた crate の `lib.rs` を touch して作り直し、`release/xtask` をコピーして回すと安定した。

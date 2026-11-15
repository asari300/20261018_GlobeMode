# S331 `upstream-table-count`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s331-upstream-table-count` |
| worktree | `.git-worktree/s331-upstream-table-count` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S331-upstream-table-count/` |
| セッション記録 | なし（専用の記録が `log/local/` に無い） |

## 依頼

S330 が見つけた Rumoca の方程式の数え方の誤り（表の大きさが `tau0 = mu0*cgeo*fn` の方程式の数になる）を、上流への issue の下書き（英語・最小の `.mo`）にする。提出はしない。最新の release でも再現するかを確かめる。

## 決めたこと

- 最新の release（v0.10.4、2026-11-15 でも最新）では**再現しない**。下書きは「v0.10.4 で修正（出さない）」の記録にした。慣例（25g・26）に合わせ、日本語版とスライドは作っていない。
- `data/rumoca` は v0.10.2 のままで、v0.10.4 の配布バイナリは scratchpad に置いて確かめた（xtask の固定の版は変えない。rev を上げるのは次の版の最初）。
- 自己完結の `.mo` は flat の `scalar_count` だけを見せる（toy の外部オブジェクトは `compile` の段で `ED019` に当たる）。釣り合いの誤り（`ED001` 7 本・未知数 2）は MSL の `W4` を本文に載せた。

## やったこと

- `doc/research/upstream/36-flat-equation-count-from-external-object-table-size.{md,mo}`（`.mo` は `globe fmt` 済み）。
- `doc/upstream.md` の一覧に 36 の行、英語の要約の件数（40 → 41、修正 6 → 7）、提出の順の表の「v0.10.4 で修正」の行に追記。`doc/research/upstream/README.md` の表・環境・日本語版の注に 36 を追記。
- `doc/sessions/S331-upstream-table-count/`（スライドと解説書）と索引の行。

## 検証

- v0.10.2 と v0.10.4 で `Rep36.A/B/C/D` の `scalar_count`、`W4`、MSL の 10 本の `compile`（ED001 の有無）と `flat-json` の方程式の数を比べた（結果は下書きの Actual・Impact の表）。
- `check_docs.py`・`render_slides.py`（S331 のデッキだけ）の結果は最終報告のとおり。

## 残したこと

- Rumoca main は未ビルド。直した変更の特定も未。
- rev を v0.10.4 に上げたあとの `msl:diff-frontend`・`msl:funnel` は回していない。
- S330 の ADR-0039 の付記（4 点セット未作成と書いてある）は S330 のブランチの側で直す。

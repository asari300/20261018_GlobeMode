# S072 `upstream-second-batch`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s072-upstream-second-batch` |
| worktree | `.git-worktree/s072-upstream-second-batch` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S072-upstream-second-batch/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S070 の「次回」のとおり、ER023・ER003（ClaRa）、EI006（Buildings・IBPSA）、ED019（`function shape proof` ほか）、ED008 について、ライブラリの実際の箇所を 1 つ選び、最小の再現 `.mo`（`Rep22`〜`Rep26`）を書き、`globe check` と omc で確かめ、Rumoca のバグか GlobeMode 側の回避か MLS の規定上の正当な拒否かを判定し、バグは英語の下書き `doc/research/upstream/22〜26-*.md` にする。文書だけで、Rust のコードは変えない。main へのマージ・push は親が行う。

## 決めたこと

- 番号は 22 ER023・23 ER003・24 EI006・25 ED019・26 ED008。`.mo` は 5 つとも置き、`.md` は穴と判定した 24〜26 だけ（既定のまま進めた）。
- 代表の箇所は各コードで最多のメッセージ（EI006 `X_w` 40 本、ED019 `function conditional` 44 本、ED008 `Media.Air.dryair.cp` 47 本）。依頼が名指した ED019 の `function shape proof`（`SensorVerification`）は MSL 4.0.0 で再現したが最小にできず【未確認】として残した。
- ER023 は正当: MLS 3.6 §4.6 は record の成分を record か type に限り、ExternalObject は class。omc が通すのは緩さ。
- ER003 は正当: ClaRa は取得しない依存 SMArtInt（取得すれば避けられる）、AixLib の 1 本は MSL のどの版にも無い名前を引くライブラリの誤り。
- 既存の下書き（01〜21）と重なるものは無かった。

## やったこと

- `doc/research/upstream/22〜26-*.mo`（再現）、`24〜26-*.md`（下書き）、`README.md`（表と S072 の節）。
- `doc/research/libraries-2026-11-01.md` に「2 回目の判定（S072）」の節。
- `doc/sessions/S072-upstream-second-batch/`（スライド・解説書）、`doc/sessions/README.md` に 1 行。

## 検証

- コードの検査: Rust を変えていないので回していない。
- 再現: モデル 17（失敗 9・対照 8）。`globe check` と Rumoca v0.10.2 の CLI のコードは 17/17 一致。omc 1.27.2 の `checkModel` は `Rep23.M` だけを断り 16 を通す。
- 実際の例題: IBPSA の `FMUs.Fan`（EI006）・`WindPressureProfile`（ED019）・`Orifice`（ED008）、ClaRa の `SteamCycle_01`（ER023）、AixLib の `SensorVerification`（MSL 4.0.0、ED019）で同じコード。
- 本数: run 41250636800 の artifact で数え直した。ER023 3・ER003 3・EI006 117・ED019 125・ED008 108。
- `check_docs.py`: デッキ OK、文書 321 件・相対リンク 1568 件・切れ 0 件。
- `render_slides.py`: 1 回試して `Page.screenshot` が 30 s で時間切れ（スクリーンショット 0 枚）。CI に任せる。
- セッション記録の `--check`・漏洩検査: 親が行う。

## 残したこと

- ED019 の `function shape proof`（12 本）と `splineDerivatives`、ED008 のループ変数 `i`（18 本）、EI006 の `X_w`・`x_pTphi` 以外の名前（50 本）は最小にしていない。
- 下書きは Rumoca の main で確かめていない（出す前にユーザーが確かめる）。
- Chemical・DeltaRobot の artifact（run 41248143483）は数え直していない（S063 の上位にこの 5 つのコードは無い）。

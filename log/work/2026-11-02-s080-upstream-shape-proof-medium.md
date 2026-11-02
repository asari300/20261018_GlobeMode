# S080 `upstream-shape-proof-medium`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s080-upstream-shape-proof-medium` |
| worktree | `.git-worktree/s080-upstream-shape-proof-medium` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S080-upstream-shape-proof-medium/` |
| セッション記録 | （親が写す） |

## 依頼

S070・S072 で【未確認】だった 2 つ（ED019 `function shape proof`、EF025 の replaceable な Medium の関数）を二分法で最小の再現にし、`globe check`・Rumoca の CLI・omc で確かめて、既存の下書き 25・20 に追記する。文書だけ。main へのマージ・push は親が行う。

## 決めたこと

- 新しい番号の下書きは作らず、20・25 に「Minimal reproduction」の節を足した。再現ファイルは `20b-*.mo`（`Rep28`）と `25b-*.mo`（`Rep27`）。ファイル名を 27・28 にしなかったのは、README の表で新しい下書きと読まれないため。
- 2 つとも **Rumoca の穴**と判定（MLS 3.6 §10.1・§7.2、§5.3。omc 1.27.2 は全部通す）。
- EF025 の本数を数え直した: libraries の run 41365960542（2026-11-02）で 138 本のうち `loadResource` 136・Medium 2。S070 の「約 85 本」は表の 1 行（51 本）の読み違え。libraries-2026-11-01.md と下書き 20 を直した。

## やったこと

### ED019 `function shape proof` の二分法

| 段 | 形 | 結果 |
| --- | --- | --- |
| 1 | `SensorVerification`（AixLib、MSL 4.0.0）、CLI | ED019（同じメッセージ） |
| 2 | MSL だけ: `MassFlowSource_T` + `TemperatureTwoPort` + `MassFlowRate` + `FixedBoundary`（Medium `ConstantPropertyLiquidWater`） | ED019 |
| 3 | `MassFlowSource_T` + `FixedBoundary` / `Boundary_pT` + 各センサ | 4 通りとも ED019 |
| 4 | `FixedBoundary` だけ / `BaseClasses.PartialSource` を extends するだけ | ED019 |
| 5 | `density_pTX`・`density_phX`・`setState_pTX`・`checkBoundary` を直接呼ぶだけ | 通る |
| 6 | MSL 4.0.0 の写しで `Media/package.mo` 6035・6051・6061 行の `X[nX]` を `X[:]` | 消える |
| 7 | 6035 行（`specificEnthalpy_pTX`、`BaseProperties` から呼ぶ）だけ `X[:]` | 消える → 原因 |
| 8 | 自己完結: 次元 `nX` の関数を `BaseProperties` から呼ぶ（修正なしの extends） | 通る |
| 9 | 8 に `extends SM(names = {"water"})`（`nX` が依存する定数を修正） | ED019 → `Rep27` |
| 10 | 9 の修正をスカラの `extends P(n = 2)` に | ED019（最小の形） |

### EF025 の Medium の二分法

| 段 | 形 | 結果 |
| --- | --- | --- |
| 1 | artifact の JSON で Medium の 2 本を特定（MixtureGases、ControlVolumeEvaporation）、CLI | 2 本とも `multiple exact override package selections` |
| 2 | MSL の 2 つの Medium の `Boundary_pT` を 1 つのモデルに | 通る |
| 3 | 自己完結: 部品ごとに `redeclare package Medium`（2 つ） | 通る |
| 4 | 自己完結: 同じモデルに replaceable な別名 2 つ、基底を修正付きで継ぐ、`BaseProperties` が修飾なしで呼ぶ | EF025 → `Rep28` |
| 5 | 4 の別名を 1 つに / 修正なし / 同じ package 2 回 | 通る |

### ファイル

- `doc/research/upstream/25b-ed019-function-shape-proof-modified-package-constant.mo`、`20b-ef025-two-medium-aliases-modified-extends.mo`（新規）
- `doc/research/upstream/25-*.md`・`20-*.md`（追記）、`README.md`（表の 20・25 の行と S080 の節）
- `doc/research/libraries-2026-11-01.md`（S080 の節、EF025 の本数）
- `doc/sessions/S080-upstream-shape-proof-medium/`、`doc/sessions/README.md`

## 検証

- コードの検査: Rust を変えていないので回していない
- 再現: `globe check`（release、target-shared）と Rumoca v0.10.2 の CLI で 7 本のコードが一致（Rep27.M・Rep28.M が失敗、対照 5 本が通る）。omc 1.27.2 の `checkModel` は 7 本とも completed successfully
- 例題（CLI）: MixtureGases・ControlVolumeEvaporation（MSL 4.1.0）は EF025、SensorVerification（MSL 4.0.0）は ED019 で同じメッセージ
- `check_docs.py`: デッキ全件 OK（NG 0）/ 文書 340 件・相対リンク 1651 件中の切れ 0 件
- `render_slides.py`: 1 回試して時間切れ（screenshot の待ち、機械が重い）。CI に任せる
- セッション記録の `--check`: 親が行う

## 残したこと

- MSL 4.1.0 で出る `function shape proof`（`dependent extent`）の 12 本が 25b と同じ形かは【未確認】。`axis 1 is variable-size` の 5 本は見ていない
- 途中で見えた CLI の別の失敗（継いだ入れ子のモデルを置き換えできない別名で引くと ER002、モデルの配列の次元を package の定数にすると EF024・ET004）は追っていない
- 機械が重く（load average 100〜170）、1 回の検査に数分かかった

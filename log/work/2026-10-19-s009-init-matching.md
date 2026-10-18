# S009 `init-matching`

| | |
| --- | --- |
| 日付 | 2026-10-19 |
| ブランチ | `feature/s009-init-matching` |
| worktree | `.git-worktree/s009-init-matching` |
| Phase | P2 |
| セッション文書 | `doc/sessions/S009-init-matching/` |
| セッション記録 | `log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md`（S005・S007・S008 と同じセッション） |

## 依頼

「s009を進めて」（roadmap の S009 の候補から選ぶ。1 の omc と食い違った 2 本の調査を最初に、3 の S8 の精密化を主な実装にした）。

## 決めたこと

既定のまま進めたこと（ユーザーの指示は「s009を進めて」だけ）:

- **範囲は roadmap の S009 の候補 1（omc と食い違った 2 本の調査）と 3（S8 の精密化）**。1 の調査で `NonCircularPipes` の食い違いが「S8 が数だけで冗長を見られない」ことだったので、3 をそのまま主な実装にした。候補 4（when の区別）は入力の調査が別に要るので送った。
- **`AdvancedSolenoid` は S6 の限界で、直さない**。接地の無い部品の単体の検査（枝 8 本 ≥ 節点 7 つ）で、二部グラフには完全マッチングがあるが、キルヒホッフの電流則が数値的に従属（omc は記号的な簡約の後に `0.0 = 0.0` を見つける）。構造の検査では見えないので、S10（プリフライト）で初期化のヤコビ行列の階数を見る候補にし、検証用のモデル `Invalid.UngroundedLoop` に記録した（枝 < 節点 の `Connect.Subsystem` は構造でも特異で、README の「OK」は S004 の古い記述だったので直した）。
- **S8 のマッチングは「拡張した系（DAE とその微分）+ 初期条件」で組む**（[ADR-0011](../../doc/adr/0011-init-system-matching.md)）。初期方程式だけの小さなグラフでは、拘束で決まる状態の冗長（`NonCircularPipes` の形）を見られない。S7 の Pantelides 法の結果（微分の回数と族の最高階）をそのまま使う。
- **過剰は、過剰な部分に DAE の行があれば Error にしない**（`GM-W8004`）。整合する冗長（MSL の `GenerationOfFMUs` 3 本など）を Error にしないため。omc も Warning で外して進む。
- **初期化で決めるパラメータの参照は `InitStructure` 側に持つ**（`equation_parameter_refs`。そのパラメータがあるときだけ DAE の方程式をもう一度歩く）。`DaeStructure`（S6・S7 の入力）を変えないため。最初のファネルで `InitSpringConstant` に偽の「冗長 1・決まらない 61」が出た（`spring.c` が部品の束縛 `spring.spring.c = c` を通してしか DAE に現れない）ので、束縛を通した依存を閉包で追い、束縛を行にした（`parameter_binding_refs`）。
- **omc が手前で止めるモデル（`GM-W8003` 相当）は S8 の警告のまま**。構造の検査は値を使わないし、部品の単体の検査で値を与えないのは普通のため。
- **Rumoca の制約 2 つ（inner の修正が outer に伝わらない、外からの修正が部品の中の同名の変数に漏れる）は直さず記録する**（`Init.OuterModification`・`Init.ModifierScope`、db のテスト、research/rumoca.md）。GlobeMode 側で直せない（Rumoca の flatten の問題）。上流への報告はユーザーが出す。最初のファネルで omc と食い違った `NonCircularPipes`（初期方程式が消える）と `RollingWheelSet*`（別名の両方が fixed）はこの 2 つで説明がつく。

## やったこと

- `globe-mode-ir`: `InitStructure` に初期方程式の参照（`initial_incidence` / `initial_state_refs` / `initial_parameter_refs` / `initial_discrete_refs`）、`equation_parameter_refs`、`has_incidence`。`without_ranges` でも残す。
- `globe-mode-frontend`: `structure.rs` の未知数の鍵を `unknowns_and_keys` に切り出して `init.rs` と共有。`init.rs` で初期方程式（Residual / Structured / 取り分けた定義）を射影して参照を写す。`fixed = false` のパラメータがあるときだけ連続の方程式ももう一度歩く。テスト 3 本。
- `globe-mode-analysis`: `init_system.rs`（`match_init_system`、`InitMatching`、`InitCondition`、`InitUnknown`。テスト 5 本）、`init.rs` の `analyze_init_system` と `InitReport::matching`、マッチングがあるときの診断（過剰と不足を別々に、冗長の候補と決まらない未知数を名指し）。
- `globe-mode-db`: `init_report` が `dae_shape` と `index_report` も読む。端から端までのテスト 5 本（`Init.RedundantAndMissing`、`Invalid.UngroundedLoop`、マッチングの有無、`Init.OverDetermined` の冗長の候補、inner/outer の記録）。
- CLI: `globe check` の S8 にマッチングの行（方程式 / 未知数 / マッチング、冗長 k・決まらない m）。`--json` の `s8.matching`。
- `xtask`: `msl-funnel` の S8 に冗長・決まらないの数（JSON の `matched` / `extra` / `missing`）、分布の区分を「冗長があれば過剰」に。`omc-compare` の S8 の照合を「過剰 = 冗長が 1 つ以上」に。
- 検証用のモデル: `Init.RedundantAndMissing`、`Init.OuterModification`、`Init.ModifierScope`、`Invalid.UngroundedLoop`。README の表（`Connect.Subsystem` の行も直した）。
- 文書: ADR-0011（新規）、ADR-0009・0010 の追記、architecture / features / roadmap / research（rumoca、openmodelica-dymola）、omc-compare.md と msl-funnel.md の再生成、セッション文書、この記録。
- 調査の切り分け（スクラッチ。コミットしない）: omc の `-d=bltdump` で `AdvancedSolenoid` の `0.0 = 0.0`、`IfInit` 〜 `IfInit4` の 15 通りの小さなモデルで inner/outer の制約を特定。

## 検証

- コードの検査（`mise run check`）: fmt の差分 0、clippy 0 件、テスト 232 本すべて通過（analysis 74・frontend 24 + 20 + 12・db 7 + 11 + 16 + 17 + 13・ir 6・core 13・xtask 19）、`cargo deny` 4 項目 ok、`uv sync --locked` ok。
- MSL の全例題（`cargo xtask msl-funnel`、2 回目。1 回目は束縛の閉包を入れる前）: 566 本: OK 369・NG 13・GM-U 184・panic 0（S008 と同じ）。S8 369 本: 合う 111・過剰 6（すべて GM-W8004）・不足 252。マッチングできた 369 / 369。検査 723.8 s（1 本 1.28 s）、全体 748.4 s、最大 RSS 7.1 GiB（1 回目は 938.1 s / 7.5 GiB）。
- omc との照合（`cargo xtask omc-compare --reuse target/omc-compare-s008.json`。omc は S008 の 566 本の結果を再利用、391 s）: S5 382 / 382、数が同じ 307 / 382、GM-U 184 / 184、S6 362 / 363、S8（過剰）352 / 355（1 回目は 351 / 355。`InitSpringConstant` の偽の冗長を直して 1 本戻った）。
- `mise run msl:compare`（フロントエンドの抽出を変えたので）: MSL の例題 20 本中 20 本が Rumoca の CLI と一致（両方とも成功 17 本）。
- 検証用のモデル（`globe check`）: `Init.RedundantAndMissing` 冗長 1・決まらない 1（GM-W8004 + GM-W8001 + GM-W8002）、`Init.OuterModification` 0 / 2（GM-W8001。Rumoca の制約）、`Init.ModifierScope` 冗長 2（GM-W8004。Rumoca の制約）、`Invalid.UngroundedLoop` OK（S6 16 / 16、限界の記録）、`Connect.Subsystem` S6 で停止（README を直した）。omc 1.27.1: `UngroundedLoop` と `Subsystem` は特異で停止、`RedundantAndMissing` は「Assuming fixed start value for y」。
- `check_docs.py`: デッキ 15 件 OK（S009 のスライドは 15 枚）/ 文書 80 件、相対リンク 439 件中の切れ 0 件
- Playwright（`render_slides.py`）: デッキ 15 件 / 問題 0 件（スクリーンショットを目で確認。最初の版で 4 枚が縦にはみ出し、SVG のラベルが切れたので、サブエージェントが詰めて直した）
- セッション記録の `--check`: 検査 4 件 / 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（37 ファイル。`data/` と `log/local/` は含まない）

## 残したこと

- 離散変数の when の区別（S009 の候補 4 → S010 の 1）。マッチングでは離散変数の列が DAE の行から参照されないので、初期条件の無い離散変数は常に不足に入る（MSL の `MultiBody` の例題で「決まらない 60〜84」の大半は `pre(world.axisColor…)` のような離散変数）。
- 数値的な特異（`UngroundedLoop`、`AdvancedSolenoid`）は S10 のプリフライト（初期化のヤコビ行列の階数）で。
- Rumoca の制約 2 つ（inner/outer、修正の漏れ）の上流への報告はユーザーが出す。直ったら db のテスト 2 本（`outer_modification_is_lost_by_the_compiler`、`modifier_leaks_into_subcomponents_in_the_compiler`）が落ちるので、README と research/rumoca.md の記録を直す。
- `AdvancedSolenoid` の S8（冗長 1・決まらない 3）は、omc が特異で止まるので照合できない。
- omc との S8 の照合で、別名の両方に `fixed = true` があるときの扱い（omc は別名を 1 つにまとめてから数える）。Rumoca の修正の漏れが直れば `RollingWheelSet*` は消えるが、本当に両方に fixed を書いたモデルでは食い違いが残る。
- スクラッチの小さなモデル（`IfInit*`、`Alias*`）はコミットしない。検証用のパッケージには代表の 2 本（`OuterModification`、`ModifierScope`）だけ入れた。

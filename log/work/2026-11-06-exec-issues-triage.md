# S193 `exec-issues-triage`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s193-exec-issues-triage` |
| worktree | `.git-worktree/s193-exec-issues-triage` |
| Phase | P3（実行系の取りこぼし） |
| セッション文書 | `doc/sessions/S193-exec-issues-triage/` |
| セッション記録 | （親が写す） |

## 依頼

2026-11-06 のユーザーの指示「Issue、PR を確認し、必要な修正があればワークツリーで修正」。2026-10-19 に立った実行系の open の issue 8 件（#1・#3・#4・#5・#6・#9・#14・#15）を `gh issue view` で読み、いまの main（0.2.0、S191 まで）で 1 つずつ再現し直す。(a) 直っている、(b) 小さく直せる、(c) 大きい・設計の判断が要る、に振り分ける。issue へのコメント・クローズは親がやる。

## 決めたこと

- #1 は (a)。S024（f2c2807）で `--param` が上書きを組み込んでコンパイルし直すようになり、dopri45 の MAT の補いに使う既定値が上書きの値になった。issue の案（`run_rk` でパラメータを読む）は入れない。CLI に MAT を読み戻す試験を足した。
- #3 は (a)。S021（13a26ab）が diffsol の SDIRK の失敗の上限を外した。試験 `sdirk_survives_more_than_fifty_newton_failures` が既にある。
- #5 は症状が (a)。issue の `GenerationOfFMUs` は、S013（3a09851、ADR-0015）から、ネイティブも session の道も `auto` で同じ `GM-U0004`。MSL を読む試験（`#[ignore]`）を足した。`into_session` の失敗の段（Integration と Initialization）の食い違いは、再現する例が無いので触らず roadmap の表に書いた。
- #9 は (b)（試験だけ）。`fixed_step_shortens_the_last_step_to_land_on_the_stop_time`。
- #14・#15 は (b)。再現し、直す前に試験が落ちることを確かめてから直した。#15 の `radau5` は S030 で実装済み、別名（`DASSL`）は置き換えた先についての警告でよいので、名前が引けなかったときだけ警告を止めた。
- #4 は (c)。issue の案を実装して試したが、`CircleChart`（tsit45・許容誤差 1e-3）で刻みを 1/4 ずつ 10 回縮めても毎回同じ所で失敗し、30 組の失敗で 0 組も救えなかったので、コードは戻した。
- #6 は (c)。ADR-0028 (c) が決めた設計（残りの時間を渡す版は CI で `Lightning…` を 1 本減らしたので採らなかった）。
- ソルバの挙動は変えていない（直したのは設定の警告と MAT の読み込みの誤りだけ）ので、`msl:sim` と `msl-sim.yml` は回していない。

## やったこと

- `globe-mode-sim/src/mat.rs`: `off.checked_add(len)`（#14）と試験。
- `globe-mode-sim/src/settings.rs`: 名前が引けなかったソルバでは刻み・許容誤差の警告を出さない（#15）と試験。
- `globe-mode-exec/tests/integrators.rs`: 最後の短い刻みの試験（#9）。
- `globe-mode-exec/tests/simulate.rs`: `msl_auto_stops_alike_on_both_routes_when_the_directional_derivative_is_unavailable`（`#[ignore]`、#5）。
- `globe-mode-cli/tests/simulate.rs`: `mat_dependent_parameters_follow_an_override_with_dopri45`（#1）。
- 文書: 解説書、スライド、索引、roadmap（一覧と「いま残っているもの」の表）、features.md、CHANGELOG。

## 検証

- 直す前に落ちることを確かめた試験: #14（`attempt to add with overflow`）、#15（`GM-E10005` に `GM-W10003`・`GM-W10001` が付く）。
- `cargo test -p globe-mode-sim`: 108 passed。
- `cargo test -p globe-mode-exec --test integrators -- fixed_step_shortens`: 1 passed。
- `cargo test -p globe-mode-cli --test simulate -- mat_`: 3 passed。
- `mise run check`: 通った（fmt・clippy -D warnings・test・deny〈advisories・bans・licenses・sources ok〉・`globe fmt --check src/Modelica`〈71 ファイル整形済み〉・`uv sync --locked`）。load は 1〜2 から始めた。
- `cargo test --release -p globe-mode-exec --test simulate -- --ignored msl_auto_stops_alike`: 1 passed（14 秒）。
- `msl:sim`・`msl-sim.yml`: 回していない（ソルバの挙動を変えていない。実行できた 312、管に収まった 303 / 311 は変わらない）。
- `mise run wasm:node`: 通った。`mise run wasm:compare`: 387 / 400 件が一致、既知の差 13 件、一致しない 0 件（前と同じ）。
- `check_docs.py`: デッキ OK / リンク切れ 0 件。`render_slides.py`（S193 のデッキだけ）: 7 枚 × 3、問題 0 件。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件
- 全体検査（`ci.yml -f full=true`）: TODO

## 残したこと

- #4: 陽的 RK が chart の折り目を越える刻みを受理する問題（上流の候補 13 と同じ機構）。
- #5: `into_session` の失敗の段の食い違い（例が見つかってから）。
- #6: `auto` の全体の予算（予算の測り方を揃えてから）。
- #9: 断られた試しの点（`fixed.rs:80-88`）と有限でない端の点（`fixed.rs:185-190`）の分岐の試験。

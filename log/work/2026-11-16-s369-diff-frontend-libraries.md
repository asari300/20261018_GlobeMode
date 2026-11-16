# S369 `diff-frontend-libraries`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s369-diff-frontend-libraries` |
| worktree | `.git-worktree/s369-diff-frontend-libraries` |
| Phase | Track B（検査の道具） |
| セッション文書 | `doc/sessions/S369-diff-frontend-libraries/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

`cargo xtask diff-frontend`（S300）は自前の parse → resolve → flat → dae を Rumoca と MSL の例題 566 本・`GlobeModeValidation` で突き合わせる。S365 で自前の flat に上限（反復 100 万 `MAX_ITERS`・スカラー 200 万 `MAX_SCALARS`）を入れたが、MSL でしか確かめていない。MSL 以外の公開ライブラリの検査 `libraries.yml`（S058）は `msl-funnel` で Rumoca の道だけを回し、自前のフロントエンドを通らない。

1. `diff-frontend` に `--library <dir> --package <根> [--examples-in <段>] [--shard i/n]` を足し、公開ライブラリの例題でも自前と Rumoca を突き合わせる（前回の JSON との回帰の判定もライブラリごと）。
2. `libraries.yml` にライブラリごとの `diff-frontend --library …` のステップを `continue-on-error` で足し、報告を artifact に残す。
3. feature を push して `libraries.yml` を 1 回回し、ライブラリごとに、自前で平坦化できた本数・`too-large` で断った本数・Rumoca との一致・panic の有無を書く。

## 決めたこと

- **例題の選び方・`--shard` の意味は `msl-funnel` と同じ関数を使う**（`funnel::library_examples`・`funnel::parse_shard`。後者は `pub(crate)` にしただけ）。`--package` が `Modelica`（既定）なら今まで通り（MSL の例題 + `GlobeModeValidation`）。`Modelica` 以外なら、そのライブラリの例題だけを検査し、MSL は一緒に読むだけ（`GlobeModeValidation` は読まない）。報告・JSON の `group` は `library`。
- **前回の JSON との回帰はライブラリごとの JSON で判定する**。`--json`・`--baseline-json` はファイル名を分けるだけで、回帰の規則（通っていた旗が通らなくなった）は変えない。`--shard` は `skip_json_reason`（一部のモデルだけのとき基準の JSON を書かない規則）の対象にしなかった。分割ごとに別の JSON を書く前提（`libraries.yml`）で、書かないと CI が JSON を出せない。
- **自前の段の panic は 1 本の行にして先へ進む**（`catch_row`。理由 `panic: …` を flat の欄に書く）。MSL 566 本では自前の panic が無かったので素通りだったが、809 本の Buildings で 1 本の panic が分割全体の JSON を捨てると、残りも見られない。Rumoca 側の panic は今まで通り `catch_unwind` で `rumoca_flat` が通らなかった扱い。
- **`too-large` は報告に例題名と理由を出す**（`render_too_large`）。表の「理由」は先頭の語と例を 1 つしか出さないため。
- **`libraries-report` は `-diff.json` を足し算して別の表にした**（既存の表の列は変えない）。funnel・sim・diff の 3 つの JSON を拡張子の前の語で見分ける。
- **ステップの順は funnel → diff-frontend → sim、上限は diff が 100 分**。1 ジョブの上限 360 分に対し funnel 170 + sim 170 があるため、diff を sim の後に置くと間に合わないときに最初に削られる。最初は 60 分にしたが、Buildings の 4 分割目で 51 分かかったので 100 分にした（根拠は下）。
- **MSL の全体の検査は手元で回していない**（指示）。xtask の変更が MSL の結果を変えないことは、`--class` の 3 本（`PID_Controller`・`Rotational.Examples.First`・`CauerLowPassAnalog`）で全部の旗が 3/3 のまま、`MSL` 側の行の作り方は変えていないことで確かめた。
- **自前の flat・DAE と Rumoca の食い違い（13 本）は直していない**。flat は S367（`globe-mode-flat`）の持ち場で、この WP は xtask と workflow だけ。下の表に例題と差を書いた。

## やったこと

- `src/Rust/xtask/src/diff_frontend.rs`: `Options` に `libraries`・`package`・`example_segments`・`shard`。例題・読み込むディレクトリ・Rumoca の `make_checker` をモードで切り替え、`--shard` を `--filter` の後・`--limit` の前に当てる。報告にライブラリの行・`--shard`・ライブラリの節（`{package} の例題`）・`too-large` の節。`catch_row`。試験 3 本（`too_large_examples_are_listed`・`a_panic_becomes_a_row`・`baseline_is_protected` に shard の 1 行）。
- `src/Rust/xtask/src/main.rs`・`funnel.rs`: 引数の定義。`parse_shard` を `pub(crate)`。
- `src/Rust/xtask/src/libs.rs`: `Totals` に diff の列、`add_diff`、表「自前の flat・DAE と Rumoca の突き合わせ」。`totals_add_up_across_shards` に diff の 1 組。
- `.github/workflows/libraries.yml`: ステップ `diff-frontend`（`continue-on-error`、100 分）、要約に「自前の段」の行、artifact に `<id>-diff.md`・`<id>-diff.json`。
- `CLAUDE.md`・`doc/architecture.md`（2 章の xtask・2.5 章の `libraries.yml`）・`doc/roadmap.md`・`CHANGELOG.md`・`doc/adr/0039-native-cst-parser.md`（付記 S369）・`doc/sessions/README.md`・`doc/sessions/S369-diff-frontend-libraries/`・この記録。

## 検証

- 手元（debug。release は回さない）: `cargo fmt --check -p xtask`、`cargo clippy -p xtask --all-targets -- -D warnings`、`cargo test -p xtask` 53 本通過（前は 51 本）。依存は変えていないので `cargo deny` は省く。
- MSL（`--class` の 3 本）: 3/3 のまま。`diff-frontend --library data/libs/Buildings --package Buildings --shard 1/4 --limit 8`（debug）が動き（8 本、flat 5、Rumoca も通ったもの 2 本とも一致）、`--class Buildings.Fluid.Sensors.Examples.EnthalpyFlowRate --dump` で食い違いの flat を印字できた。
- **CI（feature の push、`gh workflow run libraries.yml --ref feature/s369-diff-frontend-libraries`、run 43755915341、全ジョブ success）**: 結果は下の表。自前の panic は 0 本、`too-large` は 0 本。

| ライブラリ | 例題 | 自前で平坦化できた | `too-large` | 自前の panic | Rumoca も flat | 名前の集合が一致 | 方程式の数が一致 | 自前で DAE | Rumoca も釣り合い | 釣り合い・数・状態・正則が一致 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| AixLib | 642 | 314 | 0 | 0 | 183 | 181 | 179 | 314 | 113 | 113・113・113・112 |
| Buildings | 809 | 351 | 0 | 0 | 197 | 197 | 194 | 351 | 116 | 116・116・115・116 |
| Chemical | 76 | 10 | 0 | 0 | 8 | 8 | 8 | 10 | 8 | 8・8・8・8 |
| ClaRa | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | - |
| DeltaRobot | 21 | 13 | 0 | 0 | 9 | 9 | 9 | 13 | 8 | 8・8・8・8 |
| IBPSA | 457 | 249 | 0 | 0 | 147 | 147 | 144 | 249 | 85 | 85・85・85・85 |
| ThermofluidStream | 122 | 117 | 0 | 0 | 25 | 23 | 25 | 117 | 4 | 4・4・4・4 |
| 計 | 2132 | 1054 | 0 | 0 | 569 | 565 | 559 | 1054 | 334 | 334・334・333・333 |

- 平坦化できなかった 1078 本の理由（先頭の語）: `unsized-array` 371、`expandable` 266、`composite-binding` 141、`inner-outer` 63、`redeclare` 58、`connect-array` 58、`non-constant-dim` 53、`connect` 29、`conditional-component` 22、`unresolved` 10、`stream` 5、`non-constant-for` 1、`break` 1。**`expandable`・`composite-binding` の 407 本は S367（main に取り込み済み）が通す範囲**で、この run は S367 の前（base は S366 の main）。上限（`too-large`）で断った例題は 0 本（平坦化できた 1054 本は上限に届かず、残りの 1078 本は別の理由で断られた）。
- Rumoca が panic した例題は、funnel の表（`doc/libraries.md`）の Buildings 4・IBPSA 4。diff-frontend では `rumoca_flat` が通らなかった扱い（ログに `thread 'main' panicked at rumoca-phase-instantiate/src/inheritance.rs:1754:10`）。
- 時間: diff-frontend は ClaRa 5 s、DeltaRobot 11 s、Chemical 16 s、ThermofluidStream 100 s、IBPSA 189 s、AixLib 295 s（2 分割）、Buildings 3426 s（4 分割）。**Buildings の 4 分割目だけで 3080 s（51 分）**。原因は 1 本 `Buildings.BoundaryConditions.GroundTemperature.Examples.CorrectedNFactors`（2963 s。funnel も同じ 2962 s で、Rumoca 側の遅さ。自前の flat・DAE は通り、Rumoca と全部一致）。最大 RSS は Buildings の 2.8 GiB。
- main の取り込み（S367・S368・S371）: `git merge origin/main` は衝突なし。取り込んだあとも `cargo fmt --check -p xtask`・`cargo clippy -p xtask --all-targets -- -D warnings`・`cargo test -p xtask`（53 本）が通る。CI の run は取り込む前の commit（88ab60b0）。
- `check_docs.py`: スライド全部 OK・リンク 文書 1344 件 / 相対リンク 5549 件 / 切れ 0 件。
- `markdownlint-cli2`（MD013 を除く既定）: CLAUDE.md・CHANGELOG・architecture・roadmap・sessions README・新しい解説書・この記録・ADR-0039 で 0 件。
- `render_slides.py`（自分のデッキだけ）: デッキ 1 件 / 問題 0 件（desktop-light・desktop-dark・mobile 各 9 枚）。
- 文書の漏洩検査（メールアドレス・ホームのパス・トークン）: 追加した行で 0 件。

### 自前と Rumoca の食い違い（Rumoca も flat・釣り合いまで通ったもの。13 本 + DAE 2 本。直していない）

| 例題 | 食い違い |
| --- | --- |
| `Buildings.Fluid.Interfaces.Examples.PrescribedOutlet_Water`・`IBPSA.…`・`AixLib.…`（3 ライブラリで同じモデル） | 方程式の数 237 対 242（Rumoca が 5 本多い）。変数は 662 対 662 で名前の差なし |
| `Buildings.Fluid.Sensors.Examples.EnthalpyFlowRate`・`IBPSA.…`・`AixLib.…` | 方程式の数 61 対 63（変数 120 対 120）。`--dump` で見ると、Rumoca の方程式の由来は部品の名前だけ（`equation from senH_flow`）で、自前の `eq[n]` と 1 本ずつは対応づけられていない（原因は未調査） |
| `…Sensors.Examples.MoistAirEnthalpyFlowRate`（3 ライブラリ） | 方程式の数 91 対 93 |
| `AixLib.Airflow.WindowVentilation.Examples.VentilationFlowRateSashOpening`・`…SimpleOpening` | Rumoca にだけ `….openingArea.opnWidth_in` の変数がある（2 個・12 個）。方程式の数は 235 対 237・119 対 119 |
| `ThermofluidStream.Idealized.Examples.TUMExercisesThermodynamicCycles.Exercise3OttoEngine.BaseModel`・`Exercise4DieselEngine.BaseModel` | 自前にだけ `dropOfCommons.assertionLevel`（変数 22 対 21・19 対 18。方程式は 0 対 0） |
| `Buildings.DHC.Plants.Steam.BaseClasses.Examples.BoilerPolynomial`（DAE） | 状態の数 6 対 7（方程式・未知数は 161 対 161） |
| `AixLib.ThermalZones.HighOrder.Components.Examples.WindowsDoors.WindowSimple`（DAE） | 構造の正則: 自前は特異、Rumoca は正則 |

## 残したこと

- 上の 13 本 + DAE 2 本の原因（S367 の持ち場。`PrescribedOutlet_Water`・`EnthalpyFlowRate`・`MoistAirEnthalpyFlowRate` は 3 ライブラリで同じモデルなので、原因は 1 つずつのはず）。
- **S367 が入った main で `libraries.yml` をもう 1 回回す価値がある**。`expandable`・`composite-binding` の 407 本（平坦化できなかった 1078 本の 38%）が通ると、上限（`MAX_ITERS`・`MAX_SCALARS`）に実在のモデルが届くかをまだ確かめていない範囲で確かめられる。1 回の run は Buildings の 4 分割目が長く（funnel 54 分 + diff 51 分 + sim）、全体で約 2 時間（この run は 20:32〜22:32 UTC）。
- Buildings の `CorrectedNFactors`（Rumoca が 2963 s）は上流の Rumoca の遅さ。上流の下書きにするかは未判断。funnel の panic（Buildings 4・IBPSA 4）も Rumoca 側。
- `diff-frontend` の Rumoca の panic は数えていない（`rumoca_flat` が通らなかった扱いに混ざる）。

# S005 check-queries

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/s005-check-queries` |
| worktree | `.git-worktree/s005-check-queries` |
| Phase | P2 遅延クエリと検査（2 回目） |
| セッション文書 | [doc/sessions/S005-check-queries/](../../doc/sessions/S005-check-queries/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md](../sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md) |

## 依頼

「これまでの経緯を確認し、計画をレビューしてください。修正の必要がなければ Auto モードの Opus で継続して S005 の作業をしてください。並列で作業できるようにタスクを分割して作業方針を提案してください。」

- roadmap の S005: `flat` と `check` を salsa のクエリにする（MSL は一度だけ読む）、GM-U の振り分けを決める、S6（完全マッチングと DM 分解）の最初の版、MSL の全例題に判定を出す。
- 計画のレビューで直した点は「決めたこと」に書く。

## 決めたこと

計画のレビュー（ユーザーが承認した計画 `~/.claude/plans/auto-opus-s005-expressive-gosling.md`）で、roadmap の S005 を次のように直した。

- **防火壁 0 を `source_shape`（コメントを除き空白を畳んだ本文）にする。** ADR-0005 は `outline`（Eq）を第 1 の防火壁としていたが、Rumoca の compile に渡せるのは本文であってアウトラインではない。空白だけの編集で `flat` / `check` を止めるには、本文そのものを正規化して Eq で比べるしかない。`outline_shape` は索引とグラフの防火壁として残す。
- **MSL は salsa の外の `Checker`（`Session` を `Arc<Mutex<_>>` で持つ）で一度だけ読む。** Rumoca v0.10.2 の `Session` は Clone でき、`add_document` で差し替え、`remove_document` で消せる。文書を変えると resolve 済みの状態は捨てられるが、parse 結果とモデルごとの LRU は残る。
- **GM-U は A（コード）+ B（一覧）の併用。** S004 では B を既定、C（omc）を任意としたが、調査で「コードだけで未対応 / 内部エラーと決まるもの」（ED019 / ED013 / ED008 …）が分かった。ED001 のように曖昧なコードだけ一覧で決める。omc は入っていないので S005 では照合しない。
- **incidence は自前で組む。** Rumoca の `build_incidence` は `pub(crate)` で使えない。`rumoca-ir-dae` / `rumoca-eval-dae` / `rumoca-core` を同じ rev で frontend に足し（新しい外部依存は増えない）、`for_each_scalar_coordinate` でスカラー単位の（方程式 × 未知数）を作る。Rumoca の `phase_structural::analyze` は突き合わせにだけ使う。
- **例題の定義は Rumoca と同じ名前の規則（566 本）。** `Modelica.*.Examples.*`、途中はすべて package、Utilities / BaseClasses / Internal / Interfaces を除く、partial でない。Rumoca の基準値（parse 566 / flatten 494 / DAE 305 / balanced 293）と比べられるようにするため。
- **並列化は 1 つの worktree で、所有ファイルを分けた Opus のサブエージェント 3 つ。** A: frontend / db / cli、B: analysis と検証モデル、C: xtask と GM-U。共有の型（IR の `DaeStructure` など）は先に凍結した。cargo のロックで直列化されるが、差分ビルドなので支障は小さいと判断した。
- **S6 の判定は `x` と `der(x)` を 1 つの列にまとめた二部グラフで行う（最初の版を直した）。** 最初は Rumoca と同じ der の形（未知数 = `der(x)`・代数・出力）で完全マッチングを調べたが、MSL の全例題を回すと正しい例題 166 本（`Rotational.Examples.First`、`Pendulum`、`PID_Controller` など）が構造特異になった。これらは微分インデックスが 1 より大きい DAE で、der の形では完全マッチングが無いのが正常。Pantelides が失敗する条件は「`x` と `der(x)` をまとめたグラフに完全マッチングが無い」ことなので、S6 はそちらで判定し、der の形の結果は S7 の情報（`GM-I7001`）として出す。IR に `state_refs` を足した。
- **MSL の例題 566 本には partial のモデル 13 本が含まれる。** 計画には「partial でない」と書いたが、Rumoca の基準値の 566 本は partial を含んでいた（名前は一致）。ファネルでも含めて「（partial）」と印を付ける。
- `rumoca-ir-dae` などは Rumoca の内部の crate からも使われるので、cargo-deny の `wrappers` には書けない（書くと Rumoca の内部の依存まで違反になる）。frontend だけが直接依存することは Cargo.toml のレビューで守る。

## やったこと

- **新しい crate `globe-mode-analysis`**（core・ir・serde だけに依存。wasm32 でビルドできる）:
  - Hopcroft–Karp（`hopcroft_karp`）、粗い Dulmage–Mendelsohn 分解（`dulmage_mendelsohn`）、`analyze` と `StructuralReport`。
  - 診断は `GM-E6001`（S6、過剰な方程式と不足している未知数を位置つきで）と `GM-I7001`（S7 の情報）。
- **core**: `normalize`（防火壁 0 の本文の正規化と位置の対応表）、コード `GM-E6001` / `GM-I7001` / `GM-U0001`〜`GM-U0003`、`Diagnostic::{warning, info, is_unsupported}`。
- **ir**: `DaeStructure`（未知数・方程式・`incidence`・`state_refs`、`without_ranges`、`merged_incidence`）、`Unknown`、`EquationRow`、`EquationOrigin`、`FlatSummary`。
- **frontend**:
  - `Checker`（ライブラリを一度だけ読んだ Session を使い回す。panic の後は pristine に戻す。`reset()`、`reference_structure`）。`Frontend::check` は薄い包みにした。
  - `structure.rs`（`for_each_scalar_coordinate` で `DaeStructure` を組む）、`unsupported.rs`（GM-U の振り分け、ADR-0007）。
  - S1〜S5 の診断に位置を付けた。ED001 のときは部品ごとの方程式数を補足に出す。
  - 依存に `rumoca-ir-dae`・`rumoca-eval-dae`・`rumoca-core` を足した（同じ rev）。
- **db**: クエリ `shape` / `source_shape`（防火壁 0）/ `shape_map` / `check` / `flat_summary` / `dae_shape`（防火壁 2）/ `structural`。`GlobeDb::with_libraries`、`with_checker`、`check_model`（S0〜S6 と位置の戻し）。テスト `tests/checks.rs`。
- **cli**: `globe check` を db 経由にし、S6 と GM-U（`unsupported[GM-U…]`）を表示する（`--json` にも）。`globe flat` を足した。
- **xtask**: `msl-funnel`（`funnel.rs`。例題の列挙、`catch_unwind`、`--reset-every`、S6 と Rumoca の照合、Markdown と JSON の報告）。`mise.toml` に `msl:funnel`。
- **Modelica**: `Invalid.StructurallySingular`、`Invalid.HighIndexDuplicate`、`Invalid/package.order`、`src/Modelica/README.md` の表。既知の制約の一覧 `src/Modelica/msl-known-unsupported.txt`（50 行）。
- **CI**: `ci_plan.py` の `WASM_CRATES` に analysis、`EXTRA_INPUTS` に一覧 → frontend。`test_ci_plan.py` に 2 件。`ci.yml` の wasm の行に analysis。
- **依存とライセンス**: `Cargo.toml`、`deny.toml`（wrappers に書けない理由）、`doc/licenses.md` の行。
- **文書**:
  - ADR-0007（GM-U の振り分け）を書き、ADR の索引に足した。
  - ADR-0005 と ADR-0002 に S005 の結果を書き足した。
  - architecture（crate の表、§3.1、§3.2 のクエリの表を実装に合わせて書き直し、S6 / S7 の行、§7）、roadmap（S005 で済んだこと、P2 の進み具合、S006 の候補）、features、research/rumoca（S005 で調べた API）、README の Status、CLAUDE.md（`msl:funnel`）を更新した。
  - S005 のスライド（14 枚）と解説書、セッションの索引。

## 検証

数字で書く。通っていない項目は「通っていない」と書く。

- コードの検査（fmt / clippy / test / deny）: clippy の警告 0。テストは core 13・ir 4・frontend 32（単体 13・結合 19）・db 25（ほかに計測の無視指定 2）・analysis 32・xtask 11。CI の判定のテスト 18 件。cargo-deny は 4 項目とも ok。`mise run check` の通し: fmt の差分 0、clippy の警告 0、テスト 117 件すべて通過（無視指定 2）、cargo-deny 4 項目 ok、`uv sync --locked` ok。
- wasm32: core・ir・frontend・graph・db・analysis の `cargo check --target wasm32-unknown-unknown` が通過。
- 防火壁: 空白とコメントだけの編集（同じファイル・別のファイル）で `check`・`dae_shape`・`structural` は 0 回。パラメータの値の長さを変えると `structural` は 0 回。部品と connect を足すと 1 回ずつ（方程式 5 → 7）。
- 再検査の時間: 中央値 0.01 ms・最大 0.02 ms（`cargo test --release -p globe-mode-db -- --ignored --nocapture`、20 回、`RealCircuit`）。最初の検査は 17.2 ms。
- MSL: 読み込み 6〜16 s・1.24 GB。別のモデル 0.12 s、同じモデルの再検査 0.08 s。release の `globe check -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit` は 11.3 s・1.5 GB、S6 は 44 / 44（S004 までの CLI は検査のたびに Session を作り直していた。release で 1 本 5〜6 s（S6 なし）。debug ビルドの ChuaCircuit は 18.5 s・1.1 GB）。
- 検証用モデル: `StructurallySingular` は S6 / GM-E6001 が 2 件（12 行目 `x = 2`、9 行目 `der(z)`）、終了コード 1。`HighIndexDuplicate` は S6 を通り（3 / 3）、GM-I7001（der の形 2 / 3）、終了コード 0。
- MSL の全例題（最初の実行、S6 を直す前）: 566 本（partial 13）。OK 215・NG 167（S5 1・S6 166）・GM-U 184（U0001 82・U0002 52・U0003 50）・panic 0。S6（der の形）と Rumoca の照合 381 / 381。構文 566 / flatten 532 / DAE 382 / 釣り合い 381（Rumoca の基準値 566 / 494 / 305 / 293）。検査 1011.6 s（1 本 1.79 s）、最大 RSS 7.5 GiB。
- MSL の全例題（最終の実行）: OK 369・NG 13（S5 1・S6 12）・S7 の情報 154・GM-U 184（U0001 82・U0002 52・U0003 50）・panic 0。照合 381 / 381 一致。検査 1068.3 s（1 本 1.89 s）、最大 RSS 7.2 GiB（1 本の大きなモデル Spice3BenchmarkFourBitBinaryAdder が単独で 6.3 GiB。Session のキャッシュの成長ではない）。S6 で止まった 12 本は接地の無い部品モデル（omc では未確認）。
- MSL の例題 20 本での Rumoca の CLI との突き合わせ（`msl-compare`）: 20 本中 20 本が一致（両方とも成功 17 本）。
- `check_docs.py`: デッキ 6 件 OK / リンク 201 件中の切れ 0 件。
- Playwright（`render_slides.py`、S005 のデッキ）: 14 枚 × 3 表示で問題 0 件。
  - 最初は S11（ファネルの表）が縦に 56 px はみ出したので、2 枚に分けた。
  - スマートフォンで S09 のコードの枠と S06 の表が横にはみ出していたのを、目で見て直した。
- セッション記録の `--check`: 取りこぼし 0 件（検査 3 件）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- **omc を入れていない**。P2 の完了条件「S5 が omc と 95% 以上一致」は測れていない。入れるかどうかはユーザーが決める。入れたら、GM-U0003 の一覧が本当に Rumoca の制約か（omc では通るか）も確かめる（ADR-0007 の C）。
- `deps_closure` が無い。`check` はワークスペースの全ファイルを渡すので、どのファイルの中身が変わっても再実行される。
- `Checker` は同じ Session で多くのモデルを検査すると大きくなる（566 本で最大 7.2 GiB。大部分は 1 本の大きなモデルが単独で使う 6.3 GiB）。`msl-funnel` は `--reset-every` で抑えたが、IDE で `reset()` を呼ぶ条件は決めていない。
- S7（Pantelides と状態選択）・S8（初期化）は未着手。S7 は「微分インデックスが 1 より大きい見込み」の情報だけ。
- 数値的な特異（`HighIndexDuplicate` の同じ電圧則 2 本）は、構造（S6）では見つからない。
- `msl-funnel` は CI に入れていない（時間がかかる）。検査の段階・GM-U の振り分け・Rumoca の rev を変えたら手で回す（CLAUDE.md §2）。
- Rumoca の基準値は別の commit（`b92932c0`）で記録されたもので、GlobeMode の本数（flatten 532 など）との差の内訳は確かめていない。
- Rumoca の制約の上流への報告（ライブラリ間の extends、ED001 の数え方、ET004 の次元）。
- `class_index` の残り: for や if の中の connect、import、継承したクラスの中の入れ子のクラス。

## 追記: S005 の CI の失敗（run 39115769958）と修正

- **何が落ちたか**: plan ジョブの `test_ci_plan.py` が 2 件失敗し、ほかのジョブは動かなかった。
  - `test_graph_change_reaches_db_and_cli` と `test_analysis_change_reaches_db_and_cli`。期待する crate の集合に `xtask` が無かった。
- **原因**: S005 で xtask（`msl-funnel`）が `globe-mode-db` と `globe-mode-analysis` に依存するようになり、graph / analysis の変更が xtask にも及ぶようになった。判定のテストは Phase 0（依存を足す前）に直して、そのあと回し直していなかった。`mise run check` に入っていないので、手元の検査で気づけなかった。
- **直したこと**（`feature/ops-ci-plan-xtask`）:
  - 期待値に `xtask` を足した。
  - `mise run check` の先頭で `test_ci_plan.py` を回すようにした（CI の plan ジョブと同じ）。
- `mise.toml` を変えるので、CI は自動で全体を検査する。

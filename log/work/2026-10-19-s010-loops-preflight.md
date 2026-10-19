# S010 `loops-preflight`

| | |
| --- | --- |
| 日付 | 2026-10-19 |
| ブランチ | `feature/s010-loops-preflight` |
| worktree | `.git-worktree/s010-loops-preflight` |
| Phase | P2 |
| セッション文書 | `doc/sessions/S010-loops-preflight/` |
| セッション記録 | `log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md`（S005・S007〜S009 と同じセッション） |

## 依頼

「S010を進めてください。サブエージェントは`/effort`を`medium`で複数並列実行してください。」

## 決めたこと

ユーザーの指示は「S010 を進める」「サブエージェントは effort medium で複数並列」だけ。ほかは既定のまま進めた:

- **範囲は roadmap の S010 の候補のうち 3（S9・S10）、1（離散変数の when の区別）、2（閉包の切り替えの費用）**。並列にできる独立した作業が 5 つ取れるので、この組にした。4（`Checker` のメモリ）は P4 のプロセス構成が決まってから、5（上流への報告）はユーザーが出す、6（`class_index` の残り）と 7（S8 の残り）は送った。
- **並列の分け方**: 1 つの worktree で、所有するファイルを分けた 5 つのサブエージェント（effort medium）を同時に走らせた。共有の型（core の診断コード、`DaeStructure::nonlinear`、S10 の入力の型、空の解析関数、検証用のパッケージ `Loops`・`Preflight`）は先に私が骨格として固め、コミットした（`9f33c79`）。
  - A: S9 の BLT と tearing（analysis の `loops.rs`）
  - B: S9 の線形・非線形の判定（frontend の `structure.rs`、検証用の `Loops.*`）
  - C: S10（ir・frontend・analysis の `preflight.rs`、検証用の `Preflight.*`）
  - D: 閉包の切り替え（frontend の `Checker`、db）
  - E: S8 の離散変数（frontend と analysis の init、検証用の `Init.Discrete*`）
  - 統合（db のクエリ、CLI、`msl-funnel`、`omc-compare`、文書）は私がやった。B・C・E は途中で API の使用量の上限で止まり、上限が戻ってから続きを頼んだ。
- **S9 の解く系**: インデックス 1 なら der の形、2 以上なら Pantelides 法の最高階の系（状態選択の前の見込み）。状態選択の後の系は dummy derivative の選び方が要るので送った。
- **S9 は Error を出さない**: start 値の無い反復変数は既定の 0 から反復するだけで、解けないとは限らない（omc も警告）。
- **線形の判定は族の単位で広めに**: 誤るのは「線形なのに非線形」の側（警告が多めに出る側）だけ。`abs`・`min`・`max` も非線形にした。
- **S10 は静的な部分だけ**: Rumoca の評価器は実行時の座標を評価しないので、t0 の残差と初期化のヤコビ行列の階数は P3 の実行系と一緒にする（ADR-0012）。
- **experiment は生の値で判定**: Rumoca は不正な値を黙って捨てるので、根のクラスの本文を Rumoca の parser で読み直す。StopTime = StartTime は MSL の `Inverse_sine` が使うので誤りにしない。
- **閉包は和集合**: Rumoca の Session にライブラリの解決だけを残す道が無い（D が調べた）ので、文書の組を変えない方法にした。結果が閉包だけの検査と同じになるよう、本文の変わった・消えた・名前のぶつかる文書は取り除く。
- **字面の 0 除算の振り分け（`ED019` → `GM-U0001`）は変えない**: ADR-0007 のコードの規則を文言で例外にするのは方針の変更なので、候補として残した。
- **Rumoca の `algebraic_loops` が空**なので、照合は ES002 の文言から大きさを読む（B が見つけた）。

## やったこと

- 骨格（Phase 0）: core に `GM-I9001`・`GM-W9001`・`GM-E10001`〜`GM-E10004`・`GM-W10001`・`GM-I10001`・`GM-I10002`、ir に `DaeStructure::nonlinear` と `preflight.rs`、analysis と frontend に空の `loops.rs` / `preflight.rs`、`CheckReport::preflight`、`ReferenceStructure::loops`、検証用のパッケージ `Loops`・`Preflight`。
- A（`globe-mode-analysis/src/loops.rs`）: 解く系、BLT、線形・非線形、貪欲な tearing、start 値、診断。テスト 12 本（乱数の構造 300 個で BLT と tearing の性質、10 万本の鎖と 1 つの大きなループ）。
- B（`globe-mode-frontend/src/structure.rs`、`tests/loops.rs`、`Loops/*.mo`）: 残差の式木を族の単位で歩く線形・非線形の判定。テスト 7 本。ChuaCircuit の extract は 0.97 → 1.5〜2.3 ms。
- C（`preflight.rs` 3 つ、`tests/preflight.rs`、`Preflight/*.mo`）: パラメータと start 値の評価、experiment の生の値、判定。テスト 32 本。
- D（`rumoca.rs` の `Checker`、`globe-mode-db/src/lib.rs`、`tests/switching.rs`）: 文書の和集合、`check_retaining`、`new_handle`、`document_changed`、`GlobeDb::remove_file`。テスト 12 本（`tests/switching.rs` 11 本と `rumoca.rs` の単体 1 本）。
- E（init の 4 ファイル、`Init/Discrete*.mo`）: 離散変数の決まり方、離散の方程式の行、`pre(d)` の列、規則の変更、StateGraph の `pre(a) = pre(b)` の束ね。テスト 19 本。
- 統合（私）: db の `loop_report` / `preflight_shape` / `preflight_report` と `check_model` の S9・S10、`ExperimentVerdict` の再公開、CLI の S9・S10 の行と `--json`、`msl-funnel` の S9・S10、`omc-compare` の S9（`-d=initialization,iterationVars`、反復変数の一覧と zero start の読み取り）、`Checker::reference_structure` の ES002 の読み取り、`extract_for_model` への切り替え、db のテスト 9 本（stages.rs）、funnel と omc のテスト 4 本。
- 文書: ADR-0012（新規）、ADR-0005・ADR-0011 の追記、ADR の索引、architecture・features・roadmap・README・CLAUDE.md、research（rumoca.md・openmodelica-dymola.md）、`src/Modelica/README.md`、`doc/msl-funnel.md`・`doc/omc-compare.md`（生成）、スライドと解説書。

## 検証

- コードの検査: `cargo fmt --all --check` 差分 0、`cargo clippy --workspace --all-targets -- -D warnings` 0 件、`cargo test --workspace` 324 本通過・6 本 ignore（S009 は 232 本）。`mise run check` も通過（テスト 324 本、cargo deny は advisories・bans・licenses・sources とも ok、`uv sync --locked` ok）。`cargo check --target wasm32-unknown-unknown`（core・ir・frontend・graph・db・analysis）も通過。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。
- `msl-funnel`（release、566 本）: OK 369・NG 13（S5 1・S6 12）・GM-U 184・panic 0（S009 と同じ本数）。
  - S8: 合う 269・過剰 6（すべて `GM-W8004`）・不足 94（S009 は 111・6・252）。`NonCircularPipes` は要る数が 40 → 10（Fluid の部品の離散変数が when の外の式で決まると分かった）になったが、冗長 0・決まらない 6 で omc（冗長 2）とは不一致のまま（inner/outer の制約）。
  - S9: 369 本（ループあり 226・非線形あり 142・`GM-W9001` あり 57（反復変数 188 個）、ループは線形 254・非線形 260、最大 4584 本（EngineV6）、最高階の系 154・解析なし 0、Rumoca との照合 215 / 215 一致）。
  - S10: 369 本、Error 0、`GM-I10001` 16 本。
  - 検査 849.8 s（1 本 1.50 s。S009 は 723.8 s・1.28 s。後半に `msl:compare` を同時に回した）、全体 14 分 42 秒、最大 RSS 7.1 GiB。
- `omc-compare`（omc を回し直した。S9 の一覧のため。12 プロセス、omc 370.2 s（照合を含む全体は 6 分 11 秒）、時間切れ 0・結果なし 0）: S5 382 / 382（100%）、数が同じ 307 / 382、GM-U 184 本すべて omc で数が合う、S6 362 / 363、S8（過剰）352 / 355（S009 と同じ）。**S9（非線形のループの有無）282 / 355（79.4%。両方 103・globe だけ 38・omc だけ 35）、S9（start の無い非線形の反復変数の有無）312 / 355（87.9%。両方 29・globe だけ 28・omc だけ 15）**。
  - 最初は omc の「Nonlinear iteration variables with default zero start attribute」と比べて 303 / 355 だったが、この文言は MSL で 5 本にしか出ない（属性に start の無い非線形の反復変数は 81 本）。同じものを数えていないので、反復変数の一覧の属性と比べるように直し、`--reuse` で照合だけやり直した。
- 閉包の切り替え（D の計測、release）: 一度検査したモデルへ編集なしで戻る 2099 ms → 49 ms。初めてのモデル 1791 → 1950 ms（変わらない）。使わないファイルの編集 0.19 → 0.45 ms（揺れ）。
- `check_docs.py`: デッキ 16 件すべて OK（S010 は 16 枚）/ 文書 84 件・相対リンク 516 件中の切れ 0 件。`render_slides.py`（Playwright）: デッキ 16 件 / 問題 0 件（S010 は最初 4 枚がはみ出し、文章を詰めて直した）。
- セッション記録の `--check`: 4 件を検査し、取りこぼし 0 件。
- ステージした差分の漏洩検査: 0 件（下のコミットの前に確かめた）。

## 残したこと

- S9 は状態選択の前の最高階の系で数える（インデックス 2 以上の 154 本）。dummy derivative の後の系は P3 で。
- S9 の線形の判定は族の単位で広め。`abs`・`min`・`max`、`smooth` の中の条件も非線形にしている。
- S10 の t0 の残差・初期化のヤコビ行列の階数・ソルバの能力は P3。
- 字面の 0 除算は Rumoca の `ED019` → `GM-U0001` のまま（振り分けの見直しは S011 の候補 6）。
- S8: アルゴリズムの `when {…, initial()}` の読み出しが `pre` になる形（`Interaction1`、omc は何も言わない）、連続の方程式の中の `pre(d)`、クロックの区間の変数。
- 閉包の違うモデルを初めて検査するときは今も約 1.7〜2.0 s（ワークスペースを最初に全部入れる案は S011 の候補 3）。
- 上流への報告の候補に「`analyze` が `algebraic_loops` を埋めない」を足した（ユーザーが出す）。

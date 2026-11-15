# S344 `solver-budget-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s344-solver-budget-issues` |
| worktree | `.git-worktree/s344-solver-budget-issues` |
| Phase | P3（実行系） |
| セッション文書 | `doc/sessions/S344-solver-budget-issues/` |
| セッション記録 | 専用のものは無い（親のセッションから呼ばれたサブエージェント。`log/local/` に専用の記録が無いので省いた） |

## 依頼

open のソルバの issue #6・#46・#45・#4 を、直す前に自分で再現を確かめて直す。再現しない・既に直っていれば直さず、issue にコメントで根拠を書く（close はユーザー）。#6 と #46 は「予算とキャンセル」の話なので、残り時間を回し直しに引き継ぐ最小の変更でまとめて直す（ADR-0017 の上限との関係を確かめ、必要なら付記）。実行系を変えたので `wasm:node`・`wasm:compare` も通し、msl-sim は手元で回さず `msl-sim.yml` を feature で 1 回回して、実行できた本数・管に収まった本数を前回と比べる。

## 決めたこと

- **#6 は ADR-0028 (c) と衝突する。** ADR-0028 (c) は「残りの予算を渡す版」を CI（run 41282411061）で回して `LightningSegmentedTransmissionLine` が 1 本減ったので採らなかった（S193 も「設計で決めたもの」として #6 を直さなかった）。依頼は「残りを引き継ぐ」なので、**依頼どおりに入れて、全例題の CI で確かめる**ことにした。前回との違いは、残りが無いときに回し直さず**時間切れ（`GM-E11005`）で返す**こと（issue #6 の「直し方」どおり）。時間切れなら msl-sim の「時間切れを 180 s で回し直す」に乗る。前回の試しは BDF の失敗のまま返したとみられる【未確認。その版の差分は残っていない】。ADR-0041 を新しく書き、ADR-0028 (c) に付記した。
- **#46 の予算・キャンセルの部分だけ直した。** 回し直しの lowering は残りが無ければ始めない（#6 と同じ変更）。放置した型付きの道のスレッド（`checked_detached`）に `Observer.cancel` を渡し、段の境目と回し直しの前で止まるようにした。lowering の回数（ネイティブの dopri45 の 2 回）は、`rk45::simulate_artifact` が `pub(crate)` で口が無い（rev 276e82bd と 9c8c56f の両方で確かめた）ので直せない。session の道に替える案は #43 で測って取り消してある（ADR-0013 の付記）。`Refs #46`（`Fixes` にしない）。
- **#45 は直さない。** 3 回解くのはコードで確認した（`build_simulation_artifact` が `drop(check_prepared_component)`、こちらの `check_prepared_initialization`、`run_prepared_simulation`。rev 276e82bd・9c8c56f とも同じ）が、2 回目が「初期化」の段の目印で、外すと BDF の始動の失敗（`MeIntegrationError::Numerical`、段なし）が積分の失敗に混ざって `auto` が切り替わる。S226 の判断のまま。
- **#4 は直さない。** issue の案を実装して試した: Rumoca の `SplitCircleChart`（`vy(start = 3)`、`tsit45`、`--stop-time 20 --tolerance 2e-2`）は直す前も案の後も同じ `GM-E11002`（`algebraic projection did not establish coordinate convergence`）で止まり、`step()` の `Err` の腕に 1 回も入らなかった（debug の出力で確認）。S193 の `CircleChart`（48 組中 30 組が失敗し、救えたのは 0 組）と合わせて、案は効かない。効かない修正を入れない（S193 の方針）。コードは `rk.rs` のコメント 1 か所だけ。
- コメントを書いた issue: #4・#45（上のとおり。直さない根拠）。#6・#46 は親がマージしてから `Fixes #6`・`Refs #46` が付く。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca.rs`: `run_auto` に `max_wall` を足し、回し直しの閉包が残りの予算を受け取る。残りが 0 以下なら `SimError::Timeout` を返して回し直さない。`with_wall`（`SimOptions` の壁時計を差し替える）。`checked_detached` のスレッドの `Observer` に `cancel: Some(token)`。単体試験 `auto_hands_the_remaining_budget_to_each_retry`。
- `src/Rust/mod/globe-mode-exec/tests/simulate.rs`: `#[ignore]` の `msl_auto_falls_back_to_dopri45_when_bdf_fails_to_integrate` の期待を S064 の順序（GM-I11003 → GM-I11002）に直した。
- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: 「小さい刻みでやり直した」のコメントを、SDIRK の話だと分かるように直した（#4）。
- `doc/adr/0041-auto-remaining-budget.md`（新規）、`doc/adr/0028-…`（関係と (c) に付記）、`doc/adr/0017-…`（付記）、`doc/adr/README.md`、`doc/architecture.md`、`doc/roadmap.md`、`CHANGELOG.md`。
- `doc/sessions/S344-solver-budget-issues/`（スライドと解説書）、`doc/sessions/README.md`。

## 検証

- 直す前の確認: #4 は上のとおり（案の前後で同じ失敗）。#6 は `run_auto` が回し直しに `opts` をそのまま渡すことを読んで確認し、実機（`TwoMass`、release の `globe`、直す前）の `--timeout` の測定は機械の負荷で ±1 s 揺れて決め手にならなかった。
- 実機（直した後、debug の `globe`）: `TwoMass` を `auto --timeout 0.2` で回すと `GM-E11005` だけ（回し直した旨の info なし）、`--timeout 60` なら BDF → esdirk34 → dopri45 で終了（`GM-I11003`・`GM-I11002`）。
- `cargo clippy -p globe-mode-exec --all-targets -- -D warnings`: 警告 0。`cargo fmt --check`: OK。
- `cargo test -p globe-mode-exec`: lib 25 本・`integrators` 14 本・`simulate` 31 本（5 本は `#[ignore]`）が通る。新しい試験 `auto_hands_the_remaining_budget_to_each_retry` を含む。`cargo test -p globe-mode --test observed`（中断の 4 本）も通る。
- `cargo test -p globe-mode-exec --test simulate -- --ignored msl_auto`（MSL を読む。debug）: `msl_auto_stops_alike_on_both_routes_when_the_directional_derivative_is_unavailable` は通る。`msl_auto_falls_back_to_dopri45_when_bdf_fails_to_integrate`（`TwoMass`、予算 300 s）は、**直す前から**古くなっていた期待で落ちた（S064 から GM-I11002 の補足は「esdirk34 の失敗」で、「BDF の失敗」は GM-I11003 の方。`#[ignore]` で CI に入っておらず、気づかれなかった）。期待を BDF → esdirk34（GM-I11003）→ dopri45（GM-I11002）に直し、通った（実際のモデルで、予算の残りを渡す回し直しが終了時刻まで進む確認にもなる）。
- `mise run wasm:node`: 通る（smoke・壊れた塊・分割読み込み 5 本が全部読みと一致、mismatched 0）。
- `mise run wasm:compare`: **480 件中 466 件が一致、既知の差 13 件、一致しない 1 件**（`Pendulum` / `dopri853`、相対差 2.0e-9、閾値 1e-9）。S322 の記録と同じ数字・同じ 1 件で、`dopri853` は `run_auto` も `checked_detached` も通らない（この変更の外）。`compare.mjs` は終了コード 1（S322 も同じ）。
- `check_docs.py`: デッキ OK（S344 を含む）、リンク 5,215 件中の切れ 0 件。`render_slides.py`（S344 のデッキだけ）: 7 枚・問題 0 件。最初のスクリーンショットが 30 s を超えて（手元の負荷 load 2〜11 のどれでも）素のままでは時間切れになったので、`page.screenshot` の既定の時間切れだけ 120 s にした薄いラッパで回した（検査の中身は同じ）。
- **全例題の CI（msl-sim は手元で回さず、`gh workflow run msl-sim.yml --ref feature/s344-solver-budget-issues -f 'solvers=["auto"]'`。run 43708045819）**: `msl-sim` ジョブは成功（1 時間 15 分）、`split-all` も成功、`solvers` は取り消した。同じ設定の main の run 43632447630（2026-11-14）と比べて、**実行できた 312 → 312、管に収まった / 比べた 303 / 311 → 303 / 311、協調型の時間切れ 2・外から停止 6、時間切れの回し直し 14（実行できた 6・管に収まった 4）、`auto` の回し直し 16（auto 301・auto → esdirk34 10・auto → esdirk34 → dopri45 1）が前後で同じ**。356 本を JSON で 1 本ずつ突き合わせて、結果・診断のコード・管の外れの数が違うモデルは 0 本。`LightningSegmentedTransmissionLine` は回し直しで esdirk34 が終えて管に収まった（ADR-0028 (c) の最初の試しで減ったもの）。ワーカーの時間は 3681 s → 3522 s（ランナーの速さの揺れ）。**`doc/msl-sim.md` は手で更新していない**（前後で本数が同じ。報告は run の artifact にある）。

## 残したこと

- **#46 の lowering の回数**（ネイティブの dopri45 は 2 回、`auto` は残りがあれば最大 3 回）。Rumoca に lowering 済みを渡す口（`rk45::simulate_artifact` の公開、`run_prepared_simulation` が初期化済みの状態を受け取る口）が要る。上流 issue の下書きにはしていない（`doc/research/upstream/` の 4 点セットが要るので、出すときに作る）。session の道に替える案は #43 で取り消してある。
- **#45・#4** も上流の口が要る（#45）、chart の切り替えの設計（#4）で、このセッションでは直していない。根拠は issue のコメント。
- lowering は Rumoca の予算の外のまま。重い lowering の歯止めは msl-sim の外からの停止（ADR-0017）。
- 検証していないこと: 放置した型付きの道のスレッドが、段の境目でトークンを見て止まること（スレッドが結果を呼び手へ返さない作りで、直接は観察できない。`cargo test -p globe-mode --test observed` の 4 本は通る）。`mise run check` の全体（`cargo deny`・workspace 全体の test）は回していない（変更は `globe-mode-exec` の中だけ、依存の追加なし）。
- 手元の `/tmp`（tmpfs 3.9 GB）が他の担当の出力で一時的に 0 MB になった（共有のスクラッチパッドが約 2.4 GB）。

# S123 `progress-cancel-core`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s123-progress-cancel-core` |
| worktree | `.git-worktree/s123-progress-cancel-core` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S123-progress-cancel-core/` |
| セッション記録 | （親が写す） |

## 依頼

S121 の ADR-0033（Proposed）の「結果」の実装の順の 1: core・IR・exec・facade。`Progress`（段ごとに 1 回、積分中の t は 100 ms に 1 回まで、時計は facade の `now`）、`CancelToken`（`Arc<AtomicBool>`）。実行系は各段の前、`auto` の回し直しの前、session の道の積分の中で見る。session の道は `run_to_stop` を `advance_to` を出力点で区切る形にし、まず速度を測り、5% 以上遅くなれば 100 ms の間隔で区切る。facade に `SimulateHandle`・`simulate_observed`、exec に `run_observed`（`run_with_progress` は包みで残す）。中断後は `GM-I11004`、積分中なら `Termination::Cancelled { time }` と途中の列、それより前なら `ok: false` と診断だけ。試験は ADR の (a)〜(d)。ADR は Proposed のまま「結果」に数字を追記、`doc/architecture.md` に 1 行。2 段目以降、`globe-mode-sim`・積分器の数値・db・graph・Rumoca・外部の依存は触らない。

## 決めたこと

- **session の道の区切りは、指示の `advance_to` でも 100 ms の間隔でもなく、積分器（`MeIntegratorBackend`）を包む形にした**（既定のまま進めた判断。ADR-0033 の 1 段目の結果に書いた）。`advance_to(t)` の t は Rumoca の `reachable_bound` で刻みの上限になるので、出力の点で区切った版は `tests/integrators.rs` の 13 本中 6 本が落ちた（euler の x(1) 0.348678 → 0.367511、trbdf2 の HarmonicOscillator が積分に失敗、ほか）。100 ms の間隔で区切る案は区切りが壁時計で決まり、同じ入力で数値が揺れる。包む形は刻みの要求を変えない。
- 速さ（release、`Simulation.*` 20 本 × 実装のあるソルバすべてを session の道で 3 回ずつ、最小の `run_seconds` の和）: `run_to_stop` 22.368 s、包む形 22.630 s（+1.2%、モデルごとに −3.1%〜+4.6%）、出力の点ごとの `advance_to` 22.107 s（小さなモデルで +72%〜+366%）。計測は一時的な環境変数の切り替えと ignore の試験で行い、コミットしていない。
- 止めたときの時刻は、`finish()` で取った列の最後の出力の点。包みは「次の 1 歩の前」に止めるので、ADR の試験 (b) の上限は「0.5 + 出力の間隔」でなく「0.5 + 1 歩」（試験は 0.5 ≤ time < 1 と、列の最後の時刻と一致）。
- 止めたあとの失敗（段の前・`auto` の回し直しの前で止めて返る誤り）は Error にせず `GM-I11004` だけ。`auto` は止められていれば回し直さない。
- facade で検査とコンパイルの前にはトークンを見ない（exec の最初の段の前で見る）。1 段目の範囲を exec に限った。
- `Termination` に変種が増えたので、CLI の表示と xtask の `msl-sim`（来ない。`Terminated` 扱い）に腕を足した。

## やったこと

- `src/Rust/mod/globe-mode-core/src/lib.rs`: `codes::RUN_CANCELLED = "GM-I11004"`。
- `src/Rust/mod/globe-mode-ir/src/sim.rs`: `Termination::Cancelled { time }`。
- `src/Rust/mod/globe-mode-exec/src/lib.rs`: `Progress`・`CancelToken`・`Observer`・`run_observed`、`run_with_progress` を包みに、`cancelled_diagnostic`。
- `src/Rust/mod/globe-mode-exec/src/rumoca.rs`: `Hooks` に進捗・トークン・時計、`enter`・`tick`・`is_cancelled`。段の前・`auto` の回し直しの前で見る。`simulate_with` で `Cancelled` と `GM-I11004`。単体試験に「止められていれば回し直さない」。
- `src/Rust/mod/globe-mode-exec/src/rumoca/session.rs`: 積分器を包む `Observed`、止めたら `finish()` の列を返す。
- `src/Rust/mod/globe-mode/src/lib.rs`・`tests/observed.rs`: `SimulateHandle`・`simulate_observed` と試験 (a)〜(d)。
- `src/Rust/mod/globe-mode-cli/src/simulate.rs`・`src/Rust/xtask/src/sim.rs`: `Cancelled` の腕。
- `doc/adr/0033-progress-and-cancel.md`（1 段目の結果）、`doc/architecture.md`（exec の行と診断の表）、`doc/roadmap.md`、セッション文書。

## 検証

- `cargo test -p globe-mode-core -p globe-mode-ir -p globe-mode-exec -p globe-mode`: 全部通った（core 13、ir 8、exec 21 + integrators 13 + simulate 31（ignore 3）、facade observed 3 + graph_library 3 + library_closure 3）。
- `cargo clippy`（変えた crate と cli・xtask、`--all-targets -D warnings`）: 警告 0
- `mise run wasm:node`: ok（共有 target の古い wasm を避けるため、変えた crate と core・fmi・fmt・wasm を touch して作り直した）
- `mise run wasm:compare`: 400 件中 387 件が一致、既知の差 13、一致しない 0（S064 ほかの記録は 360 件・不一致 0。件数はソルバが増えた分）
- `mise run check`（load average 3 のとき）: test_ci_plan・fmt・clippy は通った。`cargo test --workspace` は共有 target の別の作業のビルドで `globe-mode-fmu` の試験の実行ファイルが消えて止まったので、変えた crate を touch して `cargo test --workspace --no-fail-fast` を回し直し、64 本の試験の実行ファイルで 626 passed・0 failed。`cargo deny check`（advisories・bans・licenses・sources ok）、`uv sync --locked` ok
- `check_docs.py`: デッキ OK、相対リンク 2,640 件中の切れ 0 件。`render_slides.py`: 7 枚 × 3 表示で問題 0 件
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（main との差分全体も 0 件）

## 残したこと

- ADR-0033 の順の 2〜5（wasm・Svelte、Python、C・NIF、LSP・Desktop）。
- 積分器の 1 歩ごとに `now` を呼ぶ。wasm の `performance.now()` の費用は 2 段目で測る。
- 状態の無いモデル（積分器が無い）は積分の中で止められない。
- facade の検査とコンパイルの前ではトークンを見ていない。

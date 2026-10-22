# S018 `init-resettle-check`

| | |
| --- | --- |
| 日付 | 2026-10-22 |
| ブランチ | `feature/s018-init-resettle-check` |
| worktree | `.git-worktree/s018-init-resettle-check` |
| Phase | P3（実行系の 7 回目） |
| セッション文書 | `doc/sessions/S018-init-resettle-check/` |
| セッション記録 | `log/sessions/2026-10-22-b4423585-4c94-49e2-bb23-ae4c23d0d96e.md`（S017 と同じセッション） |

## 依頼

S017 の締めのあと「進めて」。roadmap の S017 の候補のうち、S016 で見つけた **4. 初期化の後の初期化方程式の検査**（EngineV6 の `filter.x` が t = 0 から外れる原因は Rumoca の初期化で、GlobeMode は黙って通す）を選んだ。Rumoca に初期化方程式の残差を評価する公開の入口があるかは【未確認】だった。

## 機械

S017 と同じ Raspberry Pi（aarch64、4 コア、RAM 7.9 GiB、swap 8 GB）。`MISE_DISABLE_TOOLS=elixir`。

## 決めたこと

- **候補の選び方**（既定のまま）: 候補 1（止まった例題）・8（上流への報告）は上流の問題で GlobeMode のコードが増えない。候補 4 は S016 で原因まで分かっていて、GlobeMode の側で使う人に知らせる手段を足せる。RPi で回す費用（exec の変更なので `msl:sim` が要る）も見込めた。
- **残差ではなく解き直しで見る**（既定のまま。[ADR-0020](../../doc/adr/0020-init-resettle-check.md)）: Rumoca rev 276e82bd は初期化方程式の残差を評価する `eval_initial_residual` を公開していない（crate の中の trait）。公開の `settle_initialization_system` ほかを Rumoca の FMI ME の kernel と同じ順に呼べば実行が始まる点を再現でき、関係式の記憶はパラメータの枠に残るので、そこからもう一度解くと反転した後の枝で解ける。動けば警告 GM-W10004。
- **警告にとどめ、実行は変えない**（既定のまま）: 初期化の後の状態を差し替える公開の入口が無い。Rumoca の CLI と結果が食い違うのも避ける。
- **1 回目との差ではなく、実行の t0 の値との差で警告する**（既定のまま。最初の実装を MSL の結果で直した）: 1 回目との差で警告すると `IMC_Initialize` で 169 変数が出るが、結果の t = 0 の行は 2 回目の値で、管も収まる。Rumoca の FMI ME の host は初期化のあとの初期イベントの反復で、離散変数のあるモデルでは解き直す。そこで `run` が実行のあとに結果の最初の行と比べ、2 回目と違うものだけを警告する。単独の `preflight` は GM-W10004 を出さない。2 回目の初期化の失敗（`TanksWithOverflow` の homotopy）も黙る。
- **全部のモデルで行う**（既定のまま）: 根の条件があるモデルに限ると、関係式の記憶が原因の分しか見ない。費用は `msl:sim` の段ごとの秒で測る。
- **target の使い回し**（既定のまま）: S017 の worktree の `target/`（18 GB）を新しい worktree に `cp -a` し、依存のビルドを省いた（workspace の crate だけ作り直す）。

## やったこと

- `src/Rust/mod/globe-mode-core/src/lib.rs`: `codes::INIT_RESETTLE_MOVED`（GM-W10004）。
- `src/Rust/mod/globe-mode-exec/src/rumoca.rs`: `eval_at_start` を `probe_start`（GM-W10002 の評価と `resettle`）にし、`preflight_diagnostics` が `t0_not_finite_diagnostics` と `resettle_diagnostic` を並べる。`resettle` は 1 回目（初期化系 → 関係式の記憶 → 代数変数。許容誤差は設定の値、反復 64）→ 2 回目（初期化系 → 代数変数）で、`|y2 - y1| > max(10 tol, 1e-6) × max(1, |y1|, |y2|)` を動いたとする。値の表示は小数 6 桁で末尾の 0 を落とす。
- `src/Modelica/GlobeModeValidation/Simulation/InitRelationBranch.mo`（と `package.order`）: 最小の再現。
- `src/Rust/mod/globe-mode-exec/tests/simulate.rs`: `an_initialization_that_reads_a_relation_flipped_at_t0_is_warned`（プリフライトの本文、4 つのソルバの `run` で警告 1 件と x(0) = 2・u(0) = 1）。`VALIDATION_MODELS` に足した（session の道との一致）。
- `src/Rust/xtask/src/sim.rs`: 報告に「初期化の解き直しで値が変わる警告（GM-W10004）が出た」の本数。
- 文書: ADR-0020、`doc/adr/README.md`、`doc/roadmap.md`（S018 の節、候補 4）、`doc/architecture.md`（S10 の行、exec の行）、`doc/features.md`、`doc/research/msl-sim-failures.md`（節「S018 で足したもの」）。

## 検証

数字で書く。通っていない項目は「通っていない」と書く。

- `cargo test -p globe-mode-exec --test simulate`: **28 本通過・0 失敗・3 ignore**（MSL を読むもの）。新しい試験 `an_initialization_that_reads_a_relation_flipped_at_t0_is_warned` は、本文が `x = 2 → 1` と一致し、4 つのソルバで警告 1 件・x(0) = 2・u(0) = 1。`the_session_route_matches_the_typed_route` も 13 本で一致。S017 の `target/` を写したので、ビルド込みで 13.5 分（依存は作り直していない）。
- `mise run msl:sim`（RPi、初回。参照結果 353 本・1054.7 MB を 106.7 s で取得）: **356 本: 実行できた 308・管に収まった 301 / 307（98.0%）**・実行 NG 41（時間切れ 4）・外から停止 7・panic 0。回し直し 19 本のうち 6 本が実行できた。全体 5845 s（97.4 分。壁時計は release のビルドと取得を含めて 13:02〜14:42 の 99 分）。S015（x86_64、309 本・301 / 308）との違いは `EngineV6` の 1 本だけ: S015 は回し直しで 169 s で終わったが、RPi では lowering 112.5 s のあと積分の段で 240 s の応答の上限に当たった（機械の速さ。プリフライトは 15.0 s）。
- **1 回目の実装（1 回目との差で警告）では GM-W10004 が 2 本**: `Electrical.Machines.Examples.InductionMachines.IMC_Initialize`（169 変数。実行でき、管 OK 8 / 8。結果の t = 0 の行は 2 回目の値だった → 誤り）と `Fluid.Examples.Tanks.TanksWithOverflow`（2 回目の homotopy が失敗。S015 と同じく積分で止まる）。これで判定を「実行の t0 の値との差」に直した（決めたこと）。直した版の `msl:sim` は下。
- `mise run msl:sim` の 2 回目（判定を直した版。14:56〜16:27 の 91 分、全体 5439 s）: **356 本: 実行できた 307・管に収まった 300 / 306（98.0%）**・実行 NG 41・外から停止 8・panic 0。**GM-W10004 は 0 本**（IMC_Initialize と TanksWithOverflow は出なくなった。EngineV6 は RPi では外から止まるので報告に入らない）。1 回目との差は `LightningSegmentedTransmissionLine` の 1 本: 1 回目は 197.7 s で終了時刻まで（回し直しで）、2 回目は 240 s の応答の上限で止まった。2 回目は機械が全体に遅く（compile 9.8 → 15.1 s、lowering 16.2 → 25.2 s、初期化 27.9 → 31.3 s）、プリフライトも 33.6 → 45.9 s。この本は RPi では上限ぎりぎりで、解き直しの費用（初期化の約 2 回分）が効いている。プリフライトの段の合計は 93.6 s。
- **EngineV6 で確かめた**（`globe simulate -L data/msl -m …EngineV6 --stop-time 0.01 --tolerance 1e-7`、RPi で準備 150.5 s・実行 6.2 s、3 分 12 秒）: GM-W10004 が `filter.x[1] = 0.065992 → 563.186238, filter.x[2] = 0.065992 → 563.186238, filter.y = …, filteredEngineTorque = …, $formal_derivative.1.filter.x[1] = 27487.728804 → 0`（計 7 個）で出た。S016 で参照結果と見比べた値（0.0659918 と 563.18623550）と一致する。`msl:sim` の全体では、RPi では EngineV6 が 240 s の応答の上限で止まるので、報告には入らない。
- **プリフライトの段の費用**（`target/msl-sim.json` の `stage_times`）: 356 本の合計 98.5 s（S015 は約 12 s）、中央値 0.014 s、最大は `LightningSegmentedTransmissionLine` の 33.6 s（この本は実行の初期化そのものが 27.9 s で、解き直しの 2 回分が足された）。次いで `EngineV6` 15.0 s、`PointGravityWithPointMasses2` 4.1 s、`Engine1b_analytic` 3.9 s。全体の 5845 s の 1.7%。
- `mise run check`（RPi、2026-10-22〜23）: 1 回目は `cargo test` の `globe` のリンクで `ld` がメモリ不足で止まった（signal 9。fmt と clippy は通過。22:19〜02:24 の 4 時間、ほとんどスワップ待ち。swap は 5.7 GiB しか見えなかった）。**2 回目は `CARGO_BUILD_JOBS=2` で通過**（29 分）: `cargo test --workspace` は 488 本通過・0 失敗・10 ignore、`cargo deny` は advisories・bans・licenses・sources とも ok、`uv sync --locked` も通過。
- `mise run wasm:node`: ok。`mise run wasm:compare`: **123 / 126 件が一致・既知の差 3 件・一致しない 0 件**（線形メモリ 4 MiB）。`InitRelationBranch` は 9 つのソルバすべてで wasm とネイティブの両方が GM-W10004 を出し、結果も一致した（最大 63 ulps、tsit45）。
- ディスク: SD カード（117 GB）が 95% になったので、S017 の worktree の `target/`（18 GB）を消した。`target/` を USB のカード（`/mnt/usb_msd128d`）に移すことも試したが、書き込みが約 1.4 MB/s しか出ず（23 分で 1.9 GB）、やめた。

## 残したこと

- 根の条件の無いモデルでは解き直しを省く（ADR-0020 の案 D）。`LightningSegmentedTransmissionLine` は解き直しの分で RPi の 240 s の上限に当たるようになった。Lightning に根の条件があるかは確かめていない（Rumoca の CLI の `--inspect structure` は MSL の解決で失敗した）。
- レビュー（ponytail-review）の指摘で直していないもの: (1) 解き直しが panic すると GM-W10002 の結果も捨てられる（`resettle` だけを別の `catch_unwind` で包む）、(2) `resettle_diagnostic` の「t0 の値と 2 回目が同じなら出さない」の単体試験が無い（`msl:sim` でしか確かめていない）、(4) 「動いた」の判定式が 2 か所にある、(5) `Resettle::moved` の 1 回目の値は使っていない、(6) `tests/simulate.rs` で新しい試験が S014 の試験の doc コメントの先頭行を取った、(7) `VALIDATION_MODELS` のコメントが「12 本」のまま。
- RPi では `cargo` を `CARGO_BUILD_JOBS=2` で回す（4 本並べるとリンクでメモリが足りない）。swap が 8 GB から 5.7 GiB に戻った理由は確かめていない。

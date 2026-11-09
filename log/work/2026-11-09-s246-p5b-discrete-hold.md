# S246 `p5b-discrete-hold`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s246-p5b-discrete-hold` |
| worktree | `.git-worktree/s246-p5b-discrete-hold` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S246-p5b-discrete-hold/` |
| セッション記録 | （親が写す） |

## 依頼

S237 から残る P5b の正しさの不足 2 件: (1) `DiodeBridge2Pulse`・`DiodeCenterTap2Pulse` の `Math.Mean` が事象の間で保たれない、(2) `DemonstrateSignalExtrema` の極値のずれ。再現してから直す。

## 決めたこと

- (1) は再現した（FMPy の ME で同じ時刻に 2 回目の事象。`when sample` の中が `EventUpdateRowFilter::All` で回り直す）。直しは `ModelInstance::update_discrete_states`（`globe-mode-exec/src/rumoca/instance.rs`）。brief は「`globe-mode-fmu/src/lib.rs` の中」としていたが、事象処理は `ModelInstance` にあり、これを使うのは P5b の FMU だけ（Rumoca と exec の host は触っていない）。
- (2) は直さない。FMU は解析解に合い、参照（exec の BDF）が 0.32% 外れている。ADR に原因を書き、`fmu_msl_check.py` の許容は広げない。
- 最小のモデルは `GlobeModeValidation.Simulation.SampleAtZeroCrossing`。`when {s < 0, s > 0}` の計数やずらした零交差の形は、直す前でも再現しなかった（整流器の `if s > 0` の形だけが当たる）。

## 数字

- MSL の 20 本（P5b、FMI 2.0、ME・CS、FMPy 0.3.32）: 16/20 から 18/20。落ちるのは ChuaCircuit と DemonstrateSignalExtrema。ほかの 16 本は直した後も通った。
- DemonstrateSignalExtrema の t = 0.64 の y_max: 解析解 4.648119、exec 4.633220、FMPy ME（1e-6）4.650760、FMPy ME（1e-10）4.648122、P5b の CS 4.648119。
- `SampleAtZeroCrossing`（StopTime 0.015）: 直す前の ME は yLast = -1.1e-20、直した後は 0.6355（ME）・0.6366（CS）、exec 0.6365。

## 検証

- `mise run check` 通過（fmt・clippy・test・deny・`globe fmt --check src/Modelica` 73 ファイル整形済み・`uv sync --locked`）。
- `cargo test --release -p globe-mode-fmu --test fmpy -- --test-threads=1`: 18 本通過（新しい `fmpy_matches_exec_on_sample_at_zero_crossing` は直しを外すと落ちることを確認）。
- `check_docs.py`: 切れ 0 件。`render_slides.py`（S246 のデッキ）: 問題 0 件。
- セッション記録は親が写す。CI（feature ブランチ）: `fmu-export.yml` run 42556889948 が success（MSL の step に DiodeBridge2Pulse を含む）、`ci.yml -f full=true` run 42556893345 が success。

## 残したこと

- 状態事象が先で時刻事象があとの順（Rumoca の `Unconsumed`）は P5b の FMU では見ていない。
- DemonstrateSignalExtrema を試験に入れるなら、参照を解析解にするか、ME の許容を 1e-10 にした比べ方が要る。

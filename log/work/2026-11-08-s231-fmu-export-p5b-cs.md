# S231 `fmu-export-p5b-cs`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s231-fmu-export-p5b-cs` |
| worktree | `.git-worktree/s231-fmu-export-p5b-cs` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S231-fmu-export-p5b-cs/` |
| セッション記録 | （親が写す） |

## 依頼

roadmap の「FMU の Export の残り」から P5b の Co-Simulation。`globe-mode-fmu` に CS の入口、`modelDescription.xml` に `<CoSimulation>`、CLI `--kind`、試験、`fmu-export.yml`、issue #89、文書。

## 決めたこと

- 積分器は `globe-mode-fmi-import` の `ode.rs` を再利用しない。`pub(crate)` で `Error` が Import の型、Import の crate が `libloading`・zip・tempfile を引く。`globe-mode-exec` に 1 歩進める道が無い。crate の中に RK4 を 20 行ほど置いた（依頼は「新しい積分器は書かない」だったが、再利用の道が依存の向きで無いため。報告に書く）。
- CLI `--kind` は足さない。`globe fmu export` は P5a だけで、P5b を作る道が CLI に無い。P5a は ME と CS を元から書く。代わりに `examples/md.rs`（`fmu-export.yml` が使う）と `model_description_kind` を足した。
- #89 は `serde` を optional でなくした（使う側を feature で囲むより小さい）。`cargo check -p globe-mode --tests` の `E0433` を直す前に再現した。

## やったこと

- `src/Rust/mod/globe-mode-fmu/src/lib.rs`: `Kind`・`model_description_kind`、`Instance` に `cs`・`t_next`・`z_prev`、`do_step`・`rk4`、`fmi2DoStep`・`fmi3DoStep`・`fmi3EnterStepMode`・`fmi3InstantiateCoSimulation`、未対応の `fmi2*`（`unsupported!`）。
- `src/Rust/mod/globe-mode-fmi/src/lib.rs`: `CoSimulation` の要素に `canHandleVariableCommunicationStepSize="true"`。
- `tests/fmpy.rs`・`tests/fmpy_run.py`・`examples/md.rs`、`.github/workflows/fmu-export.yml`。
- `src/Rust/mod/globe-mode/Cargo.toml`（#89）。
- 文書: features.md・roadmap.md・ADR-0031 の追記・CHANGELOG・セッションの索引・スライドと解説書。

## 検証

- `cargo test -p globe-mode-fmu --test fmpy co_simulation model_description_kind`: 2 件 OK。FMPy の CS で Decay の x(1) = 0.3678794411714462（e⁻¹ と 4e-15）、BouncingBall の h = 0.26074173・v = -0.16586914・n = 3（exec は 0.26074169・-0.16586948・3）、InputDecay の x = 1.1548181、y = 2.3096362（2.0・3.0 とも同じ）。
- `fmu-export.yml`（feature ブランチ、run 42453859348）success。新しい step で P5b の FMU（ME と CS を 1 つに書く）を FMPy と fmusim が validate し、CS の Decay が FMI 2.0・3.0 とも FMPy・fmusim で x(1) = 0.3678794411714462（e⁻¹ と 3.886e-15）。
- 検査・ci.yml の結果は最後に埋める。

## 残したこと

- 誤差の制御つきの積分、陰的な積分、`fmi2CancelStep`・`GetStatus`・入力の微分、2.0 の `terminate()`、P5b を作る CLI、Python・UI の選択肢。

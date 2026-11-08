# S231 `fmu-export-p5b-cs`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s231-fmu-export-p5b-cs` |
| worktree | `.git-worktree/s231-fmu-export-p5b-cs` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S231-fmu-export-p5b-cs/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

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
- `mise run check`: EXIT=0（`globe-mode-fmu` の試験 16 件 OK。最初の回は、既存の `explicit_euler_through_the_c_abi` が「CS は断る」を前提にしていたのと、ME と CS の試験が同じ作業ディレクトリを使って並行に消し合ったのとで 2 件落ち、直した。ci.yml の最初の run 42454907042 も前者で rust-linux・rust-windows が落ちた）。
- `ci.yml -f full=true`（feature ブランチ、run 42456868789）: 全 17 ジョブ success。
- `cargo test -p globe-mode`（feature 無し）OK。直す前は `cargo check -p globe-mode --tests` が `E0433` で落ちるのを再現した。
- `check_docs.py`: 切れ 0 件。`render_slides.py`（S231 のデッキ）: 問題 0 件。セッション記録の写しと漏洩検査は親がやる。

## 残したこと

- 誤差の制御つきの積分、陰的な積分、`fmi2CancelStep`・`GetStatus`・入力の微分、2.0 の `terminate()`、P5b を作る CLI、Python・UI の選択肢。

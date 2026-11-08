# S224 `fmu-export-types`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s224-fmu-export-types` |
| worktree | `.git-worktree/s224-fmu-export-types` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S224-fmu-export-types/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

roadmap の表「FMU の Export の残り」から Enumeration・String・`dependencies`。P5a（`globe-mode-fmi/src/export.rs`）と P5b（`globe-mode-fmu`）の `modelDescription.xml`、P5b の実行時、`GlobeModeValidation` に試験用のモデルを 1 つ、`fmu-export.yml` で FMPy・fmusim が読めること。#50・#51（S223）、Import、3.0 の SE、P5b の CS、MSL を使うモデルは対象外。

## 決めたこと

- 型の名前とリテラルは flat モデル（`variable_type_names`・`enum_literal_ordinals`）から `CompiledModel` に持たせる。DAE には無い。
- P5a は Rumoca のテンプレートを改変せず、#48 と同じく XML を後処理する。**FMI 3.0 の Enumeration は書き換えない**: 3.0 の Enumeration は Int64 の getter で、Rumoca の fmi3 の C は Int64 が未対応。壊れた FMU を出すより Int32 のままにする（ユーザーの指示の「3.0 は `<Enumeration>`」から外れる。理由を報告）。
- P5b は値参照を増やさず、Rumoca のパラメータの枠・変数の枠をそのまま使う。String は `fixed`、書くと Error。
- `dependencies` は持ち越し。出力ごとの依存を返す情報が `globe-mode-analysis`・frontend に無い（incidence だけ）。閉包を新しく書くのは指示の外。

## やったこと

`globe-mode-frontend/src/rumoca.rs`（`EnumType`・`enum_type_of`・`string_parameters`・`enum_types`）、`globe-mode-exec/src/rumoca/instance.rs`（`ValueType::Enumeration`）、`globe-mode-fmi/src/{lib,export}.rs`（`EnumerationType`・`declared_type`・読み書き・`declare_enums`）、`globe-mode-fmu/src/lib.rs`（describe と `fmi2/3` の Get/Set）、`src/Modelica/GlobeModeValidation/Simulation/EnumString.mo`、`.github/workflows/fmu-export.yml`、試験、文書。

## 検証

- `mise run check`（fmt / clippy -D warnings / `cargo test --workspace` / cargo deny / `globe fmt --check src/Modelica` / `uv sync --locked`）: 通った。試験 809 本 OK・0 本失敗（`--workspace` の `test result` の合計）。
- 追加した試験: `globe-mode-frontend/tests/enum_types.rs`（型とリテラル、String のパラメータ）、`globe-mode-fmi/tests/parse.rs`（Feedthrough 2.0 の String・Enumeration が往復）、`globe-mode-cli/tests/fmu_export.rs::exports_enumeration_and_string_types`（P5a の 2.0 と 3.0）、`globe-mode-fmu/tests/fmpy.rs::enumeration_and_string_through_the_c_abi` と `fmpy_validates_enumeration_and_string`（FMPy の validate と ME、2.0・3.0）。
- 手元で Rumoca の C を `--build` した EnumString（2.0・3.0）: FMPy の validate が問題なし、ME・CS の x(1) = 0.1353353（e⁻² との差 9e-9）。
- `check_docs.py`: デッキ OK / リンク 4282 件中の切れ 0 件。`render_slides.py`（S224 の 1 デッキ）: 問題 0 件。
- CI: 親が回す（feature の push と `fmu-export.yml`・`ci.yml -f full=true` は報告の run を見る）。
- セッション記録の `--check`・ステージ差分の漏洩検査: 取りこぼし 0 件（親が写した）

## 残したこと

`dependencies`、P5a の 3.0 の Enumeration、P5b の離散の出力の `causality` と初期化の事象、`Item` の description・配列の Enumeration・`min`/`max`。

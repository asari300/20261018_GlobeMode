# S315 `ssp-roundtrip-2`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s315-ssp-roundtrip-2`（origin/main から切った） |
| worktree | `.git-worktree/s315-ssp-roundtrip-2` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S315-ssp-roundtrip-2/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

SSP の往復の残り: `.ssp` → spec.json の逆変換 `globe fmu ssp-import`、名前に `.` を含む FMU の `write_ssp`。

## 決めたこと

- FMU は `.ssp` の外（JSON と同じフォルダの `<名前>.fmu`）に出し、JSON は相対パス。埋め込みより小さく、既存の `read_spec` の規則に合う。同名は上書きしない。
- `.` を含む名前は平らのまま書く（XSD の `name` は `xs:string` で `.` を禁じない。2026-11-15 に確認）。入れ子に戻す書き出しは持ち越し。
- `write_ssp` の `var_of` を最長一致に直した（`s.a.y` を `s` と `a.y` に割っていた）。

## やったこと

`globe-mode-fmi-import/src/ssp.rs`（`Ssp::export_spec`、`write_ssp` の名前の検査、`var_of`）、`cosim.rs`（`valid_fmu_path` を crate 内公開）、`globe-mode-fmi/src/ssp.rs`（`MAX_NAME` を公開）、`globe-mode-cli/src/fmu.rs`（`ssp-import` と help）、試験 3 本、ADR-0026 の (x)・roadmap・features。

## 検証

- `cargo test -p globe-mode-fmi-import --lib ssp`（4 本 pass）、`-p globe-mode-fmi`（pass）、`-p globe-mode-cli --test fmu_cosim`（17 本 pass。reference FMU あり、`GLOBE_REQUIRE_REFERENCE_FMUS=1`）。
- `cargo clippy -D warnings`（fmi・fmi-import・cli の全 target）・`cargo fmt --check`: 警告 0。`mise run check` 全体は回していない（変えた crate に絞った）。

## 残したこと

入れ子の System への書き出し、`.ssv` の書き出し、UI・LSP から `ssp-import` を呼ぶ口。

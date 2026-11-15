# S318 `ssp-ssv-export`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s318-ssp-ssv-export`（`feature/s315-ssp-roundtrip-2` から切った） |
| worktree | `.git-worktree/s318-ssp-ssv-export` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S318-ssp-ssv-export/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

`write_ssp` が `start` を `.ssv` のファイルとして書けるようにする（`globe fmu ssp-export --ssv`。既定は今のまま）。仕様の要点を調査に確かめて書く。

## 決めたこと

- 口は `write_ssp_with(spec, out, ssv)`（`write_ssp` は `false` の別名）。CLI は `--ssv`。
- 型の要素の名前空間は XSD（v2.0.1、`gh api` で取得）で `ssv`。S263・S295 の【未確認】を調査で解消した。
- `.ssv` は `start` を持つ Component だけ。

## やったこと

`globe-mode-fmi-import/src/ssp.rs`（`write_ssp_with`、試験 1 本）、`lib.rs`・facade の再 export、`globe-mode-cli/src/fmu.rs`（`--ssv`・help）、`tests/fmu_cosim.rs`（`--ssv` の場合）、調査・ADR-0026 の (y)・roadmap・features。

## 検証

- `cargo test -p globe-mode-fmi-import ssp`（5 本 pass。新規 1 本を含む）、`-p globe-mode-cli --test fmu_cosim ssp`（12 本 pass。reference FMU あり、`GLOBE_REQUIRE_REFERENCE_FMUS=1`。`--ssv` の CSV がバイト一致）。
- `cargo clippy -D warnings`（fmi-import・cli の全 target、facade も確認）・`cargo fmt --check`: 警告 0。`check_docs.py`: 切れ 0。`mise run check` 全体と render_slides は回していない（変えた crate に絞った）。

## 残したこと

`.ssm` の書き出し、UI・LSP からの `--ssv`、1 つの `.ssv` を複数 Component で共有する書き方。

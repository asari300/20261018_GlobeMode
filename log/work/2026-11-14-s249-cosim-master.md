# S249 `cosim-master`

| | |
| --- | --- |
| 日付 | 2026-11-14 |
| ブランチ | `feature/s249-cosim-master` |
| worktree | `.git-worktree/s249-cosim-master` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S249-cosim-master/` |
| セッション記録 | なし（並行のサブエージェントの担当で、このセッションだけの記録が `log/local/` に無い） |

## 依頼

FMI Import のコシミュレーションのマスター（最小版）。複数の CS の FMU を固定の通信刻みでつなぎ、Jacobi 型で回して全 FMU の出力を 1 つの CSV に出す。CLI `globe fmu cosim`、facade に 1 関数、Real のみ。Python・UI はやらない。

## 決めたこと

- 結線の記述は JSON のファイル（CLI の引数より小さく済み、facade・ワーカーでそのまま使える）。FMU のパスは spec.json からの相対。
- FMU の読み込みは今までどおりワーカー（別プロセス）の中。`WorkerRequest` に省略可の `cosim` を足し、`names`/`rows` のプロトコルは変えない。FMU は親の作った展開先の下に `<名前>/` で展開。
- 断るもの（実行の前）: Real でない端、出力でない始点、入力でない終点、`fmus` に無い名前、不正な FMU の名前。
- 初期値の `set`、Gauss–Seidel、可変刻み、`fmu-import.yml` への足しはやらない（ADR-0026 の (q) の限界に書いた）。

## やったこと

`globe-mode-fmi-import`（`cosim.rs` 新、`lib.rs`・`worker.rs`）、`globe-mode`（`run_cosim`、`run_fmu` の尾を `run_worker` に括り出し）、`globe-mode-cli`（`fmu cosim`、`tests/fmu_cosim.rs`）、文書（ADR-0026 (q)、roadmap、architecture、セッション文書、索引）。

## 検証

- `cargo fmt --check`（fmi-import・globe-mode・globe-mode-cli）、`cargo clippy -p globe-mode-fmi-import -p globe-mode -p globe-mode-cli --all-targets -- -D warnings` 警告 0。
- `cargo test -p globe-mode-cli --test fmu_cosim`（新）2 本 pass（Dahlquist 2.0 → Feedthrough 3.0 で b の出力 = a.x の 1 刻み前、全 10 刻みでちょうど等しい。パラメータへの結線は「入力でない」で断る）、`--test fmu_worker` 5 本 pass、`cargo test -p globe-mode-fmi-import` 28 本 pass（12 + 7 + 4 + 5）。
- `check_docs.py`: S249 のデッキ OK / リンク 4541 件中の切れ 0 件。`render_slides.py`（S249 のみ）: 問題 0 件。
- `mise run check` 全体（deny・全 crate の test）と `fmu-import.yml` への足しは回していない。依存は増やしていない（Cargo.lock 変更なし）。

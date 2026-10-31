# S050 `solver-matrix-ci`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s050-solver-matrix-ci` |
| worktree | `.git-worktree/s050-solver-matrix-ci` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S050-solver-matrix-ci/` |
| セッション記録 | `log/sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md`（S048〜S052 と同じセッション） |

## 依頼

ソルバごとに MSL の例題を実行した本数と参照結果の管に収まった本数を GitHub Actions で出し、1 枚の表 `doc/msl-solvers.md` にする（roadmap P8 の準備）。

## 決めたこと

- ジョブ `solvers`（`needs: msl-sim`）の matrix は bdf・dopri45・esdirk34・trbdf2・tsit45・esdirk45・sdirk34hw・dopri853・cerk23・cerk34・cerk45・radau5・cvode・ida の 14 本。`auto` は `msl-sim` ジョブの `target/msl-sim.json` から表に入れる。
- 入れないもの: `qss`（硬い系は壁時計の上限で止まり時間がかかる）、固定刻み euler・heun・rk4・impeuler・trapezoid（既定の刻み＝出力の間隔では精度が足りず、管の本数に意味が出ない）、`esdirk23`（`trbdf2` と刻みの端の値が一致）、別名。
- cvode・ida は入れた。`cargo xtask` の alias は feature を渡せないので、`cargo run --release -q -p xtask --features globe-mode-exec/sundials -- msl-sim …` を直接呼ぶ。ワーカーは xtask 自身（`current_exe`）なので feature はワーカーに届く（`cargo tree -p xtask --features globe-mode-exec/sundials -i sundials-sys` で確かめた）。ランナーの cmake でビルドできた。rust-cache のキーは sundials つきと分けた。
- baseline は渡さない。`fail-fast: false`、`solvers-report` は `if: always() && msl-sim が成功`。
- 表の列: 対象・実行できた・管に収まった/比べた・割合・GM-U0004・初期化（E11001）・積分（E11002）・イベント（E11003）・時間切れ（E11005）・外から停止・その他・秒。「その他」は対象から他の列を引いた残りで、実際は `GM-U0005` だった。

## やったこと

- `.github/workflows/msl-sim.yml`: ジョブ `solvers`・`solvers-report` を追加（既存の `msl-sim`・`split-all` は変えず、冒頭のコメントに 1 行足しただけ）。
- `src/Rust/xtask/src/solvers.rs`（新規）と `main.rs` の dispatch（`mod`・variant・match の腕・冒頭の docコメント）。単体試験 1 本。
- `doc/msl-solvers.md`（artifact `msl-solvers` を取ってコミット）、セッション文書、README の行、roadmap の節、CLAUDE.md §2 に 1 行。

## 検証

- run 41194959752（`gh workflow run msl-sim.yml --ref feature/s050-solver-matrix-ci`）: msl-sim 77 分、split-all 41 分、solvers 14 ジョブすべて成功（63〜174 分）、solvers-report 成功。
- `cargo test -p xtask solvers`: 1 件 OK。`cargo clippy -p xtask --all-targets -- -D warnings`、`cargo fmt --check`: 通過。

ソルバごとの数字（対象 356 本、`--tolerance-scale 0.1`）:

| ソルバ | 実行 | 管 / 比べた | 割合 | 秒（ワーカー合計） |
| --- | ---: | ---: | ---: | ---: |
| auto | 309 | 301 / 308 | 97.7% | 3864 |
| bdf | 301 | 293 / 300 | 97.7% | 3294 |
| cerk23 | 288 | 279 / 287 | 97.2% | 10044 |
| cerk34 | 286 | 275 / 285 | 96.5% | 9806 |
| cerk45 | 285 | 274 / 284 | 96.5% | 8624 |
| cvode | 294 | 281 / 293 | 95.9% | 6472 |
| dopri45 | 292 | 280 / 291 | 96.2% | 7372 |
| dopri853 | 283 | 270 / 282 | 95.7% | 8123 |
| esdirk34 | 294 | 285 / 293 | 97.3% | 6883 |
| esdirk45 | 293 | 280 / 292 | 95.9% | 7410 |
| ida | 289 | 283 / 288 | 98.3% | 7861 |
| radau5 | 290 | 279 / 289 | 96.5% | 8233 |
| sdirk34hw | 294 | 285 / 293 | 97.3% | 7946 |
| trbdf2 | 295 | 280 / 294 | 95.2% | 7962 |
| tsit45 | 287 | 276 / 286 | 96.5% | 8424 |

- 締め（親のセッション、main の S048・S049・S051・S052 を取り込んだ後）: `mise run check` ok（test 45 件 ok）。`check_docs.py` 切れ 0 件（1220 本）、`render_slides.py` デッキ 58 件 / 問題 0 件。セッション記録の `--check` 取りこぼし 0 件。

## 残したこと

- `mise run check` は手元で回していない（RAM 3 GB）。Rust の変更は xtask の追加だけ。
- 時間切れの多い cerk23・cerk34（27 本）の原因。
- 固定刻みと `qss` の行。
- `bdf` 単独の `GM-U0005`（実行時の契約違反）10 本は、auto が dopri45 で救っているが、`bdf` 側の原因は未調査。

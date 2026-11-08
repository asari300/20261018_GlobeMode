# S237 `fmu-export-msl-models`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s237-fmu-export-msl-models` |
| worktree | `.git-worktree/s237-fmu-export-msl-models` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S237-fmu-export-msl-models/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

roadmap の「FMU の Export の残り」のうち「MSL を使うモデル」。MSL の例題 20 本を P5a と P5b で FMU にして FMPy で回し、`globe simulate` と比べて通る本数・落ちる本数・落ち方を表にする。GlobeMode の側で直せる上位 1〜2 つを直し、Rumoca の限界は上流の下書きに貯める。通った 3 本を `fmu-export.yml` に足し、MSL ありのときだけ回る試験を 1 本足す。

## 決めたこと

- 「小さい順」は compile 秒数（`doc/msl-sim.md` の元の JSON）。Clocked は上位を埋めるので除き、パッケージごとに 2 本まで。依頼の 4 本を先頭に。
- 比較は Python（`.github/scripts/fmu_msl_check.py`）で、`compare.rs` と同じ幅の矩形の管。管の外が 1% 以下の信号は合格（FMPy の ME は CVode で事象の時刻が数 ms ずれる）。Rust の `compare.rs` を呼ぶ道は CLI に無い。
- P5a の落ちる分（17 本）は Rumoca の限界で、改変しない。直せるのは P5b の側だった（P5b は MSL を読めなかった）。
- P5b で MSL を読む道: FMU の `resources/lib/` に MSL の `.mo` をすべて入れる（shortcut。閉包への絞り込みは持ち越し）。`resources/<完全修飾名>.mo` はモデルの文面で、ライブラリの中のクラスならコメントだけ。
- 上流の下書き 35 は、ふつうのモデルが当たる初期化の 1 件（状態への `initial equation`）。最小の再現 `E`・`F`・対照 `A`、v0.10.4 の配布バイナリでも再現。

## やったこと

- `globe-mode-exec/src/rumoca/instance.rs`: 関係式の記憶（`update_relation_memory_from_state`）、初期化の事象（`run_initial_event`）、`held_y`（離散の値を保つ）。
- `globe-mode-fmu/src/lib.rs`: `compile_in`・`model_description_lib`・`read_resource` が `resources/lib`、`$` の名前の引用符。`examples/md.rs` の 5 番目の引数。
- 試験: `globe-mode-fmu`（`relation_memory_starts_from_the_initial_point`、`fmpy_runs_an_msl_example_through_resources_lib`、`EnumString` の期待を 1 から 2 に）、`globe-mode-cli/tests/fmu_export.rs`（MSL の 2 本）、`fmpy_run.py` の 10 番目の引数。
- `.github/scripts/fmu_msl_check.py`（新）、`.github/workflows/fmu-export.yml`（MSL の 3 本の step）。
- 文書: ADR-0031 の追記、features・roadmap・CHANGELOG、`doc/upstream.md`・`doc/research/upstream/`（35）、セッション文書。

## 検証

- 測定（FMI 2.0、FMPy 0.3.32、release の `globe`、20 本）: P5a 3/20、P5b 16/20。P5a の FMI 3.0 も通った 3 本は同じ。
- P5b の直す前の最初の測定（`resources/lib` だけ足した状態、15 本目まで）: 通ったのは 6 本。validate が `$` の名前で 5 本落ち、compare が関係式の記憶などで落ちた。直した後は 16 本。
- 手元の試験: `cargo test --release -p globe-mode-fmu`（17 本。`EnumString` の期待を直す前に 1 本が落ちた）。`mise run check` 内の `fmu_export`（MSL の 2 本を含む）と `fmpy_runs_an_msl_example_through_resources_lib` も通った。
- コードの検査（`mise run check`）: 通った（fmt・clippy・test 925 本 ok・deny・`uv sync --locked`）。実行系の `run` の経路は変えていない（`ModelInstance` は `globe-mode-fmu` だけが使う）ので `wasm:node`・`wasm:compare`・`msl:sim` は回していない。
- CI: `fmu-export.yml` run 42494360236 が success（`fmu-export` の MSL の step: P5a の FMI 2.0・3.0 と P5b の 2.0 の 3 本、3.0 の Accelerate が FMPy の ME・CS で ok。fmusim の `mass.s(1)`: P5a の 2.0・3.0 は ME 1.0000407、CS 1.0（0.9999999999999998）、P5b の 2.0・3.0 は ME 1.0000005、CS 1.0。`fmu-native` の 3 OS も success）。`ci.yml -f full=true` run 42494363197 が success。
- `check_docs.py`: 切れ 0 件。`render_slides.py`: S237 のデッキと上流の下書き 35 の 1 枚で問題 0 件

## 残したこと

- P5b の 4 本（ChuaCircuit はカオス、DemonstrateSignalExtrema は刻みの点、DiodeBridge2Pulse・DiodeCenterTap2Pulse は `Math.Mean` の出力が保たれない。原因は未調査）。
- P5b を作る CLI、`resources/lib` の閉包への絞り込み。
- 大きい例題（外部 C 関数・`ModelicaStandardTables`・文字列・配列・record）での測定。
- 上流の下書き 35 に初期化の残り 2 つのメッセージの最小のモデル。

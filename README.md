# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode is planned to run on Linux (Debian family), Windows 11, Android, in the browser (WebAssembly, computing on the visitor's machine), as a CLI, and inside marimo notebooks. Features planned:

- an incremental (salsa-based) language service, plus a pre-simulation check that reports why a model cannot be simulated;
- multiple ODE/DAE solvers, selectable by their OpenModelica and Dymola names;
- FMI 2.0 and 3.0 import and export (Model Exchange, Co-Simulation, Scheduled Execution);
- an Obsidian-style network graph built from `connect()` relationships;
- APIs for Rust, Python, Julia, Zig, Elixir and Gleam.

Phase A uses [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0) behind an adapter as the compiler frontend.

> **Status:** P1 (skeleton) and P2 (lazy queries and checks) done; P3 (simulation) started in S011. The `globe` CLI checks models through salsa queries (whitespace-only edits do not recompile; MSL is loaded once; only the files a model uses are passed to the compiler, so editing an unused file does not recompile), reports structural singularity (S6, perfect matching with a Dulmage–Mendelsohn breakdown), estimates the structural index and state-selection candidates with the Pantelides algorithm (S7), checks the initial conditions by matching the initialization system (S8: the DAE with its derivatives, initial equations and `fixed = true`, so redundant initial conditions and undetermined unknowns are named; discrete variables are told apart by whether a `when` decides them), finds algebraic loops by BLT and greedy tearing and warns about nonlinear iteration variables without a start value (S9), runs the static part of the preflight (S10: parameter and start-value evaluation, the raw `experiment` annotation), and separates compiler limitations (GM-U) from model errors. Switching back to a model checked before no longer re-resolves MSL (about 2.1 s → 49 ms). All 566 MSL 4.1.0 examples get a verdict. S5 (equation/variable balance) agrees with OpenModelica's `omc checkModel()` on all 382 examples that have a verdict (S008; `omc` is only called as a separate process, for comparison). `globe simulate` runs a model after the checks pass (Phase A uses Rumoca's simulation stack: diffsol BDF and Dormand–Prince 5(4)) and writes a CSV; 303 of the 357 non-partial MSL examples that pass the checks run to their stop time, and 292 of the 302 with a reference stay inside the tube (relative tolerance 0.2 %, as csv-compare) around the MSL 4.1.0 reference results (S011, [doc/msl-sim.md](doc/msl-sim.md)). See [doc/roadmap.md](doc/roadmap.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions.

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。P1（骨格）と P2（遅延クエリと検査）まで済み、S011 から P3（シミュレーション）を進めている。CLI `globe` で、アウトラインの表示、connect グラフ、S0〜S10 の検査（方程式数と変数数、構造特異、Pantelides 法による構造インデックスと状態選択の候補、初期化系のマッチングによる冗長な初期条件と初期条件の無い未知数の名指し、BLT と tearing による代数ループと start 値の無い非線形の反復変数、パラメータと start 値の評価と experiment annotation のプリフライト）ができる。検査は salsa のクエリで、空白だけの編集や、モデルが使わないファイルの編集では再コンパイルしない（`globe deps` で使うファイルを見られる）。MSL 4.1.0 の全例題 566 本に判定を出した。S5（方程式数と変数数）は OpenModelica の `omc checkModel()` と、判定のある 382 本すべてで一致した（S008。omc は照合のために別プロセスで呼ぶだけ）。`globe simulate` で、検査を通したモデルを実行して CSV に書ける（Phase A は Rumoca の実行系: diffsol の BDF と Dormand–Prince 5(4)）。検査を通る MSL の例題（partial を除く 357 本）のうち 303 本が終了時刻まで進み、参照結果のある 302 本のうち 292 本が MSL 4.1.0 の参照結果の管（csv-compare と同じ許容差 0.2%）に収まった（S011、[doc/msl-sim.md](doc/msl-sim.md)）。

| 文書 | 内容 |
| --- | --- |
| [doc/features.md](doc/features.md) | OpenModelica と Rumoca を参考に整理した、必要な機能の一覧 |
| [doc/architecture.md](doc/architecture.md) | crate の構成、遅延評価、実行前のアサーション、ソルバ、FMI、connect グラフ |
| [doc/roadmap.md](doc/roadmap.md) | P0〜P8 と Track B（自前のコア）、各段階の完了条件 |
| [doc/licenses.md](doc/licenses.md) | ライセンス方針。依存の許可・条件付き・禁止 |
| [doc/adr/](doc/adr/) | 設計判断の記録 |
| [doc/research/](doc/research/) | 調査の記録（出典 URL と確認日つき） |
| [doc/sessions/](doc/sessions/) | セッションごとの HTML スライドと解説書 |
| [CLAUDE.md](CLAUDE.md) | このリポジトリでの作業ルール |

ライセンスは MIT OR Apache-2.0。依存を足すときは [doc/licenses.md](doc/licenses.md) の区分に従う。

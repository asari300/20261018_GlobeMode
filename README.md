# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode is planned to run on Linux (Debian family), Windows 11, Android, in the browser (WebAssembly, computing on the visitor's machine), as a CLI, and inside marimo notebooks. Features planned:

- an incremental (salsa-based) language service, plus a pre-simulation check that reports why a model cannot be simulated;
- multiple ODE/DAE solvers, selectable by their OpenModelica and Dymola names;
- FMI 2.0 and 3.0 import and export (Model Exchange, Co-Simulation, Scheduled Execution);
- an Obsidian-style network graph built from `connect()` relationships;
- APIs for Rust, Python, Julia, Zig, Elixir and Gleam.

Phase A uses [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0) behind an adapter as the compiler frontend.

> **Status:** P1 (skeleton) done; P2 (lazy queries and checks) in progress. The `globe` CLI checks models through salsa queries (whitespace-only edits do not recompile; MSL is loaded once; only the files a model uses are passed to the compiler, so editing an unused file does not recompile), reports structural singularity (S6, perfect matching with a Dulmage–Mendelsohn breakdown), estimates the structural index and state-selection candidates with the Pantelides algorithm (S7), checks the count of initial conditions (S8), and separates compiler limitations (GM-U) from model errors. All 566 MSL 4.1.0 examples get a verdict. Comparison with OpenModelica (`omc`) is postponed until `omc` is installed. See [doc/roadmap.md](doc/roadmap.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions.

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。P1（骨格）まで済み、P2（遅延クエリと検査）を進めている。CLI `globe` で、アウトラインの表示、connect グラフ、S0〜S8 の検査（方程式数と変数数、構造特異、Pantelides 法による構造インデックスと状態選択の候補、初期条件の数）ができる。検査は salsa のクエリで、空白だけの編集や、モデルが使わないファイルの編集では再コンパイルしない（`globe deps` で使うファイルを見られる）。MSL 4.1.0 の全例題 566 本に判定を出した。omc との照合は、omc を入れてから行う。

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

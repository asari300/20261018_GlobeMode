# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode is planned to run on Linux (Debian family), Windows 11, Android, in the browser (WebAssembly, computing on the visitor's machine), as a CLI, and inside marimo notebooks. Features planned:

- an incremental (salsa-based) language service, plus a pre-simulation check that reports why a model cannot be simulated;
- multiple ODE/DAE solvers, selectable by their OpenModelica and Dymola names;
- FMI 2.0 and 3.0 import and export (Model Exchange, Co-Simulation, Scheduled Execution);
- an Obsidian-style network graph built from `connect()` relationships;
- APIs for Rust, Python, Julia, Zig, Elixir and Gleam.

Phase A uses [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0) behind an adapter as the compiler frontend.

> **Status:** P1 (skeleton). The `globe` CLI can outline a Modelica file and check a model through DAE balance using Rumoca. See [doc/roadmap.md](doc/roadmap.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions.

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。現在は P1（骨格）の段階。CLI `globe` で、アウトラインの表示と、DAE までの検査（方程式数と変数数）ができる。

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

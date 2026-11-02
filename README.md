# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode is planned to run on Linux (Debian family), Windows 11, Android, in the browser (WebAssembly, computing on the visitor's machine), as a CLI, and inside marimo notebooks. Features planned:

- an incremental (salsa-based) language service, plus a pre-simulation check that reports why a model cannot be simulated;
- multiple ODE/DAE solvers, selectable by their OpenModelica and Dymola names;
- FMI 2.0 and 3.0 import and export (Model Exchange, Co-Simulation, Scheduled Execution);
- an Obsidian-style network graph built from `connect()` relationships;
- APIs for Rust, Python, Julia, Zig, Elixir and Gleam.

Phase A uses [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0) behind an adapter as the compiler frontend.

> **Status:** P1 (skeleton) and P2 (lazy queries and checks) done; P3 (simulation) started in S011, and all three P3 exit criteria are met as of S012. The `globe` CLI checks models through salsa queries (whitespace-only edits do not recompile; MSL is loaded once; only the files a model uses are passed to the compiler, so editing an unused file does not recompile), reports structural singularity (S6, perfect matching with a Dulmage–Mendelsohn breakdown), estimates the structural index and state-selection candidates with the Pantelides algorithm (S7), checks the initial conditions by matching the initialization system (S8: the DAE with its derivatives, initial equations and `fixed = true`, so redundant initial conditions and undetermined unknowns are named; discrete variables are told apart by whether a `when` decides them), finds algebraic loops by BLT and greedy tearing and warns about nonlinear iteration variables without a start value (S9), runs the static part of the preflight (S10: parameter and start-value evaluation, the raw `experiment` annotation), and separates compiler limitations (GM-U) from model errors. Switching back to a model checked before no longer re-resolves MSL (about 2.1 s → 49 ms). All 566 MSL 4.1.0 examples get a verdict. S5 (equation/variable balance) agrees with OpenModelica's `omc checkModel()` on all 382 examples that have a verdict (S008; `omc` is only called as a separate process, for comparison). `globe simulate` runs a model after the checks pass (Phase A uses Rumoca's simulation stack: diffsol BDF and Dormand–Prince 5(4)) and writes a CSV; 303 of the 357 non-partial MSL examples that pass the checks run to their stop time, and 292 of the 302 with a reference stay inside the tube (relative tolerance 0.2 %, as csv-compare) around the MSL 4.1.0 reference results (S011, [doc/msl-sim.md](doc/msl-sim.md)). In S012 the same check → compile → simulate path runs as WebAssembly (`wasm32-unknown-unknown` + wasm-bindgen) on Node, and BDF matches the native build on all validation models; nine solvers are available (`bdf`, `dopri45`, `auto`, plus `esdirk34`, `trbdf2`, `tsit45` and the fixed-step `euler`, `heun`, `rk4`, plugged into Rumoca's model-exchange host); `auto` falls back to Dormand–Prince when BDF fails; tunable parameters can be overridden (`--param`; since S024 the override is compiled into the model, so it also reaches values copied by same-name modifications and time events); results can be written as MAT v4 (`--format mat`). With the new defaults, 309 of 356 MSL examples run to the stop time (303 in S011) and 299 of the 308 compared (97.1%) stay inside the reference tube (292 / 302 in S011). See [doc/roadmap.md](doc/roadmap.md).

Editor features (formatter, lint, LSP, VS Code extension), the current rules, likely next features and the processing flow are listed in the Japanese section below ("エディタ機能").

## Install

The `globe` CLI is not on crates.io yet (it depends on Rumoca through a git dependency, see [ADR-0002](doc/adr/0002-rumoca-adapter.md)). Two ways:

- Download `globe-<version>-<target>.tar.gz` (Linux x86_64 / aarch64) or `.zip` (Windows x86_64) from the [Releases](https://github.com/asari300/20261018_GlobeMode/releases) page, check it against `SHA256SUMS`, and put `globe` on your `PATH`. Release builds are made by [release.yml](.github/workflows/release.yml); they do not include the optional `sundials` solvers (`cvode`, `ida`).
- Or build from a checkout: `cargo install --path src/Rust/mod/globe-mode-cli` (add `--features sundials` if cmake and a C compiler are installed).
- Python: the Releases page also has a wheel (`globe_mode-<version>-cp312-abi3-<platform>.whl`, not on PyPI). The marimo / Jupyter plot widget needs the `widget` extra: `pip install "./globe_mode-<version>-cp312-abi3-<platform>.whl[widget]"` (`pip install "globe-mode[widget]"` once it is on an index).

Then `globe check <path> -m <Model>` and `globe simulate <path> -m <Model>`. Changes are listed in [CHANGELOG.md](CHANGELOG.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions.

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。P1（骨格）と P2（遅延クエリと検査）まで済み、S011 から P3（シミュレーション）を進めている（S012 で P3 の完了条件を 3 つとも満たした）。CLI `globe` で、アウトラインの表示、connect グラフ、S0〜S10 の検査（方程式数と変数数、構造特異、Pantelides 法による構造インデックスと状態選択の候補、初期化系のマッチングによる冗長な初期条件と初期条件の無い未知数の名指し、BLT と tearing による代数ループと start 値の無い非線形の反復変数、パラメータと start 値の評価と experiment annotation のプリフライト）ができる。検査は salsa のクエリで、空白だけの編集や、モデルが使わないファイルの編集では再コンパイルしない（`globe deps` で使うファイルを見られる）。MSL 4.1.0 の全例題 566 本に判定を出した。S5（方程式数と変数数）は OpenModelica の `omc checkModel()` と、判定のある 382 本すべてで一致した（S008。omc は照合のために別プロセスで呼ぶだけ）。`globe simulate` で、検査を通したモデルを実行して CSV に書ける（Phase A は Rumoca の実行系: diffsol の BDF と Dormand–Prince 5(4)）。検査を通る MSL の例題（partial を除く 357 本）のうち 303 本が終了時刻まで進み、参照結果のある 302 本のうち 292 本が MSL 4.1.0 の参照結果の管（csv-compare と同じ許容差 0.2%）に収まった（S011、[doc/msl-sim.md](doc/msl-sim.md)）。S012 で、検査 → コンパイル → 実行を WebAssembly（wasm32-unknown-unknown + wasm-bindgen）にして Node で回し、bdf は検証用のモデルすべてでネイティブと一致した（P3 の完了条件を 3 つとも満たした）。ソルバは 9 つ（`bdf`・`dopri45`・`auto` に、Rumoca の FMI ME 風の host に差す自前の `esdirk34`・`trbdf2`・`tsit45` と固定刻みの `euler`・`heun`・`rk4`）。`auto` は BDF が失敗したら dopri45 で回し直す。tunable なパラメータは `--param` で変えられ（S024 から上書きを組み込んでコンパイルし直すので、同じ名前の修飾で写された値や時間イベントの時刻にも届く）、結果は MAT v4 でも書ける（`--format mat`）。新しい既定で、MSL の例題 356 本のうち 309 本が終了時刻まで進み（S011 は 303 本）、参照結果と比べた 308 本のうち 299 本（97.1%）が管に収まった（S011 は 292 / 302 本）。

### インストール

`globe` の CLI は、Releases から `globe-<version>-<target>.tar.gz`（Linux x86_64 / aarch64）か `.zip`（Windows x86_64）を取り、`SHA256SUMS` と照らして `PATH` に置く。ソースからは `cargo install --path src/Rust/mod/globe-mode-cli`。crates.io には出していない（Rumoca を git 依存で使うため。ADR-0002）。配布物に `sundials` のソルバ（`cvode`・`ida`）は入っていない。変更点は [CHANGELOG.md](CHANGELOG.md)。

Python の wheel（`globe_mode-<version>-cp312-abi3-<platform>.whl`）も Releases にある（PyPI には出していない）。marimo・Jupyter の折れ線の widget（anywidget）は extra `widget` で入れる: `pip install "./globe_mode-<version>-cp312-abi3-<platform>.whl[widget]"`（index に載れば `pip install "globe-mode[widget]"`）。

### エディタ機能（整形・lint・LSP）

`.mo` は CLI（`globe fmt` / `globe lint` / `globe lsp`）、VS Code 拡張（[src/VSCode/globe-mode](src/VSCode/globe-mode)、Shift+Alt+F で整形）、ブラウザの UI と Web アプリ（[doc/web-apps.md](doc/web-apps.md)）から同じ規則で整形・検査できる。規則の詳細は [doc/format-style.md](doc/format-style.md)。

#### 処理の流れ

```mermaid
flowchart LR
  src[".mo の本文"] --> entry{"入口<br/>CLI / VS Code / ブラウザ / Web アプリ"}
  entry -->|整形| fmt["globe-mode-fmt<br/>字句 → 構造 → 80 桁の配置<br/>annotation の並べ替え・HTML の段付け"]
  fmt --> guard{"空白以外の<br/>トークンが不変？<br/>（HTML の文字列は空白以外の文字）"}
  guard -->|はい| out["整形した本文"]
  guard -->|いいえ| keep["元のまま + エラー"]
  entry -->|lint| lint["globe-mode-fmt の lint<br/>GM-L0001〜0004"]
  entry -->|lint / LSP| outline["Rumoca のアウトライン<br/>→ クラスの索引"]
  outline --> types["connect の型の突き合わせ<br/>GM-L0005"]
  outline --> graph["connect グラフ"]
  entry -->|検査| check["S0〜S10 の検査<br/>（globe check・LSP の診断）"]
  check --> sim["コンパイル → 実行"]
```

#### 今の規則

| 種類 | 規則 | 既定 | 切り方 |
| --- | --- | --- | --- |
| 整形 | インデント 2・行幅 80 桁。収まらない呼び出しと配列は開き括弧で改行 | オン | `--indent 4` |
| 整形 | 図形プリミティブの引数を並べ替える（位置・形状 → points → fill → line） | オン | — |
| 整形 | `annotation` を experiment → Diagram → Icon → Documentation、`Documentation` を header → info → revisions に並べ替える | オン | — |
| 整形 | 図形プリミティブに `origin = {0, 0}` を足す | オン | `--no-insert-origin` |
| 整形 | `within` の後・クラス頭の後・`end` の前に空行 | オン | — |
| 整形 | `equation` / `algorithm` の前後に空行 | オン | `--no-blank-around-sections`、VS Code の `globeMode.blankAroundSections` |
| 整形 | `Documentation` の HTML を改行とインデントで整える（空白だけを変える。lint はしない。S062） | オン | `--no-format-html`、VS Code の `globeMode.formatHtml` |
| 整形 | 演算子の両側に空白、カンマは後ろだけ、行末の空白を消す | オン | — |
| lint | `line-too-long`（GM-L0001）: 整形しても 80 桁を超える行 | warning | `globe lint --disable <規則>` |
| lint | `class-name-pascal-case`（GM-L0002）: クラス名が大文字で始まらない | warning | 同上 |
| lint | `missing-description`（GM-L0003）: クラスに説明の文字列が無い | warning | 同上 |
| lint | `annotation-order`（GM-L0004）: annotation の引数が上の順でない。整形で直る | 通知 | 同上 |
| lint | `connect-type-mismatch`（GM-L0005）: connect の両端のコネクタの型が合わない（S062） | warning | 同上 |
| 検査 | S0〜S10（構文、方程式数と変数数、構造特異、初期化、代数ループ、プリフライト） | error / warning | — |

#### 今後必要になりそうな機能

| 機能 | 今 | 理由 |
| --- | --- | --- |
| LSP がワークスペースのフォルダ全体を読む | 開いたファイルだけ | 開いていないファイルの型を connect の型の突き合わせで解決するため |
| MSL などライブラリの型での connect の型の突き合わせ | ワークスペースの型だけ | `--library` で読んだ型は索引に無く、突き合わせを省いている |
| connect の向きの検査（input 同士、output 同士） | 無い | 型が合っていてもつなげない組み合わせがある |
| 配列の大きさ・単位の食い違いの検査 | 無い | 型の検査の次の段 |
| 定義ジャンプ・参照・リネーム | 無い | features.md の P4 の残り |
| `globe fmt --diff` と `--exclude` | 無い | ruff と同じ操作性にするため |
| 整形の設定ファイル | CLI のフラグと VS Code の設定だけ | プロジェクトごとに規則を揃えるため |
| ブラウザの UI と Web アプリで空行・HTML の規則を切る口 | 既定のオンだけ | VS Code・CLI と揃えるため |
| `src/Modelica` の既存ファイルを今の規則で整形 | していない | 差分が大きいので、ユーザーの判断を待つ |
| FMI の読み込み・書き出し | P5 の準備中 | [doc/roadmap.md](doc/roadmap.md) |

| 文書 | 内容 |
| --- | --- |
| [doc/features.md](doc/features.md) | OpenModelica と Rumoca を参考に整理した、必要な機能の一覧 |
| [doc/architecture.md](doc/architecture.md) | crate の構成、遅延評価、実行前のアサーション、ソルバ、FMI、connect グラフ |
| [doc/roadmap.md](doc/roadmap.md) | P0〜P8 と Track B（自前のコア）、各段階の完了条件 |
| [doc/licenses.md](doc/licenses.md) | ライセンス方針。依存の許可・条件付き・禁止 |
| [doc/upstream.md](doc/upstream.md) | 上流（Rumoca・diffsol）への issue の下書きの一覧、提出の順と手順 |
| [doc/adr/](doc/adr/) | 設計判断の記録 |
| [doc/research/](doc/research/) | 調査の記録（出典 URL と確認日つき） |
| [doc/sessions/](doc/sessions/) | セッションごとの HTML スライドと解説書 |
| [CLAUDE.md](CLAUDE.md) | このリポジトリでの作業ルール |

ライセンスは MIT OR Apache-2.0。依存を足すときは [doc/licenses.md](doc/licenses.md) の区分に従う。

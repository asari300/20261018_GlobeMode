# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/); versions will follow [Semantic Versioning](https://semver.org/lang/ja/) from v0.1.0. 日本語で書く（英語の要約は [README.md](README.md)）。各行の S 番号は [doc/sessions/README.md](doc/sessions/README.md) の索引と `log/work/` の作業記録に対応する。

## [Unreleased]

v0.1.0 に向けた S001〜S041 の要点。version はまだ 0.0.1 で、crates.io には出していない（Rumoca の git 依存。[ADR-0002](doc/adr/0002-rumoca-adapter.md)）。

### 運用・計画（P0）

- S001: 調査と計画、運用基盤（worktree、mise・uv・cargo の分担、ライセンス規則、セッション記録）。
- S006: WSL2 から Windows の LM Studio（Bonsai-27B）を副レビュアーに呼ぶスキル `/local-review`。
- S033: リリースの見積もりと、エージェントに渡す並行作業の指示書 WP-0〜WP-J（`doc/plan/`）、Codex の監査のための `log/review/`。

### P1 骨格

- S002: workspace の骨格、Rumoca を `Frontend` のアダプタで包む、cargo-deny、Playwright。
- S003: MSL 4.1.0 の例題 20 本を Rumoca の CLI と突き合わせて 20 本一致（P1 完了）。

### P2 遅延クエリと検査

- S004: salsa の遅延クエリと `connect()` のグラフ、検証用パッケージ `GlobeModeValidation`。
- S005: `check` のクエリ化（`source_shape` の防火壁）、GM-U（コンパイラの限界）の振り分け（[ADR-0007](doc/adr/0007-unsupported-classification.md)）、S6（マッチングと Dulmage–Mendelsohn 分解）。MSL の全例題 566 本に判定。
- S007: `deps_closure`（検査に渡すファイルの絞り込み）、S7（Pantelides 法と状態選択の候補）、S8（初期化系の検査）。
- S008: omc 1.27.1 との照合。S5 は判定のある 382 本すべてで一致（P2 完了。[ADR-0010](doc/adr/0010-omc-comparison.md)）。
- S009: 初期化系のマッチングで冗長な初期条件と初期条件の無い未知数を名指し（[ADR-0011](doc/adr/0011-init-system-matching.md)）。
- S010: S9（BLT と tearing による代数ループ、非線形の反復変数の start 値）、S10（プリフライトの静的な部分）。

### P3 シミュレーション

- S011: `globe simulate`、実行系を `globe-mode-exec` に閉じ込める（[ADR-0013](doc/adr/0013-exec-backend.md)）、結果の CSV と参照結果の管。MSL の例題 357 本のうち 303 本が終了時刻まで進み、292 / 302 本が管に収まる。
- S012: 検査 → コンパイル → 実行を WebAssembly で（`globe-mode-wasm`、Node 上の bdf、[ADR-0014](doc/adr/0014-session-route-wasm.md)）。`auto`（BDF が失敗したら dopri45）、`--param`、MAT v4。
- S013〜S016: MSL の突き合わせの許容誤差を参照結果と同じにし（[ADR-0015](doc/adr/0015-auto-and-reference-tolerance.md)）、実行の段をワーカーに知らせ（[ADR-0016](doc/adr/0016-preflight-on-build-lowering.md)）、lowering の上限と回し直しを入れた（[ADR-0017](doc/adr/0017-lowering-limit-and-hard-timeout-retry.md)）。S015 で 309 本が実行でき 301 / 308 本が管に収まる。
- S017: Raspberry Pi（aarch64）で回せるようにした（[ADR-0019](doc/adr/0019-rpi-aarch64-environment.md)）。
- S018〜S020: 初期化の解き直しの検査 GM-W10004（[ADR-0020](doc/adr/0020-init-resettle-check.md)・[ADR-0021](doc/adr/0021-resettle-only-when-relations-flip.md)）、重い検査（`msl:funnel`・`msl:sim`・`wasm:node`・`wasm:compare`）の再実行。
- S021〜S023: esdirk34 の詰め（Newton 法の失敗の数え方、ヤコビ行列を取る点、最初の刻みの下限）。全体 356 本で実行 286・管 277。
- S024・S025: `--param` が同じ名前の修飾で写された値と時間イベントに届かず黙って誤っていたのを直した（[ADR-0022](doc/adr/0022-structural-parameter-overrides.md)）。配列は要素だけ受け、配列全体は GM-E10006 で断る。
- S026〜S031: 自前のソルバを足した。固定刻みの陰的 `impeuler`・`trapezoid`、SDIRK の `esdirk23`・`esdirk45`・`sdirk34hw`、`dopri853`、連続の陽的 RK の `cerk23`・`cerk34`・`cerk45`、`radau5`、`qss`（QSS2）。
- S032・S034: feature `sundials` を立てたビルドだけに SUNDIALS 7.1.1 の `cvode` と `ida` を足した（既定・wasm・CI のビルドは変わらない）。
- S036: wasm を Web Worker で（ブラウザの module Worker と Node の `worker_threads` で共用）。panic の後は作り直して再生する。
- S037: wasm の MSL の分割読み込み。読み込み後の線形メモリ 262 → 62 MiB、`ChuaCircuit` の後 513 → 113 MiB。`msl:funnel`・`msl:sim` を GitHub Actions で手動で回すワークフロー。

### P4 UI・LSP

- S035: `globe-mode-protocol`（`--json` と wasm の JSON の serde 型）と facade `globe-mode`（`Workspace`）。CLI と wasm はその経由になり、JSON は snapshot 試験で 1 バイトも変えていない。
- S038: stdio の LSP サーバ `globe lsp`（診断・documentSymbol・hover）。
- S040: [ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md) Desktop と Web のプロセス構成（Desktop は `globe` を Tauri の sidecar に、Web は Web Worker のまま）。コードは変えていない。
- S041: `src/Svelte` にブラウザ UI の骨格（Svelte 5 + CodeMirror 6 + `WorkerBackend` + 診断パネル）。wasm に `check` を足した。

### P7 言語 API

- S039: Python の wheel `globe_mode`（`globe-mode-py`、PyO3・abi3-py312・maturin）。load_dir → check → graph → simulate → polars の表の参照シナリオを `pytest` で通した。

### 既知の限界

- Rumoca は git 依存の固定 rev（v0.10.2）で、crates.io には出せない。
- 配布するバイナリに `sundials` のソルバ（`cvode`・`ida`）は入れない（cmake と C コンパイラが要る）。
- S041 の時点で、Desktop（Tauri）、プロット、connect グラフの UI、FMI は入っていない（[doc/roadmap.md](doc/roadmap.md)）。

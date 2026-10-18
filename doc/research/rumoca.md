# Rumoca 調査

- 確認日: 2026-10-18
- 確認できなかった項目には【未確認】と書いた。
- 結論: GlobeMode の Phase A では、Rumoca を**コンパイラのフロントエンドとして git 依存で使う**（[ADR-0002](../adr/0002-rumoca-adapter.md)）。

## 概要

| 項目 | 内容 |
| --- | --- |
| リポジトリ | <https://github.com/CogniPilot/rumoca>（中心的なコミッタは jgoppert） |
| ライセンス | **Apache-2.0**（ルートの `LICENSE` を確認）。**NOTICE ファイルは無い** |
| 版 | 最新の GitHub Release は v0.10.2（2026-10-04）。main のワークスペースは 0.10.3 |
| crates.io | **0.7.28（2025-12-27）で止まっている**。0.8 以降は GitHub Releases（Python wheel と `rumoca` / `rumoca-lsp` のバイナリ）だけで配布 |
| 活動 | 非常に活発。2026-10-08〜16 の間に約15コミット。star は約140 |
| ツールチェーン | `rust-toolchain.toml` が `nightly-2026-02-27` を固定している（wasm のスレッド対応、wasm-bindgen-rayon のため）。target は `wasm32-unknown-unknown` と `wasm32-wasip2` |
| 論文 | arXiv 2606.14998（2026-06）、Modelica Conference の論文（<https://ecp.ep.liu.se/index.php/modelica/article/view/1388>） |

## ワークスペースの構成（約57 crate）

```text
IR:        rumoca-ir-ast → rumoca-ir-flat → rumoca-ir-dae → rumoca-ir-solve  (+ ir-galec)
フェーズ:  phase-parse / resolve / typecheck / instantiate / flatten / dae /
           structural / solve / autodiff / codegen
集約:      rumoca-compile（純粋なパイプライン。CLI や通信は含まない）
シミュ:    rumoca-sim, rumoca-solver(-diffsol | -rk45), exec-cranelift / exec-mlir / exec-wasm
ツール:    tool-lsp（tower-lsp 0.20）, tool-fmt, tool-lint
バインド:  bind-python（pyo3 0.22）, bind-wasm
実行時:    transport-udp / websocket / zenoh, input-gamepad / keyboard
テスト:    rumoca-test-msl, rumoca-contracts（仕様テスト）
```

- **パーサ**: `parol =4.2.2` と `scnr2`。打鍵ごとに走らせられる程度にはエラーに強いとされる。
  - 文法が LL(k) か LALR(1) かは【未確認】。
- **パイプライン**: parse → resolve（安定した `DefId`）→ typecheck → instantiate → flatten → DAE → 構造解析 → Solve IR。
  - instantiate では modifier、extends、**redeclare**、条件付きコンポーネントを処理する。
  - 構造解析ではマッチング、BLT/Tarjan、Pantelides、tearing、初期化の計画を行う。
- **インクリメンタル計算**: salsa のようなクエリ基盤は**見当たらない**。GlobeMode がここを補う（[ADR-0005](../adr/0005-lazy-query-db.md)）。
- **コード生成**: MiniJinja のテンプレート。`target.toml` と Jinja で独自ターゲットを足せる。組み込みのターゲットは次のとおり。
  - base-modelica, flat-modelica, dae-modelica
  - c-ode, casadi-ode, jax-ode, cuda-ode, wgsl-ode, rust-ode, rust-fixed-ode, mlir
  - **fmi2, fmi3**, fmi-ls-wasm
  - galec 系
- **シミュレーション**: ソルバは `auto` / `bdf`（diffsol 0.13、MIT）/ `rk-like`。
  - 対応: ゼロ交差の検出、`sample` の正確なスケジューリング、AD による Jacobian、感度解析と随伴、A/B/C/D 線形化。
  - イベントの多いモデルでは止まることがある。

## 言語の対応状況（MSL 4.1.0 の例題。2026-06-05 時点）

| 段階 | 通過したモデル数 |
| --- | --- |
| parse | 566 |
| flatten | 562 |
| DAE | 477 |
| balanced | 464 |
| 初期化に成功 | 224 |
| シミュレーションに成功 | 153 |
| OMC と高い一致 | 比較した 143 本のうち 89 本 |

- **対応済み**: extends、modifier、connect/flow、配列、関数、when/pre/edge/reinit/sample。
- **0.7.28 時点で未実装**だったもの: stream / expandable / overconstrained connector、演算子オーバーロード、state machine。
  - 現在の状況は【未確認】。v0.10.1 でも Fluid は作業中だった。
- GlobeMode は、これらを「コンパイラが未対応」の **GM-U コード**として「モデルの誤り」と分けて報告する（[architecture.md](../architecture.md#32-遅延評価とシミュレーション前のアサーション)）。

## 依存として取り込むときの注意

`rumoca`（umbrella crate）を**使わない**。理由はデフォルト feature にある。

```toml
default = ["scheduled-sim", "scenario-config", "input-keyboard", "input-gamepad",
           "transport-udp", "transport-zenoh", "viewer-web", "process-control",
           "native-allocator", "fmu-packaging", "component-build"]
```

- `transport-zenoh` は zenoh（EPL-2.0 OR Apache-2.0）を引き込む。gamepad・UDP・web viewer も IDE のコアには要らない。
- 依存してよいのは次の crate だけ。
  - `rumoca-compile`
  - `rumoca-phase-codegen`
  - 必要な feature だけを有効にした `rumoca-sim`
  - `rumoca-solver-diffsol`
- ワークスペース側の主な外部依存とライセンス:
  - faer 0.24（MIT）、minijinja（Apache-2.0）、parol（MIT OR Apache-2.0）、diffsol 0.13（MIT）
  - cranelift 0.125（Apache-2.0 WITH LLVM-exception）、tower-lsp 0.20、pyo3 0.22、wasm-bindgen
- 推移的依存を含めた確認は P1 で `cargo deny check` を回して行う（[licenses.md](../licenses.md)）。

## 参考にする設計

- IR を層に分け、フェーズを crate ごとに分ける構成（GlobeMode の中立 IR の境界の参考にする）
- 構造解析（マッチング・BLT・Pantelides・tearing）の実装
- テンプレートでコード生成する方式と `target.toml`
- LSP と WASM プレイグラウンドの構成、MSL のファネル計測（`rumoca-test-msl`）

## 出典

- <https://github.com/CogniPilot/rumoca>
- <https://github.com/CogniPilot/rumoca/releases>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/Cargo.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/rust-toolchain.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/crates/rumoca/Cargo.toml>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/user-guide/src/simulation/solvers.md>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/user-guide/src/language/support-status.md>
- <https://raw.githubusercontent.com/CogniPilot/rumoca/main/docs/dev-guide/src/tooling/msl-baseline-promotion-analysis-2026-06-05.md>
- <https://docs.rs/crate/rumoca/latest>
- <https://crates.io/api/v1/crates/rumoca/versions>

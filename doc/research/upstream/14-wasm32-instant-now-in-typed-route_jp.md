# 下書き（未提出）14: 型付きの実行の道が `std::time::Instant::now()` を呼び、wasm32 で panic する

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [14-wasm32-instant-now-in-typed-route.md](14-wasm32-instant-now-in-typed-route.md)、1 枚の解説は [_jp.html](14-wasm32-instant-now-in-typed-route_jp.html)。再現用の `.mo` は無い（本文の手順）。

判定（GlobeMode、S012。S162 で v0.10.4 のソースを読んで確認）: **Rumoca の移植性の穴**。v0.10.4 と main のソースに同じ呼び出しが残る。wasm32 のビルドはしていない【未確認】。

## 概要

`rumoca_sim::build_simulation` → `check_prepared_initialization` → `run_prepared_simulation`（CLI が使う道）と `rk45::simulate` が `std::time::Instant::now()` を呼ぶ。`wasm32-unknown-unknown` には時計が無いので panic する（`RuntimeError: unreachable`）。`web-time` / `instant` を使うか、時計を渡せる（または時計無しにできる）ようにして、ブラウザで型付きの道を使えるようにしたい。

## 再現手順

1. `wasm32-unknown-unknown` 向けに wasm-bindgen で `rumoca_sim::build_simulation` を呼む crate を作る。
2. Node v24 で小さなモデルを回す。

小さな自己完結のプログラムは【未作成】。GlobeMode の試作が観測の元。

## 期待

道が組み上がって走る（時間制限には時計が要るが、任意にできる）。

## 実際

`RuntimeError: unreachable ... <std::time::Instant>::now ... build_simulation`（GlobeMode S012、Node v24.22.0）。

## ソースの再確認（S162）

- v0.10.4 と main（`590ac739`）で、`crates/rumoca-sim/src/diffsol.rs`（202・208 行）と `crates/rumoca-sim/src/rk45.rs`（134・139 行）が `use std::time::Instant;` のまま `Instant::now()` を呼ぶ。
- `crates/rumoca-solver/src/runtime/timeout.rs` は wasm32 で `instant` crate に切り替える（`Cargo.toml` の `cfg(all(target_arch = "wasm32", target_os = "unknown"))`）。残るのは `rumoca-sim` だけ。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）のソースを読んだ。main（`590ac739`）はビルドしていない【未確認】。
- 対象は wasm32-unknown-unknown と wasm-bindgen、Node v24.22.0。

## GlobeMode 側の回避

あり（S012、[ADR-0014](../../adr/0014-session-route-wasm.md)）。`Instant::now()` を呼ばない FMI-ME 風のホスト（`rumoca_solver::fmi_me`）に BDF/RK の plugin をつなぐ。そのため JIT の選択（`admitted_native_execution_backend`、当時 `pub(crate)`）を使えず、ネイティブでも解釈器で走る。

## 備考

- 関連の要望: `admitted_native_execution_backend` の公開。diffsol の `ClosureNoJac` が非公開なので、ヤコビ行列つきの `rhs_implicit` で陽的 RK を組む。どちらも API の可視性の要望で、別の下書きにはしていない。
- getrandom 0.3 は cfg のほかに `wasm_js` feature も要る。これは利用側の設定で、バグではない。

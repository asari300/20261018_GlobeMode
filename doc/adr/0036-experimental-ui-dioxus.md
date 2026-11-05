# ADR-0036 Dioxus 版 UI は Leptos 版（ADR-0035）と同じ作りで、`dx` を使わず wasm-bindgen の流儀でビルドする

- 状態: Proposed（2026-11-05、S155。ブラウザ（Linux）の 1 列だけ測った）
- 関係: [ADR-0035](0035-experimental-ui-leptos.md)（共通の決定: 機能の絞り方、facade 直結、別の workspace、測り方、CI に入れない）、[ADR-0034](0034-ui-bench-method.md)（測り方）、[ADR-0023](0023-desktop-and-web-process-layout.md)（本線の構成。ここでは試験版だけの例外）

## 背景

[roadmap](../roadmap.md) の P7 の完了条件「`doc/bench/ui.md` に 3 UI × 3 OS の比較を書く」の 3 つ目の UI。Leptos 版（S153）を型にして、同じ機能・同じ測り方で Dioxus 版を作る。**ここには Leptos 版との違いだけを書く。** 機能（textarea・検査・実行・MSL・ダークモード・`?smoke=1`）、facade `globe-mode` を同じ wasm にリンクすること（メインスレッドが止まるのは試験版の例外）、別の `[workspace]`・`Cargo.lock`・`deny.toml`、ルートと同じ diffsol の `[patch]`、CI に入れないことは ADR-0035 と同じ。

## 決定

1. **依存**: Dioxus 0.7.10（MIT OR Apache-2.0、2026-07-30 公開の最新の安定版。0.8.0-alpha.3 は採らない）。`default-features = false, features = ["minimal", "web"]`。既定の feature の devtools・logger・asset・document・cli-config などを引き込まない。[licenses.md](../licenses.md) に足し、`cargo deny` を leptos と同じ流儀の `dioxus/deny.toml` で通す。出典: https://crates.io/crates/dioxus （確認 2026-11-05）。
2. **ビルドの道**: `dx`（dioxus-cli）は使わない。`mise run dioxus:build` は `leptos:build` と同じ `cargo build --release --target wasm32-unknown-unknown` → `wasm-bindgen --target web` で、ビルドの道具は増えない。Dioxus は `index.html` の `<div id="main">` に載るので、`dx` が作る HTML の代わりに `index.html` に置く。`dx` が足す最適化（wasm-opt、アセットのハッシュ化）は無い。そのぶんバンドルの大きさは `dx build --release` と同じではない。
3. **状態**: Leptos の `RwSignal` に当たるのは Dioxus の `Signal`（Copy）。画面の状態を `Ui` の構造体にまとめ、`check` / `simulate` / `load_msl` / `smoke` を同じ名前で持つ。読みは `peek()`（購読しない）。
4. **描画の待ち**: smoke の `decay_ms` は、Leptos 版の `tick()` に当たるものとして `setTimeout(0)` を待つ（`js-sys` を 1 つ足す。MIT OR Apache-2.0、wasm-bindgen と同じ組）。
5. **測り方**: `bench_ui.py browser --ui dioxus` が `dioxus/dist/` を配り、`dioxus/linux` の列に入れる。試験は `dioxus/tests/test_dioxus.py`（`test_leptos.py` と同じ内容）。

## 結果

- 数字は [bench/ui.md](../bench/ui.md) と [S155 の解説書](../sessions/S155-ui-dioxus/解説書.md)。
- 悪い: Leptos 版と同じく、メインスレッドが止まり、中断も効かない。本線の UI にはならない。
- 悪い: `dx` を使わないので、Dioxus の標準の道（ホットリロード、アセット管理、`Dioxus.toml`）は使わない。比較には要らないが、本線に採るなら見直す。

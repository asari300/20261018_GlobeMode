# ADR-0035 Leptos 版 UI は UI と計算を 1 つの wasm に入れ、別の workspace で、wasm-bindgen の流儀（trunk なし）でビルドする

- 状態: Accepted（2026-11-06。ユーザーの判断。Proposed は 2026-11-05、S153。そのときはブラウザ（Linux）の 1 列だけ測った。3 OS までは [ADR-0034](0034-ui-bench-method.md) の付記）
- 関係: [ADR-0023](0023-desktop-and-web-process-layout.md)（本線の UI は計算を別のプロセスか Worker に置く。ここでは**試験版だけの例外**にし、覆さない）、[ADR-0034](0034-ui-bench-method.md)（測り方）、[ADR-0014](0014-session-route-wasm.md)（wasm の道）

## 背景

- [roadmap](../roadmap.md) の P7 の完了条件に「`doc/bench/ui.md` に 3 UI × 3 OS の比較を書く」がある。[architecture](../architecture.md) は「Leptos 版と Dioxus 版は `src/Rust/apps/experimental/` に、機能を絞って作る（エディタ、検査パネル、グラフ）」と書いている。S125 で Svelte 版の測定ハーネスはできた（ADR-0034）。
- Leptos は Rust だけで UI を書くフレームワークなので、Svelte 版と違い、UI から計算を **JS 経由でなく直接**呼べる。この違いが負荷（起動時間、メモリ、バンドル）に出るかを見るのが比較の目的である。

## 決定

1. **何を作るか**（機能を絞る）: `<textarea>` のエディタ、検査（check）と診断の一覧、実行（simulate）と結果の表（最後の値）、MSL を読む（`msl/MSL.json` の fetch と ChuaCircuit の検査）、ダークモード（`prefers-color-scheme` に追従 + 手動の「テーマ」ボタン）。`?smoke=1` で Svelte 版と同じ `GM-SMOKE check=… simulate=… x1=… msl=… ready_ms=… decay_ms=… msl_ms=…` を console に出す。
2. **何を作らないか**: プロット、graph、ライブラリの木、タブ、定義ジャンプ・hover、整形、FMU export、実行の設定の UI、Worker、CodeMirror。テーマの保存（再読み込みで auto に戻る）。Tauri への組み込み。Windows・macOS の測定。Dioxus 版（次の WP）。
3. **facade `globe-mode` を直接依存にする**: `globe-mode-wasm`（Worker 用の束ね）を JS 経由で呼ばず、Leptos の wasm に `globe-mode` をリンクして同じ wasm の中で呼ぶ。理由は、(a) 「UI と計算が 1 つの wasm」が Leptos 版の特徴で、比べたい点そのものだから、(b) JSON の往復と Worker の `postMessage` が無く、Rust の型（`CheckResponse`・`SimulateResponse`）のまま使えるから。代わりに**重い検査・実行の間はメインスレッドが止まる**（ADR-0023 の (a) の逆。試作として許容し、本線には持ち込まない）。時計は JS の `performance.now()`（facade の `now_ms` は wasm32 で 0）。
4. **置き場**: `src/Rust/apps/experimental/leptos/`、ルートの workspace には入れず、自分の `[workspace]` と `Cargo.lock` と `deny.toml` を持つ（`src/Rust/apps/desktop` と同じ。Leptos とその依存を差分 CI の `cargo build --workspace` に巻き込まないため）。ルートの `[patch.crates-io] diffsol`（ADR-0013）は workspace のルートでしか効かないので、同じものをこちらにも書く。
5. **ビルドの道**: `trunk` は使わない。`mise run leptos:build` が、`wasm:build` と同じく `cargo build --release --target wasm32-unknown-unknown`（lto・opt-level=s は環境変数で与える）→ `wasm-bindgen --target web` で `dist/pkg/` に糊を作り、`index.html`・`main.js`・`style.css` を並べる。ビルドの道具は増えない（wasm-bindgen の CLI は mise の固定版 0.2.129、crate は `=0.2.129` で揃える）。target はルートのものと混ぜず `${CARGO_TARGET_DIR:-target}/leptos`。
6. **依存**: Leptos 0.8.23（MIT、`csr` feature だけ。0.9.0-rc1 は採らない）。[licenses.md](../licenses.md) に足し、`cargo deny` を desktop と同じ流儀で通す。
7. **測り方**: `bench_ui.py browser --ui leptos` が Leptos 版の `dist/` を配って同じ手順で測り、`leptos/linux` の列に入れる。試験は `leptos/tests/test_leptos.py`（`test_ui.serve()` の再利用）。**CI には入れない**（入れるかは次の WP）。

## 結果

- 良い: ビルドの道具が増えない。`GM-SMOKE` の行だけ守れば比較の表に載る。
- 良い: UI から計算までが Rust の型で通り、JSON の往復が無い。
- 悪い: メインスレッドが止まる（ブラウザの「ページが応答しない」になりうる）。中断（止める）も効かない。本線の UI にはならない。
- 悪い: 別の workspace なので、依存を上げるときにルートと二重に見る（Rumoca の rev はルートの `[workspace.dependencies]` を path 依存の側が引くので 1 か所）。
- 数字は [bench/ui.md](../bench/ui.md) と [S153 の解説書](../sessions/S153-ui-leptos/解説書.md)。

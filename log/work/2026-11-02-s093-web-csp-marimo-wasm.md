# S093 `web-csp-marimo-wasm`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s093-web-csp-marimo-wasm` |
| worktree | `.git-worktree/s093-web-csp-marimo-wasm` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S093-web-csp-marimo-wasm/` |
| セッション記録 | （親が写す） |

## 依頼

P6 の完了条件「ブラウザが CSP `connect-src 'self'` のまま実行できる（サーバーで計算しない）」を `src/Svelte` の Playwright の試験で確かめる。「marimo の WASM で 2 GB 未満」は調査だけ（`doc/research/marimo-wasm-2026-11-02.md`）で、`cargo check -p globe-mode-py --target wasm32-unknown-emscripten` を試す。

## 決めたこと

- CSP は S041 から `index.html` の meta に依頼と同じ形であったので、変えない。
- meta の CSP は Web Worker に効かないので、`test_ui.py` の http.server が meta と同じ CSP を応答の header にも付ける（wasm の初期化と MSL の取得は worker の中）。
- 「サーバーで計算しない」は、context の要求が配った origin・`data:`・`blob:` だけであることで確かめる。
- marimo の WASM は、先に anywidget の `_esm` から `globe-mode-wasm` を呼ぶ道を試すのがよいと判断した（emscripten の wheel は emsdk と Pyodide の版の固定を抱える）。

## やったこと

- `src/Svelte/tests/test_ui.py`: CSP の header、origin の外への要求の検査、access log を黙らせる（`partial` に `log_message` を付けても効いていなかったのを class に移した）。
- `doc/research/marimo-wasm-2026-11-02.md`: 新規。
- `doc/roadmap.md`: P6 の表と S093 の節。

## 検証

- コードの検査（fmt / clippy / test / deny）: Rust は変えていないので回していない。
- `mise run ui:check`: 成功（514 s。console の error 0、origin の外への要求 0、ChuaCircuit の検査 3.4 s・実行 1.9 s）。log_message の修正の後に `test_ui.py` だけ回し直して成功。
- `cargo check -p globe-mode-py --target wasm32-unknown-emscripten`: emsdk 無しは `psm` の `emar` で失敗、clang・llvm-ar-20 で成功（223 crate、3 分 33 秒）。
- `check_docs.py`: デッキ OK（S093 を含む）/ 文書 378 件・相対リンク 1823 件中の切れ 0 件。
- `render_slides.py`: 1 回試して時間切れ（`Page.screenshot` の途中で 170 s）。CI に任せる。

## 残したこと

- 本番の配信先での CSP の header（配信先が未定）。
- marimo の WASM での実測（2 GB の条件は未達のまま）。
- main の `data/msl/msl`（15:33 に別のセッションが作った、行き先の無い symlink）が `pack-msl.mjs` を落とす。この worktree の `data/msl` は `msl` を除いた symlink の集まりにして避けた。main 側は触っていない。

# S102 `marimo-wasm-anywidget`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s102-marimo-wasm-anywidget` |
| worktree | `.git-worktree/s102-marimo-wasm-anywidget` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S102-marimo-wasm-anywidget/` |
| セッション記録 | 親が写す（S048〜S052 と同じセッション） |

## 依頼

S093 の持ち越しの 1 つ目: marimo の WASM（Pyodide）版で、anywidget の `_esm` から `globe-mode-wasm` を直接呼ぶ道を試作し、`marimo export html-wasm` で動くかとメモリ（2 GB の条件）を測る。研究文書に結果を追記。ついでに README に `pip install "globe-mode[widget]"` の 1 行（S097 の持ち越し）。Rust・desktop・ci.yml・release.yml・android.yml は触らない。新しい Python の依存は足さない。

## 決めたこと

- 試作は新しいノートブック `src/Python/marimo/wasm_app.py` に置いた（`globe_mode` のパッケージは `_native` を import するので Pyodide では読めない）。widget は anywidget と traitlets だけ。
- wasm は `mise run wasm:build` の `wasm-pkg/web`（target-shared の既存の成果物、2026-11-02 17:03 のビルド）を書き出し先の `pkg/` に写し、`_esm` が `new URL("pkg", document.baseURI)` で引く。Rust はビルドしていない。
- 計算はメインのスレッドで同期に回す（`GlobeWorker` は使わない）。最小の試作のため。
- 検査は手元の `wasm_check.py` だけ。12 s ほどで終わり RAM も足りたので、GitHub Actions の workflow は足さなかった。
- 負荷が高いときの marimo 自身の `RPC request timed out.`（Pyodide の起動前）は、その 1 つの文言だけ判定から除いた。起動後の `bridge` は成功する。
- marimo 0.25.1 の `export html-wasm --offline` を見つけたので、`wasm_check.py --offline` で外への要求が 0 件になることも確かめた。CSP の header は付けていない（持ち越し）。
- README の 1 行は、wheel が PyPI に無い（2026-11-02 の判断）ので、Releases の wheel に extra を付ける形（`pip install "./globe_mode-….whl[widget]"`）にし、index に載ったときの `pip install "globe-mode[widget]"` も併記した。

## やったこと

- `src/Python/marimo/wasm_app.py`（新規）: widget `WasmSim` と Decay の文面。
- `src/Python/marimo/wasm_check.py`（新規）: 書き出し・配信・Chromium・x(1) とメモリ。psutil・playwright は既に環境にある（新しい依存なし）。
- `src/Python/marimo/README.md`: 表に 2 行。
- `README.md`: 英語と日本語の Install に wheel と extra `widget` の 1 行ずつ。
- `doc/research/marimo-wasm-2026-11-02.md`: 「(C) の試作の結果」を追記（出典の確認日 2026-11-03）。
- `doc/roadmap.md`: P6 の行と「S102 で済んだこと」。`doc/sessions/README.md`: 索引の行。
- `doc/sessions/S102-marimo-wasm-anywidget/`: slides.html・解説書.md。

## 検証

- `wasm_check.py`: 4 回通過（うち `--offline` 1 回）。x(1) = 0.367879468116207（誤差 2.7e-8）、check passed、Pyodide v314.0.0。
- メモリ: JS heap used 22〜25 MiB / total 25〜37 MiB、wasm の線形メモリ 2 MiB、ブラウザの RSS の合計 857 / 879 / 966 / 951（`--offline`）MiB。2 GiB 未満。
- 開いてから結果まで 11.1 s・11.8 s（load average 29 の時は約 58 s）。外への要求: 既定 3 ホスト、`--offline` 0 件。
- `uv run marimo check src/Python/marimo/wasm_app.py`: 警告 0。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。`uv sync --locked`: 通過（35 パッケージ）
- `check_docs.py`: デッキ 106 件 OK / 文書 424 件・相対リンク 2078 件中の切れ 0 件
- `render_slides.py`: S102 のデッキ 1 件・3 表示（desktop-light・dark・mobile）× 6 枚、問題 0 件（1 回目は Chromium の screenshot が 30 s で時間切れ、負荷が下がってから通過）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- MSL（15 MiB の JSON）を widget に渡す道と、そのメモリ（S036 の記録では線形メモリ 262 MiB）。
- marimo の WASM に合う CSP の header（`--offline` の書き出しで）。
- Web Worker（`GlobeWorker`）で回す道。
- emscripten の wheel（調査の道 (B)）。
- `wasm_check.py` は CI に入れていない（CDN とネットに頼るため。`--offline` でも書き出しの時にネットが要る）。

# marimo の WASM（Pyodide）で globe_mode を動かす道

- 確認日: 2026-11-02（S093）
- 確認できなかった項目には【未確認】と書いた。コードは書いていない（調査だけ）。
- 関係する正典: [roadmap.md](../roadmap.md) の P6「marimo の WASM で 2 GB 未満に収まる」、[licenses.md](../licenses.md)

## 結論

| 道 | 可否 | 要るもの | 評価 |
| --- | --- | --- | --- |
| (A) 今の wheel（abi3、manylinux）を micropip で入れる | **不可** | — | Pyodide は emscripten 向けの wheel しか入れられない |
| (B) `globe-mode-py` を `wasm32-unknown-emscripten` 向けに作り直す | **見込みあり**（`cargo check` は通る。link と実行は【未確認】） | emsdk（Pyodide の版と同じ emscripten）、Pyodide の決めた Rust のツールチェーン、`pyodide build` か maturin、Pyodide の版ごとの wheel | Python の API（`Workspace`・`table`・`SimulationPlot`）がそのまま使える。CI の行列と配布の手間が増える |
| (C) marimo の anywidget の `_esm` から `globe-mode-wasm` の JS を呼ぶ | **見込みあり**（既に動いている wasm をそのまま使う） | wasm（約 15 MiB）と JS の糊を widget から読み込む手段 | 新しいビルドは要らない。ただし計算は Python から見えず、結果は traitlets で返す |

- **先に (C) を試すのがよい**。ブラウザで計算する wasm（`globe-mode-wasm`、S012・S036）は既に CI で回っている。(B) は emsdk と Pyodide の版の固定を新しく抱える。
- 2 GB の条件: ブラウザの wasm の線形メモリは MSL を読んだ後 262 MiB（S012 の記録。[S036 の解説書](../sessions/S036-wasm-web-worker/解説書.md)）。(C) なら Pyodide 本体と別の wasm インスタンスになる。(B) は Pyodide の 1 つの線形メモリに Python と GlobeMode が同居する。どちらも 2 GB 未満の見込みだが、marimo の上で測っていない【未確認】。

## 事実

### Pyodide と wheel

- Pyodide は manylinux の wheel を入れられない。maturin が普通に作る manylinux の wheel を `micropip.install` すると失敗する（<https://github.com/kafonek/py-rust-fib>、2026-11-02 確認）。今の `globe_mode` の wheel（`pyo3/abi3-py312`、S039）はこの形なので使えない。
- Rust の拡張は `wasm32-unknown-emscripten` の target で作る。`pyodide build .` が `CARGO_BUILD_TARGET=wasm32-unknown-emscripten`、linker の `emcc`、`-C link-arg=-sSIDE_MODULE=2` から始まる RUSTFLAGS、`PYO3_CROSS_INCLUDE_DIR`・`PYO3_CROSS_LIB_DIR`・`PYO3_CROSS_PYTHON_VERSION` を設定する。Rust のツールチェーンは `pyodide config get rust_toolchain` の版を自分で入れる。Python 3.14 より前は stable の Rust では target が使えず nightly が要る（<https://pyodide-build.readthedocs.io/en/stable/tutorials/rust.html>、2026-11-02 確認）。
- emscripten の版は Pyodide の platform と合わせる。`pyemscripten_2026_0`（Pyodide 314.x、CPython 3.14）は Emscripten 5.0.3、Pyodide 0.28・0.29 は CPython 3.13 と `pyemscripten_2025_0`。wheel の platform tag は `pyemscripten_{年}_{patch}` で、PEP 783 により PyPI に直接上げられる。maturin では `MATURIN_PYEMSCRIPTEN_PLATFORM_VERSION` で tag を与える。Python の版ごとに別の wheel が要る（<https://pydantic.dev/articles/emscripten-wheels-pydantic>、2026-11-02 確認）。
- abi3 の wheel を Pyodide が複数の版で読めるか【未確認】（上の 2 つの資料は abi3 に触れていない）。版ごとに作る前提で考える。
- PyO3 と maturin の公式の文書は emscripten に触れていない（<https://pyo3.rs/main/building-and-distribution.html>、<https://www.maturin.rs/platform_support.html>、2026-11-02 確認）。

### marimo の WASM

- WASM のノートブックは PyPI の pure Python の wheel と、Pyodide が同梱するパッケージ（NumPy・SciPy・polars・duckdb ほか）を使える。足りないものは自動で入れようとし、`micropip` でも入れられる。メモリの上限は 2 GB（<https://docs.marimo.io/guides/wasm/>、2026-11-02 確認）。
- marimo が使う Pyodide の版は npm の `pyodide` の版から決まる（`frontend/src/core/wasm/worker/getPyodideVersion.ts`。<https://github.com/marimo-team/marimo/blob/main/frontend/src/core/wasm/worker/getPyodideVersion.ts>、2026-11-02 確認）。今の版の番号と Python の版【未確認】。
- anywidget が marimo の WASM で動くこと【未確認】（第三者のパッケージ pywidget が marimo と Pyodide で anywidget を使うと書いている。<https://pypi.org/project/pywidget/>、2026-11-02 確認）。
- `globe_mode` の Python 側の依存は `polars`・`anywidget`・`traitlets`（`pyproject.toml`）。polars は Pyodide にある。anywidget・traitlets は pure Python。

### GlobeMode の依存と emscripten

- cranelift の JIT（`rumoca-exec-cranelift`）は `rumoca-sim` の `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` にだけある。`wasm32-unknown-emscripten` も `target_arch = "wasm32"` なので、py crate を emscripten 向けに作ると cranelift は自動で外れる（`cargo tree -p globe-mode-py --target wasm32-unknown-emscripten -i cranelift-codegen` が空。2026-11-02、S093 で確認）。
- JIT が無い wasm32 では、`globe-mode-exec` はすべてのソルバを session の道（lowering と実行を分け、`Instant::now` を呼ばない道。`src/Rust/mod/globe-mode-exec/src/rumoca.rs`）で回す。`globe-mode-wasm` は facade（`globe-mode`）と protocol だけに依存し、特別な feature を立てていない。`globe-mode-py` も facade だけに依存するので、同じ feature の組で作れる。
- diffsol → faer → rand → getrandom 0.3: wasm32-unknown-unknown では `getrandom_backend="wasm_js"` が要る（`.cargo/config.toml`、S012）。emscripten では getrandom の既定の backend で `cargo check` が通った（実行時の乱数は【未確認】）。
- C のコードを含む依存は `psm`（`pest` → `nalgebra-sparse` → `diffsol` の経路。stacker のアセンブリ）だけが build script で引っかかった。emsdk の無い手元では `emar` が見つからず失敗し、`CC_wasm32_unknown_emscripten=clang`・`AR_wasm32_unknown_emscripten=llvm-ar-20` を与えると通った。本番は emsdk の `emcc`・`emar` を使う。
- SUNDIALS（feature `sundials`）は wasm では使わない（既に wasm32-unknown-unknown でも外している）。

### 試した `cargo check`（2026-11-02、S093）

```bash
rustup target add wasm32-unknown-emscripten        # rustc 1.99.1（stable）
export CARGO_TARGET_DIR=…/target-shared
# 1 回目: emsdk が無いので psm の build script が「failed to find tool "emar"」で失敗（96 crate まで進んだ）
PYO3_CROSS_PYTHON_VERSION=3.13 cargo check -p globe-mode-py --target wasm32-unknown-emscripten
# 2 回目: C の道具を clang に差し替えると通る（223 crate、3 分 33 秒）
CC_wasm32_unknown_emscripten=clang AR_wasm32_unknown_emscripten=llvm-ar-20 \
CFLAGS_wasm32_unknown_emscripten=--target=wasm32-unknown-emscripten \
PYO3_CROSS_PYTHON_VERSION=3.13 cargo check -p globe-mode-py --target wasm32-unknown-emscripten
```

- `check` は型と借用までで、link（`emcc -sSIDE_MODULE=2`）と Pyodide での import は確かめていない【未確認】。stable の Rust で `check` は通ったが、Pyodide の決めた nightly で通るかは【未確認】。
- abi3 の feature（`pyo3/abi3-py312`）は付けずに試した（`pyproject.toml` の maturin の設定で付くもので、Cargo の既定では付かない）。

## (C) の道筋（次に試すなら）

1. `globe-mode-wasm` の `--target web` の糊（`globe_mode_wasm.js`）と `.wasm` を Python パッケージの資源として同梱するか、marimo のノートブックと同じ場所に置いて `fetch` する。
2. anywidget の `_esm` で `import()` して `GlobeWorker`（`web/client.js`）か `globe_mode_wasm.js` を直接使い、モデルの本文と設定を traitlets で受け、結果の列を traitlets で返す。
3. marimo の WASM の書き出し（`marimo export html-wasm`）で開き、ブラウザの開発者ツールでメモリを測る。

- 気をつけること: anywidget の `_esm` の中からの相対の `import()` や wasm の `fetch` がどの URL を基準にするか【未確認】。MSL（15 MiB の JSON）も同じ道で渡す必要がある。

## (B) の道筋

1. Pyodide の版（marimo が使う版）を決め、`pyodide config get rust_toolchain`・`emscripten_version` を読む。
2. CI に emsdk と nightly の Rust を入れ、`pyodide build` で `src/Python/mod` から wheel を作る。
3. Pyodide の venv か Node の Pyodide で `import globe_mode` → Decay の check → simulate を回す。

- 気をつけること: 版の固定が mise の rust（stable）と別に要る。Python の版ごとに wheel を作る。threading は使えない（diffsol・faer が rayon を使うなら単一スレッドで動くか【未確認】。`globe-mode-wasm` は wasm32-unknown-unknown で動いているので、少なくとも同じ cfg の道は通る見込み）。

## (C) の試作の結果（2026-11-03、S102）

- 確認日: 2026-11-03。試作は `src/Python/marimo/wasm_app.py`（ノートブック）と `src/Python/marimo/wasm_check.py`（書き出し・配信・Chromium）。手順は [S102 の解説書](../sessions/S102-marimo-wasm-anywidget/解説書.md)。
- **動いた**。`marimo export html-wasm --mode run`（marimo 0.25.1）で書き出したページを localhost で配り、Playwright の Chromium で開くと、anywidget の `_esm` が `globe-mode-wasm` の `--target web` の糊を `import()` し、文面に書いた Decay を check（passed）→ simulate（bdf）して x(1) = 0.367879468116207（e⁻¹ との差 2.7e-8）を出した。結果の traitlet `result` は `save_changes` で Pyodide 側へ戻る（marimo の RPC の `bridge` が成功で返る）。
- 書き出しが読む Pyodide は v314.0.0（`index.html` などの `cdn.jsdelivr.net/pyodide/v314.0.0`。上の「今の版の番号」の【未確認】はこれで埋まる。v314 は CPython 3.14）。marimo の WASM で anywidget が動くこと（上の【未確認】）も確かめた。anywidget は Pyodide が起動時に読み込む（console に `Loading … anywidget …`）。
- `_esm` は blob の URL で読まれる前提で、置き場は `new URL("pkg", document.baseURI)`（ページを基準）にした。`OUT/pkg/` に糊（`globe_mode_wasm.js`）と `globe_mode_wasm_bg.wasm`（15 MiB）を置けば、相対の `import()` と `fetch` の基準の問題（上の「気をつけること」）は起きない。

### メモリ（Decay だけ、MSL なし。手元の WSL、Chromium 1248 headless、`--enable-precise-memory-info`）

| 量 | 値 | 測り方 |
| --- | --- | --- |
| ページの JS heap | used 22〜25 MiB / total 25〜37 MiB | `performance.memory`（メインのページだけ。Pyodide の Worker は含まない） |
| `globe-mode-wasm` の線形メモリ | 2 MiB | `memory_bytes()`（Workspace を 1 つ作り、Decay を回した後） |
| ブラウザの RSS の合計 | 857〜966 MiB（4 回、`--offline` の 1 回を含む） | `psutil` でこの Python の子孫（Playwright の driver と Chromium の全プロセス）の RSS を足す。共有のページを重ねて数えるので上限の見積り |
| 開いてから結果まで | 11.1〜11.8 s（負荷の低い時）、約 58 s（load average 29 の時） | `page.goto` から `window.__globe` まで |

- 2 GB の条件: Decay では RSS の合計でも 1 GiB 未満で、**2 GB 未満に収まる**。MSL を読むと S036 の記録で線形メモリが 262 MiB 増える見込みで、それでも 2 GB 未満の見込みだが、MSL（15 MiB の JSON）を widget に渡す道は試していない【未確認】。
- 手元の負荷が高いと、marimo の RPC が Pyodide の起動（約 45 s）より先に `RPC request timed out.` を投げる（ページの未捕捉の例外）。起動後の `bridge` は通り、結果も出るので、`wasm_check.py` はこの 1 つの文言だけを除いて判定する。

### 試作で分かった制限

- 既定の書き出しのページは Pyodide（`cdn.jsdelivr.net`）、marimo の資源（`wasm.marimo.app`）、PyPI（`files.pythonhosted.org`）を外から取る。`--offline` を付けると Pyodide とパッケージを書き出し先に取り（Playwright の Chromium で依存を解く。書き出し先は 42 → 52 MiB）、**配った origin の外への要求は 0 件**になった（`wasm_check.py --offline`、RSS の合計 951 MiB、11.8 s）。それでも `wasm_check.py` は CSP の header を付けていない。marimo の WASM に合う CSP（S093 の `connect-src 'self'` で足りるか、`'unsafe-eval'` が要るか）は持ち越し。
- 計算はメインのスレッドで同期に走る（Web Worker の `GlobeWorker` は使っていない）。長いモデルでは UI が止まる。
- `globe_mode` の Python の API（`Workspace`・`table`）は Pyodide から使えない（(B) の道が要る）。

### 出典（2026-11-03 確認）

- marimo の WASM の書き出し（`marimo export html-wasm`）: <https://docs.marimo.io/guides/exporting/>。`--offline`（Pyodide とパッケージを書き出し先に取る。Playwright の Chromium が要る）と HTTP サーバでの配信: <https://docs.marimo.io/guides/publishing/self_host_wasm/>。`--mode [edit|run]` は marimo 0.25.1 の `marimo export html-wasm --help`
- marimo の anywidget（`mo.ui.anywidget`）: <https://docs.marimo.io/api/inputs/anywidget/>
- anywidget の `_esm` と `model.get`・`model.set`・`save_changes`: <https://anywidget.dev/en/afm/>
- `performance.memory`（Chromium の非標準。`--enable-precise-memory-info` で丸めない）: <https://developer.mozilla.org/en-US/docs/Web/API/Performance/memory>

## MSL・Web Worker・CSP（2026-11-03、S104）

- 確認日: 2026-11-03。手順は [S104 の解説書](../sessions/S104-marimo-wasm-msl-worker/解説書.md)。上の「試作で分かった制限」のうち、MSL・Web Worker・CSP の 3 つを埋めた。
- **MSL**: `_esm` が `fetch("msl/MSL.json")`（`src/Svelte` の `pack-msl.mjs` の出力、15.2 MiB）で文面を取り、`GlobeWorker.library(json, "MSL")` に渡す（ブラウザ版の `App.svelte` と同じ道。Pyodide を通さない）。`Modelica.Electrical.Analog.Examples.ChuaCircuit` は check passed、simulate（bdf）ok、C1.v(T) = -0.9928894683235796。MSL の読み込み 0.7 s、simulate 0.5〜0.6 s（2 回。手元の WSL、load average 10 未満の時）。
- **Web Worker**: `globe-mode-wasm` の `web/client.js`（`GlobeWorker`）と `worker.js`（S036、ブラウザ版の `WorkerBackend` と同じもの）を書き出し先の `wasm/` に写し、`_esm` から `import()` して `new Worker(…, { type: "module" })` で起こす。`_esm` は data: の URL で読まれる（下の CSP の違反の文言で分かった。S102 で「blob の URL」と書いたのは誤り）が、置き場は `document.baseURI` を基準にしているので影響しない。
- **メインのスレッドが固まらないこと**: simulate の間、ページで 20 ms ごとの `setInterval` を回し、間隔の最大を測る。ChuaCircuit の simulate（0.5〜0.6 s）の間に 26〜29 回走り、最大の間隔 20 ms（同期で回せば simulate の長さだけ止まる）。

### メモリ（MSL と ChuaCircuit を含む。手元の WSL、Chromium headless、`--offline`）

| 量 | 値 |
| --- | --- |
| ページの JS heap | used 36 MiB / total 40 MiB |
| `globe-mode-wasm` の線形メモリ | Decay の Worker 2 MiB、ChuaCircuit の Worker 112 MiB（MSL を読み、ChuaCircuit を回した後） |
| ブラウザの RSS の合計 | 1152〜1154 MiB（2 回。Pyodide・2 つの Worker を含む。共有のページを重ねて数える上限の見積り） |
| 開いてから結果まで | 14.9〜17.0 s |

- 2 GB の条件: MSL を読んで ChuaCircuit を回しても RSS の合計 1152〜1154 MiB で、**2 GB 未満に収まる**。

### `--offline` の書き出しに合う最小の CSP

`default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' data:; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:`

`wasm_check.py --offline` がこの header をすべての応答に付け、console の CSP の違反 0 件・origin の外への要求 0 件で Decay と ChuaCircuit が通る。1 つずつ外した結果（Chromium headless）:

| 外したもの | 結果 | 何が止まるか |
| --- | --- | --- |
| `script-src` の `'unsafe-inline'` | 動かない（違反 4 件） | marimo の書き出しの `index.html` のインラインの script |
| `script-src` の `data:` | 動かない（違反 2 件） | anywidget が `_esm` を `data:text/javascript;base64,…` の URL で `import()` する |
| `script-src` の `'wasm-unsafe-eval'` | 動かない（違反 1 件） | Pyodide の Worker（`assets/worker-*.js`）の WebAssembly の compile |
| `style-src` の `'unsafe-inline'` | 動くが違反 2 件 | marimo のインラインの style |
| `img-src` の `data:` | 動くが違反 6 件 | data: の favicon など |
| `font-src` の `data:` | 動くが違反 1 件 | data: の woff2 |
| `script-src` の `blob:`・`worker-src` | 要らない（違反 0 件） | Worker は同じ origin の URL から起こすので、`worker-src` が無ければ `script-src 'self'` に落ちる |

- `'unsafe-eval'` は要らない（`'wasm-unsafe-eval'` だけで WebAssembly が通る）。S093 のブラウザ版（`src/Svelte/index.html`）の CSP と比べると、marimo の書き出しの都合で `script-src` に `'unsafe-inline'` と `data:` が増える。`connect-src` は `default-src 'self'` に落ち、外への接続は無い。
- `wasm_check.py` は CSP の違反を console の文言（`Content Security Policy` を含む）で数える。Worker の中の違反（`'wasm-unsafe-eval'` を外した時）もページの console に来た。

### 出典（2026-11-03 確認）

- `script-src` と `'wasm-unsafe-eval'`（`'unsafe-eval'` より狭く、WebAssembly の compile と instantiate だけを許す）: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src>
- `worker-src` が無いときは `child-src` → `script-src` → `default-src` の順に落ちる: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/worker-src>

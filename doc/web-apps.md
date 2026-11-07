# 整形・lint のウェブアプリ群（共通設計）

S053 で決めた。GlobeMode の整形器と lint（`globe-mode-fmt`、[format-style.md](format-style.md)）を、IDE の外からも使えるように、
いろいろな言語・フレームワークで**同じ機能のウェブアプリ**を作る。どのアプリも同じ契約に従う。

## 1. エンジン（計算は Rust 側だけ）

| 呼び方 | 使う側 |
| --- | --- |
| crate `globe-mode-fmt`（`src/Rust/mod/globe-mode-fmt`、依存 0） | Rust のアプリ（`path` 依存） |
| CLI `globe fmt --json -` / `globe lint --json -`（標準入力 → 標準出力の JSON） | Rust 以外のアプリ（子プロセス） |

CLI の契約（`globe-mode-cli`）:

```text
echo '<Modelica>' | globe fmt --json - [--indent 2] [--width 80] [--no-insert-origin]
→ {"ok": true, "text": "<整形後>", "error": null}            終了コード 0
→ {"ok": false, "text": null, "error": "<理由> (byte N)"}     終了コード 1（文法が壊れている。入力は触らない）

echo '<Modelica>' | globe lint --json - [--disable <rule>]...
→ {"messages": [{"file": "<stdin>", "rule": "line-too-long", "line": 5, "column": 81, "message": "…"}]}
   終了コード 0（0 件）/ 1（1 件以上）
```

- 標準入力は `-` を明示する（`globe fmt --json -`）。`--json` だけでパスも `-` も無ければ、使い方を出して終了コード 2（S209。S199 までは `--json` のパスなしだけ標準入力だった）。
- バイナリの場所は環境変数 `GLOBE_BIN`。無ければ PATH の `globe`。手元では `cargo build -p globe-mode-cli` の `target/debug/globe`。
- 本文は UTF-8。行番号・桁は 1 始まり、桁は文字単位。
- CLI を呼ぶ 6 つのアプリ（FastAPI・Astro・Phoenix・Gleam・Streamlit・marimo）は lint に `--disable connect-type-mismatch` を常に付ける（#73）。CLI の lint は既定で connect の型検査（Workspace の構文解析）も回し、1 MiB の入力で 11.8 s・RSS 2.6 GB になる（付けると 0.6 s・229 MB）。Rust の 4 アプリは `fmt::lint` だけを呼ぶので、もともと出さない。

## 2. 画面（全アプリ共通。日本語、外部 CDN なし）

1 ページだけ。

| 要素 | 中身 |
| --- | --- |
| 入力 | `textarea`（名前 `source`）と、`.mo` ファイルを選ぶ `input type=file`（選んだら textarea に読み込む） |
| 設定 | インデント（2 か 4、既定 2）、`origin = {0, 0}` を足す（チェック、既定オン） |
| ボタン | 「整形」「lint」 |
| 出力 | 整形後の本文（読み取り専用の textarea。「入力へ戻す」ボタンで入力側に書き戻せる）、lint の表（rule / line / column / message）、エラー（赤字） |

例題は `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`（整形後は同じ場所の `objects_after.mo`）。

## 3. HTTP API（サーバのあるアプリ共通）

| 要求 | 本文（JSON） | 応答（JSON） |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（text 以外は省略可） | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。HTTP は常に 200 |
| `POST /api/lint` | `{"text": "...", "disable": ["line-too-long"]}`（disable は省略可） | `{"messages": [{"rule": "...", "line": 1, "column": 1, "message": "..."}]}` |
| `GET /` | | 上の画面（HTML） |
| `GET /healthz` | | `{"ok": true}` |

Streamlit・marimo のようにサーバの API を持たないものは、画面だけを同じ機能で作る。

実装で決めた細部（S053）: `indent` は 0〜16 の外なら `ok: false`（HTTP 200）。本文が JSON でない・`text` が無いときはフレームワークの 4xx（400 / 422）。lint の応答は CLI の `file` 欄を落とす。認証・CORS は無い。

### 3.1 入力の守り（S215。CLI を子プロセスで呼ぶ 6 種）

信用できない本文がネットワークから来るので、次の 3 つを守る。

| 項目 | 決めた値 | 超えたとき |
| --- | --- | --- |
| 本文の上限 | 1 MiB（1048576 バイト） | HTTP 413。画面だけのアプリ（Streamlit・marimo）は画面にエラーを出して `globe` を呼ばない |
| `globe` の時間切れ | 10 秒（環境変数 `GLOBE_TIMEOUT_MS` で変える。試験が使う） | 子を kill して 5xx（FastAPI は 504、ほかは「`globe` を実行できない」と同じ扱い）。画面だけのアプリはエラー |
| 一時ファイル | 使うなら排他（`O_EXCL`）で作り、`finally` 相当で必ず消す。使わないのが望ましい | Phoenix と Gleam だけが使う（標準入力を閉じられないため） |

`indent` は 0〜16、`disable` の規則名は `^[a-z][a-z-]*$`（外れたら 4xx。Gleam の整形だけは §3 のとおり HTTP 200 の `ok: false`）。

## 4. 置き場と約束

| 言語 | フレームワーク | 場所 | エンジン |
| --- | --- | --- | --- |
| Rust | axum | `src/Rust/apps/web-axum/` | crate |
| Rust | actix-web | `src/Rust/apps/web-actix/` | crate |
| Rust | Rocket | `src/Rust/apps/web-rocket/` | crate |
| Rust | loco | `src/Rust/apps/web-loco/`（`default-features = false`、DB なし。推移的依存に MPL-2.0 などがあり、ルートの `deny.toml` では通らない。[licenses.md](licenses.md)） | crate |
| Elixir | Phoenix | `src/Elixir/phoenix_fmt/` | CLI |
| Gleam | wisp + mist | `src/Gleam/wisp_fmt/` | CLI |
| Python | Streamlit | `src/Python/streamlit/`（独自の `pyproject.toml`。ルートの依存に streamlit は無い） | CLI |
| Python | marimo | `src/Python/marimo/`（ルートの依存の marimo を使う） | CLI |
| Python | FastAPI（検査は httpx） | `src/Python/fastapi_fmt/` | CLI |
| TypeScript | Astro（API ルート） | `src/Astro/fmt/` | CLI |

すべて S053 で作った。各アプリの検査は README のとおりで、`GLOBE_BIN` に `target/debug/globe` を渡す。画面はブラウザで操作していない（HTML の配信と API を curl と各フレームワークのテストクライアントで確かめた）。

提案（未着手）: Go `net/http`（標準ライブラリだけ）、Deno + Hono、Julia Oxygen.jl、Zig zap、Svelte は `src/Svelte` の IDE 本体に整形ボタンを足す（S053）。

- **自分のディレクトリの外を変えない。** 共有ファイル（`Cargo.toml`、`pyproject.toml`、`uv.lock`、`mise.toml`、`doc/licenses.md`）は触らない。ライセンスの行は各アプリの README に書き、あとでまとめて `doc/licenses.md` に写す。
- Rust のアプリは**独立した workspace**（`Cargo.toml` に空の `[workspace]`）にし、ルートの workspace には入れない（CI と cargo deny の対象を増やさない）。`.cargo/config.toml` で `target-dir = "../../../../target"` にしてビルド物を共有する。`globe-mode-fmt` は `path = "../../mod/globe-mode-fmt"`。
- 依存は **MIT / Apache-2.0 / BSD** だけ（[licenses.md](licenses.md)）。ロックファイルはコミットする。
- 各アプリに `README.md`（起動手順、検査の手順、依存の表: 名前 / 版 / ライセンス / 出典 URL / 確認日）と、API を 1 往復する検査を 1 本置く。
- 画面は外部 CDN を使わない。JS は素の `fetch` でよい。
- メモリが少ない（RAM 3 GB）ので、Rust のビルドは `CARGO_BUILD_JOBS=2`、重いものは順番に回す。

## 5. 起動コマンドの一覧

S060 で足した。先に `globe` を作り、Rust 以外のアプリに場所を渡す（Rust の 4 つは crate を直接使うので要らない）。

```bash
cargo build -p globe-mode-cli                 # リポジトリのルートで
export GLOBE_BIN=$PWD/target/debug/globe      # 省くと PATH の globe
```

どれも独立したプロジェクトなので、そのディレクトリに入ってから起動する。止めるのは Ctrl+C（Phoenix は 2 回）。

| アプリ | 場所 | 初回だけ | 起動 | 開く URL |
| --- | --- | --- | --- | --- |
| axum | `src/Rust/apps/web-axum` | | `PORT=8080 cargo run` | <http://127.0.0.1:8080> |
| actix-web | `src/Rust/apps/web-actix` | | `PORT=8080 cargo run` | <http://127.0.0.1:8080> |
| Rocket | `src/Rust/apps/web-rocket` | | `cargo run`（ポートは `ROCKET_PORT`） | <http://127.0.0.1:8000> |
| loco | `src/Rust/apps/web-loco` | | `CARGO_BUILD_JOBS=2 cargo run`（ポートは `config/development.yaml`） | <http://127.0.0.1:8080> |
| Phoenix | `src/Elixir/phoenix_fmt` | `mise exec -- mix deps.get` | `mise exec -- mix phx.server`（ポートは `PORT`） | <http://localhost:4000> |
| Gleam wisp | `src/Gleam/wisp_fmt` | （`gleam run` が依存を取る） | `PORT=8080 mise exec -- gleam run` | <http://localhost:8080> |
| Streamlit | `src/Python/streamlit` | `uv sync` | `uv run streamlit run fmt_app.py` | 起動時に表示される URL |
| marimo | リポジトリのルート | | `uv run marimo run src/Python/marimo/fmt_app.py` | 起動時に表示される URL |
| FastAPI | `src/Python/fastapi_fmt` | `uv sync` | `uv run uvicorn app:app --port 8080` | <http://127.0.0.1:8080> |
| Astro | `src/Astro/fmt` | `mise exec -- pnpm install` | `mise exec -- pnpm dev` | <http://localhost:4321> |

- 初回の欄が空のものは、起動のときに依存を取る（cargo、uv run）。Phoenix は `mix deps.get` を飛ばすと「the dependency is not available」で止まる。
- 8080 を使うものは 1 つずつ起動するか、`PORT` を変える。
- サーバのある 8 つは、画面を使わずに確かめられる（ポートは上の表）。

```bash
curl -s localhost:8080/healthz
curl -s -H 'content-type: application/json' \
  -d '{"text":"model A Real x(start=1); end A;"}' localhost:8080/api/format
```

# wisp_fmt（Gleam + wisp + mist）

English summary: a small web app written in Gleam (wisp + mist) that formats and lints Modelica code
by calling the GlobeMode CLI `globe` as a child process (`globe fmt --json -` / `globe lint --json -`).
It serves one Japanese page at `/` and the JSON API `POST /api/format`, `POST /api/lint`, `GET /healthz`
(see `doc/web-apps.md`). The computation lives in Rust; this app only moves text and JSON.
Requires Linux or macOS (the FFI runs the child through `/bin/sh`).

GlobeMode の整形器と lint を Gleam（wisp + mist）から使うウェブアプリ。計算は Rust 側の CLI `globe` に任せ、
このアプリは文字列と JSON を運ぶだけ。共通の契約は [doc/web-apps.md](../../../doc/web-apps.md)、
整形の規則は Rust の crate `globe-mode-fmt`（`src/Rust/mod/globe-mode-fmt`）が正。

## 起動

前提: mise（[doc/mise-setup.html](../../../doc/mise-setup.html)）で gleam 1.19.2 と erlang 27 が入っていること。
`globe` を先に作る（worktree のルートで）。

```sh
cd <worktree のルート>
CARGO_BUILD_JOBS=2 cargo build -p globe-mode-cli    # target/debug/globe ができる
cd src/Gleam/wisp_fmt
GLOBE_BIN=<worktree のルート>/target/debug/globe PORT=8080 mise exec -- gleam run
```

- `GLOBE_BIN`: `globe` のパス。無ければ PATH の `globe`。
- `PORT`: 待ち受けるポート。無ければ 8080。
- 画面: http://localhost:8080/

## 入力の守り

本文は 1 MiB まで（超えたら HTTP 413。Streamlit・marimo は画面にエラー）、`globe` は 10 秒で kill する（環境変数 `GLOBE_TIMEOUT_MS` で変える。試験が使う）。決めた値は [doc/web-apps.md の 3.1](../../../doc/web-apps.md)（S215）。

## 検査

```sh
cd src/Gleam/wisp_fmt
GLOBE_BIN=<worktree のルート>/target/debug/globe mise exec -- gleam test
```

`test/wisp_fmt_test.gleam` は wisp の `wisp/simulate` で 2 つのハンドラ（`/api/format`、`/api/lint`）を
直接呼び、`src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`（整形後は `objects_after.mo`）を使う。
`GLOBE_BIN` が無いときは `../../../target/debug/globe`（worktree のルートから見た既定の場所）を使う。
`gleam test` は worktree のルートに `globe` が無いと失敗する（終了コード 127 のエラーが返る）。

API を 1 往復だけ確かめる例は次のとおり。

```sh
curl -s localhost:8080/healthz
curl -s -H 'content-type: application/json' -d '{"text":"model X end X;"}' localhost:8080/api/format
curl -s -H 'content-type: application/json' -d '{"text":"model X\nend X;\n"}' localhost:8080/api/lint
```

## API（doc/web-apps.md §3 と同じ形）

| 要求 | 本文（JSON） | 応答 |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（text 以外は省略可） | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。HTTP は常に 200 |
| `POST /api/lint` | `{"text": "...", "disable": ["missing-description"]}`（disable は省略可） | `{"messages": [{"rule", "line", "column", "message"}]}`。不正な JSON は 400、globe が異常終了したら 500（`{"error": "..."}`） |
| `GET /` | | 画面（HTML） |
| `GET /healthz` | | `{"ok":true}` |

indent は 1〜16、width は 1〜1000 の範囲。外れたら `ok: false`。

## 構成

| ファイル | 中身 |
| --- | --- |
| `src/wisp_fmt.gleam` | 起動（mist で待ち受ける） |
| `src/wisp_fmt/web.gleam` | ルーティングと API |
| `src/wisp_fmt/engine.gleam` | `globe fmt --json -` / `globe lint --json -` を呼んで結果を読む |
| `src/wisp_fmt/page.gleam` | 画面（HTML。外部 CDN なし） |
| `src/wisp_fmt_ffi.erl` | 子プロセスの起動。入力は一時ファイルに書き、`sh` の `<` で渡す（ポートは入力だけを閉じられないため） |
| `test/wisp_fmt_test.gleam` | gleeunit のテスト 5 本 |

## 依存

直接の依存は `gleam.toml` の `[dependencies]`（wisp・mist・gleam_json・gleam_erlang・gleam_http）と
`[dev_dependencies]`（gleeunit・simplifile）。下の表は `manifest.toml` に入る全 22 本（推移的依存を含む）。
ライセンスは hex.pm の API（`https://hex.pm/api/packages/<name>` の `meta.licenses`）で確認した。
すべて MIT か Apache-2.0 で、[doc/licenses.md](../../../doc/licenses.md) の許可の区分に入る。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| wisp | 2.2.2 | Apache-2.0 | https://hex.pm/packages/wisp | 2026-10-31 |
| mist | 6.0.3 | Apache-2.0 | https://hex.pm/packages/mist | 2026-10-31 |
| gleam_json | 3.1.0 | Apache-2.0 | https://hex.pm/packages/gleam_json | 2026-10-31 |
| gleam_stdlib | 1.0.5 | Apache-2.0 | https://hex.pm/packages/gleam_stdlib | 2026-10-31 |
| gleam_erlang | 1.3.0 | Apache-2.0 | https://hex.pm/packages/gleam_erlang | 2026-10-31 |
| gleam_http | 4.5.0 | Apache-2.0 | https://hex.pm/packages/gleam_http | 2026-10-31 |
| gleeunit | 1.11.0 | Apache-2.0 | https://hex.pm/packages/gleeunit | 2026-10-31 |
| simplifile | 2.7.0 | Apache-2.0 | https://hex.pm/packages/simplifile | 2026-10-31 |
| directories | 1.2.0 | Apache-2.0 | https://hex.pm/packages/directories | 2026-10-31 |
| envoy | 1.2.0 | Apache-2.0 | https://hex.pm/packages/envoy | 2026-10-31 |
| exception | 2.1.1 | Apache-2.0 | https://hex.pm/packages/exception | 2026-10-31 |
| filepath | 1.1.2 | Apache-2.0 | https://hex.pm/packages/filepath | 2026-10-31 |
| gleam_crypto | 1.6.0 | Apache-2.0 | https://hex.pm/packages/gleam_crypto | 2026-10-31 |
| gleam_otp | 1.3.0 | Apache-2.0 | https://hex.pm/packages/gleam_otp | 2026-10-31 |
| glisten | 9.0.1 | Apache-2.0 | https://hex.pm/packages/glisten | 2026-10-31 |
| gramps | 6.0.1 | Apache-2.0 | https://hex.pm/packages/gramps | 2026-10-31 |
| houdini | 1.2.1 | Apache-2.0 | https://hex.pm/packages/houdini | 2026-10-31 |
| hpack_erl | 0.3.0 | MIT | https://hex.pm/packages/hpack_erl | 2026-10-31 |
| logging | 1.5.0 | Apache-2.0 | https://hex.pm/packages/logging | 2026-10-31 |
| marceau | 1.3.0 | Apache-2.0 | https://hex.pm/packages/marceau | 2026-10-31 |
| platform | 1.0.0 | Apache-2.0 | https://hex.pm/packages/platform | 2026-10-31 |
| splitter | 1.3.0 | Apache-2.0 | https://hex.pm/packages/splitter | 2026-10-31 |

ツール（配布物には入らない）: Gleam 1.19.2 と Erlang/OTP 27 は mise が入れる。ライセンスは確認していない【未確認】。

## 注意

- `gleam build` / `gleam test` の出力に、wisp の内部の「Deprecated type」の警告が出る（wisp 2.2.2 の中。このアプリの警告ではない）。
- 整形の結果は `globe` の版に依存する。`globe` を作り直したら再度 `gleam test` を回す。

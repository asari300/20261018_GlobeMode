# phoenix_fmt

GlobeMode の整形（`globe fmt`）と lint（`globe lint`）を、Phoenix のウェブアプリとして使えるようにしたもの。
計算は Rust 側の CLI（`globe`）だけで、このアプリは子プロセスで呼んで JSON を受け渡す。

English summary: A small Phoenix (1.8, Bandit) web app that formats and lints Modelica code by calling the `globe` CLI (`globe fmt --json -` / `globe lint --json -`) as a child process. Routes: `GET /` (Japanese UI), `POST /api/format`, `POST /api/lint`, `GET /healthz`. Set `GLOBE_BIN` to the path of the `globe` binary.

## 起動

前提: mise で Elixir 1.18.3 / OTP 27（リポジトリの `mise.toml`）。`globe` のバイナリは先にビルドする。

```bash
# リポジトリのルートで
cargo build -p globe-mode-cli            # target/debug/globe ができる

cd src/Elixir/phoenix_fmt
mise exec -- mix deps.get
GLOBE_BIN=/絶対パス/target/debug/globe mise exec -- mix phx.server
```

- 既定の待ち受けは http://localhost:4000 （環境変数 `PORT` で変える）。
- `GLOBE_BIN` を省くと PATH の `globe` を使う。

## 入力の守り

本文は 1 MiB まで（超えたら HTTP 413。Streamlit・marimo は画面にエラー）、`globe` は 10 秒で kill する（環境変数 `GLOBE_TIMEOUT_MS` で変える。試験が使う）。決めた値は [doc/web-apps.md の 3.1](../../../doc/web-apps.md)（S215）。

## 検査

```bash
cd src/Elixir/phoenix_fmt
GLOBE_BIN=/絶対パス/target/debug/globe mise exec -- mix test    # 9 件
mise exec -- mix test                                           # GLOBE_BIN が無いと :engine の 4 件は飛ばされ、警告が出る
mise exec -- mix format --check-formatted
mise exec -- mix compile --warnings-as-errors
```

- 試験は `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`（整形後は `objects_after.mo`）を使う。
- 起動後の確認（例）:

```bash
curl -s http://localhost:4000/healthz
curl -s -X POST -H 'content-type: application/json' \
  -d '{"text":"model A Real x end A;"}' http://localhost:4000/api/format
```

## API

| 要求 | 本文（JSON） | 応答 |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（text 以外は省略可） | CLI の JSON（`{"ok", "text", "error"}`）。文法エラーも HTTP 200 で `ok: false` |
| `POST /api/lint` | `{"text": "...", "disable": ["rule"]}`（disable は省略可） | CLI の JSON（`{"messages": [{"file", "rule", "line", "column", "message"}]}`） |
| `GET /` | | 日本語の画面（HTML。外部 CDN なし） |
| `GET /healthz` | | `{"ok": true}` |

- 本文が無い、`indent` / `width` が正の整数でない、`disable` が文字列の配列でないときは 400 `{"error": "..."}`。
- CLI が JSON を返さなかったとき（`GLOBE_BIN` が無いなど）は 500。format は `{"ok": false, "text": null, "error": "..."}`、lint は `{"error": "..."}`。

## 依存

直接の依存は phoenix、bandit、jason、dns_cluster、telemetry_metrics、telemetry_poller。残りは推移的な依存。
版は `mix.lock` が正（コミット済み）。ライセンスは各パッケージの Hex メタデータ（`deps/*/hex_metadata.config`）の `licenses` 欄で、2026-10-31 に取得して確認した。
ライセンス規則は [doc/licenses.md](../../../doc/licenses.md)（MIT / Apache-2.0 / BSD のみ）。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| bandit | 1.14.1 | MIT | https://hex.pm/packages/bandit/1.14.1 | 2026-10-31 |
| dns_cluster | 0.2.0 | MIT | https://hex.pm/packages/dns_cluster/0.2.0 | 2026-10-31 |
| hpax | 1.1.0 | Apache-2.0 | https://hex.pm/packages/hpax/1.1.0 | 2026-10-31 |
| jason | 1.4.5 | Apache-2.0 | https://hex.pm/packages/jason/1.4.5 | 2026-10-31 |
| mime | 2.0.7 | Apache-2.0 | https://hex.pm/packages/mime/2.0.7 | 2026-10-31 |
| phoenix | 1.8.16 | MIT | https://hex.pm/packages/phoenix/1.8.16 | 2026-10-31 |
| phoenix_pubsub | 2.4.1 | MIT | https://hex.pm/packages/phoenix_pubsub/2.4.1 | 2026-10-31 |
| phoenix_template | 1.1.0 | MIT | https://hex.pm/packages/phoenix_template/1.1.0 | 2026-10-31 |
| plug | 1.20.3 | Apache-2.0 | https://hex.pm/packages/plug/1.20.3 | 2026-10-31 |
| plug_crypto | 2.2.1 | Apache-2.0 | https://hex.pm/packages/plug_crypto/2.2.1 | 2026-10-31 |
| telemetry | 1.4.2 | Apache-2.0 | https://hex.pm/packages/telemetry/1.4.2 | 2026-10-31 |
| telemetry_metrics | 1.2.0 | Apache-2.0 | https://hex.pm/packages/telemetry_metrics/1.2.0 | 2026-10-31 |
| telemetry_poller | 1.3.0 | Apache-2.0 | https://hex.pm/packages/telemetry_poller/1.3.0 | 2026-10-31 |
| thousand_island | 1.6.0 | MIT | https://hex.pm/packages/thousand_island/1.6.0 | 2026-10-31 |
| websock | 0.5.3 | MIT | https://hex.pm/packages/websock/0.5.3 | 2026-10-31 |
| websock_adapter | 0.7.0 | MIT | https://hex.pm/packages/websock_adapter/0.7.0 | 2026-10-31 |

- このアプリの生成には `phx_new` 1.8.16（`mix archive.install hex phx_new`）を使ったが、プロジェクトの依存ではない。
- 注意: 整形・lint の本体は `globe`（GlobeMode の Rust 製 CLI）。このアプリのコードは `lib/` と `priv/index.html` だけ。

## 設計メモ

- `PhoenixFmt.Globe` が `GLOBE_BIN` を `sh -c` で起動する。標準入力は一時ファイルを `<` でつないで渡す（`System.cmd` は標準入力を渡せず、Port は入力だけを閉じられないため）。一時ファイルは呼び出しの後に消す。
- POSIX の `sh` が要る（Windows では動かない）。
- 子プロセスに時間制限は付けていない（巨大な入力は待つ）。

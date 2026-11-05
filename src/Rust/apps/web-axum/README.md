# web-axum（Modelica の整形・lint の Web アプリ、Rust + axum）

GlobeMode の整形器と lint（crate `globe-mode-fmt`）を、ブラウザから使えるようにした小さなウェブアプリです。
画面と API の形は [doc/web-apps.md](../../../../doc/web-apps.md) の契約に従います。

English summary: a small axum web app that exposes the Modelica formatter and linter of
`globe-mode-fmt` (a path dependency). It serves one HTML page at `/`, the JSON endpoints
`POST /api/format` and `POST /api/lint`, and `GET /healthz`. It is a standalone Cargo workspace
and is not part of the root workspace.

## 起動

依存を取って、ビルドし、起動します（メモリが少ないので `CARGO_BUILD_JOBS=2`）。

```bash
cd src/Rust/apps/web-axum
CARGO_BUILD_JOBS=2 cargo build
PORT=8080 cargo run
```

ブラウザで <http://127.0.0.1:8080/> を開きます。`PORT` を省くと 8080 です。
外から見えないように、`127.0.0.1` だけで待ち受けます。

## API

| 要求 | 本文（JSON） | 応答 |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（`text` 以外は省略可） | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。整形の失敗も HTTP は 200 |
| `POST /api/lint` | `{"text": "...", "disable": ["line-too-long"]}`（`disable` は省略可） | `{"messages": [{"rule": "...", "line": 1, "column": 1, "message": "..."}]}` |
| `GET /` | | 画面（HTML） |
| `GET /healthz` | | `{"ok": true}` |

- `indent` は 0〜16 の整数だけ受け付けます。範囲外は `ok: false` で返します。
- 本文が JSON でない、または `text` が無いときは、契約の形ではなく axum が 4xx を返します。

## 検査

```bash
cd src/Rust/apps/web-axum
CARGO_BUILD_JOBS=2 cargo test          # tests/api.rs がサーバを立てて、整形と lint を HTTP で 1 往復ずつ確かめる
cargo fmt --check
CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings
cargo deny --config ../../../../deny.toml --manifest-path Cargo.toml check licenses
```

テストの例題は `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`（整形後は `objects_after.mo`）です。

## 依存

直接の依存は次の 4 つです。推移的な依存（計 49 パッケージ）は、`cargo deny check licenses` で
リポジトリの許可リスト（MIT / Apache-2.0 / BSD など）に収まることを確かめています。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| axum | 0.8.9 | MIT | <https://crates.io/crates/axum> | 2026-10-31 |
| tokio | 1.54.1 | MIT | <https://crates.io/crates/tokio> | 2026-10-31 |
| serde | 1.0.229 | MIT OR Apache-2.0 | <https://crates.io/crates/serde> | 2026-10-31 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | <https://crates.io/crates/serde_json> | 2026-10-31 |

- `globe-mode-fmt` 0.2.0（この repo の `src/Rust/mod/globe-mode-fmt`、path 依存、MIT OR Apache-2.0）も使います。
- ライセンスは各 crate の `Cargo.toml` の `license` 欄を `cargo metadata` で読んだ値です。
- tokio は `macros`・`rt-multi-thread`・`net`、serde は `derive` の機能を使います。

## 注意

- 整形と lint は要求を受けた task の上で同期に行います（入力が小さい前提）。
- 認証も CORS も入れていません。外部に公開するときは、前段で制限を入れてください。

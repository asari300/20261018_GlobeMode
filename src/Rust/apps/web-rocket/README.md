# web-rocket（Modelica の整形・lint の Web アプリ、Rust + Rocket 0.5）

GlobeMode の整形器と lint（crate `globe-mode-fmt`）を、ブラウザから使えるようにした小さなウェブアプリです。
画面と API の形は [doc/web-apps.md](../../../../doc/web-apps.md) の契約に従います。

English summary: a small Rocket 0.5 web app that exposes the Modelica formatter and linter of
`globe-mode-fmt` (a path dependency). It serves one HTML page at `/`, the JSON endpoints
`POST /api/format` and `POST /api/lint`, and `GET /healthz`. The port is read from `ROCKET_PORT`
(Rocket's own setting, default 8000). It is a standalone Cargo workspace and is not part of the root workspace.

## 起動

依存を取って、ビルドし、起動します（メモリが少ないので `CARGO_BUILD_JOBS=2`）。

```bash
cd src/Rust/apps/web-rocket
CARGO_BUILD_JOBS=2 cargo build
cargo run
```

ブラウザで <http://127.0.0.1:8000/> を開きます。port は環境変数 `ROCKET_PORT` で変えられます（Rocket の設定で、既定は 8000）。

```bash
ROCKET_PORT=8080 cargo run
```

`address` は既定で `127.0.0.1` です（外から見えないように。`ROCKET_ADDRESS` で上書きできる）。

## API

| 要求 | 本文（JSON） | 応答 |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（`text` 以外は省略可） | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。整形の失敗も HTTP は 200 |
| `POST /api/lint` | `{"text": "...", "disable": ["line-too-long"]}`（`disable` は省略可） | `{"messages": [{"rule": "...", "line": 1, "column": 1, "message": "..."}]}` |
| `GET /` | | 画面（HTML） |
| `GET /healthz` | | `{"ok": true}` |

- `indent` は 0〜16 の整数だけ受け付けます。範囲外は `ok: false` で返します。
- 本文が JSON でない、または `text` が無いときは、契約の形ではなく Rocket の 4xx（JSON の読めない要求）になります。
- JSON の本文は Rocket の既定の上限（1 MiB）までです。

## 検査

```bash
cd src/Rust/apps/web-rocket
CARGO_BUILD_JOBS=2 cargo test          # tests/api.rs が rocket::local::blocking::Client で経路を呼ぶ（サーバは立てない）
cargo fmt --check
CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings
cargo deny --manifest-path Cargo.toml --config ../../../../deny.toml check licenses
```

テストの例題は `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`（整形後は `objects_after.mo`）です。

## 依存

直接の依存は次の 3 つです（`globe-mode-fmt` は path 依存）。推移的な依存は計 150 crate で、
`cargo deny check licenses` でリポジトリの許可リスト（[deny.toml](../../../../deny.toml)）に収まることを確かめています。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| rocket | 0.5.2（features: `json`） | MIT OR Apache-2.0 | <https://crates.io/crates/rocket> | 2026-10-31 |
| serde | 1.0.229（features: `derive`） | MIT OR Apache-2.0 | <https://crates.io/crates/serde> | 2026-10-31 |
| globe-mode-fmt | 0.0.1（この repo の `src/Rust/mod/globe-mode-fmt`、path 依存） | MIT OR Apache-2.0 | <https://github.com/asari300/20261018_GlobeMode> | 2026-10-31 |

- ライセンスは `cargo metadata` の license 欄の値です（2026-10-31）。
- 推移的な依存に、`r-efi`（`MIT OR Apache-2.0 OR LGPL-2.1-or-later`）が 1 つあります。MIT を選べるので許可の範囲です（`getrandom` が引く UEFI 向けの小さな crate で、Linux のビルドには入りません）。
- `unicode-ident` は `(MIT OR Apache-2.0) AND Unicode-3.0`、`encoding_rs` は `(Apache-2.0 OR MIT) AND BSD-3-Clause`。どちらも許可リストに入っています。

## 注意

- 整形と lint は要求を受けた task の上で同期に行います（入力が小さい前提）。
- 認証も CORS も入れていません。外部に公開するときは、前段で制限を入れてください。

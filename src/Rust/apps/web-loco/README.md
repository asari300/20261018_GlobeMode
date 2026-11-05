# web-loco（Modelica の整形・lint の Web アプリ、Rust + loco）

GlobeMode の整形器と lint（crate `globe-mode-fmt`）を、ブラウザと HTTP API から使えるようにした小さなウェブアプリです。
画面と API の形は [doc/web-apps.md](../../../../doc/web-apps.md) の契約に従います。
フレームワークは loco（loco-rs 1.2.0）を、DB・認証・メーラ・ワーカーを切った最小の構成で使います。

English summary: a small web app built on loco (`loco-rs` with default features off, no database)
that exposes the Modelica formatter and linter of `globe-mode-fmt` (a path dependency). It serves one
HTML page at `/`, the JSON endpoints `POST /api/format` and `POST /api/lint`, and `GET /healthz`.
It is a standalone Cargo workspace and is not part of the root workspace.

## 起動

依存を取って、ビルドし、起動します（メモリが少ないので `CARGO_BUILD_JOBS=2`）。

```bash
cd src/Rust/apps/web-loco
CARGO_BUILD_JOBS=2 cargo run
```

ブラウザで <http://127.0.0.1:8080/> を開きます。ポートは `config/development.yaml` の `server.port`（既定 8080）、
待ち受けは `server.binding`（既定 `127.0.0.1`）で決まります。

## API

| 要求 | 本文（JSON） | 応答 |
| --- | --- | --- |
| `POST /api/format` | `{"text": "...", "indent": 2, "width": 80, "insert_origin": true}`（`text` 以外は省略可） | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。整形の失敗も HTTP は 200 |
| `POST /api/lint` | `{"text": "...", "disable": ["class-name-pascal-case"]}`（`disable` は省略可） | `{"messages": [{"rule": "...", "line": 1, "column": 1, "message": "..."}]}` |
| `GET /` | | 画面（HTML。外部 CDN なし） |
| `GET /healthz` | | `{"ok": true}` |

- `indent` は 0〜16 の整数だけ受け付けます。範囲外は `ok: false` で返します。
- 本文が JSON でない、または `text` が無いときは、契約の形ではなく loco（axum）が 4xx を返します。

## 検査

```bash
cd src/Rust/apps/web-loco
CARGO_BUILD_JOBS=2 cargo test    # tests/api.rs が loco_rs::testing の request で HTTP を 1 往復ずつ確かめる（DB は無い）
cargo fmt --check
cargo deny --config ../../../../deny.toml --manifest-path Cargo.toml check licenses
```

テストの内容は次のとおりです。

- `healthz_returns_ok`: `/healthz` が `{"ok": true}` を返す。
- `format_sample_inserts_origin`: 例題 `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo` が `ok: true` になり、`origin = {0, 0}` が入る。
- `format_broken_source_reports_error`: 文法の壊れた入力（`model A` の `end` 無し）が `ok: false` と `error` を返す。
- `lint_reports_and_honours_disable`: 小文字の class 名が `class-name-pascal-case` になり、`disable` で止まる。

`cargo deny` の結果は下の「ライセンス」を見てください（現状は失敗します）。

## 依存

直接の依存は次のとおりです。ライセンスは各 crate の `Cargo.toml` の `license` 欄を `cargo metadata` で読んだ値、
確認日は 2026-10-31 です。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| loco-rs | 1.2.0 | Apache-2.0 | <https://crates.io/crates/loco-rs/1.2.0> | 2026-10-31 |
| axum | 0.8.9 | MIT | <https://crates.io/crates/axum/0.8.9> | 2026-10-31 |
| async-trait | 0.1.92 | MIT OR Apache-2.0 | <https://crates.io/crates/async-trait/0.1.92> | 2026-10-31 |
| tokio | 1.54.1 | MIT | <https://crates.io/crates/tokio/1.54.1> | 2026-10-31 |
| serde | 1.0.229 | MIT OR Apache-2.0 | <https://crates.io/crates/serde/1.0.229> | 2026-10-31 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | <https://crates.io/crates/serde_json/1.0.154> | 2026-10-31 |
| loco-rs（dev-dependencies。機能 `testing`） | 1.2.0 | Apache-2.0 | <https://crates.io/crates/loco-rs/1.2.0> | 2026-10-31 |
| globe-mode-fmt | 0.2.0 | MIT OR Apache-2.0 | この repo の `src/Rust/mod/globe-mode-fmt`（path 依存） | 2026-10-31 |

- loco-rs は `default-features = false` で使います。DB（`with-db`）・認証・キャッシュ・ワーカー・CLI は入れません。
- `testing` 機能は開発時だけ（`[dev-dependencies]`）に入れます。
- 推移的な依存のうち、loco-rs の通常の依存に入る次の 3 つは、リポジトリの `deny.toml` の許可リストに入っていません（`cargo deny check licenses` が失敗する原因）。
  - `colored` 2.2.0 と 3.1.1: MPL-2.0（条件付き。CLAUDE.md の規則では理由を `doc/licenses.md` に書いてから足す）
  - `webpki-roots` 1.0.11: CDLA-Permissive-2.0（lettre 経由。loco-rs の非任意依存）
  - `quoted_printable` 0.5.2: 0BSD（lettre 経由。許可リストに無い）
- 開発時だけ（`testing` 機能、scraper 経由）の `cssparser` 0.36.0、`cssparser-macros` 0.6.1、`selectors` 0.33.0、`dtoa-short` 0.3.5 は MPL-2.0 です。

## 注意

- 整形と lint は要求の中で同期に行います（入力が小さい前提）。
- 認証も CORS も入れていません。外部に公開するときは、前段で制限を入れてください。
- この app は独立した workspace です。ビルド物は `.cargo/config.toml` の `target-dir` でリポジトリ直下の `target/` に置きます。

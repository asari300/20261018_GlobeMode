# GlobeMode 整形・lint（Astro）

GlobeMode の整形器と lint を、ブラウザから使う Astro のアプリです（[doc/web-apps.md](../../../doc/web-apps.md)）。
計算は Rust 側の `globe` CLI（`globe fmt --json` / `globe lint --json`）が行い、このアプリは子プロセスとして呼ぶだけです。

English summary: A small Astro (server output, `@astrojs/node` standalone) web app for the GlobeMode Modelica formatter and linter.
The API routes (`/api/format`, `/api/lint`, `/healthz`) call the `globe` CLI through `GLOBE_BIN`. Run `pnpm dev`, or `pnpm build` and `node dist/server/entry.mjs`.

## 起動

前提: mise で node 24.22.0 と pnpm 12.14.0 が入っていること（リポジトリ直下の `mise.toml`）。`globe` CLI をビルドしておく。

```bash
# リポジトリ直下で（target/debug/globe ができる）
cargo build -p globe-mode-cli

cd src/Astro/fmt
mise exec -- pnpm install --frozen-lockfile
export GLOBE_BIN=/絶対パス/target/debug/globe   # 未指定なら PATH の globe

mise exec -- pnpm dev                           # 開発サーバ（http://localhost:4321）
mise exec -- pnpm build && node dist/server/entry.mjs   # 本番の起動（PORT と HOST で変えられる）
```

ブラウザで `/` を開くと、入力（貼り付けか `.mo` ファイル）に対して「整形」「lint」を押せます。

## API

| 要求 | 応答 |
| --- | --- |
| `POST /api/format` `{"text", "indent"?, "width"?, "insert_origin"?}` | `{"ok", "text", "error"}`（CLI の終了コード 1 も 200） |
| `POST /api/lint` `{"text", "disable"?}` | `{"messages": [{"rule", "line", "column", "message"}]}` |
| `GET /healthz` | `{"ok": true}` |

入力が不正（JSON でない、`text` が無い など）のときは 400 を返します。format は `{"ok": false, "text": null, "error": "..."}`、lint は `{"error": "..."}`。

## 検査

```bash
cd src/Astro/fmt
GLOBE_BIN=/絶対パス/target/debug/globe mise exec -- pnpm test    # node --test。例題 objects_before.mo の整形と lint を確かめる
mise exec -- pnpm build                                          # 本番ビルド
```

## 依存

直接の依存（`package.json`）:

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| astro | 7.4.4 | MIT | https://www.npmjs.com/package/astro （リポジトリ https://github.com/withastro/astro） | 2026-10-31 |
| @astrojs/node | 11.1.9 | MIT | https://www.npmjs.com/package/@astrojs/node （同じリポジトリ） | 2026-10-31 |

ライセンスは `pnpm licenses list`（2026-10-31）で確かめた。推移的依存は計 197 パッケージ:
MIT 166、ISC 11、BSD-2-Clause 8、BSD-3-Clause 3、Apache-2.0 3、CC0-1.0 1。
次の 2 種は許可の一覧（[doc/licenses.md](../../../doc/licenses.md)）に無いので、採否を決めるまで要判断:

- BlueOak-1.0.0（3 件: lru-cache、sax、common-ancestor-path）。lru-cache は Astro の session 保存で実行時に入る。【要判断】
- MPL-2.0（2 件: lightningcss、lightningcss-linux-x64-gnu）。vite がビルド時の CSS 変換に使う。改変せず、`dist/` には入らない。

`sharp`（LGPL-3.0-or-later）は Astro の画像機能の任意の依存で、このアプリは使わないので、`pnpm-workspace.yaml` の `ignoredOptionalDependencies` で入れない。

## 置き場

`src/Astro/fmt/`。`src/lib/globe.ts` が CLI の呼び出し（`spawnSync`、引数は配列で渡し、シェルを通さない）。API は `src/pages/api/` に、画面は `src/pages/index.astro`（外部 CDN なし）に置く。

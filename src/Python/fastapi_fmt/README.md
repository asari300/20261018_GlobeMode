# globe-fmt-fastapi

GlobeMode の Modelica 整形と lint を、FastAPI のウェブアプリとして使う。計算は `globe` CLI（`globe fmt --json -` / `globe lint --json -`）を子プロセスで呼んで行う。

English summary: a small FastAPI web app that formats and lints Modelica by calling the `globe` CLI. Run `uv run uvicorn app:app --port 8080` and open http://localhost:8080.

契約（CLI の JSON、API の形、画面の要素）は [doc/web-apps.md](../../../doc/web-apps.md) に従う。

## 起動

前提: Rust で `cargo build -p globe-mode-cli` を済ませて、`globe` のバイナリがあること（手元では `target/debug/globe`）。

```bash
cd src/Python/fastapi_fmt
GLOBE_BIN=/path/to/target/debug/globe uv run uvicorn app:app --port 8080
```

`GLOBE_BIN` を省くと PATH の `globe` を使う。ブラウザで http://localhost:8080 を開く。

## API

| 要求 | 応答 |
| --- | --- |
| `GET /` | 画面（HTML） |
| `POST /api/format` | `{"ok": true, "text": "...", "error": null}` または `{"ok": false, "text": null, "error": "..."}`。本文は `{"text", "indent", "width", "insert_origin"}`（indent・width・insert_origin は省略可） |
| `POST /api/lint` | `{"messages": [{"rule", "line", "column", "message"}]}`。本文の `disable` で規則を止められる |
| `GET /healthz` | `{"ok": true}` |

`globe` が見つからない・時間切れのときは 503・504 を返す。

## 入力の守り

本文は 1 MiB まで（超えたら HTTP 413。Streamlit・marimo は画面にエラー）、`globe` は 10 秒で kill する（環境変数 `GLOBE_TIMEOUT_MS` で変える。試験が使う）。決めた値は [doc/web-apps.md の 3.1](../../../doc/web-apps.md)（S215）。

## 検査

```bash
cd src/Python/fastapi_fmt
GLOBE_BIN=/path/to/target/debug/globe uv run pytest
```

`tests/test_api.py` が、例題 `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo` で整形（`origin = {0, 0}` が入る）と lint を 1 往復ずつ確かめる。文法エラーと、存在しない `GLOBE_BIN` の場合も確かめる。

## 依存

いずれも `uv.lock` で版を固定している。版と license は、インストールされた配布物のメタデータ（License-Expression、無ければ classifier と LICENSE 文面）で 2026-10-31 に確かめた。出典 URL は PyPI の版ごとのページ。ただし PyPI のページは開いて確かめておらず、【未確認】。

直接の依存（`pyproject.toml`）は次のとおり。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| fastapi | 0.162.0 | MIT | https://pypi.org/project/fastapi/0.162.0/ | 2026-10-31 |
| uvicorn | 0.54.0 | BSD-3-Clause | https://pypi.org/project/uvicorn/0.54.0/ | 2026-10-31 |
| httpx | 0.28.1 | BSD-3-Clause（テストでも使う。METADATA は classifier のみ、LICENSE の文面で確認） | https://pypi.org/project/httpx/0.28.1/ | 2026-10-31 |
| pytest | 9.2.0 | MIT（dev グループ） | https://pypi.org/project/pytest/9.2.0/ | 2026-10-31 |

推移的依存（uv.lock に入る残り。dev のテストを含む）は次のとおり。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| annotated-doc | 0.0.5 | MIT | https://pypi.org/project/annotated-doc/0.0.5/ | 2026-10-31 |
| annotated-types | 0.8.0 | MIT | https://pypi.org/project/annotated-types/0.8.0/ | 2026-10-31 |
| anyio | 4.15.1 | MIT | https://pypi.org/project/anyio/4.15.1/ | 2026-10-31 |
| certifi | 2026.10.23 | MPL-2.0。**MPL-2.0 は条件付き**（httpx の推移的依存。改変せず使う） | https://pypi.org/project/certifi/2026.10.23/ | 2026-10-31 |
| click | 8.5.0 | BSD-3-Clause | https://pypi.org/project/click/8.5.0/ | 2026-10-31 |
| h11 | 0.16.0 | MIT | https://pypi.org/project/h11/0.16.0/ | 2026-10-31 |
| httpcore | 1.0.9 | BSD-3-Clause | https://pypi.org/project/httpcore/1.0.9/ | 2026-10-31 |
| idna | 3.20 | BSD-3-Clause | https://pypi.org/project/idna/3.20/ | 2026-10-31 |
| iniconfig | 2.3.1 | MIT | https://pypi.org/project/iniconfig/2.3.1/ | 2026-10-31 |
| opentelemetry-api | 1.45.2 | Apache-2.0 | https://pypi.org/project/opentelemetry-api/1.45.2/ | 2026-10-31 |
| packaging | 26.3 | Apache-2.0 OR BSD-2-Clause | https://pypi.org/project/packaging/26.3/ | 2026-10-31 |
| pluggy | 1.7.0 | MIT | https://pypi.org/project/pluggy/1.7.0/ | 2026-10-31 |
| pydantic | 2.14.2 | MIT | https://pypi.org/project/pydantic/2.14.2/ | 2026-10-31 |
| pydantic_core | 2.50.2 | MIT | https://pypi.org/project/pydantic_core/2.50.2/ | 2026-10-31 |
| Pygments | 2.21.0 | BSD-2-Clause | https://pypi.org/project/Pygments/2.21.0/ | 2026-10-31 |
| starlette | 1.10.0 | BSD-3-Clause | https://pypi.org/project/starlette/1.10.0/ | 2026-10-31 |
| typing_extensions | 4.16.0 | PSF-2.0。**PSF-2.0 は doc/licenses.md の許可の一覧に無い**（推移的依存） | https://pypi.org/project/typing_extensions/4.16.0/ | 2026-10-31 |
| typing-inspection | 0.4.4 | MIT | https://pypi.org/project/typing-inspection/0.4.4/ | 2026-10-31 |

### 要判断（ライセンス）

- **certifi（MPL-2.0）**: httpx が必ず引く。[doc/licenses.md](../../../doc/licenses.md) は MPL-2.0 を「改変しないで使う場合に限る」条件付きとしている。このアプリは改変せず使うが、表には未登録。
- **typing-extensions（PSF-2.0）**: fastapi・pydantic などが引く。doc/licenses.md の許可の一覧に PSF-2.0 が無い。
- 依存を足したら、同じコミットで doc/licenses.md の表を更新する規則（CLAUDE.md §3）に従い、この 2 件を表に写すかどうかを決める。

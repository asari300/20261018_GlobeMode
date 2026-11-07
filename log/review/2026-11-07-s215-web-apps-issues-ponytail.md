# S215 `web-apps-issues` のレビュー（ponytail）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s215-web-apps-issues` |
| 対象 | issue #22 と #36（`/ponytail-audit` 5.3.2 がリポジトリ全体を読んで立てた。コードを読んだだけで実行していない） |
| レビュアー | ponytail（Claude Code `/ponytail-audit`） |
| 依頼の文面 | （ユーザーが立てた issue。S215 の依頼は、いまの main で 1 件ずつ再現してから直す） |
| 作業記録 | `log/work/2026-11-07-s215-web-apps-issues.md` |

## 指摘（原文のまま）

### #22

> Rust のウェブアプリ 4 つがコンパイルできない（Options のフィールド不足。CI が見ていない） / The four Rust web apps no longer compile (missing Options fields; not covered by CI)
>
> > **ponytail によるレビュー。** Claude Code の ponytail（`/ponytail-audit` 5.3.2）が 2026-11-07 にリポジトリ全体を監査して出した指摘（1/20）。コードを読んだだけで、ビルドも実行もしていない。想定した負荷は 1 プロセスに開発者 1 人（整形・lint のウェブアプリだけはネットワークに置かれ、信用できない本文を受ける）。
>
> **反する原則（『プリンシプル オブ プログラミング』）:** DRY（同じ構造体を 4 か所に写している）、割れた窓の法則（CI が見ておらず壊れたまま）
>
> **場所:** `src/Rust/apps/web-axum/src/lib.rs:77-81`。web-actix・web-rocket・web-loco も同じ。
>
> **何か:** 各アプリが整形の設定 `globe_mode_fmt::Options` を、`indent`・`width`・`insert_origin` の 3 つを名指しして作っている。
>
> **問題:** `Options` にはあとから `blank_around_sections`（S061）と `format_html`（S062）が足された（`globe-mode-fmt/src/lib.rs:94-105`）。フィールドが足りないので 4 つとも `cargo build` がコンパイルエラーで止まる。別 workspace で、CI にビルドする job が無く、`.github/scripts/test_ci_plan.py:194` が明示的に除外しているので、`tests/api.rs` も一度も回っていない。
>
> **直し方:** 各リテラルに `..Options::default()` を足す（1 行 × 4）。CI に app ごとの `cargo check` を 1 つ足すか、フレームワーク比較が済んでいれば 3 つを消す（#41（lean）を見る）。
>
> **放置すると:** アプリとその試験が死んだまま。

### #36

> Rust 以外の整形・lint ウェブアプリに上限・時間切れ・一時ファイルの守りが足りない（Astro・Gleam・Phoenix・FastAPI） / The non-Rust format/lint web apps lack limits, timeouts and a safe temp file (Astro, Gleam, Phoenix, FastAPI)
>
> > **ponytail によるレビュー。** Claude Code の ponytail（`/ponytail-audit` 5.3.2）が 2026-11-07 にリポジトリ全体を監査して出した指摘（15/20）。コードを読んだだけで、ビルドも実行もしていない。想定した負荷は 1 プロセスに開発者 1 人（整形・lint のウェブアプリだけはネットワークに置かれ、信用できない本文を受ける）。
>
> **反する原則（『プリンシプル オブ プログラミング』）:** 防御的プログラミング（外からの入力の上限、子プロセスの時間切れ、一時ファイルの排他）、契約による設計（`doc/web-apps.md` の契約〔4xx〕からのずれ）
>
> **場所:** `src/Astro/fmt/src/pages/api/format.ts:135`・`src/lib/globe.ts:13`、`src/Gleam/wisp_fmt` の Erlang 補助 15-18・35-37 行、Phoenix `globe.ex:26`、`src/Python/fastapi_fmt/app.py:76`、Phoenix `api_controller.ex:58-60`、Gleam `web.gleam`（`lint_request_decoder`）。
>
> **問題:**
> - Astro: `{"text":"…","indent":1000000000}` は `Number.isInteger` を通り、`globe` が約 1 GB の行を作ってメモリ切れか 500。`spawnSync` に `timeout` が無く、遅い `globe` 1 つで全要求が止まる。Node の既定 `maxBuffer`（1 MB）で、1 MB を越える整形結果は ENOBUFS → 「globe を起動できない」として 500。
> - Gleam: 本文を予測できる名前 `/tmp/wisp_fmt_<pid>_<n>.in` に `file:write_file`（symlink を辿る）で書く。同じ機械の別ユーザーが symlink を置けば、サーバのユーザーが持つ任意のファイルを要求の本文で上書きできる。60 秒の `after` は出力の塊ごとに延び、`port_close` は始まった `globe` を殺さない。
> - Phoenix: `System.cmd`（26 行）に時間切れが無い（一時ファイルは `:exclusive` で正しい）。
> - FastAPI・Phoenix・Gleam: `disable` の値をそのまま CLI に渡す。`["--json"]` は clap が終了コード 2 で拒み、`doc/web-apps.md` §3 の 4xx でなく 500 になる（引数注入ではない。リストか `"$@"` で渡している）。
> - 契約のずれ: Phoenix（`positive_int?`）と Gleam（`indent >= 1`）は `indent: 0` を拒むが、`doc/web-apps.md:52` は有効としている。
>
> **直し方:** indent を 0..16 に（ほかのアプリと同じ）、`spawnSync` に `{ timeout: 60000, maxBuffer: 64 << 20 }`、Gleam は `[write, exclusive]` で開く、`disable` は Astro の `^[a-z][a-z-]*$`（`lint.ts:162`）を写す。
>
> **放置すると:** 共有の箱で簡単にサービスを止められる。

## 採否

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 22 | 高 | `web-{axum,actix,rocket,loco}/src/lib.rs` | `Options` のフィールド不足でビルドできない | 採用 | 4 種とも `cargo build` が E0063 で落ちるのを確かめた（axum は main のまま、ほかは直しを戻して）。`..defaults` を足し、CI にジョブ `web` |
| 36-a | 中 | Astro | `indent` の上限が無い、`spawnSync` に時間切れ・`maxBuffer` が無い | 採用 | 本文 1 MiB（413）、indent 0〜16、`timeout` 10 秒、`maxBuffer` 16 MiB。issue の「60000」ではなく共通の 10 秒（[web-apps.md の 3.1](../../doc/web-apps.md)） |
| 36-b | 中 | Gleam | 一時ファイルが排他でない、60 秒の `after` が延びる、kill しない | 採用 | `exclusive` で書き、書けたときだけ消す。全体の期限で数え、`kill -9`。試験で時間切れを確かめた |
| 36-c | 低 | Phoenix | `System.cmd` に時間切れが無い | 採用 | Port に替えて期限で `kill -9`。試験が直す前は落ちた |
| 36-d | 低 | FastAPI・Phoenix・Gleam | `disable` の値をそのまま渡す（`["--json"]` で 500） | 採用 | 規則名の形（`^[a-z][a-z-]*$`）に合わなければ 4xx。FastAPI と Phoenix の試験は直す前に落ちた |
| 36-e | 低 | Phoenix・Gleam | `indent: 0` を拒む（契約からのずれ） | 採用 | 0〜16 に直した |

## 結論

- 採用 6 件（行）・保留 0 件・不採用 0 件。#22 は 4 種とも再現した。#36 は FastAPI の試験 3 本と Phoenix の試験 4 本が直す前に落ちた（残りは読みと新しい試験で確かめた）。
- 指摘のうち誤っていた点: 「Phoenix の一時ファイルは `:exclusive` で正しい」は正しく、変えていない。Gleam の `file:write_file` が symlink を辿る件は、`exclusive`（`O_EXCL`）が symlink を辿らないので、これで防げる。

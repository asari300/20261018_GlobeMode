# S053 `fmt-lint-completion`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s053-fmt-lint-completion` |
| worktree | `.git-worktree/s053-fmt-lint-completion` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S053-fmt-lint-completion/` |
| セッション記録 | `log/sessions/<日付>-<セッションID>.md` |

## 依頼

コード補完・リンタ・フォーマッタの機能を作って調整する。言語設計を確かめて計画を立て、設計の調査中にユーザーと対話して好みの整形の型を記録する。並行するセッションと衝突しない形で進める。IDE のほかに CLI・VS Code 拡張・ウェブアプリ（marimo・Elixir・Axum・Actix-web・Svelte）から単体でも動かせる設計にする。基本は Rust。

## 決めたこと

ユーザーと決めたこと（対話、AskUserQuestion）:

- 整形の型（`doc/format-style.md`）: インデント 2（4 は明示したときだけ）、80 桁で折り返す、annotation の図形プリミティブの引数を「位置・形状 → points → fill → line」に並べ替えてグループの境目で必ず改行、`origin` が無ければ `origin = {0, 0}`（オフにできる）、`within` の後・クラス頭の後・`end` の前に空行、`equation` / `algorithm` / `annotation` はクラスと同じ段、二項演算子の両側に空白・カンマは後ろだけ・行末の空白なし。ユーザーの Before/After（Alfheim の Objects）をそのままテストの期待値にした（末尾の空白と `),` の段ずれだけ直した）。
- lint の既定: `line-too-long` / `class-name-pascal-case` / `missing-description`。`unformatted` は入れない。
- 作業場所: main 側の `.git-worktree/s053-…`（最初の checkout `asari300/loggerhead` は規約と違ったため）。
- 途中の追加依頼: 整形器の完成を待たずにウェブアプリ群を作る。共通の設計を先に決め、フレームワークごとに別のサブエージェント（Haiku、effort high）。必須は Phoenix・Streamlit・Astro・FastAPI(httpx)・Rocket・loco に加えて axum・actix-web・marimo。追加で Gleam wisp を提案して作った。Go・Deno・Julia・Zig は提案だけ（`doc/web-apps.md`）。

既定のまま進めたこと:

- 上流 `rumoca-tool-fmt` / `tool-lint` は不採用（折り返し・並べ替え・空行・origin が無い。`crates/rumoca-tool-fmt/src/format_options.rs` で確認）。自前の crate `globe-mode-fmt` は依存 0 で、wasm と CLI でそのまま使う（ADR-0024）。
- 安全装置: 出力を字句解析し直してトークン列を照合。壊れた入力は `Err` で本文を触らない。
- 補完は予約語 + `Outline` の名前 + ライブラリの最上位クラス名。`.` の後ろは部品の型の部品名だけ。新しい解析は入れない。
- 衝突の回避: 新しいコードは新ファイル（`fmt` crate、`lsp/completion.rs`、`wasm/fmt.rs`、`Svelte/lib/editor.ts`）。s049 が触る `lsp/lib.rs`・`App.svelte`、s051 が触る `wasm/lib.rs` への変更は差し込みだけ。
- ウェブアプリは独立したプロジェクト（Rust は空の `[workspace]`、Python は各自の `pyproject.toml`/`uv.lock`）。ルートの workspace・`pyproject.toml`・CI・`doc/licenses.md` はサブエージェントに触らせず、ライセンスの表は親がまとめて写した。Streamlit はルートの依存に無かったので、独自の `pyproject.toml` にした。
- ライセンスの判断: Python の typing-extensions（PSF-2.0）と npm の lru-cache など（BlueOak-1.0.0）を許可に足した。certifi（MPL-2.0）は改変しないので条件付きで許可。astro の optional な sharp（LGPL）は除外した。
- `src/Modelica` の 69 本は整形で全部変わる（空行の規則）が、書き換えはしない。

## やったこと

- `src/Rust/mod/globe-mode-fmt/`（新規）: `lexer.rs`（コメントもトークン、直前の改行数）、`structure.rs`（クラス・節・ブロックの段付けと空行、`;` 抜けと `end` 不一致の検出）、`layout.rs`（括弧の木を 80 桁に配置）、`annotation.rs`（並べ替えと origin）、`lint.rs`、`tests/format.rs`（Before/After、冪等、壊れた入力、インデント 4、lint）。
- `globe-mode-cli`: `fmt`（`--check` / `--indent` / `--width` / `--no-insert-origin` / 標準入力の `--json`）と `lint`（`--disable` / `--json`）。
- `globe-mode-lsp`: capabilities に formatting と completion、`completion.rs`、lint の診断（`GM-L0001`〜`0003`）。統合テストに 3 つの往復を追加。
- `globe-mode-wasm`: `fmt.rs`（`format` / `lint`）、`worker.js` / `client.js` に口を追加。
- `src/Svelte`: `@codemirror/autocomplete` と `@codemirror/lint` を追加、`lib/editor.ts`（補完・lint の波線・検査の診断の波線）、`Backend.format` / `lint`、「整形」ボタン、UI テストに整形の手順。
- ウェブアプリ 10 種（`doc/web-apps.md`）: `src/Rust/apps/web-{axum,actix,rocket,loco}`、`src/Elixir/phoenix_fmt`、`src/Gleam/wisp_fmt`、`src/Python/{streamlit,marimo,fastapi_fmt}`、`src/Astro/fmt`。
- 文書: `doc/format-style.md`、`doc/web-apps.md`、ADR-0024、features / architecture / roadmap / licenses / ADR 索引、セッション文書。

## 検証

- コードの検査（fmt / clippy / test / deny / uv）: `mise run check` 通過（1 回目は LSP の補完の単体テストが落ちた。試験の本文の `p.` が文法として壊れていて outline が取れなかったので本文を直した）。`mise run ui:check`: svelte-check 0 件、Playwright の `test_ui.py`（整形ボタンと lint の波線を含む）と `test_graph.py` 通過（1 回目は lint の波線の `data:` SVG が CSP に弾かれたので `img-src 'self' data:` を足した）。`globe fmt --check src/Modelica`: 69/69 本に差分
- `check_docs.py`: 238 文書 / リンク 1207 件中の切れ 0 件。`render_slides.py`: 56 デッキ、問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（メール・ホームのパス・トークン）

- main（S049〜S052）の取り込み: 競合は `lsp/lib.rs`・wasm の `client.js` / `worker.js`・sessions 索引の 4 つで、どれも差し込み行どうし。S049 の `TauriBackend` が `Backend.format` / `lint` を持たず svelte-check が落ちたので、LSP に独自の要求 `globe/format` / `globe/lint` を足して Desktop からも同じ口で呼べるようにした（ADR-0023 の表に追記）。

## 残したこと

- 設定ファイル（`globe-fmt.toml`）、定義ジャンプ・リネーム、部品の型を辿る深い補完、VS Code 拡張の実体。
- 文法が壊れた文書（入力途中の `p.` など）は outline が Err になり、補完は予約語だけになる。前回の outline を保つ案は次回。
- `src/Modelica` の 69 本を新しい整形で書き換えること（ユーザーの判断待ち）。
- ウェブアプリ 10 種はサブエージェントの単体テストだけで、ブラウザからの操作は確かめていない。`web-loco` は `cargo deny` が通らない（licenses.md に記載）。
- markdownlint は手元に無く、目視だけ。

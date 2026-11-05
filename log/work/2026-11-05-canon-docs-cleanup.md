# S177 `canon-docs-cleanup`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s177-canon-docs-cleanup` |
| worktree | `.git-worktree/s177-canon-docs-cleanup` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S177-canon-docs-cleanup/` |
| セッション記録 | （親が写す） |

## 依頼

2026-11-02〜05 の 80 本近い並行取り込みで、features・roadmap・索引・architecture・upstream・licenses・ADR 索引に溜まった重複・食い違い・古い記述を、事実を変えずに片付ける。CLAUDE.md は触らず、古い所は報告に書く。

## 決めたこと

- 重なった行は新しい方を残し、古い方にしかない文を足して消す。並行の WP（S176）と衝突しない最小の差分にする。
- 済んだ持ち越しは、同じ文書の別の節（済んだセッションの節）に根拠があるものだけ `~~…~~（S◯◯ で済）` にした。確かめられないものは触っていない。
- 索引は降順（新しい順）に統一した。S 番号の衝突（別ブランチの S136・S137・S150・S152）は索引に手を入れず報告だけ。
- 「いま残っているもの」は roadmap の末尾の 1 つの表。S172 の 3 行の表は置き換え、S164 の箇条書きはそこを指す。

## やったこと

[解説書](../../doc/sessions/S177-canon-docs-cleanup/解説書.md) の「直した所」の表のとおり。変更は `doc/` の 6 文書と、S177 のセッション文書・この作業記録・索引の 1 行だけ。

## 検証

- `check_docs.py`: 文書 724 件 / 相対リンク 3664 件 / 切れ 0 件（デッキは S177 を含め全部 OK）。
- `render_slides.py`（S177 のデッキだけ）: デッキ 1 件 / 問題 0 件（1 回目で通った）。
- セッション記録の `--check`: 親がコピーするので回していない。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件。
- `src/`・`.github/`・`CLAUDE.md` の差分: 0 件。
- コードの検査（fmt / clippy / test / deny）: Rust に触れていないので回していない。

## 食い違いの記録（直さず報告したもの）

CLAUDE.md（触っていない。古い所だけ）:

| 場所 | 古い所 | 実際 |
| --- | --- | --- |
| 2 章の表 | `src/VSCode/` と `src/Astro/` の行が無い | 拡張 `src/VSCode/globe-mode`（S059）と、ウェブアプリ群の `src/Astro/fmt` がある |
| 2 章の表 | `src/Rust/apps/` を「Tauri のシェル、Leptos / Dioxus の試験版」とだけ書く | `apps/desktop`、`apps/experimental/{leptos,dioxus}`、`apps/web-{actix,axum,loco,rocket}`（S053）。試験版は `apps/experimental/` |
| 2 章の表 | `src/Python/mod/` `marimo/` `streamlit/` | `fastapi_fmt/` もある。`src/Elixir/phoenix_fmt`・`src/Gleam/wisp_fmt` は「薄いラッパ」でなくウェブアプリ |
| 2 章の表 | `src/Svelte/` を「Tauri とブラウザで共用」 | Android（WorkerBackend）でも共用 |
| 2 章の表 | `doc/bench/`、`doc/plan/`、`doc/format-style.md`、`doc/web-apps.md` の行が無い | あり。`doc/msl-*.md`・`doc/libraries.md` は生成物で、一部だけ本文が指す |
| 2 章の説明 | workflow の説明に `desktop.yml`（S049・S160）・`web.yml`（S145）・`release.yml`・`resources.yml`・`fmu-export.yml` が無い | architecture.md の 2.5 章に 11 本の一覧を置いた |
| 2 章の説明 | mise の行（表）に node・pnpm・cargo-deny・wasm-bindgen が無い（「rust / uv / zig / julia / erlang / elixir / gleam」） | `mise.toml` の `[tools]` に node・pnpm・cargo-deny・wasm-bindgen もある |

README / CHANGELOG と features の食い違い:

| 場所 | 食い違い |
| --- | --- |
| CHANGELOG | 冒頭が「S001〜S164 の要点」。S166・S167・S169〜S171・S173〜S175 の行が無い（S165 と S168 と S172 はある） |
| README の "What works" | 参照とリネームを "inside one class" と書く。S166 でクラス名（この文書で定義したクラス）も同じ口でできる。文意は壊れていない |

## 残したこと

- ADR-0032・0034・0035・0036 の状態（Proposed。実装と測定は済んだ）はユーザーの判断。
- 別ブランチの S 番号の衝突: remote の `feature/s136-review-map`・`s137-review-fmt-panics`・`s150-review-fmt-crlf`・`s152-review-fmt-structure` は、索引の S136（progress-cancel-lsp）・S137（fmu-runtime-fmi3）・S150（ui-tunable-params）・S152（ui-dark-mode）と同じ番号を使う。ブランチの 1 つは中身を S151 に振り直したコミットを持つ。
- ADR-0027 は欠番（git の履歴にも無い）。

# S217 `doc-yomiyasu`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s217-doc-yomiyasu` |
| worktree | `.git-worktree/s217-doc-yomiyasu` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S217-doc-yomiyasu/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

「doc と日本語の内容監査を yomiyasu でリファクタリング」（2026-11-08、ユーザー）。正典と準正典の日本語を yomiyasu の規則で推敲し、文書どうし、文書とコードの食い違いを見つけて直す。

## 決めたこと

- 意味・条件・数値・担当・【未確認】を変えない直しだけを採る。計画の中身、ADR の決定の文言、英語の要約は触らない。
- lint の「版」「閉包」は残す（この文書群で定着した語）。
- 食い違いはコードや他の文書で確かめたものだけ直す。

## やったこと

文書ごとの lint の前後（見直し候補の件数）:

| 文書 | 前 | 後 |
| --- | --- | --- |
| README.md | 5 | 1 |
| CLAUDE.md | 4 | 3 |
| doc/architecture.md | 24 | 23 |
| doc/roadmap.md | 62 | 56 |
| doc/features.md | 8 | 7 |
| doc/licenses.md | 8 | 7 |
| doc/upstream.md | 7 | 5 |
| doc/format-style.md | 5 | 4 |
| doc/web-apps.md | 2 | 1 |
| doc/release.md | 6 | 6 |
| doc/adr/*.md（35 本） | 180 | 137 |
| src/*/README.md（15 本） | 37 | 24 |

直した食い違い:

- README: 日本語の「できていないもの」が FMI の読み込みを未とし、「今後必要になりそうな機能」の表も読み込みを未としていた。CLI の `globe fmu run`（S204・S205・S210）は済んでいるので、UI と各言語の API が未、と直した。同じ表の `src/Modelica` の整形が「していない」だったのを、S189 で済み、CI が `fmt --check` を回す、に直した（`.github/workflows/ci.yml`・`mise.toml` で確認）。
- CLAUDE.md: workflow の一覧に `fmu-import.yml`（S204）が無かったので足した（architecture.md の 2.5 章と `.github/workflows/` は 12 本で一致）。
- architecture.md: Tauri のシェルの対象に macOS が無い、mise.toml の道具の一覧が古い、crate 表の注記「（S002）と書いたものは実装済み」が古い。`globe-mode-solver`・`globe-mode-testkit` の 2 つは `src/Rust/mod/` に無いので、そう書いた。
- roadmap.md の末尾の表: FMI Import の置き方が「いまは Proposed」だったが ADR-0026 は Accepted（S204）。S185 の項の「README が補完と検索を『無い』と書いたまま」は README が S171・S167 の状態になっているので打ち消し線にした。
- format-style.md: 「`globe/format`・`globe/lint` は設定ファイルを読まない」は S143 から古く、S214 で `globe/format` の indent は設定ファイルが勝つ（`globe-mode-lsp/src/lib.rs` で確認）。`src/Modelica` は整形済み（S189）。
- web-apps.md: 「Svelte の IDE 本体に整形ボタンを足す」は作ってあるので提案から外した。
- src/Svelte/README.md: 「使う部品」に挙げた `@codemirror/lsp-client`・cosmos.gl・Svelte Flow は `package.json` に無い。入っているものに書き直した。src/Rust/README.md は S002 の時点の表であることを注記し、構成に macOS と web-* を足した。
- mise-setup.html: S001 の時点の手順であることと、いまの正（`mise.toml`・ADR-0003）を冒頭に足した。

## 検証

- `check_docs.py`: デッキ全件 OK / リンク 4223 件中の切れ 0 件（文書 874 件）。
- `render_slides.py`（S217 のデッキだけ。`uv run --no-project --with playwright`。`uv run` だけだと maturin が worktree に `target/` を作るので避けた）: 3 構成 × 6 枚、問題 0 件。
- コードの検査（fmt / clippy / test / deny）: Rust は触っていない。
- セッション記録の `--check`: 親が写す。

## 残したこと

直せなかった、または直さなかった食い違い:

- README の英語の要約（17 行目）に「Not done: FMI import」が残る。日本語の側は直した。英語は触らない指示のため。
- `globe-mode-lsp/src/lib.rs` の 34 行目のコメント「`globe/format`・`globe/lint` は wasm と同じく params だけで決める」は、同じファイルの 502 行目以降（設定ファイルを土台にする）と食い違う。コードなので触っていない。
- `doc/mise-setup.html` の完成例（uv 0.12、ツールの一覧）とタスク定義の例は S001 のまま。
- ADR-0026 の「trait の形（実装済み＝CS。ME は設計だけで…）」は S204 の時点の記述で、ME は S205 で済んでいる。ADR は追記で扱うので直していない。

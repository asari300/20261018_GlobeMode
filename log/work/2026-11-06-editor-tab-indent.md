# S188 `editor-tab-indent`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s188-editor-tab-indent` |
| worktree | `.git-worktree/s188-editor-tab-indent` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S188-editor-tab-indent/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

ユーザーの決定（2026-11-06）「テキストエディタでは Tab は字下げ、フォーカス移動のショートカットは別で用意」。Svelte 版のエディタで Tab を字下げにし、フォーカスを出す手を別に用意して案内する。S186 が `App.svelte`・`test_ui.py` を触っていたので、その merge（`f03137c`）を待って main から切った。

## 決めたこと

- 字下げは `indentWithTab` + `indentUnit.of("  ")`。タブ文字は入れない。幅は整形器の既定の 2（`fmt.indent` への追従は持ち越し）。
- フォーカスを出す手は CodeMirror の組み込み（Esc のあと 2 秒以内の Tab / Shift+Tab、Ctrl+M か Mac の Shift+Alt+M の `toggleTabFocusMode`）。使っている版 `@codemirror/view` 6.43.15・`@codemirror/commands` 6.11.1・`@codemirror/autocomplete` 6.20.4 のソースを npm から落として確かめた。Ctrl+M は App が使っている `defaultKeymap` に既にある。出典は解説書。確認日 2026-11-06。
- Esc は当たらない: view の keydown が Esc で窓を開くだけで握りつぶさない。補完の Esc は従来どおり一覧を閉じる（試験で確認）。
- 補完の一覧が開いているとき: `completionKeymap` は Enter だけが確定で Tab は束ねていない。依頼文の「Tab で確定」は既定ではなかったので足さず、Tab は字下げ、確定は Enter のまま。
- 案内の要素は `class="note"` にしない（`p.note` を strict に探す既存の試験が 2 要素で落ちた）。`class="hint"` にして同じ色の規則に足した。
- `indentMore` はカーソルの位置でなく行頭に入れる（CodeMirror の仕様）。

## やったこと

- `src/Svelte/src/lib/editor.ts`: `EDITOR_HELP`、`modelicaTab`（`indentUnit`・`indentWithTab`・`aria-describedby`）。
- `src/Svelte/src/App.svelte`: 拡張に `modelicaTab`、エディタの下に `#editor-help` の 1 行（`.hint`）。
- `src/Svelte/tests/test_ui.py`: S175 の `tab_to` はエディタにいるとき Esc のあと Tab（以前は Tab でエディタを出られた）。S179 のスキップリンクの試験は本文の先頭からの Tab で、変更なし。新しい節「エディタの Tab（S188）」。`GM-SMOKE` の行は変えていない。
- README のショートカットの表に 2 行、CHANGELOG（挙動が変わる点）、`doc/features.md` の Text ビュー、`doc/roadmap.md`（S175 の持ち越しに打ち消し線、S188 の節）、索引、デッキと解説書。Rust は触っていない（`mise run check` は不要）。

## 検証

- wasm: main `f03137c` の先頭を release でビルドし（`~/.cache/globe-mode-s188/wasm-pkg`、終わりに消した）、`pnpm build` に渡した。
- `pnpm check`: 0 エラー 0 警告。
- `test_ui.py`: 全部通った（`GM-SMOKE` の行は変えず、`isolated=1` など従来どおり）。新しい節は途中で 4 回直した（試験の書き方の問題で、実装は変えていない）: 補完は `insert_text` では開かず `type` で開く、Enter の確定は `li[aria-selected]` を待つ、`focus()` のあとカーソルの行が変わるので Ctrl+Home、最後の Esc の確認は `" mod"`（語頭になるように）。`test_graph.py`: ok。
- `check_docs.py`: デッキ OK（S188 を含む）/ リンク 3815 件中の切れ 0 件。`render_slides.py` S188: デッキ 1 件 / 問題 0 件。
- セッション記録の `--check`・コピーは親が行う（欄は「親が写す」）。ステージした差分の漏洩検査は下のコミット前に行った。

## 残したこと

- `fmt.indent`（`globe-mode.toml`）への追従、Enter のあとの自動字下げ、Tab での補完の確定。
- Leptos・Dioxus の試験版（`<textarea>`）はそのまま。VS Code の拡張は VS Code の既定。
- スクリーンリーダーの実機での読み上げは未確認。

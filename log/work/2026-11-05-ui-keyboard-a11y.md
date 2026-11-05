# S175 `ui-keyboard-a11y`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s175-ui-keyboard-a11y` |
| worktree | `.git-worktree/s175-ui-keyboard-a11y` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S175-ui-keyboard-a11y/` |
| セッション記録 | （親が写す） |

## 依頼

S164 の監査の穴 8（N10）: 診断の行がキーで選べない、プロットの canvas に代替テキストが無い、実行・検査のショートカットが無い。キーボードだけで開く・書く・検査・診断へ飛ぶ・実行・止める・結果・保存まで通せるようにする。まずマウスなしの Playwright で詰まる所を洗い出し、最小で直す。

## 決めたこと

- 診断・参照の行は `tr` に `tabindex` や `role` を付けず、先頭のセルに本物の `button.jump` を置く（表の意味を壊さない。button は onclick を持たず、click が tr の既存の onclick へ泡立つ）。
- ショートカットは `<svelte:window onkeydowncapture>` で受ける。Ctrl+Enter の実行は CodeMirror の `Mod-Enter`（空行を挿入）を覆うが、依頼の例のとおり。ほかは既定のキーと当たらない。Ctrl+. を止めるに選んだのは、macOS の「中止」の慣習で、Esc は補完・検索・リネームの取り消しと当たるため。
- 完了の読み上げは `role` なしの `aria-live="polite"` の `p`。`role="status"` にしなかったのは、既存の試験が `get_by_role("status")` を 1 つと見ているため。
- プロットの `aria-label` は canvas 自身に付ける（`Plot.svelte` の描画は変えず、作った後の属性だけ）。`model` は `untrack`（入力のたびに作り直さない）。
- 洗い出しで他の面（実行の設定・信号・書き出し・テーマ・強行・タブ・木の検索）はキーだけで通ったので触っていない。

## やったこと

- `src/Svelte/src/App.svelte`: `shortcut`、ボタンの `title` と `aria-keyshortcuts`、診断・参照の行の button、`live` と `aria-live` の `p`、`h1`、`.jump` の CSS。
- `src/Svelte/src/theme.css`: `:focus-visible`、`.sr-only`。
- `src/Svelte/src/lib/editor.ts`: エディタのフォーカスの輪郭 1 行。
- `src/Svelte/src/lib/plot/signals.ts`: `plotLabel`。`Plot.svelte`: canvas への `role` と `aria-label`。
- `src/Svelte/tests/test_ui.py`: キーだけの 1 本。
- `README.md`（英語の表と日本語の要約）、`doc/features.md`、`doc/roadmap.md`、`doc/sessions/README.md`、S175 のスライドと解説書。

## 検証

- wasm: Rust を変えていないので、main 5eb8787（S173 のマージ）の `globe-mode-wasm` を release でビルドし（共有 target、`CARGO_BUILD_JOBS=2`、wasm-bindgen の出力は scratch の `wasm-pkg/web`）、`CARGO_TARGET_DIR=<scratch>` で `pnpm build` に渡した。
- 洗い出し（直す前、5eb8787 の dist）: 診断の行に Tab で届かない、Ctrl+Shift+Enter・Ctrl+Enter・Ctrl+.・Ctrl+S が効かない、canvas が無ラベル、`h1` なし、エディタの輪郭が 1px の点線。直した後は同じスクリプトで全項目 OK（明・暗とも）。表は解説書 2.1。
- `pnpm check`: 0 ERRORS 0 WARNINGS（265 FILES）。
- `tests/test_ui.py`: 最後まで通った（`ok`）。「キーボードだけ」の 1 本を含む。`GM-SMOKE` の行は変わらない（`check=ok simulate=ok … tree=ok fmt=ok graph=ok cancel=ok settings=ok plot=ok def=ok isolated=1 cancel_path=flag`）。出力の `ConnectionResetError` の Traceback は socketserver の接続切れの雑音（ブラウザが閉じるとき）。
- フォーカスの輪郭のコントラスト比: 明 `#2563eb` 対 `#ffffff` は 5.17、暗 `#58a6ff` 対 `#0d1117` は 7.49（3 以上）。
- `check_docs.py`: S175 のデッキ OK / リンク 3,631 件中の切れ 0 件。`render_slides.py`（S175 のデッキだけ、他の cargo が止まってから）: 7 枚 × 3 構成、問題 0 件。
- ステージした差分の漏洩検査（メール・ホームのパス・トークン）: 0 件。セッション記録の写しは親がやる。
- Rust は触っていないので `mise run check` は回していない。

## 残したこと

- スキップリンク、タブの矢印キー、Esc で止める、エディタの Tab での字下げ。
- 【未確認】スクリーンリーダー（NVDA・VoiceOver・TalkBack）の実機、Mac の Cmd、Android の外付けキーボード。
- Ctrl+Enter が CodeMirror の「空行を挿入」を覆う（End と Enter で代わりになる）。

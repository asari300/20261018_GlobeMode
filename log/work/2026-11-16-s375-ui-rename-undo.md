# S375 `ui-rename-undo`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s375-ui-rename-undo`（main の `7aa4e1e9`〔S374 のマージ〕から切った） |
| worktree | `.git-worktree/s375-ui-rename-undo` |
| Phase | P4（UI のリネーム） |
| セッション文書 | `doc/sessions/S375-ui-rename-undo/` |
| セッション記録 | 専用の記録が `log/local/` に無いので省いた（並行の担当のうち、この WP の記録は取っていない） |

## 依頼

S368・S372 で UI のリネーム（F2）は開いている自分のタブの全部に当たるが、今のタブは 1 回の dispatch、ほかの自分のタブは `state.update` で当てるので、Ctrl+Z がタブごとにしか戻らない（S368 の残したこと）。最小版として、複数のタブにまたがるリネームを 1 つの操作として戻せるようにする（リネームの直後に「元に戻す（n ファイル）」ボタン。押すと各タブの本文をリネーム前に戻し、各タブの履歴に 1 つの変更として残る。リネームのあとに編集されたタブは戻さず理由を出す。キーボードで届く）。`src/Svelte` だけを触る。試験は `test_ui.py` の S368・S372 の形で 1 本。`pnpm check`、`test_ui.py` 全体。README・features の「参照・リネーム」の行と ADR-0023 の付記を直す。並行で S373（globe-mode-flat）が動く。

## 決めたこと

- **履歴は束ねない**。CodeMirror の履歴は `EditorState` ごとなので、タブをまたぐ履歴を作るより、リネームの時点で各タブの「変更の逆」（`tr.changes.invert(元の doc)`）とリネーム直後の本文（`tr.state.doc`）を持ち、ボタンで当てるほうが小さい。
- **2 ファイル以上に当たったときだけボタンを出す**。1 ファイルならそのタブの Ctrl+Z で足りる。既存の 1 タブのリネームの試験（S161・S166・S186）の画面も変わらない。
- **戻し方は「本文がリネーム直後のまま」のタブだけ**（`Text.eq`）。本文が変わった・閉じたタブは戻さず、`failure`（`role="alert"`）にタブの名前と理由を出す。ほかのタブは戻す（依頼どおり「そのタブは戻さず理由を出す」。全部戻すか何もしないかの二択にはしていない。一部だけ戻った状態になりうるのは残したこと）。
- **戻す transaction に `isolateHistory.of("full")` を付けた**。付けないと、リネームの直後（500 ms 以内）に戻すと履歴の同じ項に混ざり、戻したあとの Ctrl+Z の挙動が時間次第になる。付けたので、戻したあとにそのタブで Ctrl+Z を押すとリネームがやり直される（試験で確かめた）。
- **モデル名**: 今のタブは S186 の `modelRenamed` を `from`・`to` 逆にして同じ transaction に載せる。リネームしたタブが今のタブでなければ `tab.model` を `from` に戻す。
- **キーボード**: 通常の `<button>`。エディタから Esc のあと Tab（S188）で届く（試験は Tab を最大 40 回押して `[data-rename-undo]` の中に着くことを確かめ、Enter で押す）。専用のショートカットは足さなかった（`Ctrl+Shift+Z` などは CodeMirror の redo と重なる）。リネームの直後の `live` でボタンの在りかを知らせ、戻したあとはファイル数を `live` で知らせてフォーカスをエディタへ返す。
- 置き場は `App.svelte`（リネームの当て方がここにあるので）。`lib/` に切り出すほどの量ではない（約 30 行）。Rust・依存は触らない。

## やったこと

- `src/Svelte/src/App.svelte`: `RenameUndo` 型・`renameUndo` 状態、`rename()` で各タブの逆の変更と直後の本文を集める、`undoRenameAll()`、ボタンの行（`data-rename-undo`）。`isolateHistory`・`ChangeSet`・`Text` の import。
- `src/Svelte/tests/test_ui.py`: `cross_file_rename_undo`（S375）と、冒頭の説明、`main` からの呼び出し。
- 文書: README の 3 か所（概要・UI の範囲・拡張の行）、`doc/features.md` の「参照・リネーム」の行、ADR-0023 の付記 S375（と S368 の限界 (3) への付記）、roadmap（S375 の節と機能の穴の行）、CHANGELOG、セッション文書・索引、この記録。

## 検証

- `pnpm check`（svelte-check）: 288 ファイル 0 エラー 0 警告（最後の編集のあとも）。
- `test_ui.py` 全体: `data/msl` を MAIN から symlink し、`pnpm build`（wasm は共有の `target-shared/wasm-pkg/web`。Rust は触っていないので main の wasm と同じ）して回し、最後の `ok`・終了コード 0（S375 の行を含む。`busy` の守りと `disabled` を足した最終のビルドで 1 回、その前のビルドでも 1 回通した）。終わったら symlink を外した。S368・S372・S375 の 3 関数だけを回す駆動（scratchpad の `s375_only.py`）でも通した（`errors: []`）。
- 試験が確かめること（`cross_file_rename_undo`）: モデル名の欄が `Core` に追う、Esc・Tab（最大 40 回）でボタンに届き Enter で押す、A.mo・B.mo・モデル名が元に戻る、A.mo の Ctrl+Z 1 回でリネームがやり直される（履歴に 1 件）、もう一度リネームして B.mo を編集してから押すと B.mo はそのまま・`alert` に `B.mo` と「戻さなかった」が出て、A.mo だけ戻る。
- `check_docs.py`: 文書 1357 件 / 相対リンク 5583 件 / 切れ 0 件、デッキ（S375 を含む）OK。`render_slides.py`（自分のデッキだけ。desktop のライト・ダーク・mobile の 3 つ、6 枚ずつ）: デッキ 1 件 / 問題 0 件（1 回目は共有のランナーの負荷で screenshot が 30 s で時間切れ、撮り直して通った。S03・S04 を目で確かめた）。
- Rust は触っていないので `mise run check`・`wasm:node`・`wasm:compare` は回していない。
- `save_session_log.py --check`（`log/work/` の 1 枚）: 取りこぼし 0 件。ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークンの grep）: 0 件。

## 残したこと

- Ctrl+Z 1 回で全タブが動くわけではない（束ねるのはボタン）。ボタンの状態は保存しない（再読み込みで消える）。
- 一部のタブが戻らないと、リネームが一部だけ戻った状態になる（理由は出す）。
- 本文の比較は一致だけ。編集して元に戻した本文は「変わっていない」とみなして戻す。
- UI の実機（Desktop・Android の WebView）での確認はしていない（ブラウザの Chromium だけ）。

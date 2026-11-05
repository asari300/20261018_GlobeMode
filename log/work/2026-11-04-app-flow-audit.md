# S140 `app-flow-audit`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s140-app-flow-audit` |
| worktree | `.git-worktree/s140-app-flow-audit` |
| Phase | P4・P6 |
| セッション文書 | `doc/sessions/S140-app-flow-audit/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

ユーザーの方針「先に全体の流れとして使えるアプリを完成させる」に沿って、ブラウザ版・Desktop・Android で「使える流れ」がどこまで通り、何が欠けているかを 3 列の表にし、埋める順を提案する。正典（features.md・README・roadmap・索引）を S118〜S136 に同期する。コードは触らず、cargo は回さない。

## 決めたこと

- 判定はコードと試験を読んで行い、`test_ui.py` は回さなかった（main に `dist/` が無く、作るには wasm のビルド＝cargo が要る。他の WP が Rust をビルド中。他の worktree の `dist/` は別ブランチの S138 のビルドで、main の判定には使えない）。
- ○ は「その列で自動試験が通る」、△ は「コードはあるがその列の試験が無い」とした。Desktop・Android の UI の操作の試験が無いことが △ の数に出る。
- 点検で、依頼の流れに無かった段を足した: ファイルを開く・保存（依頼にあった）、実行の設定（ソルバ・終了時刻・パラメータ）、プロットの書き出し、小画面。
- 索引は S001〜S069 が昇順、S070〜S136 が降順で混ざっていたので、全行を S 番号の降順に並べ直した（行の中身は変えていない）。
- README の「今後」の表の 2 行（定義ジャンプ、整形の設定ファイル）が S130・S132 の後も「無い」だったので直した。

## やったこと

- `doc/research/app-flow-2026-11-04.md`: 23 段 × 3 列の表、要約、埋める順 10 件、根拠。
- `doc/features.md`: Text ビュー・FMI 2.0 Export・複数ソルバの 3 行。`README.md`: 英語と日本語の状態の段落、「今後」の表の 2 行。`doc/roadmap.md`: 「S140 で済んだこと」の節。`doc/sessions/README.md`: S140 の行と並べ直し。
- `doc/sessions/S140-app-flow-audit/slides.html`・`解説書.md`。

## 検証

- 表の集計（23 段）: ○ はブラウザ 11・Desktop 7・Android 4、△ は 3・7・10、× は 8・8・9、— は 1・1・0。
- コードの検査: 触っていない（Rust・Svelte・workflow は変更 0）。cargo は回していない。
- `test_ui.py`: 回していない（上の理由。CI の `ui` に任せる）。
- `check_docs.py`: スライドはすべて OK（S140 を含む）、リンク切れ 0（最終の実行は下）。
- `render_slides.py`: 手元では通らず、CI の docs で確認。main の `.venv` の python で 4 回（全デッキ 2 回、S140 だけ 1 回、既存の S135 だけ 1 回）とも最初のスクリーンショットが 30 s で時間切れ（`Page.screenshot` の `waiting for fonts to load... fonts loaded` の後）。既存のデッキでも同じなので、S140 のスライドの問題ではない。`check_docs.py` の外部参照・`<style>`・`style=` の検査は OK。
- セッション記録の `--check`: 記録は親が写す。
- ステージした差分の漏洩検査: 0 件（下のコミット前に実行）。

## 残したこと

- 埋める順は調査の 4 章。上位 5 つは、ファイルを開く・保存、実行の設定の UI、エディタの LSP 接続（S138）、FMU export の UI、Desktop の `globe-mode.toml`。
- Android の IME・タッチ・小画面、Desktop の WebKitGTK・WebView2 での sigma の WebGL は【未確認】。
- 親が main へマージするときは、S137〜S139 が索引に足す行と並びがぶつかる。S 番号の降順を保つ。

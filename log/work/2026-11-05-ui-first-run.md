# S179 `ui-first-run`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s179-ui-first-run` |
| worktree | `.git-worktree/s179-ui-first-run` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S179-ui-first-run/` |
| セッション記録 | （親が写す） |

## 依頼

roadmap 末尾の表「機能の穴（P4）UI の使い心地」: 起動直後の案内、スキップリンク、タブの矢印キー、未保存の印（再読み込みをまたいで正しく。`beforeunload`）。ついでに CHANGELOG の `[Unreleased]` に S166・S167・S169〜S171・S173〜S177 と S179 を足し、README の "inside one class" を S166 に合わせる。Rust・experimental・`plot/`・`LibraryTree.svelte`・`editor.ts` の中身・workflow は触らない。

## 決めたこと

- 案内は保存したタブが無い初回だけ。閉じた印は `globe-mode.welcome`（localStorage）。smoke では出さない。localStorage が読めなければ出さない。MSL の促しは未読み込みの間の 1 行。
- スキップリンクは `href="#editor"` / `#results` を `preventDefault` して `view.focus()` / 空の見出し `h2#results`（`tabindex=-1`、読み上げ専用）へ。`#results` を囲む要素は作らず、既存の DOM（`.plot`・`.diagnostics`）を動かさない。
- タブは選ぶと開く方式の roving tabindex。Delete・Ctrl+W は足さない。
- 保存済みの本文は、編集中のタブだけ localStorage に持つ（本文と同じなら書かない。`cap` の字数に数える）。古い保存（`clean` なし）は戻した本文を保存済みとみなす（従来どおり）。
- `beforeunload` は無かったので足す。未保存があるときだけ、ブラウザのみ（`!tauri`、smoke を除く）。
- 未保存の印は `rev`（`$state`）を進めて再計算する。`view` が `$state` でないため。

## やったこと

- `src/Svelte/src/App.svelte`: `welcome`・`closeWelcome`・`smokeMode`・`rev`、`persist` に `clean`、`onMount` の復元、`tabKeys`、スキップリンク `nav.skip`、`h2#results`、`#editor`、`beforeunload`、タブの `●`、CSS。
- `src/Svelte/tests/test_ui.py`: 1 本と docstring。
- `CHANGELOG.md`: S174 と、P4 に S166〜S179 の行（S166 は既存の行に合流）。冒頭の「S001〜S164」を「S001〜S179」に。S176 は main にまだ無く、ブランチの記述で 1 行（P8）。
- `README.md`: 英語の "inside one class" と日本語の 3 か所を S166 に合わせ、キーボードの段落に案内・スキップリンク・タブ・未保存の印を足した。
- `doc/features.md`・`doc/roadmap.md`（S179 の節、S175・S168 の持ち越しの打ち消し、末尾の表の該当行）、`doc/sessions/README.md`、S179 のスライドと解説書。

## 検証

- wasm: Rust を変えていないので、main の b14345f（S177 のマージ）の `globe-mode-wasm` を release でビルドし（共有 target、`CARGO_BUILD_JOBS=2`、wasm-bindgen の出力は scratch の `wasm-pkg/web`）、`CARGO_TARGET_DIR=<scratch>` で `pnpm build` に渡した。
- `pnpm check`: 0 ERRORS 0 WARNINGS（265 FILES）。
- `ready_ms`（`?smoke=1`、3 回、同じ環境）: 前 409・322・382（中央値 382）、後 343・321・339（中央値 339）。`decay_ms` は 201・199・236 から 203・195・202。smoke では案内を出さないので、差は雑音。
- `tests/test_ui.py`: 最後まで通った（`ok`、新しい 1 本を含む）。`GM-SMOKE` の行は変わらない（`check=ok simulate=ok … tree=ok fmt=ok graph=ok cancel=ok settings=ok plot=ok def=ok isolated=1 cancel_path=flag`）。出力の `ConnectionResetError` の Traceback は接続切れの雑音。
- Tab でエディタまで: 22 回（スキップリンク 2 つと案内の「閉じる」を含む）。スキップリンクで 1 回。
- 案内を出した初回の画面（1280 幅・1100x520・390 幅の暗）を撮って目で見た。表の崩れなし。結果の位置の試験（S170）は案内が出た状態でも通る（プロットの上端 y は 290 から 332）。
- `check_docs.py`: S179 のデッキ OK / リンク 3,675 件中の切れ 0 件。`render_slides.py`（S179 のデッキだけ、cargo が止まってから 1 回目）: 7 枚 × 3 構成、問題 0 件。
- セッション記録の `--check`: 記録は親が写すので、この worktree では対象なし。ステージした差分の漏洩検査（ホームのパス・メール・トークン）: 0 件。
- Rust は触っていないので `mise run check` は回していない。

## 残したこと

- タブの並べ替え、ツリーのアイコン、案内を出し直す操作、案内の多言語化、Esc で止める。
- 【未確認】スクリーンリーダー・Mac の Cmd・Android と Desktop の実機（`beforeunload` を Tauri に付けない判断の確かめを含む）。
- CHANGELOG の S176 の行は main に S176 がまだ無いブランチの記述。マージの順で外すか確かめる。

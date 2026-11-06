# S196 `ui-review-fixes` のレビュー（Opus/medium）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s196-ui-review-fixes` |
| 対象 | main `cc846ba` の `src/Svelte/src/App.svelte`・`lib/editor.ts`・`lib/tauri-backend.ts`（読むだけのレビュー。3 本のうち UI の分） |
| レビュアー | Opus（medium、読んで論証しただけ。再現はこのセッションでした） |
| 依頼の文面 | App.svelte と editor.ts の指摘（全部）、Desktop のシェルの 1 と 7（`tauri-backend.ts`）、LSP の参照・リネーム・補完の 3 の UI の側を直す |
| 作業記録 | `log/work/2026-11-06-s196-ui-review-fixes.md` |

## 指摘（原文のまま）

### App.svelte と editor.ts

```text
# レビュー: App.svelte と editor.ts（Opus/medium、main cc846ba、読むだけ）
1. [データ消失・確認済み・Must] App.svelte:583,593,645（原因 tauri-backend.ts:150,157）Desktop で同じ名前の別のファイルを開くと Ctrl+S が別のファイルを上書き（desktop.md の 1 と同じ）。
2. [正しさ・確認済み・Must] App.svelte:563-577 activate が写すのは file・model・own・diagnostics・saved だけ。renaming・refs・unsupported・canForce・forced・lastFinal が前のタブのまま残る。(a) タブ A で F2、タブ B を押して Enter → A の位置で B に rename。(b) A の参照の一覧の行を B で押す → RangeError か無関係な選択。(c) A の未対応の要約・強行の印・最後の値が B に出る。直し方: activate に `refs = []; renaming = null; unsupported = ""; canForce = null; forced = false; lastFinal = [];`。
3. [データ消失・確認済み・Must] App.svelte:600,120,568 ライブラリのタブは編集できるのに isDirty が own でないと常に偽（● も confirm も beforeunload も出ず、persist もしない）。閉じても setFile(…, "") は自分のファイルだけなので、書き換えた本文が Workspace に残る可能性（未確認）。直し方: isDirty から own の条件を外す。根本はライブラリのタブを EditorState.readOnly に（自分のファイルに切り替える道 :584 で state を作り直す）。
4. [正しさ・Must] App.svelte:554-560,479,374-389,402,451,790 busy を見ずに activate / run に入る道が 3 つ（Alt+← の goBack、F12 の definition、木の onOpen → openLibrary → run）。実行中にタブが切り替わり結果が別のタブに書かれる、run が二重に動き busy が早く戻る。モデル名の欄を検査中に打ち換えると canForce・checkedModel が新しい名前に。直し方: run の先頭と goBack・definition に `if (busy) return;`、check と simulate の頭で `const m = model`。
5. [負荷・確認済み] App.svelte:137,117-124,782 1 打鍵ごとに全タブ（ライブラリのタブも）の doc.toString() と JSON、localStorage へ同期で書く。rev++ で全タブの isDirty も。直し方: filter を map の前に、persist を約 300 ms 遅らせ beforeunload と pagehide で書き出す。
6. [データ消失・黙って落とすのは確認済み] App.svelte:88-90,92-101,122 localStorage が満杯でも上限（cap）で落ちても何も言わない。Desktop（Tauri）には beforeunload が無い（:700）。直し方: store が成否を返し、persist は失敗と kept.length < docs.length のとき saved に 1 文。
7. [正しさ] App.svelte:123 ブラウザで 2 つのウィンドウ → 最後に打った側だけが保存に残る。直し方: storage イベントで「別のウィンドウでも開いている」。
8. [a11y・可能性] App.svelte:764 進捗の行が role="status" で更新のたびに読み上げ。直し方: 実行中は role を外す。
9. [簡素化・確認済み] editor.ts:219,229 modelicaLinter の引数 file は未使用（void file）。App.svelte:672-673 jump は editor.ts:213 の byteToChar と同じ。App.svelte:489 at は :497 の offsetOf と同じ。約 -6 行。
10. App.svelte 862 行。分けるなら smoke()（:193-333）。先に指摘 2 の 1 行。
問題なし: XSS、ショートカット、$effect、補完の古い応答、リスナーの解放。
```

### Desktop のシェル（1 と 7）

```text
1. [データ消失・確認済み・Must] src/Svelte/src/lib/tauri-backend.ts:145,150,157,162 #paths が「ファイル名だけ」をキーに保存先を覚え、openFile / openPath が、UI がタブに本文を入れるかを決める前に書き換える。dir1/package.mo を編集中（未保存）に dir2/package.mo を開く（App.svelte:593 の kept で本文は dir1 のまま）→ 保存で dir1 の本文が dir2/package.mo を上書き。直し方: openFile / openPath は #paths に書かずパスを返すだけ。App がタブに本文を入れたときだけ登録。（App のレビューも同じ指摘: タブの名前をパスにして save に file を渡す案）。試験なし。
7. [正しさ・確認済み] tauri-backend.ts:192 Windows でドライブの根に FMU（C:\Foo.fmu）→ outDir が "C:"（ドライブ相対）。直し方: X: の形なら \ を足す、または Rust 側で Path::parent。
```

### LSP の参照・リネーム・補完（3 の UI の側）

```text
3. [正しさ・確認済み] completion.rs:286-289（LIMIT）、lib.rs:294 上位 200 件で切り、配列で返す（isIncomplete = false 扱い）。`Modelica.Units.SI.`（type が 516 個）で Voltage・Velocity・Temperature が打ち足しても出ない。直し方: LSP は CompletionList { is_incomplete: out.len() == LIMIT, items }。UI（editor.ts:117）は 200 件のとき validFor を付けない。
```

## 採否

再現は `src/Svelte/tests/test_ui.py` の `review_fixes`（偽の `window.__TAURI__` を含む）と `tests/test_tauri.mjs` で、**直す前の dist で落ちること**を確かめてから直した。

| # | 重さ | 場所 | 採否 | 理由・直した所 |
| --- | --- | --- | --- | --- |
| app 1 = desktop 1 | Must | `tauri-backend.ts` の `#paths` | 採用 | 再現（`/d1/package.mo` を編集中に `/d2/package.mo` を開くと別のタブにならず、保存先だけ d2 になる）。`#paths` を捨て、タブが本文を受け取ったパス（`Tab.path`）を `saveFile(…, path)` に渡す。同じ名前の別のパスは `tabKey` で別の key（`d2/package.mo`）のタブ。ブラウザ版は保存が毎回ダウンロードで上書きしないので取り違えない |
| app 2 | Must | `activate` | 採用 | 再現（参照の一覧が前のタブから残る）。`refs`・`renaming`・`unsupported`・`canForce`・`forced`・`lastFinal`・`strayLib`・`roHit` を切り替えで空に |
| app 3 | Must | ライブラリのタブ | 採用 | 再現（読み取りの案内が無い）。`EditorState.readOnly`（Compartment）、タブの上に案内、打つと案内（`beforeinput`）、整形・リネームは断る。ディスクの同名ファイルで自分のファイルに置き換わる既存の道（`openDoc` の同名）は編集可能に戻す（`makeEditable`）。直し方の `isDirty` から own を外す案は、編集できなくしたので不要 |
| app 4 | Must | `busy` を見ない道 | 採用 | 再現（実行中の F12 でカーソルが動く）。`run`・`goBack`・`definition` の頭で `busy` を見る。`check`・`simulate` は押した時点のモデル名を使う。木から開く（`openLibrary`）は `busy` 中は何もしない（画面の上では検査の途中に押せた「ライブラリのファイルを開く」が二重に動いていた） |
| app 5 | 負荷 | 打鍵ごとの保存 | 採用 | 打鍵の保存を 300 ms まとめる（`persistSoon`）。`persist` は閉じる・保存・タブの切り替え・`pagehide`・`visibilitychange`（隠れた）・`beforeunload`・コンポーネントの破棄ですぐ書く。ライブラリのタブは文字列にしない（`flatMap`）。再読み込み直前の打鍵が残る試験 |
| app 6 | データ消失 | localStorage | 一部採用 | 書けない・上限で落としたときは `storeNote` に 1 文（再現: setItem が投げる）。Desktop の `beforeunload` の無さは持ち越し（下記） |
| app 7 | 正しさ | 2 つのウィンドウ | 採用 | `storage` イベントで「別のウィンドウでも開いている」（`data-other-window`）。マージはしない |
| app 8 | a11y | 進捗の行 | 採用 | 実行中は role を付けず、止めた・終わったあと `status` |
| app 9 | 簡素化 | 消せるコード | 採用 | `modelicaLinter` の `file`、`jump` と `byteToChar`、`definition` の `at` と `offsetOf` |
| app 10 | 分割 | 862 行 | 不採用 | 範囲外（指摘 2 を先にとの注記のとおり） |
| desktop 7 | 確認済み | `exportFmu` の `outDir` | 採用 | `dirOf` を切り出し、`C:\Foo.fmu` は `C:\`。`test_tauri.mjs` に 5 件 |
| lsp 3（UI の側） | 確認済み | `rustCompletion` の `validFor` | 採用 | 再現（`Modelica.Units.SI.` のあと `Vol` で `Voltage` が出ず `ComplexVoltage` だけ）。応答が配列でも `{isIncomplete, items}` でも受け、200 件で切れていれば `validFor` を付けない |

## 結論

- 採用 11 件・一部採用 1 件（app 6）・不採用 1 件（app 10）。
- 持ち越し: Desktop（Tauri）の未保存のままウィンドウを閉じる確認。`onCloseRequested` は確認のあと閉じるのに `core:window:allow-destroy` の権限が要り（`capabilities/default.json` は `core:default` だけ）、手元で Desktop を起動して確かめられないので、権限を足さずに確認だけ入れると閉じられなくなる恐れがある。なお編集中の本文は localStorage に残るので、閉じても再起動で戻る。
- レビュアーが外した点: 指摘 3 の直し方（`isDirty` から own を外す）だと、ライブラリのタブの編集が自分のファイル扱いで保存される。読み取り専用にして編集できなくするほうが小さい。

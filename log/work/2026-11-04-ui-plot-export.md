# S146 `ui-plot-export`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s146-ui-plot-export` |
| worktree | `.git-worktree/s146-ui-plot-export` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S146-ui-plot-export/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S140 の点検（`doc/research/app-flow-2026-11-04.md`）の順位 8。プロットの書き出し（CSV と PNG）で結果を持ち帰れるようにする。最小の変更で。`App.svelte`（S144・S145 が触る）、Rust、workflows は触らない。MAT・凡例つきの PNG・SVG・ダークモードはやらない。

## 決めたこと

既定のまま進めたこと（ユーザーと新たに決めたことは無い）:

- `Plot.svelte` の中で完結させる。Plot は backend を受け取らないので、Desktop は `new TauriBackend()`（`#paths` を使わない書き出しだけなので状態を共有しなくてよい）、それ以外は `browser-file.ts` の関数を直接使う。
- PNG は canvas だけ。透明のままだと暗い背景で線が見えないので、白で塗った別の canvas に写して `toBlob`。凡例（DOM）は入らない。
- CSV は画面と同じ選択（選んだ系列と、x の列＝パラメトリックなら選んだ列）。見出しは必要なときだけ引用。
- 保存ダイアログの絞り込みは `.mo` 固定をやめ、名前の拡張子から作る（`extFilter`）。
- `browserSave(name, text)` の署名は変えない（`Backend.saveFile` の 3 つ目は `saveAs`）。内部の `download(name, data, type)` を共有する。

## やったこと

- `src/Svelte/src/lib/plot/Plot.svelte`・`plot/csv.ts`（`toCsv`）: ボタンと書き出し。
- `src/Svelte/src/lib/backend.ts`（`saveBinary?`）・`browser-file.ts`・`worker-backend.ts`・`tauri-backend.ts`（`saveBinary`、`Fs.writeFile`、`extFilter`）。
- `src/Rust/apps/desktop/capabilities/desktop.json`: `fs:allow-write-file` を 1 つ。
- `src/Svelte/tests/test_ui.py`（Decay の書き出しを 1 本）・`test_columns.mjs`（`toCsv`）。
- `doc/features.md`・`doc/roadmap.md`（P4 の行と「S146 で済んだこと」）・`doc/sessions/README.md`、スライドと解説書。

## 検証

- `pnpm check`: 0 エラー 0 警告（最初の版は `browserSave` の署名の衝突で 3 エラー、直した）。
- `test_columns.mjs`: ok。
- `test_ui.py`（`pnpm build` 後、main の `.venv` を `-I` で）: 通過。`Decay の書き出し: CSV 502 行 ['time,x', '0,1']、PNG 16121 バイト`。console の error と外への要求は 0。1 回目は load average 25 の中で実行ボタンが有効にならず時間切れ（負荷が下がってから再実行して通った）。
- `desktop.yml`（`gh workflow run --ref feature/s146-ui-plot-export`）: run 41703488573 成功（ui・Linux の deb・Windows の msi の 3 ジョブ。capability を足しても smoke が通る）
- コードの検査（root の `mise run check`）: Rust の crate は触っていない（capability の JSON だけ）ので回していない。cargo は回さない指示。
- `check_docs.py`: デッキ S146 は OK / リンク 2941 件中の切れ 0 件
- `render_slides.py`: 手元では通らず（S146 のデッキだけを 2 回、`Page.screenshot` が 30 s で時間切れ。全デッキの実行も S082 の途中で Chromium が閉じた）、CI の docs で確認する
- セッション記録の `--check`: 記録は親が写すので、ここでは回していない。
- ステージした差分の漏洩検査: 0 件

## 残したこと

- MAT の読み書き、凡例つきの PNG（uPlot の `root` を描く）、SVG、ダークモード。
- Android の WebView で Blob のダウンロードが動くか【未確認】（S141 と同じ）。
- Desktop の dialog の操作そのものは自動試験が無い（smoke は capability を足して起動するところまで）。

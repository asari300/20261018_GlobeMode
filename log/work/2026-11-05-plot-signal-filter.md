# S169 `plot-signal-filter`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s169-plot-signal-filter` |
| worktree | `.git-worktree/s169-plot-signal-filter` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S169-plot-signal-filter/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S164 の監査の穴 7（N5）。プロットの信号の一覧に絞り込みを足し、内部名（`$` で始まるもの）を既定で隠し、CSV・PNG のファイル名をモデル名と時刻から作る。

## 決めたこと

- 内部名の判定は TS の 1 行（`plot/signals.ts` の `isInternal`）。Rust 側（exec・sim・facade）に判定も列の属性も無かった。MSL の例題 6 本の CSV の見出しで調べると、内部名は `$derivative.*`（Cauer 3 本）、`$formal_derivative.*`（PID_Controller 13 本、Rotational.First 15 本）、`$state_coordinates[*]`（PID 6 本、First 4 本）で、先頭が `$` でないものは 0 本、DoublePendulum・ControlledTemperature・ChuaCircuit は 0 本。Modelica の識別子は `$` で始められないので利用者の変数とぶつからない。wasm / LSP / Python の応答に印を足すと 3 面に及ぶので採らない。
- 内部名はチェックを外すと、選択されていてもプロットにも CSV にも出さない（画面に出ているものだけ書き出す。S146 の仕様は保つ）。
- 一覧は一致した先頭 200 本 + 「ほか n 件」。「表示中をすべて選ぶ / 外す」は一覧に並んでいる分だけ（1,302 系列を一度に uPlot に渡さない）。
- 既定の選択を「内部名を除いた先頭 3 本」にした（Plot の挙動の小変更。`GM-SMOKE` の `plot=ok` は先頭の信号を押すだけで通る）。
- ファイル名は `<モデル名の末尾>-<YYYYMMDD-HHMMSS>.<ext>`（ローカル時刻、文字・数字・`_`・`-` 以外は `_`、空なら `result`）。`browser-file.ts`・`tauri-backend.ts` は名前をそのまま受けるので触らない。`App.svelte` は `<Plot {columns} {model} />` の 1 行。

## やったこと

- `src/Svelte/src/lib/plot/signals.ts`（新規）: `isInternal`・`matches`・`exportName`。
- `src/Svelte/src/lib/plot/Plot.svelte`: `model` 引数、入力欄・「内部の変数も表示」・一括ボタン・件数、`ys`（プロットと CSV の系列）を 1 つの `$derived` に、x 軸の選択肢で内部名を隠す。
- `src/Svelte/src/App.svelte`: Plot の行のみ。
- `src/Svelte/tests/test_columns.mjs`・`test_ui.py`: 純粋な関数、Decay の絞り込みとファイル名、MSL があれば `Rotational.Examples.First` で内部名の隠れ・チェック・CSV。
- 文書: 解説書・スライド・索引・features.md・roadmap.md。

## 検証

- Rust は触っていない（`mise run check`・`wasm:node` は回していない）。
- wasm は変えていないので、共有 `target-shared/wasm-pkg/web`（2026-11-05 19:49 のビルド、他のセッションのもの。main の `839f382` から Rust を変えていない）を scratch にコピーして `pnpm build`。
- `pnpm check`: 0 エラー 0 警告。`node tests/test_columns.mjs`: ok。
- `test_ui.py`（MSL あり、`data/msl` の symlink）: 全部通過。`GM-SMOKE … plot=ok` を含む行は項目の変化なし。
- 新しい出力: `Decay の絞り込み: 一致なしで一覧 0 本・選択 1 本のまま`、`First の内部名: 既定で一覧 54 本（隠れた内部名 19 本）、絞り込みで 3 本、チェックを外すと CSV から消えた`。
- DoublePendulum（信号 1,302 本、一覧 200 本）で入力から一覧の更新まで: `a` 84 ms、`ab` 68 ms、`abc` 66 ms、`frame_a r_0` 132 ms、空 64 ms。
- `check_docs.py`: デッキ OK / リンク 3547 件中の切れ 0 件。`render_slides.py`（S169 のデッキ）: 問題 0 件（1 回目）。
- セッション記録の `--check`: 親が写すので未実施。

## 残したこと

- x 軸の `<select>` の絞り込み。「最後の時刻の値」（App.svelte）の内部名。既定の選択を出力の信号に。プロットの canvas の代替テキスト。
- Desktop の保存ダイアログの `defaultPath` が新しい名前になることは自動では確かめていない（名前を渡す道は S146 のまま）。

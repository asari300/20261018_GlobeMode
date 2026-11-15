# S363 `ui-layout-shift`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s363-ui-layout-shift`（base はローカルブランチ `feature/s360-ci-windows-call-hierarchy`。S360 が main に未着だったため） |
| worktree | `.git-worktree/s363-ui-layout-shift` |
| Phase | P4（試験の安定）・UI の使い心地 |
| セッション文書 | `doc/sessions/S363-ui-layout-shift/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S360 で、UI の試験がたまに落ちる原因が「検査の終わりやタブの切り替えの直後に `SimSettings` の高さが非同期に 6〜47 px 動き、下のタブ列や診断の表がずれる」ことと分かった。S360 は試験の押し直しだけを直した。利用者が押す直前にずれると別の場所を押すので、UI の layout shift を直す。何が高さを変えているかを特定し、領域を先に確保する・出し入れを `visibility` にするなど最小の CSS・マークアップで直す。試験: 検査の前後・タブの切り替えの前後でタブ列の上端が動かない（±1 px）ことを確かめる 1 本。Svelte だけを触り、Rust は触らない。

## 決めたこと

- **測り方**: タブ列の上端（`.tabs` の `getBoundingClientRect().top + scrollY`）を、ページ内の `setInterval(5)` で記録した。viewport の座標は、検査・クリックによるスクロールで動いて紛らわしいので、文書の座標にした。`main` の直下の子（タブ列より上）の高さと `SimSettings` の子の高さも同時に記録して、何が動いたかを特定した。
- **原因は 3 つ（幅によって出方が違う）**:
  1. 「パラメータ」の `details` が、検査のあと `tunableParameters` が届いて初めて出る（検査のボタンが戻った約 200 ms 後）。1280 px で +15.2 px、390 px で +58.8 px（`summary` が 2 行に折り返し、`details` を挟んで前後のインラインの行も割れる）。
  2. ソルバの一覧が wasm から後で届いて、選択欄が「既定」だけの幅 48 px から 83 px に伸びる。740 px・750 px の幅（1 幅 10 px の走査で 2 か所）で同じ行の欄が折り返し +19.6 px。1280 px では出ない。
  3. タブの切り替え: `models` は 1 つの変数なので、2 つのモデルのファイルのタブから 1 つのモデルのタブへ戻った直後は前のタブの候補が残り、`model` が候補に無いので select に「（このファイルのモデルから選ぶ）」が足されてバーが 64 → 104 px（+40 px）になる。約 2 ms で戻るが、`refreshModels` が遅ければ長く続く。
- **`SimSettings` 以外（App のバー）も直す**: 依頼の文面は `SimSettings` だが、タブの切り替えの後の動きの原因はバーの `models`。タブ列の上端が動かない、が目的なので直した（3 行）。
- **直しは場所の先取り**: (1) `details` を検査の前から出す。1 行に固定（`white-space: nowrap; overflow: hidden; text-overflow: ellipsis`、全文は `title`）。まだ一覧が無いときの文は「パラメータ（検査すると、上書きできる一覧が出る）」、あるときは `パラメータ（モデル名。N 本。<注記か「値を打った行だけ上書き」>）`（注記を括弧の中に入れたのは、390 px で省略記号に隠れないため）。本体の表は `{#if listedModel}` のまま。(2) `select { min-width: 7rem }`（いちばん長い名前 `sdirk34hw` も収まる）。(3) `Tab.models`（離れるとき写し、戻るとき入れる）。`visibility` で出し入れする案は、`details` が `display` を持つ要素なので採らなかった（最初からある方が簡単）。
- **直さないもの**: 実行の開始の `p.run`・終了の `p.last`（押した操作の結果。`lastSettings` は `run()` の中、`busy` が戻る前に入るので、タブのボタンが無効のうちに終わる）。ソルバの一覧が届くまで「複数」のチェックボックスの欄が空（検査の終わり・タブの切り替えではない。一覧を静的に持つと Rust の登録表の複製になる）。
- **試験**: `layout_shift` 1 本。幅 1280・740・390 px。記録器は `add_init_script`、各段の 300 ms 後に最大と最小の差を見て 1 px 超で落とす。「開く」操作そのものの動き（新しいタブ・モデルの select が出る）は数えない。

## やったこと

- `src/Svelte/src/lib/SimSettings.svelte`: `details.params` を常に出す、`head`（`$derived`）、`.params summary` の `nowrap`、`select { min-width: 7rem }`。
- `src/Svelte/src/App.svelte`: `Tab.models`（離れるとき `Object.assign(prev, { …, models })`、戻るとき `models = t.models ?? []`）。
- `src/Svelte/tests/test_ui.py`: `SHIFT_REC`・`layout_shift`、冒頭の説明、`main()` から呼ぶ（`check_frontend_select` の後）。
- `src/Svelte/README.md`・`CHANGELOG.md`・`doc/sessions/S363-ui-layout-shift/`・索引・この記録。正典（roadmap・features・architecture）は計画が変わらないので触らない。

## 検証

- `pnpm check`: 287 ファイル、0 errors・0 warnings。`pnpm build` 通る。
- `layout_shift` を直す前の `src/Svelte/src` で回した（`git stash`）: 1280 px の初回の検査で **15.2 px** 動いて落ちる。直した後は 3 回連続で通る（各 13 秒）。
- 3 つの直しを 1 つずつ外した: `App.svelte` の `models` を外す → 1280 px のタブ 0 への切り替えで 40.0 px、`min-width` を外す → 740 px の初回の検査で 19.6 px、`summary` の `nowrap` を外す → 390 px の初回の検査で 19.6 px。どれも単独で必要。
- 幅の走査（`select` の `min-width` が無い版、初回のソルバの一覧の到着だけ、560〜1290 px・10 px 刻み 74 点）: 740 と 750 で 19.6 px。`min-width` ありは 0 点。
- スクリーンショット（1280・390 px、検査の前・後・`details` を開いたあと）で、見た目が崩れていないことを確かめた（検査の前に「▶ パラメータ（検査すると、上書きできる一覧が出る）」の行が増えた以外は同じ。390 px は括弧の途中で省略記号）。
- `test_ui.py` 全体（`data/msl` は main への symlink）: 最終版（注記を括弧の中に置いた版）で EXIT=0（`GM-SMOKE … msl=ok …`、`layout_shift` を通過、押し直しが働いたのは S360 の回帰試験の 1 回だけ）。注記を括弧の外に置いていた 1 つ前の版でも通った。`test_graph.py` も ok。
- `check_docs.py`: スライド全件 OK、リンク 5448 件中の切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件（1 回目はスクリーンショットの時間切れで、撮り直して通った）。markdownlint-cli2 はこの環境に無く、取得（`pnpm dlx`）が許可されなかったので回していない。目で、見出し・リスト・表・コードブロックの前後の空行を確かめた。

## 残したこと

- 保存された設定が「複数」のとき、ソルバの一覧が届くまでチェックボックスの欄が空で、届くと伸びる。直すなら欄の最小の高さを確保する。
- CI のフォント（Liberation など）での折り返しの位置は【未確認】。`ci.yml -f full=true` の `ui` で確かめる（手元は Chromium・Arial 相当）。
- 実行の開始・終了の表示（`p.run`・`p.last`）は直していない。
- 手元の調べ用スクリプト（記録器・幅の走査・スクリーンショット）はコミットしていない（scratchpad）。`layout_shift` の記録器がその最小版。

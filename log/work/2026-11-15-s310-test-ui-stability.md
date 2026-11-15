# S310 `test-ui-stability`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s310-test-ui-stability` |
| worktree | `.git-worktree/s310-test-ui-stability` |
| Phase | P4（試験の安定） |
| セッション文書 | `doc/sessions/S310-test-ui-stability/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

ブラウザ UI の試験 `src/Svelte/tests/test_ui.py`（Playwright）が、手元（WSL、負荷が高いとき）で時々落ちるのを直す。CI では通る。報告されていた落ち方は、冒頭の fold gutter のクリックの時間切れ、CodeMirror のホバー位置の RangeError、`render_slides.py` の `Page.screenshot` の 30 秒の時間切れ。

## 決めたこと

- 時間切れの上限は延ばさない。待ち方を替える。
- 負荷は、CPU を使い切る `while :; do :; done` を 8〜16 本走らせて作った（16 コアの機械で load 11〜43）。実際に cargo が走っている状況の代わり。
- wasm（`target-shared/wasm-pkg/web`）は別の担当が 10:47 に作ったものを使い、再ビルドしなかった。UI の試験が確かめるのは wasm の中身ではなく、待ちの挙動だから。

## 落ち方・原因・直し方

| 落ち方 | 再現 | 原因 | 直し |
| --- | --- | --- | --- |
| 冒頭の fold gutter の `mark.first.click()` が 30 秒で時間切れ（call log は `waiting for element to be visible, enabled and stable` で止まる） | 負荷あり（load 4 → 11）で 1 回目に落ちた。負荷を上げた切り分けの 3 回も毎回 8 秒で落ち、DBG の rAF は最初の 1 秒で 1 回しか出ず、そのあと毎秒 60 回に戻る | Playwright の actionability の「stable」は rAF で位置を 2 回比べる。検査の直後に CPU を取り合うと frame が出ない間、ここで詰まる。DOM の要素は 1 つで動いておらず（rect は一定）、要素の不具合ではない | `test_ui.py` の `click_without_stable()`: `Locator.click` の既定を、`expect` の可視・有効（ポーリング、30 s）を待ってから `force=True` で押すものにした。既存の `.click(force=True)` が 92 か所あり同じ流儀。Chromium の起動引数（`--disable-renderer-backgrounding` など）は効かなかった（A/B 各 3 回で同じく落ちた） |
| 同じ「stable」で `table.refs tr` のクリックが 30 秒（fold だけ force にした 2 回目の実行） | fold を直した直後に出た | 同じ。fold だけの直しでは足りず、69 か所の既定のクリックがすべて対象 | 上の `click_without_stable()` で全部に効く |
| ホバー位置の RangeError（1 回報告） | 再現できていない【未確認】 | `modelicaHover` は `await` の間（setFile・hover）に本文が変わっても、返ってきた range で `doc.line(...)` と位置を決める。タブの切り替えや打鍵で本文が変わると、古い range が本文の外を指し、CodeMirror の tooltip の位置決めで RangeError になりうる。アプリ側の競合で、試験側の位置の取り方（行の長さを超える座標）ではない（`hover_over` は描画された文字の矩形の中心を取る） | `editor.ts`: 待つ前の `doc` を覚え、`view.state.doc !== doc` なら何も出さない（1 行） |
| `ui_polish` の `expect(running).to_have_count(1)`（行 465、補完の一覧を Esc で閉じた直後）に実行中の印が無い（自然な負荷 load 6〜18 で 5 回中 2 回、PID 名前空間で隔離した実行でも 1 回） | 再現した（上の 3 回とも同じ行） | App.svelte の Esc（capture）は `completionStatus(view.state) !== "active"` で「一覧が開いていない」と見て実行を止める。`Rea` を打った直後は候補の再問い合わせが応答待ち（pending）になり、実行中の Worker は答えないので pending のまま、前の候補は見えているのに status は pending（pending は active より先に返る）。CodeMirror 側は pending も閉じて Esc を食べるので、一覧を閉じつつ実行も止めてしまい、`[data-run=running]` が消える。応答のタイミング次第なので負荷で出方が変わる | `App.svelte`: 条件を `!currentCompletions(view.state).length`（見えている候補が無いとき）に替えた。見えていない pending は従来どおり数えず、Esc 1 回で実行が止まる |
| `render_slides.py` の `Page.screenshot` が 30 秒で時間切れ | cargo と同時に回したとき（既知。S304 でも出た） | 同じ CPU の取り合い | 時間切れのとき 1 回だけ撮り直す（`PlaywrightTimeout` を捕まえる）。引数は足さない |

## やったこと

- `src/Svelte/tests/test_ui.py`: `click_without_stable()` を足して `main()` の先頭で呼ぶ。
- `src/Svelte/src/lib/editor.ts`: ホバーの本文の変化への保険。
- `src/Svelte/src/App.svelte`: Esc で実行を止めるかの判定を、見えている候補の有無にした。
- `.claude/skills/session-docs/render_slides.py`: screenshot の再試行。
- `doc/sessions/S310-test-ui-stability/`、索引、この記録。

## 検証

RESULTS

## 残したこと

- `test_ui.py` の 2310 行（`PID_Controller` の検査の後の `expect(check_button).to_be_enabled(timeout=120_000)`）が、`click_without_stable()` を入れた実行の 5 回中 2 回で 120 秒たっても「検査」が disabled のまま落ちた。負荷が高い（load 12〜13、他の担当の cargo も走る）ときだけで、同じ場面だけを切り出して 9 回回すと毎回 6.5 秒で終わった。busy が戻らなかった原因は特定できていない【未確認】。メモリの圧迫（swap）か、worker の応答が失われた競合かのどちらか。
- 既定のクリックの hit target の待ち（上に重なる要素が消えるまで待つ）は無くなる。CI の `ui` ジョブで確かめる。

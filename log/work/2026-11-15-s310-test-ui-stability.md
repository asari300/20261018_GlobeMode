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
| 検索の候補を待たずに Enter（行 1973、`Examples.BusUsage`）。負荷 load 18 で 60 秒たってもモデル名が変わらない | 再現した（2 回） | 検索の結果は非同期で、候補が出る前の Enter は何も開かない。ほかの 4 か所は候補を待ってから Enter しているのに、ここだけ抜けていた | `test_ui.py`: 候補（`#library-hits [role=option]`）が出るのを `expect` で待ってから Enter |
| 実行の途中で `Target page, context or browser has been closed`（行 1612・570・1854・2310 など、場所はばらばら） | 負荷に関係なく 10〜40 % の実行で出た。PID 名前空間・専用の TMPDIR で隔離しても 1 回出た【隔離の効き方は未確認】 | **別の担当の `pkill -f '[c]hromium'` が、私の Playwright の Chromium も終了させていた**。`DEBUG=pw:browser` で Chromium が `exitCode=143`（SIGTERM）で終わった時刻が 13:39:48.8（JST）、同じ時刻の 0.5 秒前（04:39:48.324Z）に別の担当の記録に `pkill -f '[c]hromium' ; sleep 2; uv run python .claude/skills/session-docs/render_slides.py` がある。CLAUDE.md・MEMORY の「render_slides は cargo と同時に回さない」の片付けの手順が、同じ機械で動く他の Playwright の試験を巻き込む。`expect(...)` が disabled のまま 300 秒の途中で落ちるように見えるのは、最後に見た値を出すため | 直していない（試験側では防げない）。親への提案: 片付けは自分の `render_slides.py` の process group だけ（`setsid` で起動して pgid を kill、または自分の `--user-data-dir` を含むものに絞る）にして、`pkill -f '[c]hromium'` は使わない |
| `render_slides.py` の `Page.screenshot` が 30 秒で時間切れ | cargo と同時に回したとき（既知。S304 でも出た） | 同じ CPU の取り合い | 時間切れのとき 1 回だけ撮り直す（`PlaywrightTimeout` を捕まえる）。引数は足さない |

## やったこと

- `src/Svelte/tests/test_ui.py`: `click_without_stable()` を足して `main()` の先頭で呼ぶ。
- `src/Svelte/src/lib/editor.ts`: ホバーの本文の変化への保険。
- `src/Svelte/tests/test_ui.py`: 検索の候補を待ってから Enter（行 1973）。
- `src/Svelte/src/App.svelte`: Esc で実行を止めるかの判定を、見えている候補の有無にした。
- `.claude/skills/session-docs/render_slides.py`: screenshot の再試行。
- `doc/sessions/S310-test-ui-stability/`、索引、この記録。

## 検証

`test_ui.py` を、直したあとの dist（wasm は `target-shared/wasm-pkg/web` の 10:47 のもの）で回した。負荷は別の担当の cargo と、自分で足した CPU を使い切るループ（`while :; do :; done`）。

| 実行 | 条件 | 結果 |
| --- | --- | --- |
| 直す前（`test_ui.py` は main のまま） | load 4 → 11 | 1 回目から fold gutter で 30 秒の時間切れ。切り分け 3 回も毎回同じ |
| `click_without_stable()` のみ | load 14〜43、5 回 | 3 回通り、2 回は別の落ち方（Esc・検索の Enter・Chromium の終了） |
| 上に Esc の直しを足す | load 2〜18、5 回 ×4 組 | 通った 17 / 20。落ちた 3 回はすべて Chromium が外から終了した（pkill） |
| 最終版（Esc・検索の Enter・click を全部入れた） | load 3〜7、5 回 | **5 回連続で通った**（`rc=0` ×5） |
| 最終版 | load 15〜19（CPU を使い切るループ 12 本）、5 回 | 通った 3 回（1・2・5）。2 回は Chromium が外から終了した |
| CI（ci.yml full=true、feature ブランチ、run 43670200157） | WIP の 1 つ目のコミット | ジョブ `ui` が success（全体も success） |

最終版の CI は、親が取り込んだあとの main の ci.yml に任せる（Esc と検索の Enter の直しを入れたあとの feature での全体検査はまだ回していない。下の「残したこと」）。

- コードの検査: Rust は触っていない。`pnpm check`（svelte-check）: 284 ファイル、エラー 0・警告 0。
- `check_docs.py`: スライド全件 OK、リンク 4918 件中の切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件（1 回目は他の担当の `pkill` で Chromium が終了して失敗、撮り直しで通った）。markdownlint（v0.24.1）: MD013 を除いて 0 件。
- 事実の確認: Chromium が外から終了した時刻と別の担当の `pkill` の時刻の一致は、`~/.claude/projects/` の各担当の記録（`pkill -f '[c]hromium'` の時刻）で確かめた。

## 残したこと

- 手元の「Target page ... has been closed」は他の担当の `pkill -f '[c]hromium'` が原因で、試験側では防げない（上の表）。2310 行（`PID_Controller` の検査の後の 120 秒の待ち）で落ちた 2 回も、この終了で説明がつく。同じ場面だけを切り出して 9 回回すと毎回 6.5 秒で終わった。親に、片付けを自分のプロセスだけにする運用の変更を提案する。
- 既定のクリックの hit target の待ち（上に重なる要素が消えるまで待つ）は無くなる。CI の `ui` ジョブで確かめる。

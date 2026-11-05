# S170 `ui-result-visibility-open`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s170-ui-result-visibility-open` |
| worktree | `.git-worktree/s170-ui-result-visibility-open` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S170-ui-result-visibility-open/` |
| セッション記録 | （親が写す） |

## 依頼

S164 の監査の穴 2（結果と進捗が画面の下にある）と穴 5（開く・モデル名の扱い）を Svelte 版 UI で埋める。S166・S167 が main に入ってから（`7fe79e5` の上）。ポニーテール（最小の完全な変更）で。

## 決めたこと

- 穴 2: 進捗・失敗・食い違いの案内をボタンの行の下へ移し、終わったとき 1 回だけ `scrollIntoView({ block: "nearest" })`（失敗、無ければプロット、無ければ診断）。即時（滑らかにしない）。パラメータの一覧は閉じて始める。
- 穴 5: モデルの候補は Rust の outline から（`definition::models`、wasm `models`、LSP `globe/models`、`Backend.models`）。2 つ以上で select。モデル名はタブごとに保存。未保存の編集があるタブと同名のファイルを開くときは差し替えず切り替える（確認ダイアログは出さない。差し替えなければ失わない）。打ち替えの食い違いは、ライブラリにある名前なら案内とボタン、無ければ今までの診断。
- 滑らかなスクロールにしなかった理由: 試験の click が動いている間にずれた（`boxes.first.check` が落ちた）。reduced-motion も満たす。
- 既定のまま進めたこと: 自動で食い違いのファイルを開くことはしない（smoke の流れと、勝手にタブが増える挙動を避ける）。

## やったこと

- Rust: `globe-mode-lsp/src/definition.rs` に `models`、`lib.rs` に `globe/models`（と表の行）、`globe-mode-wasm/src/lib.rs` に `models`、`web/client.js`・`web/worker.js` に口。単体 2 本（wasm の `models_lists_the_runnable_classes_of_a_file`、LSP の `models_over_rpc`）。
- Svelte: `App.svelte`（候補・select・案内・保存・開き直し・移動・スクロール）、`SimSettings.svelte`（一覧を閉じて始める）、`backend.ts`・`worker-backend.ts`・`tauri-backend.ts`（`models`）。
- 試験: `test_ui.py` に 2 本、一覧の開閉 2 行。
- 文書: 解説書・スライド・索引・roadmap・features。

## 検証

- wasm の出所: main `7fe79e5`（S166・S167 の merge 済み）の上のこの worktree（未コミット時に release ビルド。`CARGO_PROFILE_RELEASE_*` は `wasm:build` と同じ）。出力先は共有の `wasm-pkg` でなく自分の場所に作り、`pnpm build` の `CARGO_TARGET_DIR` に渡した。
- `test_ui.py`: ok（MSL.json あり、新しい 2 本を含む）。`pnpm check`: 0 errors / 0 warnings。
- `wasm:node` 相当（`smoke.mjs --msl data/msl`）: ok。
- 位置（Cauer、1280 x 800）: プロットの上端 1019 から 855（ページ）、自動スクロール後は画面の中の 290。390 幅は 1805 から 1680、画面の中の 304。実行中の表示 723（S164 では 809）から 96。
- GM-SMOKE の行: 項目は増やしていない。`ready_ms` 346〜404（前）と 347〜432（後）、`decay_ms` 194〜223（前）と 205〜270（後）。`bench_ui.py` は `target/` に書くので回さず、同じ GM-SMOKE の行を前後 3〜6 回ずつ取って比べた。
- `mise run check`: 通った（fmt は 1 回目に落ちて `cargo fmt` で直し、2 回目で fmt・clippy・test 664 本 ok / 0 failed・deny（advisories・bans・licenses・sources ok）・`uv sync --locked`）
- `check_docs.py`: デッキ S170 OK / リンク 3572 件中の切れ 0 件。`render_slides.py`（S170 のデッキだけ）: 1 回目で問題 0 件（8 枚、ライト・ダーク・スマートフォン。S05 の表を目で確認し余白あり）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークンの grep）

## 残したこと

- 未対応の例題の見せ方と force、キーボード全般、`.txt` を黙って開く件。
- 候補は打つたびには更新しない（検査・実行・開く・タブ切り替えのとき）。
- Desktop の sidecar の `globe/models` は単体（LSP）で確かめただけで、Tauri の画面では回していない（手元で起動できない。desktop.yml の smoke は GM-SMOKE の行の項目が変わらないので影響を受けない）。

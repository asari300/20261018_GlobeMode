# S360 `ci-windows-call-hierarchy`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s360-ci-windows-call-hierarchy` |
| worktree | `.git-worktree/s360-ci-windows-call-hierarchy` |
| Phase | P4（試験の安定）・Track B の製品へのつなぎ（S354 の続き） |
| セッション文書 | `doc/sessions/S360-ci-windows-call-hierarchy/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

main の CI（run 43727293319、S353 のマージ `ce38ea4f`）が 2 つのジョブで落ちた。(1) `rust-windows`: `globe-mode-lsp/tests/call_hierarchy.rs:75` の `incomingCalls` が空。(2) `ui`: `test_ui.py` 2371 行（タブを戻したあと `table.diagnostics tr` が 0 行）。根の原因を 1 か所で直し、各修正に回帰試験を足す。試験を Windows で緩めない。UI は CI で再現するか確かめ、再現しなければ「再現せず」と run 番号を書く。

## 決めたこと

- **Windows**: `Target` の `==` を `same_path_key` での比較にした（`definition.rs`）。呼び元ごとに正規化すると `class_hits` を通る別の経路（`references`・`rename`・部品）が取り残されるので、`==` 1 か所。`derive(PartialEq)` をやめて手書きにした（`Eq` は空の impl）。
- **UI**: CI では再現しなかったが、手元の負荷で再現し、原因を押下の mousedown の対象で確かめた。直しは、S310 の `click_without_stable()` に「押した点が要素の外なら押し直す」を足すこと。Playwright の既定（`force` なし）に戻す案は、この機械では空いたページでも rAF が出ず、1 回の押下ごとに 3 秒詰まるので採らなかった（S310 の目的を保つ）。
- UI の layout shift（`SimSettings` の高さが検査の終わり・タブの切り替えのあと動く）は直さない。依頼は CI の失敗で、見た目の直しは別の WP。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/definition.rs`: `Target` に手書きの `PartialEq`（`crate::same_path_key` で比べる）。
- `src/Rust/mod/globe-mode-lsp/src/call_hierarchy.rs`: 単体試験 `calls_are_found_when_tree_path_spells_backslashes`（木のパス `C:\lib\Fns.mo`、項目のパス `C:/lib/Fns.mo`。incoming は `[("M", 2)]`、outgoing は `[("f", 2)]`）。
- `src/Svelte/tests/test_ui.py`: `click_without_stable()` に押し直し、`check_click_survives_shift(page)`（`main()` の冒頭）を追加。
- `CHANGELOG.md`・`doc/adr/0039-native-cst-parser.md`（付記 S354 に S360 の修正）・`doc/sessions/S360-…/`・索引・この記録。

## 原因の調べ方（要点）

| 項目 | 内容 |
| --- | --- |
| Windows | `class_of` は `end 名前`・`class 名前` の上で `Target { path: path.to_string() }`（呼び手の綴り）を返し、型名の解決は `Resolver::locate` がライブラリの木の綴りを返す。`LibraryTree` は `display()` のまま持つ（`GlobeDb` は `\` を `/` にするが木は変えない）。incoming の項目の URI は `file_uri` が `\` を `/` にするので、戻したパスは `C:/…`、木は `C:\…`。Linux では同じ綴りなので通っていた。木のパスを `C:\lib\Fns.mo` と綴った単体試験が、Linux でも直す前は `[]` で落ちる（手元で確かめた）。Windows の CI で通ったのは直した後（run 43730056976 の `rust-windows`） |
| UI の再現 | 検査 → タブ 1 → タブ 0 を新しいページで繰り返し、押した mousedown の対象を記録した（`tabs_repro.py`。負荷は CPU を使い切るループ 8〜12 本）。直す前は 60 回中 2 回、`[BUTTON[tab] (69,398), …, BUTTON[tab] (176,351), P.note (69,372)]` で、タブ 0 の押下が `p.note` に当たり、選択はタブ 1 のまま、表は 0 行。12 本のときは別の 1 回、タブ 1 の押下が外れて本文が Decay のままだった |
| 配置のずれ | タブ列の上端を 5 ms ごとに記録した: 検査が終わった直後に 388 → 341 → 356 px（62 ms 以内に 2 回）、タブ 1 を押した 46 ms 後に +6 px |
| rAF | この機械では空いたページ（`set_content`・http の静的なページ・アプリのページ）で `requestAnimationFrame` が 1.5 秒たっても出ない（負荷 8 本のときは出ていた）。Playwright の既定の押下は「stable」の rAF で詰まる（S310 の指摘どおり）。押下の確認を rAF に頼れない |

## 検証

- `cargo test -p globe-mode-lsp`: 全部通った。`cargo clippy -p globe-mode-lsp --all-targets -- -D warnings`・`cargo fmt` 0 件。
- Windows の CI: `ci.yml -f full=true`（feature の `a119e384`、run 43730056976）は全ジョブ success（`rust-windows` の `cargo test --workspace` に `call_hierarchy` の統合試験を含む）。最後の版の CI は下の「残したこと」の run。
- UI の CI での再現: run 43730056976（feature、full）と run 43729347424（main の S356 のマージ）の `ui` はどちらも success。**CI では再現せず**。
- UI の手元の負荷（CPU を使い切るループ 8〜12 本）: 直す前の版 60 回中 2 回落ちた。Playwright の既定を 3 秒試す版（採らなかった）は 80 回と 100 回（12 本）で 0 回。押し直す最終版は **150 回で 0 回、そのうち 4 回で押し直しが働いた**（押下が 5 回のところ 6 回）。直す前の版ならこの 4 回が落ちていた。
- 回帰試験 `check_click_survives_shift`: S310 の版は 0 回で落ち、押し直す版は 1 回で通る（毎回 0.4 秒）。
- `test_ui.py` を手元で全部回した（`data/msl` 無し、load 15 前後）: 冒頭から smoke の手前まで通った。押し直しの表示は回帰試験の 1 回だけで、ほかの押下は 1 回で要素に当たった。smoke の `msl=ng`（`MSL.json を取得できない（404）`）は `data/msl` が無いための手元の都合で、MSL の試験は CI に任せる。
- `check_docs.py`: スライド全件 OK、リンク 5353 件中の切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件。markdownlint-cli2（MD013・MD033 を除く）: 5 ファイルで 0 件。
- `cargo test -p globe-mode-lsp`: lib 76 本と ignored 1 本、統合 call_hierarchy 1・cross_file 5・folding 2・tolerant_outline 2 がすべて通った。

## 残したこと

- UI の layout shift（`SimSettings` の高さが検査の終わり・タブの切り替えのあと 6〜47 px 動く）は直していない。試験は押し直しで吸収したが、利用者が押す直前にずれると別の場所を押す。
- 最後の版の CI（`ci.yml -f full=true`）の run 番号と結果は、親への報告に書く（このコミットの時点では未確認）。
- ドライブ文字が小文字の URI（VS Code）での実機の call hierarchy は【未確認】（`same_path_key` が畳むことは既存の単体試験で確かめている）。
- 手元の負荷試験の再現用スクリプトはコミットしていない（scratchpad に置いた。作り方は解説書の「再現手順」）。

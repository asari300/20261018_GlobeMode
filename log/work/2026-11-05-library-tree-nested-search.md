# S167 `library-tree-nested-search`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s167-library-tree-nested-search` |
| worktree | `.git-worktree/s167-library-tree-nested-search` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S167-library-tree-nested-search/` |
| セッション記録 | （親が写す） |

## 依頼

S164 の監査の穴 1（N1）。ライブラリの木に、1 つのファイルの中で定義された入れ子のクラスが出ない（`Modelica.Blocks.Examples` の 32 本、`Blocks.Sources.Sine` など。例題 553 本のうち約 123 本が木に出ない）ことと、検索が無いことを埋める。展開で直下の子だけを引く作りを保つ。モデルを押すとそのファイルを開き、完全な名前をモデル名に入れる。検索は名前の部分一致（大文字小文字を区別しない）で、一致の計算は Rust、キーボードだけで開ける。

## 決めたこと

- 入れ子の取り方: `globe-mode-fmt` の `LibraryTree::new` が、各ファイルを `lex` と `structure::split`（整形器の文の切り出し）にかけ、`class_defs` でクラス定義を全部（短いクラス `type X = Real;` も）索引に入れる。Rumoca の構文解析（`Outline`）は使わない。理由は、検索が入れ子のクラスも引くには全ファイルの分が要り、MSL 全体を Rumoca で解析するのは重い一方、字句解析だけなら 15 MB で約 90 ms（ネイティブ）で済むため。展開は索引の直下の子を引くだけで、今までと変わらない。依頼文の選択肢のうち「MSL を読むときに作る軽い索引（既存の字句解析で）」の側を選んだ。
- 正しさの確かめ: MSL 4.1.0 の 2,552 ファイル全部で `globe outline --json`（Rumoca）のクラスの一覧と突き合わせた。最初は 1,793 件の差（短いクラスを入れていなかった）、短いクラスを入れて 1,071 件の差（原因は下）、直して差 0 件（6,487 件が過不足なく一致）。
- 整形器の誤り: `structure::split` の `model extends X(修飾)` の読みが、修飾の閉じ括弧を `scan_to` で探していて、`scan_to` は括弧の深さが負になると断るので、入れ子の括弧の修飾は必ず「閉じ括弧が多い」になった。括弧の対を読み飛ばす `skip_group` に置き換えた（Media の `BaseProperties(p(…), …)` の形）。整形器の振る舞いが変わるのはこの形のファイルだけ（今まで整形できなかった）。
- 子を持つ package 以外のクラス（関数を持つ block など）は、展開の ▸ に加えて「開く」ボタン。そうしないと開けなくなる。
- 検索: `LibraryTree::search(query, limit)`。ASCII の大文字小文字を区別しない部分一致。`.` を含めると完全修飾名に当てる。名前の完全一致、前方一致、部分一致の順、同じ中ではパスが短い順、名前の順。最大 50 件。口は wasm `library_search`、LSP `globe/librarySearch`、`Backend.librarySearch`。Desktop の Rust（rpc）は method をそのまま通すので変えていない。
- 項目の JSON の組み立て: wasm と LSP で同じ形なので `globe_mode_lsp::tree_entries_json` に 1 つ置き、wasm も使う。
- `lib.rs`（LSP）への追記は `collect_mo` の手前と `globe/libraryTree` の隣だけ（S166 との衝突を小さくするため）。`definition.rs`・`editor.ts`・`App.svelte` は触っていない。
- 開いたクラスの行へエディタを動かすことはしない（索引にファイルの中の位置が無い）。持ち越し。

## やったこと

- `src/Rust/mod/globe-mode-fmt/src/structure.rs`: `class_defs`、`skip_group`、`scan_to` の判定順（安い判定を先に）。
- `src/Rust/mod/globe-mode-fmt/src/tree.rs`: 入れ子の取り込み、`search`、`entry`、試験 3 本。
- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `tree_entries_json`、`globe/librarySearch`、試験。
- `src/Rust/mod/globe-mode-wasm/`: `tree_search`・`library_search`、`web/worker.js`・`web/client.js`、`tests/decay.rs`、`node/smoke.mjs`（MSL の木と検索の確認）。
- `src/Svelte/src/lib/`: `backend.ts`・`worker-backend.ts`・`tauri-backend.ts`（`librarySearch`）、`LibraryTree.svelte`（検索欄と「開く」ボタン）。`src/Svelte/tests/test_ui.py`（検索 PID_Con と木）。
- 文書: スライド、解説書、索引、`features.md`・`roadmap.md`。

## 検証

- コードの検査: `mise run check`（fmt / clippy -D warnings / test / deny / `uv sync --locked`）が通った。テスト 659 本 ok、0 件失敗、cargo deny は advisories・bans・licenses・sources とも ok。最初の clippy で `type_complexity` が出たので `type Class` を置いた。
- `mise run wasm:node`: ok（33 s）。`smoke.mjs` が MSL の `Blocks` の子に `Examples` があること、`PID_Con` の先頭が `Modelica.Blocks.Examples.PID_Controller` であること、`modelica.blocks.sources.sine` が `Sine` に当たることまで確かめる。`mise run wasm:compare` は回していない（実行系・ソルバは触っておらず、木と検索の口だけを足したため）。
- `pnpm check`: 0 errors / 0 warnings。`pnpm build`: ok。
- `test_ui.py`（MSL あり、main の `data/msl` を symlink）: 通った。検索 `PID_Con` は候補まで 0.03〜0.3 s、Enter で `PID_Controller` が開いて検査 error 0 行。木から `Blocks` → `Examples` → `PID_Controller`、検索 `sources.sine` → `Blocks.Sources.Sine` も開いた。`GM-SMOKE` の行は `tree=ok` のまま項目の追加なし。1 回目は `block Sine` の文字列が DOM に無いのを期待して落ちた（CodeMirror は見える範囲だけを描く）ので、`package Sources` に直して通った。
- MSL 4.1.0 の 2,552 ファイルで、木の索引（6,487 件の入れ子のクラス）と `globe outline --json` の一覧が過不足 0 件（短いクラスを入れる前は差 1,793 件、`extends` つきクラスの括弧を直す前は差 1,071 件）。
- 木に出ない例題（553 本、partial を除く）: 変更前 123 本（S164 の推定どおり。祖先が同じファイルにない本数で再計算）、変更後 0 本。
- 索引の時間・メモリ・検索の応答は[解説書](../../doc/sessions/S167-library-tree-nested-search/解説書.md)の表。ネイティブ 79〜94 ms、wasm で MSL を読む時間が +100〜150 ms、検索 2〜8 ms。
- `check_docs.py`: デッキ OK / リンク 3,333 件中の切れ 0 件。`render_slides.py`（S167 のデッキだけ）: 8 枚 × 3 構成、問題 0 件。
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）。

## 残したこと

- 開いたクラスの行へエディタを動かす、`package.order` の順、アイコン、検索結果の説明。
- 検索は ASCII の大文字小文字だけを区別しない（非 ASCII の識別子は大文字小文字を区別する）。
- 索引は読み込み時にすべてのファイルを字句解析する。ライブラリが MSL より大きいとき（Buildings など）は、読み込みの時間とメモリを測り直す。

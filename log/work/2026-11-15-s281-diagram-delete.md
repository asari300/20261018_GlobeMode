# S281 `diagram-delete`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s281-diagram-delete` |
| worktree | `.git-worktree/s281-diagram-delete` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S281-diagram-delete/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

図の上で部品と結線を消せるようにする（Delete キーと選び）。Rust の `delete_part`（宣言とつながる `connect` をまとめて）・`delete_connect`、LSP・wasm・Backend、Svelte の DiagramView。VS Code のパネルは今回やらない。

## 決めたこと

- `integrate-1115`（S264・S265 を含む）が main に無かったので、`feature/integrate-1115` から切った。`RPC_METHODS` は 31 → 33。
- 複数の名前の宣言（`A a, b;`）は、`annotation` の帰属が曖昧なので名前だけ外す編集は入れず、断る。
- 結線の指し方は `Connection` の `from`・`to`（`connect_ends` が添字を落とした字句）。逆の並びも同じ、同じ端が複数あれば全部消す。配列のコネクタの結線の選び分けは持ち越し。
- 結線の当たり判定は透明な太い `polyline.hit` を `[data-conn]` の外に置く（`test_ui.py` の `[data-conn] polyline` の数を変えない）。
- 部品のクリックで宣言へ飛ぶとフォーカスがエディタへ移り、Delete がエディタの文字を消すので、飛んだ後で選んだ部品・結線へフォーカスを戻す。

## やったこと

- `globe-mode-fmt/src/diagram.rs`: `delete_part`・`delete_connect`・`erase`（行ごとか要素だけか）と試験 1 本。
- `globe-mode-lsp`: `diagram::delete_part`・`delete_connect`、`globe/deletePart`・`globe/deleteConnect`、試験 1 本、README の表。
- `globe-mode-wasm`: `delete_part`・`delete_connect`、worker.js・client.js、decay.rs に 1 件の追加、smoke.mjs。
- Desktop の `RPC_METHODS` 33。Svelte: `backend.ts`・`tauri-backend.ts`・`worker-backend.ts`・`DiagramView.svelte`・`DocsPanel.svelte`・`App.svelte`・`shape-svg.ts`・`test_tauri.mjs`。
- 文書: ADR-0037 付記、roadmap・features、索引、スライドと解説書。

## 検証

- `cargo test -p globe-mode-fmt --lib diagram` 8 本 pass、`cargo test -p globe-mode-lsp --lib delete_part` 1 本 pass、`cargo test -p globe-mode-wasm --test decay diagram_is_read` 1 本 pass。
- `pnpm check`（Svelte）0 エラー 0 警告、`node tests/test_tauri.mjs` ok、VS Code の `node --test test/panel.test.mjs test/diagram-edit.test.mjs` 7 本 pass。
- ブラウザでの操作（選び・Delete）は確かめていない（wasm とバンドルを作っていない）。

## 残したこと

VS Code のパネルでの削除、複数選び、配列のコネクタの結線の選び分け、宣言の複数の名前から 1 つだけ外す、消した部品を参照する他の文の整理、`test_ui.py` での操作の試験。

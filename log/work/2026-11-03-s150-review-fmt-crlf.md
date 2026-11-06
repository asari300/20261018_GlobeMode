# S150 調査台帳: fmt lib → lexer の CRLF 経路

- 2026-11-03、revision `e73af106c8b7b220a09e162d69798abfae0efcad`、`feature/s150-review-fmt-crlf`。地図 [PR #18](https://github.com/asari300/20261018_GlobeMode/pull/18) は別ブランチの過去時点であり、ここに今回の台帳を残す。
- `src/Rust/mod/globe-mode-fmt/src/lib.rs:91–115, 145–185`: Options と入力から再字句化・比較までの CRLF 経路を部分確認。ファイル全体は未監査。
- `src/Rust/mod/globe-mode-fmt/src/lexer.rs:53–82, 140–148`: 空白・行／ブロックコメントの分岐とトークン切り出しを部分確認。他の字句分岐は未監査。
- `src/Rust/mod/globe-mode-fmt/src/layout.rs:109–124, 250–264`: 期待トークンとテキストの出力を原因追跡のため部分確認。別の DFS 枝の監査完了を意味しない。
- 再現コマンド・結果は [review_en.md](../review/2026-11-03-review-fmt-crlf/review_en.md)。指定 CRLF ブロック Err byte 11、LF ブロックと CRLF 行コメント Ok。仮コピーの出力ログで `\r\r\n` を確認。既定機能 rustc 単体テスト 7 passed / 0 failed。[Issue #19](https://github.com/asari300/20261018_GlobeMode/issues/19) を readback。
- 未検証: Cargo の依存解決・build script・config feature・統合試験・CLI・wasm・CI。origin 自動追加は仕様で対象外。次の DFS 枝へ進まず、他ファイルの完了は主張しない。製品コード変更なし。

# 整形器の CRLF 調査

確認日 2026-11-03、基準は取得直後の `origin/main` `e73af106c8b7b220a09e162d69798abfae0efcad`。問題は [Issue #19](https://github.com/asari300/20261018_GlobeMode/issues/19) に登録し、本文を読み戻した。

## 調査範囲

DFS の `globe-mode-fmt/src/lib.rs` → `lexer.rs` のうち、**CRLF コメントのトークン保持経路のみ**。出力の追跡に `layout.rs` を部分的に参照した。ファイル全体や crate 全体の監査完了は主張しない。正確な範囲と実行記録は[調査台帳](../../../log/work/2026-11-03-s150-review-fmt-crlf.md)。次の枝には進んでいない。`origin` 追加は仕様として対象外。

## 結果と根拠

指定の CRLF 複数行ブロックコメントでは `Err`（2 番目のトークン、byte 11）。同じブロックの LF 版と CRLF 行コメント版は `Ok`。期待はコメント内容を保ち整形できること。借用入力は変更されず、整形結果は返らない。ディスク破損は観測していない。

[lexer.rs:69–82](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lexer.rs#L69-L82) はブロックコメント内部の CRLF をトークンに含め、[layout.rs:259](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/layout.rs#L259) はそのテキストを出す。[lib.rs:145–161](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lib.rs#L145-L161) で LF を CRLF に置換する際、内部の `\r\n` が `\r\r\n` に変わる。[lib.rs:166–185](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lib.rs#L166-L185) の再字句化と比較が拒否する。製品コードではなく一時コピーへのログ挿入で中間出力 `/* first\r\r\n  second */` を確認した。

## 再現

[repro.rs](repro.rs) の入力と `insert_origin: false`, `format_html: false` を使う。実行コマンドと出力全文は [英語記録](review_en.md)。rustc 1.99.1 で直接ビルドして指定例の Err と 2 対照の Ok を確認、既定機能の単体テストは 7 passed / 0 failed。

## 制限

直接 rustc は Cargo の依存解決、workspace build script、任意 `config`、CLI、wasm、結合テスト、CI を検証しない。対照例は網羅的でない。製品コード修正なし。公開前に open / closed の Issue・PR を検索し、重複なし。[PR #17](https://github.com/asari300/20261018_GlobeMode/pull/17) は別の panic、[PR #18](https://github.com/asari300/20261018_GlobeMode/pull/18) は過去時点の調査地図。

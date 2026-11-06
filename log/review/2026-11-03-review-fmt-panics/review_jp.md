# 整形器の panic 修正：レビューと再現

| 項目 | 内容 |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s137-review-fmt-panics` |
| 基準 | `5726edf62f6c` とコード・テストの変更計 3 ファイル |
| 成果物 | [English](review_en.md)、[スライド](review.html)、[再現](repro.rs) |

## 目的

既存の検証済み修正 2 件の資料整備であり、新たな監査ではない。整形仕様は [format-style](../../../doc/format-style.md) を参照。

## 決定と理由

1. `!é` は従来 2 バイト演算子を調べる際に UTF-8 文字の途中で文字列を切り panic した。バイト列の接頭辞比較にし、先頭バイト位置 0 の `FormatError` を返す。
2. 2 出力の関数呼び出し `(, x) := f();` を幅 5 で整形すると、先頭の空要素で非空の assertion に達し panic した。空ノードを平坦化の経路に渡さず折り返す。再現では該当トークンの保持と冪等性を確認する。

## 図

```mermaid
flowchart LR
  A[入力] --> B[字句解析]
  B --> C[構造とレイアウト]
  C --> D[出力を再検査]
  B --> E[エラーを返す]
```

## 再現手順

リポジトリのルートで、修正を含む状態で実行する。Git、tar、edition 2024 対応の `rustc` と C リンカが必要。最新 main の基準版は Git のオブジェクトから一時ディレクトリに展開し、worktree は上書きしない。旧版の 2 件は別プロセスで実行する。

```sh
set -eu
T=$(mktemp -d)
BASE=e73af106c8b7b220a09e162d69798abfae0efcad
CRATE=src/Rust/mod/globe-mode-fmt
REPRO=log/review/2026-11-03-review-fmt-panics/repro.rs
# Before: extract only the committed source, without changing the worktree.
git archive "$BASE" "$CRATE/src" | tar -x -C "$T"
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib "$T/$CRATE/src/lib.rs" -o "$T/libglobe_mode_fmt.rlib"
rustc --edition=2024 "$REPRO" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/repro"
set +e
"$T/repro" unicode; printf 'before unicode exit=%s\n' "$?"
"$T/repro" tuple; printf 'before tuple exit=%s\n' "$?"
set -e
# After: compile the modified worktree source.
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib "$CRATE/src/lib.rs" -o "$T/libglobe_mode_fmt.rlib"
rustc --edition=2024 "$REPRO" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/repro"
"$T/repro" unicode
"$T/repro" tuple
# Default-feature unit and integration tests without Cargo.
rustc --edition=2024 --test --crate-name globe_mode_fmt "$CRATE/src/lib.rs" -o "$T/unit"
"$T/unit"
CARGO_MANIFEST_DIR="$PWD/$CRATE" rustc --edition=2024 --test "$CRATE/tests/format.rs" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/format-tests"
"$T/format-tests"
rm -rf "$T"
```

最新 main `e73af106` を基準とした移設・マージ後の実測（`rustc 1.99.1 (58cf8f9c3 2026-10-13)`）:

```text
before unicode exit=101  (lexer: end byte index 2 is not a char boundary)
before tuple exit=101    (layout: 引数は空でない)
unicode: error at byte 0 (no panic)
tuple: formatted and idempotent (no panic)
test result: ok. 7 passed; 0 failed
test result: ok. 11 passed; 0 failed
```

修正前は両例とも panic で終了コード 101、修正後は両例とも終了コード 0。親・独立レビュー側でも 18 テスト成功との報告があり、上記の移設後再実行（単体 7 + 結合 11）とは区別する。これは Cargo を使わない既定機能の直接 `rustc` 試験であり、Cargo、追加 feature、clippy、CI や workspace 全体は未検証。再現は Modelica 全体の意味検証を保証しない。

## 次回

全体 Mermaid は S136 で別途作成済み。Cargo、CI、HTML のブラウザ表示は未検証。

# S152 整形器 structure 監査

基準: worktree 作成時の `origin/main`、`e73af106c8b7b220a09e162d69798abfae0efcad`。ブランチ `feature/s152-review-fmt-structure`。独立再実行・仕様照合済み。

## 実証した不具合

### 1. [Issue #20](https://github.com/asari300/20261018_GlobeMode/issues/20): 正常な `redeclare model extends M(...)` を拒否

`src/Rust/mod/globe-mode-fmt/src/structure.rs:127-145` は開き括弧の次から `scan_to` を呼ぶ。`:243-255` は閉じ括弧で停止条件を見る前に深さを減らすため、`M(x = 2)` の `)` を余分な閉じ括弧と誤認する。`repro.rs` は継承された `replaceable model M` と再宣言を含む完全な package の組を用いる。括弧なしの対照は `Ok`、修飾付きは `Err(FormatError { message: "閉じ括弧が多い", offset: 145 })`。期待は整形成功。Modelica 3.6 §7.3.1 は `redeclare class extends` に任意の class modification を認める（https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html、2026-11-03 確認）。仕様は継承された replaceable class に対する任意修飾を認め、`long-class-specifier` も `extends IDENT [class-modification] … end IDENT` と定める。この例は Base から M を継承し修飾する。omc と取得済み Rumoca CLI は見つからず、独立コンパイラによる具体例の意味検査は【未確認】。

### 2. [Issue #21](https://github.com/asari300/20261018_GlobeMode/issues/21): 異なるブロック終端を一致と扱う

`structure.rs:18-21,86-105,114-116` のスタックはブロック種を記録せず、どの `BLOCK_END` でも閉じる。再現では正常な `if ... end if;` が成功し、不正な `if ... end for;` も `Ok` で整形本文を返す。期待は文書を変更せず `Err`：`src/Rust/mod/globe-mode-fmt/src/lib.rs:144-147` と `doc/format-style.md:23` は不一致の `end` を拒否すると明記。完全な構文検証を契約している根拠は未確認だが、この個別の不一致 end は明示的な拒否条件に当たるため採用した。実害として、不正な本文が整形成功として返ることを確認した。Modelica コンパイラによる受理は主張しない。

## 確認範囲・実行結果

- DFS: `src/Rust/mod/globe-mode-fmt/src/lib.rs:144-187` → `structure.rs:1-341` を静的に追い、クラス・短縮クラス・節・ブロックの境界とエラー処理を確認。実行例は限定的で網羅ではない。他モジュールの監査はしていない。
- リポジトリルートで以下を実行:

```sh
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib src/Rust/mod/globe-mode-fmt/src/lib.rs -o /tmp/libs152_fmt.rlib
rustc --edition=2024 log/review/2026-11-03-review-fmt-structure/repro.rs --extern globe_mode_fmt=/tmp/libs152_fmt.rlib -o /tmp/s152_repro
/tmp/s152_repro
rustc --edition=2024 --test --crate-name globe_mode_fmt src/Rust/mod/globe-mode-fmt/src/lib.rs -o /tmp/s152_fmt_tests
/tmp/s152_fmt_tests
```

再現は対照2件 `Ok`、修飾付き `Err` byte 145、不一致ブロック `Ok`、assert 全通過。直接 rustc の既定 feature テスト 7 件成功。`cargo test -p globe-mode-fmt --offline` は workspace の git 依存 `rumoca` revision `276e82bd` が offline に無くビルド前に失敗。Cargo の feature 解決・統合テスト・config feature・workspace 全体は未検証。

- 既存 Issue #19 の CRLF、PR #17 の2件の panic、仕様である origin 補完は重複報告しない。`gh` で全状態の Issue と PR を各上限200件取得し、該当タイトルなし。重複なしとして Issue #20・#21 を作成し、本文と open 状態を readback した。製品コードは変更していない。
- 指定範囲外の新しい DFS 枝には進んでいない。

文書検査: `check_docs.py` は相対リンク 3005 件・切れ 0 件。Playwright Chromium の `render_slides.py` は desktop light / dark / mobile 各 4 枚、問題 0 件。

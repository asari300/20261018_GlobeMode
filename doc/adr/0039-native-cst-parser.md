# ADR-0039 Track B の B1 は独立した crate `globe-mode-syntax` に置き、rowan の CST と寛容なパーサを自前で書く

- 状態: Accepted（2026-11-15、S268）
- 関係: [ADR-0002](0002-rumoca-adapter.md)（Track B = 自前コア）、[roadmap.md](../roadmap.md) の Track B、[architecture.md](../architecture.md) の crate 表、[ADR-0024](0024-own-formatter-and-lint.md)（`globe-mode-fmt` の字句解析）

## 背景

Track B（自前コア）は B1 rowan の CST と寛容なパーサ → B2 resolve → B3 flatten → B4 DAE の順で、`Frontend` の feature `native` として作る計画だった（ADR-0002）。今の IDE の機能（アウトライン、補完、リネーム、整形）は、Rumoca の AST が構文の誤りで落ちるところを `globe-mode-fmt` の字句解析と括弧の木で補っている。B1 はその土台になる、空白とコメントを落とさず、誤りがあっても木を返すパーサである。

## 決定

1. **新しい crate `globe-mode-syntax`**（依存は rowan 0.17 だけ）。`globe-mode-frontend` の feature `native` にはまだつなげない（B2 以降と一緒に決める）。Rumoca にも依存しない（wasm32 でも Track B の他の層からもそのまま使える）。
2. **字句解析は自前**（`lex`）。Modelica 3.6 の予約語 59 語・識別子（`'…'` を含む）・数・文字列・演算子（`.+` `.*` `:=` など）。空白・`//`・`/* */` もトークンで、**全バイトがどれかのトークンに入る**（知らない文字は `ErrorToken`、閉じていない文字列とコメントは診断）。`globe-mode-fmt` の `lexer` は整形用の別物で、置き換えない（依存 0 のまま）。
3. **パーサは「誤りがあっても木を作る」**。panic しない。読めなかったところは `Error` 節点に入れて `ParseError { range, message }` を残す。木の文字列は元の文字列と**必ず**一致する（試験で全ファイル・途中で切った入力を確かめる）。入れ子（クラス・if/for/when）は 128 段で打ち切り、深い入力でスタックを溢れさせない。
4. **最初の一切れの文法**: `within`、クラスの定義（`class` `model` `block` `connector` `type` `package` `record` `function` `operator` と `partial` `encapsulated` `expandable` `pure` `impure` `final` `replaceable` など。short class と `class extends`）、`extends`、`import`（別名・`.*`・`.{…}`）、部品の宣言（型名・配列の添字・宣言ごとの修飾・条件・説明文字列・`annotation`・`constrainedby`）、`public` / `protected`、`equation` / `algorithm`（`initial` 付きも）、`external`、クラスの `annotation`。
5. **式・修飾・注釈・文の中身は不透明**。括弧の対応（`( ) [ ] { }`。`[` の中の `;` は行列の区切りとして許す）と、`;` までだけ見る。`if` / `for` / `when` / `while` の文は `end if;` などまでを入れ子の `Equation` / `Statement` 節点にする（`if` 式の `then` を数えて条件の終わりを決める）。式の文法（優先順位、`Expr` の木）は次回。
6. **節点の種類は粗く始める**（`ClassDef`・`ShortBody`・`ExtendsClause`・`ImportClause`・`ComponentClause`・`Declaration`・`Modification`・`Condition`・`Comment`・`Annotation`・`EquationSection`・`AlgorithmSection`・`Equation`・`Statement`・`Expr`・`Error` ほか）。型付きの AST の薄い層は B2 の resolve を書くときに足す。

## 結果

- 試験: `src/Modelica` の全 75 ファイルが誤り 0・往復一致、各ファイルを 12 か所で切っても木ができる、`globe-mode-frontend/tests/data` の誤ったファイルで panic せず `Error` 節点が出る（`cargo test -p globe-mode-syntax`）。
- 計測（release、`cargo test --release -p globe-mode-syntax -- --ignored --nocapture`、`GLOBE_MSL` か `data/msl`）: MSL 4.1.0 の `.mo` 2556 本が誤り 0・往復一致・panic 0（0.4 秒）。MSL 4.0.0 は 2512 本が同じ。`data/libs` の 10627 本は往復一致・panic 0 で、誤りが出た 2 本は Buildings と AixLib の実際の構文の誤り（`end for` の後ろの `;` 抜け。診断が出るのは正しい）。
- 良い点: 文法の広さの割に小さい（パーサ約 680 行）。誤りの後ろも読めるので、IDE の機能を Rumoca の落ちる入力でも動かせる土台になる。
- 悪い点・持ち越し: 式の木が無いので、名前の解決（B2）にはまだ使えない。診断の文言は英語の仮のもので、`GM-E` の番号は付けていない。`Rumoca` の `parse 566` との比較（`xtask diff-frontend`）は、`Frontend` につなぐ B2 以降で行う。
- 依存: rowan 0.17.0（MIT OR Apache-2.0）。`Cargo.lock` に増えたのは rowan・countme・hashbrown 0.14.5・memoffset・text-size の 5 つ（rustc-hash 1.1.0 は既にあった）。いずれも許可のライセンス（[licenses.md](../licenses.md)）。

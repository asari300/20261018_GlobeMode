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

## 付記（S269）: 式・修飾・注釈・文を構造にした

決定 5（式・修飾・注釈・文の中身は不透明）を次のとおり置き換えた。それ以外の決定は変えない。

- **式は Pratt パーサ**（`parser.rs` の `expr` / `expr_bp`）。優先順位は Modelica 3.6 の文法（`or` < `and` < `not` < 関係 < `+ -` < `* /` < `^`、単項の `+ -` は `+ -` と同じ位置で `-a * b` は `-(a * b)`、`2 ^ -1` は許す）。範囲 `a:b:c` は式の最も外側（`RangeExpr`）、`if` 式は被演算子の位置でも読む（仕様より寛容）。
- **同じ優先順位の連鎖は 1 つの `BinaryExpr` に平らに入れる**（`a - b + c` は被演算子 3・演算子 2。左から結合する）。`1 + 1 + …` が 20 万個でも木が深くならず、再帰も増えない。`^` の連鎖も同じ扱いで、仕様が連鎖を認めないものは左結合として読む。
- 節点（`SyntaxKind`）: `Literal`・`ComponentRef`（`a.b[i].c`、先頭の `.` も）・`EndExpr`・`ParenExpr`（`(a, , b)` の空の位置も）・`ArrayExpr`・`MatrixExpr`（`;` は字句として残る）・`UnaryExpr`・`BinaryExpr`・`RangeExpr`・`IfExpr`・`CallExpr`（`der` / `initial` / `connect` は予約語のまま呼び出し）・`FunctionArgs`・`NamedArg`・`Comprehension`（`for` つき）・`ForIndices` / `ForIndex`・`PartialApp`（`function f(a = 1)`）。旧 `Expr`（不透明な式）は廃止した。
- **修飾**: `Modification` = [`ClassModification`] [`=` / `:=` 式]。`ClassModification` の中は `ElementModification`（`each`・`final`・`Name`・`Modification`・説明文字列）、`BreakModification`（Modelica 3.6 の `break x` / `break connect(a, b)`。Buildings に実在）、`Redeclaration`。`Annotation` の中身も `ClassModification`。`extends`・`class extends`・short class の `(…)` も同じ。`Subscripts` の中も式。
- **方程式と文**は `Equation` / `Statement` の 1 種のまま、中身が式になった（`a = b`、`a := b`、`(a, b) := f(x)`、`f(x);`、`connect(…);`、`return;` / `break;`、説明文字列と annotation）。`if` / `for` / `when` / `while` は条件・`ForIndices` が式の節点になった。種類の区別は先頭の字句（`KwIf` など）か `=` / `:=` の有無で行う（型付きの層で `enum` にする）。
- **不透明なまま残すもの**: `redeclare` / `replaceable` の引数（`,` か `)` まで）、`import A.{…}` の中、`enumeration(…)` と `der(F, x)` の short class の括弧の中、誤りから戻るために読み飛ばす範囲（`Error` 節点）。診断の `GM-E` の番号はまだ無い。
- **入れ子の上限**: 式・修飾・クラス・if/for/when を合わせて 128 段（`depth` は 1 つ）。超えたら括弧 1 組か 1 字句を `Error` に入れて先へ進む（`(`・`-`・`f(`・`{`・`[`・`if a then `・`not `・`x(a = ` を 5000 個並べた入力で試験）。
- **型付き AST**（`globe_mode_syntax::ast`）: 式の節点だけ。`Expr` の `enum` と、`BinaryExpr::{operands, ops}`・`UnaryExpr`・`RangeExpr`・`IfExpr::{branches, else_branch}`・`CallExpr::{callee, args, named_args}`・`ComponentRef::parts`・`Literal::token`。文・宣言・クラスの型は B2 の resolve を書くときに足す。
- 結果: `cargo test -p globe-mode-syntax` は 9 本 pass（優先順位 34 例の S 式比較、修飾と文の構造、型付き AST で平らな連鎖を畳む、壊れた式 13 例と全位置での切り落とし、深い・長い入力）、1 本 ignored。`src/Modelica` の 75 本は誤り 0。MSL 4.1.0 の 2556 本・4.0.0 の 2512 本は誤り 0・往復不一致 0・panic 0（release、0.7 秒）。`data/libs` の 10627 本は往復一致・panic 0 で、誤りは S268 と同じ 2 本だけ（`end for` の後ろの `;` 抜け。実際の構文の誤り）。初回は Buildings の 2 本が `break` の修飾（Modelica 3.6）で誤りになり、`BreakModification` を足して 0 にした。

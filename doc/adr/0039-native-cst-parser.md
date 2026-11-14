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

## 付記（S272）: B2 の最初の一切れ、名前解決は新しい crate `globe-mode-resolve`

新しい ADR は立てず、同じ流れの続きとしてここに足す。

- **新しい crate `globe-mode-resolve`**（依存は `globe-mode-syntax` と、syntax と同じ rowan の `TextRange` だけ。新しい外部依存は無い）。`Frontend` の feature `native`・LSP・Rumoca の `class_index` にはまだつなげない。
- **型付き AST を足した**（`globe_mode_syntax::ast`）: `StoredDefinition`（`within`・クラス）、`ClassDef`（名前・`encapsulated`・`replaceable`・`class extends`・`elements()`）、`ShortBody`（基底・`enumeration` の要素）、`ExtendsClause`、`ImportClause`（`ImportKind` = 修飾・別名・`*`・`{…}`）、`ComponentClause`・`Declaration`、`Name`。文の型はまだ無い。
- **クラスの木は `within` で組む**。ディレクトリ構造（`package.mo`）・単一ファイル・`Modelica 4.1.0/` のような版つきのディレクトリを、パスではなく各ファイルの `within` で同じに扱う（`package.order` は順序だけなので読まない）。`within` の親が無いものと、同じ名前の重複は診断に残す。ファイルごとに木を捨て、クラス情報と名前の使われ方（`Ref`）だけを持つ。
- **解決するもの**: 型名（部品・short class・`constrainedby`）、`extends` と short class の基底、`import` のパス、式の成分参照（`a.b.c`）。結果は `Class` / `Component` / `Local`（`for` の添字）/ `Builtin` / `Unresolved(Reason)`。先頭がクラスなら続きもクラスの要素としてたどり、部品なら先頭で止める。`annotation` の中と `redeclare` の引数の中は読まない。
- **探索（Modelica 3.6 の 5.3）**: 宣言した要素 → 受け継いだ要素（再帰。解決結果は覚え、解決中にまた来たら `Cycle`）→ `import`（名前つき・別名が先、`*` が後）を内側のクラスから外へ。`encapsulated` で止まって最上位へ、最後に組み込み。`extends` の基底の名前は、そのクラス自身の受け継ぎを見ない（外へ探す）。`class extends X` は外側のクラスの継承元にある同名のクラスを継ぐ。`import` は最上位から引く。
- **未対応と出し方**: `redeclare` つきの `extends` を通った探索、`replaceable` なクラスの中の探索が失敗したら `Reason::Redeclare`。ほかに `NotFound`・`Cycle`・`Inherited`（継承元が解決できない）・`NotAClass`。`protected` の要素への修飾つきアクセスの検査、部品の型を通した `a.b.c` の続きは未対応（先頭で止める）。
- **組み込みの名前**は表（`BUILTINS`。型・`time`・数学関数・演算子）で、探索で見つからなかったときだけ使う。先頭の `.`（`.sqrt`）にも使う。
- **結果**（`cargo test -p globe-mode-resolve`。`#[ignore]` の `msl_stats` は `GLOBE_MSL` か `data/msl`）: 小さなパッケージの単体試験（入れ子・import 4 種・extends・`encapsulated`・継承の輪・`replaceable`・列挙・`for` の添字・組み込み）と、`src/Modelica` の `GlobeModeValidation` で `redeclare` 以外の未解決 0 本（MSL を同時に読む）が pass。MSL 4.1.0 の 2556 本・6751 クラス・参照 161869 件（debug、読み込み 7 秒・解決 0.3 秒、診断 0）: 型名 32508（クラス 23683・組み込み 8825）、`extends` の基底 6164（クラス 5650・組み込み 514）、`import` 1017（クラス 720・部品 297）、式の成分参照 122180（部品 105902・組み込み 8347・クラス 5611・`for` の添字 2310）が解決し、**未解決は 10 件で、すべて `Redeclare`**（`Medium.saturationPressure` など、`replaceable` な `Medium` を通る参照）。初回は未解決 2229 件で、`class extends`（`redeclare model extends BaseProperties`）の受け継ぎと先頭の `.`（`.abs`）の組み込みを足して 10 件にした。
- **LSP との突き合わせ**（`globe-mode-lsp/tests/resolve_compare.rs`、`#[ignore]`、dev-dependency の追加だけ）: MSL の型名を、ファイルを 40 本おきに選んで 1 ファイル最大 3 箇所、計 110 箇所（`Modelica.Units.SI.Force` など、修飾つきの名前を含む）で、LSP の `textDocument/definition` の結果（ファイルとクラス名の位置）と自前の解決が **110 箇所すべて一致**（LSP が null・不一致は 0）。debug で約 5 分（大半は LSP 側の読み込み）。
- 悪い点・持ち越し: 「解決できた」は「正しい」とは限らない（突き合わせた箇所は一部）。`redeclare` / `replaceable` と修飾で変わる型、部品の型を通した続き、`Frontend` / LSP へのつなぎ（既存の `class_index` との置き換え）、`xtask diff-frontend` への組み込みは次の切れ。

## 付記（S275）: 成分参照の続きと `redeclare` / `replaceable`（B2 の 2 つ目）

新しい ADR は立てず、同じ流れの続きとしてここに足す。S272 の「部品なら先頭で止める」「`redeclare` つきの失敗は未対応」を次のとおり置き換える。

- **成分参照の続き（`a.b.c`）**: 先頭が部品なら、その部品の型の中で次の名前を探す（受け継ぎを含む。添字は `parts` に入らないので無視）。型は宣言したクラスの中で解決する。型が組み込み（`Real`、`type V = Real`）なら続きは属性（`x.start`）として `Builtin`。結果は最後の名前（`Component` か `Class` か `Builtin`）。`Library::resolve` は続きまで、`resolve_head` は S272 と同じ先頭までで、2 つの差が解決率の前後になる。
- **`replaceable` の既定の型と `constrainedby`**: 部品は `constrainedby` の型を先に、宣言の型を後に試す（`Component::tys`）。`replaceable package M = X constrainedby Y` は基底を `X`、`Y` の順に持つ。
- **`redeclare package M = X [constrainedby Y]`（短いクラスの形）**: `extends B(…)`・部品の修飾・short class の修飾（`= B(…)`）の直下にあるものを `ast::RedeclaredClass` に取り出し（`Redeclaration` 節点は字句が平らなので、前置きと `package M = X` の字句から読む）、探索でその場の `M` を `X`（なければ `Y`）に置き換える。`extends` では `member` がその基底に入る前に、部品の修飾では続きの最初の名前で、short class の修飾では基底として同じ道で効く。置き換えは 1 段で、置き換え先の `X` は宣言を含むクラスの中で解決する（循環したら置き換えない）。
- **未対応のまま `Reason::Redeclare`**: 置き換えが 2 段以上先で効く所（`replaceable` なクラスを継ぐ側で `redeclare record` したものを、継がれた側の `BaseProperties` の部品が使う `state.T` など）、部品の再宣言（`redeclare Foo x`）、長い形の再宣言（`redeclare model M … end M`）。
- **結果**（`cargo test -p globe-mode-resolve`、`#[ignore]` の `msl_stats`・`globe-mode-lsp` の `resolve_compare`）: 単体試験に `a.b.c`（添字・受け継ぎ・属性・失敗）と `replaceable` / `redeclare`（既定・`constrainedby`・`extends` の修飾・部品の修飾・short class の修飾・部品の `constrainedby`）を足した。MSL 4.1.0 の式の成分参照 122180 件のうち、続きのあるもの（`parts` が 2 つ以上）は 45463 件。S272（先頭だけ）では続きが 0 件解決だったのが、いまは **44824 件（98.6%）が最後の名前まで解決**し、残り 639 件はすべて `Redeclare`（大半は Media の `state.T`・`state_a.p` など `ThermodynamicState` で、上の「2 段以上先」。`cellData.…` は `extends` の修飾の部品の再宣言 `redeclare … cellData`。`internalThermalPort.heatPort…` は原因を調べていない）。S272 の未解決 10 件（`Medium.saturationPressure` ほか）は 0 になった（`extends` の修飾の `redeclare package Medium = …` を読むようにしたため）。`NotFound`・`Cycle`・`Inherited` は 0。
- **LSP との突き合わせ**: LSP の定義ジャンプは `p.v` の `v`（部品の型の中の部品）にも答えるので、型名に続けて成分参照の続きも突き合わせた（`definition_agrees_on_msl_component_paths`。自前が部品に解決した続きをファイル 40 本おきに最大 3 箇所）。93 箇所で、**一致 86・不一致 0・LSP が null 7**（`Modelica.Constants.pi`・列挙の要素・`massflowsink1.ports` など、LSP が答えない形）。型名の 110 箇所は引き続きすべて一致。
- 悪い点・持ち越し: 「解決できた」は「正しい」とは限らない（突き合わせは一部）。`Redeclare` の 639 件、`Frontend` / LSP へのつなぎ、`protected` へのアクセス検査は次の切れ。

## 付記（S276: Track B を初めて製品につないだ）

新しい ADR は立てず、同じ流れの続きとしてここに足す。

- **LSP の `documentSymbol` と `foldingRange` が、構文の誤りがあるファイルでも出る**。Rumoca のアウトラインが引けない（`Err`）ときだけ、`globe-mode-syntax` の CST から作ったアウトライン（`globe-mode-lsp/src/tolerant.rs`。クラスの名前・種類・`partial`・範囲と、部品の名前・型名・範囲。`extends`・`import`・connect は空）を使う。引けるときは今までと同じで、正常なファイルの出力は変わらない。新しい RPC は無い。`globe-mode-lsp` が `globe-mode-syntax` に依存するようになった（新しい外部依存は無い）。
- 以前は、誤りがあると `documentSymbol` は `null`、折りたたみはコメントと annotation だけだった。試験（`globe-mode-lsp/tests/tolerant_outline.rs`・`tests/folding.rs`）で、`SyntaxError.mo`（`;` 抜け）からクラス `Broken` と部品 `x : Real` が出ること、クラスと equation 節が折りたためること、途中まで書いた入れ子のクラスでも出ること、正常なファイルは同じであることを確かめた。`cargo test -p globe-mode-lsp` 全 pass（既存 1 本の期待値だけ、誤りのあるファイルでクラスの範囲が増えた分を直した）。
- 持ち越し: 補完・定義ジャンプの `globe-mode-resolve` へのつなぎ、`Frontend`（診断・コンパイル）は Rumoca のまま。

## 付記（S279）: LSP の補完を自前の構文と名前解決に（IDE の機能を先に自前に、S276 に続く 2 つ目）

新しい ADR は立てず、同じ流れの続きとしてここに足す。S272・S275 では「Frontend・LSP にはまだつなげない」とし、S276 で LSP のアウトライン・折りたたみだけをつないだ。IDE の機能から自前にする方針（Track B）の 2 つ目として、LSP の `textDocument/completion`（と `globe/completion`）の `a.` の後ろに自前の寛容な構文の木（`globe-mode-syntax`）の上の名前解決（`globe-mode-resolve`）を**足す**。

- **足し方**: 今の道（`Outline` で型を引く。カーソルの行を除いて読み直す S171 の `repair` を含む）で候補が出るときは今の道のまま（読める文書の出力と既存の試験の期待値は変えない）。今の道で何も出ないとき（カーソルの行を除いても、ほかの行にも誤りがある・`end` の前にファイルが切れている、など）だけ、自前の道を使う。
- **自前の道**: カーソルの前の `a.b.` を字句から取り、書きかけの文書だけを寛容に読んで `Library` に一時的に足し（`Library::with_source`。終わったら取り除く）、カーソルを含む一番内側のクラスを引き（`class_at`。`Class::range` を足した。`end` の無いクラスは読めたところまで）、`members_of` で `a.b` の直下を返す。部品なら型の部品と入れ子のクラス（受け継ぎを含み、`protected` の部品は除く）、クラスならその入れ子と部品。型の候補は S275 の `constrainedby` 先・宣言の型後の順。
- **ライブラリの木は 1 回だけ**: `Library` はライブラリのファイル全部から最初に要るときに 1 回だけ作り、`Server` が持つ（`Resolver::lib`。`LibraryTree` と同じく、起動後のライブラリの更新は見ない）。文書の編集では作り直さず、補完のたびに書きかけの文書 1 本を足して引く。補完は `Ref` を使わないので、作ったあとに `refs` を捨てる。wasm（ブラウザ）は `lib` を持たないので今の道だけ。
- **計測**（debug、`GLOBE_MSL=data/msl cargo test -p globe-mode-resolve --test resolve -- --ignored --nocapture msl_members`）: MSL 4.1.0（2556 本）で `Library` を作るのに 8.8 秒（debug。release ならこの 1 桁下の見込みだが、**release は測っていない**）、RSS は作る前 3.3 MB から 83 MB、`refs` を捨てて 75 MB（約 72 MB が持つ分）。書きかけの文書を足して `Modelica.Electrical.Analog.Basic.Resistor r;` の `r.` を引くのに 0.4〜0.5 ms。出たのは `R`・`n`・`p`・`v`・`i`・`T`・`T_heatPort`・`heatPort`・`useHeatPort` ほか 13 件。
- **試験**: `globe-mode-lsp` に補完 3 本（ほかの行に誤りのある書きかけで `r.`・`r.R`・`r.p.`、`end` の前に切れた `connect(r.`、読める文書では今の道と同じ）と、LSP の RPC で GlobeModeValidation の `Resistor` を 2 回引く試験（2 回目は作り直さない）、`globe-mode-resolve` に `with_source` がライブラリを元に戻すこと。今の道では空になることも同じ試験で確かめた。
- **悪い点・持ち越し**: 型が別の開いている文書にあるときは、自前の道は読まない（今の道が読める間は今の道が引く）。文書がライブラリのファイルと同じクラスを定義しているときは、ライブラリ側のクラスが優先される（重複として扱う）。`redeclare` が 2 段以上先で効く型、組み込みの型の属性（`x.` の `start`・`min`）、補完のほかの位置（`a.` 以外、名前引数、`annotation` の中）は出さない。ライブラリのファイルの更新で `Library` を作り直す仕組みは入れていない（`tree` と同じ）。

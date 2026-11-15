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

## 付記（S278）: B3 の最初の一切れ、flatten は新しい crate `globe-mode-flat`

新しい ADR は立てず、同じ流れの続きとしてここに足す。

- **新しい crate `globe-mode-flat`**（依存は `globe-mode-syntax` と `globe-mode-resolve`。rowan は両方と同じで、`TextRange` の型だけ。新しい外部依存は無い）。`Frontend` の feature `native`・LSP にはまだつなげない。`globe-mode-resolve` の中は変えていない（公開の API だけを使う）。
- **作り**: `Project` が `Library`（名前解決）と、クラスごとの CST（ファイルを読み直して `(ファイル, 名前の位置)` で引く）、全 `Ref` の解決結果の表を持つ。`resolve` は `extends` の基底（`bases`）を公開していないので、基底は CST の `extends` の名前を解決表で引いて `Project` 側で補った。`flatten(モデル名)` はクラスをインスタンス化する: `extends` を取り込みながら宣言と節を集め、修飾（外側が勝つ。式は書いた場所の前置きで名前を付け替える）を当て、変数と方程式を出す。
- **出力**: 変数（名前・`parameter`/`constant`/`discrete`/連続・`input`/`output`・`flow`/`stream`・型・大きさ・`start`・束縛・その他の属性）と、方程式（名前を付け替えて正規化した式の印字と、スカラーの数）。配列の変数は 1 個（大きさつき）で、部品の配列は要素ごと（`c[2].x`）。束縛は Rumoca の flat と同じく変数に残し、`FlatModel::binding_equations` で DAE の方程式の数を出す。`when` は別の列、`algorithm` 節は印字だけ。
- **評価するのは大きさ・`for` の範囲・添字だけ**（リテラル・parameter の束縛・`size`・四則・`min`/`max` など。外側の修飾の値も使う）。`if` の方程式・`when`・関数の呼び出しは式のまま残す。`[:]` は束縛が配列リテラルのときだけ大きさを決める。
- **`connect`**: インスタンスごとに集合を作り（union-find）、potential は等式（n-1 本）、flow は内側のコネクタを `+`・外側を `-` とした和が 0。つなげなかった内側のコネクタの flow は 0。最上位のモデル自身のコネクタの flow も 0（Rumoca の flat と同じ）。`for` の中の `connect` は展開して数える。`stream` は断る。
- **断るもの（`Unsupported::kind`）**: `redeclare`・`replaceable`・`class-extends`・`inner-outer`・`conditional-component`・`external`・`stream`・`expandable`・`unsized-array`・`non-constant-dim`・`composite-binding`・`array-type`・`connect-array`（配列のコネクタの一括接続）など。暗黙に近似せず、理由を返す。
- **Rumoca の flat との突き合わせ**（`tests/compare.rs`、`#[ignore]`。`FlatSummary::variable_names` を足した。JSON には出さない）: `GlobeModeValidation` の model・block 75 本のうち、自前で平らにできた 71 本（断ったのは `redeclare` 3・`inner`/`outer` 1）。Rumoca の flat まで通った 74 本と両方で通った 70 本は、**変数の名前の集合 70/70、方程式の数 70/70 が一致**。初回は名前 69・方程式 52 で、差は (1) 最上位のコネクタの flow = 0、(2) Rumoca は束縛と `when` を `equations` に数えない、(3) 組み込みの列挙 `StateSelect` の変数は flat に入れない、の 3 つだった。
- **MSL の例題 566 本**（`#[ignore]` の `msl_examples_flatten`。選び方は `xtask msl-funnel` と同じ。debug、読み込み 19 秒・flatten 1.2 秒）: **自前で平らにできたのは 67 本**（Rumoca の flatten は 562 本）。Rumoca の flat まで通った 66 本で、変数の名前の集合は 66/66、方程式の数は 64/66 が一致（合わない 2 本は `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`（10 対 12）と `UniformNoiseXorshift64star`（10 対 11）。`(a, b) = f(x)` とクロックの `previous` の数え方で、原因は未確認）。断った 499 本の理由（最初の 1 つだけ数える）: `redeclare` 205（大半は resolve が型を決められない `Redeclare`）、条件つきの宣言 131、`inner`/`outer` 43、大きさが定数でない 35、レコードの束縛 33、`[:]` の大きさが決まらない 25、`class extends` 11、`replaceable` 5、配列の型 5、配列のコネクタの一括接続 4、`expandable` 2。最初の計測は 11 本だった（共有 target の古い resolve のビルドが混ざり、`a.b` が部品の型を通した要素に解決されていた。touch して 66 本、`[:]` の大きさを足して 67 本）。
- **S275 との統合**（integrate-1116）: S275 で `Library::resolve` は成分参照の最後の名前（`a.b.c` の `c`）に変わったので、`globe-mode-flat` は先頭で止める `Library::resolve_head`（S272 の解決）を使う。突き合わせは同じ数（70/70・両方一致 70）。
- **持ち越し**: `redeclare` / `replaceable`（`globe-mode-resolve` の S275 と合わせる）、条件つきの宣言（`if` が定数なら外す）、`inner`/`outer`、配列のコネクタの一括接続、レコードの束縛、`xtask diff-frontend` への組み込み、B4（DAE）。

## 付記（S284）: B3 の続き、条件つきの宣言・`inner`/`outer`・レコードの束縛・大きさ

新しい ADR は立てず、S278 の続きとしてここに足す。変えたのは `globe-mode-flat` の中だけ（`globe-mode-resolve`・`globe-mode-syntax` は変えていない。新しい外部依存も無い）。

- **条件つきの宣言（`Real x if cond;`）**: 条件が定数（parameter・constant・列挙・リテラルの束縛から評価できる Boolean）なら、偽のとき宣言を落とし、その部品（と中・要素）につながる `connect` も落とす（MLS 4.4）。真なら普通の宣言。評価できなければ今までどおり `conditional-component` で断る。
- **定数の条件の `if` の方程式**: 条件を先頭から評価でき、真になる枝（無ければ `else`、それも無ければ空）が決まるなら、その枝の方程式だけを平らにする（`connect` も入る）。Rumoca の flat も同じ数え方で、`if not useHeatPort then Q_flowHeatPort = 0; end if` のような方程式が消える。評価できない条件は今までどおり式のまま残す。これで `connect-in-if-when` の 13 本が通った。
- **`inner`/`outer`**: `inner` の完全な名前を登録し、`outer` の部品は作らず、外側のインスタンスを 1 つずつたどって最初の同じ名前の `inner` を探して、参照（成分参照の先頭）を `inner` の名前に付け替える（`inner outer` は外側に `inner` があれば付け替え、なければ `inner` として振る舞う）。見つからない `outer` は `inner-outer` で断る。
- **レコードの束縛**: `R r = R(a = 1)` はコンストラクタの引数（名前つき・位置。位置は `extends` の欄を先に、`constant` を除いた宣言順）を欄ごとの修飾にし、`r = other` は `other` の平らにした変数（`other.f.g`）を欄ごとの修飾（印字だけ `other.f.g`）にする。明示した修飾 > 束縛 > 型の修飾。関数の戻り値のレコード・外部オブジェクト・欄がまだ分からない `other`・配列は `composite-binding` で断る。
- **大きさ**: 添字・`[:]` を決められる式を増やした。列挙の型名を添字にした配列（要素の数）、列挙の要素の比較（`filterType == FilterType.LowPass`）、パッケージの定数（`constant Integer L = …` を、そのパッケージの文脈で評価）、`max`/`min` の配列・行列の引数、`record.field` の束縛、後ろで宣言した配列に対する `size(x, 1)`（宣言から大きさを出す。深さ 16 で止める）。`[:]` は束縛の式の形（行列 `[a, b; c, d]`・範囲・`fill`/`zeros`/`ones`/`identity`・`{…}`・演算の被演算子・配列のパッケージの定数）から決める。
- **コネクタの中の parameter・constant は `connect` の等式にしない**（Rumoca の flat と同じ。`FluidHeatFlow` の `medium.cp` など）。
- **Rumoca の flat との突き合わせ**（S278 と同じ手順）: `GlobeModeValidation` の 75 本は、自前で平らにできたのが 71 本から **72 本**、両方で通った 71 本で名前の集合 **71/71**・方程式の数 **71/71** が一致（S278 は 70/70）。MSL の例題 566 本は、自前で平らにできたのが 67 本から **192 本**、Rumoca の flat まで通った 189 本で名前の集合 **189/189**、方程式の数 **186/189** が一致（S278 は 66/66 と 64/66）。合わない 3 本は `UniformNoise`（10 対 12）・`UniformNoiseXorshift64star`（10 対 11）（S278 から。原因は未確認）と、`Mechanics.Rotational.Examples.GenerationOfFMUs`（210 対 214。`use_w = false` / `use_a = false` で消えるコネクタと、その `connect` の数え方が違うと見ているが、原因は未確認）。
- **断った理由の内訳**（MSL の例題 566 本、最初の理由だけ数える。S278 → S284）: `redeclare` 205 → 263（上の機能で先へ進んだ分が `redeclare` に当たるようになった。数が増えたのは、前は別の理由で止まっていたため）、条件つきの宣言 131 → 6、`inner`/`outer` 43 → 0、大きさが定数でない 35 → 2、レコードの束縛 33 → 41（先へ進んだ分の関数の戻り値のレコードなど）、`[:]` 25 → 13、`class extends` 11、`replaceable` 5 → 9、配列の型 5 → 13、配列のコネクタの一括接続 4 → 7、`connect` の集合の不一致 0 → 7（`StateGraph` の例題が多い。原因は未確認）、`expandable` 2。
- **持ち越し**: `redeclare` / `replaceable`（263 本。resolve の S275 の `Redeclare` と合わせる）、配列の修飾つきの条件（`useHeatPort = fill(…)`）、関数の戻り値のレコード（Spice3）、外部オブジェクト、配列のコネクタの一括接続、`xtask diff-frontend` への組み込み、B4（DAE）。

## 付記（S279）: LSP の補完を自前の構文と名前解決に（IDE の機能を先に自前に、S276 に続く 2 つ目）

新しい ADR は立てず、同じ流れの続きとしてここに足す。S272・S275 では「Frontend・LSP にはまだつなげない」とし、S276 で LSP のアウトライン・折りたたみだけをつないだ。IDE の機能から自前にする方針（Track B）の 2 つ目として、LSP の `textDocument/completion`（と `globe/completion`）の `a.` の後ろに自前の寛容な構文の木（`globe-mode-syntax`）の上の名前解決（`globe-mode-resolve`）を**足す**。

- **足し方**: 今の道（`Outline` で型を引く。カーソルの行を除いて読み直す S171 の `repair` を含む）で候補が出るときは今の道のまま（読める文書の出力と既存の試験の期待値は変えない）。今の道で何も出ないとき（カーソルの行を除いても、ほかの行にも誤りがある・`end` の前にファイルが切れている、など）だけ、自前の道を使う。
- **自前の道**: カーソルの前の `a.b.` を字句から取り、書きかけの文書だけを寛容に読んで `Library` に一時的に足し（`Library::with_source`。終わったら取り除く）、カーソルを含む一番内側のクラスを引き（`class_at`。`Class::range` を足した。`end` の無いクラスは読めたところまで）、`members_of` で `a.b` の直下を返す。部品なら型の部品と入れ子のクラス（受け継ぎを含み、`protected` の部品は除く）、クラスならその入れ子と部品。型の候補は S275 の `constrainedby` 先・宣言の型後の順。
- **ライブラリの木は 1 回だけ**: `Library` はライブラリのファイル全部から最初に要るときに 1 回だけ作り、`Server` が持つ（`Resolver::lib`。`LibraryTree` と同じく、起動後のライブラリの更新は見ない）。文書の編集では作り直さず、補完のたびに書きかけの文書 1 本を足して引く。補完は `Ref` を使わないので、作ったあとに `refs` を捨てる。wasm（ブラウザ）は `lib` を持たないので今の道だけ。
- **計測**（debug、`GLOBE_MSL=data/msl cargo test -p globe-mode-resolve --test resolve -- --ignored --nocapture msl_members`）: MSL 4.1.0（2556 本）で `Library` を作るのに 8.8 秒（debug。release ならこの 1 桁下の見込みだが、**release は測っていない**）、RSS は作る前 3.3 MB から 83 MB、`refs` を捨てて 75 MB（約 72 MB が持つ分）。書きかけの文書を足して `Modelica.Electrical.Analog.Basic.Resistor r;` の `r.` を引くのに 0.4〜0.5 ms。出たのは `R`・`n`・`p`・`v`・`i`・`T`・`T_heatPort`・`heatPort`・`useHeatPort` ほか 13 件。
- **試験**: `globe-mode-lsp` に補完 3 本（ほかの行に誤りのある書きかけで `r.`・`r.R`・`r.p.`、`end` の前に切れた `connect(r.`、読める文書では今の道と同じ）と、LSP の RPC で GlobeModeValidation の `Resistor` を 2 回引く試験（2 回目は作り直さない）、`globe-mode-resolve` に `with_source` がライブラリを元に戻すこと。今の道では空になることも同じ試験で確かめた。
- **悪い点・持ち越し**: 型が別の開いている文書にあるときは、自前の道は読まない（今の道が読める間は今の道が引く）。文書がライブラリのファイルと同じクラスを定義しているときは、ライブラリ側のクラスが優先される（重複として扱う）。`redeclare` が 2 段以上先で効く型、組み込みの型の属性（`x.` の `start`・`min`）、補完のほかの位置（`a.` 以外、名前引数、`annotation` の中）は出さない。ライブラリのファイルの更新で `Library` を作り直す仕組みは入れていない（`tree` と同じ）。

## 付記（S290）: B4 の最初の一切れ、平らなモデルから DAE の構造と釣り合いの数

新しい ADR は立てず、S278・S284 の続きとしてここに足す。新しい crate `globe-mode-dae`（依存は `globe-mode-flat`・`globe-mode-syntax`・`globe-mode-ir`・`globe-mode-analysis`。dev に `globe-mode-frontend`。新しい外部依存は無い）を足しただけで、ほかの crate は変えていない。

- **やること**: `build(&FlatModel) -> Result<Dae, Unsupported>`。変数を状態（`der(x)` が現れる連続変数）・代数・離散（`discrete`・Integer/Boolean/列挙・`when` で代入する Real）・既知（parameter・constant・トップレベルの input・String・Clock）に分け、`DaeStructure`（連続の行と未知数）と `BalanceCount`（Rumoca の釣り合いと同じ分け方）を作る。方程式 = 連続の行 + 離散の定義、未知数 = 状態 + 代数 + 離散。`Dae::is_balanced`（総数の一致）と `Dae::is_regular`（`globe-mode-analysis` の構造の正則）がある。
- **incidence**: flat が出す方程式の印字を `model M equation … end M;` に包んで `globe-mode-syntax` で読み直し、`ComponentRef` から取る。`der(x)` は `der(x)` の未知数、状態そのものの参照は `state_refs`、`pre()` の中は無視する。
- **shortcut（広めの incidence。釣り合いの数は変わらない）**: `if`・`when`・`algorithm` は 1 本のまとまりとして枝の中の未知数の和集合を行にする。配列は添字が定数ならその要素、添字なしで大きさが行の数と同じなら行 k に要素 k、それ以外は全要素。非線形の判定はしない（`nonlinear` は空）。位置は付けない。
- **Rumoca に合わせた数え方**（計測で見つけて直した 3 つ）: (1) Clock 型の変数は数えず、辺の 1 つが Clock 型の変数そのものの方程式（`c = Clock(…)`・`y = c`）も数えない。`z = sample(u, c)` のように式の中で使うだけなら数える。(2) `initial algorithm` は数えない。(3) 方程式が離散の定義かは左辺だけで決める（右辺が離散でも左辺が代数なら連続の行）。
- **Rumoca との突き合わせ**（`tests/compare.rs`、`#[ignore]`。Rumoca 側は `Checker::check` の `balance` と、`structure` を `analyze` に通した `is_regular`。debug）:
  - `GlobeModeValidation` の 75 本: 平らにできた 72 本がすべて DAE になり、両方で DAE まで通った 71 本で、**釣り合いの判定 71/71・方程式と未知数の数 71/71・状態の数 71/71**。構造の正則は自前 66 / Rumoca 65、比べられた 70 本で一致 70。
  - MSL の例題 566 本: 平らにできた 192 本（S284）が**すべて DAE になった**。Rumoca の釣り合いまで通った 146 本で、釣り合いは自前 143 / Rumoca 145、**判定の一致 144/146**、**方程式と未知数の数の一致 144/146**、**状態の数の一致 141/146**、構造の正則は自前 119 / Rumoca 133、一致 132/146。
  - 修正の経過（MSL、同じ 146 本）: 最初の版は数の一致 70・判定の一致 124。Clock・`initial algorithm`・左辺だけの判定で数の一致 111（判定 111）。Clock の方程式を辺が Clock 型の変数そのものに絞って 144（判定 144）。最初の版で判定が合っていた 124 本には、数が合わないまま釣り合いの判定だけ合っていたものが含まれる（数の一致は 70 本だけ）。
- **既知の差**:
  - 数の差 2 本は `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`（方程式 7 対 9、離散 3 対 7）と `UniformNoiseXorshift64star`（7 対 8、離散 2 対 6）。乱数の状態を持つ離散変数（外部オブジェクトや配列の状態）の数え方が違う見込みだが、**原因は未確認**。
  - 状態の数の差 5 本、構造の正則の差 14 本（自前が正則と言わない本が多い）。差の一覧は出す仕組みを足したが、**この版では取り直していないので、モデル名と原因は未確認**。
  - `is_regular` は広めの incidence で正則寄りに出るはずだが、MSL では自前 119 が Rumoca 133 より少ない。配列の行の割り当て（添字なしの全要素）や、`algorithm` を 1 行にまとめる shortcut が効いている見込みだが、**確かめていない**。
  - 断ったモデルは flat の理由のまま（`redeclare` 263 ほか。S284）。DAE の段で断るのは、方程式の印字を読み直せないときだけ（MSL の 192 本では 0 本）。
- **持ち越し**: 差の分類と直し、`when` の中の `if` の枝ごとの数え方、非線形の判定、位置の付与、`Frontend` の feature `native` へのつなぎ、`xtask diff-frontend` への組み込み、B5 以降（simulate）。計測は debug。release は測っていない。

## 付記（S296）: LSP の定義ジャンプとホバーを自前の構文と名前解決に（IDE の機能を先に自前に、S279 に続く 3 つ目）

新しい ADR は立てず、同じ流れの続きとしてここに足す。S279 の補完と同じ形で、`textDocument/definition`・`globe/definition`・`textDocument/hover`・`globe/hover` を、**文書が今の道で読めないとき**だけ自前の道で答える。

- **足し方**: 今の道（`Outline`）が引けるときは変えない（読める文書の出力と既存の試験の期待値は同じ）。文書の `Outline` が `Err`（構文の誤り・`end` の前で切れている）のときだけ、`Resolver::definition` が自前の道を使う。`globe/hover` と `globe_definition` は `definition` を通るので、同じ道で答える。`textDocument/hover` は今の道が null で文書が読めないときだけ、同じ解決の宣言の 1 行を `**…**` で返す（読める文書では今まで通り、カーソルの下のクラスの見出し）。wasm（`lib` を持たない）は今の道だけ。
- **自前の道**（`definition.rs` の `tolerant_definition`）: S279 の `Library`（ライブラリのファイル全部から 1 回だけ作って `Server` が持つ。作る処理は `completion::tolerant_lib` に切り出して共有）に、書きかけの文書だけを `with_source` で一時的に足す。そのとき文書の `Ref`（型名・`extends`・成分参照）が `refs` に入るので、カーソルを含む一番狭い `Ref` を取り、`a.b.c` ならカーソルの段まで（`r.p.v` の `p` の上なら `r.p`）に切って `Library::resolve` で引く。クラスならそのクラスの範囲、部品なら部品の範囲を、そのファイルのパス（ライブラリのファイル、または文書）と共に返す。組み込み（`Real`）・`for` の添字・解決できないものは null。
- **ホバーの文面**: 今の `globe/hover` と同じで、宣言の行（クラスは見出しの 1 行、部品は `;` まで。説明文字列を含む）。`hover_at` が定義の `Target` から作るので、書き換えていない。
- **試験**: `globe-mode-lsp` に RPC の 1 本（ほかの行に誤りのある文書で、型名・`r1.p`・`r1.p.v` の定義ジャンプが `Components.mo`・`Connectors.mo` の同じ位置に飛ぶこと、`textDocument/hover` が `model Resistor` と部品の宣言を出すこと、`end` の前で切れた文書でも同じ）。既存の定義・ホバーの試験は変えずに通る。
- **悪い点・持ち越し**: 段は名前の頭からカーソルまでの `.` の数で決めるので、添字に `.` がある `a[x.y].b` ではずれる。型が別の開いている文書にあるときは読まない（S279 と同じ）。`redeclare` が 2 段以上先で効く型、`Component` が属性（`x.start`）のときは null。ライブラリのファイルの更新で `Library` を作り直す仕組みは入れていない。読める文書でも今の道が null のときの自前の道への落ち方は入れていない（読めない文書だけ）。

## 付記（S300）: 自前の段と Rumoca の突き合わせを `cargo xtask diff-frontend` に一本化

新しい ADR は立てず、Track B の「`xtask diff-frontend` で回帰が 0 件になったら既定を切り替える」の入口をここに足す。

- **何を一度で回すか**: 自前の parse → resolve → flat → dae を、MSL の例題（`msl-funnel` と同じ選び方の 566 本）と `GlobeModeValidation`（親がパッケージの model・block・class、partial を除く）で数え、Rumoca の flat（変数の名前の集合・方程式の数）・釣り合い（方程式と未知数の数・状態の数・判定）・構造の正則と突き合わせる。S278・S284・S290 の `tests/compare.rs`（`#[ignore]`）の数え方を xtask の `diff_frontend.rs` に一本化し、`globe-mode-flat`・`globe-mode-dae` の `compare.rs` と、そのための `globe-mode-frontend` の dev 依存を消した（その結果、CI の判定で frontend の変更が flat・dae の試験を回さなくなった）。
- **旗**: 段ごとの検査は「はい / いいえ」の旗（`parse`・`resolve`・`flat`・`rumoca_flat`・`flat_names`・`flat_eqs`・`dae`・`rumoca_balance`・`balance`・`counts`・`states`・`regular`）にして JSON（`target/frontend-diff.json`）に書く。次の実行は書く前にそれを読み、前回は通っていた旗が通らなくなったモデルを**回帰**として報告（`doc/frontend-diff.md`。手で編集しない生成物）の先頭に出し、回帰が 1 本でもあれば終了コード 1。「回帰が 0 件」はこの数で測る。
- **Rumoca を呼ぶ範囲**: 自前で平らにできたものだけ（呼ぶと flat・釣り合い・構造が 1 回で取れる）。Rumoca の全体の本数は `msl-funnel` の報告にある。resolve は例題のクラスとその中のクラスの参照だけを数える（継承元の中の参照は含めない）。
- **持ち越し**: 判定の閾値（どの旗が何本なら既定を切り替えるか）は決めていない。`Frontend` の feature `native` はまだ無い。

## 付記（S305）: `globe check --frontend native` で自前の段を検査につなぐ

新しい ADR は立てず、S300 の「`Frontend` の feature `native` はまだ無い」の続きとしてここに足す。既定は Rumoca のまま（切り替えの閾値は決めていない）。

- **形**: 計画どおり `globe-mode-frontend` の feature `native`（`src/native.rs`、`NativeFrontend`）として足した。`Frontend::check` を実装し、`globe-mode-syntax`・`-resolve`・`-flat`・`-dae`・`-analysis` を使う。外部依存は増えない（Cargo.lock の差は workspace 内の辺 5 本）。`Frontend::outline` は構文の誤りだけを返し、読めるファイルは `GM-U0006` にする（アウトラインを作る道は自前の段に無い。LSP は構文の木を直接読んでいる）。CLI の `globe-mode-cli` が feature を有効にして `globe check --frontend <rumoca|native>`（既定 `rumoca`）を持つ。facade・db・LSP・wasm は変えていない（salsa の検査の道は Rumoca のまま）。
- **段**: S0（入力のファイルの構文の誤りを位置つきで全件、`GM-E0001`）→ S1（モデルが無ければ `GM-E1002`、モデルから届くクラスの参照の未解決を位置つきで 20 件まで、`GM-E1001`）→ S4（`Project::flatten`）→ S5（`globe_mode_dae::build`、数が合わなければ `GM-E5002`）→ S6（`analyze`。`GM-E6001`）。S7 以降（インデックス・初期化・ループ・プリフライト）は検査しない。最後の行は `OK（native: S0〜S6 を通過。S7 以降は検査していない）`。`--json` は `CheckResponse` の形（`balance`・`notes`・`summary`。S7 以降は null）。
- **未対応**: 自前の段が断ったもの（平坦化の `kind: detail`、DAE を作れない、`redeclare` / `replaceable` が関わる名前）は、Rumoca に黙って切り替えず `GM-U0006`（新しいコード。`globe-mode-core` の `codes::NATIVE_UNSUPPORTED`。`GM-U0001`〜`0003` は Rumoca のコードで決まるもの、`0004`・`0005` は実行系で、自前の段の分を分けた）で `native では未対応: …` と出す。終了コードは既存の GM-U と同じ 3（`failure_code`）。新しい番号の範囲は要らなかった（ほかは既存の `GM-E0001`・`1001`・`1002`・`5002`・`6001`）。
- **試験**（`globe-mode-cli/tests/check_native.rs` の 3 本）: `GlobeModeValidation` の `Simulation.Decay`・`Index.Pendulum`・`Connect.RealCircuit`・`Invalid.StructurallySingular` で、`--frontend rumoca` と `native` が同じ終了コードと「方程式 N / 変数 M（状態・代数・出力・離散）」の行を出す（`StructurallySingular` は両方 `GM-E6001` で終了コード 1）。構文の誤り（`s.mo:4:12: error[GM-E0001]`）と未解決の名前（`n.mo:2:3: error[GM-E1001]`）は行と列つき。`expandable connector` は `GM-U0006`・終了コード 3。
- **手で確かめた MSL**（debug、`-L data/msl`）: `Modelica.Mechanics.Rotational.Examples.First`（54 / 54）・`Modelica.Blocks.Examples.PID_Controller`（89 / 89）・`Modelica.Thermal.HeatTransfer.Examples.TwoMasses`（20 / 20）は Rumoca と方程式・変数・状態・離散の数が同じ（代数と出力の分け方だけが違う。自前は出力を代数に含める）。`Modelica.Electrical.Analog.Examples.CauerLowPassAnalog` は `redeclare` で `GM-U0006`（終了コード 3）。debug で MSL を読むのに 15 秒（Rumoca の道は 7 秒。自前は毎回ライブラリ全体を読む。Rumoca の道は閉包だけ）、最大 RSS 約 227 MB。
- **悪い点・持ち越し**: (1) 自前の `DaeStructure` は位置を持たないので、`GM-E6001` の診断に行・列が付かない（Rumoca の道は付く）。(2) S7 以降は無い（`Init.OverDetermined` は native では OK）。(3) 自前の段は毎回ライブラリ全体を読む（閉包の読み込みは無い。速さは未最適化）。(4) 未解決の名前を数える走査は `xtask diff-frontend` と同じ手順の複製（共通化は未）。(5) 平坦化で断った理由を `GM-U0006` に一括している（モデルの誤りと未対応の見分けは `diff-frontend` の旗が頼り）。(6) LSP・wasm・Python・UI は native を選べない。次の段: 位置の付与（B4）、S7 以降を `DaeStructure` から回す（`globe-mode-analysis` は中立 IR なので自前の構造にも使える）、`redeclare`（263 本）。


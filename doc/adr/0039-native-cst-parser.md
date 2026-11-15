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

## 付記（S297）: B3 の続き、`redeclare` / `replaceable` / `class extends`

新しい ADR は立てず、S284 の続きとしてここに足す。変えたのは `globe-mode-flat` の中だけ（`globe-mode-resolve`・`globe-mode-syntax` は変えていない。新しい外部依存も無い）。

- **`redeclare` を型の差し替えとしてインスタンス化のときに当てる**。修飾の `redeclare …` は `globe-mode-syntax` では字句の並びなので、`model R <本文>; end R;` に包んで読み直し（`Project::snippet`）、部品の宣言かクラスの宣言にする。名前は書いた場所のクラスから `globe-mode-resolve` の `Library::resolve_head` で引いて仮のファイル番号の表に入れる（`res_of` が同じ形で引ける）。種類ごとに:
  - `redeclare [parameter] T x(…) = …`（部品）: `Mod::redecl` に型・接頭辞・添字を持たせ、元の宣言の修飾は残して新しい修飾を足す。外側が勝つ。`extends B(redeclare …)`・部品の修飾・short class の修飾 `model A = B(redeclare …)` のどれでも同じ。
  - `redeclare package M = X(…)`・`redeclare model M = X(…)`: インスタンスごとの表 `Ctx::cenv`（名前 → クラスと `(…)` の修飾）に足す。型名の先頭（`Medium.BaseProperties` の `Medium`）がこの表にあれば差し替え先のクラスの中を引く。差し替え先の名前は書いた場所の表で先に解決するので、`redeclare package Medium = Medium` で入れ子の部品へ運べる。`extends` の中のものも同じ表に足す（先に足したもの、つまり外側が勝つ）。
  - `redeclare function f = g`: 関数は平らにしないので無視する。長い形のクラス（`redeclare model M … end M`）は断る。
- **`replaceable` は既定の型で平らにする**（`constrainedby` は型の検査用なので見ない）。`class extends`（`redeclare model extends M(…)`）は受け継いだ `M`（外側のクラスの継承元にある同名のクラス）を先に取り込み、`(…)` の修飾を当てる。
- **差し替えたパッケージのメンバーを、受け継いだクラスの中でも差し替え後で引く**（`Ctx::heir`）。`Medium.BaseProperties` のように差し替え後のパッケージ（または `package Medium = MoistAir` の別名、パッケージの中のモデルを直接平らにするときはその親）を通して入れたクラスの中では、静的に解決した `ThermodynamicState` や `nXi` を、そのパッケージの同名のメンバーに引き直す。パッケージ自身の `extends P(redeclare record R = X)` が決めた `R` も使う。パッケージの定数は `extends` の修飾で決まる値も含めて引き（`pkg_env`。`extends PartialMedium(mediumName = …, nS = …)`）、`Medium.nX`・`size(substanceNames, 1)` を評価する。関数の呼び出しは式のまま残し、評価しない。
- **ついでに直したもの（redeclare を通すと露わになった）**: (1) `type T extends Real(unit = "V"); end T;`（長い形の型）を組み込みの型として扱う、(2) `type Axis = Real[3]` の添字を使う側の添字の後ろに足す（`array-type` の 13 本）、(3) `a + b = zeros(n)` のような方程式の数を `dims_of` で出す（`n = 0` の配列で 1 本多く数えていた）、(4) `protected` のコネクタは、親から見て「つながっていない内側のコネクタ」の flow = 0 の対象にしない（最上位のモデルは `protected` も対象にする。Rumoca の flat と同じ）。
- **Rumoca の flat との突き合わせ**（S278 と同じ手順）: `GlobeModeValidation` の 75 本は、自前で平らにできたのが 72 本から **74 本**（断ったのは `stream` 1 本だけ）、両方で通った 73 本で名前の集合 **73/73**・方程式の数 **73/73** が一致（S284 は 71/71）。MSL の例題 566 本は、自前で平らにできたのが 192 本から **279 本**、Rumoca の flat まで通った 273 本で名前の集合 **273/273**、方程式の数 **254/273** が一致（S284 は 189/189 と 186/189）。方程式の数が合わない 19 本は原因が未確認のものを含む: `Clocked` の 4 本（`UniformNoise` 系。`(a, b) = f(x)` とクロックの `previous` の数え方）、`QuasiStatic` の 11 本（`Complex` と Overconstrained connection の `Connections.isRoot` の扱いが違うと見ているが【未確認】）、DC 機械 2 本（`protected` のコネクタの数え方。自前は 4 本少ない）、`SuperCapDischargeCharge`（1 本）、`GenerationOfFMUs`（S284 から）。
- **断った理由の内訳**（MSL の例題 566 本、最初の理由だけ数える。S284 → S297）: `redeclare` 263 → 0、`replaceable` 9 → 0、`class extends` 11 → 0、配列の型 13 → 0、レコードの束縛 41 → 155（先へ進んだ分。`frictionParameters = dcpmData.frictionParameters`〔後ろで宣言したレコード〕46、外部オブジェクト `tableID` 29、関数の戻り値のレコード `R = Frames.nullRotation()` 27、配列のレコード 11 ほか）、配列のコネクタの一括接続 7 → 72（`source.sineVoltage.p` のように部品の配列の同名のコネクタをまとめてつなぐ形）、`[:]` 13 → 20、条件つきの宣言 6 → 19（条件に配列の修飾つきの `useHeatPort` や文字列の比較）、`connect` の集合の不一致 7 → 8、配列の修飾 0 → 8、`expandable` 2、`stream` 1。
- **持ち越し**: 後ろで宣言したレコードを欄ごとの修飾にするための宣言順の入れ替え（46 本）、関数の戻り値のレコード・外部オブジェクト、配列のコネクタの一括接続（72 本）、`Connections.isRoot` の評価、`xtask diff-frontend` への組み込み、B4（DAE）。

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

## 付記（S307）: native の検査は、モデルが使うライブラリのファイルの閉包だけを読む

新しい ADR は立てず、S305 の「悪い点・持ち越し」(3)（毎回ライブラリ全体を読む）の続きとしてここに足す。

- **形**: `globe-mode-frontend` の `closure_sources(libraries, model, user_texts)`（feature `native`）。ディレクトリの `.mo` を本文のまま読み（`Project::read_dirs`）、Rumoca の道の分割読み込み（S037、[ADR-0025](0025-native-library-closure.md)）と同じ字句の索引 `SplitLibrary` で、モデルと入力のファイルから名前の連なりをたどった閉包（外側の `package.mo`、クラスの見つからないファイルを含む）の（名前, 本文）だけを返す。`NativeFrontend::check` はそれだけを `Project::from_sources` に渡す。`split_library.rs` は feature `rumoca` だけでなく `native` でも作る（Rumoca に依存しない字句の走査）。`globe-mode-resolve` の木をファイル単位で遅延に作る形にはしていない（索引が既にあり、`globe-mode-resolve`・`-flat` の中は変えずに済んだ）。
- **前後（`globe check -L data/msl --frontend native`、debug）**: `PID_Controller` 15.8 s・最大 RSS 227 MB → 2.9 s・67 MB、`Rotational.Examples.First` 14.4 s・227 MB → 2.3 s・67 MB、`ChuaCircuit` は後だけ 2.2 s・67 MB（前は同じ道で 14〜16 s・227 MB）。閉包は 290〜320 ファイル（MSL は 2556 ファイル）。
- **前後（release、ライブラリの段だけ。`tests/native_closure.rs` の `#[ignore]`）**: 全部読みの `Project::from_dirs` が 1.15 s、閉包 + `Project::from_sources` は 1 モデル 0.24〜0.56 s。このテストの最大 RSS は、閉包の道だけを 10 モデル回して 82 MB、全部読みを足して 208 MB。release の `globe` のバイナリは作っていない（Rumoca を含めた release のビルドを避けた）。
- **確かめたこと**: S305 の `check_native`（3 本）が同じ結果。`tests/native_closure.rs` の通常の試験（使わないファイルは入らず、`extends`・`import`・外側の `package.mo` は入り、全部読みと平坦化が一致）と、`#[ignore]` の MSL の試験（10 モデルで、閉包だけの `Project` と全部読みの `Project` の `flatten` が成功も断る理由も完全に一致。5 本が成功、5 本が `redeclare` などで断る）。`xtask diff-frontend` は全部読みのままで、数字は変わらない。
- **代償**: 診断の位置は変わらないが、モデルが使わないライブラリのファイルの構文の誤りは見ない（以前も `from_dirs` は読むだけで報告していなかった）。1 ファイルに 2 つ以上のトップレベルのクラスがあるライブラリは索引に載らない（`split_library.rs` の shortcut。クラスが見つからないファイルはいつも読む）。閉包が狭すぎると名前が未解決になる危険は Rumoca の道と同じで、MSL の全例題での突き合わせは native では未実施（10 モデルのみ）。
- **`GM-E6001` に位置を付ける件はやっていない**: 自前の `DaeStructure` の `file`・`range` は `globe-mode-dae` が `None` を入れている。原因は上流で、`globe-mode-flat` の `Equation`・`Var` が元の CST の範囲を持たない（平坦化は名前を付け替えた印字だけを残す）。位置を持たせるには `flatten.rs` の `Equation { … }`・`Var { … }` を作る 7 か所を直し、部品の入れ子と `connect` の展開で「どの宣言・どの方程式から来たか」を運ぶ必要がある。別の担当が `flatten.rs` を触っているので次の WP に回す。

## 付記（S311）: 平らなモデルに元のソースの位置を持たせ、native の `GM-E6001`・`GM-E5002` に行・列を付ける

新しい ADR は立てず、S305・S307 の「悪い点・持ち越し」（自前の `DaeStructure` が位置を持たない）の続きとしてここに足す。

- **形**: `globe-mode-flat` の `Origin { file: Arc<str>, start, end }`（ファイル名とバイト範囲）を `Var::origin`・`Equation::origin`（`Option`）に持たせた。ファイルは番号でなく名前（閉包だけ読む `Project` と全部読みの `Project` で結果が一致する S307 の性質を保つ）。
- **運び方**: 変数は宣言の節点。`extends` で受け継いだ宣言・方程式は基底クラスのファイルの位置、`redeclare` で差し替えても元の宣言、入れ子の部品の中も同じ。`for` の展開は中の方程式（何回展開しても同じ位置）、定数の条件の `if` は選んだ枝の中の方程式。`connect` から作る potential の等式と flow の和は、その集合に属する最初の `connect` の文。つながない内側のコネクタの `flow = 0` は flow 変数の宣言。`redeclare` の読み直しの仮のファイルは位置を持たない。
- **dae**: `globe-mode-dae` が `DaeStructure` の `EquationRow`・`Unknown` の `file`・`range` に由来を入れる（束縛の方程式は変数の宣言。`algorithm` の行は付けない）。`globe-mode-analysis` は変えず、`GM-E6001` の診断は既存の `locate` で行・列を出す。
- **`GM-E5002`**: native は数が合わないときも S6 の解析を回し、多い側の「相手のいない」方程式か未知数の位置に診断を置き、名前を注にする。Rumoca の道の `GM-E5002` は位置を持たない（部品ごとの方程式の数の注だけ）ので、native のほうが位置の分だけ情報が多い。
- **確かめたこと**: `globe-mode-flat` の由来の試験（入れ子・`extends`・`for`・`connect`・`flow = 0`）、`check_native` の位置つきの `GM-E6001`・`GM-E5002`。`mise run msl:diff-frontend` の回帰は 0。
- **持ち越し**: `algorithm` の文ごとの位置、`if`・`when` の枝ごとの位置、複数の `connect` を指す診断（今は集合の最初の 1 つ）。

**S303 の付記（#108）**: `diff-frontend` が基準にする JSON は、回帰がなく全モデルを回したときだけ書き換える（回帰した結果を基準にすると次の実行が回帰を見なくなる。`--filter`・`--limit` の実行は一部だけなので書かない）。基準の JSON が壊れている・形が違うときは、黙って比較を飛ばさず誤りにする。

## 付記（S313）: CLI 以外からも native の検査を選べるようにした

新しい ADR は立てず、S305・S307 の「残したこと」（LSP・wasm・Python・UI から native を選べない）の続きとしてここに足す。既定は Rumoca のまま。

- **facade**: `globe_mode::CheckFrontend { Rumoca, Native }`（`FromStr` は `rumoca`・`native`）と `Workspace::check_with(model, frontend)`。`check` は `check_with(.., Rumoca)`。`Native` は作業中のファイルと、`with_libraries` に渡したディレクトリ（`library_paths`）を `NativeFrontend` に渡す。`with_library_sources` のワークスペースにはディレクトリが無く、ライブラリが見えない（wasm は対象外）。応答の組み立ては `native_response`（CLI の `check_native` と共有）。
- **feature**: facade に `native`（`globe-mode-frontend/native`）、LSP に `native`（`globe-mode/native`）。CLI（`globe lsp` を含む）と Python の束ねが立てる。wasm は立てない（理由: native の段の crate が wasm の大きさと build に入る。入れるかどうかは別に判断する）。立てない build で `Native` を選ぶと `GM-U0006`「この build に入っていない」を返す。外部依存は増えない。
- **LSP**: `globe-mode.toml` の `[check] frontend = "rumoca" | "native"`（`LintOptions::frontend`。他の値は設定の誤りで、既存の `showMessage` の道）。`publishDiagnostics` の検査（`diagnostics`）だけが `check_with` を使う。`globe/check` の要求は変えていない。設定ファイルが無ければ今まで通り。
- **Python**: `Workspace.check(model, frontend="rumoca")`。知らない名前は `ValueError`。
- **試験**: facade `tests/check_with.rs`（`expandable` が `GM-U0006`、既定は出さない。feature があれば Decay が通る）、LSP `config_frontend_native_drives_diagnostics`、`test_check_frontend_native`（pytest）、`config` の単体試験に `[check]`。
- **持ち越し**: wasm・UI から選ぶ（wasm のビルドの大きさの測定が先）。LSP の他の要求（`globe/check`・補完など）と、`outline` 系は Rumoca のまま。

## 付記（S320）: native と LSP の issue 5 件（#111・#113・#114・#115・#116）

新しい ADR は立てず、3 回目のレビューの指摘のうち native と LSP の 5 件を再現して直した記録としてここに足す。

- **#111 入れ子の深さ**: `globe-mode-flat` の `inst_class` に 64 段の上限（`F::inst_depth`）。自分を部品に持つモデル・`redeclare` の循環は `deep-instantiation`（native の検査では `GM-U0006`）で断り、abort しない。Rumoca の `GM-E3001`（循環）と言葉は違うが、断る点は同じ。
- **#113 編集中の本文が真**: `NativeFrontend::check` は、閉包が読んだライブラリのファイルのうち、作業中のファイルと同じ名前（`\` を `/` にそろえて比べる）のものを使わない。
- **#114 UTF-8 でないファイル**: `Project::read_dirs` は U+FFFD にして読む（#85 と同じ）。
- **#115 パスの比べ方**: LSP は `same_path_key`（絶対・`..` を字句で畳む・`/`・ドライブ文字は小文字。symlink は解かない）で、ライブラリの木のファイル名と開いた文書を比べる（`drop_lib_for`・定義ジャンプの読み・開いている文書の判定）。
- **#116 漏れ**: flat の `redeclare` の節の名前解決（`extra`）は `Project` が持つ値にした（`Box::leak` をやめた。`res_of` は `Option<Resolution>` を返す）。
- 詳細と試験は [S320 の解説書](../sessions/S320-native-issues/解説書.md)。

## 付記（S312）: 残る食い違いの原因を分け、数え方を直した（B3・B4 の続き）

新しい ADR は立てず、S297・S300 で出た「方程式の数が合わない 19 本」と「DAE の釣り合いの判定が合わない 10 本」を、原因ごとに確かめて直した記録をここに足す。flat の構造体（`Var`・`Equation`）の形は変えていない（数え方の直しに限る）。

- **調べ方**: `cargo xtask diff-frontend --class <完全修飾名,…> [--dump]`（例題でなくても、このクラスをそのまま検査する。`--dump` は自前の flat の変数と方程式、Rumoca の flat の方程式の由来を並べる）。Rumoca の方程式ごとの由来（`connection equation: a = b`・`flow sum equation: …`・`unconnected flow: …`・`equation from <部品>`）は `FlatSummary::equation_origins`（JSON には出さない）に足した。部品を単独で検査して Rumoca と比べ、小さいモデルに絞ってから直した。
- **flat の方程式の数（19 本）の原因と直し**（すべて一致した）:

| 原因 | 本数（例） | 直し |
| --- | ---: | --- |
| 行列 `[y] = [u1; u2; u3]` の数え方。要素がベクトルの行列を断っていて 1 本と数えた（`Multiplex3` など） | 1（`Rotational.Examples.GenerationOfFMUs`。4 つの `TorqueToAngleAdaptor` の内側） | `dims_of` が行列の要素をスカラー・ベクトル（n×1）・行列として、行は横に、行どうしは縦に連結する |
| タプル `(r64, state64) = random(previous(state64))` の左辺は、欄の大きさの和（配列の欄を 1 と数えた） | 4（`Clocked` の乱数・AD/DA） | 左辺の各項の `shape` の和 |
| 接続集合を丸ごとのコネクタの単位で作った。`connect(a, c.x)` と `connect(b, c)` が別の集合になり、flow の和と等式が重複した | 3（`DCPM_Drive`・`DC_CompareCharacteristics`・`SuperCapDischargeCharge`） | 集合を葉の変数（`c.x.Q_flow`）の単位で作る。外側のコネクタの端（自分のコネクタを丸ごと／欄で指す端）は flow の和の符号を逆にする |
| `protected` のコネクタの flow = 0。Rumoca は「丸ごと `connect` の端になっている」か「どこにもつながっていない」葉だけを 0 にする | 同上（`DCPM_Drive` の `internalThermalPort`） | 公開のコネクタは今まで通り（最上位は常に、内側は親が）、`protected` は葉ごとに上の 2 条件だけ 0 |
| **過剰決定コネクタ**（`Connections.root`・`branch`・`potentialRoot`・`isRoot`、MLS 9.4）。Rumoca は仮想接続グラフの全域森を選び、切れた辺をまたぐ `equalityConstraint`（戻り値の大きさ 0）の等式を作らず、`if Connections.isRoot(…)` の枝を根かどうかで決める | 11（`QuasiStatic` の電気・磁気） | 新しい `flatten/graph.rs`。節点は `equalityConstraint` を持つレコードの部品と `Connections.*` の引数、必須の辺は `branch`、任意の辺は `connect`。閉路と「根を持つ木どうし」を避けて選び、選ばれた辺でつながる群ごとの連鎖にして等式を作り直す。`if` は根でなければ落とす |

S297 の見立て（Complex のレコード）は外れで、`Connections.isRoot` と `protected` のコネクタは当たっていた。全域森の選び方（辺の順）は Rumoca と違ってよい（接続集合ごとの等式の数の合計は選び方によらない）。大きさが 0 でない `equalityConstraint`（MultiBody の `Orientation`）は考えない（MultiBody はまだ平らにできない）。

- **DAE の数え方（釣り合いの判定 10 本・状態の数 16 本）の原因と直し**:

| 原因 | 本数（例） | 直し |
| --- | ---: | --- |
| 条件が定数の `if` 式の枝に `der(x)` があるとき、自前は状態に数えた。Rumoca の flat は選ばれない枝を落とす（`y1 = if use_fder then -der(y) else 0`） | 4（`Electrical.Analog` の `GenerationOfFMUs`・`ResonanceCircuits`、`Translational`・`HeatTransfer` の `GenerationOfFMUs`） | 式の印字で、条件が定数に評価できる `if` 式は選んだ枝だけを出す（`const_if_expr`） |
| パッケージの定数の評価が輪になって `None` になり、`if m > Modelica.Constants.small` を畳めなかった（`pkg_binding` が、別のパッケージの同名の定数に、差し替え後のパッケージの `extends` の修飾を当てた） | 10（`FluidHeatFlow` の例題すべて） | その修飾は、そのパッケージの系統の定数にだけ当てる |
| 束縛の中の `der`（`Real w = der(flange_a.phi)`）が状態を作らなかった | 2（`DCPM_Drive`・`DC_CompareCharacteristics`。`Utilities.Coupling` の `w = der(flange_a.phi)`） | 束縛の木も `der` を探す |
| レコードの方程式 `y = Complex(a, b)` は DAE で欄ごとの 2 本（平らにした方程式は 1 本）。行は欄の変数すべてに触れる（shortcut） | 1（`TestConversionBlock`。正則も合う） | 左辺がレコードの部品なら、欄のスカラーの数にする |
| 大きさ 0 の配列（`Xi[0]`）を未知数 1 と数えた | 2（`Media.Examples.SimpleLiquidWater`・`WaterIF97`） | 大きさ 0 は 0 と数える |

- **数字**（MSL の例題 566 本、S297 → S312。`GlobeModeValidation` 75 本は変わらず）: 方程式の数の一致 254/273 → **273/273**、釣り合いの判定 191/201 → **201/201**、方程式と未知数の数 191/201 → **201/201**、状態の数 185/201 → **201/201**、構造の正則 179/201 → **184/201**。回帰 0 本、改善 36 本（旗ごとに、方程式の数 19・釣り合いの判定 10・状態の数 16・正則 5 本。同じモデルが複数の旗で数えられる）。
- **既知の差**（正則の判定 17 本。**原因は未確認**）:
  - `Modelica.Clocked` 16 本（自前が正則と言わず、Rumoca は言う）。Clock の信号は Rumoca では離散の側に入ると見ているが、自前は連続の行・未知数のまま行列に入れるため行と未知数の数が合わない見込み。クロックの分割は持たないので直していない。
  - `Modelica.Media.Examples.WaterIF97` 1 本（同上の向き。判定・数は合っている）。
- **計測の注意**: 共有の `target` のままだと、別の worktree が同じ名前の crate を先にビルドした成果物が混ざり、直した結果が出ないことがある（このセッションで数回）。xtask を回すときは worktree 専用の `CARGO_TARGET_DIR` に置き、回した結果にコミットの内容が入っているかを crate の試験で確かめる。

## 付記（S325）: 構造の正則の食い違い 17 本を直した（クロックの区画、`if` の枝、レコードの離散の欄）

新しい ADR は立てず、S312 で残した「構造の正則の食い違い 17 本」（`Modelica.Clocked` の例題 16 本と `Modelica.Media.Examples.WaterIF97`。自前が非正則、Rumoca が正則）の原因を `cargo xtask diff-frontend --class … --dump` で確かめて直した記録をここに足す。`--dump` は、自前と Rumoca の `DaeStructure`（行ごとの incidence と、どの行からも参照されない未知数）も並べる。変えたのは `globe-mode-dae` の数え方だけ（`globe-mode-flat` は変えていない）。S312 の見立て（「Clock の信号を Rumoca が離散側に入れている」）は当たっていた。Rumoca は MLS 16.5.1 のクロックの区画を求め、区画に入る変数と方程式を連続の行・未知数から外す（`rumoca-phase-dae` の `analysis/clocks.rs`・`equation_partitions.rs`）。

| 原因 | 本数（例） | 直し |
| --- | ---: | --- |
| **クロックの区画**。`y = sample(x, c)` の `y`、そこから方程式でつながる変数が、自前では連続の行・未知数のまま。行が未知数より少なく、非正則になった | 12（`Elementary.RealSignals` の `UpSample1`・`AssignClock`・`UniformNoise` ほか、`SimpleControlledDrive`・`Systems.EngineThrottleControl`） | 方程式・束縛・クロックの `when` の変数を素集合で区画にまとめる。`sample`・`hold`・`subSample`・`superSample`・`shiftSample`・`backSample`・`noClock` の**第 1 引数は別の区画**なので入れない。区画に `sample`・`subSample`・`superSample`・`shiftSample`・`backSample` の結果、`previous`、`when Clock() then`（または Clock 型の変数の `when`）があれば「クロックの区画」とし、その Real（代数）を離散にし、区画の方程式を離散の方程式に数える（総数は変わらない） |
| **`if` の枝が離散と連続の代入を混ぜる**。`if … then noWphase = false; Tv = 0; … end if` を、最初の代入が離散だというだけで全部離散の方程式に数え、連続の代入 15 本分の行が消えた | 4（`CascadeControlledDrive` の `kinematicPTP`） | 最初の枝が単純な代入だけで、代入先の大きさの和が方程式の数に一致するとき、代入先ごとに離散か連続かを分ける（連続の行の incidence は `if` 全体の参照のまま） |
| **レコードの方程式の離散の欄**。`state2 = Medium.setState_pT(…)` は `phase`（Integer）の欄も 1 本と数え連続の行にしていた（S312 の欄ごとの数え方の続き） | 1（`WaterIF97`） | 欄が離散の変数なら離散の方程式に数える |

- **数字**（MSL の例題 566 本、S312 → S325。`GlobeModeValidation` 75 本は変わらず）: 構造の正則の一致 184/201 → **201/201**（`GlobeModeValidation` は 72/72 のまま）。方程式の数・釣り合い・状態の数は 273/273・201/201・201/201・201/201 のまま。回帰 0 本、改善 17 本（すべて旗 `regular`）。
- **試験**: `globe-mode-dae` に 3 本（`clocked_partition_is_discrete`・`if_equation_splits_discrete_and_continuous_targets`・`record_equation_discrete_field_is_discrete_equation`）。
- **shortcut（計測の範囲で Rumoca と一致するだけ）**:
  - クロックの区画の持ち主（どのクロックか）は求めず、変換の整合（`sample(u, c)` の `c` が区画の持ち主か、変換できるか）も見ない。区画にクロックの印があるかだけで離散にする。MSL の例題の計測では、Clocked 以外で結果が変わったものは無かった（回帰 0）。
  - 区画に `der` があるとき（Rumoca は断る）は考えない。
  - `if` を分けるのは、最初の枝の代入先がスカラーの 1 変数ずつのときだけ。入れ子の `if`・`for` は今まで通り最初の代入で決める。
  - 離散の欄を持つレコードの方程式の incidence は、今まで通り欄の変数すべてに触れる（行は欄ごとに分けない）。
- **計測の注意の追記**: `cargo xtask`（`.cargo/config.toml` の alias は `--release`）の成果物 `target/release/xtask` は、共有の `target-shared` を使う別の worktree が上書きし、`--dump` の出力が増えない・直した結果が出ない、ということがあった。**worktree 専用の `CARGO_TARGET_DIR`**（最初の release ビルドに約 15 分）で回すと確実。
- **既知の差**: 構造の正則の食い違いは 0 本。ただし上の shortcut は、クロックの持ち主・変換の整合・区画の `der`・入れ子の `if` を持つモデルでは Rumoca と違いうる（MSL の例題では出ていない）。


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

## 付記（S342）: 段ごとの数字の推移と、付記の案内

新しい ADR は立てず、[roadmap.md](../roadmap.md) の Track B の欄を「今の数字と残り」の要約にしたときに、そこにあった経緯をここへ集めた。付記は書いた順に並べてあり（S334 が S269 の前にある）、読みにくいので、下に案内を付ける。今の数字は [frontend-diff.md](../frontend-diff.md)（`mise run msl:diff-frontend` が書く）。

MSL 4.1.0 の例題 566 本での推移（括弧は「一致した本数 / 母数」。各行の数字はその付記の報告から写した）。

| 付記 | 自前で平らにできた | Rumoca も flat まで通った本数（名前・方程式の数） | Rumoca の釣り合いまで通った本数（判定・方程式と未知数の数・状態の数・構造の正則） |
| --- | ---: | --- | --- |
| S278 | 67 | 66（66/66・64/66） | まだ DAE が無い |
| S284 | 192 | 189（189/189・186/189） | まだ DAE が無い |
| S290 | 192（全部 DAE になった） | 測り直していない | 146（144/146・144/146・141/146・132/146） |
| S297 | 279 | 273（273/273・254/273） | 報告に無い |
| S312 | 279 | 273（273/273・273/273） | 201（201/201・201/201・201/201・184/201） |
| S325 | 279 | 273（273/273・273/273） | 201（201/201・201/201・201/201・201/201） |
| S324（S325 を取り込んだ後） | 358 | 350（350/350・340/350） | 253（253/253・253/253・253/253・249/253） |
| S330 | 358（DAE にできた 358/358） | 350（350/350・340/350） | 253（4 つとも 253/253） |
| S329（単独） | 471 | 463（425/463・435/463） | 297（判定 295/297・状態 293/297・正則 289/297。方程式と未知数の数は報告に無い） |
| S332（S329 と S330 を合わせた） | 471（DAE にできた 471/471） | 463（463/463・435/463） | 334（297/334・297/334・294/334・296/334） |
| S333 | 471 | 463（463/463・449/463） | 334（332/334・332/334・332/334・301/334） |
| S334 | 471 | 463（463/463・451/463） | 334（334/334・334/334・332/334・334/334） |
| S337 | 471 | 463（463/463・452/463） | 334（4 つとも 334/334） |
| S336（S337 と合わせた） | 477（DAE にできた 477/477） | 468（468/468・457/468） | 334（4 つとも 334/334。新しい 6 本は Rumoca の釣り合いが無い） |
| S335（S336 を取り込む前。S337・S341 は取り込み済み） | 493 | 484（484/484・473/484） | 351（4 つとも 351/351） |
| S335（S336 と合わせた） | 518（DAE にできた 518/518） | 491（491/491・480/491） | 351（4 つとも 351/351） |
| S355（S335 の上。`stream`） | 538（DAE にできた 538/538） | 511（508/511・498/511） | 367（判定 365/367・方程式と未知数の数 364/367・状態 366/367・構造の正則 363/367） |
| S361（S355 の上。外側の `inner` への `connect`） | 548（DAE にできた 548/548） | 521（518/521・508/521） | 377（判定 374/377・方程式と未知数の数 373/377・状態 376/377・構造の正則 369/377） |

- parse は MSL の例題 566/566（S300 の時点も今も）、resolve は未解決 0 が 467/566（S300 の時点も今も。残る 99 本はすべて `Redeclare`）。`GlobeModeValidation` 75 本は S334・S337 で、Rumoca も通った 73 本の名前・方程式の数・判定・数・状態が 73/73、正則が 72/72。
- 断る理由の残り（S361。18 本、最初の理由だけ数える）: `composite-binding` 6・`expandable` 5・`non-constant-dim` 4・`connect-array` 1・`non-constant-for` 1・`unsized-array` 1。`connect` は 0（S361。S355 では 10 本）、`stream` は 0（S355）、`composite-binding` の 25 本のうち 19 本は S335 が平らにした。
- 方程式の数が合わない 11 本は（S334 では 12 本。S337 で `ShowTransferFunction` を直した）、Rotational・Translational の摩擦・歯車の 10 本（S330 が Rumoca の数え方の誤りと確かめ、S331 が最新の release v0.10.4 では直ると確かめた。[upstream.md](../upstream.md) の下書き 36）、`ActuatorWithNoise`（同じ表の数え方）。状態の数が合わなかった 2 本（`IMC_DCBraking`・`IMC_Initialize`）は S337 で直した（Rumoca が初期方程式の `der` も状態にするため。自前の抜け）。

付記の案内（主題ごと）:

| 主題 | 付記 |
| --- | --- |
| B1 構文 | 本文、S269 |
| B2 名前の解決 | S272、S275 |
| B3 flatten | S278、S284、S297、S312、S324、S329、S333、S334、S335（`[:]` の大きさ）、S336（`Complex` の配列の束縛）、S337、S355（`stream`）、S361（外側の `inner` への `connect`・`inner outer`・`cardinality`） |
| B4 DAE | S290、S312、S325、S330、S332（計測と、名前・数の食い違いの原因）、S337（初期方程式の `der` と状態の数） |
| 製品へのつなぎ | S276（アウトライン・折りたたみ）、S279（補完）、S296（定義ジャンプ・ホバー）、S305（`globe check --frontend native`）、S307（閉包だけを読む）、S311（位置）、S313（CLI 以外から選ぶ）、S320（#111・#113〜#116）、S327（#118）、S352（inlay hint）、S354（call hierarchy） |
| 突き合わせ | S300（`cargo xtask diff-frontend`） |

roadmap の要約で外した、古くなっていた「残り」: 診断の `GM-E` の番号が無い（S305 の native の検査が `GM-E0001`・`GM-E1001`・`GM-E1002`・`GM-E5002`・`GM-E6001` を出す）、定義ジャンプ・ホバーが未（S296 で済）、`Frontend` の feature `native` が無い（S305 で済）、B3 の残りが後ろで宣言したレコード・関数の戻り値のレコード・配列のコネクタの一括接続（S324 で減らした）、B4 の残りが balance の差 2 本・状態 5 本・正則 14 本（S312・S325・S330・S334 で直した）。まだ残っているのは、文の型付き AST（S269 は式の節点だけ）、`redeclare` の引数の構造（不透明な節点のまま）、native の検査の S7 以降と wasm・UI から選ぶこと、実行へのつなぎ、既定を切り替える閾値（未決）。

## 付記（S269）: 式・修飾・注釈・文を構造にした

決定 5（式・修飾・注釈・文の中身は不透明）を次のとおり置き換えた。それ以外の決定は変えない。

- **式は Pratt パーサ**（`parser.rs` の `expr` / `expr_bp`）。優先順位は Modelica 3.6 の文法（`or` < `and` < `not` < 関係 < `+ -` < `* /` < `^`、単項の `+ -` は `+ -` と同じ位置で `-a * b` は `-(a * b)`、`2 ^ -1` は許す）。範囲 `a:b:c` は式の最も外側（`RangeExpr`）、`if` 式は被演算子の位置でも読む（仕様より寛容）。
- **同じ優先順位の連鎖は 1 つの `BinaryExpr` に平らに入れる**（`a - b + c` は被演算子 3・演算子 2。左から結合する）。`1 + 1 + …` が 20 万個でも木が深くならず、再帰も増えない。`^` の連鎖も同じ扱いで、仕様が連鎖を認めないものは左結合として読む。
- 節点（`SyntaxKind`）: `Literal`・`ComponentRef`（`a.b[i].c`、先頭の `.` も）・`EndExpr`・`ParenExpr`（`(a, , b)` の空の位置も）・`ArrayExpr`・`MatrixExpr`（`;` は字句として残る）・`UnaryExpr`・`BinaryExpr`・`RangeExpr`・`IfExpr`・`CallExpr`（`der` / `initial` / `connect` は予約語のまま呼び出し）・`FunctionArgs`・`NamedArg`・`Comprehension`（`for` つき）・`ForIndices` / `ForIndex`・`PartialApp`（`function f(a = 1)`）。旧 `Expr`（不透明な式）は廃止した。
- **修飾**: `Modification` = [`ClassModification`] [`=` / `:=` 式]。`ClassModification` の中は `ElementModification`（`each`・`final`・`Name`・`Modification`・説明文字列）、`BreakModification`（Modelica 3.6 の `break x` / `break connect(a, b)`。Buildings に実在）、`Redeclaration`。`Annotation` の中身も `ClassModification`。`extends`・`class extends`・short class の `(…)` も同じ。`Subscripts` の中も式。
- **方程式と文**は `Equation` / `Statement` の 1 種のまま、中身が式になった（`a = b`、`a := b`、`(a, b) := f(x)`、`f(x);`、`connect(…);`、`return;` / `break;`、説明文字列と annotation）。`if` / `for` / `when` / `while` は条件・`ForIndices` が式の節点になった。種類の区別は先頭の字句（`KwIf` など）か `=` / `:=` の有無で行う（型付きの層で `enum` にする）。
- **不透明なまま残すもの**: `redeclare` / `replaceable` の引数（`,` か `)` まで）、`import A.{…}` の中、`enumeration(…)` と `der(F, x)` の short class の括弧の中、誤りから戻るために読み飛ばす範囲（`Error` 節点）。診断の `GM-E` の番号はまだ無い。
- **入れ子の上限**: 式・修飾・クラス・if/for/when を合わせて 128 段（`depth` は 1 つ）。超えたら括弧 1 組か 1 字句を `Error` に入れて先へ進む（`(`・`-`・`f(`・`{`・`[`・`if a then`・`not`・`x(a =` を 5000 個並べた入力で試験）。
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

## 付記（S337）: 初期方程式の `der` と状態の数（Electrical.Machines 2 本）、`Complex` の配列への方程式の数

新しい ADR は立てず、S334 の持ち越し（状態の数 2 本と `ShowTransferFunction` の方程式の数）の原因を `cargo xtask diff-frontend --class … --dump` で確かめて直した記録をここに足す。**どちらも自前の抜けで、Rumoca の誤りではない**（v0.10.4 の確認は要らない。上流へも出さない）。変えたのは `globe-mode-dae`（状態の判定）と `globe-mode-flat`（方程式の数え方）。

- **状態 2 本の原因**: `--dump` の Rumoca の構造に、自前に無い未知数 `der(imc.is[1..3])`・`der(imc.idq_rs[1..2])`（`IMC_DCBraking`、計 5 本、18 → 23）と `der(aimc.idq_sr[1..2])`・`der(aimc.idq_rr[1..2])`（`IMC_Initialize`、計 4 本、18 → 22）があった。どれも、どの行にも参照されない未知数で、`initial equation` の `der(imc.is[2]) = 0`・`der(aimc.idq_sr) = zeros(2)` などの対象。Rumoca の `rumoca-phase-dae` の `analyze_model_roles` は `der` の対象を `flat.equations` と `flat.initial_equations` の両方から集める（コードを読んで確かめた。写してはいない）が、自前の `has_der` は連続側の方程式・`when`・algorithm・束縛だけを見ていた。変数ごと（配列は全要素）に状態にする点も同じ。
- **Rumoca の誤りとはしない理由**: [MLS 8.6（master）](https://specification.modelica.org/master/equations.html)（2026-11-15 確認）は、初期化では `der` と `pre` を未知数に数えると書くだけで、連続側で一度も微分されない変数が初期方程式の `der` だけで状態になるかは決めていない（例の `der(y) = 0` は連続側にも `der(y)` がある）。Rumoca の規則は意図して書かれたもの（初期方程式を明示的に走査する）なので、自前を合わせた。実際のツールは `imc.is[2]` を状態の別名とみなすと考えられるが【未確認】、Track B の目的は Rumoca の判定との一致なので、ここでは追わない。
- **直し（DAE）**: `has_der` に `m.initial_equations` の `der` の対象も足す（`globe-mode-dae` の `build`）。釣り合い（未知数と方程式の数）は変わらない（代数 → 状態で未知数の数は同じ。初期方程式は数えない）。構造の正則は、`der(x)` の未知数が行から参照されなくなるが Rumoca も同じで、334/334 のまま。
- **`ShowTransferFunction`（14 対 16）の原因**: `Modelica.ComplexBlocks.ComplexMath.TransferFunction` の `aw = {a[i]*… for i in 1:size(a,1)}`（`Complex aw[size(a,1)]`、3 本）。`Complex` の配列は要素ごとに展開され（`aw[k].re`・`aw[k].im`）、左辺の名前 `transferFunction.aw` は変数として無いので、`shape` が大きさを引けず、右辺も内包表記で引けず 1 本と数えていた（Rumoca は `transferFunction` の 7 本のうち 3 本）。`shape` の最後の手段に `record_array_len`（`t[` で始まる変数名の `t[k].x` の `k` の数。変数の表は整列しているので範囲だけをなめる）を足した。
- **試験**: `globe-mode-dae` に 1 本（`der_in_initial_equation_makes_state`）、`globe-mode-flat` に 1 本（`record_array_equation_counts_one_per_element`。直す前は 2、直して 4）。`cargo xtask diff-frontend --dump` に初期方程式の行 `ieq[…]` を足した。
- **shortcut**: 読めない初期方程式の印字は黙って飛ばす（状態の数だけが変わる）。属性（`start` など）の中の `der` は見ない（Rumoca は見る）。`if` の初期方程式は、条件が定数なら選んだ枝だけが `initial_equations` に入る（Rumoca も構造の選択で同じ）。実行（`globe-mode-exec`）は Rumoca のままなので、この規則は自前の DAE の判定にだけ効く。
- **数字**（MSL の例題 566 本、S334 → S337。`GlobeModeValidation` 75 本は変わらず 73/73・73/73・73/73・正則 72/72。回帰 0、改善 3 本）: 状態の数 332/334 → **334/334**、flat の方程式の数 451/463 → **452/463**（残り 11 本は Rumoca の数え方の誤りとみている。Rotational・Translational の摩擦・歯車の 10 本と `ActuatorWithNoise`）、DAE の判定・数・構造の正則は 334/334 のまま。`msl:diff-frontend` と同じ引数で 744 s（ほかの担当と並行。S334 は 554 s）、最大 RSS 5.2 GiB。
- **作業上の注意**: 共有 target では、同じ crate の成果物（`release/xtask`・試験のバイナリ）の名前が worktree をまたいで同じで、別の worktree のビルドに入れ替わることがある（`--dump` の出力が食い違って気付いた。試験でも起きた）。ビルド直後に自分の分を `cp` して走らせ、変えた crate の `lib.rs` を `touch` して作り直す。

## 付記（S355）: flat で `stream`（MLS 15）を展開する（`stream` 21 本 → 0、平らにできた 518 → 538）

新しい ADR は立てず、S335・S336 の続きとしてここに足す。変えたのは `globe-mode-flat`（`flatten.rs` と、新しい `flatten/stream.rs`）だけ。S336 の後に残った断る理由 `stream` の 21 本（MSL の例題 566 本の最初の理由。`Modelica.Fluid.Examples` の 20 本と `AST_BatchPlant.BatchPlant_StandardWater`）と、`GlobeModeValidation.Connect.MediumFlow` の 1 本を、MLS 15 の定義どおり（Rumoca の flat と同じ形）に平らにできるようにした。

- **接続集合の stream**（`gen_connects`）: `stream` 変数の葉の集合は、potential・flow と同じ union-find で作る。端は（葉の名前、外側のコネクタの端か）。`stream` の集合は等式にも和にもせず、`F::stream_sets` に残す。大きさ 0 の欄（`Xi_outflow[0]`）は端にしない（Rumoca の flat にも変数が無い）。外側のコネクタの端（そのモデル自身のコネクタを `connect` した端）には、端ごとに 1 本の方程式 `x = __outStream(x)` を置く（MLS 15.2。数は葉の大きさ。内側の端と、つながない内側のコネクタには作らない。Rumoca の `generate_outside_stream_equations` と同じ）。
- **展開**（`flatten/stream.rs` の `expand_streams`。`apply_connection_graph` のあと）: 方程式・`when`・初期方程式・`algorithm`・変数の束縛・start・属性の印字の中の `inStream(x)`・`actualStream(x)`・`__outStream(x)` を置き換える。`inStream(x)` は `x` が内側の端になっている集合の、`x` を除いた端の混合: 端が 0 なら `x` 自身（つながない内側のコネクタ。最上位のモデル自身のコネクタも）、1 つならその端の値、2 つ以上なら `((max(∓m_flow₁, ε)·値₁ + …) / (max(∓m_flow₁, ε) + …))`（内側の端は `-m_flow`、外側の端は `m_flow`。`ε` は集合の流量の nominal の最小の `1e-7` 倍）。外側の端の値は `inStream(その端)` で、1 段上の集合で決まる（階層の深さで止まる。64 段を超えたら断る）。`__outStream(x)` は、`x` が外側の端になっている集合の、`x` を除いた端の混合（`inStream(x)` ではない。Rumoca の `outside_equation_rhs` と同じ）。`actualStream(x)` は `(if m_flow > 0 then inStream(x) else x)`。流量が集合へ入る向きにならない端（内側で `min >= 0`、外側で `max <= 0` が数のリテラルで書いてある）は混合に入れない。大きさ 0 の欄への `inStream` は `{}`。
- **shortcut**: Rumoca の `smooth(0, m_flow * actualStream(x))`（流量の重みつきの積）は作らない（DAE の構造・数は変わらない）。添字が定数でない `inStream(ports[i].h_outflow)`、同じコネクタに流量が 1 つに決まらないコネクタ、`stream` 変数でない引数は断る（`stream`。MSL の例題では出ない）。Rumoca は流量を `normalized_parent` でも探すが、自前は同じ親（コネクタ）だけ。
- **同じ原因で止まって見えた 2 つの直し**（`stream` を通したあと、Fluid の例題で見つかった）:
  - `pending_bind`（宣言が後ろの部品の欄）: 既定値が同じ部品の別の欄そのもの（`System.massDynamics = energyDynamics`）なら、その欄の修飾か既定値にたどる。`Examples` の `inner system(energyDynamics = …)` は部品より後ろに宣言されるので、`tank.massDynamics == Dynamics.FixedInitial` の `if` が評価できず、初期方程式の全部の枝（`der(medium.p) = 0` も）が残って、状態の数が 8 本（Rumoca は 4 本）になっていた。たどると枝が選ばれて状態も合う。
  - `count_eq`（`if`・`for` の中の `lhs = rhs` の数）: 入れ子でない方程式と同じ数え方（`eq_count`。`shape` のあと `dims_of` の積）にする。`for i in 1:n loop zeros(Medium.nXi) = mbXi_flows[i,:]; end for` の `nXi = 0` が、左辺が呼び出しで `shape` に引けず 1 本と数えられていた（`IncompressibleFluidNetwork` で +44）。
- **試験**: `globe-mode-flat/tests/flat.rs` の `stream_sets_and_in_stream`（外側の端 `w.a` の `a.h = 内側 2 つの混合`、内側の端 `n.p` の `inStream` が 1 段上の端の値、外側の端を通る入れ子の混合、つながない内側の端は自分）。`declined_forms` の `stream` の断りは外した。
- **Rumoca の flat との突き合わせ**（MSL の例題 566 本、`mise run msl:diff-frontend` と同じ `diff-frontend`。S335 の結果〔`unsized-array` を直したあと〕との差）: 自前で平らにできた **518 → 538**（`stream` 21 → 0。`BatchPlant_StandardWater` は `connect` に移った）、Rumoca の flat まで通った 491 → **511** 本で名前の集合 491/491 → **508/511**・方程式の数 480/491 → **498/511**、DAE にできた **538/538**。Rumoca の釣り合いまで通った 351 → **367** 本で、判定 **365/367**・方程式と未知数の数 **364/367**・状態の数 **366/367**・構造の正則 **363/367**。`GlobeModeValidation` 75 本は 74 → **75** 本が平らになり（`MediumFlow`）、Rumoca も通った 74 本ですべて一致（名前・方程式の数・判定・数・状態 74/74、正則 73/73）。**回帰 0 本、改善 21 本**。1383 s、最大 RSS 5.8 GiB。
- **新しく比べられた 20 本のうち、合わない項目の原因**（`diff-frontend --class … --dump` で確かめた）:
  - 名前 3 本（`InverseParameterization`・`RoomCO2`・`RoomCO2WithControls`）: `pipe.flowModel.Res`・`m_flows_turbulent`・`dps_fg_turbulent` が自前にだけある。`redeclare model FlowModel = DetailedPipeFlow(show_Res = true)` で条件つき宣言 `Res = … if show_Res` の条件を真にする修飾を、**Rumoca は条件の評価に使わない**。`GlobeModeValidation` に似た小さなモデル（`replaceable model M = Impl; M m;` と `Holder h(redeclare model M = Impl(show = true))`、`Real y = 2 if show;`。scratchpad に置いた。コミットしない）で、自前は `h.m.y` を作り、Rumoca は作らないことを確かめた。条件つき宣言の条件は、修飾を当てたあとの値で決まるはずなので、自前が正しいとみている（仕様の節は確かめていない）。**上流の issue の候補**（[upstream.md](../upstream.md) に足していない。提出しない）。`InverseParameterization` の DAE の数 287 対 279 も同じ原因。
  - 方程式の数 2 本（`BranchingDynamicPipes` +4・`IncompressibleFluidNetwork` +1）: 部品の配列 `heat2[5]`・`heat8[2]`（`PrescribedHeatFlow`）の方程式を、Rumoca は 1 本と数える（`equation from heat2` が 1 本。ほかの 4 本ぶんが足りない）。自前は要素ごとに 5・2 本と数える。S330 が確かめた Rumoca の数え方の誤りと同じ種類（部品の配列の方程式）。
  - DAE の釣り合い 2・数 3・状態 1・正則 4（`HeatingSystem`・`NonCircularPipes`・`ThreeTanks`・`TankWithEmptyingPipe1`・`InverseParameterization`）: 行の incidence を突き合わせると、食い違いはどれも `Medium.ThermodynamicState` のレコード（`state_a = Medium.setState_phX(…)`、`statesFM[1:n] = mediums[1:n].state`、`pipe.flowModel.states[1].p = …`）の方程式と束縛で、Rumoca は欄ごとの行にし、自前は 1 行にしている【推測。`stream` の展開そのものの差ではない】。**持ち越し**。
- **残る 1 本**: `BatchPlant_StandardWater` は `connect` で断る（`StateGraph` の `outer` のコネクタ `stateGraphRoot.subgraphStatePort` への `connect` で、`inner` が後ろで宣言されていて葉がまだ無い。`StateGraph.Examples` の 7 本と `Fluid.Examples.ControlledTankSystem.ControlledTanks` も同じ。10 本目の `LogicalSample` は大きさの違う配列の `connect`）。
- **持ち越し**: DAE のレコードの欄ごとの行（上の 4 本）、`outer` のコネクタを後ろの `inner` に `connect` する 9 本、`smooth(0, m_flow * actualStream(x))`。

## 付記（S361）: flat で外側の階層の `inner` への `connect`・`inner outer`・`cardinality` の `if`（`connect` 10 本 → 0、平らにできた 538 → 548）

新しい ADR は立てず、S355 の続きとしてここに足す。変えたのは `globe-mode-flat`（`flatten.rs`）の中だけ（`globe-mode-dae` は変えていない）。S355 の後に断る理由 `connect` で残った 10 本（MSL の例題 566 本の最初の理由。`StateGraph.Examples` の 7 本・`Fluid.Examples.ControlledTankSystem.ControlledTanks`・`AST_BatchPlant.BatchPlant_StandardWater` の 9 本は `StateGraph` の `outer` のコネクタ `stateGraphRoot.subgraphStatePort` への `connect`、残る 1 本は `Clocked.Examples.Elementary.ClockSignals.LogicalSample`）を平らにできるようにした。

- **外側の階層の `inner` につなぐ `connect` は、`inner` を宣言する階層まで持ち越す**（`late_conns`・`late_owner`）。`outer` の部品は参照が外側の `inner` の名前に付け替わるので、`step.outerStatePort` の `connect(outerStatePort.subgraphStatePort, stateGraphRoot.subgraphStatePort)` の端は `stateGraphRoot.subgraphStatePort`（例題のモデルの `inner`）になる。これまでは `step` の階層の `gen_connects` が単独の集合にしていて、(1) `inner` が部品より後ろで宣言されていると葉がまだ無く（`connect: … differ`）、(2) 前でも、部品ごとに別の集合になって `inner` のコネクタの flow の和が部品の数だけできる（MLS 9.2 の接続集合は階層をまたいで 1 つ。Rumoca も全部の `connect` を集めてから集合にする）。端が、`pfx` の祖先の階層で宣言した `inner`（`F::inners`）の中を指していたら、その `inner` を宣言する階層（`gen_connects` に `pfx` を渡して `late_conns` を引く）の集合に入れる。持ち越す前の階層で各端が「外側のコネクタ」だったか（flow の和の符号が逆になる）は一緒に持ち越す。持ち越した端は、元の階層の「つながない内側のコネクタの flow = 0」の対象にしない。宣言する階層に届かなかったら（`inner` が作られなかった）`connect` で断る。`FirstExample` の flow の和は 1 本 `initialStep….activeSteps + stateGraphRoot….activeSteps + step….activeSteps = 0`（Rumoca と同じ）。
- **`inner outer` は実体を持つ**（`bridge`・`bridge_end`）。`CompositeStep` の `inner outer CompositeStepState stateGraphRoot(suspend = …, resume = …)` は、これまで `outer` と同じく外側の `inner` の別名にして部品を作らなかったが、Rumoca の flat は `compositeStep.stateGraphRoot.*`（`suspend`・`resume`・`subgraphStatePort` の欄・`activeSteps` の 7 変数と、`CompositeStepState` の 4 本の方程式）を作る。実体を作り（中の部品の `outer` は一番近いこの `inner outer` につながる）、`connect` の端のうち**同じ階層**でこの名前で始まるものだけ、外側の `inner` の名前に付け替える（MLS 5.4。Rumoca の `redirect_connection_for_inner_outer` と同じ。子の階層の端は付け替えない）。MSL で `inner outer` を使うのは `StateGraph.PartialCompositeStep` の 1 か所だけ。
- **`cardinality(c)` を条件に持つ `if` の方程式は、全部の `connect` がそろってから枝を選ぶ**（`defer_card_if`・`resolve_card_ifs`）。`Step` の `if cardinality(inPort[i]) == 0 then inPort[i].set = false; end if;` は、`inPort[i]` が親の階層で `connect` されるので、平らにしている間は値が決まらない。枝ごとに平らにして 1 行の目印を置いておき、`apply_connection_graph` のあと（`expand_streams` の前）に、`c` が `connect` の端に出てきた数（MLS 3.7.2.3。`conn_pairs` の端の完全な名前の出現の数。Rumoca の `compute_cardinality_counts` と同じ）で条件を評価して、真になる最初の枝だけを置き換える。評価できなければ `cardinality` で断る。入れ子は断る。`assert(cardinality(…) …)` は数 0 のまま。
- **差し替えた部品に、元の宣言の `constrainedby C(x = …)` の修飾も掛ける**（`LogicalSample`）。`PartialLogicalClock` の `replaceable PartialBooleanMISO combinator constrainedby PartialBooleanMISO(nu = nu)` を `redeclare MathBoolean.And combinator` で差し替えると、`nu = nu` が落ちて `combinator.u[nu]` が大きさ 0 になり、`connect(input_memory.Q, combinator.u)` が「大きさが違う」で断られていた（MLS 7.3.2。差し替えた側の明示した修飾が勝つ）。差し替えていない既定の型には掛けない（数字が動くか確かめていない。Rumoca の側も未確認）。
- **内包の中の `inStream`・`actualStream` は添字ごとに展開する**（`pr`）。`BatchPlant_StandardWater` は `connect` を通ると `stream: inStream(multiPort.ports_b[j].h_outflow): not a variable` で止まった（S355 の「添字が定数でない `inStream` は断る」。`Fittings.MultiPort` の `sum({positiveMax(ports_b[j].m_flow) * inStream(ports_b[j].h_outflow) for j in 1:nPorts_b})`）。印字のとき、本体に `inStream`・`actualStream` を含む内包 `e for j in r` は、`r` の値ごとに `j` を束ねて `e` を並べる（`{e(1), e(2), …}`）。添字が 1 つの内包だけ（複数は `stream` で断る。MSL に無い）。
- **試験**（`globe-mode-flat/tests/flat.rs`）: `outer_connect_to_later_inner`（後ろで宣言された `inner` に 2 つの部品がつなぐと flow の和が 1 本）、`inner_outer_is_instantiated_and_bridged`（`inner outer` の実体・葉は実体につながり、同じ階層の `connect` は外側の `inner` へ・外側のコネクタの端の符号が持ち越しても逆）、`cardinality_selects_if_branch`（未接続だけ枝が選ばれる・`<`・`else`）、`redeclare_keeps_constraining_modification`（差し替えた部品に `nu = nu` が掛かり、明示した修飾が勝つ）、`in_stream_in_comprehension`。
- **Rumoca の flat との突き合わせ**（MSL の例題 566 本、`mise run msl:diff-frontend` と同じ `diff-frontend`。S355 の結果との差）: 自前で平らにできた **538 → 548**（`connect` 10 → 0）、Rumoca の flat まで通った 511 → **521** 本で名前の集合 508/511 → **518/521**・方程式の数 498/511 → **508/521**、DAE にできた **548/548**。Rumoca の釣り合いまで通った 367 → **377** 本で判定 **374/377**・方程式と未知数の数 **373/377**・状態の数 **376/377**・構造の正則 **369/377**。`GlobeModeValidation` 75 本は変わらず（Rumoca も通った 74 本ですべて一致）。**回帰 0 本、改善 10 本**。1009 s、最大 RSS 5.8 GiB。
- **新しく通った 10 本のうち合わない項目**: 名前・方程式の数は 10 本とも一致。DAE は 6 本（`LogicalSample`・`FirstExample`・`_Variant2`・`_Variant3`・`StateGraph.Examples.ControlledTanks`・`Fluid.Examples.ControlledTankSystem.ControlledTanks`）が 4 つとも一致し、4 本が食い違う。
  - 構造の正則（`ExecutionPaths`・`ShowCompositeStep`・`ShowExceptions`。自前 false、Rumoca true）: `Alternative`・`Parallel` の `join.occupied = fill(outPort.occupied, nBranches)`（flat の印字は `{a.join[1].occupied, a.join[2].occupied} = fill(…)`、数 2）の行の incidence が、自前の DAE では空になる（`ExecutionPaths` で 7 行。Rumoca の行は 40、自前は 47）。unknown のない行は構造の正則を落とすので、これが原因とみているが、行が空になる理由は `globe-mode-dae` の側で**確かめていない**。potential の等式を、自前は集合の先頭を中心にした星形、Rumoca は木にする違いも出るが、行の incidence の違いはこの 1 種類だけ。
  - `BatchPlant_StandardWater`（方程式の数 2365 対 2389、構造の正則）: S355 の「DAE のレコードの欄ごとの行」と同じ種類【推測】（`Medium.ThermodynamicState` のレコードの方程式。差は 24 本）。
- **残る断る理由**（18 本）: `composite-binding` 6・`expandable` 5・`non-constant-dim` 4・`connect-array` 1（`HeatExchangerSimulation`。`heatPorts[HEX.nNodes : -1 : 1]`）・`non-constant-for` 1・`unsized-array` 1。
- **持ち越し**: 上の DAE の 4 本（`globe-mode-dae` の側）、`smooth(0, m_flow * actualStream(x))`、差し替えていない既定の型への `constrainedby` の修飾、`cardinality` の入れ子。

## 付記（S336）: B3 の続き、`Complex` の配列の束縛を要素ごとに（`composite-binding` 25 → 0、平らにできた 471 → 477）

新しい ADR は立てず、S329 の続きとしてここに足す。変えたのは `globe-mode-flat`（`flatten.rs`）の中と、`xtask diff-frontend --dump`（自前が断ったときも Rumoca の flat を印字する）だけ。S334 の後に残った断る理由 `composite-binding` の 25 本（MSL の例題 566 本。最初の理由）を、原因ごとに分けて減らした。

- **25 本の内訳**（`diff-frontend --class … --dump` の理由。すべて `Complex`・レコードの**配列**の束縛）: (1) `Complex uInternal[nin] = {if useConjugateInput[k] then conj(u[k]) else u[k] for k in 1:nin}`（`ComplexBlocks.Interfaces.ComplexMISO`・`ComplexMIMO`。8 本）、(2) `SI.ComplexVoltage v[m] = plug_p.pin.v - plug_n.pin.v`（配列の式。`vs`・`v`。15 本）、(3) `Permeance strayPermeance[m](final G_m = {SalientPermeance(…) for k in 1:m})`（レコードの配列を内包で修飾。1 本）、(4) `Complex sTM[m, m] = symmetricTransformationMatrix(m)`（2 次元で、関数の戻り値。1 本）。(1) の先には `Complex k[nin] = fill(Complex(1, 0), nin)` が隠れていた。
- **配列のレコードの束縛を、添字 `[i, j, …]` を付けた要素ごとの束縛にする**（`inst_decl` の `bound`。1 次元だけだったのを N 次元に）。要素の式は `pick_elem` が決める: 束縛の式が `{a, b}` なら項、`{e for k in 1:n}` なら本体 `e`（`k` はその要素の値を局所変数に持つ文脈 `Ctx::locals` で。`for` の方程式と同じ仕組み）、それ以外（別の配列の変数・関数の呼び出し）は式のまま `[i, j]` を付ける（`(f(m))[2,1].re`）。部品の配列への修飾 `strayPermeance[m](G_m = {… for k in 1:m})`（`split_mod` が `({…})[k]` にしたもの）も同じ道で解ける。大きさの検査（項の数・範囲の長さ）は `size differs` で断る。
- **`Complex` の配列の式を要素ごとに**（`complex_parts` に、束縛に付いた `[k]` を葉に当てる `index_leaf` を足した）: 配列の変数 `u`（`u[k].re`）・部品の配列を通した欄 `plug_p.pin.v`（`plug_p.pin[k].v.re`。`array_elems` と同じく最初の `.` から順に試す）は `[k]` を挟み、スカラーはそのまま（全要素で同じ）。`fill(x, n)` の要素は `x`。`Complex(re, im)`・`if`・`+ - * /` は S329 のまま。`complex_value` に切り出して、組み込みの関数（`fill`）の呼び出しの束縛からも呼ぶ。
- **部品の配列を通した欄の束縛 `Complex i[m] = plug_p.pin.i`** は、欄ごとの束縛の名前を `plug_p.pin[k].i.re` にする（これまでは `plug_p.pin.i[k].re` という存在しない名前になりうる形。名前そのままの式 `Bind::lit` で束縛する）。
- **内包 `{e for i in r}` の方程式の数**（`shape` の `Expr::Array`、`comp_len`）: 範囲の長さの積。これまでは項が 1 つとして 1 本と数えていた。Rumoca は `c = {a[k]*b[k] for k in 1:3}`（`Complex c[3]`）を 3 本と数える（`GlobeModeValidation` に置いて確かめた）。`Complex` の配列の方程式も Rumoca は要素の数で数える（`c = a - b` は 3 本。実部・虚部の 6 本ではない）。S337 が左辺が要素ごとに展開したレコードの配列（`aw = {…}`）の場合を同じ数え方にした（`record_array_len`）ので、この分は左辺が式の場合（`plug_p.pin.i + plug_n.pin.i = {Complex(0, 0) for k in 1:m}`。`PolyphaseElectroMagneticConverter`）だけが残った。
- **試験**: `globe-mode-flat` に 2 本（`complex_arrays_expand_elementwise`〔内包・部品の配列を通した欄の差と参照・関数の戻り値の 2 次元・`fill`・レコードの配列の内包の修飾〕、`comprehension_equation_counts_its_range`〔左辺が式の内包の方程式〕）。
- **shortcut**: 内包は添字が 1 つのものだけ（`{… for i in a, j in b}` は断る。MSL の例題では出ない）。`index_leaf` は葉の名前から配列を探すので、`Complex` を返す関数の配列の引数（`conj(u)` の `u` が配列）は見ない（断る）。要素の式の中の条件 `if c then …` の `c` が配列のときも見ない（MSL の例題では `useConjugateInput[k]` と添字つき）。
- **S335（`unsized-array`）との関係**: 25 本のうち 19 本（`indexNonPos[:] = f(m)` の `Modelica.Electrical.Polyphase.Functions.indexNonPositiveSequence` を持つ Machines の例題）は、`composite-binding` の先で `unsized-array` で止まる。S335 の WIP を仮に結合して（コミットせず）確かめると 25 本すべてが平らになった（Rumoca の flat まで通ったのは 7 本で、名前の集合は 7/7）。そのとき、部品の配列を通した欄の方程式 `plug_p.pin.i + plug_n.pin.i = fill(Complex(0), m)` の数が 3 から 1 に減る問題が見つかった（S335 の `var_dims` の新しい分岐が `pin.i` に `Some([])` を返していた）。S335 は同じ原因を自分の WIP で直していた（宣言が後ろの部品の欄は、まだ平らにしていない配列でない部品だけにする）。
- **Rumoca の flat との突き合わせ**（MSL の例題 566 本、`mise run msl:diff-frontend` と同じ `diff-frontend` を回した。S337 と S341 を取り込んだ後。S336 の前は S337 の 471 本・452/463。`GlobeModeValidation` 75 本は変わらず）: 自前で平らにできた **471 → 477**（`composite-binding` の 25 本のうち 6 本。ほかの 19 本は `unsized-array` に移った）、Rumoca の flat まで通った 463 → **468** 本で名前の集合 463/463 → **468/468**・方程式の数 452/463 → **457/468**（新しく比べられた 5 本が一致。残り 11 本は S337 から変わらず、Rumoca の数え方の誤りとみている）、DAE にできた 471 → **477/477**。Rumoca の釣り合いまで通った本数は 334 のまま（新しい 6 本は Rumoca が釣り合いまで通らない）で、判定・方程式と未知数の数・構造の正則・状態の数は 334/334。**回帰 0 本**（S334 の結果との差は、取り込む前の実行で改善 7 本〔新しく平らになった 6 本と `ShowTransferFunction`〕、1348 s、最大 RSS 5.4 GiB。S337・S341 を取り込んだあとの再実行は、取り込む前の結果との差で改善 100 本〔S341 の resolve 99 本 + `GlobeModeValidation` の 1 本。うち 2 本は S337 の状態の数〕）。取り込んだあとの実行は 794 s、最大 RSS 5.6 GiB。続けて S339 も取り込んだ最後の実行（`flatten.rs` の衝突は、S339 の `is_msl_complex` を `complex_value` でも使う形で解いた）は、数字が同じで回帰 0・改善 0、671 s、最大 RSS 5.5 GiB。
- **断った理由の内訳**（最初の理由。S334 → S336）: `composite-binding` 25 → **0**、`unsized-array` 28 → **47**（19 本が移った）、`stream` 21、`connect` 9、`non-constant-dim` 6、`expandable` 4、`connect-array` 1、`non-constant-for` 1。
- **持ち越し**: `unsized-array` 47 本（S335）、内包の添字が 2 つ以上・`Complex` を返す関数に配列を渡す式、S335 と合わせた 25 本のうち Rumoca が flat まで通るのは 7 本だけで（上の仮の結合）、残り 18 本は Rumoca と名前・方程式の数を比べられない（Rumoca の側の限界）。

## 付記（S335）: flat で `[:]` の大きさを束縛の式から決める（`unsized-array` 28 本）

新しい ADR は立てず、S334 の時点で flat が `unsized-array` で断っていた MSL の例題 28 本を、原因ごとに分けて直した記録をここに足す。変えたのは `globe-mode-flat`（`flatten.rs` の `dims_of`・`decl_dims` まわりと、`eval` の一部）だけ。

- **28 本の内訳**（`--dump` の最初の理由）: (1) `BooleanTable` の `table = if n > 0 then … [table[1], 1.0; table, {mod(i + 1, 2.0) for i in 1:n}] …`（12 本。`BooleanNetwork1`・`Interaction1`・`LogicalNetwork1`・`ShowLogicalSources`・`Clocked` の `BooleanSignals` 6 本・`CharacteristicThyristors`・`SoftStarter`）、(2) `TransferFunction` の `bb = vector([zeros(max(0,na-nb),1);b])`（6 本。`Dimmer`・`ThyristorBridge…_DC_Drive` ほか）、(3) Batteries の `OCV_SOC`・`OCV_SOC_internal`（`cellData = cellData1` のようにレコードを別のレコードで束縛した欄。7 本）、(4) 関数の戻り値（`relNodePositions(12)` の `xsi[nNodes]`、`GenerateStribeckFrictionTable(… nTable = 50)` の `table[nTable, 2]`。2 本）、(5) `ReadRealMatrixFromFile`（1 本）。
- **直し**: `dims_of`（式の配列の形）に次を足した。
  - 添字つきの参照 `table[1]`・`x[:, 2]`（スカラーの添字は次元を落とし、`:` とベクトルの添字は残す。`indexed_dims`）、内包 `{e for i in 1:n}`（範囲の長さのあとに `e` の形）、`vector(A)`（要素の数の 1 次元）、要素ごとの `mod`・`rem`・`div`・`floor`・`ceil`・`integer`・`sign`、`end`・`for` の添字（スカラー）。
  - 定数に決まる `if` の式は選ばれた枝（`const_if_expr`）。決まらないときは最初の枝だが、ほかの枝の形が分かって食い違うなら決めない（以前は最初の枝だけを見ていた）。
  - 定数でない `parameter`（型の `extends` の修飾も合わせた `pkg_env` にだけある欄）の束縛の形。
  - 関数の戻り値は、整数の入力を呼び出しの引数（名前つきも位置も、無ければ入力の既定値）の値に束ねた文脈（`fn_frame`。`Ctx::pfx` が `FN`）で出力の添字を評価する。実数の入力は束ねない（`output Real y[n]` の `n` は整数）。
  - レコードを `r = other` で束縛して欄に分けたもの（`Bind::suffix` が `.f`）の大きさと値は、`other.f` 自身の束縛（`field_bind`。`other` が平らになっていれば環境から）。以前は suffix を見ず、レコード `other` の大きさ（空）を欄の大きさにして `unsized-array` で断っていた。`eval` も同じ引き方をするので、`nRC = cellData.nRC` のような欄の値が評価できる。
  - 宣言が後ろの部品の欄（`reset = {stop.on}` の `stop.on`、`cellData` の `OCV_SOC_internal`）は、部品の型の宣言から（`pending_bind`・`pkg_field_dims`）。**宣言の場所で修飾された欄を、その宣言か既定値の式が直接使うときは決めない**（この文脈は修飾を知らないので、既定値で大きさを決めると黙って誤る）。
- **決められなかったもの**: `ReadRealMatrixFromFile` の `A[:,:] = readRealMatrix(file, matrixName, dim[1], dim[2])` は、`dim` が外部関数 `readMatrixSize` でファイル（`.mat`）を読んで決まるので、ファイルを読まない flat では決められない（Rumoca がこの例題を通すかは確かめていない【未確認】）。
- **試験**: `globe-mode-flat` に 1 本（`unsized_dims_from_comprehension_function_args_and_record_fields`。添字つきの参照・内包・`vector`・関数の整数の引数と既定値・宣言が後ろの部品の欄・レコードの束縛の欄、と、修飾された欄を使う大きさは断ること）。
- **shortcut**: 関数の戻り値の大きさは出力が 1 つのときだけ。欄の型の添字（`type Axis = Real[3]`）は宣言が後ろの部品の欄には足さない。修飾の影響は 1 段だけ見る（別の欄を通る参照は見ない）。
- **数字**: MSL の例題 566 本。**S336 を取り込む前**（S334 → S335。S337・S341 は取り込み済み。前回の JSON は S334 のもの）: 609 s、最大 RSS 5.5 GiB、回帰 0、改善 120 本。flat で自前で平らにできた 471 → 493（`unsized-array` の 28 本は 1 本〔`ReadRealMatrixFromFile`〕に。`IMC_withLosses` 2 本は `eval` が suffix の欄を評価するようになって `non-constant-dim` から通った）、Rumoca の flat まで通った 463 → 484 本で変数の名前 463/463 → 484/484、方程式の数 452/463 → 473/484（合わない 11 本は Rumoca の数え方の誤り）、DAE は 471 → 493/493、Rumoca の釣り合いまで通った 334 → 351 本で判定・数・状態・構造の正則がすべて 351/351。**S336 を取り込んだ後**（S336 の JSON〔S339・S351 を含む〕と比べて）: **883 s、最大 RSS 5.8 GiB、回帰 0、改善 41 本**。flat で**自前で平らにできた 477 → 518**（S336 が `composite-binding` の先で止めていた `unsized-array` の 19 本を含む。`composite-binding` は 6 本〔`rcData[] = cellData*` の 5 本と `CCCV_Stack`。配列のレコードを別のレコードの欄の配列で束縛する形〕に戻った）、Rumoca の flat まで通った 468 → 491 本で変数の名前 **468/468 → 491/491**、方程式の数 457/468 → **480/491**（合わない 11 本は同じ 11 本）、DAE は 477 → **518/518**、Rumoca の釣り合いまで通った 334 → 351 本で判定・数・状態・構造の正則がすべて **351/351**。`GlobeModeValidation` 75 本は変わらない。S336 の担当が気にしていた `plug_p.pin.i + plug_n.pin.i = fill(Complex(0), m)` の数が 3 → 1 に減る問題は、合わせても起きていない（Machines の 50 本は方程式の数が一致したまま）。最初の全体の検査は、`var_dims` の補助が部品の配列の欄 `plug_p.pin.i` を要素の数をかけずに 1 本と数え、Machines と FundamentalWave の 52 本で方程式の数が Rumoca より少なくなる回帰を出した（`not_instantiated` で直した。`SoftStarter` の 8 本の差も同じ原因だった）。

## 付記（S334）: 構造の正則の不一致 33 本と `PrismaticConstraint`・`Fourbar1` の方程式の数（MultiBody）

新しい ADR は立てず、S333 の持ち越しの原因を `cargo xtask diff-frontend --class … --dump` で確かめて直した記録をここに足す。**どちらも自前の誤りで、Rumoca は正しかった**（Rumoca 側の誤りではないので、v0.10.4 の確認は要らない）。変えたのは `globe-mode-dae`（incidence）と `globe-mode-flat`（`flatten.rs`・`flatten/graph.rs`）。

- **構造の正則 33 本の原因**: 正則の判定は `incidence ∪ state_refs` の最大マッチングが完全かどうか（`globe-mode-analysis`）。`--dump` の行を自前と Rumoca で突き合わせると、`Parts.Body` の `w_a = Frames.angularVelocity2(frame_a.R)` の行で、Rumoca は関数の本体を見て `frame_a.R.w[i]` を行の未知数に入れるのに、自前は引数のレコード `frame_a.R`（変数ではなく欄 `T`・`w` の持ち主）が名前として引けず、incidence が空だった。`frame_a.R.w[1..3]` を `from_Q` の 12 行だけで取り合う形になり、`der(Q[3])`・`der(Q[4])` ほか 3 つの未知数がマッチしなくなっていた（最小の形は `FreeBody`。Body と世界だけで再現する）。
- **直し（DAE）**: 変数でも部品の配列を通した欄でもないレコードの参照 `a.R` は、欄すべての変数（`a.R.T`・`a.R.w`）に触れるとみなす（`Ctx::leaves_of`。左辺のレコードを欄ごとに数える `record_leaves` と同じ引き方）。Rumoca は関数の本体を見て欄を絞るので、自前の incidence は広めになる（S290 の shortcut の延長。釣り合いの数は変わらず、正則は甘く出る方に傾く）。
- **`PrismaticConstraint`（2 本差）の原因**: `Frames.angularVelocity2(R_b) = Frames.resolve2(R_b, …) + w_rel_b`（`Joints.Internal.InitAngularVelocity`）は 3 本の方程式だが、左辺が関数の呼び出しで大きさが分からず、1 本と数えていた。`dims_of` にユーザー定義の関数の呼び出しを足した（`fn_out_dims`。出力が 1 つで、宣言の添字が定数のとき、その大きさ。スカラーやレコードの出力は今までどおり `None`）。
- **`Fourbar1`（3 本差）の原因**: 閉路を作る `connect`（`Orientation` を持つコネクタの輪）は、木に選ばれなかった辺をまたぐ R の等式（`T` 9 本・`w` 3 本）が `zeros(3) = Orientation.equalityConstraint(a.R, b.R)` に置き換わる。S312 は戻り値の大きさを 0 と見て等式を作らなかった（`QuasiStatic` の `Reference.equalityConstraint` は `residue[0]` なので正しい）。`oc_nodes` に関数の完全修飾名と戻り値の大きさ（`residue[n]`）を持たせ、`graph.rs` が木に選ばれなかった任意の辺ごとに、`n > 0` なら `zeros(n) = f(a, b)` を `n` 本として足す（`PrismaticConstraint` の `Joints.Constraints.Prismatic` の明示の `zeros(3) = equalityConstraint(frame_a.R, frame_b.R)` は、書いてある方程式なので以前から数えていた）。
- **試験**: `globe-mode-dae` に 1 本（`whole_record_argument_touches_every_field`）、`globe-mode-flat` に 2 本（`broken_edge_gets_equality_constraint_of_its_residue_size`〔`residue[0]` と `[3]` で 3 本差。0 なら作らない〕、`user_function_with_array_output_gives_equation_size`）。
- **shortcut**: 関数の戻り値の大きさは出力が 1 つで添字が定数のときだけ（`size(x, 1)` などは今までの道）。レコードを関数に渡す incidence は欄すべて（Rumoca は絞る）。木に選ばれなかった辺のうち、同じ節点の対の `connect` が複数あるものは 1 本（`seen` の重複の除去）。
- **数字**（MSL の例題 566 本、S333 → S334。`GlobeModeValidation` 75 本は変わらず 73/73・73/73・73/73・正則 72/72。回帰 0）: flat の方程式の数 449/463 → **451/463**（残り 12 本は Rumoca の数え方の誤りとみている。Rotational・Translational の摩擦・歯車の 10 本は S331 で v0.10.4 では直ると確かめた。`ActuatorWithNoise` は S330 と同じ表の数え方。`ShowTransferFunction` の 14 対 16 は【原因未確認】）、MultiBody を含む DAE の判定・方程式と未知数の数 332/334 → **334/334**、状態 332/334 → **332/334**（残りの 2 本は `IMC_DCBraking`・`IMC_Initialize`〔Electrical.Machines。MultiBody ではない〕）、構造の正則 301/334 → **334/334**。`mise run msl:diff-frontend` は 554 s、最大 RSS 5.8 GiB。前回の JSON は S332 のものなので、差の回帰 0・改善 39 本は S333 の分を含む。

## 付記（S333）: `Connections.isRoot`・`rooted` の根の決定（MultiBody の DAE の不一致）

新しい ADR は立てず、S332 が特定した MultiBody の DAE の不一致（`Parts.Body` の `if not Connections.isRoot(frame_a.R) … elseif … else …`）を直した記録をここに足す。変えたのは `globe-mode-flat`（`flatten.rs`・`flatten/graph.rs`）だけで、`globe-mode-dae`・`globe-mode-syntax` は変えていない。

- **仕様**: [MLS 9.4（master）](https://specification.modelica.org/master/connectors-and-connections.html)（2026-11-15 確認）。確定した根（`Connections.root`）は 1 つの全域木の根、確定した根が無い部分グラフは `potentialRoot` の優先度の数が一番小さいものが根（Section 9.4.2.2）。`isRoot(R)` は R が根に選ばれたとき真、`rooted(R)` は `branch` で R とつながるもう一方の節点より R が根に近いとき真（Section 9.4.1）。木の選び方は MLS が決めない。
- **直し**（Rumoca の `rumoca-phase-flatten/src/vcg.rs` の `build_vcg` と同じ段取りを、自前の `graph.rs` に足した。コードは写していない）:
  - `if` の方程式の条件が `[not] Connections.isRoot(N)`・`[not] Connections.rooted(N)` を含むとき、`flatten.rs` の `defer_root_if` が枝ごとに方程式を平らにして `graph::OcIf` に置く（条件が定数の枝は先に決める。`connect` を含む・ほかの条件が混ざるときは今までの道）。置き場所には 1 行の仮の行を入れておく。
  - 全部の `connect` を集め終えた `apply_connection_graph` が根を決め、`OcIf` ごとに最初に条件が真の枝（`else` なら常に真）の方程式で仮の行を置き換える。`initial equation` の `if` も同じ。節点が過剰決定の接続の外なら Rumoca の既定（`isRoot` 真・`rooted` 偽）。
  - 根は、確定した根、無ければ潜在的な根（優先度が小さく、同じなら名前が先）、無ければ名前が先の節点。`rooted(a)` は `branch(a, b)` の `a` の木の上の深さが `b` より小さいこと（同じ `a` の枝が複数なら最後のもの）。木は、必須の辺（`branch`）に、根（確定した根、無ければ選んだ根）からの距離が近い任意の辺（`connect`）から順に足して作る（Rumoca の辺の順と同じ。S312 は `connect` の書いた順だった）。
  - S312 の `if Connections.isRoot(N) then … end if`（`else` なし）の文字による判定は、この道に統合した。
- **見つけて直したもの**: `a * b`（ベクトルどうしの積）の方程式を 3 本と数えていた。`dims_of` の `*` を行列の積の形にし（ベクトル × ベクトルはスカラー、行列 × ベクトルは行の数）、`.*` は要素ごとのまま。これは `Joints.Constraints.Universal` の `frame_a.t * n_a = 0` で、`if` を枝の和集合で数えていたときは別の誤差と打ち消し合って一致して見えていた（最初の計測で `UniversalConstraint` が回帰に出て気づいた）。
- **数字**（MSL の例題 566 本、S332 → S333。`GlobeModeValidation` 75 本は変わらず 73/73・73/73・73/73・正則 72/72。回帰 0、改善 39 本）: flat の方程式の数 435/463 → **449/463**（残り 14 本は Rotational と `ActuatorWithNoise` など Rumoca の数え方の誤りの 12 本と、`PrismaticConstraint`〔2 本差〕・`Fourbar1`〔3 本差〕）。MultiBody の 37 本（DAE が Rumoca の釣り合いまで通ったもの）の DAE の判定 297/334 → **332/334**、方程式と未知数の数 297/334 → **332/334**、状態の数 294/334 → **332/334**、構造の正則 296/334 → **301/334**。`EngineV6`（7392 対 7464）・`EngineV6_analytic`（6583 対 6631）の flat の方程式の数は Rumoca と一致（判定・数・状態も一致。正則だけ不一致）。`mise run msl:diff-frontend` は 519 s、最大 RSS 6.0 GiB（途中の 1 回目は回帰 1 本のため JSON を更新せず、直して回し直した）。
- **試験**: `globe-mode-flat` に 4 本（`is_root_if_with_elseif_keeps_only_the_chosen_branch`〔`Parts.Body` の最小の形。世界につなぐと非根、単独だと根、`initial equation` の `if` も〕、`rooted_follows_the_branch_direction_in_the_spanning_tree`〔世界 — 継手 — Body。向きを逆につなぐと `rooted` が偽〕、`potential_root_with_lower_priority_number_becomes_the_root`、`product_shapes_count_scalars`）。S312 の試験 `overconstrained_connections_drop_broken_edges_and_non_root_ifs` も通る。
- **shortcut**: 条件は `[not] isRoot(N)`・`[not] rooted(N)` の形だけ（`and`・`or` や、条件が複数の関数を含む形は今までの道）。枝の中に条件が `isRoot` の `if` を入れ子にした形は `nested-isroot-if` で断る（MSL には無い）。`rooted` は `branch` が複数あるときの MLS の制約（ちょうど 1 つ）を確かめない。大きさが 0 でない `equalityConstraint`（MultiBody の `Orientation`）を木の外の接続でどう扱うかは S312 のまま（等式を作らない。`Prismatic` の `zeros(3) = equalityConstraint(…)` の数え方は未確認）。
- **持ち越し**: 構造の正則の不一致 33 本（MultiBody。DAE の判定・数・状態は合うので、正則の判定の差【原因未確認】。`Joints.Universal` と世界と `BodyShape` だけの最小の形でも再現するが、`R.T` の 2 次元の添字の名前〔自前 `T[1]`・Rumoca `T[1,1]`〕など、行の比べだけでは分けられなかった）、`PrismaticConstraint`・`Fourbar1` の方程式の数の差（2・3 本）。

## 付記（S332）: S329 と S330 を合わせた計測、括弧の式への添字、名前の食い違い 37 本の原因（Rumoca が配列を展開しない条件）

新しい ADR は立てず、S329（`flat-arrays-complex`）と S330（`dae-mismatches-2`）を合わせて `mise run msl:diff-frontend` を回した記録をここに足す。変えたのは `globe-mode-syntax`（式の読み 1 か所）と `xtask diff-frontend`（比べ方と `--dump`）だけで、`globe-mode-flat`・`globe-mode-dae` は変えていない。

- **DAE が読めなかった 41 本**（S329 の申し送り）: `f(…).T` の 37 本は S330 の読みで直る見込みだったが、**実際は 37 本とも別の形**で、`f(…).T` ではなかった。S329 の flat が配列の部品の修飾を `0 = (({a + f(…) for i in 1 : n})[1])`（`cylinders[n](lengthDirection = {…})` の 1 要素目）と印字し、**括弧の式への添字 `(x)[1]`** を `globe-mode-syntax` が読めなかった（`expected )`）。MLS の文法の primary は `"(" output-expression-list ")" [ ( array-subscripts | "." IDENT ) ]`（[MLS の付録 A.2.7（master）](https://specification.modelica.org/master/modelica-concrete-syntax.html)、2026-11-15 確認。括弧のあとの `.IDENT` は読んでいない）。直し: `ParenExpr` の節の中に `[…]`（`Subscripts`）を取り込む（木の形・往復は変えない。`items()` は添字を飛ばす）。残りの 4 本（`if` の式・括弧の読み）は S330 までに別の形で直っていた。`dae-parse` は **0 本**になり、DAE にできたのが **430 → 471 / 471**。
- **名前の食い違い 37 本（`cylinders[n](…)`・`Complex aw[2]`）: Rumoca が配列を展開しない条件は、「次元の式を Rumoca の評価器が評価できないとき」**。Rumoca の `evaluate_array_dimensions`（`rumoca-eval-ast/src/eval_instantiate/function_eval.rs`、rev `276e82bd`）は、`shape_expr` を評価できなければ前もって計算した `shape` に戻り、それも空ならスカラー扱いにする。配列にならなかった部品の欄は、あとで `dim_recovery.rs` が親の次元を足して `cylinders.R.T`（次元つき）にする（コメントに「`Complex[3] aw` が展開されないとき（次元が instantiation で評価できない）」とある）。`Pin pin[m]`（`m` は整数の引数）は評価できるので展開する。
  - 再現（`diff-frontend --validation <dir> --class V1 --dump`。`--dump` に Rumoca の変数名 `rumoca var …` を足した）: `block TF parameter Real a[:] = {1}; Complex aw[size(a, 1)]; … end TF;` を `parameter Real aa[2] = {1, 2}; TF tf(a = aa);`（**配列を別の引数への参照で修飾**）と使うと、Rumoca の変数は `tf.aw.re`・`tf.aw.im`、自前は `tf.aw[1].re` … 。`parameter Integer m = 3; Pn p(n = m);`（整数の引数への参照）は両方展開する。MSL の `ShowTransferFunction`（`final a = a`）と `Visualizers.Internal.Lines`（`n = size(lines, 1)`）がこの形。**配列の値をリテラルで修飾したとき（`a = {1, 2}`）に展開するかは未確認【未確認】**（比べを名前の正規化に変えたあとの計測では見分けがつかなかった）。
  - **どちらが正しいか**: 配列の次元は引数・定数から評価できる式で与える（Rumoca の `evaluate_array_dimensions` のコメントも MLS §10.1 を挙げる。MLS の本文は今回は読み直していない【未確認】）。次元が評価できる式なので、自前の展開（要素ごと）が普通の flat で、Rumoca が評価器の限界で配列の変数にまとめていると見る。**自前は Rumoca に合わせない**（展開しない条件を Rumoca の評価器の癖に合わせて真似ると、`size` の引数の種類ごとの場合分けが要り、Rumoca の更新で変わる）。
  - **決定: 比べるときだけ名前を写像する**。`xtask diff-frontend` の `compare_flat` が、両方の変数名から**部品・レコードの配列の要素の添字**（`.` の直前の `[…]`。`cylinders[1].R.T` → `cylinders.R.T`）を落として集合を比べる（`without_elem_subs`、試験 1 本）。`x[1,2]` のような末尾の添字は落とさない。これで名前の食い違い 37 本が一致し、flat の名前の一致は 425/463 → **463/463**。**弱くなること**: 要素の数の違い（`cylinders[3]` と `cylinders[2]`）は名前の比べでは見えなくなる（方程式の数と DAE の比べに残る）。報告の「一致」の母数は変えていない。
- **`ActuatorWithNoise`（方程式 728 対 736）は S330 と同じ Rumoca の数え方の誤り**。`Gearbox` の `LossyGear` の `lossTable` の表が、方程式の数に漏れる。`LossyGear` を 1 つだけ使う最小の試験（`lossTable=[0,0.85,0.8,0.1,0.1]`、1×5）で自前 55 対 Rumoca 63（**+8**）、表を 2×5 にすると 55 対 73（**+18**）で、表の大きさに依存して増える。`ActuatorWithNoise` の +8 は歯車 1 つ分で説明できる。既知の差に残す（S330 の 10 本と同じ原因。上流への報告の 4 点セットに足す候補）。
- **`EngineV6`・`EngineV6_analytic`（7392 対 7464、6583 対 6631。DAE は状態の数も）と MultiBody の DAE の不一致の原因は、`Connections.isRoot` を含む `if` の方程式（`Parts.Body`）**。Rumoca は過剰決定の接続のグラフ（MLS 9.4。`rumoca-phase-flatten/src/vcg*`）で根を決め、`if not Connections.isRoot(frame_a.R)` の**1 つの枝**を選ぶ。自前の dae は枝の和集合を 1 行にする shortcut なので、選ばれなかった枝の `der(Q)`・`der(phi)`・`phi_dd` も状態・未知数に数える（`--dump` の行 `if not Connections.isRoot(box.body.frame_a.R) then … -> der(box.body.Q[1..4]), der(…phi[1..3]), …`）。`Parts.Body` を世界と 1 つだけ置いた最小の試験で、flat の方程式の数が自前 53 対 Rumoca 51（Rumoca は 11、自前は 13 の `if`）、状態が 19 対 13（Body 1 つで +6）。**flat の方程式の数の差（−72 など）は、`if` を 1 つの枝の数にする規則が Rumoca と合わない分【原因の細かい内訳は未確認】**。直すには `Connections.root`・`potentialRoot`・`branch`・`isRoot` の根の決定（過剰決定の接続のグラフ）を `globe-mode-flat` に作る必要があり、この WP では扱わない。**次の WP の候補**。
- **数字**（MSL の例題 566 本。S329 の報告 → S332。`GlobeModeValidation` 75 本は変わらず）: DAE にできた 430 → **471** / 471。Rumoca の釣り合いまで通った 297 → **334**。flat の名前の一致 425/463 → **463/463**、方程式の数 435/463 は変わらず。DAE の判定・数・状態・正則は、新しく比べられた 37 本（MultiBody。すべて上の `isRoot`）が不一致なので 295/297・295/297・293/297・295/297 → 297/334・297/334・294/334・296/334（**不一致が増えたのではなく、比べられる本数が増えて、新しい分がまだ合わない**）。S329 の報告に対して回帰 0、改善 85 項目。`mise run msl:diff-frontend` は 551 s、最大 RSS 5.7 GiB。
- **試験**: `globe-mode-syntax` に 1 本（`subscript_after_paren_stays_in_the_paren`）、`xtask` に 1 本（`element_subscripts_are_dropped_only_before_a_field`）。
- **shortcut**: 括弧への添字は木に含めるだけで、`globe-mode-dae` は添字を読まず括弧の中身の参照に触れる（`({…})[1]` は式全体の incidence）。`Connections.isRoot` の根の決定は作っていない。
- **持ち越し**: 過剰決定の接続のグラフ（`Connections.isRoot`。MultiBody の 37 本の DAE の不一致の原因）、`a = {1, 2}`（リテラルの修飾）で Rumoca が展開するかの確認、`HeatLosses` の +21 の内訳（S330 から）、上流への報告（表の方程式の数）。

## 付記（S330）: S324 の後に残った食い違いの原因（方程式の数 10 本は Rumoca の数え方の誤り、正則 4 本と dae-parse 1 本を直した）

新しい ADR は立てず、S324 で残した 3 つ（方程式の数 10 本・構造の正則 4 本・`dae-parse` 1 本）を `cargo xtask diff-frontend --class … --dump` で確かめた記録をここに足す。変えたのは `globe-mode-dae`（と `globe-mode-syntax` の式の読み 1 か所）で、`globe-mode-flat` は変えていない。`--dump` は、自前と Rumoca の `DaeStructure` の各行に `state_refs`（`x` 側の参照）も並べる（S6 は `incidence ∪ state_refs` で完全マッチングを見るため）。

- **方程式の数が合わない 10 本（Rotational の摩擦・歯車）は Rumoca の数え方の誤りで、自前は直さない**。S324 の見立て（`tau0 = mu0*cgeo*fn` を Rumoca が複数と数える）は当たっていた。原因は `tableID` の表引きそのものではなく、**外部オブジェクトを作る `table = mu_pos`（行列）の大きさが、それを引数にとる関数の結果（`getTable1DValueNoDer2(tableID, 1, 0)`、戻り値は `Real` 1 つ）の大きさとして `mu0` に伝わること**。`mu0` を含む `tau0 = mu0*cgeo*fn` が、`mu_pos` の要素の数（`[0, 0.5]` は 1×2 = 2）の方程式に数えられる。`mu_pos` を 3×2 にすると 6 になった。
  - 最小の再現（`diff-frontend --validation <dir> --class W4 --dump`。flat の方程式の数は自前 2、Rumoca 7。`mu_pos = [0, 0.5; 1, 1; 2, 3]`）:

    ```modelica
    model W4
      import Modelica.Blocks.Types.{ExternalCombiTable1D, Extrapolation, Smoothness};
      import Modelica.Blocks.Tables.Internal.getTable1DValueNoDer2;
      parameter Real mu_pos[:, 2] = [0, 0.5; 1, 1; 2, 3];
      Real fn;
      Real tau0;
      final parameter ExternalCombiTable1D tableID = ExternalCombiTable1D(
        tableName = "NoName", fileName = "NoName", table = mu_pos, columns = {2},
        smoothness = Smoothness.LinearSegments, extrapolation = Extrapolation.LastTwoPoints,
        verboseRead = false);
      final parameter Real mu0 = getTable1DValueNoDer2(tableID, 1, 0);
    equation
      fn = time;
      tau0 = mu0*fn;
    end W4;
    ```

    `final`・`parameter` は要らない（`Real mu0 = …` でも同じ）。`mu0` を使わず、右辺に `getTable1DValueNoDer2(tableID, 1, 0)*fn` と直接書くと 1 本になる。表を使わない `mu0 = 2` や `mu0 = f(1, 0)`（普通の関数）も 1 本。
  - **自前に合わせない理由**: 方程式 `tau0 = mu0*cgeo*fn` の 3 つの変数はどれもスカラーで、MLS ではスカラーの方程式 1 本。`Real` を返す関数の結果が引数の配列の大きさを引き継ぐ規則は無い。Rumoca の `infer_simple_equation_scalar_count`（`rumoca-phase-flatten/src/equations/shape_inference.rs`）の式の大きさの推論が、`mu0` の束縛の呼び出しをたどって行列の大きさを拾うと見ているが、実装の細かい道筋までは確かめていない【未確認】。`OneWayClutch` は Rumoca の balance（`rumoca_balance`）が通らず、DAE の側の比べには入らない。
  - **10 本の差の内訳**（`--dump` の Rumoca の方程式ごとの数と、自前の数の差が一致した）: `Clutch`・`Brake`・`OneWayClutch` は摩擦の部品ごとに +1（既定の `mu_pos = [0, 0.5]` が 2 個）、`LossyGearDemo1〜3` は歯車 2 つで +8（歯車の表は Rumoca が 5 と数える）、`SimpleGearShift`・`Friction` は +2、`CoupledClutches` は +3。`table_signs = if … then {1, 1} …` の長さ 2 の方程式は、自前も 2 本と数える（これは差にならない）。`HeatLosses` の +21 は部品ごとの内訳を確かめていない【未確認】。
  - 以降 `flat_eqs` は 340/350 のまま残る。**既知の差**として扱い、flat の数え方は変えない。上流への報告の候補（[upstream.md](../upstream.md) の 4 点セットは未作成）。
- **構造の正則 4 本（`Transformers` の `AsymmetricalLoad`・`Rectifier12pulse`・`Rectifier6pulse`・`TransformerTestbench`）の原因と直し**: 部品の配列を通した欄の参照。`PartialCore` の束縛 `SI.Voltage v2[m] = plug_p2.pin.v - plug_n2.pin.v` は、`plug_p2`（`Pin pin[m]` を持つコネクタ）が束縛より**後ろ**で宣言されているため、flat は `a.pin.v` を `{a.pin[1].v, …}` に直せず、そのまま印字する。`globe-mode-dae` の `refs` は `plug_p2.pin.v` を変数に引けず（変数は `plug_p2.pin[1].v` …）、束縛の行 `v2[k] = …` は `v2[k]` にしか触れなかった。`v2[k]` と `plug_p2.pin[k].v` が方程式でつながらず、`core.v2`・`core.v3`・`core.im` の 9 行が完全マッチングから外れた。
  - 直し: 変数に引けない参照は `a.pin[i].v`（`i` = 1, 2, …、最初に見つかる `.` の前に `[i]` を挟む）の変数の並びに直し、行 k はその k 番目の要素だけに触れる（新しい `Sel::Part`）。行の数が要素の数と違うとき（スカラーの行）は全部に触れる。
  - 平らなモデルの印字が `{a.pin[1].v, …}` と展開済みのときは、今まで通り要素ごとの参照に触れる（この修正で変わらない）。
- **`f(…).T` の読み（`Modelica.Media.Examples.MixtureGases` の 1 本）**: `globe-mode-syntax` の式の読みで、呼び出しのすぐ後ろの `.欄`（`.ident` を何個でも）を `CallExpr` の節の中に取り込む（木の形は変えない。呼ぶ関数の名前を取る `callee()` は今まで通り）。構文を直したので、flat が印字する `R.T = f(…).T` を dae が読み直せる。Rumoca はこの例題を平らにできない（`rumoca_flat` が通らない）ので、判定の比較には入らない。`dae-parse` は 0 本になった。
- **数字**（MSL の例題 566 本、S324 → S330。`GlobeModeValidation` 75 本は変わらず）: 自前で DAE にできた 357 → **358** / 358、構造の正則の一致 249/253 → **253/253**。方程式の数 340/350・釣り合い 253/253・方程式と未知数の数 253/253・状態の数 253/253 は変わらない。回帰 0 本、改善 5 本（正則 4・dae 1）。
- **試験**: `globe-mode-dae` に 2 本（`field_through_a_component_array_touches_each_element`・`field_after_a_call_of_a_record_function_is_read`）、`globe-mode-syntax` に 1 本（`field_after_call_stays_in_the_call`）。
- **shortcut**: 部品の配列を通した欄の参照は、`.` の前に `[i]` を 1 か所だけ挟む（`a.pin.v`。`a.b.pin.v` のように配列の部品が奥にあるときは最初の `.` から順に試し、最初に見つかった位置を使う）。2 か所以上が配列（`a[i].b[j].v`）の参照は直さない。MSL の例題では出ていない。

## 付記（S324）: B3 の続き、レコードの束縛・外部オブジェクト・配列のコネクタの一括接続

新しい ADR は立てず、S297・S312 の続きとしてここに足す。変えたのは `globe-mode-flat` の中だけ（`globe-mode-dae`・`globe-mode-syntax` は変えていない。新しい外部依存も無い。`xtask diff-frontend` に診断の出力だけ足した）。S297 の時点で断る理由の上位だった `composite-binding`（155 本）と `connect-array`（72 本）を減らした。

- **レコード全体の束縛 `r = other` を、型の欄ごとの束縛にする**（`WHOLE`。修飾の表の中の特別な名前 `"*"`）。S297 の担当は「宣言全体の巻き戻しと取り直し」が要ると見ていたが、**束縛の相手の欄を調べず、束縛される側の型の欄に `other.f` を当てれば済む**（`record_value` は、相手の欄がもう変数にあるときは今まで通りその欄を使い、無いときだけ `WHOLE` にする）。`inst_class_body` が、外側の明示した束縛が無い宣言 `f` に `other` と欄の接尾辞 `.f` を束縛として当て（外側の明示した修飾 > `r = other` > 宣言の既定値）、入れ子のレコードは接尾辞が `.f.g` と伸びる。配列の欄（`other.f` が配列）も同じ式で通る。宣言が後ろにある兄弟（`frictionParameters = dcpmData.frictionParameters` の `dcpmData`、`modelcard` など）がこの形。
- **レコードを返す関数の呼び出し `R = Frames.nullRotation()`** は評価せず、欄ごとに `R.T = Frames.nullRotation().T` の形の束縛にする（戻り値が `function` のクラスのときだけ。コンストラクタ `R(a = 1)` は今まで通り欄ごとの引数）。
- **外部オブジェクト**（`extends ExternalObject` を持つクラス。`tableID`）は、欄に分けず不透明な 1 つの変数にする（型の名前は完全修飾、`parameter`、束縛はコンストラクタの呼び出しの式のまま）。Rumoca の flat も同じ名前の変数を残す。
- **配列のレコード** `portsData[] = {VesselPortsData(…)}`・`states[] = statesFM`: 1 次元だけ、リテラルは要素ごとの式、別の配列は `x[k]` に欄を足して束縛する（大きさが合わなければ断る）。
- **配列のコネクタの一括接続**: `connect(a.p, b.n)` の端が部品の配列を通した名前（`source.sineVoltage.p` → `source.sineVoltage[1].p` …）か、配列のコネクタ（`a.pin` → `a.pin[1]` …）のとき、`gen_connects` が要素ごとの接続に展開してから集合を作る。片方が部品の配列の要素ごとの端で、もう片方が配列の変数 1 つ（`connect(sensor.i, i)`、`i[m]`）なら、変数を `i[1]` … にする。大きさが合わなければ `connect`（`differ in size`）で断る。展開は 1 つの次元だけ（`[i, j]` は断る）。
- **部品の配列を通した欄の参照 `a.pin.v`**（`pin` が部品の配列）は、方程式の数を要素の数にし、印字を Rumoca の flat と同じ `{a.pin[1].v, a.pin[2].v}` にする（これまで 1 本と数えていた）。
- **Rumoca の flat との突き合わせ**（S312 と同じ手順。MSL の例題 566 本、`mise run msl:diff-frontend`）: 自前で平らにできたのが **279 → 358 本**（回帰 0。S325 を取り込む前は改善 79 本、取り込んだあとは 96 本）、Rumoca の flat まで通った 350 本で名前の集合 **350/350**、方程式の数 **340/350**（S312 は 273/273）。合わない 10 本は Rotational の摩擦・歯車（`OneWayClutch`・`LossyGearDemo1〜3`・`SimpleGearShift`・`Friction`・`CoupledClutches`・`Brake`・`HeatLosses`）で、`tableID` の表引き（`getTable1DValueNoDer(tableID, 1, 0)` を束縛に持つ parameter `mu0`）を使う方程式 `tau0 = mu0*cgeo*fn` を Rumoca が 2 本と数える（`OneWayClutch` は 40 対 41。Rumoca 側の数え方の誤りと S330 で確かめた）。DAE は 357/358 が通り（残り 1 本は下）、Rumoca の釣り合いまで通った 253 本で判定 **253/253**・方程式と未知数の数 **253/253**・状態の数 **253/253**・構造の正則 **249/253**（S325 は 201/201。正則の食い違いは新しく平らにできた 4 本。S325 を取り込んだあとの数。取り込む前は 232/253）。`GlobeModeValidation` の 75 本は変わらない（73/73・73/73・73/73・正則 72/72）。
- **断った理由の内訳**（MSL の例題 566 本、最初の理由だけ数える。S297 → S324）: レコードの束縛 155 → **31**（残りは `Complex` の演算子 `V_m = port_p.V_m - port_n.V_m`〔14〕・`vs = plug_sp.pin.v - plug_sn.pin.v`〔9〕・`ComplexMath.exp(Complex(…))` など、`Complex` を返す式）、配列のコネクタの一括接続 72 → **1**（`heatPorts[HEX.nNodes : -1 : 1]` の範囲の添字）、`connect` の集合の不一致 8 → 9（`StateGraph` 8 と、`LogicalSample` の `connect` の大きさ 1）。先へ進んだ分で新しい上位になったのは、条件つきの宣言 19 → 58、配列の修飾 8 → 38（`Visualizers.Internal.Lines` の `cylinders[n](length = {… for i in 1:n})`。MultiBody の 27 本が当たる）、`[:]` 20 → 26、`stream` 2 → 20（`Modelica.Fluid`）、`for` の範囲が定数でない 0 → 16（`for i in 1:mSystems`）、大きさが定数でない 2 → 6。
- **DAE（S325 の担当）への申し送り**: レコードを返す関数の値 `R.T = f(…).T` は、`globe-mode-dae` が方程式の印字を読み直すとき `f(…).T`（呼び出しのあとの `.欄`）を読めず `dae-parse` になる（`Modelica.Media.Examples.MixtureGases` の 1 本。ほかは `parameter` の束縛で、方程式にならない）。`globe-mode-syntax` の式の読み方に、呼び出しのあとの欄の参照を足すか、DAE 側で受けるかは S325 に任せる。
- **持ち越し**: `Complex` の演算子の式をレコードの束縛にする（`re`・`im` に分ける。31 本）、配列のレコードの修飾 `each` でない配列の値（`cylinders[n](length = {…})`、38 本）、条件つきの宣言の評価（58 本）、`stream`（20 本）、Rotational の方程式の数 10 本の原因（S330 で Rumoca 側の誤りと確かめた）。

## 付記（S329）: B3 の続き、部品の配列への修飾・`Complex` の演算子・条件と範囲の評価

新しい ADR は立てず、S324 の続きとしてここに足す。変えたのは `globe-mode-flat` の中だけ（`globe-mode-dae` は触っていない）。S324 の後に上位になった断る理由（array-modification 38、conditional-component 58、composite-binding 31、non-constant-for 16）を減らした。

- **部品の配列への修飾**（`split_mod`）: `c[n](a = {…})`・`fill(x, n…)` を要素ごとの修飾に分ける。1 次元で項が並んだ `{…}` は項そのもの、`for` つきの内包と多次元は `({…})[k]`・`({…})[i,j]`（式には括弧を付けて印字）、`fill` は次元の数が合うとき `x`。`each` は分けない。項の数が合わなければ `array-modification` で断る。array-modification 38 → **0**。
- **`Complex` の演算子**（`complex_parts`・`cx_bin`）: 束縛が `+ - * /`・単項 `-`・`if`・`Complex(re, im)`・`ComplexMath` の関数の式のとき、評価せず欄 `re`・`im` の式にする（`a - b` → `.re = a.re - b.re`、`.im = a.im - b.im`。`*`・`/` は MSL の `Complex` の定義どおり）。**演算子の関数の中身は読まず、MSL の `Complex` の定義を決め打つ**（演算子の関数の `algorithm` を読んで展開するには、引数の型による解決を作る必要があり、`+ - * /` の 4 つで足りるので小さい方を選んだ）。Real の被演算子は虚部を持たず、0 の項は出さない。型が分からない参照（宣言が後ろの部品）は Complex と見なす。`^`・`==` は断る。`x.i[1]` のように添字のある参照は `WHOLE` で欄ごとに束縛する。式は `Bind::lit`（評価せず組み立てた文字列）に持つ。
- **条件・範囲の評価**: 文字列の比較（`terminalConnection <> "D"`。中身でなく hash を持つ `Val::S`）、`outer` の部品の欄 `world.enableAnimation` を inner の名前に付け替え、まだ平らにしていない（宣言が後ろの）inner の欄を修飾か型の既定値から引く（`pending_field`）、条件が定数に決まる `if` のレコードの式は選ばれた枝（`R_rel = if rotationType == … then … else …`）、`for k in 1:mSystems` の `numberOfSymmetricBaseSystems(m)`（MSL のこの 1 つの関数だけを決め打つ）。
- **Rumoca の flat との突き合わせ**（MSL の例題 566 本、`mise run msl:diff-frontend`。S324 → S329）: 自前で平らにできた本数 **358 → 471**（回帰 0、改善 113 本）。Rumoca も通った本数が 350 → 463。名前の集合 350/350 → **425/463**、方程式の数 340/350 → **435/463**。DAE は Rumoca の釣り合いまで通った 253 → 297 本で判定 253/253 → 295/297、状態の数 253/253 → 293/297、構造の正則 249/253 → 289/297。`GlobeModeValidation` の 75 本は変わらない。646 s、最大 RSS 4.8 GiB。
- **名前の食い違いの内訳**（flat まで通ったのに合わない 51 本。flat の本数が増えても同じ 51 本）: 37 本は `cylinders[n](…)`（MultiBody の `Visualizers.Internal.Lines`）と `Complex aw[2]`（`ShowTransferFunction`）のような**部品・レコードの配列で、Rumoca が要素に展開せず `cylinders.R.T` のように 1 つの配列の変数にする**（自前は `cylinders[1].R.T` …）。同じ要素でも `Pin pin[m]` は Rumoca も展開するので、Rumoca が展開しない条件（非 `each` の配列の修飾か、大きさを評価できないとき）は未確認。ほかは Rotational・Translational の 10 本（S324 から。`tau0 = mu0*cgeo*fn`）、`ActuatorWithNoise`（方程式 728 対 736）、`EngineV6`・`EngineV6_analytic`（方程式 7392 対 7464・6583 対 6631。DAE も状態の数が合わない）で、原因は未確認。
- **断った理由の内訳**（最初の理由。S324 → S329）: array-modification 38 → 0、conditional-component 58 → 0、composite-binding 31 → 25（`Complex` でない関数の戻り値・`Complex` の `^` など）、non-constant-for 16 → 1、unsized-array 26 → 28、stream 21 → 22、connect 9、non-constant-dim 6、expandable 3 → 4、connect-array 1。
- **DAE（S330 の担当）への申し送り**: flat まで通って DAE が読めなかったのは 41 本（DAE 430/471）。`f(…).T`（`Frames.nullRotation().T`）が 37 本、`if` の式・括弧の読み（`expected else`・`expected )`）が 4 本。`globe-mode-dae` か `globe-mode-syntax` の式の読み方の側の宿題。
- **持ち越し**: `stream`（22 本）、unsized-array（27 本。`table = if n > 0 then …` など関数の戻り値の大きさ）、`numberOfSymmetricBaseSystems` の決め打ちを関数の評価器に置き換えること、Rumoca が配列を展開しない条件の確認、`EngineV6`・`ActuatorWithNoise` の方程式の数。

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
- **feature**: facade に `native`（`globe-mode-frontend/native`）、LSP に `native`（`globe-mode/native`）。CLI（`globe lsp` を含む）と Python の束ねが立てる。wasm は S313 では立てない（理由: native の段の crate が wasm の大きさと build に入る。入れるかどうかは別に判断する → S347 で測って入れた）。立てない build で `Native` を選ぶと `GM-U0006`「この build に入っていない」を返す。外部依存は増えない。
- **LSP**: `globe-mode.toml` の `[check] frontend = "rumoca" | "native"`（`LintOptions::frontend`。他の値は設定の誤りで、既存の `showMessage` の道）。`publishDiagnostics` の検査（`diagnostics`）だけが `check_with` を使う。`globe/check` の要求は変えていない。設定ファイルが無ければ今まで通り。
- **Python**: `Workspace.check(model, frontend="rumoca")`。知らない名前は `ValueError`。
- **試験**: facade `tests/check_with.rs`（`expandable` が `GM-U0006`、既定は出さない。feature があれば Decay が通る）、LSP `config_frontend_native_drives_diagnostics`、`test_check_frontend_native`（pytest）、`config` の単体試験に `[check]`。
- **持ち越し**: wasm・UI から選ぶ（S347 で済。下の付記）。LSP の他の要求（`globe/check`・補完など）と、`outline` 系は Rumoca のまま。

## 付記（S347）: wasm とブラウザの UI からも native の検査を選べるようにした

新しい ADR は立てず、S313 の持ち越し（wasm・UI から選ぶ。wasm のビルドの大きさの測定が先）の続きとしてここに足す。既定は Rumoca のまま。

- **大きさ**（`mise run wasm:build` と同じ release の設定〔fat LTO・codegen-units 1・opt-level s・strip〕で、同じ共有 target に `-p globe-mode-wasm` を feature `native` なし・ありで続けてビルドした。S347 の時点の main から。2026-11-15）:

  | | cargo の出力 | `wasm-bindgen` 後（配る `.wasm`） | `gzip -9` |
  | --- | ---: | ---: | ---: |
  | `native` なし | 20,885,103 | 19,242,637 | 5,891,577 |
  | `native` あり | 21,261,979 | 19,597,599 | 5,998,898 |
  | 増分 | +376,876 | +354,962（+1.84%、0.34 MiB） | +107,321（+1.82%） |

  この測定のあとで診断の注の文字列（下）を足したので、配る `.wasm` は 19,598,045 バイト（+355,408、+1.85%）。増分は小さく（ブラウザの転送は gzip で +105 KiB）、外部依存も増えない（自前の 5 つの crate）。ので、**wasm の既定の feature に入れた**（`globe-mode-wasm` の `default = ["native"]`。CI・android・web・release の wasm のビルドは `cargo build -p globe-mode-wasm` のままで入る）。外したいときは `--no-default-features` で、`check_with(.., "native")` は `GM-U0006`「この build に入っていない」を返す。
- **wasm**: `Workspace.check_with(model, frontend)`（`frontend` は `"rumoca"`・`"native"`。知らない名前は例外）。`check(model)` はそのまま Rumoca。Worker の `check` は `{model, frontend}` を取り、`GlobeWorker.check(model, frontend?)` が渡す（省くと Rumoca）。native が読むのは作業中のファイル（`set_file`）だけで、ブラウザで読んだ MSL（`with_library_sources`。ライブラリのファイルは DB の外の `Checker` にあり、ディレクトリも無い）のクラスは見えない（S313 と同じ制限）。Node で確かめた: MSL を読んだ wasm で `ChuaCircuit` を検査すると、Rumoca は通り、native は `GM-E1002`（モデルが見つからない）、自分のファイルが `Modelica.Electrical.Analog.Basic.Resistor` を使うと `GM-E1001`（名前を解決できない）で止まる。これを「モデルの誤り」と読まれないよう、facade の `Native` の道は、ファイルの組で読んだライブラリがあるワークスペース（`library_files > 0` で `library_paths` が空）では、`GM-E1001`・`GM-E1002` の診断に「native は … ライブラリのクラスを見られない。Rumoca で確かめる」の注を足す（診断の種類・段は変えない。ディレクトリで読むワークスペース〔LSP・CLI・Python〕と、ライブラリの無いワークスペースには付けない）。ライブラリを native に見せるには、facade がファイルの組を持つ必要がある（MSL の本文の写しが増えるので、今回はやらない）。
- **UI**: 検査ボタンの横に「Rumoca / 自前（試験）」の select（`aria-label="検査のフロントエンド"`、既定は Rumoca。S350 で「設定に従う」を足して既定にした）。`Backend.check(model, frontend?)` と、選べる実行環境だけが持つ `Backend.checkFrontends`（`WorkerBackend` は true。ブラウザと Android）。native で検査すると Rust が作る要約（「S0〜S6 を通過。S7 以降は検査していない」）を 1 行出し、`GM-U0006` の原文の見出しは「自前の段の原文」にする。業務ロジックは Rust のまま（UI は名前を渡し、要約を出すだけ）。
- **Desktop は未（S349 で済。下の付記）**: sidecar の LSP `globe/check` が `frontend` を取らない（S313: 要求は変えていない）。取らない要求に選択を出すと黙って Rumoca になるので、`TauriBackend` は `checkFrontends` を持たず、select を出さない。LSP の `globe/check` が `params.frontend` を読むようになれば、`TauriBackend` は `check(model, frontend)` で渡して `checkFrontends = true` にするだけ（LSP は S345 と並行だったので触っていない）。
- **試験**: wasm（Node）`node/smoke.mjs` で `check_with(Decay, "native")` が通って要約に「native: S0〜S6」、`rumoca` は `check` と同じ、知らない名前は例外。`web/worker-node.mjs` で Worker 越しにも届く。ネイティブの `tests/decay.rs`（`expandable` は `GM-U0006`、feature があれば Decay が通る）。facade の `tests/check_with.rs` の `native_explains_a_library_it_cannot_see`（`with_library_sources` のライブラリを使うモデルは `GM-E1001` に注が付き、使わないモデルは通る）。MSL があれば `smoke.mjs` が `ChuaCircuit` で同じ注を確かめる。UI の `tests/test_ui.py` の `check_frontend_select`（選択肢・既定・Decay が自前で通って要約が出る・`expandable` が「自前の段の原文」・Desktop に選択肢が無い〔S349 で「ある」に変えた〕）。
- **持ち越し**: Desktop の UI（上）。実行（`simulate`）・BLT・段・グラフは Rumoca のまま。選択はタブごとにも保存にも持たない（ページを開くたびに Rumoca）。

## 付記（S349）: Desktop の UI からも native の検査を選べるようにした

新しい ADR は立てず、S347 の持ち越し（Desktop の UI）の続きとしてここに足す。既定は Rumoca のまま。

- **LSP**: `globe/check` の params は `{model, frontend?}`。`frontend` は `"rumoca"`・`"native"`（facade の `CheckFrontend::from_str` と同じ名前）で、`Workspace::check_with` に渡す。無ければ `globe-mode.toml` の `[check] frontend`（`publishDiagnostics` と同じ値）、それも無ければ Rumoca。知らない名前・文字列でない値は -32602（`params.model` が文字列でないときと同じ）。LSP の他の要求と `outline` 系は Rumoca のまま。`globe-mode-protocol` に検査の要求の型は無く（応答の `CheckResponse` だけ）、TS の型は S347 の `CheckFrontend` をそのまま使うので、型の変更は無い。
- **UI**: `TauriBackend.check(model, frontend?)` は `frontend` があれば `params` に乗せ（省けば乗せない）、`checkFrontends = true`。S347 の select（`aria-label="検査のフロントエンド"`）と要約の 1 行が Desktop にもそのまま出る。S349 の時点では App.svelte の select が最初から `rumoca` を渡していたので、Desktop の「検査」ボタンは `[check] frontend = "native"` があっても、select を変えない限り Rumoca で検査した（エディタの波線の診断は設定どおり）。S350 で select に「設定に従う」を足して既定にし、直した（下の付記 S350）。
- **VS Code の拡張**: 変えていない。拡張は `globe/check` を使わない。
- **試験**: LSP の `check_selects_the_frontend`（`expandable` は既定と `rumoca` で通り、`native` は `GM-U0006`、`"nope"`・数は -32602）、`tests/test_tauri.mjs`（`check(M, "native")` は `params.frontend` に乗り、省けば乗らない。`checkFrontends` は true）、`tests/test_ui.py` の `check_frontend_select`（偽の Tauri で select があり、「自前」で検査すると `globe/check` の `frontend` が `rumoca`・`native` の順に届き要約が出る）。
- **持ち越し**: 実機の Desktop（Tauri）での目視は【未確認】（偽の `window.__TAURI__` と sidecar の LSP の単体試験まで）。実行（`simulate`）・BLT・段・グラフは Rumoca のまま。

## 付記（S350）: select の既定を「設定に従う」にした

新しい ADR は立てず、S349 の持ち越し（Desktop の「検査」が設定の `[check] frontend` を使わない）の続きとしてここに足す。

- **select**: 「設定に従う / Rumoca / 自前（試験）」の 3 択で、既定は「設定に従う」（App.svelte の値は `config`）。「設定に従う」を選んでいるあいだは `backend.check(model)` に `frontend` を渡さない（`TauriBackend` は `params` に乗せず、`WorkerBackend` は Rumoca）。Desktop は LSP の `check_frontend`（S349）が `[check] frontend` → Rumoca の順で決めるので、エディタの波線と「検査」ボタンが同じ設定に従う。設定ファイルの無いブラウザ・Android では「設定に従う」は Rumoca と同じで、select の形はどの実行環境でも同じ。
- **応答の `frontend`**: 「設定に従う」では、使った方を UI が知らない（設定が native なら要約を出し、`GM-U0006` の原文の見出しを「自前の段の原文」にしたい）。`CheckResponse` に `frontend`（自前の段で検査したときだけ `"native"`。Rumoca の応答には出ない）を足し、`native_response` が付ける。UI は `checkFrontend === "native" || checked.frontend === "native"` のとき要約を出す（Rust が決めた値を読むだけで、設定の読み方を UI に置かない）。S349 では protocol の型を変えなかったが、これは応答の欄を 1 つ足す変更で、Rumoca の応答と既存の JSON は変わらない。
- **試験**: LSP の `config_frontend_native_drives_diagnostics`（`[check] frontend = "native"` で `globe/check` が `frontend` を省くと native〔応答の `frontend` が `"native"`〕、`rumoca` を渡すと Rumoca）と `check_selects_the_frontend`（native の応答に `frontend`）、facade の `tests/check_with.rs`（Rumoca の応答に `frontend` が無く、native の応答は `Some("native")`）、`tests/test_tauri.mjs`（`check(M)` と `check(M, undefined)` は `params` に `frontend` が乗らず、`rumoca`・`native` は乗る）、`tests/test_ui.py` の `check_frontend_select`（既定は「設定に従う」で、偽の Tauri の `globe/check` に `frontend` が乗らない。`rumoca`・`native` を選ぶと乗る。応答に `frontend: "native"` があれば、渡さなくても要約が出る）。native の応答に `frontend` が付く試験は feature `native` を立てたとき（`cargo test -p globe-mode-lsp --features native`）に効く。
- **持ち越し**: 実機の Desktop（Tauri）での目視は【未確認】。UI の `checkFrontend === "native"` の項は、応答に `frontend` が無い古い wasm・sidecar でも、自前を選んだときの要約が出るように残した（この WP では release の wasm を作り直さず、共有 target にある S347 の wasm でブラウザの試験を回した。応答の `frontend` は CI が作る wasm の native の応答には載る）。

## 付記（S351）: 第 6 回のレビューの issue #132〜#134

新しい ADR は立てず、第 6 回のレビュー（S337・S340〜S349 の変更）の issue 3 件を、1 件ずつ再現してから直した記録をここに足す。3 件とも直す前に再現し（#132 は `members_of` を直接呼ぶ使い捨ての crate、#133 は flat → `globe_mode_dae::build` の数、#134 は Node の `GlobeWorker`）、レビューの指摘どおりだった。直す試験は、直す前なら落ちることを確かめた。

- **#132（補完が、差し替えていない同名の別の型で出る）**: S345 の `members_of` は、受け継いだ部品の型を `redeclared_types` の 2 つ目の節（継ぐ側のクラスから名前を引き直した型）で**いつも先に**取り、書いてある型を後回しにしていた。S341 の `step` は、既定の型で続きの名前が引けず `Reason::Redeclare`（`redeclare` / `replaceable` が絡む）になるときだけこの引き直しを使うので、`members_of` にだけ門が無かった。`Lib.OnePort` の `Pin p` を継ぐ `MyLib.R` が、`MyLib` の自前の `Pin`（`redeclare` ではない）の欄 `x` を出していた（正しくは `i`・`v`）。**直し**: `members_of` が `resolve` に空の名前を足した参照（`p.` の続きを引く参照）を引き、結果が `Unresolved(Redeclare)` のときだけ、差し替えた後の型を先にする。門の判定を `members_of` に書き写さず、`step` 自身に答えさせた（`step` の `Walk` が持つ「道の途中の `extends` の `redeclare`」と「型の `replaceable`」の両方が効く）。それ以外は書いてある型。S345 の `members_of_follows_redeclared_types`・LSP の `completion_and_definition_follow_redeclared_types` は変わらず通る。
- **#133（レコードの配列への方程式が数えられない）**: S337 は `Complex aw[3]` の `aw = {…}` の flat の数を要素の数（3）にしたが、DAE の `build` はレコードの葉を `count == 1` のときだけ欄に展開していたので、`count == 3` の方程式は 3 本と数えられ（`aw` の欄は `aw[k].re`・`aw[k].im` の 6 本）、釣り合う模型が `UNBALANCED` になった。直す前の数は、for 文の形が 8 対 8、配列の形・配列どうしが 8 対 5・12 対 6。**直し**: `record_leaves` が、`t.`（スカラーのレコードの欄）で引けなかったとき `t[k].x` の変数すべてを `array_leaves` で集める（添字つきの参照・変数そのものは対象外）。欄の変数の数（要素 × 欄のスカラー）が方程式の数になり、`own`（行が触れる未知数）にも欄が入る。flat の `count`（Rumoca と合わせた数）は変えない。`globe-mode-dae` に `record_array_equation_counts_per_field`（for 文・内包表記・配列の式・配列どうし・長さ 1 の配列が 8/8・12/12・2/2）。
- **#134（壊れたパックの Blob を捨てない）**: Android の束（S340）は、パックの Blob を `lazy.blob` に持ち、取れなかったときだけ捨てていた。応答が 200 で中身が壊れていた（短い・索引と版が合わない）ときは、`fetchChunks` が「塊 i が壊れている」「塊に無いファイル」で失敗しても Blob が残り、配信が直っても同じ Blob を切り出して失敗を繰り返した（Node では `The blob could not be read`）。**直し**: その 2 つの失敗で `lazy.blob = null`（次の呼び出しで取り直す）。c/*.json の束は `getText` を毎回呼ぶので元から直れば通る。`node/broken-chunks.mjs` に「壊れたパック → 直したパック」の節を足した。
- **shortcut**: #132 の門は、続きの名前に空の名前を使って `step` に問い合わせる（補完の 1 回につき `resolve` が 1 回増える。補完は今の道が候補を出せないときだけ自前の道に回るので、頻度は低い）。型が見つからなかったときは差し替えた型へ回す（#132 の前と同じ）。#133 の `array_leaves` は `t[` で始まる変数を全部なめる（O(変数の数)。`t.` で引けず、添字のない、変数でない左辺の方程式だけ）。左辺が配列で右辺だけがレコードの配列という方程式（`{…} = aw`）は見ない（Rumoca の `record equation` の左辺が参照でない形）。#134 は、壊れた応答のあとの 1 回の取り直しの費用（パック全体の取得）を払う。
- **数字**（MSL の例題 566 本・`GlobeModeValidation` 75 本、S350 → S351。`msl:diff-frontend` 相当を 582 s・最大 RSS 5.7 GiB で）: 表は main の `doc/frontend-diff.md` と全行同じ（resolve 566/566、flat 471/566、DAE 471/471、判定・数・状態・正則 334/334、検証 75 本 73/73・正則 72/72）、**回帰 0 本**。モデルごとの JSON を前回（S339）と比べて、DAE の数・釣り合い・正則の旗は 1 本も動かない（動いたのは S337・S341 の分だけ）。Rumoca は `Complex aw[3]` の `aw = {…}`・`aw = bw` の小さな模型で釣り合いまで通らない（`rumoca_balance` 0/2。for 文の形は通って自前と 8/8 で一致）ので、この形の数は Rumoca と比べられない（MLS の意味で数えた）。

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

## 付記（S327）: 相対の `--library` の URI（#118）

- **#118**: S320 の `same_path_key`（#115）は比べる側だけで、LSP が返す URI は相対のまま `file:///data/msl/...`（存在しない）になっていた。URI を作る共有の関数 `file_uri` が、`/` 始まりでも `C:` 始まりでもないパスを `std::path::absolute`（cwd を足す）で絶対にしてから組み立てる。定義・参照・リネーム・ライブラリの診断の URI はすべてここを通る。symlink は解かない（`same_path_key` と同じ）。
- 詳細と試験は [S327 の解説書](../sessions/S327-review4-issues/解説書.md)。

## 付記（S341）: resolve の `Redeclare` の 99 本を、差し替えた後の型で引いて 0 にした（B2 の続き）

新しい ADR は立てず、同じ流れの続きとしてここに足す。S275 の「未対応のまま `Reason::Redeclare`」のうち、`xtask diff-frontend` の resolve の列（未解決の参照が 0）が 0 にならなかった MSL の例題 99 本（未解決の参照 1330 件）と `GlobeModeValidation` の 1 本（13 件）を直した。変えたのは `globe-mode-resolve` と、`globe-mode-syntax` の AST（`RedeclaredComponent` を足しただけ）。flat は先頭で止める `resolve_head` と型名の解決しか使わないので、結果は変わらない。

- **原因**（99 本のクラスから `diff-frontend` と同じ手順で未解決の参照を全部出して形ごとに数えた）: すべて式の成分参照の続き `a.b` で、既定の型の中に `b` が無く、`redeclare` / `replaceable` が見えたので `Redeclare` と数えていたもの。形は 2 つ。
  - **型の名前を、受け継いだクラスから見直す形**（Media が大半）: `state.T`・`state_a.p`・`refState.X`。部品 `state` の型 `ThermodynamicState` は、基底 `PartialMedium` では中身が空の `replaceable record`。受け継いだ `PartialSimpleMedium.BaseProperties`・`WaterIF97_base.setSmoothState` などの外側のパッケージが `redeclare record extends ThermodynamicState`（`class extends`）で差し替えており、`T`・`p` はそちらにある。
  - **`extends B(redeclare T x)` の部品の再宣言**（Electrical）: `internalThermalPort.heatPortRotorWinding`・`thermalPort.heatPortPermanentMagnet`（`redeclare final …ThermalPortIMC internalThermalPort`）、`cellData.nRC`・`cellData.rcData.R`（`redeclare …TransientData.CellData cellData`）。S275 は「部品の再宣言（`redeclare Foo x`）」を未対応にしていた。
- **直し**（`Library::resolve` の `step`。部品の型の中で続きの名前を探す所）: 既定の型で引けず、失敗の理由が `Redeclare` のときだけ、次の順で差し替えた後の型を試す。
  1. 参照のあるクラス（`Ref::scope`）の受け継ぎを外側から（外側が勝つ）たどって、部品の持ち主を受け継ぐ基底の `extends B(redeclare T x)` のうち部品 `x` を差し替えるものを探し、`T`（あれば `constrainedby` の `C` も）の中で続きを引く（`comp_redecl`）。`globe-mode-syntax` の `ast::RedeclaredComponent`（接頭辞・`final`・添字・`constrainedby` を読み飛ばして型と名前を取る）で読み、`PathRef::comp_redecls` に持つ。
  2. 参照のあるクラスが部品の持ち主を受け継ぐ（`reaches`）なら、部品の型の名前を参照のあるクラスから引き直し、その型の中で続きを引く。`class extends` で差し替えた型が外側のパッケージにあるとき、引き直せば差し替え先に当たる。
  - 既定の型で引けたものは今まで通り（差し替えた型は引けないときだけ使う）なので、これまで解決した参照の結果は変わらない。
- **診断**: `cargo xtask diff-frontend --class … --dump` が、resolve で未解決になった参照（理由・名前・種類・クラス）を `unresolved …` の行で出す。
- **試験**: `globe-mode-resolve` に `redeclared_component_types`（`class extends` で差し替えた型を受け継いだ `BaseProperties`・関数から引く、`extends Base(redeclare PortX port)` の差し替え、差し替えた型にも無い名前は `Redeclare`、差し替えていない側は `NotFound`）、`globe-mode-syntax` に `redeclared_components_in_extends`（接頭辞・`final`・添字・`constrainedby`・クラスの再宣言を拾わない）。
- **数字**（MSL の例題 566 本・`GlobeModeValidation` 75 本、S334 → S341）: resolve 467/566 → **566/566**、未解決の参照の総数 1330 → **0**。`GlobeModeValidation` は 74/75 → **75/75**、13 → **0**。flat・dae の列は変わらない（flat 471/566・方程式の数 451/463・DAE 471/471・正則 334/334 のまま）。**回帰 0 本、改善 100 本（すべて resolve の旗）**。`mise run msl:diff-frontend` は 1880 s（他の WP と同時で swap し、`Spice3BenchmarkFourBitBinaryAdder` だけで 1095 s。S334 は 554 s）、最大 RSS 3.9 GiB。MSL の全 `.mo`（2556 本、参照 161869 件）の成分参照の続き 45463 件は、S275 から残っていた `Redeclare` の 639 件が 0 になり、すべて解決する（`msl_stats`）。`globe-mode-lsp` の `resolve_compare`（定義ジャンプの Rumoca の道との突き合わせ）は不一致 0 のまま。
- **shortcut（計測の範囲で未解決が 0 になるだけ）**:
  - 部品の再宣言は `extends` の修飾（と short class の基底）の中だけ。部品の修飾の中の `A a(redeclare T x)` は見ない（MSL の例題には出ていない）。
  - 差し替えは名前と受け継ぎで決める。差し替え先が `constrainedby` に合うか、`replaceable` かは見ない。「解決できた」は「正しい」とは限らない。
  - LSP の補完（`members_of`）は `state.` の候補を既定の型の要素のままにしている。
  - 差し替えた型の名前の見直しは、部品の持ち主を受け継ぐクラスからの参照だけ。持ち主を受け継がないクラスから `a.state.T` のように 2 段先で効く差し替え（`Medium` が `replaceable package` のとき）は、これまで通り `Redeclare`（MSL の例題には出ていない）。

## 付記（S345）: LSP の補完・定義ジャンプ・ホバーも、差し替えた後の型で引く（B2 の続き）

新しい ADR は立てず、S341 の付記の「shortcut」（LSP の補完は `state.` の候補を既定の型の要素のままにしている）を直した記録としてここに足す。変えたのは `globe-mode-resolve` の `members_of`・`step`（resolve の数え方は変えない）と、`globe-mode-lsp` の `definition`。

- **補完**（`Library::members_of`）: 最後の名前が部品のとき、その型を「`scope`（カーソルのあるクラス）から見て差し替わった後の型」で引く。差し替えた型の探し方は S341 の `step` と同じなので、`step` の中にあった「`extends B(redeclare T x)` の部品の型 → 受け継ぐクラスから見直した型の名前」を `redeclared_types` に切り出して、`step` と `members_of` の両方が使う（LSP 側に規則を複製しない）。差し替えた型があればそれ、無ければ書いてある型。差し替えた型は既定の型を受け継ぐか制約の下位の型なので、要素は既定の型の要素を含む。
- **定義ジャンプ・ホバー**: 書きかけの文書（今の道の `Outline` が引けない）は、S296 から `Library::resolve` で引いているので、S341 の `step` のまま差し替えた型に飛ぶ（直す所は無かった）。**読める文書**は今の道（`globe-mode-fmt` のアウトライン）が既定の型で引いて `null` になっていたので、成分参照（名前に `.` がある）が引けなかったときだけ、同じ自前の道（`tolerant_definition`）で引き直す。点のない名前（`Real`・`time`）では木を作らせない（木を作るのは最初の 1 回で、MSL では debug 8.8 秒・RSS 約 75 MB（S279 の計測。#107 のコメントは 0.6 s））。
- **試験**: `globe-mode-resolve` に `members_of_follows_redeclared_types`（`class extends` で差し替えた record の欄、`extends Base(redeclare PortX port)` の差し替えた部品の欄、差し替えていない側は既定のまま）、`globe-mode-lsp` に `completion_and_definition_follow_redeclared_types`（別の行にも誤りのある書きかけの `port.`・`state.` の補完、読める文書と書きかけの文書での `port.hot` の定義ジャンプとホバー）。
- **数字**: `cargo test -p globe-mode-resolve --test resolve`（8 本 pass）・`cargo test -p globe-mode-lsp`（lib 72・cross_file 5・folding 2・tolerant_outline 2 本 pass）、clippy（`--all-targets -D warnings`）0 件。`globe-mode-lsp` の `resolve_compare`（定義ジャンプを Rumoca の道と突き合わせる `#[ignore]` 試験。debug で 1312 s）は、型名 110 箇所が一致 110・LSP が null 0 のまま、成分参照の続き 93 箇所が一致 **86 → 91**・LSP が null **7 → 2**・不一致 0。残る 2 は突き合わせの位置（参照の範囲の最後の字）が添字の中・後ろになる参照（`massflowsink1.ports[1]`・`cellData.rcData[k].C`）で、名前を `1`・`C` と切るため null になる（`massflowsink1.ports` の上では `Sources.mo` の `ports` に飛ぶことを確かめた。`cellData.rcData[k].C` は S296 の添字の shortcut で、`name_at` が `]` で切る）。`msl:diff-frontend` は回していない（resolve の数え方は変えていない）
- **shortcut**:
  - 補完は、今の道（アウトライン）で候補が 1 件でも出る間は自前の道に回さない（木を作る費用を毎回払わないため）。既定の型に要素があり、差し替えた型にだけある要素は、今の道が読めない（カーソルの行を除いても誤りがある）ときだけ出る。MSL の Media の `ThermodynamicState` のように既定の型が空なら、カーソルの行だけ書きかけの文書でも出る。
  - 差し替えの規則は S341 と同じなので、持ち主を受け継がないクラスから見る `a.state.` のような 2 段先の差し替え、部品の修飾の中の `A a(redeclare T x)` は同じく見ない。
  - 定義ジャンプの引き直しは、点のある名前だけ。`Redeclare` 以外の理由で今の道が `null` のときも（`x.start` などの属性・誤字）、1 回だけ余計に木を引く。

## 付記（S352）: LSP の inlay hint（関数の呼び出しの位置引数の名前）

新しい ADR は立てず、S279・S296・S345 と同じ流れの続きとして足す。IDE の機能の穴（features の「アウトライン・inlay hint・call hierarchy」）のうち inlay hint を、**最初から自前の構文と名前解決だけで**作った（今の道〔Rumoca のアウトライン〕を持たない機能なので、読める文書でも書きかけの文書でも同じ道）。

- **出すもの**: `textDocument/inlayHint`（capability `inlayHintProvider`）。関数の呼び出しの位置引数の前に引数の名前（`f(x, y)` → `f(a: x, b: y)`。`kind` = Parameter、`paddingRight`）。`range` の中に引数の頭があるものだけ。VS Code の拡張は `vscode-languageclient` 9 が capability を見て自動で登録するので、拡張側の変更は無い（`editor.inlayHints.enabled` の既定で出る。【未確認】: 本物の VS Code では回していない。`vscode.yml` の起動試験にも足していない）。
- **つなぎ**（`globe-mode-lsp` の `inlay.rs`）: S279 の `Library`（`completion::tolerant_lib`）に文書を `with_source` で一時的に足し、文書の `CallExpr` ごとに、呼び先の `ComponentRef` と同じ範囲の `Ref` を引いて `resolve`。`Resolution::Class` のときだけ、新しい `Library::function_inputs`（`globe-mode-resolve`）で位置引数の名前を取る。`input` の前置きのある部品を宣言の順に、`extends` の基底のものを先に。基底が引けない・輪になる関数は None（推測しない）。`Component` に `input: bool` を足した（resolve の解決の規則は変えていない）。
- **出さないもの**: 呼び先が組み込み（`sin`・`der`）・部品・解決できない名前のとき、`input` より多い引数、名前つき引数（`f(b = 1)` は名前が書いてある）、`for` 内包の引数、`annotation` の中（resolve が `annotation` を読まない）、レコードの構築子（`Complex(1, 2)` の欄は `input` でなく宣言の順）。
- **試験**: `globe-mode-lsp` の lib に `inlay_hints_name_the_positional_arguments_of_a_resolved_function`（`extends` の基底の `input` が先、入れ子の呼び出し、名前つき引数・`der` は出ない、最後の行が壊れた書きかけの文書、範囲の内側だけ、capability）と `inlay_hints_are_silent_when_the_callee_cannot_be_resolved`（組み込み・未定義・部品・`input` より多い引数）。
- **shortcut**: 呼び先はライブラリ（`-L`）とその文書の中の関数だけ（別の開いた文書の関数は引けない。S279 の補完と同じ）。ライブラリの木は S279 と同じく起動後のディスクの更新を見ない。引数の名前と同じ名前の引数（`f(x)` の `x:`）も省かず出す。

## 付記（S354）: LSP の call hierarchy（関数の呼び元・呼び先）

新しい ADR は立てず、S352 の続きとして足す。features の「アウトライン・inlay hint・call hierarchy」の最後の穴。**新しい規則は作らず、既にある 2 つの道をつないだ**（呼び元は `references`〔S166・S230〕の道、呼び先は inlay hint〔S352〕の道）。

- **出すもの**（`globe-mode-lsp` の `call_hierarchy.rs`。capability `callHierarchyProvider`）: `textDocument/prepareCallHierarchy`（カーソルの関数の名前 → 宣言の `CallHierarchyItem`。宣言の名前の上でも、呼び出しの上でも）、`callHierarchy/incomingCalls`（その関数を呼ぶクラスと、そのクラスの中の呼び出しの範囲 `fromRanges`）、`callHierarchy/outgoingCalls`（その関数の本体が呼ぶ関数と、呼び出しの範囲）。対象は **`function` だけ**（モデル・部品・レコードの上は `null`）。項目は `selectionRange`（関数の名前）から毎回引き直すので、サーバは状態を持たない。呼び元の項目は関数とは限らない（`kind` は関数が 12、それ以外のクラスが 5）。
- **呼び元**: `class_name_at`（`textDocument/references` と同じ、カーソルの名前の定義）で関数を決め、`class_hits`（開いた文書）と `disk_class_hits`（根の下の開いていない `.mo` とライブラリ。S230）で同じ関数に行き着く識別子を集める。そのうち、自前の構文（`globe-mode-syntax`）の `CallExpr` の呼び先の名前の終わりに当たるものだけが呼び出し（宣言・`end f`・`extends f(…)`・型の使い方は除く）。呼び元は呼び出しを囲む一番内側の `ClassDef`。
- **呼び先**: S352 と同じ。`Library::with_source` に文書を足し、関数の本体の `CallExpr` ごとに `Ref` を引いて `resolve`。`Resolution::Class` かつ関数のものだけを呼び先にする。書きかけの文書でも出る。`Class` に `is_function: bool` を足した（`globe-mode-resolve`。`function` の字句があれば、`pure`・`impure`・`operator` の前置きがあっても真。解決の規則は変えていない）。
- **出さないもの**: 引けない呼び先、組み込み（`sin`）、レコードの構築子（`R(x)` は `record`）、`annotation` の中（resolve が読まない）。
- **試験**: `globe-mode-lsp/tests/call_hierarchy.rs` の 1 本（ライブラリ `-L` の関数 `f`・`g` とレコード `R`、開いた文書 `M` と開いていない `LibUser`）。prepare は別のファイルの関数の呼び出しの上で出てモデルの名前の上は null、incoming(f) は `g`（2 箇所）と `M`（2 箇所）で宣言・`end f` は数えず、incoming(g) は開いた文書 `M` と開いていない `LibUser`、outgoing(g) は `f` だけ（`sin`・引けない `nope`・`R(x)` は出ない。`is_function` を外すと `R` が出て落ちることを確かめた）、outgoing(f) は空、モデルの項目は null、capability。
- **shortcut**: 呼び元の探索はアウトラインの道（`class_name_at`）なので、構文の誤りで Rumoca のアウトラインが引けないファイルで宣言した関数は対象にできない（呼び先はその文書でも出る）。呼び元の探索は `references` と同じ費用（関数の名前の字面が部分文字列で入るファイルを全部字句に分ける。索引は持たない）。呼び先は、ライブラリ（`-L`）とその文書の中の関数だけ（別の開いた文書の関数は引けない。S352 と同じ）。レコードの構築子・`operator function` の呼び先は対象外。本物の VS Code の call hierarchy の表示は【未確認】（`vscode-languageclient` 9 が capability を見て自動で登録するので、拡張側の変更は無い。`vscode.yml` の起動試験にも足していない）。

## 付記（S339）: 5 回目のレビューの issue（#121〜#127・#130・#131）

5 回目のレビュー（S324〜S333 の変更）が立てた issue のうち、native の `globe-mode-flat`・LSP・xtask に届く 9 件を、再現を確かめてから直した。**issue の題は本文より 1 つずれている**（#122 の題は #121 の本文のもの、…、#126 と #121 の題は `x`）。番号は本文の指摘の番号で書く。#128・#129 は [ADR-0040](0040-multi-solver-compare.md)（#128）と CLI（#129）。

- **#121 `Complex` の `*`・`/` の展開が指数的**: `cx_bin` は実部・虚部の式のどちらにも両方の被演算子を使うので、`a*a*…` は a を 1 個足すごとに式が約 2 倍になる（`depth > 24` の歯止めは入れ子にしか効かない）。結果の式の長さ（実部＋虚部）が 64 KiB（`CX_MAX_LEN`）を超えたら `None`（＝ `composite-binding` で断る）にした。a が 40 個の式が 1.5 ms で断られる（修正前は 60 s で終わらない）。
- **#122 `initial equation` の `isRoot` が 2 つ以上でずれる**: 通常の方程式は `replace`（位置 → 差し替え）で新しい列に積み直すのに、初期方程式だけ `splice` で、先の `if` が方程式の数を変えると後の `at` がずれて別の初期方程式を上書きした。積み直しを関数 `restack` にして、`eqs`・`ieqs` の両方に使う。
- **#123 名前だけが `Complex` のレコード**: 展開に入る条件に「欄が `re`・`im` だけ」（`is_msl_complex`）を足した。欄が違えば今までどおり `composite-binding`（修正前は束縛が黙って落ちた）。
- **#124 部品の配列への `{…}` の修飾が O(n²)**: 原因は 2 つあった。`split_mod` が要素ごとに項の `Vec` を作り直す（issue の指摘）のに加え、`for` つきの内包かを調べる `children().any(…)` も要素ごとに全項を走査していた（こちらが支配的）。項と内包の判定を、配列の式ごとに 1 回だけ集める表（`HashMap<SyntaxNode, Option<Vec<SyntaxNode>>>`）に置いた。debug で n = 3000 が 10.2 s → 0.26 s（n = 6000 でも 0.46 s。n の 2 倍で 2 倍）。項の作り直しだけを直した状態では n = 3000 で 9.6 s と変わらなかった。
- **#125 `isRoot` の判定が O(N³)**: `root_of` を節点 → 優先度の表（`pot`）で引く（成分の大きさ O(M)）に、`is_root` は成分ごとの根を 1 回だけ決めた表（`root_by_comp`）を引くだけにした。debug で N = 1200 が 13.6 s → 0.68 s。
- **#126 入れ子の `for` の方程式の数に上限が無い**: `F::equation` の入口で、`eqs + ieqs + whens + connect の数` が `MAX_EQS`（100 万。`MAX_VARS` の 5 倍）を超えたら `too-large`（native の検査では GM-U の理由）で断る。1 本あたり debug で約 50 µs（100 万で 52 s）なので、試験では `cfg(test)` の上限を 1 万本にして、`flatten.rs` の中の単体試験で確かめる。
- **#127 `gen_connects` の葉の探索が線形**: 葉の名前 → 番号を `BTreeMap` にして引く（`id`）、集合の位置の走査は `a`・`a.…`・`a[…` の範囲だけを引く（`origins`）、`outside` は `own` の線形探索をやめて `.`・`[` の手前までの前置きを `HashSet` で引く（これも O(N²) だった）。debug で `connect(p, q)` N = 3000 が 6.7 s → 0.9 s。**残る**: 接続が無くても N = 8000 で 3.9 s かかる（部品の配列の展開そのものが 2.7 倍 / 2 倍で増える。`connect` の分は 0.4 s）。issue の範囲外なので別の WP の候補。
- **#130 次元の積の溢れ**: `by_dims` と `equalityConstraint` の戻り値の大きさの積を `dims_product`（`checked_mul`）にした。溢れたら見積もれない（`None`。それぞれ 1 本・0 本と数える）。
- **#131 `diff-frontend` の名前の比べが要素の欠けを隠す**: `without_elem_subs` を `BTreeMap<名前, (個数, 落とした添字の数)>` にした（`Elems`）。名前の集合が同じで、落とした添字の数が同じ（両方が同じ深さで展開した）名前は個数も同じでなければ不一致にする。Rumoca が配列を展開しない（`a.x` が 1 つ。深さ 0）ときは個数を比べない（S332 の措置は保つ）。

試験は、`globe-mode-flat/tests/flat.rs`（#121〜#125・#127・#130）、`flatten.rs` の単体試験（#126）、`xtask` の単体試験（#131）。計算量の試験は、増え方が変わったことだけを見る余裕のある上限（debug で 8〜20 秒）にしてある。

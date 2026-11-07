# S211 `fmt-issues-2` のレビュー（ponytail）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s211-fmt-issues-2` |
| 対象 | GitHub の issue #24・#26・#27・#28・#32（本文は `gh issue view <n>`。2026-11-07 にユーザーが立てた） |
| レビュアー | Claude Code の ponytail（`/ponytail-audit` 5.3.2）。コードを読んだだけで、ビルドも実行もしていない |
| 依頼の文面 | リポジトリ全体の監査（ユーザー） |
| 作業記録 | `log/work/2026-11-07-s211-fmt-issues-2.md` |

## 指摘（原文のまま）

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| #24 | 高（監査の 3/20） | `globe-mode-fmt/src/annotation.rs:87-95`、`lib.rs:155` | `apply` は図形プリミティブ（`Rectangle`・`Ellipse`・`Polygon`・`Line`・`Text`・`Bitmap`）の引数を並べ替え、`origin = {0, 0}` を補う。annotation の中かどうかを見ず、その名前の識別子に `(` が続けば一致する。`extends Pkg.Line(R = 1, L = 2);` が `extends Pkg.Line(origin = {0, 0}, R = 1, L = 2);` になり、クラスに `origin` の修飾子が無いのでコンパイルが通らなくなる。同じ名前の関数呼び出し・record のコンストラクタ・`model X = Lines.Line(...)` も同じ。「字句が変わっていない」の安全検査は `apply` の後に `expected` を作るので、並べ替えや追加を見つけられない。直し方: `apply` に `in_annotation: bool` を渡し、`annotation(` のグループの下だけで `sort_args` を呼ぶ。`extends X.Line(a = 1);` の試験を 1 つ足す | 採用 | main で再現した（`extends Pkg.Line(R = 1, L = 2)`、`model N = Lines.Line(…)`、`Real x = Text(b = 2, a = 1)` に `origin` が入る）。`Documentation(…)` の並べ替えと lint の `annotation-order` も同じ条件にそろえた |
| #26 | 中（5/20） | `globe-mode-cli/src/main.rs:699-731` | `fmt` の標準入力の分岐が `check` フラグを読んでいない。`cat bad.mo \| globe fmt --check -` は整形した本文を出して 0 で終わるので、標準入力を使う pre-commit や CI の門番は必ず通る。直し方: 標準入力の分岐で `check` のときは、`out != text` なら 1 で終え、標準出力に何も出さない | 採用 | main で再現した（終了コード 0）。`--json` との併用は JSON の契約（`ok`・`text`）を変えないため対象外 |
| #27 | 中（6/20） | `globe-mode-cli/src/main.rs:755` | 整形した本文を `std::fs::write(path, out)` で書き戻す。この呼び出しは先にファイルを空にしてから書く。ディスクが満杯か、途中でプロセスが殺されると `.mo` が途中で切れ、元の本文は戻らない。直し方: 同じフォルダの `path.with_extension("mo.tmp")` に書いてから `fs::rename` で上書きする | 採用（直し方は変更） | `ulimit -f` で再現した（6.4 MB が 512 KB に）。一時ファイル名は `<名前>.globe-tmp-<pid>`（`.mo.tmp` だと同じ名前の衝突と `foo.mo` と `foo.mo.tmp` の取り違えがあるので pid を付けた）。symlink は指す先を書き、権限を写し、読み取り専用は書かない |
| #28 | 高（7/20） | `globe-mode-fmt/src/layout.rs:39-103` | `seq` が括弧の段ごとに再帰する（`flatten`・`render`・`annotation::apply` も）。`{"text":"model A Real x=" + "("*100000 + "1" + ")"*100000 + "; end A;"}` は約 200 KB で、各フレームワークの本文の上限（1〜2 MB）より小さい。2 MB のワーカースタックを越え、スタックオーバーフローは `catch_unwind` で捕まえられないので、サーバがプロセスごと死ぬ。`/api/lint` も `lint.rs:125` が `format` を呼ぶので同じ。LSP ではそのファイルを開いた時点で `globe lsp` が死に、VS Code は数回再起動してあきらめる。直し方: `seq` に深さのカウンタを持たせ、256 段ほどを越えたら `err("括弧が深すぎる", …)` を返す | 採用（fmt 側）。検査（Rumoca）側は持ち越し | 再現した（`globe lint` が 100,000 段で main スレッドの 8 MB を溢れ、試験は SIGABRT）。上限は 256。Rumoca の `check` は 20,000 段で溢れるが、Rumoca は改変しないので直さない（解説書 2.3） |
| #32 | 中（11/20） | `globe-mode-core/src/normalize.rs:44`、`globe-mode-db/src/lib.rs:629`、`globe-mode-fmt/src/lexer.rs:312` | ユーザーのファイルから `\u{FEFF}` を取り除く所が無い。Windows のメモ帳が保存した BOM + `model M ... end M;` は 1:1 に S0 の構文エラー。整形器の字句解析は 0x80 以上のバイトを識別子の一部と見るので、最初の字句が `\u{FEFF}model` になり、クラスと認識されず「文の途中に end」で拒む。直し方: `GlobeDb::set_file` と `fmt::format`/`lint` の先頭で `strip_prefix('\u{feff}')`（書き戻すときは BOM を残す） | 採用（`set_file` は取り除かず空白 3 個に） | 再現した（`outline` が S0 の誤り 4 件、`fmt` が「対応しない end M」）。取り除くと、BOM 付きの本文で診断の範囲を行・桁に直す呼び出し元（CLI・LSP）が 3 バイトずれるので、空白 3 個に置き換えて位置を保つ。`normalize.rs` は `outline` が通らないので使わない |

## 結論

- 採用 5 件・保留 0 件・不採用 0 件。#28 の検査側（Rumoca の深い入れ子）は持ち越し。
- 指摘は 5 件とも正しかった。#27 の直し方の `with_extension("mo.tmp")` は pid 付きの名前にし、#32 の `strip_prefix` は `set_file` では空白置換にした。

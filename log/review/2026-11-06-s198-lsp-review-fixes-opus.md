# S198 `lsp-review-fixes` のレビュー（Opus/medium）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s198-lsp-review-fixes` |
| 対象 | main `cc846ba` の `globe-mode-lsp`（`definition.rs`・`completion.rs`・`lib.rs`）の参照・リネーム・補完。読むだけのレビュー |
| レビュアー | Opus（medium） |
| 依頼の文面 | LSP の参照・リネーム・補完を読んで、利用者の本文を壊す点と正しさの穴を探す |
| 作業記録 | `log/work/2026-11-06-lsp-review-fixes.md` |

## 指摘（原文のまま）

1. [本文を壊す・確認済み] globe-mode-lsp/src/definition.rs:347 component_refs が除くのは「前が .」だけ。`(` か `,` の直後で後ろが `=` の識別子（ほかの部品の修飾子、名前つきの引数）も参照に数える。`parameter Real amplitude = 1; Sine sine(amplitude = amplitude);` で amplitude を amp にすると `sine(amp = amp)`（class_of の 420 行目には同じ除外がある）。直し方: run の頭のトークンの前が ( か , で、後ろ tokens.get(i + width) が = なら除く。試験 1 本。
2. [本文を壊す・確認済み] definition.rs:337-339 範囲が「カーソルを含むいちばん内側のクラス」全体で、入れ子のクラスの同名の部品も拾う（`model A Real x; model B Real x; end B; equation x = 1; end A;` で A の x を y にすると B の Real x も）。直し方: c.classes のうち old を部品として宣言しているクラスの範囲のトークンを除く。試験 1 本。
3. [正しさ・確認済み] completion.rs:286-289（LIMIT）、lib.rs:294 上位 200 件で切り、配列で返す（isIncomplete = false 扱い）。`Modelica.Units.SI.`（type が 516 個）で Voltage・Velocity・Temperature が打ち足しても出ない。直し方: LSP は CompletionList { is_incomplete: out.len() == LIMIT, items }。UI（editor.ts:117）は 200 件のとき validFor を付けない。
4. [正しさ・確認済み] definition.rs:605-607 部品のリネームの衝突の検査が継承した部品を見ない（`model Base Real y; end Base; model A extends Base; Real x; end A;` で x を y に）。直し方: taken に r.component(q, path, class, n, 0).is_some() を足す。562 行目の shortcut の注記に無い穴。
5. [正しさ・可能性] definition.rs:572-594、lib.rs:411-418 クラスのリネームが、開いていないファイルの参照・within・ファイル名（M.mo）を見ない。直し方: 最上位のクラスは断る、またはワークスペースの根の下の .mo を字句で調べ class_hits があれば断る。意図した割り切りなら shortcut の注記に。
6. [安全・可能性] definition.rs:112-114,143-146,307 scratch に入れたライブラリのファイルが古いまま。hover_at の body[..t.range.start as usize] が範囲外・文字の途中で panic（LSP のプロセスが落ちる。handle に catch_unwind 無し）。直し方: 307 行を body.get(..)? に、library_class_range は毎回 set_file で読み直す。
7. [正しさ（低）・確認済み] definition.rs:331-336,596 引用符つきの識別子 'a b' の上で name_at が a だけを切り出し、部品 a が書き換わる。直し方: on_ident を「カーソルの下の識別子のトークンの text が segs の最後と等しい」に。
8. [簡素化] lib.rs:330-351 globe/definition と globe/hover が with_resolver と同じ組み立てを書き写している。約 -15 行。
9. [簡素化] lib.rs:409-411 リネームの口で class_name_at を 2 回。
10. [正しさ（小）] lib.rs:398-399 文書が開いていないとき globe/references は null、textDocument/references は []。globe/rename も null（-32803 でない）。直し方: [] と -32803。

試験が無い: 1・2・4・7。問題なし: UTF-16 の換算、CRLF、文書の末尾、空の文書、end の名前、import の別名、同名の入れ子のクラスの区別、repair、models、MSL での計算量。

## 採否

| # | 再現 | 採否 | 理由・試験 |
| --- | --- | --- | --- |
| 1 | した（`sine(amp = amp)` になった） | 採った | `is_modifier_name`（前が `(` か `,`、`each`・`final` を挟んでもよい、後ろが `=`）を `component_refs` と `class_of` で共用。試験 `component_rename_skips_modifier_and_named_argument_names`（`freqHz = 2, amplitude = amp`・`each final`・`f(amplitude = …)` の関数呼び出しも）。MSL の `Pulse pulse(amplitude = amplitude, …)` は `#[ignore]` の `msl_pulse_modifier_keeps_its_name`（`data/msl` で通る） |
| 2 | した（B の `Real x` と `x = 2` も `y` になった） | 採った | 同名の部品を宣言する入れ子のクラスの範囲を除く（`shadowing_classes`。その中の入れ子は見ない）。入れ子の側からのリネームで外側が変わらないことも。試験 `component_rename_stays_out_of_nested_classes_that_declare_it` |
| 3 | 切る順は指摘どおりではなかった（絞ってから切っていた）。配列で `isIncomplete` が無いのは事実 | 採った（形のみ） | `complete_list`: 200 件に達したら `{isIncomplete: true, items}`。LSP の `textDocument/completion`・`globe/completion` が使う。wasm の `completion` は配列のまま。試験 `typed_word_narrows_before_the_limit_cuts`（300 件の木で `Lib.` は 200 件、`Lib.Vol` は `Voltage`）、`completion_is_incomplete_only_when_the_limit_cuts`（RPC の形、開いていない文書も同じ形）。VS Code 拡張の `lsp.test.mjs` を `items` に |
| 4 | した（`y` への変更が通った） | 採った | `taken` に `r.component(q, path, class, n, 0)`（継承した部品を含む）。試験 `component_rename_sees_inherited_names` |
| 5 | 論証のとおり（`rename` が見るのは開いている文書だけ） | 採った（断る側。LSP の `textDocument/*` だけ） | 最上位のクラスが `M.mo`・`package.mo` に対応していれば断る。根の下の開いていない `.mo` を字句で調べ（`old` を含むファイルだけ lex）、同じ名前の識別子があれば断る。`globe/rename`（UI のメモリ上の文書）は S186 の「モデル名の変更」の流れを守るため対象外。試験 `class_rename_refuses_what_the_disk_would_contradict` |
| 6 | した（ホバーの範囲が古い本文とずれて別の行を返す・範囲外で panic。試験は両方とも直す前に落ちる） | 採った | `hover_at` は `get(..)?`。`Server::with_resolver` が scratch のライブラリのファイルの更新時刻を見て、変わったものを除いて読み直す。`serve` は 1 要求を `catch_unwind` で包み、panic なら -32603 を返す（panic の内容は標準エラー）。試験 `hover_on_a_file_that_changed_under_us_does_not_panic`・`library_file_changed_on_disk_is_reread`。`catch_unwind` 自体の試験は無い（panic を起こす口が無い） |
| 7 | した（`'a b'` の `a` の上で部品 `a` が書き換わる） | 採った | カーソルの下の識別子のトークンが `name_at` の最後の区切りと丸ごと一致するときだけ。試験 `quoted_identifier_is_not_cut_at_a_space` |
| 8 | （簡素化） | 採った | `globe/definition`・`globe/hover`・`textDocument/definition` が `with_resolver` を使う。`textDocument/definition` は開いている文書の本文を先に読むようになった |
| 9 | （簡素化） | 一部 | 他の開いた文書があるときだけ 2 回目を呼ぶ。`rename` の中の呼び出しとの重複は `rename` の署名を変えるので持ち越し |
| 10 | した | 採った | `globe/references` は `[]`、`globe/rename`・`textDocument/rename` は -32803。試験は `class_rename_refuses_what_the_disk_would_contradict` と `completion_is_incomplete_only_when_the_limit_cuts` の末尾 |

## 結論

- 採った 9 件・一部 1 件（9）。再現しなかった指摘は無い（3 は「切る順」の記述だけ事実と違った）。

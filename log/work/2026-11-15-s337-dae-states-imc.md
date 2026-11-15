# S337 `dae-states-imc`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s337-dae-states-imc`（main から切った） |
| worktree | `.git-worktree/s337-dae-states-imc` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S337-dae-states-imc/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

Track B の自前の DAE で、Rumoca と状態の数が合わない 2 本（`IMC_DCBraking` 18 対 23、`IMC_Initialize` 18 対 22）の原因を `diff-frontend --class … --dump` で確かめ、自前の誤りなら直す。Rumoca の誤りなら直さず、根拠を記録する。余力で `ShowTransferFunction`（flat の方程式 14 対 16）。回帰 0 が条件。

## 決めたこと

- **状態 2 本は自前の抜け**。`--dump` で Rumoca だけに `der(imc.is[k])`（3）・`der(imc.idq_rs[k])`（2）の未知数があり（どの行からも参照されない）、`IMC_Initialize` は `der(aimc.idq_sr[k])`・`der(aimc.idq_rr[k])`（2 + 2）。どれも `initial equation` の `der(x) = 0` の対象で、差の本数と一致した。Rumoca の `rumoca-phase-dae` の `analyze_model_roles` は `flat.equations.chain(&flat.initial_equations)` から `der` の対象を集める（コードの確認。写してはいない）。自前の `has_der` は連続側の方程式・`when`・algorithm・束縛だけだった。
- **Rumoca の誤りとはしない**。MLS 8.6（<https://specification.modelica.org/master/equations.html>、2026-11-15 確認）は、初期方程式にだけ現れる `der(x)` の `x` が状態かを決めていない（`der` を初期化の未知数に数えると書くだけ。この例は連続側にも `der(y)` がある）。Rumoca の規則は意図して書かれているので、doc/upstream.md には足さず、上流へも出さない。自前を Rumoca に合わせた。
- **`ShowTransferFunction` は自前の誤り**。`transferFunction` の方程式が Rumoca 7 本・自前 5 本。`aw = {… for i in 1:size(a,1)}`（`Complex aw[3]`）を 1 本と数えていた。左辺 `transferFunction.aw` は要素ごとに展開されて変数名が `aw[k].re`・`aw[k].im` になり、`shape` が大きさを引けなかった。要素の数（3）を数えるようにした。
- 共有 target では、同じ crate・同じ試験の成果物（`release/xtask`・試験のバイナリ・rlib）の名前が worktree をまたいで同じで、別の worktree のビルドが上書きすることがある（マージ後の試験でも、別の worktree の試験のバイナリが走って 1 本落ちた〔37 本だったのが、crate の lib.rs を touch して作り直したら自分の 36 本で通過〕。ADR-0039 の付記にも書いた）。途中で `--dump` の出力が食い違い（`ieq` の行が消え、状態の数が戻った）、気付いた。以降は、ビルド直後に自分の分を scratchpad へ `cp` して走らせた。全体の検査も同じ引数（`mise run msl:diff-frontend` と同じ）で、その写しを `flock` の下で走らせた。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `has_der` に初期方程式の `der` も足す（読めない初期方程式は黙って飛ばす shortcut。釣り合いには数えないので状態の数だけが変わる）。試験 1 本（`der_in_initial_equation_makes_state`）。
- `src/Rust/mod/globe-mode-flat/src/flatten.rs`: `record_array_len`（`t[k].x` の要素の数）を `shape` の ComponentRef の最後の手段に足す。試験 1 本（`record_array_equation_counts_one_per_element`。直す前は 2 で落ち、4 で通る）。
- `src/Rust/xtask/src/diff_frontend.rs`: `--dump` に初期方程式（`ieq[…]`）を足す。
- 文書: ADR-0039 付記（S337）、roadmap・architecture、`doc/frontend-diff.md`（生成）、セッション文書、索引。

## 検証

- `msl:diff-frontend` と同じ引数（`xtask diff-frontend --report doc/frontend-diff.md --json target/frontend-diff.json`。ビルド直後に写した release の xtask を `flock` の下で）: 744 s、最大 RSS 5.2 GiB。MSL の例題 566 本（S334 → S337）で状態の数 332/334 → **334/334**、flat の方程式の数 451/463 → **452/463**、DAE の判定・数・構造の正則 334/334 のまま。**回帰 0、改善 3 本**（`IMC_DCBraking`・`IMC_Initialize`: states、`ShowTransferFunction`: flat_eqs）。`GlobeModeValidation` 75 本は変わらず（73/73・73/73・73/73・正則 72/72）。
- 1 回目の全体の検査は、メモリ逼迫（used 7.6 GiB、swap 13 GiB。ほかの担当の全体検査と重なった）で `Spice3BenchmarkFourBitBinaryAdder` が 896 s かかり、`record_array_len` の変数の全走査が原因かと疑って、整列した `var_idx` の範囲走査に直して打ち切った。2 回目は 182 s（S334 は 156 s、S329 は 181 s）で、遅くなったのは負荷のため。範囲走査は残した（全走査より安い）。
- マージ（origin/main の S340・S342）の後に、変えた crate の lib.rs を touch して再ビルドし、試験・clippy を通し直した。コミットしたコードの release の xtask でも 3 本（`IMC_DCBraking`・`IMC_Initialize`・`ShowTransferFunction`）の states・flat_eqs が合うことを確かめた。
- `cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend -p globe-mode-lsp` 通過（doctest の 1 回は共有 target の rlib が別の worktree のビルドで消えて落ち、単独で回し直して通過）。`cargo clippy -p globe-mode-flat -p globe-mode-dae -p xtask --all-targets -- -D warnings` 0 件。`cargo fmt --check` 通過。
- `check_docs.py`: 全デッキ OK、リンク 5172 件中の切れ 0 件。`render_slides.py` は自分のデッキで問題 0 件（1 回目は Chromium のスクリーンショットが 30 s で時間切れ、撮り直して通過）
- セッション記録の `--check`: なし（専用の記録が無い）
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークンらしきもの）

## 残したこと

- flat の方程式の数の残り（Rumoca の数え方の誤りと見ている 12 本）。
- 初期方程式の `der` が連続側で一度も微分されない変数を状態に数えるのは Rumoca に合わせた形。実行（`globe-mode-exec`）は Rumoca のままなので影響しない。自前の DAE が実行・LSP につながるときに、この規則（`der(x)` の未知数が行から参照されない）をどう扱うかを決める。

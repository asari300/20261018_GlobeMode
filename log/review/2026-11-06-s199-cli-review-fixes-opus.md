# S199 `cli-review-fixes` のレビュー（Opus/medium、ponytail-review）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s199-cli-review-fixes` |
| 対象 | main `cc846ba` の CLI と frontend の案内（読むだけ・実行なし） |
| レビュアー | Opus/medium（ponytail-review） |
| 依頼の文面 | CLI（`globe-mode-cli`）と frontend の候補・案内の関数を、利用者のファイルを壊さないか・終了コード・簡素化の観点で読む |
| 作業記録 | `log/work/2026-11-06-s199-cli-review-fixes.md` |

## 指摘（原文のまま）

```text
# レビュー: CLI と frontend の案内（Opus/medium、ponytail-review、main cc846ba、読むだけ・実行なし）
1. [安全・確認済み] simulate.rs:225-231, 316 `-o` に入力の .mo を渡すと File::create がモデルのソースを CSV で上書きする（`globe simulate model.mo -m RC -o model.mo`。ライブラリのファイルも）。直し方: -o の拡張子が mo なら事前検査で終了コード 2。厳密には正規化したパスが files か library の配下なら拒む。
2. [安全・確認済み] main.rs:550-557 事前検査が壊れた symlink を消し、リンク先に空のファイルを残す（path.exists() はリンクを辿る）。直し方: `let existed = path.symlink_metadata().is_ok();`。
3. [正しさ+簡素化・確認済み] simulate.rs:386-404, 416-425 `-o -` のとき stderr の診断から notes と `ファイル:行:列` が落ちる（S180・S187 の候補・案内が出ない）。直し方: print_diagnostics に「stderr に出す」引数を足し、重複したループと severity() を消す（約 -20 行）。
4. [正しさ・確認済み] simulate.rs:270-272 `-o - | head` の BrokenPipe で即 0 を返し finish を飛ばす（Error の診断でも 0、--json の報告が出ない）。直し方: BrokenPipe のとき return せず finish に進む。
5. [正しさ・理屈は確認済み] main.rs:38-53 println! の置き換えマクロが失敗の終了コードを 0 に変える（`globe fmt --check src | head -5`、`globe check -m Broken | head -1` が pipefail の CI で 0）。Unix の慣習は SIGPIPE の 141。ENOSPC（> /dev/full）は panic で 101、文書の「書き出しの失敗は 2」と合わない。直し方: BrokenPipe は exit(141)（0 を保つなら ADR と試験）。ENOSPC は eprintln して exit(2)。
6. [文面・経路は確認済み] globe-mode-db/src/lib.rs:335-337 add_workspace_candidates がワークスペースの全モデルを 1 行に並べ、パッケージ名の診断にも付く（`check src/Modelica -m Typo`、`check model.mo -L data/msl -m Modelica.Blocks` に「候補: RC」）。直し方: notes が空のときだけ足し take(5)。
7. [文面・確認済み] rumoca.rs:1131,1147,1160 遠い開いている文書のモデルがライブラリの完全一致より先（並べ替えの鍵の先頭が !local）。直し方: 鍵を (tier, !local, …) に、または tier 2 以下があれば tier 3 を捨てる。
8. [負荷・可能性] rumoca.rs:1126-1145 長い -m（100 KB）で編集距離が数十秒。直し方: len > 64 なら編集距離を飛ばす。format!(".{query}") をループの外で 1 回。
9. [正しさ（軽）・確認済み] simulate.rs:234 ライブラリを読めないときの報告が -o - でも stdout に出る（CSV のはずのファイルに診断）。直し方: -o - のとき report_early_failure を stderr に。
10. [試験+簡素化・確認済み] `| head` の試験は write_result の道だけ（マクロの道 check・fmt --check・deps の試験なし）。-o の事前検査に「既存のファイルの中身と権限を変えない」「計算が失敗したら新しいファイルを残さない」の試験なし。main.rs 1,454 行（print_s7〜s10 と check の表示 約 300 行を別モジュールへ）。
問題なし: library:<パス> の戻し方（C:\・UNC・:）、負の数、--json の stdout、添字と unwrap。
```

## 採否

「読んで論証しただけ」の指摘なので、1 件ずつ失敗する試験（または古い `globe` での手動の再現）で確かめてから直した。

| # | 再現 | 採否 | 理由・試験 |
| --- | --- | --- | --- |
| 1 | 古い `globe` で `simulate m.mo -m Decay -o m.mo` が終了コード 0 で `m.mo` を CSV で上書きした | 採った | `-o` が入力のファイル（正規化して比較）・`-L` の配下・拡張子 `.mo`（大文字小文字を問わず、リンク先も）なら計算の前に終了コード 2。`fmu export --out` は、作り直す `<out>/<名前>/` に `.mo` か入力・ライブラリがあれば同じく 2（`remove_dir_all` が消すため）。試験 `output_never_overwrites_sources`・`fmu_export_does_not_remove_a_source_folder` |
| 2 | 古い `globe` で壊れた symlink が消え、リンク先に空のファイルが残った | 採った | `symlink_metadata` で調べ、リンク先の無い symlink は触らず終了コード 2。試験 `a_broken_symlink_is_neither_removed_nor_followed` |
| 3 | 読みで確認（`-o -` の stderr の診断は notes と位置を落とす道が別だった） | 採った | 表示行を作る `diagnostic_lines` を 1 つにして、標準出力・標準エラーの両方がそれを使う（重複していた約 20 行と `severity()` を削除）。試験 `with_stdout_output_diagnostics_go_to_stderr_in_full` |
| 4 | 読みで確認 | 採った | BrokenPipe でも `finish` まで進む（報告と終了コードは本来のまま）。試験 `closed_stdout_still_reports_the_run` |
| 5 | 古い `globe` は閉じた標準出力で何でも 0、`/dev/full` は panic（101）。**ユーザーが体系を決めた**: SIGPIPE の 141 にはしない | 採った（レビューの直し方とは別の形） | BrokenPipe は以後の出力を黙って捨て、処理は最後まで進めて本来の終了コードで終わる。それ以外の書き込みの失敗は 1 行出して終了コード 2。未対応（GM-U）だけで止まったら 3。`fmt` の標準入力は `-` を明示したときだけ。試験 `closed_stdout_keeps_the_exit_code`・`a_full_disk_is_exit_code_two_without_a_panic`・`unsupported_only_is_exit_code_three`・`stdin_needs_an_explicit_dash` |
| 6 | 再現（`Connect.Nope` の候補にワークスペースの全モデルが 1 行に並んだ） | 採った | 補足が空のときだけ、前からの一致が長い順に最大 5 件（「ほか n 件」）。パッケージ名の診断には足さない。試験 `workspace_candidates_are_five_at_most_and_skip_package_names` |
| 7 | 再現（`Decay` で `Net`（遠い開いている文書）が `A.Sub.Decay` より先） | 採った | 並べ替えの鍵を（段, 開いている文書が先, …）に。試験 `a_distant_open_document_does_not_outrank_a_library_match` |
| 8 | 再現した（程度は軽い）。小さなライブラリの試験は古いコードでも通る。MSL 4.1.0 で `check -L data/msl -m <100 KB の名前>` が release の古い `globe` で 4.8 s（負荷のある WSL。「数十秒」までは出なかった）、直した debug 版で 0.9 s | 採った | 名前が 64 文字を超えたら編集距離を飛ばす。`format!(".{query}")` をループの外へ。試験 `a_very_long_model_name_skips_the_spelling_distance`（5 秒以内） |
| 9 | 読みで確認 | 採った | ライブラリを読めない報告も `-o -` のときは標準エラーへ（`report_early_failure` に引数）。試験は 3 と同じ |
| 10 前半 | 試験が無いことを確認 | 採った | `| head` の道（check・fmt --check・deps）、事前検査が既存のファイルの中身・権限・更新時刻を変えないこと、失敗した計算が新しいファイルを残さないことの試験を足した（`the_precheck_leaves_files_alone_and_a_failed_run_leaves_none` ほか） |
| 10 後半 | — | 採った | `main.rs` の `check` の人間向けの表示 `print_s7`〜`print_s10` を `report.rs` へ行を移すだけ（別のコミット） |

## 結論

- 採った 10 件（10 は前半・後半）。不採用 0・持ち越し 0。
- レビューが誤った点: 5 の「BrokenPipe は exit(141)」は採らなかった（成功した検査を `| head` しただけで非 0 になり、`pipefail` の CI を壊すため。ユーザーの決定）。

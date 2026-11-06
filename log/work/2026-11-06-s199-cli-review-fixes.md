# S199 `cli-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s199-cli-review-fixes` |
| worktree | `.git-worktree/s199-cli-review-fixes` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S199-cli-review-fixes/` |
| セッション記録 | （親が写す） |

## 依頼

CLI と frontend の案内のレビュー（10 件、`log/review/2026-11-06-s199-cli-review-fixes-opus.md`）を、1 件ずつ失敗する試験で再現してから直す。利用者のファイルを壊す 2 件（指摘 1・2）を最優先。あわせてユーザーが「CLI は任せる」と決めた終了コードの体系（読み手が先に閉じても保つ、書き込みの失敗は 2、未対応だけは 3、`fmt` の標準入力は `-` を明示したときだけ、MSL の取得コマンドは足さず README に zip の手順）を実装する。

## 決めたこと

- **`fmt`・`lint` のパスなしの `--json` は標準入力のまま**にした。ユーザーの決定は「パスも `-` も無いときは使い方を出して 2」だが、`--json` はウェブアプリ 10 種・marimo・streamlit が頼る契約（[web-apps.md](../../doc/web-apps.md)）で、パスなしでは標準入力の意味しか無い。呼び出し元に手を入れずに済み、ターミナルでの `globe fmt` と `globe fmt --check` は使い方で 2 になる。
- **終了コード 3 は `check`・`simulate` だけ**（ユーザーの決定どおり）。`flat`・`fmu export` は未対応だけでも 1 のまま。分類は `unsupported_summary`（protocol）を使い、CLI に規則を置かない。
- **`fmu export --out`**: 指摘には無いが同じ観点で確かめた。`write_fmu` は `<out>/<名前>/` を消して作り直すので、そこに `.mo` か入力・ライブラリがあれば終了コード 2。`--out build` が入力のフォルダの中にある普通の使い方は断らない。
- 指摘 5 の「BrokenPipe は 141」は採らない（ユーザーの決定）。
- 壊れた symlink は「作って消す」道に入れず、リンク先が無いと言って 2 にした（指摘の直し `symlink_metadata` だけだと、リンクは残るがリンク先に空のファイルができる）。
- 指摘 6 の候補は、前からの一致が長い順に並べてから 5 件に切る（並べずに切ると任意の 5 件になる）。
- 共有の `git stash` スタックを別の worktree と取り違えて、隣の worktree（s200）の Python の変更を自分の worktree に popしてしまった。内容が同じことを確かめて `git checkout --` で戻し、自分の変更は書き直した。以後 `git stash` は使わない。

## やったこと

- `globe-mode-cli/src/main.rs`: 標準出力の置き換えマクロ（BrokenPipe は旗、それ以外は 1 行出して 2）、`check_writable_file`（`.mo`・入力・ライブラリ・壊れた symlink）、`prepare_out_dir`（FMU の作り直す先）、`diagnostic_lines`・`emit`・`failure_code`、`paths_or_stdin`（`-` だけ）、ヘルプの末尾の終了コードの表。
- `simulate.rs`: `-o -` のときの診断・早期失敗の報告を stderr に、BrokenPipe で `finish` まで進む、未対応だけなら 3。
- `report.rs`（新規）: `print_s7`〜`print_s10` を行を移すだけで分けた。
- `globe-mode-frontend/src/rumoca.rs`: 並べ替えの鍵、64 文字を超える名前は編集距離を飛ばす。
- `globe-mode-db/src/lib.rs`: `add_workspace_candidates`（補足が空のときだけ、最大 5 件、前からの一致順）。
- 試験: `globe-mode-cli/tests/review_fixes.rs`（12 本）、`library_hints.rs` に 2 本、`globe-mode-db/tests/deps.rs` に 1 本、`fmt_dirs.rs` の標準入力の試験を `fmt -` に。
- 文書: README（MSL の zip 3 行・終了コードの表・`fmt -`）、`doc/architecture.md`・`doc/format-style.md`・`doc/features.md`・`doc/roadmap.md`・CHANGELOG、`wasm/node/compare.mjs` のコメント 1 行（終了コード 3 を足した）。

## 検証

（締めで埋める）

## 残したこと

- `flat`・`fmu export` の失敗の終了コード 3。
- Windows の CRLF の fixture での確認は CI（`ci.yml` の全体検査）に任せる。symlink・SIGPIPE・`/dev/full` の試験は `#[cfg(unix)]`。

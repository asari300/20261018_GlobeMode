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

- 再現（「前」）: 共有 target の古い `globe`（他の worktree が上書きした main 相当の版）で `review_fixes` の 12 本を回すと 11 本が落ち、1 本（事前検査が既存のファイルを変えない）だけ通った（この 1 本は退行を防ぐ試験）。落ち方: 終了コード 1 が 3 でない・0 が 2 でない・`/dev/full` で panic（101）・`-o -` の診断に位置が無い・候補が出る・symlink とソースが残らない。古い release の `globe` では `simulate m.mo -o m.mo` が終了コード 0 で `m.mo` を CSV にし、壊れた symlink が消えてリンク先に空のファイルが残るのを手で再現した。frontend の並べ替え（`library_hints`）と db の候補（`deps`）の試験は直す前に落ちた。
- 後: `cargo test -p globe-mode-cli` 全部 OK（`review_fixes` 12 本）、`-p globe-mode-frontend -p globe-mode-db` 全部 OK、MSL の `#[ignore]` 3 本（`data/msl` あり）OK。`cargo test --workspace --no-fail-fast` は CLI の 2 ターゲットだけ落ち（古い `globe` を使った `review_fixes` と、zig が PATH に無かった `zig_cross_builds_a_win64_dll`）、ほかはすべて OK。`mise run check` は `cargo fmt --all --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`test_ci_plan.py` OK、`cargo deny check` OK、`fmt --check src/Modelica` 71 ファイル整形済み、`uv sync --locked` OK。
- `mise run msl:compare`: 20 本中 20 本が一致。`mise run wasm:node`: OK。`mise run wasm:compare`: 387 / 400 件が一致、既知の差 13 件、一致しない 0 件（前と同じ）。
- `msl:funnel`・`msl:sim` は回していない（検査の規則・実行系を変えていない。CLI の終了コードを数えている所は無いことを `.github`・`mise.toml`・xtask・`wasm/node/compare.mjs`・各言語のラッパで確かめた。`compare.mjs` は `execFileSync` の例外から stdout を取るので 3 でも動く）。
- 指摘 8: MSL 4.1.0 で `check -L data/msl -m <100 KB の名前>` が release の古い `globe` で 4.8 s、直した版（debug）で 0.9 s。
- `check_docs.py`: デッキ 195 件 OK（S199 を含む）/ リンク 3951 件中の切れ 0 件。`render_slides.py`（S199 のデッキ）: 問題 0 件。
- セッション記録の `--check`・ステージした差分の漏洩検査: 親が写してから（セッション記録は `（親が写す）`）。差分の検査は下のとおり。
- push した feature の CI: ci.yml（full=true）・fmu-export.yml・resources.yml の run の ID と結果は下の「CI」。

### CI

| workflow | run | 結果 |
| --- | --- | --- |
| `ci.yml`（`full=true`、Windows を含む 11 ジョブ） | 42170428082 | success |
| `fmu-export.yml` | 42170453966 | success |
| `resources.yml` | 42170457555 | ジョブ `rss`（CLI の `check`・`simulate`・`deps`、`-o /dev/null`）は success。ジョブ `wasm`（wasm の線形メモリ。CLI の終了コードは使わない）は報告の時点で実行中 |

### 共有 target の落とし穴（今回の発見）

worktree が違っても、workspace 内の crate の成果物のハッシュは同じなので、共有 target では最後にビルドした worktree の成果物が「新しい」ことになる。ほかの worktree のビルドの後に自分の `cargo test` を回すと、自分のソースの mtime のほうが古いために `Fresh` と判断され、**他人の古い `globe` で試験が回る**（今回は `review_fixes` が 11 本落ちた）。変えた crate のソースを `touch` してから `cargo test` を回すと直る（memory の「共有 target の古いビルド」のとおり）。`CARGO_BIN_EXE_globe` は `target/debug/globe` を指すので、`cargo test --workspace` のように長く回すと途中で上書きされることもある。

## 残したこと

- `flat`・`fmu export` の失敗の終了コード 3。
- Windows の CRLF の fixture での確認は CI（`ci.yml` の全体検査）に任せる。symlink・SIGPIPE・`/dev/full` の試験は `#[cfg(unix)]`。

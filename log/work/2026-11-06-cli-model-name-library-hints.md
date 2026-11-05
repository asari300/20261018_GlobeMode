# S180 `cli-model-name-library-hints`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s180-cli-model-name-library-hints` |
| worktree | `.git-worktree/s180-cli-model-name-library-hints` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S180-cli-model-name-library-hints/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S178 の監査（`doc/research/cli-python-flow-2026-11-06.md`）の穴 1・2（WP の案 A1）。ライブラリを使うときのモデル名の誤りに候補つきの「見つからない」を出す。`-L` も FILES も無いときと、`-L` に MSL のサブフォルダを渡したときの案内を出す。判定は Rust の 1 か所。Python の `Workspace("data/msl")` は S181、`main.rs` の `-o` なし・負の時刻・ヘルプの例は S182。

## 決めたこと

- 判定の場所は `globe-mode-frontend` の `Checker::check_synced_inner` の失敗の分岐。CLI・Python・LSP・wasm・UI の検査がすべて通る 1 か所。
- 「見つからない」と言う条件は、Rumoca が要求したモデルの結果も失敗の記録も持たず、かつ索引と Session のどちらにも名前が無いとき。名前が正しい失敗とパッケージ名は今のまま。索引が空のときも言わない。
- 名前の索引は S167 の `LibraryTree`。最初は Session の文書の構文木だけで作ったが、Rumoca の文書は入れ子のクラスを読み込みの途中までしか持たず（`-m PID_Controller` の 1 回目は 2,556 件しか見えず、候補が出なかった）、`LibraryTree` に替えた。ネイティブはライブラリのファイルを失敗のときだけ読み直す。`globe-mode-fmt`（依存 0）を frontend の依存に足した。
- サブフォルダの案内は warning にせず、失敗した `GM-E1001` の補足にした。成功する検査には何も足さない。親は自動で足さない。
- `-L` も FILES も無いときは、5 つの副命令の `required_unless_present` を触らず、`Cli::try_parse` で clap の `<FILES>` の誤りだけを拾って例を足す（終了コードは 2 のまま）。

## やったこと

- `globe-mode-frontend/src/rumoca.rs`: `library_model_not_found`・`edit_distance`・`library_root_hint`。`globe-mode-frontend/Cargo.toml` に `globe-mode-fmt`。
- `globe-mode-fmt/src/tree.rs`: `LibraryTree::classes()`（全クラスの名前と種類）。
- `globe-mode-cli/src/main.rs`: `-L` / `FILES` の検査の所（`main` の最初）だけ。
- 試験: `globe-mode-frontend/tests/library_hints.rs`（新規。4 本、うち MSL の 1 本は `#[ignore]`）、`globe-mode-cli/tests/json_snapshots.rs` と `tests/snapshots/check-library-model-not-found.json`、`tests/simulate.rs`。
- 文書: 索引、`doc/features.md`・`roadmap.md`・`architecture.md`・`CHANGELOG.md` の該当行。

## 検証

- 再現（S178 の記述どおり）: 直す前の `globe`（共有 target の debug）で 3 通りとも再現した。
- 直したあと（MSL 4.1.0、debug）: `-m PID_Controller` と `-m Modelica.Blocks.Examples.PID_Contoller` と `…PID_Controller.mo` に `GM-E1002` と `候補: Modelica.Blocks.Examples.PID_Controller`。`-m zzzz` は完全修飾名の案内。`-m Modelica.Blocks`（パッケージ）は今のまま。`-L` なしは例を足した。サブフォルダは `GM-E1001` に補足（試験で確認。CLI の手元の出力では `head` で切れていた）。
- 時間（debug）: 短い名前の失敗は 0.91 s から 0.71 s（雑音の範囲）、ドット入りは 5.8 s から 6.3 s。ドット入りが遅いのは前からの Rumoca の閉包（索引を作る分は 0.1 s 前後）。
- コードの検査（`mise run check`、fmt / clippy / test / deny / uv sync）: 通った。テスト 678 本 pass、0 fail、13 ignored。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。
- `mise run wasm:node`: ok（release の wasm。frontend と fmt が wasm に入るため回した）。
- `msl:funnel`・`msl:sim`: 回していない。変えたのは失敗の診断の文面と、名前を引けなかった形のときのコードだけで、MSL の例題 566 本は実在の名前を渡すので OK・GM-U・NG の本数は変わらない（「見つからない」は今も NG。funnel が NG を数えるのは検査を通らなかった例題）。`msl:compare` の 20 本が一致したことと合わせて、判定は変えていないと判断した。確かめる必要が出たら `gh workflow run msl-sim.yml --ref feature/s180-cli-model-name-library-hints`。
- `check_docs.py`: 切れ 0 件。`render_slides.py`（S180 のデッキだけ）: 問題 0 件。
- セッション記録のコピーと `--check`、漏洩検査、main へのマージ・push は親が行う（依頼どおり）。

## 残したこと

- 短い名前を完全修飾名に補う提案、パッケージ名を渡したときの案内（Rumoca の原文のまま）。
- 判定を検査の前に置くとドット入りの誤りが速くなるが、毎回の検査に索引が要るので今はしない。
- Python の `Workspace("data/msl")` は S181。

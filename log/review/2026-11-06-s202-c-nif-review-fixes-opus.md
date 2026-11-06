# S202 `c-nif-review-fixes` のレビュー（Opus/medium、ponytail-review）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s202-c-nif-review-fixes` |
| 対象 | main cc846ba の `globe-mode-c`・`globe-mode-nif`（読むだけのレビュー） |
| レビュアー | Opus/medium（読んで論証しただけ） |
| 依頼の文面 | C ABI と NIF のレビュー。指摘 1〜6 を S202 で直す |
| 作業記録 | `log/work/2026-11-06-c-nif-review-fixes.md` |

## 指摘（原文のまま）

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 1 | 負荷・確認済み | globe-mode-nif/src/lib.rs:60-63 | `set_file` だけ通常のスケジューラ。別プロセスが simulate（DirtyCpu）で Ws の Mutex を持つ間、set_file が BEAM の通常スケジューラを最大 150〜180 s 止める。直し方: `#[rustler::nif(schedule = "DirtyCpu")]`。 | 採用（DirtyIo） | 再現した（`+S 1` 相当の試験で通常のスケジューラが 4.5 s 止まる。`test/scheduler_test.exs`）。DirtyCpu でなく DirtyIo にした: Mutex を待つだけで CPU を使わず、DirtyCpu はスケジューラ数と同じ本数しか無い（`+S 1` なら 1 本）ので、待つ `set_file` が実行の席を取り合う。`load_dir`・`export_fmu` と同じ側。 |
| 2 | 安全・確認済み | globe-mode-c/src/lib.rs:324-331,390-397、include/globe_mode.h:57-60 | `gm_simulate_observed` が &mut Workspace を持ったまま callback を呼ぶ。callback の中で同じ ws の gm_* を呼ぶと &mut の別名（UB）、gm_workspace_free なら use-after-free。直し方: ヘッダと # Safety に「callback の中で同じ ws の gm_* を呼ばない・放さない」。守るなら GmWorkspace に busy: Cell<bool> を持たせ ws_arg が使用中なら {"error"}。 | 採用（守りを入れた） | 注意書きだけにせず使用中の印（`AtomicBool`）を入れた。印は生のポインタからそのフィールドだけを読み、印を見る前に `&mut GmWorkspace`・`&mut Workspace` を作らない。コールバックの中の `gm_check` などは誤り、`gm_workspace_free` は放さず stderr に 1 行。UB は試験で「再現」できない（旧実装ではコールバックの中の `gm_check` が成功してしまう）ので、新しい挙動を Rust の単体・Zig・Julia の試験で固定した。Miri は環境に無く回していない。 |
| 3 | 負荷・可能性 | globe-mode-nif/src/lib.rs:226-235 | `format`・`lint` が通常のスケジューラ。DirtyCpu に。 | 採用 | 再現した（1 MB の本文で lint 2.3 s・format 0.09 s。1 MB の `lint` が通常のスケジューラを 2.2 s 止める）。DirtyCpu に。 |
| 4 | 試験・確認済み | globe-mode-c/tests/header.rs | 関数の署名だけ。GmProgress の並び・大きさを試験していない。直し方: size_of/offset_of の assert 1 行（40, has_time=16, stop_time=32）。 | 採用 | lib の単体に `size_of`・`offset_of`、`tests/header.rs` にヘッダの `typedef struct GmProgress` を C の規則で並べて Rust と突き合わせる試験。 |
| 5 | 簡素化・確認済み | C と NIF | 同じコードが 4 か所（request()、outline の誤りの文面、列の応答の JSON の形、段の名前 format!("{:?}").to_lowercase()）。facade に寄せる。約 -30 行。Python・wasm にも同じ形があるかは未確認。 | 採用（C と NIF だけ） | `simulate_request`・`Simulated::layout_json`・`ExecStage::as_str`・`Workspace::outline_json` を facade・exec に足し、C と NIF から使った。応答の JSON は変わらない（Zig・Julia・Elixir・Gleam の既存の試験が通る）。Python・wasm・LSP にも同じ形がある（Python の `request()` は同一、段の名前は 4 か所）。今回は触らず、対応表の 4 章と roadmap の持ち越しに書いた。 |
| 6 | 安全（軽）・可能性 | globe-mode-c/src/lib.rs:169-174 | gm_workspace_free の Drop が catch_unwind の外（panic で abort）。catch_unwind で包む。 | 採用 | Drop が panic する Workspace を作る道が無いので試験での再現はしていない（読んで確かめた）。`catch_unwind` で包み、panic は stderr に出す。 |

問題なし（レビュアー）: NULL の引数、UTF-8 でない文字列、返す文字列の NUL、列のバッファの対、panic の境界、コールバックのスレッド、署名、NIF の進捗の送信、binary の長さ、Mutex。

## 結論

- 採用 6 件・保留 0 件・不採用 0 件。1・3 は試験で再現、2・6 は再現できない（2 は新しい挙動を試験で固定、6 は読んで確かめた）。

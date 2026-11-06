# S197 `desktop-review-fixes` のレビュー（Opus/medium）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s197-desktop-review-fixes` |
| 対象 | main cc846ba の `src/Rust/apps/desktop/`（読むだけのレビュー） |
| レビュアー | Opus/medium（読んで論証しただけ） |
| 依頼の文面 | Desktop のシェルのレビュー。2〜6 を S197 で直す（1・7 は S196、8 は持ち越し） |
| 作業記録 | `log/work/2026-11-06-desktop-review-fixes.md` |

## 指摘（原文のまま）

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 2 | 正しさ・負荷 | `worker.rs:253-265,298` | globe/check などは予算 None で rx.recv() を無期限に待ち、Running も登録されないので cancel_run が効かない。Worker の Mutex を握ったままで rpc・notify・fetch_msl が全部待つ。直し方: exchange で予算に関係なく Running を登録、wait は deadline が None でも POLL ごとに asked を見る（2 秒で kill して作り直す。ADR-0018 と矛盾しない）。 | 採用 | 再現した（試験 `cancel_kills_a_silent_check_without_budget` が登録を外すと「Running に登録されない」で落ちる）。exchange で常に登録し、wait は POLL ごとに asked を見る。 |
| 3 | 安全・可能性 | `main.rs:52,67`、`tauri.conf.json` の csp: null | rpc / notify は method を検査せず中継、globe/exportFmu の outDir は任意のディレクトリ。注入の道は見つかっていない。直し方: CSP を入れる、rpc の method を限る、exportFmu の限界を ADR-0023 に。 | 採用 | method の許可（`shell.rs`、UI の呼び出しとの突き合わせ試験つき）、CSP（index.html の meta と同じ＋ipc。試験で一致を確認）、ADR-0023 に限界。CSP の実機は desktop.yml・android.yml の run で確認（作業記録）。 |
| 4 | 正しさ・確認済み | `main.rs:159-162` | install_msl が libs と set_libraries を先に書き換え、ensure_live が失敗しても戻さない。 | 採用 | 再現した（試験 `failed_restart_restores_libraries_and_releases_the_flag`。壊れた sidecar で `libraries` が残る）。失敗したら `set_libraries(current)` に戻し、`Libraries` は成功後に書く。 |
| 5 | 正しさ・可能性 | `msl.rs:60-62,98`、`main.rs:157` | fetch_msl に同時実行の排他が無い。HEAD にタイムアウト無し、展開中に cancel を見ない。 | 採用 | 排他（取得中の旗。同じ試験で 2 つ目が「取得中」）、HEAD に `--max-time 10`、展開のループで取り消し（試験 `install_stops_when_cancelled_while_extracting`）。HEAD のタイムアウトは試験なし（curl の引数）。 |
| 6 | 試験・確認済み | `worker.rs:286-291` | 中断して 2 秒で kill する道に CI で回る試験が無い。 | 採用 | 2 の試験がこれ。応えない sidecar を立て、中断の後 5 秒未満で誤りが返り、pid が無くなる（`GLOBE_BIN` 不要）。 |

指摘 1・7（`tauri-backend.ts`）は S196、8（`reload_config` の削除。TS も触る）は持ち越し。

## 結論

- 採用 5 件・保留 0 件・不採用 0 件（8 は持ち越し）。

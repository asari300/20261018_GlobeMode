# S203 `misc-issues-triage`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s203-misc-issues-triage` |
| worktree | `.git-worktree/s203-misc-issues-triage` |
| Phase | P3・P6・P8（S012 のレビューの取りこぼし） |
| セッション文書 | `doc/sessions/S203-misc-issues-triage/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

2026-11-06 のユーザーの指示「Issue、PR を確認し、必要な修正があればワークツリーで修正」。残りの open の issue 8 件（#2・#7・#8・#10・#11・#12・#13・#16）を `gh issue view` で全文読み、いまの main で 1 件ずつ再現して振り分ける。S201（`web/*.js`・`worker-node.mjs`）、S198・S199・S200・S202 と並行なので、それらのファイルは触らない。

## 決めたこと

- #2 は facade（`globe-mode/src/lib.rs`）で直す。issue の行は S035 で facade に移っていた。`ok` を `report.run.is_some() && 診断に Error が無い` にした。強行した結果の列・CSV・`final_values` はそのまま返る。
- #12 は入口で消す（`take_last_panic()` は JS の変更が要るので採らない）。`parse_request`（3 つの実行の入口が通る）と `js_new`。
- #13 は実際の `simulate` では起きない（時系列が必ず整う）が、`is_ok()` で捨てる形は直した。`csv_text()` に分けて単体で試す。
- #8 の上限は観測した最大の 3 倍以上。issue の一律 1e-3 は、いまの観測（3.1e-3、dopri853 の 1.8）で落ちるので使わない。
- #7 は `.cargo/*` に加えて `.gitattributes` も全体の検査にした（S004 の Windows の CRLF の失敗の原因）。別の workspace の `src/Rust/apps/web-*/.cargo` は対象外。`Astro` は CI が無い（ci_plan の漏れではない）ので「いま残っているもの」の表に。
- #10 は `tube_failed()` を消し、`ref_failed += 1` の分岐の中で数える。報告の `.min()` も不要になった。
- #16 は xtask の `None` の分岐を残した（型の変更が `sim.rs` の大半に及ぶ）ので `Fixes` を付けない。`memory.mjs` の写しは S201 の領域に近いので残した。
- ソルバの挙動は変えていない（`msl:sim`・`msl:funnel` は回さない）。`ok` の意味が変わるのは強行して Error が残るときだけ。

## やったこと

- `globe-mode/src/lib.rs`: `ok` の式（#2）、`csv_text()`（#13）と単体の試験。
- `globe-mode-wasm/src/lib.rs`: `clear_last_panic()`（#12）と単体の試験。`tests/decay.rs`: `!forced.ok`（#2）。
- `globe-mode-wasm/node/`: `csv-compare.mjs`（`compareCsv`・`withinKnownBound`）、`csv-compare.test.mjs`、`common.mjs`、`compare.mjs`・`smoke.mjs`・`simulate.mjs` の整理（#8・#11・#16）。`mise.toml` の `wasm:compare` に `node --test` を足した。
- `.github/scripts/ci_plan.py`・`test_ci_plan.py`（#7）。
- `xtask/src/sim.rs`（#10）。
- `globe-mode-exec/.../fixed.rs`、`globe-mode-sim/src/integrators.rs`（#16 の使われていない行）、`globe-mode-sim/tests/omc_mat.rs` と `tests/data/omc_decay_res.mat`（omc 1.27.2 が書いた実物）。
- 文書: 解説書、スライド、索引、roadmap、features.md、CHANGELOG。

## 検証

- 直す前に落ちることを確かめた試験: #2（`!forced.ok`）、#12（`js_new` のあとも古い文言が残る）、#10（`(1, 2)` で落ちる）。#8・#11 は `csv-compare.test.mjs` を書いて通した（旧 `compareCsv` は inline で試験できない）。
- `wasm:compare` の基準（直す前、ただし #2 の facade の変更は入れた後）: 387 / 400 件が一致、既知の差 13 件、一致しない 0 件。
- `mise run check`: 通った（fmt・clippy -D warnings・test・deny〈advisories・bans・licenses・sources ok〉・`globe fmt --check src/Modelica`・`uv sync --locked`）。最初の 1 回は facade の試験の整形で落ち、`cargo fmt` で直した。
- `cargo test -p globe-mode-wasm -p globe-mode -p globe-mode-sim -p xtask -p globe-mode-exec`: 通った（`omc_mat` は最初、omc が最後の時刻を 2 行書くのを数え違えて落ち、試験の期待を直した）。
- `python3 .github/scripts/test_ci_plan.py`: 25 件 OK。
- `mise run wasm:node`: 通った。`mise run wasm:compare`: `node --test` 3 件通り、387 / 400 件が一致、既知の差 13 件、一致しない 0 件（直す前と同じ）。
- `msl:compare`: 回していない（`globe-mode-frontend` は触っていない。`globe-mode-exec` の差は `fixed.rs` の通らない検査の削除だけ）。`msl:funnel`・`msl:sim`: 回していない（ソルバの挙動は変えていない）。
- `check_docs.py`: 通った。`render_slides.py`（S203 のデッキだけ）: 7 枚 × 3、問題 0 件。
- セッション記録の `--check`: 親が写す。
- 全体検査（`ci.yml -f full=true`、feature ブランチ）: run 42176173992 success（全ジョブ）。

## 残したこと

- `src/Astro/fmt` に CI が無い。
- xtask の `solver`・`known_deviations` の `None` の分岐。
- `memory.mjs` の `readMo`。

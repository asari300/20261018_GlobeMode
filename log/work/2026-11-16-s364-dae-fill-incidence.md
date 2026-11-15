# S364 `dae-fill-incidence`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s364-dae-fill-incidence`（`feature/s361-flat-outer-connect` から切り、S361 が main に入ったあと `origin/main` を取り込んだ） |
| worktree | `.git-worktree/s364-dae-fill-incidence` |
| Phase | Track B（B4 DAE） |
| セッション文書 | `doc/sessions/S364-dae-fill-incidence/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S361・S362 のあと、Rumoca の釣り合いまで通った 377 本で構造の正則が 373/377。合わない 4 本を `cargo xtask diff-frontend --class <名前> --dump` で確かめ、自前の誤りなら直す（`ExecutionPaths`・`ShowCompositeStep`・`ShowExceptions` は `join.occupied = fill(outPort.occupied, nBranches)` の行の incidence が自前で空になる理由が未確認、`BatchPlant_StandardWater` は構造の正則だけ合わない）。Rumoca の誤りなら直さず、根拠を記録する（上流には出さない）。小さな回帰試験を 1 本。最後に全体の検査を 1 回回し、数字を ADR-0039 の付記・roadmap・README・CHANGELOG・features に書く。

## 決めたこと

- **base は `feature/s361-flat-outer-connect`**（依頼のとおり。S361 が main に未着だった）。作業の途中で S361 が main に入ったので、`origin/main` を取り込んだ（衝突なし）。回帰の基準は親が写した `target-shared/bin-copies/frontend-diff-s361.json`（S361 の全体の検査の JSON）を `target/frontend-diff.json` に置いた。
- **原因は 4 本とも同じ 1 つで、自前の誤り**。
  - `ExecutionPaths` の `--dump`: 自前の行 47・Rumoca の行 40・未知数はどちらも 40。自前にだけある 7 行（`unmatched row[27..33]`）は、すべて `{alternative.join[1].occupied, …} = fill(…)`・`{Parallel1.split[1].set, …} = fill(…)`・`{Parallel1.join[1].reset, …} = fill(…)` で incidence が空。Rumoca の行にはこの方程式が無い（離散の方程式は連続の行にならない）。
  - 変数は `Boolean`（`occupied`・`set`・`reset`）なので離散。DAE が離散の方程式にするかは `defines_discrete`（左辺の最初の式が離散の変数か）で決めるが、**成分参照のときだけ**調べ、配列の構築子 `{…}` は偽にしていた。連続の行になった方程式は、連続の未知数に触れず incidence が空になる。方程式と未知数の数は離散の数が `count` で合っていたので元から一致していて、構造の正則だけが落ちた。
  - `BatchPlant_StandardWater` は、直す前の `xtask-s361d` の `--dump` で `unmatched row` が `controller.Parallel1` の `split`・`join` の同じ形の 4 行だった（S362 が「原因は確かめていない」としていたもの）。
  - Rumoca の誤りは無い。上流の下書きは足さない。
- **直し**: `defines_discrete`（`globe-mode-dae/src/lib.rs`）に、左辺が配列の構築子で要素がすべて離散の変数なら真、を足した。それ以外は今までどおり（最初の式が離散の成分参照か）。離散と連続が混ざった構築子の左辺は連続の行のまま（shortcut。MSL の例題に出ない）。
- **試験 1 本**: `fill_into_discrete_array_constructor_is_discrete_equation`。`connector P Boolean occupied; end P;`・`P join[2]`・`Boolean b` で、`join.occupied = fill(b, 2)` の行が、直す前は連続の行 2 本・離散 1 本（`(2, 1)`）、直した後は連続 0 本・離散 3 本で、釣り合って正則。直す前の `lib.rs` で落ちることを確かめた。
- **全体の検査は 1 回**（`xtask` をビルドした直後に `bin-copies/xtask-s364` へ写し、`flock` の下で回した）。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `defines_discrete` の変更（`disc` のクロージャに括り出して配列の構築子の場合を足した）と試験 1 本。
- 文書: ADR-0039 付記（S364）と推移の表・主題の表・S361 の付記の「確かめていない」への注記、roadmap・README・CHANGELOG・features・architecture の数字、セッション文書（`doc/sessions/S364-dae-fill-incidence/`）、索引。`doc/frontend-diff.md` は全体の検査が書き直した（手では触っていない）。

## 検証

- **全体の `diff-frontend`**（基準は S361 の JSON）: MSL の例題 566 本で、平らにできた 548/566（変わらず）、Rumoca も通った 521 本で名前 518/521・方程式の数 508/521（変わらず）、DAE 548/548、Rumoca の釣り合いまで通った 377 本で判定 377/377・方程式と未知数の数 376/377・状態 376/377（変わらず）・構造の正則 373 → **377/377**。`GlobeModeValidation` 75 本は変わらず（Rumoca も通った 74 本で 74/74・正則 73/73）。**回帰 0 本、改善 4 本**（`ExecutionPaths`・`ShowCompositeStep`・`ShowExceptions`・`BatchPlant_StandardWater`、4 本とも構造の正則だけ）。1003 s、最大 RSS 5.5 GiB。
- `diff-frontend --class` の 4 本（直したあと）: 4 本とも全項目が一致（`ExecutionPaths` 10 s・`ShowCompositeStep` 11 s・`ShowExceptions` 18 s・`BatchPlant_StandardWater` 42 s）。
- `cargo test -p globe-mode-dae`: 18 本合格（新しい 1 本。直す前の `lib.rs` では `(2, 1)` で落ちる）。`cargo fmt --all --check`・`cargo clippy -p globe-mode-dae -p xtask --all-targets -- -D warnings`: 通過。`cargo test -p globe-mode-dae -p globe-mode-frontend -p xtask`: 117 本合格・失敗 0。`cargo test -p globe-mode-db -p globe-mode-cli`（`globe-mode-dae` の依存元。native の検査の試験）: 204 本合格・失敗 0。`mise run ci:plan`: 変更したファイル 1 本の時点で `deny` なし。
- `check_docs.py`: 全デッキ OK、相対リンク 5495 件中の切れ 0 件。`render_slides.py`（S364 のデッキだけ。1 回目は他の担当の `msl:diff-frontend`〔5.4 GiB〕と重なって screenshot が時間切れ、負荷が下がってから撮り直し）: 問題 0 件。markdownlint-cli2（MD013 を除く）: 解説書・作業記録で 0 件。`mise run check` の全体は回していない（変えた crate と依存元に絞った）。
- セッション記録: 専用の記録が `log/local/` に無いので省いた。コミットの差分のメールアドレス・ホームのパス・トークンの検査: 0 件。

## 残したこと

- 残る 2 本の食い違い（`InverseParameterization` の方程式と未知数の数、`HeatingSystem` の状態）は Rumoca の側。自前は合わせない（S362。上流の下書き 40・07。提出しない）。
- 離散と連続が混ざった配列の構築子の左辺（`{x, b} = …`）は連続の行のまま。MSL の例題に出ない。
- 関数の本体を見て欄を絞る incidence は作らない（S362 のまま）。
- B3 が断る 18 本（`composite-binding` 6・`expandable` 5・`non-constant-dim` 4 ほか）。

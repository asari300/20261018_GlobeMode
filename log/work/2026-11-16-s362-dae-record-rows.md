# S362 `dae-record-rows`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s362-dae-record-rows`（`feature/s355-flat-stream` の 4d916cb0 から切った。S355 が main に未着のため） |
| worktree | `.git-worktree/s362-dae-record-rows` |
| Phase | Track B（B4 DAE） |
| セッション文書 | `doc/sessions/S362-dae-record-rows/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S355 で `stream` が平らになり、Rumoca の釣り合いまで通った 367 本で、判定 365・方程式と未知数の数 364・状態 366・構造の正則 363 になった。合わない 5 本（`Modelica.Fluid.Examples.HeatingSystem`・`NonCircularPipes`・`Tanks.ThreeTanks`・`TankWithEmptyingPipe1`・`InverseParameterization`）の原因を、`cargo xtask diff-frontend --class <名前> --dump` で Rumoca の DAE の行と突き合わせて確かめ、自前の誤りなら直す（Rumoca の誤りなら直さず根拠を記録する）。

## 決めたこと

- **base は `feature/s355-flat-stream` の 4d916cb0**。S355 が main に未着で、回帰の基準の `target/frontend-diff.json`（S355 の全体の検査の結果、538/566）が S355 の worktree にしか無い。基準は S355 の worktree から写した。
- **原因は 3 つに分かれた**（S355 の推測「レコードの方程式が欄ごとの行にならない」は半分だけ当たり）。`diff-frontend --class … --dump` の行を、変数名の集合（incidence）の多重集合で比べ、さらに自前の行から増加路で完全マッチングを探して（scratchpad の `match.py`。コミットしない）、`ThreeTanks` の行の集合の Hall の条件の破れ（64 行が未知数 63）を見つけた。
  - 範囲 `t[lo : hi]` のレコードの配列への方程式（`NonCircularPipes` -16、`HeatingSystem` -4）は、flat の数（要素の数）のまま欄を掛けていなかった。**自前の誤り**。始まり `lo` が整数のリテラルなら、要素 `lo..lo+count` の欄を取る（範囲の終わりは式のまま印字されるので、`count` を信じる）。始まりが式なら今までどおり（shortcut）。
  - レコードの配列を関数に渡す参照（`Medium.pressure(states[i + 1])`・`Medium.temperature(states)`・`mediums[1 : n].state`）が incidence に入らなかった。**自前の誤り**（S334 の「レコードそのものを関数に渡す参照」の配列版）。配列そのもの・添字が決まらない・範囲を、すべて全要素の欄に触れる参照にした。範囲と添字は見ない（広めの incidence。正則は甘く出る方に傾く）。Rumoca は関数の本体を見て欄を絞る（`Ts[1]` は `states[1].T` だけ）が、自前は絞らない。
  - `HeatingSystem` の状態 20 対 11 は **Rumoca の誤り**。Rumoca の flat（`--emit flat-json`、v0.10.2・v0.10.4）の `system.energyDynamics` は `SteadyStateInitial` なのに `initial_equations` は 8 本で `der` を含むものが 0。自前の 9 本の余計な状態は、初期方程式の `der(medium.T) = 0`（Rumoca の規則で状態になる）から来る。下書き 07（outer が inner の修正でなく型の既定を読む）と同じ族と推測。直さない。
  - `InverseParameterization` の DAE の数 287 対 279 は **Rumoca の誤り**。S355 が見つけた `redeclare model M = Impl(show = true)` の修飾の脱落で、Rumoca の flat は `h.m.show` を `false` のままにする（成分に書いた `h(m(show = true))` は通る。omc 1.27.2 も通る）。自前の 6 変数（`Res`・`m_flows_turbulent`・`dps_fg_turbulent` × 2 本の管）が +8 スカラーの差と一致。新しい上流の下書き **40**（番号は S359 の 37〜39 のあと。マージ時に S359 が main に入っていなければ振り直す）を英語・日本語・1 枚・`.mo` の 4 点セットで足した。提出しない。`.mo` は `globe fmt` で整形済み。
- **`last_subscript` に括り出した**（`resolve` の最後の添字を探す処理を 2 か所で使う。行数は増えない）。
- **新しい試験 2 本**（`record_array_slice_equation_counts_per_field`・`record_array_argument_touches_every_field`）。直す前の `lib.rs`（`record_vars`・`slice_leaves` を外した版）で 2 本とも落ちることを確かめた（10 対 8、`st` の欄に 1 つも触れない）。
- **`diff-frontend --dump` の出力に、構造特異のとき自前・Rumoca のマッチしなかった行と未知数を足した**（`unmatched row`・`unmatched unknown`）。次に正則の食い違いを見るときの手間が減る。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `record_leaves` に範囲、`slice_leaves`・`record_vars`・`range_split`・`last_subscript` の追加、`refs` の `leaves_of` を `record_vars` に、試験 2 本。
- `src/Rust/xtask/src/diff_frontend.rs`: `dump_structure` に unmatched の印字。
- 文書: ADR-0039 付記（S362）と推移の表・主題の表、roadmap・architecture・features・README・CHANGELOG の数字、`doc/upstream.md`・`doc/research/upstream/README.md` の行 40 と 07 の影響欄、`doc/research/upstream/40-…`（4 点セット）と 07 の補足、セッション文書、索引。

## 検証

- **全体の `diff-frontend`**（`xtask` を作った直後に `target-shared/bin-copies/xtask-s362` へ写し、`flock` の下で回した。基準は S355 の `target/frontend-diff.json` を写したもの）: MSL の例題 566 本で、平らにできた 538/566（変わらず）、Rumoca も通った 511 本で名前 508/511・方程式の数 498/511（変わらず）、DAE 538/538、Rumoca の釣り合いまで通った 367 本で判定 365 → **367/367**・方程式と未知数の数 364 → **366/367**・状態 366/367・構造の正則 363 → **367/367**。`GlobeModeValidation` 75 本は変わらず（Rumoca も通った 74 本で 74/74・正則 73/73）。**回帰 0 本、改善 4 本**（`TankWithEmptyingPipe1`: 正則、`HeatingSystem`: 判定・数・正則、`NonCircularPipes`: 判定・数・正則、`ThreeTanks`: 正則）。1031 s、最大 RSS 5.8 GiB。
- `cargo test -p globe-mode-dae`: 17 本合格（新しい 2 本。直す前の版では 2 本とも落ちる）。`cargo fmt --all --check`・`cargo clippy -p globe-mode-dae -p xtask --all-targets -- -D warnings`: 通過。`cargo test -p globe-mode-dae -p globe-mode-frontend -p xtask`: 182 本合格・失敗 0。`check_docs.py`: 全デッキ OK、相対リンク 5393 件中の切れ 0 件。`render_slides.py`（S362 のデッキだけ。1 回目は他の worktree のビルドの負荷で screenshot が時間切れ、負荷が下がってから撮り直し）: 問題 0 件（表のスライドは下に 250 px 以上の余白）。markdownlint-cli2（MD013・MD033 を除く）: 変えた md で 0 件（英語の下書き 07・40 の bare URL〔MD034〕は既存の下書きと同じ書き方）。`mise run check` の全体は回していない（変えた crate と依存元の frontend・xtask に絞った）
- `diff-frontend --class` で 4 本を回し直し（直したあと）: `ThreeTanks`・`TankWithEmptyingPipe1`・`NonCircularPipes` は全項目が一致、`HeatingSystem` は数・正則が一致し状態だけ 20 対 11（Rumoca の側）。
- **`git merge origin/main`（S358・S359 を取り込んだ）**: `globe-mode-flat/tests/flat.rs`（S355 の `stream_sets_and_in_stream` と S358 の `read_dirs_skips_hidden_dirs`。両方残した）、`doc/sessions/README.md`（S362・S359・S358 の順）、`doc/upstream.md`・`doc/research/upstream/README.md`（S359 の 37〜39 のあとに 40。英語の要約の本数は 45）が衝突し、すべて両方残して解いた。取り込み後の `cargo fmt --all --check`・`cargo clippy -p globe-mode-flat -p globe-mode-dae -p globe-mode-resolve -p xtask --all-targets -- -D warnings`・`cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-resolve`（76 本合格・失敗 0）通過、`check_docs.py` の相対リンク 5471 件中の切れ 0 件。全体の `diff-frontend` は回し直していない（取り込んだ変更は resolve・flat の隠しフォルダの扱いと文書で、Track B の数え方に及ばない。S358 の記録と同じ）。
- 上流の下書き 40: Rumoca v0.10.2・v0.10.4 の配布の CLI で `Rep40.A`（修飾が落ちる）・`B`・`C`（対照）を回し、omc 1.27.2 の `instantiateModel` で A が `show = true` になることを確かめた。`.mo` は `globe fmt --check` が通る。
- /tmp（tmpfs）: 大きな写しは `bin-copies/xtask-s362`（60 MB。共有 target 側）だけ。ダンプ（HeatingSystem の flat-json 38 MB ほか）は共有 target の `tmp/s362/` に置いた。

## 残したこと

- 残る 2 本の食い違い（`InverseParameterization` の数、`HeatingSystem` の状態）は Rumoca の側。自前は合わせない。上流の下書き 40（新規）と 07（補足）に根拠を足した。提出しない。
- 関数の本体を見て欄を絞る incidence（Rumoca の `Ts[1]` は `states[1].T` だけ）は作らない。広めの incidence のままで、構造の正則が甘く出うる（今の MSL では 367/367）。
- 範囲の始まりが式の `t[n + 2 : …]`（MSL の例題には出ない）は今までの数え方のまま。
- 上流の下書きの番号 40 は、S359（37〜39。origin/main に入っている）のあと。
- `doc/frontend-diff.md` は生成物なので、S361 の worktree と同時にマージすると衝突する（新しい方の全体の検査で書き直す）。

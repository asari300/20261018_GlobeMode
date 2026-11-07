# S206 `fmi-import-review-fixes` のレビュー（Fable、ponytail-review）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s206-fmi-import-review-fixes` |
| 対象 | S205 の merge（main 2c623b4）の `globe-mode-fmi-import`（親が読んで確かめた。実行はしていない） |
| レビュアー | Fable（親。読んで論証しただけ） |
| 依頼の文面 | FMI Import の Model Exchange のレビュー。指摘 1〜4 を S206 で直す |
| 作業記録 | `log/work/2026-11-07-s206-fmi-import-review-fixes.md` |

## 指摘（原文のまま）

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 1 | Should | globe-mode-fmi-import/src/ode.rs:142-144 | 誤差が有限でない（NaN・無限大）と、刻みを縮めずに Err を返す。例: h' = -k·sqrt(h) で試しの点が h < 0 に入ると NaN。直し方: 有限でない誤差を却下として扱い、刻みを 0.2 倍にして続ける（下の「刻みが小さくなりすぎた」の検査で止まる）。試験なし。 | 採用（指摘より重かった） | 再現した（`ode.rs` の単体）。**指摘の「Err を返す」は実際には違い、NaN を黙って受理して間違った値を返していた**: `fold(0.0, f64::max)` は NaN を捨てるので `!err.is_finite()` に届かない。x' = -sqrt(x)、x(0) = 0.01、t_end = 0.15 で、直す前は `Ok` で NaN を返した（解析解 0.000625。試験の失敗は「NaN」）。直した: 試しの点か誤差に有限でない値があれば誤差を無限大として却下し、刻みを 0.2 倍にして続ける。**`Discard` も同じ道**（`Error::Discard` を足し、`check` が status 2 で返す。Dormand–Prince だけ。`Error`・`Fatal` は止める。固定刻みの RK4 と二分法の中は縮めず、`Discard` のまま止まる）。模擬 FMU の試験（`Discard` を返す FMU が縮めて完走する、RK4 は止まる）も足した。 |
| 2 | Should | globe-mode-fmi-import/src/me.rs:256 | 格子点の数 n_grid に上限が無い。刻みの既定は FMU の XML の stepSize。stepSize="1e-6"・終了 100 で 1 億点。歩数の上限 2,000 万に当たるまで行を積み 1 GB 超を使ってから失敗。直し方: n_grid を計算した直後に上限（例 1,000 万）を見て「出力の刻みが細かすぎる。--step を大きくする」。CS の run_cs（lib.rs:360 付近）にも同じ形があれば同じ検査。 | 採用 | CS にも同じ形があった（`run_cs` は上限が全く無い）。`grid_points`（上限 1000 万点）を ME の `run_me`（FMU を読み込む前）・`drive` と CS の `run_cs` から呼び、実行の前に断る。試験は FMU を動かす実物（Dahlquist、`--step 1e-6` と終了 100）と CLI の終了コード 1、単体（上限ちょうどは通り、1 つ超えると断る）。「再現」は、直す前の動きを読んで確かめた（CS は 1 億回の `do_step` を始める）。 |
| 3 | Should / lean | globe-mode-fmi-import/src/lib.rs:96 | default_step_size が XML を文字列で走査（コメントの中の <DefaultExperiment を先に拾う、`stepSize = "0.01"` の空白で読めない）。直し方: globe-mode-fmi の DefaultExperiment（globe-mode-fmi/src/lib.rs:109）に step_size を足し、走査の関数と Fmu.default_step_size を消す（-12 行）。書き出し（write_model_description_2/3）と往復の試験にも step_size を通す。 | 採用 | `DefaultExperiment.step_size` を足し、2.0・3.0 とも読み書きする（無ければ属性を書かないので、Export する `modelDescription.xml` は変わらない）。`default_step_size` と `Fmu.default_step_size` を消した（-14 行）。試験は `globe-mode-fmi/tests/parse.rs` に、コメントの中の `<DefaultExperiment` と `stepSize = "0.01"` の空白（旧実装は拾えなかった）、Dahlquist の 2.0・3.0 の往復。`DefaultExperiment` を組み立てていたのは `globe-mode-fmu` だけ（`step_size: None`）。 |
| 4 | Nice | globe-mode-fmi-import/src/abi3.rs:57-67, 463-505 | 3.0 の出力引数を Rust の bool で受けている（0/1 以外が書かれると未定義動作）。直し方: u8 で受けて != 0。CS の側（do_step の出力引数）も同じ形なら同じに。 | 採用 | CS の `do_step` の出力（3 個）も同じ形だった。`update_discrete_states`・`completed_integrator_step`・`do_step` の出力をすべて `u8` で受けて `!= 0`。入力の `bool` は Rust が書くので変えない。**試験では再現できない**（UB は、0/1 以外を書く FMU を作らないと出ない。Reference-FMUs はどれも 0/1）。型だけの直しで、既存の 3.0 の試験（Reference-FMUs の CS・ME 全部）が通ることで確かめた。 |

問題なし（レビュアー）: fmi2EventInfo の並び、3.0 の関数の引数、二分法の停止、時刻を戻さないこと、同時刻の事象の上限、0・負・NaN の入力、状態 0 個の FMU。

未確認（レビュアー）: 跳ねた後に指標が 0 以下のまま残る FMU では、上がるときにも state event が 1 回数えられるはず（結果は変わらず、事象の数の表示だけ）。

## 結論

- 採用 4 件・保留 0 件・不採用 0 件。1 は試験で再現（指摘より悪く、NaN を受理していた）、2・3 は試験を足した（2 は読んで確かめた挙動）、4 は再現できない。
- 未確認の 1 件（上がるときの state event の数え方）は直していない。事象の数の表示だけで結果が変わらないため、持ち越し。

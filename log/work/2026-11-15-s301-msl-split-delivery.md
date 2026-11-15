# S301 `msl-split-delivery`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s301-msl-split-delivery` |
| worktree | `.git-worktree/s301-msl-split-delivery` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S301-msl-split-delivery/` |
| セッション記録 | なし（このセッション専用の記録は log/local/ に無い） |

## 依頼

ブラウザ版（wasm）が MSL を読むときの `MSL.json` を分割して配り、起動と最初の検査を軽くする（roadmap の機能の穴（P6））。最小の形で、Service Worker とキャッシュは足さない。CSP `connect-src 'self'` を保つ。前後の比較（取る量、起動から最初の検査までの時間、線形メモリ）を作業記録と ADR に。

## 決めたこと

- 閉包の求め方は既存の `library_closure`（S238）を wasm に出して使う。見出しのままのファイルが閉包に入ったら塊を取って取り直す。新しい閉包の実装を作らない（なぜ: 閉包は本文の参照をたどるので、本文が無いと求まらない。固定点で全部読みと同じになる）。
- 木と検索（S167 の入れ子のクラス）を落とさないため、見出しは fmt の `tree_stub`（入れ子のクラスと説明文字列を残す）にした。木の結果が本文と同じになることを関数の中で確かめ、ならなければ本文のまま（`ModelicaReference/package.mo` だけ。クラス名を引用符で囲んだ入れ子が原因）。
- 塊は同じディレクトリを 16 KiB まで。cap 128 KiB は 102 塊 3.99 MB、32 KiB は 132 塊 3.57 MB、16 KiB は 163 塊 3.28 MB、8 KiB は 197 塊 2.90 MB、1 ファイル 1 塊は 295 塊 2.49 MB（ChuaCircuit の閉包 295 ファイル 2.38 MB に対し、一度に取るとして）。要求の数と余計な量の中間で 16 KiB。
- `Workspace` は塊を取るたびに作り直し、`set_file` した文書を足し直す（`Checker` に差し込む道を作らない最小の変更）。
- `MSL.json` は pack が引き続き作る（Leptos・Dioxus・marimo と、`index.json` が無い配信）。`globe-web` の束からは外す。UI は索引が無い・JSON でないときだけ `MSL.json` に戻る。
- ユーザーと決めたことは無い（既定のまま進めた）。

## やったこと

- `globe-mode-fmt/src/tree.rs`: `tree_stub`。試験 `a_stub_keeps_the_tree`。
- `globe-mode`（facade）: `Source`・`library_closure` を再公開。`globe-mode-wasm/src/lib.rs`: `library_closure`・`tree_stub` を出した。
- `globe-mode-wasm/web/worker.js`・`client.js`: `library_index`・`library_stats`・`ensure`（`check` などの前に呼ぶ）。`web/pack.js`: 分割の作り方（pack と試験が共有）。
- `src/Svelte`: `pack-msl.mjs` が `msl/index.json` と `msl/c/*.json` も作る。`Backend.libraryIndex`、`App.svelte` の `loadMsl`。`web-bundle.sh`・`web.yml`（束の中身の確認）。`test_ui.py` に分割配信の試験を足し、構文の誤りのライブラリの試験は `index.json` を 404 にして `MSL.json` に戻す道を通す。
- `node/split.mjs`（20 本の一致と測定）、`mise run wasm:node` に 5 本で足した。
- 文書: ADR-0023 の付記、roadmap（P6 の行・持ち越しの取り消し線・S301 の節）、features、CHANGELOG、README、セッションの索引。

## 検証

数字（Node、release の wasm、ChuaCircuit の初回。`split.mjs`）:

| | 全部 | 分割 |
| --- | --- | --- |
| 取る量（生） | 15.2 MiB | 4.6 MiB（索引 1.5 + 塊 163 本） |
| 取る量（gzip） | 2.66 MB | 約 0.87 MB（索引 0.22 + 塊 0.65） |
| 読み込み | 424 ms | 83 ms |
| 読み込み後の線形メモリ | 62.1 MiB | 9.9 MiB |
| 検査の後の線形メモリ | 95.9 MiB | 74.4 MiB |
| 初回の検査 | 2031 ms | 2331 ms |
| 20 本の一致 | | 20 / 20 |

20 本を続けて回すと塊は 235 / 838、8.8 MiB（Fluid の EmptyTanks で 4.8 → 8.7 MiB に増える）。閉包を取り直す往復は ChuaCircuit 11 回、Pendulum 8 回、PID_Controller 6 回。

BROWSER_NUMBERS

- コードの検査: `cargo fmt --check` 0、`cargo clippy -p globe-mode-fmt -p globe-mode -p globe-mode-wasm --all-targets -D warnings` 0、`cargo test -p globe-mode-fmt tree` 6 通過。`svelte-check` 0 エラー。
- `check_docs.py`・リンク・漏洩検査: CHECK_DOCS

## 残したこと

- 往復の回数（索引に塊ごとの依存を持たせれば 1〜2 回）。
- Android の APK から `MSL.json` を外す（`android.yml`）。
- 566 本での一致（今回は 20 本）。
- 全体の `mise run check`・`wasm:compare` は回していない（Rust は fmt・wasm・facade の追加だけで、既存の関数は変えていない）。

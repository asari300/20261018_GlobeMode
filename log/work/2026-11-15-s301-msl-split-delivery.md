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

- 閉包の求め方は既存の走査（S238 の `library_closure`）を、ファイルごとの走査の記憶を持ち越す `LibraryClosure`（frontend、wasm に出した）にして使う。最初は関数のまま毎回全部を渡したが、1 回 100〜190 ms かかり、操作ごとに効いたので改めた。見出しのままのファイルが閉包に入ったら塊を取って取り直す。新しい閉包の実装を作らない（なぜ: 閉包は本文の参照をたどるので、本文が無いと求まらない。固定点で全部読みと同じになる）。
- 木と検索（S167 の入れ子のクラス）を落とさないため、見出しは fmt の `tree_stub`（入れ子のクラスと説明文字列を残す）にした。木の結果が本文と同じになることを関数の中で確かめ、ならなければ本文のまま（`ModelicaReference/package.mo` だけ。クラス名を引用符で囲んだ入れ子が原因）。
- 塊は同じディレクトリを 16 KiB まで。cap 128 KiB は 102 塊 3.99 MB、32 KiB は 132 塊 3.57 MB、16 KiB は 163 塊 3.28 MB、8 KiB は 197 塊 2.90 MB、1 ファイル 1 塊は 295 塊 2.49 MB（ChuaCircuit の閉包 295 ファイル 2.38 MB に対し、一度に取るとして）。要求の数と余計な量の中間で 16 KiB。
- 木の行ごとのアイコンと説明は `extends` の基底だけをたどる（`bases_only`）。全部の参照をたどると、`ObsoleteModelica4` の行を描くだけで塊の 7 割を取った（深さ 3 の打ち切りも試して、同じだった）。図は部品の型が要るので閉包のまま。
- `Workspace` は塊を取るたびに作り直し、`set_file` した文書を足し直す（`Checker` に差し込む道を作らない最小の変更）。
- `MSL.json` は pack が引き続き作る（Leptos・Dioxus・marimo と、`index.json` が無い配信）。`globe-web` の束からは外す。UI は索引が無い・JSON でないときだけ `MSL.json` に戻る。
- ユーザーと決めたことは無い（既定のまま進めた）。

## やったこと

- `globe-mode-fmt/src/tree.rs`: `tree_stub`。試験 `a_stub_keeps_the_tree`。
- `globe-mode-frontend`: `LibraryClosure`（`set_text`・`closure(…, bases_only)`）と `SplitLibrary::closure_within`・`set_text`、試験 `stubs_grow_into_the_closure_and_bases_only_follows_extends`。`globe-mode`（facade）が再公開し、`globe-mode-wasm/src/lib.rs` が `LibraryClosure`・`tree_stub` を出した。
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
| 読み込み | 407 ms | 75 ms |
| 読み込み後の線形メモリ | 62.1 MiB | 13.7 MiB |
| 検査の後の線形メモリ | 95.9 MiB | 79.2 MiB |
| 初回の検査 | 2063 ms | 2073 ms（4 本の実行。`LibraryClosure` で走査の記憶を持ち越す前は 2331 ms） |
| 20 本の一致 | | 20 / 20 |

20 本を続けて回すと塊は 235 / 838、8.8 MiB（Fluid の EmptyTanks で 4.8 → 8.7 MiB に増える）。閉包を取り直す往復は ChuaCircuit 11 回、Pendulum 8 回、PID_Controller 6 回。

ブラウザ（Playwright の Chromium、`?smoke=1`、localhost。機械は他の作業と共有でばらつく。3 回ずつ）:

| | 全部（MSL.json） | 分割配信 |
| --- | --- | --- |
| 要求と取った量（`/msl/` の content-length） | 2 要求、15.2 MiB | 174 要求、4.8 MiB |
| 起動から ChuaCircuit の最初の検査まで（`msl_ms`。回ごとの値） | 5.3 / 13.4 / 2.8 s（中央値 5.3） | 4.7 / 3.5 / 5.0 s（中央値 4.7） |
| ブラウザ全体のピーク RSS（中央値） | 385 MiB | 346 MiB |

localhost では転送が速く、時間の差は揺れの内。差が出るのは回線が遅いとき（取る量は gzip で 2.66 → 約 0.87 MB）と、メモリ。途中で、木の行のアイコンが `ObsoleteModelica4` などの閉包（参照の全部）を引いて 204 要求・7.4 MiB になっていた（`msl_ms` 3.1〜6.6 s）のを、アイコンと説明は `extends` の基底だけをたどる（`bases_only`）ようにして 174 要求・4.8 MiB にした。

- コードの検査: `cargo test -p globe-mode-fmt -p globe-mode-frontend -p globe-mode-wasm` 全通過（fmt の `a_stub_keeps_the_tree`、frontend の `stubs_grow_into_the_closure_and_bases_only_follows_extends` を含む）、`cargo fmt --check` 0、`cargo clippy -p globe-mode-fmt -p globe-mode -p globe-mode-wasm --all-targets -D warnings` 0、`cargo test -p globe-mode-fmt tree` 6 通過。`svelte-check` 0 エラー。
- `check_docs.py`・リンク・漏洩検査: `check_docs.py` スライド全部 OK、リンク 4840 件中の切れ 0。`test_ui.py`（分割配信の試験を含む）は全体を 5 回回し、通し（`ok`）が 1 回（深さ 3 で打ち切る版）、最後まで通って CodeMirror のホバーの位置の RangeError（既存の競合）だけで落ちたのが 1 回（最終版）、フォールドの印のクリックで 30 s の時間切れが 3 回（Decay の検査の直後で MSL の前。main の dist でも同じように落ちる）。最終版で分割配信の試験（`分割配信: ChuaCircuit の検査が通り、取ったのは索引と塊 173 本の合計 4.8 MiB`）は通った。`web.yml`（束に MSL.json が無く、索引の塊が全部ある）は最初のコミットで成功（run 43658690278）

## 残したこと

- 往復の回数（索引に塊ごとの依存を持たせれば 1〜2 回）。
- Android の APK から `MSL.json` を外す（`android.yml`）。
- 566 本での一致（今回は 20 本）。
- `test_ui.py` を最終版で通しで通せていない（上のとおり、既存のばらつきで落ちた）。CI の `web.yml` を最終版で回す。
- 全体の `mise run check`・`wasm:compare` は回していない（Rust は fmt・wasm・facade の追加だけで、既存の関数は変えていない）。

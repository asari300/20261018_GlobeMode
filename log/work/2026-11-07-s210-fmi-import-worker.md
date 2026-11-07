# S210 `fmi-import-worker`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s210-fmi-import-worker` |
| worktree | `.git-worktree/s210-fmi-import-worker` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S210-fmi-import-worker/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0026 (k) の 2。FMU の読み込みと実行を別プロセスのワーカーに隔離し、FMU が落ちても呼び出し側が落ちないようにする。時間とメモリの上限、進捗、落ちる FMU の試験、3 OS。

## 決めたこと

- ワーカーは `globe` 自身の隠しの副命令 `fmu __worker`。実行ファイルを呼び出し側が渡す API（`WorkerSpec`）を crate に置いた。facade・UI・Python の口は足していない。
- 結果の形は JSON でなく行ごとのメッセージ。serde_json は NaN を `null` にして往復が壊れるので、値の行は f64 の `{:?}` を空白区切りで書く。FMU の `printf` が混ざらないよう、印 `\u{1}GW ` の行だけをメッセージにする。
- メモリの上限は `sh -c 'ulimit -v'` で Linux だけ。既存の流儀（`setrlimit`）は無く、`libc` を直接の依存にして `unsafe` を足すより小さい。Windows・macOS は持ち越し。
- 孤児の防止は、子が標準入力の EOF で終わる（手で確かめた。終了コード 3）。親側は drop で kill と wait。
- `Error::Worker`（落ちた・時間切れ・起動できない）と `Error::Child`（子の中の失敗。文面そのまま）を足した。
- 進捗は `run_cs_with` / `run_me_with` に時刻のコールバックを足した（既存の関数は同じ挙動）。S123 の進捗の形は見つからなかった。
- 依存は増えない（serde・serde_json は workspace に既にある）。`unsafe` は足していない。

## やったこと

- `globe-mode-fmi-import`: `src/worker.rs`（新規）、`lib.rs`（Error 2 種、警告の文面、`run_cs_with`、再公開）、`me.rs`（`run_me_with`、`drive` に進捗）、`Cargo.toml`。
- `globe-mode-cli`: `src/fmu.rs`（`--timeout`・`--memory-limit`・`--progress`、`__worker`）、`tests/fmu_worker.rs`（新規）、`tests/fmu_import.rs`（警告の文面）、`Cargo.toml`（dev-dependency の zip）。`main.rs` は触っていない。
- 文書: ADR-0026 の (n)・(k) 2、README、features.md、roadmap.md、CHANGELOG、セッションのスライドと解説書、索引。

## 検証

- `cargo test -p globe-mode-cli --test fmu_worker`: 4 本通過（Linux x86_64）。Reference-FMUs の 5 組がプロセス内とビット単位で同じ。親の誤りは abort が `signal: 6 (SIGABRT)`、segfault が `signal: 11 (SIGSEGV)`、無限ループが「1.500 s で終わらないので止めた」、メモリ（1 GiB）が `signal: 6 (SIGABRT)`。子は残らない。
- `clippy -D warnings`: 通った。
- `mise run check`: 通った（1 回目は共有 target の古い `globe` で `fmu_import` の 1 本が落ち、`touch` で再ビルドして通過。コードの誤りではない）。
- `Cargo.lock`: 新しい package の追加なし（serde・serde_json・zip は元から lock にある）。
- 3 OS（`fmu-import.yml`・`ci.yml -f full=true`）の結果は下に追記する。

## 残したこと

- sandbox（ディスク・ネットワーク）、macOS・Windows のメモリの上限、FMU が作る孫プロセスの掃除。
- facade の `import_fmu`、UI の「FMU を開く」、Python・C ABI。
- ME の積分器・許容誤差を要求に載せていない（CLI にも無い）。

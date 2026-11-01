# S054 `library-closure`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s054-library-closure` |
| worktree | `.git-worktree/s054-library-closure` |
| Phase | P3（S051 の削減案 1） |
| セッション文書 | `doc/sessions/S054-library-closure/` |
| セッション記録 | [`log/sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md`](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

ネイティブの `globe check --library`（facade の `Workspace::with_libraries`）が MSL の 2,556 ファイルを全部読んで最大 RSS 約 1.5 GiB。wasm の分割読み込み（S037）を既定にして減らす。

## 決めたこと

- 切り替えは facade の `with_libraries` の中だけ（CLI の main.rs、`globe-mode-lsp`、`Checker` の内部は触らない）。全部読む道は `with_libraries_full` に残し、`with_libraries` の名前は変えない。
- パスが空、またはディレクトリでないパスがあるときは全部読む道に戻る（以前の挙動のまま）。
- 2 つ目以降のモデルの閉包は、S037 の `ensure_library`（読み足して Session を作り直す）に任せる。新しい仕組みは作らない。
- `msl:compare` は xtask が `Checker::new` を直接呼ぶ（facade を通らない）ので、この変更で数字が動かない。それでも指示どおり回した。
- 試験は `#[ignore]` でなく `data/msl` の存在確認で MSL の分だけ飛ばす。MSL の検査は深く再帰するのでスレッドのスタックを 256 MiB にした（既定の 2 MiB で溢れた）。

## やったこと

- `src/Rust/mod/globe-mode/src/lib.rs`: `with_libraries`（`.mo` を集めて本文を読み、`GlobeDb::with_library_sources_split`）、`with_libraries_full`、`collect_mo`、`library_error`。
- `src/Rust/mod/globe-mode/tests/library_closure.rs`: `GlobeModeValidation` 全モデルと MSL の 3 モデルで、全部読みと閉包の `CheckResponse` の JSON が一致。
- `.github/workflows/resources.yml`: 入力 `jobs`（`all`・`rss`・`wasm`）。
- `doc/adr/0025-native-library-closure.md`、`doc/adr/README.md`、`doc/adr/0023-...md`（S054 の節）、`doc/research/resources-2026-10-31.md` §6、`doc/roadmap.md`、`doc/sessions/README.md`、`doc/sessions/S054-library-closure/`。

## 検証

- `cargo test --release -p globe-mode --test library_closure`: 2 件通過（34.7 s、最大 RSS 2.2 GiB は全部読みと閉包の両方を同じプロセスに持つため）。
- `cargo test -p globe-mode-cli -p globe-mode-lsp -p globe-mode`（debug）: 全部通過。`library_closure` は debug で 399 s（他セッションと同居。MSL の 3 モデルを 2 通りで検査するため遅い。CI の `cargo test` の時間が伸びる点に注意）。
- `cargo fmt --check`、`cargo clippy -p globe-mode --all-targets -- -D warnings`: 通過。
- 手元（release）の最大 RSS、前 → 後（MiB）: `ChuaCircuit` check 1,512 → 122（5.43 → 2.12 s）、simulate 1,539 → 159。`FullRobot` check 1,655 → 291、simulate 1,669 → 512。`HeatingSystem` check 1,614 → 476、simulate 1,604 → 477。`check` の出力の md5 は 3 モデルとも前後で一致、ChuaCircuit の simulate の CSV は `cmp` で一致。壁時計は他セッションが同居していて参考にならない（`FullRobot` の simulate の前は 17 分 55 秒で swap の影響）。
- ランナー（`resources.yml -f jobs=rss`、run 41245881321、成功、3 分 18 秒）: `ChuaCircuit` check 1,493 → 128（8.1 → 1.05 s）、simulate 1,533 → 167。`FullRobot` check 1,653 → 298、simulate 1,929 → 573（終了コード 1、前と同じ）。`HeatingSystem` check 1,618 → 480、simulate 1,589 → 486（終了コード 1、前と同じ）。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。xtask は facade を通らないので、この変更の影響を受けない確認
- `check_docs.py`: デッキ OK（S054 を含む）/ リンク 1242 件中の切れ 0 件。`mise run check` ok（親、fmt / clippy / test / deny ok、library_closure 2 件通過）。

## 残したこと

- `globe lsp` を長く使ったときの RSS の伸び（閉包は増える一方）は測っていない。
- ライブラリのファイルの構文の誤りは、そのファイルを読むときに見つかる（以前は起動時）。
- 共有の `target`（3 セッションで共有）で、`globe-mode` の release の試験のビルドが古い lib を見る fingerprint の取り違えを 1 回起こした（`touch` で直った）。

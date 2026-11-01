# S066 `libs-tolerant-roots`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s066-libs-tolerant-roots` |
| worktree | `.git-worktree/s066-libs-tolerant-roots` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S066-libs-tolerant-roots/` |
| セッション記録 | （親が写す） |

## 依頼

`Checker::new` が、ライブラリの `.mo` が 1 つでも構文の誤りだとライブラリ全体を `GM-E1003` で断る点（S058・S063 の持ち越し）を、読めるファイルだけで読み、読めなかったファイルを断らずに知らせる形に直す。xtask の迂回（ミラー）を薄くするか外す。

## 決めたこと

- **方法**: 従来の道（ルート 1 つは `load_source_root_tolerant`、2 つ以上は `parse_source_root_with_cache`）を先に試し、**失敗したルートだけ** `parse_files_parallel_lenient`（Rumoca の公開 API、ファイル単位で成功と失敗を分けて返す）で 1 ファイルずつ読む。読めた文書を `replace_parsed_source_set` に渡す。理由: Rumoca を改変せず公開 API だけで足り、全部読めるルート（MSL）の道と速さを変えない。
- **配置の検査（PKG-006・PKG-010）**: 非公開関数で呼べないので、失敗したルートでは飛ばし、警告に元の診断を書く。xtask の「`package.mo` の無いディレクトリを除く」も外した（名前は `within` で決まるため。ThermofluidStream・ClaRa で判定が変わらないことを確かめた）。
- **知らせ方**: 新しい診断コード `GM-W1001`（S1 の警告、ルートごとに 1 件、ファイル名は先頭の 5 件を notes）を `Checker` が持ち、検査の結果の `diagnostics` に毎回足す。`Checker::skipped_library_files()`・`library_warnings()` でも取れる。警告は `passed()` を変えない。
- **xtask**: `mirror_libraries`・`mirror_tree`・`link_entry`・`contains_mo`・`layout_offenders`（と最大 20 回の繰り返し）・`libs::unparsable_files` を削除。`make_checker` は `Checker::new(&[msl, lib…])` だけ。読み飛ばしの一覧 `target/library-roots/<名前>.skipped.txt`（`パス<TAB>理由`）は、libraries.yml の artifact のために今も書く。
- ADR は書かない（公開 API の組み合わせで、覆す設計の判断ではない）。番号の衝突は S 番号・ADR 番号とも無かった（S066 は空き。ADR は足していない）。

## やったこと

- `globe-mode-core/src/lib.rs`: `codes::LIBRARY_PARTIAL = "GM-W1001"`。
- `globe-mode-frontend/src/rumoca.rs`・`lib.rs`: `read_tolerant`・`collect_mo_files`・`SkippedFile`、`Checker` に `skipped`・`warnings` と 2 つの accessor、`check_synced` が警告を足す。
- `globe-mode-frontend/tests/tolerant_roots.rs` と `tests/data/tolerant/{a,b,c}`（fixture）。
- `xtask/src/funnel.rs`・`libs.rs`: 迂回の削除。
- 文書: research（節「読めるファイルだけでライブラリを読む」）、roadmap、`doc/sessions/S066-libs-tolerant-roots/`、索引。

## 検証

- `cargo fmt --all --check`・`cargo clippy -p globe-mode-frontend -p xtask --all-targets -- -D warnings`: OK。
- `cargo test -p globe-mode-frontend -p xtask`: 全部 OK（`tolerant_roots` 4 本、xtask 46 本）。`mise run check` は親が回す。
- `mise run msl:compare`: 20 本中 20 本一致。
- 手元の `msl-funnel`（release）を S063 の run（41250636800）の JSON と比較: ThermofluidStream 122 本（OK 4・GM-U 115・panic 0、134.2 秒。S063 は 135.3 秒）、ClaRa 5 本（OK 0・S1 で NG 5、6.5 秒。5.0 秒）とも、判定・段・GM コード・Rumoca コードの差 0。読み飛ばしの警告は ThermofluidStream で `Contact.mo` の 1 ファイル、ClaRa は「配置の検査を飛ばして読んだ」1 件。
- Actions: `libraries.yml`（run 41295588356、Chemical・ClaRa・ThermofluidStream・DeltaRobot）は @@RUNRESULT@@。Chemical・DeltaRobot は S058 の `skipped.txt` が空で、読み飛ばしの道を通らない。
- `check_docs.py`: リンク切れ 0（`render_slides.py` は親が回す）。
- 開発中に見つけたこと: 共有の `CARGO_TARGET_DIR`（`target-shared`）を複数の worktree で使うと、`cargo xtask`（release）が別の worktree が作った古い `globe-mode-core` を使い、コンパイルエラーになった。ソースを `touch` して直した（cargo が path の依存の指紋を mtime で見るため）。

## 残したこと

- 失敗したルートでは配置の検査を飛ばす。配置の誤りを警告以上にするには、Rumoca に `validate_source_root_package_layout` の公開が要る。
- `GM-W1001` を UI の診断の一覧に出す。wasm の `with_library_sources` は構文の誤りで断るまま。
- Buildings・AixLib・IBPSA は手元で回していない（S058 では読み飛ばしが BOM だけで、BOM は `fetch-library` が外す）。

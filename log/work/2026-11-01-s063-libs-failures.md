# S063 `libs-failures`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s063-libs-failures` |
| worktree | `.git-worktree/s063-libs-failures` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S063-libs-failures/` |
| セッション記録 | [`log/sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md`](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S058 の持ち越し。(1) `Checker::new(&[msl, lib])` で `import Modelica.Units.SI` が全例題 `GM-E1001` になる原因を突き止めて直し、xtask の symlink の迂回を外す。(2) run 41250636800・41248143483 の失敗を原因別に集計し、ライブラリごとの上位 5 つを research に書く（MSL 4.0.0 と 4.1.0 の差、取得しない依存は別に数える）。(3) ClaRa の例題 5 本（OK 0）の原因。

## 決めたこと

- **原因**: Rumoca の `load_source_root_tolerant` はルートごとに意味の要約（`sync_source_root_semantic_summary_cache`）を作り、そのルートの中だけで名前を引く。切り分けは Rumoca の Session を直接使う小さなプログラムで、ディスクの 2 ルート（失敗）・メモリの 2 組（成功）・1 組（成功）を比べた。CLI は `globe check --library data/msl --library data/libs/Chemical` は S054 から `Workspace::with_libraries` がメモリの 1 組で読むので、直す前から通っていた（依頼の「迂回なしで通る」はこの道）。再現するのは `Checker::new` を直接呼ぶ xtask と、MSL なしの最小の 2 ルート。
- **修正は `Checker::new` の中だけ**: ルートが 2 つ以上のとき `parse_source_root_with_cache`（キャッシュとルートごとの配置の検査は同じ）で読み、全文書を 1 組 `library:<a>,<b>` として入れる。1 ルートの道は変えない（要約が速さに効くかもしれないため）。Rumoca は改変しない。
- **xtask**: MSL の symlink をやめた。ミラーは、読めないファイル（構文の誤り・PKG-006/010）を除くためにライブラリだけで残す。`target/library-roots/<名前>/` の古い（MSL の symlink 入りの）ミラーが手元に残っていると MSL を二重に読むので、消してから回す（CI は毎回新しい）。
- **分類の単位**は例題 1 本に最初の診断 1 つ。`GM-E1001` は ER070・ER132 のような名前解決でない検査も含むため、`rumoca_code` とメッセージの形で分けた。
- 手元の `msl-funnel` は debug ビルド（RAM 3 GB で release を作らなかった）。Chemical の検査は 892 s。

## やったこと

- `src/Rust/mod/globe-mode-frontend/src/rumoca.rs`: `Checker::new` の 2 ルート以上の道。
- `src/Rust/mod/globe-mode-frontend/tests/multi_root.rs`（新規）: MSL なしの 2 ルートで、順序を入れ替えても `import` が解決する試験（直す前は `GM-E1001` を確認）。
- `src/Rust/xtask/src/funnel.rs`: `make_checker` が MSL を別ルートで渡す。`merge_roots` を `mirror_libraries`（ライブラリだけ）に。
- `doc/research/libraries-2026-11-01.md`: 原因の節、失敗の分類（段別・ライブラリ別の上位 5・共通の原因・版の差と依存・ClaRa）。
- `doc/roadmap.md`（S063 の節、S058 の持ち越しに済み）、`doc/sessions/S063-libs-failures/`、`doc/sessions/README.md`。

## 検証

- `cargo test -p globe-mode-frontend`（multi_root を含む）、`cargo test -p xtask`（47 本）OK。`cargo clippy -p globe-mode-frontend -p xtask --all-targets -- -D warnings` OK、`cargo fmt --check` OK。`mise run check` は親が回す。
- `msl-funnel`（手元、`--library` 1 つ）: DeltaRobot 21 本 OK 9・NG 7・GM-U 5、Chemical 76 本 OK 12・NG 21・GM-U 43、ClaRa 5 本 NG 5。S058 の run の JSON と 3 つとも判定・段・GM コード・Rumoca コードが全部同じ（差 0）。ThermofluidStream・AixLib・Buildings・IBPSA は手元で回していない。
- 分類の入力は run 41250636800・41248143483 の artifact（2,132 本、OK 351、失敗 1,781）。

## 残したこと

- 新しい Actions の run は回していない（`libraries.yml` の挙動は xtask の `make_checker` が変わっただけ。ThermofluidStream・AixLib など、除くファイルの多いライブラリでの確認は次に回すとき）。
- Rumoca 側の検査（ER132・ER070・EI032・ER023）を上流に出すか。
- MSL 4.0.0 での回し直し（版の差の本数は、メッセージに名前が出るもの 1 本しか確かめていない）。
- SDF を使って止まる本数は、名指すものが 0 本。名指さない分は分けられない。

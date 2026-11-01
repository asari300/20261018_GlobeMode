# ADR-0025 ネイティブの library も閉包だけ読む: `Workspace::with_libraries` は wasm の分割読み込みと同じ道にする

- 状態: Accepted（2026-11-01、S054）
- 関係: [ADR-0023](0023-desktop-and-web-process-layout.md)（検査は別プロセスのワーカー。RSS の見積もり）、[ADR-0014](0014-session-route-wasm.md)。分割読み込みは S037（wasm）で作り、S047 で MSL の 566 本すべてが全部読みと同じ検査結果になることを確かめた。計測は [research/resources-2026-10-31.md](../research/resources-2026-10-31.md) の §6

## 背景

- `globe check --library data/msl` は MSL 4.1.0 の 2,556 ファイルを Rumoca の Session に全部読み、`ChuaCircuit` で最大 RSS が約 1.5 GiB になる（S040、S051）。`globe lsp` も Desktop の sidecar（[ADR-0023](0023-desktop-and-web-process-layout.md) (b)）も同じ道（facade の `Workspace::with_libraries`）で読むので、Desktop のワーカーは起動するたびにこれだけ使う。
- wasm には S037 の分割読み込み（`Workspace::with_library_sources_split`）がある。ライブラリのファイルを本文のまま持ち、検査のたびにモデルと作業中の文書から名前をたどって、使うファイルの閉包だけを Session に足す。S047 の `split-all` で、MSL の全例題 566 本の検査結果が全部読みと一致した。
- S051 の手実験で、閉包の 1,075 ファイルだけを渡すと RSS は 461 MiB だった。ただしそれは `--library` と別の道で、ネイティブの `--library` にそのまま使えるかは確かめていなかった。

## 決定

- facade の `Workspace::with_libraries(paths)` を、ディレクトリの下の `.mo` をすべて文字列として読み、`with_library_sources_split` に渡す道にする。CLI（`globe check`・`simulate`・`lsp`）、Python の束縛、Desktop の sidecar は呼び出しを変えずに切り替わる。
- 全部を Session に読む道は `Workspace::with_libraries_full(paths)` として残す（比較の試験と、1 ファイルに 2 つ以上のトップレベルのクラスがあるライブラリ向け。`split_library.rs` の shortcut と同じ制約）。
- 次のときは全部読む道に戻る: `paths` が空、またはディレクトリでないパスがある。
- 閉包はモデルごとに計算する。2 つ目以降のモデル（`globe lsp` が順に検査する）で閉包が増えたら、読み足して Session を作り直す（S037 の `ensure_library`。読んだものは外さない）。

## 結果

- 検査の結果は変わらない。facade の試験 `tests/library_closure.rs` が、`GlobeModeValidation` の全モデルと、MSL の `ChuaCircuit`・`FullRobot`・`HeatingSystem` を 1 つの `Workspace` で順に検査し、`CheckResponse` の JSON が `with_libraries_full` と一致することを見る。
- 最大 RSS の前後（手元とランナー）は [research/resources-2026-10-31.md](../research/resources-2026-10-31.md) の §6。
- 代償: 診断の位置は変わらないが、ライブラリのファイルの構文の誤りは「そのファイルを読むとき」に見つかる（以前は起動時）。モデルが使わないファイルの誤りは見つからない。`Checker::libraries()` は `library:<パスをコンマでつないだ名前>` を返す（ディレクトリのパスではない）。
- 閉包が狭すぎると Rumoca の名前解決が失敗する危険は残る。MSL の 566 本は S047 で確かめた。ほかのライブラリは `with_libraries_full` に戻して比べられる。
- `Checker` の内部（`globe-mode-frontend` の `Checker::new`）は変えていない。`msl:funnel`・`msl:compare`・xtask は全部読みのままで、数字は変わらない。
